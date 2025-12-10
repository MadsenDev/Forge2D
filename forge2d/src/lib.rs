//! Forge2D - a lightweight 2D game framework.
//!
//! Phase 1 focuses on opening a window and running a basic game loop.

pub mod engine;
pub mod input;
pub mod math;
pub mod render;

pub use crate::engine::{Engine, EngineConfig, EngineContext, Game};
pub use crate::input::InputState;
pub use crate::math::{Camera2D, Transform2D, Vec2};
pub use crate::render::{Frame, Renderer, Sprite, TextureHandle};
pub use winit::event::{MouseButton, VirtualKeyCode};
