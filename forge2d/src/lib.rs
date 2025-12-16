//! Forge2D - a lightweight 2D game framework.
//!
//! Phase 5 adds asset management and audio support.

pub mod assets;
pub mod audio;
pub mod engine;
pub mod fonts;
pub mod hud;
pub mod input;
pub mod math;
pub mod physics;
pub mod render;
pub mod state;
pub mod world;

pub use crate::assets::AssetManager;
pub use crate::audio::AudioSystem;
pub use crate::engine::{Engine, EngineConfig, EngineContext, Game};
pub use crate::fonts::BuiltinFont;
pub use crate::hud::{HudLayer, HudRect, HudSprite, HudText};
pub use crate::input::{ActionId, AxisBinding, Button, InputMap, InputState};
pub use crate::math::{Camera2D, Transform2D, Vec2};
pub use crate::render::{Frame, FontHandle, Renderer, Sprite, TextureHandle};
pub use crate::state::{State, StateMachine, StateMachineLike};
pub use crate::physics::{CollisionCallback, PhysicsWorld};
pub use rapier2d::prelude::{ImpulseJointHandle, ImpulseJointSet, RigidBodyType};
pub use rapier2d::prelude::RigidBodyHandle;
pub use crate::world::{EntityId, World};
pub use winit::event::{MouseButton, VirtualKeyCode};
