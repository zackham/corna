use wayland_client::{protocol::wl_surface::WlSurface, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowId {
    Clock,
    Timer,
}

#[derive(Debug, Clone, Copy)]
pub enum AnchorPoint {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Debug, Clone, Copy)]
pub enum RelativePosition {
    LeftOf { gap: i32 },
    RightOf { gap: i32 },
    Above { gap: i32 },
    Below { gap: i32 },
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub id: WindowId,
    pub size: [u32; 2],
    pub position: PositionConfig,
    pub layer: zwlr_layer_shell_v1::Layer,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum PositionConfig {
    Anchored {
        anchor: AnchorPoint,
        margin: [i32; 4], // top, right, bottom, left
    },
    RelativeTo {
        window: WindowId,
        position: RelativePosition,
    },
    Absolute {
        x: i32,
        y: i32,
    },
}

pub struct ManagedWindow {
    pub surface: WlSurface,
    pub layer_surface: ZwlrLayerSurfaceV1,
    pub config: WindowConfig,
    pub actual_position: [i32; 2], // Calculated position
}

pub struct WindowManager {
    windows: HashMap<WindowId, ManagedWindow>,
    screen_size: [u32; 2],
}

impl WindowManager {
    pub fn new(screen_size: [u32; 2]) -> Self {
        Self {
            windows: HashMap::new(),
            screen_size,
        }
    }

    pub fn create_window(
        &mut self,
        config: WindowConfig,
        surface: WlSurface,
        layer_shell: &ZwlrLayerShellV1,
        qh: &QueueHandle<crate::wayland::WaylandState>,
    ) -> &ManagedWindow {
        // Calculate actual position based on config
        let actual_position = self.calculate_position(&config);

        // Create layer surface
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            config.layer,
            config.name.clone(),
            qh,
            (),
        );

        // Configure based on position type
        match &config.position {
            PositionConfig::Anchored { anchor, margin } => {
                let wl_anchor = match anchor {
                    AnchorPoint::TopLeft => {
                        zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left
                    }
                    AnchorPoint::TopRight => {
                        zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Right
                    }
                    AnchorPoint::BottomLeft => {
                        zwlr_layer_surface_v1::Anchor::Bottom | zwlr_layer_surface_v1::Anchor::Left
                    }
                    AnchorPoint::BottomRight => {
                        zwlr_layer_surface_v1::Anchor::Bottom | zwlr_layer_surface_v1::Anchor::Right
                    }
                    AnchorPoint::Center => zwlr_layer_surface_v1::Anchor::empty(),
                };

                layer_surface.set_anchor(wl_anchor);
                layer_surface.set_margin(margin[0], margin[1], margin[2], margin[3]);
            }
            PositionConfig::RelativeTo { .. } | PositionConfig::Absolute { .. } => {
                // For relative/absolute positioning, we anchor top-left and use margins
                layer_surface.set_anchor(
                    zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left
                );
                layer_surface.set_margin(actual_position[1], 0, 0, actual_position[0]);
            }
        }

        layer_surface.set_exclusive_zone(0);
        layer_surface.set_size(config.size[0], config.size[1]);

        surface.commit();

        let window = ManagedWindow {
            surface: surface.clone(),
            layer_surface,
            config: config.clone(),
            actual_position,
        };

        self.windows.insert(config.id, window);
        self.windows.get(&config.id).unwrap()
    }

    pub fn destroy_window(&mut self, id: WindowId) {
        if let Some(window) = self.windows.remove(&id) {
            window.layer_surface.destroy();
            window.surface.destroy();
        }
    }

    pub fn get_window(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut ManagedWindow> {
        self.windows.get_mut(&id)
    }

    fn calculate_position(&self, config: &WindowConfig) -> [i32; 2] {
        match &config.position {
            PositionConfig::Anchored { anchor, margin } => {
                // For anchored windows, position is handled by Wayland
                // Return the effective position for reference
                match anchor {
                    AnchorPoint::TopLeft => [margin[3], margin[0]],
                    AnchorPoint::TopRight => [
                        self.screen_size[0] as i32 - config.size[0] as i32 - margin[1],
                        margin[0],
                    ],
                    AnchorPoint::BottomLeft => [
                        margin[3],
                        self.screen_size[1] as i32 - config.size[1] as i32 - margin[2],
                    ],
                    AnchorPoint::BottomRight => [
                        self.screen_size[0] as i32 - config.size[0] as i32 - margin[1],
                        self.screen_size[1] as i32 - config.size[1] as i32 - margin[2],
                    ],
                    AnchorPoint::Center => [
                        (self.screen_size[0] as i32 - config.size[0] as i32) / 2,
                        (self.screen_size[1] as i32 - config.size[1] as i32) / 2,
                    ],
                }
            }
            PositionConfig::RelativeTo { window, position } => {
                if let Some(ref_window) = self.windows.get(window) {
                    let ref_pos = ref_window.actual_position;
                    let ref_size = ref_window.config.size;

                    match position {
                        RelativePosition::LeftOf { gap } => [
                            ref_pos[0] - config.size[0] as i32 - gap,
                            ref_pos[1],
                        ],
                        RelativePosition::RightOf { gap } => [
                            ref_pos[0] + ref_size[0] as i32 + gap,
                            ref_pos[1],
                        ],
                        RelativePosition::Above { gap } => [
                            ref_pos[0],
                            ref_pos[1] - config.size[1] as i32 - gap,
                        ],
                        RelativePosition::Below { gap } => [
                            ref_pos[0],
                            ref_pos[1] + ref_size[1] as i32 + gap,
                        ],
                    }
                } else {
                    // Fallback to top-left if reference window doesn't exist
                    [0, 0]
                }
            }
            PositionConfig::Absolute { x, y } => [*x, *y],
        }
    }

    pub fn update_screen_size(&mut self, size: [u32; 2]) {
        self.screen_size = size;
        // Could recalculate positions here if needed
    }
}