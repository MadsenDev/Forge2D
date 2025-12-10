use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode};

/// Tracks keyboard and mouse state across frames.
pub struct InputState {
    keys_down: [bool; 256],
    keys_pressed: [bool; 256],
    keys_released: [bool; 256],

    mouse_x: f32,
    mouse_y: f32,
    mouse_down: [bool; 8],
    mouse_pressed: [bool; 8],
    mouse_released: [bool; 8],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: [false; 256],
            keys_pressed: [false; 256],
            keys_released: [false; 256],
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: [false; 8],
            mouse_pressed: [false; 8],
            mouse_released: [false; 8],
        }
    }

    /// Clear per-frame pressed/released flags.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.fill(false);
        self.keys_released.fill(false);
        self.mouse_pressed.fill(false);
        self.mouse_released.fill(false);
    }

    /// Handle a keyboard input event from winit.
    pub fn handle_key(&mut self, input: KeyboardInput) {
        if let Some(keycode) = input.virtual_keycode {
            let idx = keycode as usize;
            if idx < self.keys_down.len() {
                match input.state {
                    ElementState::Pressed => {
                        if !self.keys_down[idx] {
                            self.keys_pressed[idx] = true;
                        }
                        self.keys_down[idx] = true;
                    }
                    ElementState::Released => {
                        self.keys_down[idx] = false;
                        self.keys_released[idx] = true;
                    }
                }
            }
        }
    }

    /// Handle a mouse button input event from winit.
    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if let Some(idx) = mouse_button_index(button) {
            match state {
                ElementState::Pressed => {
                    if !self.mouse_down[idx] {
                        self.mouse_pressed[idx] = true;
                    }
                    self.mouse_down[idx] = true;
                }
                ElementState::Released => {
                    self.mouse_down[idx] = false;
                    self.mouse_released[idx] = true;
                }
            }
        }
    }

    /// Handle mouse cursor movement from winit.
    pub fn handle_cursor_moved(&mut self, x: f64, y: f64) {
        self.mouse_x = x as f32;
        self.mouse_y = y as f32;
    }

    /// Returns true if the key is currently held down.
    pub fn is_key_down(&self, key: VirtualKeyCode) -> bool {
        self.keys_down[key as usize]
    }

    /// Returns true if the key was pressed this frame.
    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys_pressed[key as usize]
    }

    /// Returns true if the key was released this frame.
    pub fn is_key_released(&self, key: VirtualKeyCode) -> bool {
        self.keys_released[key as usize]
    }

    /// Returns true if the mouse button is currently held down.
    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        mouse_button_index(button)
            .map(|idx| self.mouse_down[idx])
            .unwrap_or(false)
    }

    /// Returns true if the mouse button was pressed this frame.
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        mouse_button_index(button)
            .map(|idx| self.mouse_pressed[idx])
            .unwrap_or(false)
    }

    /// Returns true if the mouse button was released this frame.
    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        mouse_button_index(button)
            .map(|idx| self.mouse_released[idx])
            .unwrap_or(false)
    }

    /// Current mouse cursor position in logical pixels.
    pub fn mouse_position(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }
}

fn mouse_button_index(button: MouseButton) -> Option<usize> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        MouseButton::Other(idx) if (idx as usize) < 8 => Some(idx as usize),
        _ => None,
    }
}
