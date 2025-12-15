# Input System

Forge2D provides frame-accurate input state tracking for keyboard and mouse input.

## Accessing Input

Get the input state from `EngineContext`:

```rust
let input = ctx.input();
```

## Keyboard Input

### Key States

```rust
use forge2d::VirtualKeyCode;

// Check if key is currently held down
if input.is_key_down(VirtualKeyCode::W) {
    // Move forward (fires every frame while held)
}

// Check if key was just pressed this frame
if input.is_key_pressed(VirtualKeyCode::Space) {
    // Jump (fires only once per press)
}

// Check if key was just released this frame
if input.is_key_released(VirtualKeyCode::Escape) {
    // Pause menu (fires only once per release)
}
```

### Common Keys

```rust
// Movement
VirtualKeyCode::W
VirtualKeyCode::A
VirtualKeyCode::S
VirtualKeyCode::D

// Arrow keys
VirtualKeyCode::Up
VirtualKeyCode::Down
VirtualKeyCode::Left
VirtualKeyCode::Right

// Common keys
VirtualKeyCode::Space
VirtualKeyCode::Escape
VirtualKeyCode::Enter
VirtualKeyCode::Tab
VirtualKeyCode::Shift
VirtualKeyCode::Control
VirtualKeyCode::Alt
```

See the [winit documentation](https://docs.rs/winit/latest/winit/event/enum.VirtualKeyCode.html) for all available keys.

## Mouse Input

### Mouse Position

```rust
// Get mouse position (screen coordinates)
let (x, y) = input.mouse_position();
let mouse_pos = input.mouse_position_vec2();  // As Vec2

// Convert to world coordinates (requires camera)
let mouse_world = ctx.mouse_world(&camera);
```

### Mouse Buttons

```rust
use forge2d::MouseButton;

// Check if button is currently held down
if input.is_mouse_down(MouseButton::Left) {
    // Dragging (fires every frame while held)
}

// Check if button was just pressed this frame
if input.is_mouse_pressed(MouseButton::Left) {
    // Clicked (fires only once per press)
}

// Check if button was just released this frame
if input.is_mouse_released(MouseButton::Right) {
    // Right button released (fires only once per release)
}
```

### Available Buttons

- `MouseButton::Left` - Left mouse button
- `MouseButton::Right` - Right mouse button
- `MouseButton::Middle` - Middle mouse button (scroll wheel click)
- `MouseButton::Other(u8)` - Additional buttons (e.g., side buttons)

## Input State Methods

### Keyboard

- `is_key_down(key: VirtualKeyCode) -> bool` - Key currently held
- `is_key_pressed(key: VirtualKeyCode) -> bool` - Key just pressed this frame
- `is_key_released(key: VirtualKeyCode) -> bool` - Key just released this frame

### Mouse

- `is_mouse_down(button: MouseButton) -> bool` - Button currently held
- `is_mouse_pressed(button: MouseButton) -> bool` - Button just pressed this frame
- `is_mouse_released(button: MouseButton) -> bool` - Button just released this frame
- `mouse_position() -> (f32, f32)` - Mouse position (screen coordinates)
- `mouse_position_vec2() -> Vec2` - Mouse position as Vec2

## Common Patterns

### Movement

```rust
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
    player.position += move_dir * speed * dt;
}
```

### Click to Spawn

```rust
if input.is_mouse_pressed(MouseButton::Left) {
    let mouse_world = ctx.mouse_world(&camera);
    spawn_entity_at(mouse_world);
}
```

### Exit on ESC

```rust
if input.is_key_pressed(VirtualKeyCode::Escape) {
    ctx.request_exit();
}
```

## High-Level Input Mapping (Actions & Axes)

On top of `InputState`, Forge2D provides a simple, data-driven input mapping layer:

- **`ActionId`** – named logical actions (e.g. `"jump"`, `"shoot"`)
- **`Button`** – physical inputs (keyboard keys or mouse buttons)
- **`AxisBinding`** – maps buttons to a one-dimensional axis (e.g. horizontal movement)
- **`InputMap`** – stores action and axis bindings and queries their state

This allows you to write game code against *actions* and *axes* instead of hardcoding keycodes everywhere.

### Defining an InputMap

```rust
use forge2d::{ActionId, AxisBinding, Button, InputMap, VirtualKeyCode};

let mut input_map = InputMap::new();

let axis_horizontal = ActionId::new("move_horizontal");
let axis_vertical = ActionId::new("move_vertical");

// Horizontal: A/Left = -1, D/Right = +1
input_map.set_axis(
    axis_horizontal.clone(),
    AxisBinding::new(
        vec![
            Button::Key(VirtualKeyCode::A),
            Button::Key(VirtualKeyCode::Left),
        ],
        vec![
            Button::Key(VirtualKeyCode::D),
            Button::Key(VirtualKeyCode::Right),
        ],
    ),
);

// Vertical: W/Up = -1 (up), S/Down = +1 (down)
input_map.set_axis(
    axis_vertical.clone(),
    AxisBinding::new(
        vec![
            Button::Key(VirtualKeyCode::W),
            Button::Key(VirtualKeyCode::Up),
        ],
        vec![
            Button::Key(VirtualKeyCode::S),
            Button::Key(VirtualKeyCode::Down),
        ],
    ),
);
```

### Using actions & axes in update()

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    let input = ctx.input();

    // Axis values in [-1.0, 1.0]
    let horizontal = self.input_map.axis(input, &self.axis_horizontal);
    let vertical = self.input_map.axis(input, &self.axis_vertical);

    let move_dir = Vec2::new(horizontal, vertical);
    if move_dir.length_squared() > 0.0 {
        let dir = move_dir.normalized();
        self.position += dir * self.speed * dt;
    }

    Ok(())
}
```

You can also bind *actions* to buttons and check them via:

- `input_map.action_down(&input_state, &action_id)`
- `input_map.action_pressed(&input_state, &action_id)`

This is ideal for things like `"jump"`, `"shoot"`, `"pause"`, etc.

## Frame-Accurate Input

Forge2D tracks input state per frame, ensuring:

- **`is_key_pressed()`** only returns `true` on the frame the key was first pressed
- **`is_key_released()`** only returns `true` on the frame the key was released
- **`is_key_down()`** returns `true` for all frames the key is held

This makes it easy to distinguish between "key held" and "key just pressed" without manual state tracking.

