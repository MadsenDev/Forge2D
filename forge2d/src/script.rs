use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::{anyhow, Result};
use mlua::{Lua, UserData, UserDataMethods};

use crate::entities::{SpriteComponent, Transform};
use crate::render::AnimatedSprite;
use crate::input::InputState;
use crate::math::Vec2;
use crate::physics::{PhysicsEvent, PhysicsWorld, RigidBodyType};
use crate::world::{EntityId, World};

// Implement Lua conversion for Vec2
impl<'lua> mlua::FromLua<'lua> for Vec2 {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        match lua_value {
            mlua::Value::Table(t) => {
                // Try named fields first (x, y), then fall back to indexed (1, 2)
                let x: f64 = t.get("x").or_else(|_| t.get(1))?;
                let y: f64 = t.get("y").or_else(|_| t.get(2))?;
                Ok(Vec2::new(x as f32, y as f32))
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "Vec2",
                message: Some("Expected table with x and y fields".to_string()),
            }),
        }
    }
}

impl<'lua> mlua::IntoLua<'lua> for Vec2 {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        Ok(mlua::Value::Table(table))
    }
}

/// Simple configuration value that can be passed from Rust into a script.
#[derive(Clone, Debug)]
pub enum ScriptValue {
    Number(f32),
    Bool(bool),
    Text(String),
    Vec2(Vec2),
}

impl From<f32> for ScriptValue {
    fn from(value: f32) -> Self {
        Self::Number(value)
    }
}

impl From<bool> for ScriptValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<&str> for ScriptValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for ScriptValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec2> for ScriptValue {
    fn from(value: Vec2) -> Self {
        Self::Vec2(value)
    }
}

/// Arbitrary parameters that can be consumed by a script on startup.
#[derive(Clone, Debug, Default)]
pub struct ScriptParams {
    values: HashMap<String, ScriptValue>,
}

impl ScriptParams {
    /// Insert a configurable parameter for the script.
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<ScriptValue>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

/// The script component stored on entities. Contains an ordered list of script attachments.
#[derive(Clone, Debug, Default)]
pub struct ScriptComponent {
    pub scripts: Vec<ScriptAttachment>,
}

impl ScriptComponent {
    /// Attach a script module (file path or asset identifier) with optional parameters.
    pub fn with_script(mut self, path: impl Into<String>, params: ScriptParams) -> Self {
        self.scripts.push(ScriptAttachment {
            path: path.into(),
            params,
        });
        self
    }
}

/// Single script entry in a ScriptComponent.
#[derive(Clone, Debug)]
pub struct ScriptAttachment {
    pub path: String,
    pub params: ScriptParams,
}

struct ScriptModule {
    source: String,
    modified: Option<SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ScriptInstanceKey {
    entity: EntityId,
    slot: u32,
}

struct ScriptInstance {
    key: ScriptInstanceKey,
    script_path: String,
    has_started: bool,
    last_loaded: Option<SystemTime>,
}

impl ScriptInstance {
    fn new(
        key: ScriptInstanceKey,
        script_path: String,
        params: &ScriptParams,
        module: &ScriptModule,
    ) -> Self {
        Self {
            key,
            script_path,
            has_started: false,
            last_loaded: module.modified,
        }
    }
}

#[derive(Default)]
pub struct ScriptCommandBuffer {
    commands: Vec<ScriptCommand>,
    pending_spawns: Vec<SpawnRequest>,
}

#[derive(Clone, Debug)]
pub enum ScriptCommand {
    SetTransform {
        entity: EntityId,
        position: Option<Vec2>,
        rotation: Option<f32>,
        scale: Option<Vec2>,
    },
    SetSpriteVisibility {
        entity: EntityId,
        visible: bool,
    },
    SetSpriteTint {
        entity: EntityId,
        tint: [f32; 4],
    },
    ApplyImpulse {
        entity: EntityId,
        impulse: Vec2,
    },
    SetVelocity {
        entity: EntityId,
        velocity: Vec2,
    },
    UpdateAnimation {
        entity: EntityId,
        dt: f32,
    },
    SetAnimationPlaying {
        entity: EntityId,
        playing: bool,
    },
    ResetAnimation {
        entity: EntityId,
    },
    SetAnimationSpeed {
        entity: EntityId,
        speed: f32,
    },
    SetTilemapTile {
        entity: EntityId,
        x: u32,
        y: u32,
        tile_id: u32,
    },
    FillTilemapRect {
        entity: EntityId,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        tile_id: u32,
    },
    Despawn {
        entity: EntityId,
    },
}

#[derive(Clone, Debug)]
pub struct SpawnRequest {
    pub body: SpawnBody,
    pub initial_velocity: Option<Vec2>,
    pub tag: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SpawnBody {
    Empty { position: Option<Vec2> },
    Dynamic { position: Vec2 },
}

impl ScriptCommandBuffer {
    pub fn set_transform(
        &mut self,
        entity: EntityId,
        position: Option<Vec2>,
        rotation: Option<f32>,
        scale: Option<Vec2>,
    ) {
        self.commands.push(ScriptCommand::SetTransform {
            entity,
            position,
            rotation,
            scale,
        });
    }

    pub fn set_sprite_visibility(&mut self, entity: EntityId, visible: bool) {
        self.commands
            .push(ScriptCommand::SetSpriteVisibility { entity, visible });
    }

    pub fn set_sprite_tint(&mut self, entity: EntityId, tint: [f32; 4]) {
        self.commands
            .push(ScriptCommand::SetSpriteTint { entity, tint });
    }

    pub fn apply_impulse(&mut self, entity: EntityId, impulse: Vec2) {
        self.commands
            .push(ScriptCommand::ApplyImpulse { entity, impulse });
    }

    pub fn set_velocity(&mut self, entity: EntityId, velocity: Vec2) {
        self.commands
            .push(ScriptCommand::SetVelocity { entity, velocity });
    }

    pub fn update_animation(&mut self, entity: EntityId, dt: f32) {
        self.commands.push(ScriptCommand::UpdateAnimation { entity, dt });
    }

    pub fn set_animation_playing(&mut self, entity: EntityId, playing: bool) {
        self.commands.push(ScriptCommand::SetAnimationPlaying { entity, playing });
    }

    pub fn reset_animation(&mut self, entity: EntityId) {
        self.commands.push(ScriptCommand::ResetAnimation { entity });
    }

    pub fn set_animation_speed(&mut self, entity: EntityId, speed: f32) {
        self.commands.push(ScriptCommand::SetAnimationSpeed { entity, speed });
    }

    pub fn set_tilemap_tile(&mut self, entity: EntityId, x: u32, y: u32, tile_id: u32) {
        self.commands.push(ScriptCommand::SetTilemapTile { entity, x, y, tile_id });
    }

    pub fn fill_tilemap_rect(&mut self, entity: EntityId, x: u32, y: u32, width: u32, height: u32, tile_id: u32) {
        self.commands.push(ScriptCommand::FillTilemapRect { entity, x, y, width, height, tile_id });
    }

    pub fn spawn(&mut self, request: SpawnRequest) {
        self.pending_spawns.push(request);
    }

    pub fn despawn(&mut self, entity: EntityId) {
        self.commands.push(ScriptCommand::Despawn { entity });
    }

    pub fn apply(&mut self, world: &mut World, physics: &mut PhysicsWorld) {
        for request in self.pending_spawns.drain(..) {
            let entity = world.spawn();
            match request.body {
                SpawnBody::Empty { position } => {
                    if let Some(pos) = position {
                        world.insert(entity, Transform::new(pos));
                    }
                }
                SpawnBody::Dynamic { position } => {
                    world.insert(entity, Transform::new(position));
                    let _ = physics.create_body(entity, RigidBodyType::Dynamic, position, 0.0);
                }
            }

            if let Some(initial_velocity) = request.initial_velocity {
                physics.set_linear_velocity(entity, initial_velocity);
            }

            if let Some(tag) = request.tag {
                world.insert(entity, ScriptTag(tag));
            }
        }

        for command in self.commands.drain(..) {
            match command {
                ScriptCommand::SetTransform {
                    entity,
                    position,
                    rotation,
                    scale,
                } => {
                    if let Some(transform) = world.get_mut::<Transform>(entity) {
                        if let Some(p) = position {
                            transform.position = p;
                            physics.set_body_position(entity, p);
                        }
                        if let Some(r) = rotation {
                            transform.rotation = r;
                            physics.set_body_rotation(entity, r);
                        }
                        if let Some(s) = scale {
                            transform.scale = s;
                        }
                    }
                }
                ScriptCommand::SetSpriteVisibility { entity, visible } => {
                    if let Some(sprite) = world.get_mut::<SpriteComponent>(entity) {
                        sprite.visible = visible;
                    }
                }
                ScriptCommand::SetSpriteTint { entity, tint } => {
                    if let Some(sprite) = world.get_mut::<SpriteComponent>(entity) {
                        sprite.sprite.tint = tint;
                    }
                }
                ScriptCommand::ApplyImpulse { entity, impulse } => {
                    physics.apply_impulse(entity, impulse);
                }
                ScriptCommand::SetVelocity { entity, velocity } => {
                    physics.set_linear_velocity(entity, velocity);
                    physics.wake_up(entity, true);
                }
                ScriptCommand::UpdateAnimation { entity, dt } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.update(dt);
                    }
                }
                ScriptCommand::SetAnimationPlaying { entity, playing } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.playing = playing;
                    }
                }
                ScriptCommand::ResetAnimation { entity } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.reset();
                    }
                }
                ScriptCommand::SetAnimationSpeed { entity, speed } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.speed = speed;
                    }
                }
                ScriptCommand::SetTilemapTile { entity, x, y, tile_id } => {
                    if let Some(tilemap_comp) = world.get_mut::<crate::entities::TilemapComponent>(entity) {
                        tilemap_comp.tilemap.set_tile(x, y, tile_id);
                    }
                }
                ScriptCommand::FillTilemapRect { entity, x, y, width, height, tile_id } => {
                    if let Some(tilemap_comp) = world.get_mut::<crate::entities::TilemapComponent>(entity) {
                        tilemap_comp.tilemap.fill_rect(x, y, width, height, tile_id);
                    }
                }
                ScriptCommand::Despawn { entity } => {
                    physics.remove_body(entity);
                    world.despawn(entity);
                }
            }
        }
    }
}

/// Tag component that scripts can query for targeted entity lookups.
#[derive(Clone, Debug)]
pub struct ScriptTag(pub String);

// Lua userdata types
#[derive(Clone)]
pub struct ScriptSelf {
    entity: EntityId,
    world: *const World,
    physics: *const PhysicsWorld,
    input: *const InputState,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
    dt: f32,
    fixed_dt: f32,
}

impl UserData for ScriptSelf {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("entity", |_, this, ()| Ok(this.entity.to_u32() as i64));
        methods.add_method("time", |_, this, ()| {
            Ok(TimeFacet {
                dt: this.dt,
                fixed_dt: this.fixed_dt,
            })
        });
        methods.add_method("input", |_, this, ()| {
            Ok(InputFacet {
                input: this.input,
            })
        });
        methods.add_method("world", |_, this, ()| {
            Ok(WorldFacet {
                world: this.world,
                commands: Arc::clone(&this.commands),
            })
        });
        methods.add_method("transform", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<Transform>(this.entity).is_some() {
                Ok(Some(TransformFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("physics", |_, this, ()| {
            let physics = unsafe { &*this.physics };
            if physics.has_body(this.entity) {
                Ok(Some(PhysicsFacet {
                    entity: this.entity,
                    physics: this.physics,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("sprite", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<SpriteComponent>(this.entity).is_some() {
                Ok(Some(SpriteFacet {
                    entity: this.entity,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("animation", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<AnimatedSprite>(this.entity).is_some() {
                Ok(Some(AnimationFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("tilemap", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<crate::entities::TilemapComponent>(this.entity).is_some() {
                Ok(Some(TilemapFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("position", |_, this, ()| {
            let world = unsafe { &*this.world };
            match world.get::<Transform>(this.entity) {
                Some(t) => Ok(t.position),
                None => Ok(Vec2::ZERO),
            }
        });
        methods.add_method("set_position", |_, this, pos: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, Some(pos), None, None);
            }
            Ok(())
        });
        methods.add_method("apply_impulse", |_, this, impulse: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.apply_impulse(this.entity, impulse);
            }
            Ok(())
        });
    }
}

impl ScriptSelf {
    fn new(
        entity: EntityId,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
        commands: Arc<Mutex<ScriptCommandBuffer>>,
        dt: f32,
        fixed_dt: f32,
    ) -> Self {
        Self {
            entity,
            world,
            physics,
            input,
            commands,
            dt,
            fixed_dt,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TimeFacet {
    dt: f32,
    fixed_dt: f32,
}

impl UserData for TimeFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("delta", |_, this, ()| Ok(this.dt));
        methods.add_method("fixed_delta", |_, this, ()| Ok(this.fixed_dt));
    }
}

#[derive(Clone)]
pub struct InputFacet {
    input: *const InputState,
}

impl UserData for InputFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("is_key_down", |_, this, name: String| {
            Ok(parse_key(&name)
                .map(|k| unsafe { &*this.input }.is_key_down(k))
                .unwrap_or(false))
        });
        methods.add_method("is_key_pressed", |_, this, name: String| {
            Ok(parse_key(&name)
                .map(|k| unsafe { &*this.input }.is_key_pressed(k))
                .unwrap_or(false))
        });
        methods.add_method("is_key_released", |_, this, name: String| {
            Ok(parse_key(&name)
                .map(|k| unsafe { &*this.input }.is_key_released(k))
                .unwrap_or(false))
        });
        methods.add_method("mouse_pos_screen", |_, this, ()| {
            let (x, y) = unsafe { &*this.input }.mouse_screen_pixels();
            Ok(Vec2::new(x, y))
        });
        methods.add_method("is_mouse_pressed", |_, this, button_name: String| {
            use winit::event::MouseButton;
            let button = match button_name.as_str() {
                "Left" | "left" => MouseButton::Left,
                "Right" | "right" => MouseButton::Right,
                "Middle" | "middle" => MouseButton::Middle,
                _ => return Ok(false),
            };
            Ok(unsafe { &*this.input }.is_mouse_pressed(button))
        });
        methods.add_method("is_mouse_down", |_, this, button_name: String| {
            use winit::event::MouseButton;
            let button = match button_name.as_str() {
                "Left" | "left" => MouseButton::Left,
                "Right" | "right" => MouseButton::Right,
                "Middle" | "middle" => MouseButton::Middle,
                _ => return Ok(false),
            };
            Ok(unsafe { &*this.input }.is_mouse_down(button))
        });
    }
}

#[derive(Clone)]
pub struct WorldFacet {
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for WorldFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("find_by_tag", |_, this, tag: String| {
            for (entity, t) in unsafe { &*this.world }.query::<ScriptTag>() {
                if t.0 == tag {
                    return Ok(Some(entity.to_u32() as i64));
                }
            }
            Ok(None)
        });
        methods.add_method("despawn", |_, this, entity_raw: i64| {
            if entity_raw < 0 {
                return Err(mlua::Error::RuntimeError("Entity id must be non-negative".to_string()));
            }
            let entity = EntityId(entity_raw as u32);
            if !unsafe { &*this.world }.is_alive(entity) {
                return Ok(());
            }
            if let Ok(mut commands) = this.commands.lock() {
                commands.despawn(entity);
            }
            Ok(())
        });
        methods.add_method("spawn_dynamic", |_, this, (position, velocity): (Vec2, Vec2)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.spawn(SpawnRequest {
                    body: SpawnBody::Dynamic { position },
                    initial_velocity: Some(velocity),
                    tag: None,
                });
            }
            Ok(())
        });
        methods.add_method("spawn_empty", |_, this, (position, tag): (Option<Vec2>, Option<String>)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.spawn(SpawnRequest {
                    body: SpawnBody::Empty { position },
                    initial_velocity: None,
                    tag,
                });
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct TransformFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for TransformFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, ()| {
            match unsafe { &*this.world }.get::<Transform>(this.entity) {
                Some(t) => Ok(t.position),
                None => Ok(Vec2::ZERO),
            }
        });
        methods.add_method("rotation", |_, this, ()| {
            match unsafe { &*this.world }.get::<Transform>(this.entity) {
                Some(t) => Ok(t.rotation),
                None => Ok(0.0),
            }
        });
        methods.add_method("set_position", |_, this, pos: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, Some(pos), None, None);
            }
            Ok(())
        });
        methods.add_method("set_rotation", |_, this, rot: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, None, Some(rot as f32), None);
            }
            Ok(())
        });
        methods.add_method("set_scale", |_, this, scale: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, None, None, Some(scale));
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct PhysicsFacet {
    entity: EntityId,
    physics: *const PhysicsWorld,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for PhysicsFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("velocity", |_, this, ()| {
            let physics = unsafe { &*this.physics };
            Ok(physics.linear_velocity(this.entity).unwrap_or(Vec2::ZERO))
        });
        methods.add_method("set_velocity", |_, this, velocity: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_velocity(this.entity, velocity);
            }
            Ok(())
        });
        methods.add_method("apply_impulse", |_, this, impulse: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.apply_impulse(this.entity, impulse);
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct SpriteFacet {
    entity: EntityId,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for SpriteFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("set_visible", |_, this, visible: bool| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_sprite_visibility(this.entity, visible);
            }
            Ok(())
        });
        methods.add_method("set_tint", |_, this, tint: mlua::Table| {
            let r: f64 = tint.get(1)?;
            let g: f64 = tint.get(2)?;
            let b: f64 = tint.get(3)?;
            let a: f64 = tint.get(4)?;
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_sprite_tint(this.entity, [r as f32, g as f32, b as f32, a as f32]);
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct AnimationFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for AnimationFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("update", |_, this, dt: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.update_animation(this.entity, dt as f32);
            }
            Ok(())
        });
        methods.add_method("play", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_animation_playing(this.entity, true);
            }
            Ok(())
        });
        methods.add_method("pause", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_animation_playing(this.entity, false);
            }
            Ok(())
        });
        methods.add_method("reset", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.reset_animation(this.entity);
            }
            Ok(())
        });
        methods.add_method("set_speed", |_, this, speed: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_animation_speed(this.entity, speed as f32);
            }
            Ok(())
        });
        methods.add_method("current_frame_index", |_, this, ()| {
            let world = unsafe { &*this.world };
            Ok(world.get::<AnimatedSprite>(this.entity)
                .map(|a| a.current_frame_index as i64)
                .unwrap_or(0))
        });
    }
}

#[derive(Clone)]
pub struct TilemapFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for TilemapFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("set_tile", |_, this, (x, y, tile_id): (u32, u32, u32)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_tilemap_tile(this.entity, x, y, tile_id);
            }
            Ok(())
        });
        methods.add_method("get_tile", |_, this, (x, y): (u32, u32)| {
            let world = unsafe { &*this.world };
            Ok(world.get::<crate::entities::TilemapComponent>(this.entity)
                .and_then(|t| t.tilemap.get_tile(x, y))
                .map(|t| t.id as i64)
                .unwrap_or(0))
        });
        methods.add_method("fill_rect", |_, this, (x, y, w, h, tile_id): (u32, u32, u32, u32, u32)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.fill_tilemap_rect(this.entity, x, y, w, h, tile_id);
            }
            Ok(())
        });
        methods.add_method("world_to_tile", |lua, this, world_pos: Vec2| {
            let world = unsafe { &*this.world };
            if let Some(tilemap_comp) = world.get::<crate::entities::TilemapComponent>(this.entity) {
                let (tx, ty) = tilemap_comp.tilemap.world_to_tile(world_pos);
                let table = lua.create_table()?;
                table.set("x", tx)?;
                table.set("y", ty)?;
                Ok(mlua::Value::Table(table))
            } else {
                let table = lua.create_table()?;
                table.set("x", 0)?;
                table.set("y", 0)?;
                Ok(mlua::Value::Table(table))
            }
        });
        methods.add_method("tile_to_world", |_, this, (x, y): (u32, u32)| {
            let world = unsafe { &*this.world };
            Ok(world.get::<crate::entities::TilemapComponent>(this.entity)
                .map(|t| t.tilemap.tile_to_world(x, y))
                .unwrap_or(Vec2::ZERO))
        });
    }
}

/// Central runtime that owns the embedded scripting engine and per-entity instances.
pub struct ScriptRuntime {
    pub(crate) lua: Lua,
    modules: HashMap<String, ScriptModule>,
    instances: BTreeMap<ScriptInstanceKey, ScriptInstance>,
    command_buffer: Arc<Mutex<ScriptCommandBuffer>>,
    hot_reload: bool,
}

impl ScriptRuntime {
    /// Create a new runtime with Lua backend and built-in API bindings.
    pub fn new() -> Result<Self> {
        let lua = Lua::new();
        
        // Register print function
        let print_func = lua.create_function(|_, msg: String| {
            println!("[LUA] {}", msg);
            Ok(())
        })?;
        lua.globals().set("print", print_func)?;

        // Register Vec2 type
        lua.register_userdata_type::<Vec2>(|reg| {
            reg.add_method("x", |_, this, ()| Ok(this.x));
            reg.add_method("y", |_, this, ()| Ok(this.y));
            reg.add_method_mut("set_x", |_, this, x: f64| {
                this.x = x as f32;
                Ok(())
            });
            reg.add_method_mut("set_y", |_, this, y: f64| {
                this.y = y as f32;
                Ok(())
            });
        })?;

        // Register vec2 function
        let vec2_func = lua.create_function(|_, (x, y): (f64, f64)| {
            Ok(Vec2::new(x as f32, y as f32))
        })?;
        lua.globals().set("vec2", vec2_func)?;

        // Register GridCoord type
        lua.register_userdata_type::<crate::grid::GridCoord>(|reg| {
            reg.add_method("x", |_, this, ()| Ok(this.x));
            reg.add_method("y", |_, this, ()| Ok(this.y));
        })?;
        
        // Register GridNode type
        lua.register_userdata_type::<crate::pathfinding::GridNode>(|reg| {
            reg.add_method("x", |_, this, ()| Ok(this.x));
            reg.add_method("y", |_, this, ()| Ok(this.y));
        })?;

        // UserData types are automatically registered when first used
        // No explicit registration needed - the UserData impl provides the methods

        Ok(Self {
            lua,
            modules: HashMap::new(),
            instances: BTreeMap::new(),
            command_buffer: Arc::new(Mutex::new(ScriptCommandBuffer::default())),
            hot_reload: false,
        })
    }

    /// Toggle hot reload for script files on disk.
    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload = enabled;
        self
    }
    
    /// Register a custom Lua function in the global namespace.
    /// This allows demos/examples to expose custom APIs to scripts.
    pub fn register_function<F, A, R>(&mut self, name: &str, func: F) -> Result<()>
    where
        F: 'static + Send + for<'lua> Fn(&'lua Lua, A) -> mlua::Result<R>,
        A: for<'lua> mlua::FromLuaMulti<'lua>,
        R: for<'lua> mlua::IntoLuaMulti<'lua>,
    {
        let func = self.lua.create_function(func)?;
        self.lua.globals().set(name, func)?;
        Ok(())
    }
    
    /// Get mutable access to the Lua state for advanced use cases.
    /// This allows registering custom functions and types directly.
    pub fn lua_mut(&mut self) -> &mut Lua {
        &mut self.lua
    }

    /// Drive `on_update` for all scripts.
    pub fn update(
        &mut self,
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
        dt: f32,
    ) -> Result<()> {
        self.sync_instances(world, physics, input)?;
        self.run_stage(world, physics, input, dt, 0.0, ScriptStage::Update)?;
        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    /// Drive `on_fixed_update` for all scripts.
    pub fn fixed_update(
        &mut self,
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
        fixed_dt: f32,
    ) -> Result<()> {
        self.sync_instances(world, physics, input)?;
        self.run_stage(
            world,
            physics,
            input,
            0.0,
            fixed_dt,
            ScriptStage::FixedUpdate,
        )?;
        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    /// Dispatch physics collision/trigger events into script callbacks.
    pub fn handle_physics_events(
        &mut self,
        events: &[PhysicsEvent],
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        for event in events {
            let (entity, other, is_trigger, started) = match event {
                PhysicsEvent::CollisionEnter { a, b } => (*a, *b, false, true),
                PhysicsEvent::CollisionExit { a, b } => (*a, *b, false, false),
                PhysicsEvent::TriggerEnter { a, b } => (*a, *b, true, true),
                PhysicsEvent::TriggerExit { a, b } => (*a, *b, true, false),
            };

            self.run_event(entity, other, is_trigger, started, world, physics, input)?;
            self.run_event(other, entity, is_trigger, started, world, physics, input)?;
        }

        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    fn run_event(
        &mut self,
        entity: EntityId,
        other: EntityId,
        is_trigger: bool,
        started: bool,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let key_filter: Vec<_> = self
            .instances
            .keys()
            .filter(|k| k.entity == entity)
            .cloned()
            .collect();

        for key in key_filter {
            if let Some(instance) = self.instances.get_mut(&key) {
                let ctx = ScriptSelf::new(
                    entity,
                    world,
                    physics,
                    input,
                    Arc::clone(&self.command_buffer),
                    0.0,
                    0.0,
                );
                let function_name = match (is_trigger, started) {
                    (false, true) => "on_collision_enter",
                    (false, false) => "on_collision_exit",
                    (true, true) => "on_trigger_enter",
                    (true, false) => "on_trigger_exit",
                };
                let globals = self.lua.globals();
                self.call_script_fn(&globals, function_name, (ctx, other.to_u32() as i64))?;
            }
        }

        Ok(())
    }

    fn sync_instances(
        &mut self,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let mut desired = Vec::new();
        let mut pairs = world.query::<ScriptComponent>();
        pairs.sort_by_key(|(entity, _)| entity.to_u32());

        for (entity, scripts) in pairs {
            for (slot, attachment) in scripts.scripts.iter().enumerate() {
                let key = ScriptInstanceKey {
                    entity,
                    slot: slot as u32,
                };
                desired.push(key);

                self.load_module(&attachment.path)?;
                let module_modified = self.modules[&attachment.path].modified;

                if !self.instances.contains_key(&key) {
                    let module = &self.modules[&attachment.path];
                    // Set up params in globals for this script
                    let globals = self.lua.globals();
                    let params_table = self.lua.create_table()?;
                    for (k, v) in &attachment.params.values {
                        match v {
                            ScriptValue::Number(n) => params_table.set(k.as_str(), *n)?,
                            ScriptValue::Bool(b) => params_table.set(k.as_str(), *b)?,
                            ScriptValue::Text(s) => params_table.set(k.as_str(), s.as_str())?,
                            ScriptValue::Vec2(v) => params_table.set(k.as_str(), v.clone())?,
                        }
                    }
                    globals.set("params", params_table)?;
                    
                    self.instances.insert(
                        key,
                        ScriptInstance::new(
                            key,
                            attachment.path.clone(),
                            &attachment.params,
                            module,
                        ),
                    );
                }

                let needs_reload = {
                    let entry = self.instances.get(&key).expect("entry just inserted");
                    self.hot_reload && module_modified != entry.last_loaded
                };

                if needs_reload {
                    if let Some(mut instance) = self.instances.remove(&key) {
                        self.run_destroy(&mut instance, world, physics, input)?;
                    }

                    let module = &self.modules[&attachment.path];
                    // Set up params in globals
                    let globals = self.lua.globals();
                    let params_table = self.lua.create_table()?;
                    for (k, v) in &attachment.params.values {
                        match v {
                            ScriptValue::Number(n) => params_table.set(k.as_str(), *n)?,
                            ScriptValue::Bool(b) => params_table.set(k.as_str(), *b)?,
                            ScriptValue::Text(s) => params_table.set(k.as_str(), s.as_str())?,
                            ScriptValue::Vec2(v) => params_table.set(k.as_str(), v.clone())?,
                        }
                    }
                    globals.set("params", params_table)?;
                    
                    self.instances.insert(
                        key,
                        ScriptInstance::new(
                            key,
                            attachment.path.clone(),
                            &attachment.params,
                            module,
                        ),
                    );
                }

                if let Some(mut instance) = self.instances.remove(&key) {
                    if !instance.has_started {
                        // Execute the script to load functions into globals
                        let module = &self.modules[&instance.script_path];
                        eprintln!("[Script] Executing script for instance: {}", instance.script_path);
                        let chunk = self.lua.load(&module.source).set_name(&instance.script_path);
                        if let Err(e) = chunk.exec() {
                            eprintln!("[Script] Error executing script {}: {}", instance.script_path, e);
                            return Err(anyhow!("Failed to execute script: {}", e));
                        }
                        eprintln!("[Script] Script executed successfully");
                        
                        // Verify functions are in globals (drop the reference before mutable borrow)
                        {
                            let globals = self.lua.globals();
                            if globals.get::<_, mlua::Function>("on_fixed_update").is_ok() {
                                eprintln!("[Script] on_fixed_update found in globals");
                            } else {
                                eprintln!("[Script] WARNING: on_fixed_update NOT found in globals after execution!");
                            }
                        }
                        
                        self.run_create_and_start(&mut instance, world, physics, input)?;
                    }

                    self.instances.insert(key, instance);
                }
            }
        }

        let existing: Vec<_> = self.instances.keys().cloned().collect();
        for key in existing {
            if !desired.contains(&key) {
                if let Some(mut inst) = self.instances.remove(&key) {
                    self.run_destroy(&mut inst, world, physics, input)?;
                }
            }
        }

        Ok(())
    }

    fn run_stage(
        &mut self,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
        dt: f32,
        fixed_dt: f32,
        stage: ScriptStage,
    ) -> Result<()> {
        for instance in self.instances.values() {
            // Re-execute the script to ensure functions are in globals
            // This is needed because functions might not persist between calls
            let module = &self.modules[&instance.script_path];
            let chunk = self.lua.load(&module.source).set_name(&instance.script_path);
            if let Err(e) = chunk.exec() {
                eprintln!("[Script] Error re-executing script {}: {}", instance.script_path, e);
                continue;
            }
            
            let ctx = ScriptSelf::new(
                instance.key.entity,
                world,
                physics,
                input,
                Arc::clone(&self.command_buffer),
                dt,
                fixed_dt,
            );

            let (fn_name, include_dt) = match stage {
                ScriptStage::Update => ("on_update", true),
                ScriptStage::FixedUpdate => ("on_fixed_update", true),
                ScriptStage::Draw => ("on_draw", false),
            };

            let globals = self.lua.globals();
            if include_dt {
                self.call_script_fn(
                    &globals,
                    fn_name,
                    (ctx, if stage == ScriptStage::Update { dt } else { fixed_dt }),
                )?;
            } else {
                self.call_script_fn(&globals, fn_name, (ctx,))?;
            }
        }
        Ok(())
    }

    fn run_create_and_start(
        &mut self,
        instance: &mut ScriptInstance,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        // Script should already be executed in sync_instances
        let globals = self.lua.globals();
        let ctx = ScriptSelf::new(
            instance.key.entity,
            world,
            physics,
            input,
            Arc::clone(&self.command_buffer),
            0.0,
            0.0,
        );

        // Check if functions exist before calling
        if globals.get::<_, mlua::Function>("on_create").is_ok() {
            self.call_script_fn(&globals, "on_create", (ctx.clone(),))?;
        }
        if globals.get::<_, mlua::Function>("on_start").is_ok() {
            self.call_script_fn(&globals, "on_start", (ctx,))?;
        }

        instance.has_started = true;
        Ok(())
    }

    fn run_destroy(
        &mut self,
        instance: &mut ScriptInstance,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let globals = self.lua.globals();
        let ctx = ScriptSelf::new(
            instance.key.entity,
            world,
            physics,
            input,
            Arc::clone(&self.command_buffer),
            0.0,
            0.0,
        );

        self.call_script_fn(&globals, "on_destroy", (ctx,))?;

        Ok(())
    }

    fn load_module(&mut self, path: &str) -> Result<()> {
        if !self.hot_reload && self.modules.contains_key(path) {
            return Ok(());
        }

        let contents = fs::read_to_string(Path::new(path))
            .map_err(|err| anyhow!("Failed to load script {path}: {err}"))?;

        let modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
        self.modules
            .insert(path.to_string(), ScriptModule { source: contents, modified });
        Ok(())
    }

    fn call_script_fn<'lua, A>(
        &'lua self,
        globals: &mlua::Table<'lua>,
        name: &str,
        args: A,
    ) -> Result<()>
    where
        A: mlua::IntoLuaMulti<'lua>,
    {
        match globals.get::<_, mlua::Function<'lua>>(name) {
            Ok(func) => {
                if let Err(e) = func.call::<_, ()>(args) {
                    eprintln!("[Script] Error calling {}: {}", name, e);
                    return Err(anyhow!("Lua error in {}: {}", name, e));
                }
                Ok(())
            }
            Err(e) => {
                // Function doesn't exist, which is OK for optional callbacks
                // Only log if it's not a "key not found" type error
                if !e.to_string().contains("bad argument") {
                    // This is expected for optional callbacks, so we don't error
                }
                Ok(())
            }
        }
    }
}

#[derive(PartialEq)]
enum ScriptStage {
    Update,
    FixedUpdate,
    Draw,
}

fn parse_key(name: &str) -> Option<winit::keyboard::KeyCode> {
    use winit::keyboard::KeyCode;

    match name {
        "W" | "w" => Some(KeyCode::KeyW),
        "A" | "a" => Some(KeyCode::KeyA),
        "S" | "s" => Some(KeyCode::KeyS),
        "D" | "d" => Some(KeyCode::KeyD),
        "Space" | "space" => Some(KeyCode::Space),
        "Left" | "left" => Some(KeyCode::ArrowLeft),
        "Right" | "right" => Some(KeyCode::ArrowRight),
        "Up" | "up" => Some(KeyCode::ArrowUp),
        "Down" | "down" => Some(KeyCode::ArrowDown),
        _ => None,
    }
}
