// Simple player controller used by the scripting demo.
// Horizontal movement uses WASD/arrow keys; Space provides a small hop.

// Defaults tuned for the Forge2D scripting demo scale.
let SPEED = if params.contains("speed") { params["speed"] } else { 200.0 };
let JUMP = if params.contains("jump") { params["jump"] } else { 75.0 };

fn on_start(self) {
    print("[player script] loaded scripting_demo_player.rhai");
    print("[player script] params: SPEED=" + SPEED.to_string() + ", JUMP=" + JUMP.to_string());
    let sprite = self.sprite();
    if sprite != () {
        sprite.set_tint([0.35, 0.9, 1.0, 1.0]);
    }
}

fn on_update(self, _dt) {
    let physics = self.physics();
    if physics == () { return; }

    let move_x = 0.0;
    if self.input.is_key_down("A") || self.input.is_key_down("Left") { move_x -= 1.0; }
    if self.input.is_key_down("D") || self.input.is_key_down("Right") { move_x += 1.0; }

    if self.input.is_key_pressed("A") || self.input.is_key_pressed("Left") {
        print("[player script] left pressed");
    }
    if self.input.is_key_pressed("D") || self.input.is_key_pressed("Right") {
        print("[player script] right pressed");
    }

    let velocity = physics.velocity();
    velocity.x = move_x * SPEED;

    if self.input.is_key_pressed("Space") {
        print("[player script] jump pressed");
        velocity.y = -JUMP;
    }

    physics.set_velocity(velocity);
}
