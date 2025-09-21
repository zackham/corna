mod app;
mod config;
mod features;
mod gfx;
mod wayland;

use anyhow::Result;
use app::{App, UiEvent, UiMode};
use config::Config;
use features::{clock::Clock, pomodoro::Pomodoro};
use gfx::{draw::DrawContext, gl::load_shader_program, math::{Rect, Vec2}};
use log::info;
use std::time::Instant;
use wayland::egl::EglContext;
use wayland::WaylandState;
use wayland_client::{Connection, Dispatch, QueueHandle, Proxy};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};
use wayland_client::protocol::{
    wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat,
    wl_surface,
};
use xkbcommon::xkb::{self, Context, Keymap, State as XkbState, CONTEXT_NO_FLAGS as FFI_CONTEXT_NO_FLAGS, KEYMAP_COMPILE_NO_FLAGS as FFI_KEYMAP_COMPILE_NO_FLAGS, keysyms};

fn main() -> Result<()> {
    env_logger::init();
    println!("Starting corna...");

    // Load config
    let config = Config::load().unwrap_or_default();
    let mut app = App::new(config);

    // Connect to Wayland
    println!("Connecting to Wayland...");
    let conn = Connection::connect_to_env()?;
    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, ());

    let mut state = WaylandState::new(&qh);

    // Initial roundtrip to get globals
    println!("Getting Wayland globals...");
    event_queue.roundtrip(&mut state)?;

    if let Some(size) = state.output_size {
        app.set_screen_size(size);
    }

    // Create surface
    println!("Creating surface...");
    if let Some(compositor) = &state.compositor {
        let surface = compositor.create_surface(&qh, ());
        state.surface = Some(surface.clone());

        // Create layer surface
        if let Some(layer_shell) = &state.layer_shell {
            let layer_surface = layer_shell.get_layer_surface(
                &surface,
                None,
                zwlr_layer_shell_v1::Layer::Overlay,
                "corna".to_string(),
                &qh,
                (),
            );

            // Configure layer surface for top-right corner
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Right,
            );
            layer_surface.set_exclusive_zone(0);
            layer_surface.set_margin(0, 0, 0, 0);
            layer_surface.set_size(150, 60);  // Match the default collapsed size

            surface.commit();

            state.layer_surface = Some(layer_surface);
        }
    }


    // Wait for configure
    println!("Waiting for surface configuration...");
    while !state.configured {
        event_queue.blocking_dispatch(&mut state)?;
    }
    println!("Surface configured!");

    // Initialize EGL
    println!("Initializing EGL...");
    let display_ptr = display.id().as_ptr() as *mut _;
    let mut egl = EglContext::new(display_ptr)?;

    println!("Creating EGL surface...");
    if let Some(surface) = &state.surface {
        let size = app.get_current_size();
        egl.create_surface(surface, size[0] as i32, size[1] as i32)?;
        egl.make_current()?;
    }

    // Create GL context
    println!("Creating GL context...");
    let gl = unsafe {
        glow::Context::from_loader_function(|s| egl.get_proc_address(s))
    };

    // Load shaders
    println!("Loading shaders...");
    let vert_src = std::fs::read_to_string("assets/shaders/ui.vert.glsl")?;
    let frag_src = std::fs::read_to_string("assets/shaders/ui.frag.glsl")?;
    let program = load_shader_program(&gl, &vert_src, &frag_src)?;

    // Create draw context
    let mut draw_context = DrawContext::new(gl, program)?;

    let mut clock = Clock::new();

    // Timer window variables
    let mut timer_egl: Option<EglContext> = None;
    let mut timer_draw_context: Option<DrawContext> = None;
    let mut timer_window_active = false;

    // Plasma window variables
    let mut plasma_egl: Option<EglContext> = None;
    let mut plasma_draw_context: Option<DrawContext> = None;
    let mut plasma_window_active = false;

    let mut last_frame = Instant::now();

    // Main loop
    println!("Starting main loop...");
    let mut previous_size = [100u32, 40u32];
    let mut previous_clock_width = app.get_current_size()[0];

    while state.running {
        event_queue.dispatch_pending(&mut state)?;

        // Handle input events
        for ev in state.pending_events.drain(..) {
            app.handle_event(ev);
        }

        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        app.update(dt);
        clock.update(dt, app.time);
        app.pomodoro.update(app.time);

        // Create/destroy timer window based on pomodoro state
        let should_show_timer = matches!(app.pomodoro.mode, crate::features::pomodoro::PomodoroMode::Counting { .. });

        if should_show_timer && !timer_window_active {
            // Create timer surface
            if let (Some(compositor), Some(layer_shell)) = (&state.compositor, &state.layer_shell) {
                let timer_surface = compositor.create_surface(&event_queue.handle(), ());
                let timer_layer = layer_shell.get_layer_surface(
                    &timer_surface,
                    state.output.as_ref(),
                    zwlr_layer_shell_v1::Layer::Top,
                    "corna-timer".to_string(),
                    &event_queue.handle(),
                    (),
                );

                // Position timer window properly to the left of clock
                // Use actual clock size from app.get_current_size()
                let clock_size = app.get_current_size();
                const TIMER_WIDTH: u32 = 80;
                const TIMER_HEIGHT: u32 = 30;
                const GAP: u32 = 10;

                if let Some(screen_size) = state.output_size {
                    // Clock is at top-right, timer should be to its left
                    let timer_x_margin = screen_size[0] as i32 - clock_size[0] as i32 - TIMER_WIDTH as i32 - GAP as i32;
                    timer_layer.set_anchor(
                        zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left,
                    );
                    timer_layer.set_margin(0, 0, 0, timer_x_margin);
                } else {
                    // Fallback positioning
                    timer_layer.set_anchor(
                        zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Right,
                    );
                    timer_layer.set_margin(0, clock_size[0] as i32 + GAP as i32, 0, 0);
                }

                timer_layer.set_exclusive_zone(0);
                timer_layer.set_size(TIMER_WIDTH, TIMER_HEIGHT);

                timer_surface.commit();
                state.timer_surface = Some(timer_surface);
                state.timer_layer_surface = Some(timer_layer);

                // Wait for timer surface to be configured
                event_queue.roundtrip(&mut state)?;

                timer_window_active = true;

                // Create EGL context for timer after configuration
                if let Some(timer_surf) = &state.timer_surface {
                    let mut timer_egl_ctx = EglContext::new_shared(display_ptr)?;
                    timer_egl_ctx.create_surface(timer_surf, 80, 30)?;
                    timer_egl_ctx.make_current()?;

                    let timer_gl = unsafe {
                        glow::Context::from_loader_function(|s| timer_egl_ctx.get_proc_address(s))
                    };
                    let timer_program = load_shader_program(&timer_gl, &vert_src, &frag_src)?;
                    timer_draw_context = Some(DrawContext::new(timer_gl, timer_program)?);
                    timer_egl = Some(timer_egl_ctx);
                }
            }
        } else if !should_show_timer && timer_window_active {
            info!("Destroying timer window...");

            // Switch back to main context before destroying timer
            info!("Switching to main EGL context...");
            egl.make_current()?;
            info!("Switched to main EGL context");

            // Clean up timer EGL resources first
            info!("Cleaning up timer EGL resources...");
            timer_draw_context = None;
            timer_egl = None;
            info!("Timer EGL resources cleaned up");

            // Then destroy timer surfaces
            info!("Destroying timer surfaces...");
            if let Some(layer) = state.timer_layer_surface.take() {
                layer.destroy();
            }
            if let Some(surf) = state.timer_surface.take() {
                surf.destroy();
            }
            info!("Timer surfaces destroyed");

            timer_window_active = false;
        }

        // Create/destroy plasma window for completion effect
        let should_show_plasma = matches!(app.pomodoro.mode, crate::features::pomodoro::PomodoroMode::Completion { .. });

        if should_show_plasma && !plasma_window_active {
            info!("Creating fullscreen plasma window!");
            if let (Some(compositor), Some(layer_shell)) = (&state.compositor, &state.layer_shell) {
                let plasma_surface = compositor.create_surface(&event_queue.handle(), ());
                let plasma_layer = layer_shell.get_layer_surface(
                    &plasma_surface,
                    state.output.as_ref(),
                    zwlr_layer_shell_v1::Layer::Overlay, // Highest layer
                    "corna-plasma".to_string(),
                    &event_queue.handle(),
                    (),
                );

                // Make it fullscreen
                plasma_layer.set_anchor(
                    zwlr_layer_surface_v1::Anchor::Top |
                    zwlr_layer_surface_v1::Anchor::Bottom |
                    zwlr_layer_surface_v1::Anchor::Left |
                    zwlr_layer_surface_v1::Anchor::Right
                );
                plasma_layer.set_exclusive_zone(-1); // Cover everything
                plasma_layer.set_size(0, 0); // Fill entire screen

                plasma_surface.commit();
                state.plasma_surface = Some(plasma_surface);
                state.plasma_layer_surface = Some(plasma_layer);

                // Wait for configuration
                event_queue.roundtrip(&mut state)?;

                plasma_window_active = true;

                // Create EGL context for plasma
                if let Some(plasma_surf) = &state.plasma_surface {
                    let screen_size = state.output_size.unwrap_or([1920, 1080]);
                    let mut plasma_egl_ctx = EglContext::new_shared(display_ptr)?;
                    plasma_egl_ctx.create_surface(plasma_surf, screen_size[0] as i32, screen_size[1] as i32)?;
                    plasma_egl_ctx.make_current()?;

                    let plasma_gl = unsafe {
                        glow::Context::from_loader_function(|s| plasma_egl_ctx.get_proc_address(s))
                    };
                    let plasma_program = load_shader_program(&plasma_gl, &vert_src, &frag_src)?;
                    plasma_draw_context = Some(DrawContext::new(plasma_gl, plasma_program)?);
                    plasma_egl = Some(plasma_egl_ctx);
                }
            }
        } else if !should_show_plasma && plasma_window_active {
            info!("Destroying plasma window");

            // Switch back to main context
            egl.make_current()?;

            // Clean up plasma resources
            plasma_draw_context = None;
            plasma_egl = None;

            // Destroy plasma surfaces
            if let Some(layer) = state.plasma_layer_surface.take() {
                layer.destroy();
            }
            if let Some(surf) = state.plasma_surface.take() {
                surf.destroy();
            }

            plasma_window_active = false;
        }

        // Handle normal resize for main window
        let current_size = app.get_current_size();
        let buffer_size = [
            (current_size[0] as f32 * app.scale) as u32,
            (current_size[1] as f32 * app.scale) as u32,
        ];
        if buffer_size != app.buffer_size || current_size != previous_size {
            app.buffer_size = buffer_size;
            egl.resize(app.buffer_size[0] as i32, app.buffer_size[1] as i32)?;

            if let Some(ref layer_surface) = state.layer_surface {
                layer_surface.set_size(current_size[0], current_size[1]);
            }
            if let Some(ref surface) = state.surface {
                surface.commit();
            }
            previous_size = current_size;
        }

        // Render
        egl.make_current()?;

        let size = app.buffer_size.map(|x| x as f32);
        draw_context.begin(size);
        draw_context.set_time(app.time);

        let viewport = Rect::new(0.0, 0.0, size[0], size[1]);
        // Pass show_seconds flag, color_mode and time to clock
        clock.render(&mut draw_context, viewport, app.show_seconds, app.color_mode, app.time);

        draw_context.flush();

        // Swap buffers for main window
        egl.swap_buffers()?;

        // Render plasma window if active (FULLSCREEN)
        if plasma_window_active {
            if let (Some(ref mut plasma_egl_ctx), Some(ref mut plasma_draw)) = (&mut plasma_egl, &mut plasma_draw_context) {
                plasma_egl_ctx.make_current()?;
                let screen_size = state.output_size.unwrap_or([1920, 1080]);
                let plasma_viewport = Rect::new(0.0, 0.0, screen_size[0] as f32, screen_size[1] as f32);

                // Pass completion progress to shader for fade in/out BEFORE begin
                let progress = if let crate::features::pomodoro::PomodoroMode::Completion { tl, .. } = &app.pomodoro.mode {
                    tl.progress()
                } else {
                    1.0
                };

                plasma_draw.begin([screen_size[0] as f32, screen_size[1] as f32]);
                plasma_draw.set_time(app.time);
                plasma_draw.set_progress(progress);

                // Render the FULLSCREEN plasma effect
                app.pomodoro.render(plasma_draw, plasma_viewport, app.time);

                plasma_draw.flush();
                plasma_egl_ctx.swap_buffers()?;

                if let Some(plasma_surf) = &state.plasma_surface {
                    plasma_surf.commit();
                }

                // Switch back to main context
                egl.make_current()?;
            }
        }

        // Update timer position if clock width changed
        if timer_window_active {
            let current_clock_width = app.get_current_size()[0];
            if current_clock_width != previous_clock_width {
                // Clock width changed, update timer position
                if let (Some(ref timer_layer), Some(screen_size)) = (&state.timer_layer_surface, state.output_size) {
                    const TIMER_WIDTH: u32 = 80;
                    const GAP: u32 = 10;
                    let timer_x_margin = screen_size[0] as i32 - current_clock_width as i32 - TIMER_WIDTH as i32 - GAP as i32;
                    timer_layer.set_margin(0, 0, 0, timer_x_margin);
                    if let Some(timer_surf) = &state.timer_surface {
                        timer_surf.commit();
                    }
                    info!("Updated timer position due to clock width change: {} -> {}", previous_clock_width, current_clock_width);
                }
                previous_clock_width = current_clock_width;
            }
        }

        // Render timer window if active
        if timer_window_active {
            if let (Some(ref mut timer_egl_ctx), Some(ref mut timer_draw)) = (&mut timer_egl, &mut timer_draw_context) {
                timer_egl_ctx.make_current()?;
                let timer_viewport = Rect::new(0.0, 0.0, 80.0, 30.0);
                timer_draw.begin([80.0, 30.0]);
                timer_draw.set_time(app.time);

                // Render just the timer display
                app.pomodoro.render(timer_draw, timer_viewport, app.time);

                timer_draw.flush();
                timer_egl_ctx.swap_buffers()?;

                if let Some(timer_surf) = &state.timer_surface {
                    timer_surf.commit();
                }

                // Switch back to main context
                egl.make_current()?;
            }
        }

        // Commit surface
        if let Some(surface) = &state.surface {
            surface.commit();
        }

        // Sleep briefly to cap framerate
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}