mod game;
mod entities;

use anyhow::Result;
use forge2d::Engine;
use game::AsteroidsGame;

fn main() -> Result<()> {
    Engine::new()
        .with_title("Asteroids - Forge2D")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(AsteroidsGame::new())
}

