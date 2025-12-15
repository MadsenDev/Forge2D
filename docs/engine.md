# Engine & Game Loop

## Engine Configuration

The `Engine` is the entry point for your game. Configure it using builder methods:

```rust
use forge2d::Engine;

Engine::new()
    .with_title("My Game")           // Window title
    .with_size(1280, 720)            // Window size (logical pixels)
    .with_vsync(true)                 // Enable/disable VSync
    .run(my_game)
```

### Builder Methods

- **`with_title(title: impl Into<String>)`** - Set the window title
- **`with_size(width: u32, height: u32)`** - Set window size in logical pixels
- **`with_vsync(vsync: bool)`** - Enable or disable VSync (default: true)

## The Game Trait

Your game must implement the `Game` trait:

```rust
use forge2d::{Game, EngineContext};
use anyhow::Result;

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called once at startup
        // Load assets, initialize state, set up your game world
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called every frame (variable timestep)
        // Handle input, update game logic, move entities
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called every frame
        // Render your game using the renderer
        Ok(())
    }
    
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Optional: Called at fixed intervals (default: 60 FPS)
        // Use for physics, collision detection, deterministic systems
        Ok(())
    }
}
```

### Method Execution Order

1. **`init()`** - Called once when the engine starts
2. **`fixed_update()`** - Called at fixed intervals (if implemented)
3. **`update()`** - Called every frame
4. **`draw()`** - Called every frame after update

## EngineContext

The `EngineContext` provides access to all engine systems:

### Time

```rust
let dt = ctx.delta_time();              // Duration since last frame
let elapsed = ctx.elapsed_time();      // Total time since engine started
let dt_seconds = ctx.delta_time().as_secs_f32();  // Delta as f32
```

### Fixed Timestep

```rust
if ctx.should_run_fixed_update() {
    let fixed_dt = ctx.fixed_delta_time();  // Fixed timestep duration
    let alpha = ctx.fixed_update_alpha();    // Interpolation factor (0.0-1.0)
}
```

### Input

```rust
let input = ctx.input();
// See input.md for details
```

### Rendering

```rust
let renderer = ctx.renderer();
// See rendering.md for details
```

### Assets

```rust
let texture = ctx.load_texture("assets/sprite.png")?;
let texture2 = ctx.load_texture_from_bytes("my_texture", png_bytes)?;
```

### Audio

```rust
if ctx.audio().is_available() {
    ctx.audio().play_sound_from_bytes(sound_bytes)?;
}
```

### Window

```rust
let window = ctx.window();
let size = window.inner_size();
```

### Utilities

```rust
// Convert mouse screen position to world coordinates
let mouse_world = ctx.mouse_world(&camera);

// Request the engine to exit
ctx.request_exit();
```

## Game Loop

The engine runs a game loop that:

1. Processes window events (resize, close, etc.)
2. Updates input state
3. Calls `fixed_update()` if needed
4. Calls `update()`
5. Calls `draw()`
6. Presents the frame to the screen

The loop continues until:
- The window is closed
- `ctx.request_exit()` is called
- An error occurs

## Fixed Timestep

Forge2D supports fixed timestep updates for deterministic game logic. See [Fixed Timestep](fixed-timestep.md) for details.

