# TODO

**Current phase:** Phase 5 – Assets and audio (complete)

## Phase 1 – Core engine API + game loop skeleton
- [x] Define public entrypoints (`Engine`, `EngineConfig`, `Game`, `EngineContext`).
- [x] Implement winit-driven game loop with window creation and exit handling.
- [x] Provide basic timing context (delta/total time) and stub input/reference hooks.
- [x] Add example game that runs the loop and logs timing.

## Phase 2 – Input system
- [x] Implement `input.rs` with key/mouse state tracking (down/pressed/released and cursor position).
- [x] Integrate winit event handling to update `InputState` each frame.
- [x] Expose input querying through `EngineContext` and validate via example usage.

## Phase 3 – Basic 2D rendering with wgpu
- [x] Add `render` module with `Renderer` wrapper over `WgpuBackend` (init, resize, begin/clear/end frame).
- [x] Initialize the renderer in the engine loop and surface errors gracefully.
- [x] Extend `EngineContext` with renderer access and update example to clear the screen.

## Phase 4 – Sprites, textures, and camera
- [x] Introduce math helpers (`Vec2`, `Transform2D`) for positioning and scaling.
- [x] Implement sprite and camera types with rendering support (`draw_sprite`).
- [x] Load PNG textures and render moving sprites using a simple camera in an example.

## Phase 5 – Assets and audio
- [x] Add `AssetManager` to cache textures (and other future assets) keyed by path/ID.
- [x] Wire asset loading helpers into `EngineContext` for renderer access.
- [x] Create `audio.rs` with `AudioSystem` wrapping `rodio` and expose playback APIs.

## Phase 6 – State/scene system
- [ ] Define `State` trait (`on_enter`, `on_exit`, `update`, `draw`) and `StateMachine` management.
- [ ] Integrate state management into the engine loop or game wrapper.
- [ ] Update examples to demonstrate menu/gameplay state transitions.

## Phase 7 – Optional ECS
- [ ] Evaluate integrating an ECS (e.g., `hecs`) for component-based entities.
- [ ] Define core components (transform, sprite, velocity) and systems (movement, rendering, input-driven actions).
- [ ] Provide a sample showing ECS-driven gameplay once earlier phases stabilize.
