# Forge2D

Forge2D is a lightweight 2D game framework. Phase 1 focuses on providing a clean public API, opening a window with winit, and running a simple game loop.

## Layout
- `forge2d/`: the engine crate containing the public API.
- `examples/basic_game/`: a small example that spins up the engine and exits after a few seconds.

## Getting started
Ensure you have Rust installed (edition 2021). From the repository root:

```bash
cargo run -p basic_game
```

This launches the example, prints per-frame timing to the console, and exits automatically after three seconds.
