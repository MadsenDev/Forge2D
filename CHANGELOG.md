# Changelog

All notable changes to this project will be documented in this file.

## Unreleased
### Added
- Added `AssetManager` for caching textures and other assets, preventing duplicate loads.
- Added `AudioSystem` wrapping `rodio` for sound effects and background music playback.
- Added convenience methods `EngineContext::load_texture()` and `EngineContext::load_texture_from_bytes()`.
- Added `AudioSystem::is_available()` to check if audio is working.
- Added graceful audio initialization - engine continues even if audio fails to initialize.
- Significantly enhanced `basic_game` example to showcase all engine features:
  - Player movement with WASD/Arrow keys
  - Multiple sprites (player, enemies, collectibles)
  - Camera following player smoothly
  - Mouse click interaction (spawn collectibles)
  - Collision detection (player vs collectibles)
  - Sprite rotation animations
  - Asset caching demonstration
  - Score system
### Added (previous)
- Initialized the Forge2D workspace with the core `Engine` API and game loop skeleton.
- Added a `basic_game` example demonstrating window creation and a simple timed exit.
- Introduced starter project documentation including TODO tracking.
- Implemented an `InputState` module for tracking keyboard and mouse state, integrated into the engine loop, and exposed through `EngineContext`.
- Updated the basic example to demonstrate input queries and mouse position logging.
- Added a wgpu-backed `Renderer` with frame lifecycle helpers, integrated into the engine context, and used by the example to clear the screen.
- Added math helpers (`Vec2`, `Transform2D`, `Camera2D`) plus texture-backed sprites with a simple wgpu pipeline.
- Implemented texture loading helpers on the renderer alongside sprite drawing support and camera-aware transforms.
- Updated the `basic_game` example to render a bouncing, textured sprite using the new camera and sprite APIs.
- Added comprehensive `Vec2` math utilities: `dot()`, `distance()`, `distance_squared()`, `lerp()`, `from_angle()`, `abs()`, `min()`, `max()`, and `length_squared()`.
- Added `Div` and `Neg` trait implementations for `Vec2`.
- Added `Camera2D::screen_to_world()` and `Camera2D::world_to_screen()` coordinate conversion methods.
- Added `InputState::mouse_position_vec2()` helper method.
- Added `Drop` implementation for `Frame` to ensure proper resource cleanup.
### Changed
- Significantly expanded README with comprehensive usage guide, examples, and API documentation.
- Fixed WGSL shader syntax: changed struct field separators from semicolons to commas (required by wgpu 0.17).
- Fixed corrupted PNG data in `basic_game` example: replaced with valid 64x64 red square PNG.
- Embedded the example sprite texture bytes directly into the code to avoid shipping a binary asset file.
- Fixed input system bounds checking in `is_key_down()`, `is_key_pressed()`, and `is_key_released()` methods to prevent potential panics.
- Added `#[must_use]` attributes to `Engine` builder methods (`with_title()`, `with_size()`, `with_vsync()`) to prevent accidental discarding of configured instances.
