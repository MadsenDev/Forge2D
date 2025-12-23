// Example Rhai behaviour component for Forge2D.
// - WASD moves the owner with a simple velocity set.
// - Space applies an upward impulse (jump).
// - Up arrow spawns a dynamic projectile at the owner's position.
// - Any trigger contact despawns the other entity.

let SPEED = if params.contains("speed") { params["speed"] } else { 6.0 };
let JUMP_FORCE = if params.contains("jump_force") { params["jump_force"] } else { 8.0 };

fn on_start(self) {
    let transform = self.transform();
    if transform != () { transform.set_rotation(0.0); }
}

fn on_update(self, dt) {
    let move = vec2(0.0, 0.0);
    if self.input.is_key_down("A") { move.x -= 1.0; }
    if self.input.is_key_down("D") { move.x += 1.0; }
    if self.input.is_key_down("W") { move.y -= 1.0; }
    if self.input.is_key_down("S") { move.y += 1.0; }

    if move.x != 0.0 || move.y != 0.0 {
        let len = (move.x * move.x + move.y * move.y).sqrt();
        move.x /= len;
        move.y /= len;
    }

    let physics = self.physics();
    if physics != () {
        physics.set_velocity(vec2(move.x * SPEED, move.y * SPEED));

        if self.input.is_key_pressed("Space") {
            physics.apply_impulse(vec2(0.0, -JUMP_FORCE));
        }
    }

    if self.input.is_key_pressed("Up") {
        let transform = self.transform();
        let pos = if transform != () { transform.position() } else { vec2(0.0, 0.0) };
        let launch = vec2(0.0, -12.0);
        self.world().spawn_dynamic(pos, launch);
    }
}

fn on_trigger_enter(self, other) {
    // Destroy pickups or bullets that collide with this script owner.
    self.world().despawn(other);
}
