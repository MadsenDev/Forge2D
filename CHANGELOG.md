# Changelog

All notable changes to this project will be documented in this file.

## Unreleased
### Added
- Initialized the Forge2D workspace with the core `Engine` API and game loop skeleton.
- Added a `basic_game` example demonstrating window creation and a simple timed exit.
- Introduced starter project documentation including TODO tracking.
- Implemented an `InputState` module for tracking keyboard and mouse state, integrated into the engine loop, and exposed through `EngineContext`.
- Updated the basic example to demonstrate input queries and mouse position logging.
- Added a wgpu-backed `Renderer` with frame lifecycle helpers, integrated into the engine context, and used by the example to clear the screen.
- Added math helpers (`Vec2`, `Transform2D`, `Camera2D`) plus texture-backed sprites with a simple wgpu pipeline.
- Implemented texture loading helpers on the renderer alongside sprite drawing support and camera-aware transforms.
- Updated the `basic_game` example to render a bouncing, textured sprite using the new camera and sprite APIs.
### Changed
- Embedded the example sprite texture bytes directly into the code to avoid shipping a binary asset file.
