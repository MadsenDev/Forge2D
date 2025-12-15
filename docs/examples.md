# Examples

Code examples and common patterns for Forge2D.

## Basic Game Structure

```rust
use anyhow::Result;
use forge2d::{Engine, EngineContext, Game, Vec2, VirtualKeyCode};

struct MyGame {
    // Your game state
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Initialize game
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Update game logic
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Render game
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("My Game")
        .with_size(1280, 720)
        .run(MyGame {})
}
```

## Common Patterns

### Player Movement

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    let input = ctx.input();
    
    let mut move_dir = Vec2::ZERO;
    
    if input.is_key_down(VirtualKeyCode::W) {
        move_dir.y -= 1.0;
    }
    if input.is_key_down(VirtualKeyCode::S) {
        move_dir.y += 1.0;
    }
    if input.is_key_down(VirtualKeyCode::A) {
        move_dir.x -= 1.0;
    }
    if input.is_key_down(VirtualKeyCode::D) {
        move_dir.x += 1.0;
    }
    
    if move_dir.length_squared() > 0.0 {
        move_dir = move_dir.normalized();
        self.player.position += move_dir * self.speed * dt;
    }
    
    Ok(())
}
```

### Camera Following

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let (screen_w, screen_h) = ctx.renderer().surface_size();
    
    // Target camera position (center on player)
    let target_pos = Vec2::new(
        self.player.position.x - (screen_w as f32 * 0.5),
        self.player.position.y - (screen_h as f32 * 0.5),
    );
    
    // Smooth camera following
    let camera_speed = 5.0;
    let dt = ctx.delta_time().as_secs_f32();
    self.camera.position = self.camera.position.lerp(target_pos, camera_speed * dt);
    
    Ok(())
}
```

### Click to Spawn

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    if ctx.input().is_mouse_pressed(MouseButton::Left) {
        let mouse_world = ctx.mouse_world(&self.camera);
        self.spawn_entity_at(mouse_world);
    }
    Ok(())
}
```

### Collision Detection

```rust
fn check_collisions(&mut self) {
    for i in 0..self.entities.len() {
        for j in (i + 1)..self.entities.len() {
            let pos1 = self.entities[i].position;
            let pos2 = self.entities[j].position;
            let radius1 = self.entities[i].radius;
            let radius2 = self.entities[j].radius;
            
            let distance = pos1.distance(pos2);
            if distance < radius1 + radius2 {
                // Collision!
                self.handle_collision(i, j);
            }
        }
    }
}
```

### Sprite Rotation

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    
    for sprite in &mut self.sprites {
        sprite.transform.rotation += self.rotation_speed * dt;
    }
    
    Ok(())
}
```

### Score Display

```rust
fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let renderer = ctx.renderer();
    let mut frame = renderer.begin_frame()?;
    renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;
    
    // Draw game sprites...
    
    // Draw score text
    if let Some(font) = self.font {
        let score_text = format!("Score: {}", self.score);
        
        // Re-rasterize if score changed
        renderer.rasterize_text_glyphs(&score_text, font, 24.0)?;
        
        // Position in top-left (screen space)
        let (screen_w, screen_h) = renderer.surface_size();
        let text_pos = Vec2::new(
            self.camera.position.x - (screen_w as f32 * 0.5) + 20.0,
            self.camera.position.y + (screen_h as f32 * 0.5) - 40.0,
        );
        
        renderer.draw_text(
            &mut frame,
            &score_text,
            font,
            24.0,
            text_pos,
            [1.0, 1.0, 1.0, 1.0],
            &self.camera,
        )?;
    }
    
    renderer.end_frame(frame)?;
    Ok(())
}
```

### Bounds Clamping

```rust
fn clamp_to_bounds(&mut self, entity: &mut Entity, bounds: Vec2) {
    let half_size = entity.size * 0.5;
    
    entity.position.x = entity.position.x.clamp(
        half_size.x,
        bounds.x - half_size.x,
    );
    entity.position.y = entity.position.y.clamp(
        half_size.y,
        bounds.y - half_size.y,
    );
}
```

### Fixed Timestep Physics

```rust
impl Game for MyGame {
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let fixed_dt = ctx.fixed_delta_time().as_secs_f32();
        
        // Update physics
        self.velocity += self.acceleration * fixed_dt;
        self.position += self.velocity * fixed_dt;
        
        // Apply friction
        self.velocity *= 0.95;
        
        // Check collisions
        self.check_collisions();
        
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Interpolate visual position
        let alpha = ctx.fixed_update_alpha();
        self.visual_position = self.last_position.lerp(self.position, alpha);
        Ok(())
    }
}
```

### Audio on Event

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    if ctx.input().is_key_pressed(VirtualKeyCode::Space) {
        // Play jump sound
        if ctx.audio().is_available() {
            let jump_sound = include_bytes!("assets/jump.wav");
            ctx.audio().play_sound_from_bytes(jump_sound)?;
        }
    }
    Ok(())
}
```

## Complete Example

See `examples/basic_game/` for a complete working example demonstrating:
- Sprite rendering
- Input handling
- Camera following
- Collision detection
- Text rendering
- Asset management

Run it with:

```bash
cargo run -p basic_game
```

