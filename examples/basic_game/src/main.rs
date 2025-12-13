use std::time::Duration;

use anyhow::Result;
use forge2d::{
    Camera2D, Engine, EngineContext, Game, MouseButton, Sprite, Vec2, VirtualKeyCode,
};

// Embedded texture: neutral white square (32x32). We tint per-sprite.
const RED_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x73, 0x7a, 0x7a, 0xf4, 0x00, 0x00, 0x00,
    0x2f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0xed, 0xce, 0x31, 0x01, 0x00,
    0x00, 0x08, 0xc3, 0xb0, 0x81, 0x7f, 0xcf, 0x43, 0x06, 0x4f, 0x6a, 0xa0,
    0x99, 0xb6, 0xcd, 0x63, 0xfb, 0x39, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x92, 0x03,
    0x4d, 0x88, 0x04, 0x3c, 0x4a, 0xbd, 0x9d, 0x15, 0x00, 0x00, 0x00, 0x00,
    0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

// Reuse same bytes for simplicity; actual color comes from tint.
const BLUE_PNG: &[u8] = RED_PNG;
const GREEN_PNG: &[u8] = RED_PNG;

struct Collectible {
    sprite: Sprite,
    rotation: f32,
    rotation_speed: f32,
}

struct BasicGame {
    // Player
    player: Option<Sprite>,
    player_speed: f32,
    
    // Camera
    camera: Camera2D,
    
    // World bounds and background
    world_bounds: Vec2,
    background_tiles: Vec<Sprite>,
    walls: Vec<Sprite>,
    
    // Collectibles
    collectibles: Vec<Collectible>,
    
    // Enemies (bouncing sprites)
    enemies: Vec<Sprite>,
    enemy_velocities: Vec<Vec2>,
    
    // Mouse click positions (for spawning)
    click_positions: Vec<Vec2>,
    
    // Game state
    score: u32,
    last_collectible_spawn: Duration,
}

impl Game for BasicGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Load textures using AssetManager (demonstrates caching)
        let red_texture = ctx.load_texture_from_bytes("red_square", RED_PNG)?;
        let blue_texture = ctx.load_texture_from_bytes("blue_square", BLUE_PNG)?;
        let green_texture = ctx.load_texture_from_bytes("green_square", GREEN_PNG)?;
        
        // Try loading red again - should use cache!
        let _cached_red = ctx.load_texture_from_bytes("red_square", RED_PNG)?;
        assert_eq!(red_texture, _cached_red); // Same handle = cached!
        
        // Initialize player (blue square with blue tint)
        let mut player = Sprite::new(blue_texture);
        player.transform.scale = Vec2::new(24.0, 24.0);
        player.tint = [0.3, 0.5, 1.0, 1.0]; // Blue tint
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        player.transform.position = Vec2::new(screen_w as f32 * 0.5, screen_h as f32 * 0.5);
        let player_start = player.transform.position;
        
        // Initialize camera so that the player is centered on screen (no zoom compensation)
        self.camera = Camera2D::new(Vec2::new(
            player.transform.position.x - (screen_w as f32 * 0.5),
            player.transform.position.y - (screen_h as f32 * 0.5),
        ));
        self.player = Some(player);

        // World bounds (for reference and clamping)
        self.world_bounds = Vec2::new(1000.0, 700.0);

        // Background tiles (grid around player start so you see reference immediately)
        self.background_tiles.clear();
        let tile_scale = Vec2::new(96.0, 96.0);
        for ix in -5..=5 {
            for iy in -4..=4 {
                let mut tile = Sprite::new(red_texture);
                tile.transform.scale = tile_scale;
                tile.transform.position = Vec2::new(
                    player_start.x + ix as f32 * tile_scale.x,
                    player_start.y + iy as f32 * tile_scale.y,
                );
                tile.tint = [0.2, 0.22, 0.28, 1.0]; // darker grid
                self.background_tiles.push(tile);
            }
        }

        // World boundary walls
        self.walls.clear();
        let wall_thickness = 32.0;
        let mut make_wall = |x: f32, y: f32, w: f32, h: f32| {
            let mut wall = Sprite::new(red_texture);
            wall.transform.scale = Vec2::new(w, h);
            wall.transform.position = Vec2::new(x, y);
            wall.tint = [0.2, 0.2, 0.2, 1.0];
            self.walls.push(wall);
        };
        // Top, bottom, left, right walls
        make_wall(0.0, 0.0, self.world_bounds.x, wall_thickness);
        make_wall(0.0, self.world_bounds.y - wall_thickness, self.world_bounds.x, wall_thickness);
        make_wall(0.0, 0.0, wall_thickness, self.world_bounds.y);
        make_wall(self.world_bounds.x - wall_thickness, 0.0, wall_thickness, self.world_bounds.y);
        // Moderate zoom so entities remain visible
        self.camera.zoom = 0.35;
        
        // Spawn initial collectibles (green squares)
        let collectible_texture = green_texture;
        for i in 0..5 {
            let mut sprite = Sprite::new(collectible_texture);
            sprite.transform.scale = Vec2::new(48.0, 48.0);
            sprite.transform.position = Vec2::new(
                screen_w as f32 * 0.3 + i as f32 * 80.0,
                screen_h as f32 * 0.4 + i as f32 * 50.0,
            );
            sprite.tint = [0.3, 1.0, 0.3, 1.0]; // Green tint
            
            self.collectibles.push(Collectible {
                sprite,
                rotation: i as f32 * 0.5,
                rotation_speed: 1.0 + i as f32 * 0.2,
            });
        }
        
        // Spawn initial enemies (red squares)
        let enemy_texture = red_texture;
        for i in 0..3 {
            let mut sprite = Sprite::new(enemy_texture);
            sprite.transform.scale = Vec2::new(56.0, 56.0);
            sprite.transform.position = Vec2::new(
                screen_w as f32 * 0.7 + i as f32 * 90.0,
                screen_h as f32 * 0.6 + i as f32 * 70.0,
            );
            sprite.tint = [1.0, 0.5, 0.5, 1.0]; // Slightly tinted
            
            self.enemies.push(sprite);
            self.enemy_velocities.push(Vec2::new(
                50.0 + i as f32 * 20.0,
                40.0 + i as f32 * 15.0,
            ));
        }
        
        // Check audio availability
        if ctx.audio().is_available() {
            println!("Audio system is available!");
        } else {
            println!("Audio system is not available (this is okay)");
        }
        
        println!("=== Forge2D Demo Game ===");
        println!("WASD/Arrow Keys: Move player");
        println!("Mouse Click: Spawn collectible at cursor");
        println!("ESC: Exit");
        println!("Collect green squares for points!");
        
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        
        // Exit on ESC
        if ctx.input().is_key_pressed(VirtualKeyCode::Escape) {
            ctx.request_exit();
        }
        
        // Player movement with WASD or Arrow Keys
        let mut move_dir = Vec2::ZERO;
        
        if ctx.input().is_key_down(VirtualKeyCode::W) || ctx.input().is_key_down(VirtualKeyCode::Up) {
            move_dir.y -= 1.0;
        }
        if ctx.input().is_key_down(VirtualKeyCode::S) || ctx.input().is_key_down(VirtualKeyCode::Down) {
            move_dir.y += 1.0;
        }
        if ctx.input().is_key_down(VirtualKeyCode::A) || ctx.input().is_key_down(VirtualKeyCode::Left) {
            move_dir.x -= 1.0;
        }
        if ctx.input().is_key_down(VirtualKeyCode::D) || ctx.input().is_key_down(VirtualKeyCode::Right) {
            move_dir.x += 1.0;
        }
        
        // Normalize movement direction for consistent speed
        if let Some(player) = self.player.as_mut() {
            if move_dir.length_squared() > 0.0 {
                move_dir = move_dir.normalized();
                player.transform.position += move_dir * self.player_speed * dt;
            }
            // Clamp player to world bounds (minus sprite size)
            let size = player.transform.scale;
            player.transform.position.x = player.transform.position.x.clamp(0.0, self.world_bounds.x - size.x);
            player.transform.position.y = player.transform.position.y.clamp(0.0, self.world_bounds.y - size.y);
        }
        
        // Mouse click to spawn collectible
        if ctx.input().is_mouse_pressed(MouseButton::Left) {
            let mouse_pos = ctx.input().mouse_position_vec2();
            self.click_positions.push(mouse_pos);
            
            // Spawn a new collectible at mouse position
            if let Some(green_texture) = ctx.assets().get_texture("green_square") {
                let mut sprite = Sprite::new(green_texture);
                sprite.transform.scale = Vec2::new(32.0, 32.0);
                sprite.transform.position = mouse_pos;
                sprite.tint = [0.3, 1.0, 0.3, 1.0]; // Green tint
                
                self.collectibles.push(Collectible {
                    sprite,
                    rotation: 0.0,
                    rotation_speed: 2.0,
                });
            }
        }
        
        // Update camera to follow player (smoothly), keeping player near center
        if let Some(player) = &self.player {
            let (screen_w, screen_h) = ctx.renderer().surface_size();
            let target_pos = Vec2::new(
                player.transform.position.x - (screen_w as f32 * 0.5),
                player.transform.position.y - (screen_h as f32 * 0.5),
            );
            let camera_speed = 5.0;
            self.camera.position = self.camera.position.lerp(target_pos, camera_speed * dt);
        }
        
        // Update collectibles (rotation animation)
        for collectible in &mut self.collectibles {
            collectible.rotation += collectible.rotation_speed * dt;
            collectible.sprite.transform.rotation = collectible.rotation;
        }
        
        // Check collisions: player vs collectibles
        let Some(player) = &self.player else {
            return Ok(());
        };
        let player_pos = player.transform.position;
        let player_size = player.transform.scale;
        let player_radius = player_size.length() * 0.5;
        
        self.collectibles.retain_mut(|collectible| {
            let collectible_pos = collectible.sprite.transform.position;
            let collectible_size = collectible.sprite.transform.scale;
            let collectible_radius = collectible_size.length() * 0.5;
            
            let distance = player_pos.distance(collectible_pos);
            if distance < player_radius + collectible_radius {
                self.score += 10;
                println!("Score: {} (+10)", self.score);
                false // Remove collectible
            } else {
                true // Keep collectible
            }
        });
        
        // Update enemies (bouncing movement)
        let bounds = self.world_bounds;
        
        for (enemy, velocity) in self.enemies.iter_mut().zip(self.enemy_velocities.iter_mut()) {
            enemy.transform.position += *velocity * dt;
            
            let size = enemy.transform.scale;
            let pos = &mut enemy.transform.position;
            
            // Bounce off walls
            if pos.x < 0.0 || pos.x + size.x > bounds.x {
                velocity.x = -velocity.x;
                pos.x = pos.x.max(0.0).min(bounds.x - size.x);
            }
            if pos.y < 0.0 || pos.y + size.y > bounds.y {
                velocity.y = -velocity.y;
                pos.y = pos.y.max(0.0).min(bounds.y - size.y);
            }
        }
        
        // Spawn new collectibles periodically
        if ctx.elapsed_time() - self.last_collectible_spawn > Duration::from_secs(3) {
            self.last_collectible_spawn = ctx.elapsed_time();
            
            if let Some(green_texture) = ctx.assets().get_texture("green_square") {
                let mut sprite = Sprite::new(green_texture);
                sprite.transform.scale = Vec2::new(32.0, 32.0);
                sprite.transform.position = Vec2::new(
                    (self.score as f32 * 50.0) % bounds.x,
                    (self.score as f32 * 70.0) % bounds.y,
                );
                sprite.tint = [0.3, 1.0, 0.3, 1.0]; // Green tint
                
                self.collectibles.push(Collectible {
                    sprite,
                    rotation: 0.0,
                    rotation_speed: 1.5,
                });
            }
        }
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        
        // Clear with a nice dark blue background
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;
        
        // Draw background tiles
        for tile in &self.background_tiles {
            renderer.draw_sprite(&mut frame, tile, &self.camera)?;
        }

        // Draw walls (world bounds)
        for wall in &self.walls {
            renderer.draw_sprite(&mut frame, wall, &self.camera)?;
        }

        // Draw collectibles
        for collectible in &self.collectibles {
            renderer.draw_sprite(&mut frame, &collectible.sprite, &self.camera)?;
        }
        
        // Draw enemies
        for enemy in &self.enemies {
            renderer.draw_sprite(&mut frame, enemy, &self.camera)?;
        }
        
        // Draw player (on top)
        if let Some(player) = &self.player {
            renderer.draw_sprite(&mut frame, player, &self.camera)?;
        }
        
        // Draw click indicators (small white dots)
        // Note: Could be improved with particle effects or sprites
        let _click_count = self.click_positions.len();
        
        // Clean up old click positions (keep last 10)
        if self.click_positions.len() > 10 {
            self.click_positions.remove(0);
        }
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D Demo - Collect the Green Squares!")
        .with_size(1024, 768)
        .with_vsync(true)
        .run(BasicGame {
            player: None, // Will be initialized in init()
            player_speed: 200.0,
            camera: Camera2D::default(),
            world_bounds: Vec2::ZERO,
            background_tiles: Vec::new(),
            walls: Vec::new(),
            collectibles: Vec::new(),
            enemies: Vec::new(),
            enemy_velocities: Vec::new(),
            click_positions: Vec::new(),
            score: 0,
            last_collectible_spawn: Duration::ZERO,
        })
}
