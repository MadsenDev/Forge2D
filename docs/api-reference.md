# API Reference

Complete API documentation for Forge2D.

## Engine

### Engine

```rust
pub struct Engine { /* ... */ }

impl Engine {
    pub fn new() -> Self;
    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_size(self, width: u32, height: u32) -> Self;
    pub fn with_vsync(self, vsync: bool) -> Self;
    pub fn run<G: Game>(self, game: G) -> Result<()>;
}
```

### Game Trait

```rust
pub trait Game {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> { Ok(()) }
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> { Ok(()) }
}
```

### EngineContext

```rust
pub struct EngineContext<'a> { /* ... */ }

impl EngineContext {
    pub fn delta_time(&self) -> Duration;
    pub fn elapsed_time(&self) -> Duration;
    pub fn should_run_fixed_update(&self) -> bool;
    pub fn fixed_delta_time(&self) -> Duration;
    pub fn fixed_update_alpha(&self) -> f32;
    pub fn input(&self) -> &InputState;
    pub fn renderer(&mut self) -> &mut Renderer;
    pub fn assets(&mut self) -> &mut AssetManager;
    pub fn audio(&mut self) -> &mut AudioSystem;
    pub fn window(&self) -> &Window;
    pub fn mouse_world(&self, camera: &Camera2D) -> Vec2;
    pub fn load_texture(&mut self, path: &str) -> Result<TextureHandle>;
    pub fn load_texture_from_bytes(&mut self, id: &str, bytes: &[u8]) -> Result<TextureHandle>;
    pub fn request_exit(&mut self);
}
```

## Input

### InputState

```rust
pub struct InputState { /* ... */ }

impl InputState {
    pub fn is_key_down(&self, key: VirtualKeyCode) -> bool;
    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool;
    pub fn is_key_released(&self, key: VirtualKeyCode) -> bool;
    pub fn is_mouse_down(&self, button: MouseButton) -> bool;
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool;
    pub fn is_mouse_released(&self, button: MouseButton) -> bool;
    pub fn mouse_position(&self) -> (f32, f32);
    pub fn mouse_position_vec2(&self) -> Vec2;
}
```

## Rendering

### Renderer

```rust
pub struct Renderer { /* ... */ }

impl Renderer {
    pub fn begin_frame(&mut self) -> Result<Frame>;
    pub fn clear(&mut self, frame: &mut Frame, color: [f32; 4]) -> Result<()>;
    pub fn draw_sprite(&mut self, frame: &mut Frame, sprite: &Sprite, camera: &Camera2D) -> Result<()>;
    pub fn draw_text(&mut self, frame: &mut Frame, text: &str, font: FontHandle, size: f32, position: Vec2, color: [f32; 4], camera: &Camera2D) -> Result<()>;
    pub fn load_texture_from_file(&mut self, path: &str) -> Result<TextureHandle>;
    pub fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle>;
    pub fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle>;
    pub fn rasterize_text_glyphs(&mut self, text: &str, font: FontHandle, size: f32) -> Result<()>;
    pub fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)>;
    pub fn surface_size(&self) -> (u32, u32);
    pub fn end_frame(&mut self, frame: Frame) -> Result<()>;
}
```

### Sprite

```rust
pub struct Sprite {
    pub texture: TextureHandle,
    pub transform: Transform2D,
    pub tint: [f32; 4],
}

impl Sprite {
    pub fn new(texture: TextureHandle) -> Self;
    pub fn set_size_px(&mut self, size_px: Vec2, texture_px: Vec2);
}
```

### Camera2D

```rust
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
}

impl Camera2D {
    pub fn new(position: Vec2) -> Self;
    pub fn default() -> Self;
    pub fn screen_to_world(&self, screen: Vec2, width: u32, height: u32) -> Vec2;
    pub fn world_to_screen(&self, world: Vec2, width: u32, height: u32) -> Vec2;
    pub fn view_projection(&self, width: u32, height: u32) -> Mat4;
}
```

## Math

### Vec2

```rust
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2;
    pub const ONE: Vec2;
    
    pub fn new(x: f32, y: f32) -> Self;
    pub fn length(&self) -> f32;
    pub fn length_squared(&self) -> f32;
    pub fn normalized(&self) -> Vec2;
    pub fn distance(&self, other: Vec2) -> f32;
    pub fn distance_squared(&self, other: Vec2) -> f32;
    pub fn dot(&self, other: Vec2) -> f32;
    pub fn lerp(&self, other: Vec2, t: f32) -> Vec2;
    pub fn from_angle(angle: f32) -> Vec2;
    pub fn abs(&self) -> Vec2;
    pub fn min(&self, other: Vec2) -> Vec2;
    pub fn max(&self, other: Vec2) -> Vec2;
}

impl Add<Vec2> for Vec2 { /* ... */ }
impl Sub<Vec2> for Vec2 { /* ... */ }
impl Mul<f32> for Vec2 { /* ... */ }
impl Div<f32> for Vec2 { /* ... */ }
impl Neg for Vec2 { /* ... */ }
```

### Transform2D

```rust
pub struct Transform2D {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Transform2D {
    pub fn to_matrix(&self, base_size: Vec2) -> Mat4;
}
```

## Assets

### AssetManager

```rust
pub struct AssetManager { /* ... */ }

impl AssetManager {
    pub fn new() -> Self;
    pub fn get_texture(&self, id: &str) -> Option<TextureHandle>;
    pub fn load_texture(&mut self, renderer: &mut Renderer, path: &str) -> Result<TextureHandle>;
    pub fn load_texture_from_bytes(&mut self, renderer: &mut Renderer, id: &str, bytes: &[u8]) -> Result<TextureHandle>;
}
```

## Audio

### AudioSystem

```rust
pub struct AudioSystem { /* ... */ }

impl AudioSystem {
    pub fn new() -> Self;
    pub fn is_available(&self) -> bool;
    pub fn play_sound_from_bytes(&self, bytes: &[u8]) -> Result<()>;
    pub fn play_music_loop_from_bytes(&self, bytes: &[u8]) -> Result<()>;
    pub fn stop_music(&self);
}
```

## Types

### TextureHandle

```rust
pub struct TextureHandle(pub(crate) u32);
```

### FontHandle

```rust
pub struct FontHandle(pub(crate) u32);
```

### Frame

```rust
pub struct Frame { /* ... */ }
```

## State Management

### State Trait

```rust
pub trait State {
    fn on_enter(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn on_exit(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn update(&mut self, ctx: &mut EngineContext, state_machine: &mut dyn StateMachineLike) -> Result<()>;
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()>;
}
```

### StateMachine

```rust
pub struct StateMachine { /* ... */ }

impl StateMachine {
    pub fn new() -> Self;
    pub fn with_initial_state(initial: Box<dyn State>) -> Self;
    pub fn push(&mut self, state: Box<dyn State>);
    pub fn pop(&mut self);
    pub fn replace(&mut self, state: Box<dyn State>);
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn apply_transitions(&mut self, ctx: &mut EngineContext) -> Result<()>;
    pub fn update_top(&mut self, ctx: &mut EngineContext) -> Result<()>;
    pub fn draw_all(&mut self, ctx: &mut EngineContext) -> Result<()>;
    pub fn states(&self) -> &VecDeque<Box<dyn State>>;
}

impl Game for StateMachine { /* ... */ }
```

### StateMachineLike Trait

```rust
pub trait StateMachineLike {
    fn push(&mut self, state: Box<dyn State>);
    fn pop(&mut self);
    fn replace(&mut self, state: Box<dyn State>);
}
```

## Re-exports

Forge2D re-exports the following from `winit`:

- `VirtualKeyCode` - Keyboard key codes
- `MouseButton` - Mouse button types

