//! Forge2D - a lightweight 2D game framework.
//!
//! Phase 5 adds asset management and audio support.

pub mod assets;
pub mod audio;
pub mod camera;
pub mod commands;
pub mod component_metadata;
pub mod entities;
pub mod engine;
pub mod fonts;
pub mod grid;
pub mod hierarchy;
pub mod hud;
pub mod input;
pub mod math;
pub mod pathfinding;
pub mod physics;
pub mod render;
pub mod scene;
pub mod state;
pub mod world;

pub use crate::assets::AssetManager;
pub use crate::audio::AudioSystem;
pub use crate::engine::{Engine, EngineConfig, EngineContext, Game};
pub use crate::fonts::BuiltinFont;
pub use crate::hud::{HudLayer, HudLayout, HudPanel, HudRect, HudSprite, HudText, TextAlign};
pub use crate::input::{ActionId, AxisBinding, Button, InputMap, InputState};
pub use crate::camera::{CameraFollow, update_camera_follow};
pub use crate::entities::{
    AudioSource, CameraComponent, Checkpoint, Collectible, Enemy, Hazard, MovingPlatform,
    PhysicsBody, Player, SpriteComponent, Transform, Trigger,
};
pub use crate::grid::{Grid, GridCoord, GridPathfinding};
pub use crate::math::{Camera2D, Transform2D, Vec2};
pub use crate::pathfinding::{AStarPathfinder, GridNode, PathfindingGrid};
pub use crate::render::{Frame, FontHandle, Renderer, Sprite, TextureHandle};
pub use crate::state::{State, StateMachine, StateMachineLike};
pub use crate::physics::{PhysicsEventCallback, PhysicsWorld};
pub use rapier2d::prelude::{ImpulseJointHandle, ImpulseJointSet, RigidBodyType};
pub use rapier2d::prelude::RigidBodyHandle;
pub use crate::world::{EntityId, World};
pub use crate::commands::{Command, CommandHistory, CreateEntity, DeleteEntity, SetTransform, AddComponent, RemoveComponent};
pub use crate::component_metadata::{ComponentMetadataHandler, ComponentMetadataRegistry, FieldDescriptor, TransformMetadataHandler, register_builtin_metadata};
pub use crate::hierarchy::{get_parent, set_parent, get_children, get_root, get_world_position, get_world_rotation, get_world_scale, reparent};
pub use crate::scene::{Scene, SerializableComponent, SerializablePhysics, ComponentSerializable, create_scene, restore_scene_physics, restore_scene_physics_preserve};
pub use winit::{
    event::MouseButton,
    keyboard::KeyCode,
};
