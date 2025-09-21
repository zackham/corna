pub mod egl;
pub mod window_manager;

use wayland_client::{
    protocol::{wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};
use crate::app::UiEvent;
use crate::gfx::math::Vec2;
use xkbcommon::xkb::{self, Context, Keymap, State as XkbState, CONTEXT_NO_FLAGS as FFI_CONTEXT_NO_FLAGS, KEYMAP_COMPILE_NO_FLAGS as FFI_KEYMAP_COMPILE_NO_FLAGS};
use xkbcommon::xkb::keysyms;
use std::os::unix::io::{RawFd, AsRawFd};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveSurface {
    Clock,
    Timer,
    Plasma,
}

pub struct WaylandState {
    pub running: bool,
    pub configured: bool,
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub surface: Option<wl_surface::WlSurface>,
    pub layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub timer_surface: Option<wl_surface::WlSurface>,
    pub timer_layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub plasma_surface: Option<wl_surface::WlSurface>,
    pub plasma_layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub seat: Option<wl_seat::WlSeat>,
    pub output: Option<wl_output::WlOutput>,
    pub output_size: Option<[u32; 2]>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub surface_pos: Vec2,
    pub pending_events: Vec<UiEvent>,
    pub xkb_context: Context,
    pub xkb_keymap: Option<Keymap>,
    pub xkb_state: Option<XkbState>,
    pub active_surface: Option<ActiveSurface>,
}

impl WaylandState {
    pub fn new(_qh: &QueueHandle<Self>) -> Self {
        Self {
            running: true,
            configured: false,
            compositor: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            timer_surface: None,
            timer_layer_surface: None,
            plasma_surface: None,
            plasma_layer_surface: None,
            seat: None,
            output: None,
            output_size: None,
            pointer: None,
            keyboard: None,
            surface_pos: Vec2 { x: 0.0, y: 0.0 },
            pending_events: Vec::new(),
            xkb_context: Context::new(FFI_CONTEXT_NO_FLAGS),
            xkb_keymap: None,
            xkb_state: None,
            active_surface: None,
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "wl_compositor" => {
                    let compositor = registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    );
                    state.compositor = Some(compositor);
                }
                "zwlr_layer_shell_v1" => {
                    let layer_shell = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    );
                    state.layer_shell = Some(layer_shell);
                }
                "wl_seat" => {
                    let seat = registry.bind::<wl_seat::WlSeat, _, _>(
                        name,
                        version.min(5),
                        qh,
                        (),
                    );
                    state.seat = Some(seat);
                }
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(
                        name,
                        version.min(2),
                        qh,
                        (),
                    );
                    state.output = Some(output);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandState {
    fn event(_: &mut Self, _: &wl_compositor::WlCompositor, _: wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandState {
    fn event(_: &mut Self, _: &wl_surface::WlSurface, _: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface_x, surface_y, surface, .. } => {
                state.surface_pos.x = surface_x as f32;
                state.surface_pos.y = surface_y as f32;

                // Determine which surface the pointer entered
                if let Some(ref main_surface) = state.surface {
                    if surface == *main_surface {
                        state.active_surface = Some(ActiveSurface::Clock);
                    }
                }
                if let Some(ref timer_surf) = state.timer_surface {
                    if surface == *timer_surf {
                        state.active_surface = Some(ActiveSurface::Timer);
                    }
                }
                if let Some(ref plasma_surf) = state.plasma_surface {
                    if surface == *plasma_surf {
                        state.active_surface = Some(ActiveSurface::Plasma);
                    }
                }

                state.pending_events.push(UiEvent::PointerEnter { pos: state.surface_pos });
            }
            wl_pointer::Event::Leave { .. } => {
                state.pending_events.push(UiEvent::PointerLeave);
                state.surface_pos = Vec2::new(0.0, 0.0);
                state.active_surface = None;
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.surface_pos.x = surface_x as f32;
                state.surface_pos.y = surface_y as f32;
                state.pending_events.push(UiEvent::PointerMove { pos: state.surface_pos });
            }
            wl_pointer::Event::Button { button, state: btn_state, .. } => {
                if button == 0x110 || button == 0x111 {  // BTN_LEFT or BTN_RIGHT
                    let ev = match btn_state {
                        wayland_client::WEnum::Value(wl_pointer::ButtonState::Pressed) => UiEvent::PointerDown { pos: state.surface_pos, button },
                        wayland_client::WEnum::Value(wl_pointer::ButtonState::Released) => UiEvent::PointerUp,
                        _ => return,
                    };
                    state.pending_events.push(ev);
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                // Handle scroll wheel events
                if let wayland_client::WEnum::Value(wl_pointer::Axis::VerticalScroll) = axis {
                    // Negative value = scroll up, positive = scroll down
                    let delta = if value < 0.0 { 1.0 } else { -1.0 };
                    // Include which surface the scroll happened on
                    state.pending_events.push(UiEvent::Scroll {
                        delta,
                        surface: state.active_surface,
                    });
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_seat::Event::Capabilities { capabilities } => {
                if let wayland_client::WEnum::Value(caps) = capabilities {
                    if caps.contains(wl_seat::Capability::Pointer) {
                        state.pointer = Some(seat.get_pointer(qh, ()));
                    }
                    if caps.contains(wl_seat::Capability::Keyboard) {
                        state.keyboard = Some(seat.get_keyboard(qh, ()));
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Temporarily disable keyboard handling to avoid crashes
        // TODO: Fix keymap parsing issue
        match event {
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Mode { width, height, .. } = event {
            state.output_size = Some([width as u32, height as u32]);
        }
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for WaylandState {
    fn event(_: &mut Self, _: &zwlr_layer_shell_v1::ZwlrLayerShellV1, _: zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        println!("Layer surface event received: {:?}", event);
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                println!("Layer surface configured: width={}, height={}, serial={}", width, height, serial);
                if width > 0 && height > 0 {
                    // state.size = (width, height);  // Unused now
                }
                surface.ack_configure(serial);
                state.configured = true;
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => {}
        }
    }
}