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
