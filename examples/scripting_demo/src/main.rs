use anyhow::Result;
use std::path::Path;

use forge2d::{
    input::InputState,
    math::{Camera2D, Vec2},
    physics::{ColliderShape, PhysicsWorld, RigidBodyType},
    render::{Renderer, TextureHandle},
    script::{ScriptComponent, ScriptParams, ScriptRuntime, ScriptTag},
    Engine, EngineContext, Game, KeyCode, SpriteComponent, Transform, World,
};

struct ScriptingDemo {
    runtime: ScriptRuntime,
    world: World,
    physics: PhysicsWorld,
    camera: Camera2D,
    player: Option<forge2d::EntityId>,
    player_texture: Option<TextureHandle>,
    platform_texture: Option<TextureHandle>,
}

impl ScriptingDemo {
    fn new() -> Result<Self> {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 500.0)); // Stronger gravity for better feel
        
        Ok(Self {
            runtime: ScriptRuntime::new()?,
            world: World::new(),
            physics,
            camera: Camera2D::new(Vec2::ZERO),
            player: None,
            player_texture: None,
            platform_texture: None,
        })
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.player_texture = Some(self.solid_texture(renderer, 32, [255, 255, 255, 255])?);
        self.platform_texture = Some(self.solid_texture(renderer, 64, [70, 80, 95, 255])?);
        Ok(())
    }

    fn solid_texture(
        &self,
        renderer: &mut Renderer,
        size: u32,
        color: [u8; 4],
    ) -> Result<TextureHandle> {
        let data: Vec<u8> = (0..(size * size)).flat_map(|_| color).collect();
        renderer.load_texture_from_rgba(&data, size, size)
    }

    fn spawn_player(&mut self) -> Result<()> {
        // Start player higher up so they fall onto the ground platform
        let position = Vec2::new(240.0, 200.0);
        let entity = self.world.spawn();

        self.world.insert(entity, Transform::new(position));

        if let Some(texture) = self.player_texture {
            let mut sprite = SpriteComponent::new(texture);
            sprite
                .sprite
                .set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
            sprite.sprite.tint = [0.25, 0.75, 1.0, 1.0];
            self.world.insert(entity, sprite);
        }

        self.world.insert(entity, ScriptTag("player".into()));
        let params = ScriptParams::default()
            // Give the scripted controller enough speed and jump strength to feel responsive.
            .insert("speed", 200.0)
            .insert("jump", 300.0); // Increased jump to compensate for stronger gravity

        // Use an absolute path so the script loads correctly when running from the
        // example's package directory.
        let script_path = format!(
            "{}/scripts/scripting_demo_player.lua",
            env!("CARGO_MANIFEST_DIR")
        );

        println!(
            "[ScriptingDemo] Attaching script to player: {} (exists: {})",
            script_path,
            Path::new(&script_path).exists()
        );

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );

        self.physics
            .create_body(entity, RigidBodyType::Dynamic, position, 0.0)?;
        self.physics.lock_rotations(entity, true);
        // Lower damping for more responsive movement
        self.physics.set_linear_damping(entity, 0.1);
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box { hx: 16.0, hy: 16.0 },
            Vec2::ZERO,
            1.0,
            0.8,
            0.1,
        )?;

        self.player = Some(entity);
        Ok(())
    }

    fn spawn_platform(&mut self, center: Vec2, size: Vec2) -> Result<()> {
        let entity = self.world.spawn();
        
        // Calculate scale based on desired size and texture size (64x64)
        let texture_size = Vec2::new(64.0, 64.0);
        let scale = Vec2::new(size.x / texture_size.x, size.y / texture_size.y);
        
        // Create transform with position and calculated scale
        let mut transform = Transform::new(center);
        transform.scale = scale;
        self.world.insert(entity, transform);

        if let Some(texture) = self.platform_texture {
            let mut sprite = SpriteComponent::new(texture);
            // Set size using the scale we calculated
            sprite
                .sprite
                .set_size_px(Vec2::new(size.x, size.y), texture_size);
            sprite.sprite.tint = [0.3, 0.35, 0.4, 1.0];
            self.world.insert(entity, sprite);
        }

        self.physics
            .create_body(entity, RigidBodyType::Fixed, center, 0.0)?;
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box {
                hx: size.x * 0.5,
                hy: size.y * 0.5,
            },
            Vec2::ZERO,
            1.0,
            1.0,
            0.0,
        )?;

        Ok(())
    }

    fn sync_transforms_from_physics(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<Transform>()
            .into_iter()
            .map(|(e, _)| e)
            .collect();
        for entity in entities {
            if let Some(position) = self.physics.body_position(entity) {
                if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                    transform.position = position;
                }
            }

            if let Some(rotation) = self.physics.body_rotation(entity) {
                if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                    transform.rotation = rotation;
                }
            }
        }
    }

    fn update_sprite_transforms(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<SpriteComponent>()
            .into_iter()
            .map(|(e, _)| e)
            .collect();

        for entity in entities {
            if let (Some(transform), Some(sprite)) = (
                self.world.get::<Transform>(entity).cloned(),
                self.world.get_mut::<SpriteComponent>(entity),
            ) {
                sprite.sprite.transform.position = transform.position;
                sprite.sprite.transform.rotation = transform.rotation;
                sprite.sprite.transform.scale = transform.scale;
            }
        }
    }

    fn update_camera(&mut self) {
        // Temporarily disable camera follow to see if player is moving
        // if let Some(player) = self.player {
        //     if let Some(transform) = self.world.get::<Transform>(player) {
        //         // Camera position represents the center of the view, so align it with the
        //         // player's world position just like the other demos.
        //         self.camera.position = transform.position;
        //     }
        // }
        // Keep camera fixed for debugging
        self.camera.position = Vec2::new(480.0, 270.0); // Center of 960x540 screen
    }
}

impl ScriptingDemo {
    fn log_key_presses(&self, input: &InputState) {
        let keys = [
            (KeyCode::KeyA, "A"),
            (KeyCode::KeyD, "D"),
            (KeyCode::ArrowLeft, "Left"),
            (KeyCode::ArrowRight, "Right"),
            (KeyCode::Space, "Space"),
        ];

        for (key, label) in keys {
            if input.is_key_pressed(key) {
                //println!("[ScriptingDemo] Key pressed: {}", label);
            }
        }
    }
}

impl Game for ScriptingDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.create_textures(ctx.renderer())?;
        
        // Spawn a ground platform first (at the bottom of the screen)
        let screen_h = ctx.window().inner_size().height as f32;
        let screen_w = ctx.window().inner_size().width as f32;
        let ground_y = screen_h - 50.0;
        self.spawn_platform(Vec2::new(480.0, ground_y), Vec2::new(960.0, 50.0))?;
        
        // Spawn a wall on the right side for wall jumping (visible on screen)
        self.spawn_platform(Vec2::new(screen_w - 50.0, screen_h / 2.0), Vec2::new(50.0, screen_h - 100.0))?;
        
        self.spawn_player()?;
        self.spawn_platform(Vec2::new(300.0, 420.0), Vec2::new(240.0, 24.0))?;
        self.spawn_platform(Vec2::new(520.0, 520.0), Vec2::new(420.0, 24.0))?;
        self.camera.zoom = 1.25;

        // Ensure scripts are attached and their startup callbacks run before the first frame.
        // This guarantees that `on_start` side effects (logging, tint, etc.) happen even if the
        // first update is delayed by platform event scheduling.
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), 0.0)?;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), dt)?;

        while ctx.should_run_fixed_update() {
            let fixed_dt = ctx.fixed_delta_time().as_secs_f32();
            self.runtime
                .fixed_update(&mut self.world, &mut self.physics, ctx.input(), fixed_dt)?;

            self.physics.step(fixed_dt);
            let events = self.physics.drain_events();
            self.runtime.handle_physics_events(
                &events,
                &mut self.world,
                &mut self.physics,
                ctx.input(),
            )?;
            self.sync_transforms_from_physics();
        }

        self.update_sprite_transforms();
        self.update_camera();

        self.log_key_presses(ctx.input());

        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.08, 0.09, 0.12, 1.0])?;

        for (_, sprite) in self.world.query::<SpriteComponent>() {
            if sprite.visible {
                renderer.draw_sprite(&mut frame, &sprite.sprite, &self.camera)?;
            }
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D Scripting Demo")
        .with_size(960, 540)
        .run(ScriptingDemo::new()?)
}
