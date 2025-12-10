use std::time::Duration;

use anyhow::Result;
use forge2d::{Engine, EngineContext, Game};

struct BasicGame {
    frames: u64,
}

impl Game for BasicGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        println!(
            "Starting basic Forge2D example at {:?} logical size.",
            ctx.window().inner_size()
        );
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.frames += 1;

        if ctx.elapsed_time() > Duration::from_secs(3) {
            println!(
                "Ran for {:.2?} across {} frames; requesting exit.",
                ctx.elapsed_time(),
                self.frames
            );
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time();
        println!("Frame {} | dt = {:.4?}", self.frames, dt);
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D – Basic Game")
        .with_size(800, 600)
        .run(BasicGame { frames: 0 })
}
