use std::time::Duration;

use anyhow::Result;
use forge2d::{Camera2D, Engine, EngineContext, Game, Sprite, Vec2, VirtualKeyCode};

const RED_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5, 0x1c, 0x0c, 0x02, 0x00, 0x00, 0x00, 0x0b, 0x49,
    0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc, 0xff, 0x1f, 0x00, 0x02, 0xeb, 0x01, 0xf5, 0x8b, 0x28, 0x36, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

struct BasicGame {
    sprite: Option<Sprite>,
    camera: Camera2D,
    velocity: Vec2,
}

impl Game for BasicGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let texture = ctx.renderer().load_texture_from_bytes(RED_PNG)?;

        let mut sprite = Sprite::new(texture);
        if let Some((w, h)) = ctx.renderer().texture_size(texture) {
            sprite.transform.scale = Vec2::new(w as f32, h as f32);
        }

        let (screen_w, screen_h) = ctx.renderer().surface_size();
        sprite.transform.position = Vec2::new(screen_w as f32 * 0.25, screen_h as f32 * 0.5);

        self.sprite = Some(sprite);
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if ctx.input().is_key_pressed(VirtualKeyCode::Escape) {
            ctx.request_exit();
        }

        let dt = ctx.delta_time().as_secs_f32();

        if let Some(sprite) = self.sprite.as_mut() {
            sprite.transform.position += self.velocity * dt;

            let (screen_w, screen_h) = ctx.renderer().surface_size();
            let bounds = Vec2::new(screen_w as f32, screen_h as f32);
            let size = sprite.transform.scale;
            let pos = &mut sprite.transform.position;

            if pos.x < 0.0 {
                pos.x = 0.0;
                self.velocity.x = self.velocity.x.abs();
            } else if pos.x + size.x > bounds.x {
                pos.x = bounds.x - size.x;
                self.velocity.x = -self.velocity.x.abs();
            }

            if pos.y < 0.0 {
                pos.y = 0.0;
                self.velocity.y = self.velocity.y.abs();
            } else if pos.y + size.y > bounds.y {
                pos.y = bounds.y - size.y;
                self.velocity.y = -self.velocity.y.abs();
            }

            self.camera.position = *pos - Vec2::new(bounds.x * 0.5, bounds.y * 0.5);
        }

        if ctx.elapsed_time() > Duration::from_secs(10) {
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.08, 0.08, 0.12, 1.0])?;

        if let Some(sprite) = &self.sprite {
            renderer.draw_sprite(&mut frame, sprite, &self.camera)?;
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D – Bouncing Sprite")
        .with_size(800, 600)
        .run(BasicGame {
            sprite: None,
            camera: Camera2D::default(),
            velocity: Vec2::new(180.0, 140.0),
        })
}
