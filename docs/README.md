# Forge2D Documentation

Welcome to the Forge2D documentation! This guide will help you get started with creating 2D games using Forge2D.

## Table of Contents

1. [Getting Started](getting-started.md) - Quick start guide and basic concepts
2. [Engine & Game Loop](engine.md) - Engine configuration and the Game trait
3. [Input System](input.md) - Keyboard and mouse input handling
4. [Rendering](rendering.md) - Sprites, textures, cameras, and text rendering
5. [Math Utilities](math.md) - Vec2, Transform2D, and Camera2D
6. [Asset Management](assets.md) - Loading and caching textures and other assets
7. [Audio System](audio.md) - Playing sound effects and music
8. [Fixed Timestep](fixed-timestep.md) - Deterministic game updates
9. [State Management](state-management.md) - State/scene system for menus, gameplay, pause, etc.
10. [World & Entities](world.md) - Simple world/entity layer for centralized data
11. [API Reference](api-reference.md) - Complete API documentation
12. [Examples](examples.md) - Code examples and tutorials

## Quick Links

- [Installation & Setup](getting-started.md#installation)
- [Your First Game](getting-started.md#your-first-game)
- [Common Patterns](examples.md#common-patterns)
- [Performance Tips](rendering.md#performance-notes)

## What is Forge2D?

Forge2D is a lightweight, modern 2D game framework built with Rust, winit, and wgpu. It provides a clean, simple API for creating 2D games with minimal boilerplate.

### Key Features

- ✅ Cross-platform window management
- ✅ Frame-accurate input system
- ✅ Hardware-accelerated 2D rendering
- ✅ Efficient batched sprite rendering
- ✅ Text rendering with TTF/OTF fonts
- ✅ 2D camera system
- ✅ Asset caching
- ✅ Audio support
- ✅ Fixed timestep for deterministic physics

### Requirements

- **Rust**: 2021 edition or later
- **GPU**: Graphics drivers installed (wgpu requirement)
- **Platforms**: Windows, macOS, Linux

## Getting Help

- Check the [Examples](examples.md) for code samples
- Review the [API Reference](api-reference.md) for detailed method documentation
- See the `examples/basic_game/` directory for a complete working example

