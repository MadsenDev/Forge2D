mod entities;
mod game;
mod resources;
mod systems;
mod ui;

use anyhow::Result;
use forge2d::Engine;
use game::AgeOfEmpiresDemo;

fn main() -> Result<()> {
    Engine::new()
        .with_title("Age of Empires Demo - Forge2D")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(AgeOfEmpiresDemo::new())
}
