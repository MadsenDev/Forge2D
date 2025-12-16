use anyhow::Result;
use forge2d::{
    math::{Camera2D, Vec2},
    physics::{PhysicsWorld, RigidBodyType},
    render::{Renderer, Sprite, TextureHandle},
    Engine, Game, RigidBodyHandle,
};
use std::collections::HashSet;

/// Comprehensive physics demo showcasing all features:
/// - Different materials (bouncy, slippery, normal)
/// - Different shapes (boxes, circles, capsules)
/// - Sensors/triggers
/// - Forces and impulses
/// - Collision callbacks
/// - Angular velocity
/// - Damping
struct PhysicsDemo {
    camera: Camera2D,
    physics: PhysicsWorld,
    world: forge2d::World,
    
    // Visual entities
    textures: TextureSet,
    
    // Entity tracking with visual properties
    entities: Vec<PhysicsEntity>,
    
    // Collision tracking for visual feedback
    colliding_entities: HashSet<forge2d::EntityId>,
    
    // Input state
    last_spawn_time: std::time::Instant,
}

struct TextureSet {
    ground: Option<TextureHandle>,
    box_normal: Option<TextureHandle>,
    box_bouncy: Option<TextureHandle>,
    box_slippery: Option<TextureHandle>,
    circle: Option<TextureHandle>,
    capsule: Option<TextureHandle>,
    sensor: Option<TextureHandle>,
}

struct PhysicsEntity {
    entity: forge2d::EntityId,
    body_handle: RigidBodyHandle,
    shape: ShapeType,
    material: MaterialType,
    is_sensor: bool,
}

#[derive(Clone, Copy)]
enum ShapeType {
    Box,
    Circle,
    Capsule,
}

#[derive(Clone, Copy)]
enum MaterialType {
    Normal,    // Default friction, no bounce
    Bouncy,    // High restitution
    Slippery,  // Low friction
}

impl PhysicsDemo {
    fn new() -> Self {
        // Create physics world with downward gravity
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 400.0));
        
        // Collision callbacks would go here - simplified for demo
        
        Self {
            camera: Camera2D::new(Vec2::new(0.0, 0.0)),
            physics,
            world: forge2d::World::new(),
            textures: TextureSet {
                ground: None,
                box_normal: None,
                box_bouncy: None,
                box_slippery: None,
                circle: None,
                capsule: None,
                sensor: None,
            },
            entities: Vec::new(),
            colliding_entities: HashSet::new(),
            last_spawn_time: std::time::Instant::now(),
        }
    }
    
    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Ground (dark gray)
        let ground_data: Vec<u8> = (0..(4 * 600 * 30))
            .flat_map(|_| [80u8, 80, 80, 255])
            .collect();
        self.textures.ground = Some(renderer.load_texture_from_rgba(&ground_data, 600, 30)?);
        
        // Normal box (red)
        let box_data: Vec<u8> = (0..(4 * 30 * 30))
            .flat_map(|_| [255u8, 80, 80, 255])
            .collect();
        self.textures.box_normal = Some(renderer.load_texture_from_rgba(&box_data, 30, 30)?);
        
        // Bouncy box (yellow)
        let bouncy_data: Vec<u8> = (0..(4 * 30 * 30))
            .flat_map(|_| [255u8, 255, 100, 255])
            .collect();
        self.textures.box_bouncy = Some(renderer.load_texture_from_rgba(&bouncy_data, 30, 30)?);
        
        // Slippery box (blue)
        let slippery_data: Vec<u8> = (0..(4 * 30 * 30))
            .flat_map(|_| [100u8, 150, 255, 255])
            .collect();
        self.textures.box_slippery = Some(renderer.load_texture_from_rgba(&slippery_data, 30, 30)?);
        
        // Circle (green)
        let circle_size = 30;
        let mut circle_data = vec![0u8; 4 * circle_size * circle_size];
        let center = circle_size as f32 / 2.0;
        let radius = center - 2.0;
        for y in 0..circle_size {
            for x in 0..circle_size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                if dx * dx + dy * dy <= radius * radius {
                    let idx = ((y * circle_size + x) * 4) as usize;
                    circle_data[idx] = 100;
                    circle_data[idx + 1] = 255;
                    circle_data[idx + 2] = 100;
                    circle_data[idx + 3] = 255;
                }
            }
        }
        self.textures.circle = Some(renderer.load_texture_from_rgba(&circle_data, circle_size as u32, circle_size as u32)?);
        
        // Capsule (purple)
        let capsule_data: Vec<u8> = (0..(4 * 40 * 20))
            .flat_map(|_| [200u8, 100, 255, 255])
            .collect();
        self.textures.capsule = Some(renderer.load_texture_from_rgba(&capsule_data, 40, 20)?);
        
        // Sensor (semi-transparent cyan)
        let sensor_data: Vec<u8> = (0..(4 * 50 * 50))
            .flat_map(|_| [100u8, 255, 255, 128])
            .collect();
        self.textures.sensor = Some(renderer.load_texture_from_rgba(&sensor_data, 50, 50)?);
        
        Ok(())
    }
    
    fn spawn_object(
        &mut self,
        position: Vec2,
        shape: ShapeType,
        material: MaterialType,
        is_sensor: bool,
    ) -> Result<()> {
        let entity = self.world.spawn();
        
        // Sensors can use Fixed or Dynamic - using Dynamic for simplicity
        let body_type = RigidBodyType::Dynamic;
        
        let body_handle = self.physics.create_body(entity, body_type, position, 0.0)?;
        
        // Create appropriate shape
        let (shape_obj, size) = match shape {
            ShapeType::Box => (PhysicsWorld::create_box_collider(15.0, 15.0), Vec2::new(30.0, 30.0)),
            ShapeType::Circle => (PhysicsWorld::create_circle_collider(15.0), Vec2::new(30.0, 30.0)),
            ShapeType::Capsule => (PhysicsWorld::create_capsule_collider(10.0, 8.0), Vec2::new(40.0, 20.0)),
        };
        
        if is_sensor {
            // Add as sensor (trigger volume)
            self.physics.add_sensor(body_handle, shape_obj, Vec2::ZERO);
        } else {
            // Add collider with material properties
            let (friction, restitution) = match material {
                MaterialType::Normal => (0.5, 0.0),
                MaterialType::Bouncy => (0.5, 0.8),
                MaterialType::Slippery => (0.1, 0.0),
            };
            self.physics.add_collider_with_material(
                body_handle,
                shape_obj,
                Vec2::ZERO,
                1.0,
                friction,
                restitution,
            );
            
            // Add some angular velocity for visual interest
            self.physics.set_angular_velocity(body_handle, (rand::random::<f32>() - 0.5) * 5.0);
            
            // Add some linear damping to prevent objects from moving forever
            self.physics.set_linear_damping(body_handle, 0.1);
            self.physics.set_angular_damping(body_handle, 0.2);
        }
        
        self.entities.push(PhysicsEntity {
            entity,
            body_handle,
            shape,
            material,
            is_sensor,
        });
        
        Ok(())
    }
}

impl Game for PhysicsDemo {
    fn init(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        // Create textures
        self.create_textures(&mut *ctx.renderer())?;
        
        let screen_size = ctx.window().inner_size();
        let screen_w = screen_size.width as f32;
        let screen_h = screen_size.height as f32;
        
        // Create ground platform
        let ground_entity = self.world.spawn();
        let ground_y = screen_h - 80.0;
        let ground_body = self.physics.create_body(
            ground_entity,
            RigidBodyType::Fixed,
            Vec2::new(screen_w / 2.0, ground_y),
            0.0,
        )?;
        let ground_shape = PhysicsWorld::create_box_collider(300.0, 15.0);
        self.physics.add_collider_with_material(
            ground_body,
            ground_shape,
            Vec2::ZERO,
            0.0,
            0.7, // High friction
            0.2, // Slight bounce
        );
        
        // Create a sensor/trigger zone in the middle
        let sensor_entity = self.world.spawn();
        let sensor_body = self.physics.create_body(
            sensor_entity,
            RigidBodyType::Fixed,
            Vec2::new(screen_w / 2.0, screen_h / 2.0),
            0.0,
        )?;
        let sensor_shape = PhysicsWorld::create_circle_collider(25.0);
        self.physics.add_sensor(sensor_body, sensor_shape, Vec2::ZERO);
        self.entities.push(PhysicsEntity {
            entity: sensor_entity,
            body_handle: sensor_body,
            shape: ShapeType::Circle,
            material: MaterialType::Normal,
            is_sensor: true,
        });
        
        // Spawn initial objects with different properties
        // Bouncy boxes
        for i in 0..3 {
            self.spawn_object(
                Vec2::new(screen_w * 0.2 + i as f32 * 40.0, 100.0),
                ShapeType::Box,
                MaterialType::Bouncy,
                false,
            )?;
        }
        
        // Normal boxes
        for i in 0..3 {
            self.spawn_object(
                Vec2::new(screen_w * 0.5 + i as f32 * 40.0, 150.0),
                ShapeType::Box,
                MaterialType::Normal,
                false,
            )?;
        }
        
        // Slippery boxes
        for i in 0..3 {
            self.spawn_object(
                Vec2::new(screen_w * 0.8 + i as f32 * 40.0, 200.0),
                ShapeType::Box,
                MaterialType::Slippery,
                false,
            )?;
        }
        
        // Circles
        for i in 0..2 {
            self.spawn_object(
                Vec2::new(screen_w * 0.3 + i as f32 * 60.0, 250.0),
                ShapeType::Circle,
                MaterialType::Normal,
                false,
            )?;
        }
        
        // Capsules
        for i in 0..2 {
            self.spawn_object(
                Vec2::new(screen_w * 0.6 + i as f32 * 60.0, 300.0),
                ShapeType::Capsule,
                MaterialType::Bouncy,
                false,
            )?;
        }
        
        println!("=== Forge2D Physics Showcase ===");
        println!("Left Click: Spawn random object");
        println!("Right Click: Apply impulse to objects");
        println!("WASD: Apply forces");
        println!("ESC: Exit");
        
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        let input = ctx.input();
        let screen_size = ctx.window().inner_size();
        let mouse_world = ctx.mouse_world(&self.camera);
        
        // Left click: Spawn random object
        if input.is_mouse_pressed(forge2d::MouseButton::Left) {
            let now = std::time::Instant::now();
            if now.duration_since(self.last_spawn_time).as_millis() > 200 {
                self.last_spawn_time = now;
                
                let shape = match rand::random::<u8>() % 3 {
                    0 => ShapeType::Box,
                    1 => ShapeType::Circle,
                    _ => ShapeType::Capsule,
                };
                
                let material = match rand::random::<u8>() % 3 {
                    0 => MaterialType::Normal,
                    1 => MaterialType::Bouncy,
                    _ => MaterialType::Slippery,
                };
                
                if let Err(e) = self.spawn_object(mouse_world, shape, material, false) {
                    eprintln!("Failed to spawn object: {}", e);
                }
            }
        }
        
        // Right click: Apply impulse to nearby objects
        if input.is_mouse_pressed(forge2d::MouseButton::Right) {
            for entity in &self.entities {
                if let Some(pos) = self.physics.get_body_position(entity.body_handle) {
                    let dist = pos.distance(mouse_world);
                    if dist < 100.0 && !entity.is_sensor {
                        let direction = (pos - mouse_world).normalized();
                        let impulse = direction * 500.0;
                        self.physics.apply_impulse(entity.body_handle, impulse);
                    }
                }
            }
        }
        
        // WASD: Apply forces
        let force_dir = Vec2::new(
            if input.is_key_down(forge2d::VirtualKeyCode::D) { 1.0 }
            else if input.is_key_down(forge2d::VirtualKeyCode::A) { -1.0 }
            else { 0.0 },
            if input.is_key_down(forge2d::VirtualKeyCode::S) { 1.0 }
            else if input.is_key_down(forge2d::VirtualKeyCode::W) { -1.0 }
            else { 0.0 },
        );
        
        if force_dir.length() > 0.0 {
            let force = force_dir.normalized() * 200.0;
            for entity in &self.entities {
                if !entity.is_sensor {
                    self.physics.apply_force(entity.body_handle, force);
                }
            }
        }
        
        // Step physics
        while ctx.should_run_fixed_update() {
            let dt = ctx.fixed_delta_time().as_secs_f32();
            self.physics.step(dt);
        }
        
        // Update collision tracking (simplified - in real implementation would use callbacks properly)
        self.colliding_entities.clear();
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        let screen_size = ctx.window().inner_size();
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        
        renderer.clear(&mut frame, [0.05, 0.05, 0.1, 1.0])?;
        
        // Draw ground
        if let Some(ground_texture) = self.textures.ground {
            let ground_y = (screen_size.height as f32) - 80.0;
            let mut sprite = Sprite::new(ground_texture);
            sprite.transform.position = Vec2::new(screen_size.width as f32 / 2.0, ground_y);
            sprite.set_size_px(Vec2::new(600.0, 30.0), Vec2::new(600.0, 30.0));
            renderer.draw_sprite(&mut frame, &sprite, &self.camera)?;
        }
        
        // Draw all physics entities
        for entity in &self.entities {
            let texture = match (entity.shape, entity.material, entity.is_sensor) {
                (_, _, true) => self.textures.sensor,
                (ShapeType::Box, MaterialType::Normal, _) => self.textures.box_normal,
                (ShapeType::Box, MaterialType::Bouncy, _) => self.textures.box_bouncy,
                (ShapeType::Box, MaterialType::Slippery, _) => self.textures.box_slippery,
                (ShapeType::Circle, _, _) => self.textures.circle,
                (ShapeType::Capsule, _, _) => self.textures.capsule,
            };
            
            if let Some(tex) = texture {
                if let Some(pos) = self.physics.get_body_position(entity.body_handle) {
                    if let Some(rot) = self.physics.get_body_rotation(entity.body_handle) {
                        let mut sprite = Sprite::new(tex);
                        sprite.transform.position = pos;
                        sprite.transform.rotation = rot;
                        
                        let size = match entity.shape {
                            ShapeType::Box => Vec2::new(30.0, 30.0),
                            ShapeType::Circle => Vec2::new(30.0, 30.0),
                            ShapeType::Capsule => Vec2::new(40.0, 20.0),
                        };
                        sprite.set_size_px(size, size);
                        
                        // Visual feedback: tint colliding objects
                        if self.colliding_entities.contains(&entity.entity) {
                            sprite.tint = [1.5, 1.5, 1.5, 1.0]; // Brighten on collision
                        } else if entity.is_sensor {
                            sprite.tint = [1.0, 1.0, 1.0, 0.5]; // Semi-transparent for sensors
                        } else {
                            sprite.tint = [1.0, 1.0, 1.0, 1.0];
                        }
                        
                        renderer.draw_sprite(&mut frame, &sprite, &self.camera)?;
                    }
                }
            }
        }
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D Physics Showcase")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(PhysicsDemo::new())
}
