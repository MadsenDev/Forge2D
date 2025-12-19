# Forge2D Editor

The visual editor for Forge2D, built with Tauri, React, and TypeScript.

## Setup

### Prerequisites

- Rust (latest stable)
- Node.js 18+ and npm
- Tauri CLI: `cargo install tauri-cli` (or use `cargo tauri` if using Tauri v2)

### Installation

1. Install Rust dependencies:
```bash
cargo build
```

2. Install Node.js dependencies:
```bash
npm install
```

## Development

Run the editor in development mode:

```bash
npm run dev
```

Or use Tauri CLI directly:

```bash
cargo tauri dev
```

This will:
- Start the Vite dev server on `http://localhost:1420`
- Build and run the Tauri application
- Hot-reload the frontend on changes

## Building

Build for production:

```bash
npm run build
cargo tauri build
```

## Project Structure

```
editor/
├── src/              # Rust backend (Tauri)
│   └── main.rs      # IPC commands and state
├── src/              # React frontend
│   ├── App.tsx      # Main app component
│   └── main.tsx     # React entry point
├── Cargo.toml       # Rust dependencies
├── package.json     # Node.js dependencies
└── tauri.conf.json  # Tauri configuration
```

## Current Features

- ✅ Entity list panel
- ✅ Create entity command
- ✅ Undo/redo system
- ✅ Basic UI layout

## Coming Soon

- Viewport rendering
- Entity selection
- Transform gizmo
- Inspector panel
- Scene save/load

