use crate::config::Config;
use crate::features::pomodoro::PomodoroMode;
use crate::gfx::{anim::{Timeline, lerp}, draw::DrawContext, math::{Color, Rect, Vec2}};
use crate::wayland::ActiveSurface;
use anyhow::Result;
use log::info;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiMode {
    Collapsed,
    Expanding,
    Expanded,
    Collapsing,
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    PointerEnter { pos: Vec2 },
    PointerLeave,
    PointerMove { pos: Vec2 },
    PointerDown { pos: Vec2, button: u32 },
    PointerUp,
    Scroll { delta: f32, surface: Option<ActiveSurface> },
    Key(u32),
}

pub struct App {
    pub config: Config,
    pub mode: UiMode,
    pub scale: f32,
    pub logical_size: [u32; 2],
    pub buffer_size: [u32; 2],
    pub expand_timeline: Timeline,
    pub hover: bool,
    pub last_frame_time: f32,
    pub time: f32,

    // Click detection
    pub last_click_time: f32,
    pub click_count: u32,

    // Pomodoro
    pub pomodoro: crate::features::pomodoro::Pomodoro,
    pub screen_size: Option<[u32; 2]>,

    // Clock settings
    pub show_seconds: bool,
    pub color_mode: u8,
}

impl App {
    pub fn new(config: Config) -> Self {
        let logical_size = [config.collapsed_size.width, config.collapsed_size.height];
        Self {
            config,
            mode: UiMode::Collapsed,
            scale: 1.0,
            logical_size,
            buffer_size: logical_size,
            expand_timeline: Timeline::new(0.15), // 150ms animation
            hover: false,
            last_frame_time: 0.0,
            time: 0.0,
            last_click_time: 0.0,
            click_count: 0,
            pomodoro: crate::features::pomodoro::Pomodoro::new(),
            screen_size: None,
            show_seconds: true,
            color_mode: 0,
        }
    }

    pub fn set_screen_size(&mut self, size: [u32; 2]) {
        self.screen_size = Some(size);
    }

    pub fn start_pomodoro(&mut self) {
        self.pomodoro.start(self.time);
    }

    pub fn toggle_expand(&mut self) {
        match self.mode {
            UiMode::Collapsed => {
                self.mode = UiMode::Expanding;
                self.expand_timeline.start(self.time);
            }
            UiMode::Expanded => {
                self.mode = UiMode::Collapsing;
                self.expand_timeline.start(self.time);
            }
            _ => {}
        }
    }

    pub fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::PointerEnter { .. } => {
                self.hover = true;
            }
            UiEvent::PointerLeave => {
                self.hover = false;
            }
            UiEvent::PointerDown { button, .. } => {
                // Right click (BTN_RIGHT = 0x111) starts/stops Pomodoro timer
                if button == 0x111 {
                    info!("Right click detected! Button: {:#x}, Mode: {:?}", button, self.pomodoro.mode);
                    if matches!(self.pomodoro.mode, PomodoroMode::Idle) {
                        info!("Starting pomodoro from right click");
                        self.start_pomodoro();
                    } else if matches!(self.pomodoro.mode, PomodoroMode::Counting { .. }) {
                        // If already running, stop the timer (go back to idle)
                        info!("Stopping pomodoro from right click");
                        self.pomodoro.stop();
                    }
                    return;
                }

                // Left click (BTN_LEFT = 0x110) toggles seconds display
                if button == 0x110 {
                    self.show_seconds = !self.show_seconds;
                    info!("Toggled seconds display: {}", self.show_seconds);
                }
            }
            UiEvent::Scroll { delta, surface } => {
                info!("Scroll event: delta={}, surface={:?}", delta, surface);
                // Handle scroll based on which surface we're over
                match surface {
                    Some(ActiveSurface::Clock) => {
                        // Cycle through color modes on clock surface
                        const NUM_MODES: u8 = 11; // Total number of color modes
                        if delta > 0.0 {
                            self.color_mode = (self.color_mode + 1) % NUM_MODES;
                        } else if delta < 0.0 {
                            self.color_mode = if self.color_mode == 0 {
                                NUM_MODES - 1
                            } else {
                                self.color_mode - 1
                            };
                        }
                        info!("Changed color mode to: {}", self.color_mode);
                    }
                    Some(ActiveSurface::Timer) => {
                        // Cycle through timer durations on timer surface
                        self.pomodoro.cycle_duration(delta);
                    }
                    _ => {} // Ignore scroll on other surfaces or no surface
                }
            }
            UiEvent::Key(_key) => {
                // No key handling needed anymore since we removed expand mode
            }
            _ => {}
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        // Reset click count
        if self.time - self.last_click_time > 0.5 {
            self.click_count = 0;
        }

        // Update animation timeline
        if matches!(self.mode, UiMode::Expanding | UiMode::Collapsing) {
            self.expand_timeline.update(self.time);

            if self.expand_timeline.is_complete() {
                self.mode = match self.mode {
                    UiMode::Expanding => UiMode::Expanded,
                    UiMode::Collapsing => UiMode::Collapsed,
                    _ => self.mode,
                };

                self.logical_size = if matches!(self.mode, UiMode::Expanded) {
                    [self.config.expanded_size.width, self.config.expanded_size.height]
                } else {
                    [self.config.collapsed_size.width, self.config.collapsed_size.height]
                };
            }
        }
    }

    pub fn get_current_size(&self) -> [u32; 2] {
        // Calculate width based on whether seconds are shown
        // Keep height constant at 60
        let width = if self.show_seconds {
            220  // Width with seconds (6 digits + 2 colons)
        } else {
            150  // Width without seconds (4 digits + 1 colon)
        };
        [width, 60]
    }

    pub fn render(&self, _draw: &mut DrawContext) {}

    fn render_clock(&self, draw: &mut DrawContext) {
        let text_color = Color::rgba(255, 255, 255, 255);
        draw.rect(
            20.0,
            20.0,
            self.buffer_size[0] as f32 - 40.0,
            40.0,
            text_color,
        );
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
        self.buffer_size = [
            (self.logical_size[0] as f32 * scale) as u32,
            (self.logical_size[1] as f32 * scale) as u32,
        ];
    }
}