# Minecraft RustVersion

A Minecraft-like game written in Rust with procedural noise generation and visual node editor.

## Features

- Noise generation engine with visual node editor
- Procedural terrain generation
- Real-time preview system

## Structure

- `Noise/` - Workspace for the noise generation system
  - `engine/` - Core noise generation library
  - `Editor/` - Visual node editor built with Bevy + egui
- `minecraft_rust/` - Main game implementation

## Building

```bash
cd Noise
cargo run -p noise_editor
```

## Development

This project is currently in active development.