# Rendering

Forge2D provides hardware-accelerated 2D rendering using wgpu.

## Basic Rendering Flow

Every frame, you'll follow this pattern:

```rust
fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let renderer = ctx.renderer();
    let mut frame = renderer.begin_frame()?;
    
    // Clear the screen (RGBA: 0.0-1.0)
    renderer.clear(&mut frame, [0.1, 0.1, 0.2, 1.0])?;
    
    // Draw sprites, text, etc.
    
    renderer.end_frame(frame)?;
    Ok(())
}
```

## Loading Textures

### From File

```rust
let texture = renderer.load_texture_from_file("assets/sprite.png")?;
```

### From Bytes

```rust
let texture = renderer.load_texture_from_bytes(png_bytes)?;
```

### Using AssetManager (Recommended)

```rust
// Load texture (cached, prevents duplicate loads)
let texture = ctx.load_texture("assets/sprite.png")?;
let texture2 = ctx.load_texture_from_bytes("my_texture", png_bytes)?;

// Get cached texture
if let Some(texture) = ctx.assets().get_texture("my_texture") {
    // Use texture
}
```

## Sprite Rendering

### Creating Sprites

```rust
use forge2d::{Sprite, Vec2};

// Create a sprite
let mut sprite = Sprite::new(texture);
sprite.transform.position = Vec2::new(100.0, 200.0);
sprite.transform.scale = Vec2::new(1.0, 1.0);  // Scale multiplier
sprite.transform.rotation = 0.0;  // Radians
sprite.tint = [1.0, 1.0, 1.0, 1.0];  // RGBA tint (white)
```

### Setting Sprite Size

**Important:** `Transform2D.scale` is a **multiplier** relative to the base texture size.

```rust
// Method 1: Set scale directly (multiplier)
sprite.transform.scale = Vec2::new(2.0, 2.0);  // 2x the texture size

// Method 2: Set size in pixels (helper method)
sprite.set_size_px(
    Vec2::new(64.0, 64.0),      // Desired size in pixels
    Vec2::new(32.0, 32.0),      // Original texture size
);
```

### Sprite Position

**Important:** `Transform2D.position` represents the **center** of the sprite.

```rust
// Position is the CENTER of the sprite
sprite.transform.position = Vec2::new(100.0, 200.0);

// When clamping to bounds, account for half the sprite size:
let half_size = sprite_size * 0.5;
sprite.transform.position.x = sprite.transform.position.x.clamp(
    half_size,
    world_bounds.x - half_size,
);
```

### Drawing Sprites

```rust
use forge2d::Camera2D;

let camera = Camera2D::default();
renderer.draw_sprite(&mut frame, &sprite, &camera)?;
```

### Sprite Properties

- **`texture: TextureHandle`** - The texture to render
- **`transform: Transform2D`** - Position (center), scale (multiplier), rotation (radians)
- **`tint: [f32; 4]`** - RGBA color tint (default: `[1.0, 1.0, 1.0, 1.0]`)

## Camera System

### Creating a Camera

```rust
use forge2d::Camera2D;

// Create camera at position
let mut camera = Camera2D::new(Vec2::new(0.0, 0.0));

// Or use default
let camera = Camera2D::default();
```

### Camera Properties

```rust
camera.position = Vec2::new(100.0, 200.0);  // Camera position
camera.zoom = 1.0;  // Zoom level (1.0 = 1:1 pixel-to-world ratio)
```

### Coordinate Conversion

```rust
let (screen_w, screen_h) = renderer.surface_size();

// Screen to world
let world_pos = camera.screen_to_world(screen_pos, screen_w, screen_h);

// World to screen
let screen_pos = camera.world_to_screen(world_pos, screen_w, screen_h);

// Or use the helper method
let mouse_world = ctx.mouse_world(&camera);
```

### Camera Following

```rust
// Smooth camera following
let target_pos = player.position - Vec2::new(screen_w as f32 * 0.5, screen_h as f32 * 0.5);
let camera_speed = 5.0;
camera.position = camera.position.lerp(target_pos, camera_speed * dt);
```

## Text Rendering

### Loading Fonts

```rust
use forge2d::{EngineContext, FontHandle};

// Load a font (TTF/OTF format) via AssetManager (preferred)
const FONT_BYTES: &[u8] = include_bytes!("assets/font.ttf");

fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let font: FontHandle = ctx.load_font_from_bytes("ui_font", FONT_BYTES)?;
    Ok(())
}
```

### Pre-rasterizing Glyphs

**Important:** You must rasterize glyphs before drawing text:

```rust
// Rasterize all glyphs needed for a text string
renderer.rasterize_text_glyphs("Hello World", font, 24.0)?;
```

### Drawing Text

```rust
renderer.draw_text(
    &mut frame,
    "Hello World",
    font,
    24.0,                              // Font size in pixels
    Vec2::new(100.0, 200.0),          // World position (bottom-left of text)
    [1.0, 1.0, 1.0, 1.0],            // RGBA color
    &camera,
)?;
```

### Text Rendering Notes

- Glyphs are cached automatically - re-rasterize only when the text string changes
- Position is the bottom-left corner of the first character
- Text is rendered as sprites (one sprite per glyph)

## Performance Notes

### Batched Rendering

Forge2D automatically batches all sprites into a single render pass per frame for optimal performance. You don't need to do anything special - just call `draw_sprite()` for each sprite.

### Glyph Caching

Text glyphs are cached per (font, character, size) combination. Re-rasterize only when:
- The text string changes
- You're using a new character
- You're using a new font size

### Texture Caching

Use `AssetManager` (via `ctx.load_texture()`) to prevent duplicate texture loads. The same texture loaded multiple times will return the same handle.

## Common Patterns

### Drawing Multiple Sprites

```rust
for sprite in &sprites {
    renderer.draw_sprite(&mut frame, sprite, &camera)?;
}
```

### Screen-Space UI Text

```rust
// Position text relative to camera for screen-space effect
let (screen_w, screen_h) = renderer.surface_size();
let text_pos = Vec2::new(
    camera.position.x - (screen_w as f32 * 0.5) + 20.0,
    camera.position.y + (screen_h as f32 * 0.5) - 40.0,
);
renderer.draw_text(&mut frame, "Score: 100", font, 24.0, text_pos, [1.0, 1.0, 1.0, 1.0], &camera)?;
```

### Sprite Rotation

```rust
sprite.transform.rotation += rotation_speed * dt;
```

### Sprite Tinting

```rust
// Red tint
sprite.tint = [1.0, 0.0, 0.0, 1.0];

// Semi-transparent
sprite.tint = [1.0, 1.0, 1.0, 0.5];
```

