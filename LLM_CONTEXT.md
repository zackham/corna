# Corna - LLM Development Context

## Project Overview
Corna is a minimal Wayland layer-shell widget that displays a seven-segment digital clock in the desktop corner with an integrated pomodoro timer and psychedelic completion animations.

## Architecture
- **Pure Wayland + OpenGL ES 2.0** - No GUI frameworks (no GTK/Qt/egui)
- **Layer-shell positioning** - Top-right corner, 122x46px collapsed size
- **Immediate-mode rendering** - Custom draw API with GLSL shaders
- **Feature modules** - Clock and Pomodoro timer implementations

## Key Components

### Core Systems
- `src/main.rs` - Event loop, Wayland setup, window management
- `src/app.rs` - Application state, input handling, UI modes
- `src/config.rs` - Configuration loading/saving via TOML

### Wayland Integration
- `src/wayland/mod.rs` - Wayland protocol handlers
- `src/wayland/egl.rs` - EGL context management
- `src/wayland/window_manager.rs` - Multi-surface window management

### Graphics Pipeline
- `src/gfx/draw.rs` - Immediate-mode drawing API
- `src/gfx/gl.rs` - Shader compilation utilities
- `src/gfx/math.rs` - Vec2, Rect, Color types
- `src/gfx/anim.rs` - Timeline-based animations
- `assets/shaders/` - GLSL vertex/fragment shaders

### Features
- `src/features/clock.rs` - Seven-segment clock with 11 color themes
- `src/features/pomodoro.rs` - Timer with durations (5-30 min) and plasma effects

## Input Controls
- **Left click** - Toggle seconds display
- **Right click** - Start/stop pomodoro timer
- **Scroll on clock** - Cycle through color themes
- **Scroll on timer** - Change timer duration

## Building & Running
```bash
cargo build --release
./restart.sh  # Rebuild and restart in background
```

## Configuration
Config file: `~/.config/corna/config.toml`
- Window positioning and sizing
- Color themes
- Animation settings

## Dependencies
- wayland-client, wayland-protocols, wayland-egl
- khronos-egl, glow (OpenGL bindings)
- time, serde, toml
- No heavy GUI frameworks

## Development Notes
- Frame callbacks drive rendering (no busy loops)
- Multiple layer surfaces for timer window
- Fractional scaling support planned
- Plasma shader uses multiple noise layers for psychedelic effect
- Timer creates separate 80x30px window when active

## Current State
- Clock fully functional with color themes
- Pomodoro timer working with multiple durations
- Plasma completion animation implemented
- Input handling complete (click, scroll)
- Published to GitHub: github.com/zackham/corna