use crate::app::UiEvent;
use crate::gfx::{anim::{Timeline, lerp}, draw::DrawContext, math::{Color, Rect, Vec2}};
use log::info;

// Reuse clock's segment map
const SEGMENT_MAP: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],     // 0
    [false, true, true, false, false, false, false], // 1
    [true, true, false, true, true, false, true],    // 2
    [true, true, true, true, false, false, true],    // 3
    [false, true, true, false, false, true, true],   // 4
    [true, false, true, true, false, true, true],    // 5
    [true, false, true, true, true, true, true],     // 6
    [true, true, true, false, false, false, false],  // 7
    [true, true, true, true, true, true, true],      // 8
    [true, true, true, true, false, true, true],     // 9
];

#[derive(Debug, Clone)]
pub enum PomodoroMode {
    Idle,
    Reveal { start: f32, tl: Timeline },
    Counting { start: f32 },
    Completion { start: f32, tl: Timeline },
}

pub struct Pomodoro {
    pub mode: PomodoroMode,
    duration: f32,
    remaining: f32,
    minute_digits: [u8; 2],
    second_digits: [u8; 2],
    flip_tl: Timeline,
    last_sec: i32,
    duration_index: usize,
}

impl Pomodoro {
    const DURATIONS: [f32; 6] = [
        30.0 * 60.0,  // 30 minutes
        25.0 * 60.0,  // 25 minutes
        20.0 * 60.0,  // 20 minutes
        15.0 * 60.0,  // 15 minutes
        10.0 * 60.0,  // 10 minutes
        5.0 * 60.0,   // 5 minutes
    ];

    pub fn new() -> Self {
        Self {
            mode: PomodoroMode::Idle,
            duration_index: 0,
            duration: Self::DURATIONS[0],
            remaining: Self::DURATIONS[0],
            minute_digits: [0, 0],
            second_digits: [0, 0],
            flip_tl: Timeline::new(0.12),
            last_sec: -1,
        }
    }

    pub fn start(&mut self, now: f32) {
        self.mode = PomodoroMode::Counting { start: now };
        self.remaining = self.duration;
        self.last_sec = -1;
        info!("Pomodoro started! Mode: {:?}, Duration: {}", self.mode, self.duration);
    }

    pub fn stop(&mut self) {
        info!("Stopping pomodoro timer");
        self.mode = PomodoroMode::Idle;
        self.remaining = self.duration;
        self.last_sec = -1;
    }

    pub fn cycle_duration(&mut self, delta: f32) {
        info!("cycle_duration called with delta: {}, current mode: {:?}", delta, self.mode);
        // Allow duration change when idle OR when counting (will update remaining time)
        // This way users can adjust duration even when timer is running

        if delta > 0.0 {
            // Scroll up - go to next duration
            self.duration_index = (self.duration_index + 1) % Self::DURATIONS.len();
        } else {
            // Scroll down - go to previous duration
            if self.duration_index == 0 {
                self.duration_index = Self::DURATIONS.len() - 1;
            } else {
                self.duration_index -= 1;
            }
        }

        self.duration = Self::DURATIONS[self.duration_index];
        self.remaining = self.duration;

        // Update remaining time if timer is running
        if matches!(self.mode, PomodoroMode::Counting { .. }) {
            // Keep the same proportion of time remaining
            let proportion = self.remaining / Self::DURATIONS[self.duration_index];
            self.remaining = self.duration * proportion;
        }

        let minutes = (self.duration / 60.0) as u32;
        info!("Changed timer duration to: {} minutes (index: {})", minutes, self.duration_index);
    }

    pub fn trigger_completion(&mut self, now: f32) {
        info!("trigger_completion called at time {}, current mode: {:?}", now, self.mode);
        if matches!(self.mode, PomodoroMode::Counting { .. }) {
            self.mode = PomodoroMode::Completion {
                start: now,
                tl: Timeline::new(2.0),
            };
            self.remaining = 0.0;
            info!("Pomodoro completion triggered manually! Mode is now: {:?}", self.mode);
        } else {
            info!("Cannot trigger completion - not in Counting mode");
        }
    }

    pub fn update(&mut self, now: f32) {
        self.flip_tl.update(now);
        match &mut self.mode {
            PomodoroMode::Idle => {}
            PomodoroMode::Reveal { .. } => {
                // This should no longer be used, but keep for compatibility
            }
            PomodoroMode::Counting { start } => {
                self.remaining = (self.duration - (now - *start)).max(0.0);
                let current_sec = self.remaining.floor() as i32;
                if current_sec != self.last_sec {
                    self.last_sec = current_sec;
                    self.flip_tl.start(now);
                }
                if self.remaining <= 0.0 {
                    let mut tl = Timeline::new(5.0);  // 5 seconds of awesome visualization
                    tl.start(now);  // START the timeline!
                    self.mode = PomodoroMode::Completion {
                        start: now,
                        tl,
                    };
                    info!("Pomodoro complete!");
                }
            }
            PomodoroMode::Completion { tl, .. } => {
                tl.update(now);
                if tl.is_complete() {
                    self.mode = PomodoroMode::Idle;
                    info!("Pomodoro completion animation finished");
                }
            }
        }
        self.update_digits();
    }

    fn update_digits(&mut self) {
        let total_sec = self.remaining.floor() as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        self.minute_digits = [(mins / 10) as u8, (mins % 10) as u8];
        self.second_digits = [(secs / 10) as u8, (secs % 10) as u8];
    }

    pub fn render(&self, draw: &mut DrawContext, viewport: Rect, _time: f32) {
        match &self.mode {
            PomodoroMode::Idle => return,
            PomodoroMode::Completion { .. } => {
                draw.set_effect_mode(2);
                draw.rect(0.0, 0.0, viewport.width, viewport.height, Color::rgba(255, 255, 255, 255));
                draw.set_effect_mode(0);
            }
            PomodoroMode::Counting { .. } => {
                // Show blue LCD timer display
                self.render_timer_display(draw, viewport);
            }
            _ => {
                let (reveal_progress, flip_progress) = match &self.mode {
                    PomodoroMode::Reveal { tl, .. } => (tl.eased_progress(), 0.0),
                    PomodoroMode::Counting { .. } => (1.0, self.flip_tl.eased_progress()),
                    _ => (1.0, 0.0),
                };

                // Reveal pattern background
                draw.set_effect_mode(1);
                draw.rect(0.0, 0.0, viewport.width, viewport.height, Color::rgba(0, 0, 0, 255));
                draw.set_effect_mode(0);

                // Timer display (adapted from clock)
                let outer_padding = 8.0;
                let r_w = 0.64;
                let r_s = 0.18;
                let r_c = 0.30;
                let r_m = 1.8;

                let dh_by_h = (viewport.height - outer_padding * 2.0) / (1.0 + 2.0 * r_m * r_s * r_w);
                let denom_w = r_w * (4.0 + 3.0 * r_s + r_c + 2.0 * r_m * r_s);
                let dh_by_w = (viewport.width - outer_padding * 2.0) / denom_w;
                let mut digit_height = dh_by_h.min(dh_by_w).max(0.0);
                digit_height *= reveal_progress; // Scale reveal

                let digit_width = digit_height * r_w;
                let spacing = digit_width * r_s;
                let colon_width = digit_width * r_c;
                let total_width = digit_width * 4.0 + spacing * 3.0 + colon_width;
                let mut margin = spacing * r_m;
                if margin < 5.0 { margin = 5.0; }
                let face_w = total_width + margin * 2.0;
                let face_h = digit_height + margin * 2.0;

                // Center
                let face_x = (viewport.width - face_w) / 2.0;
                let face_y = (viewport.height - face_h) / 2.0;

                draw.rect(face_x, face_y, face_w, face_h, Color::rgba(0, 0, 0, 255));

                let start_x = face_x + margin;
                let start_y = face_y + margin;
                let seg_color = Color::rgba(74, 158, 255, 255); // Accent

                // Minutes
                self.render_digit(draw, self.minute_digits[0], start_x, start_y, digit_width, digit_height, seg_color, 1.0);
                self.render_digit(draw, self.minute_digits[1], start_x + digit_width + spacing, start_y, digit_width, digit_height, seg_color, 1.0);

                // Colon (always visible)
                let colon_x = start_x + digit_width * 2.0 + spacing * 2.0;
                let dot = digit_width * 0.12;
                draw.rect(colon_x, start_y + digit_height * 0.3, dot, dot, seg_color);
                draw.rect(colon_x, start_y + digit_height * 0.6, dot, dot, seg_color);

                // Seconds with flip
                let sec_x = colon_x + colon_width + spacing;
                let flip_scale = 1.0 - flip_progress * 0.2;
                self.render_digit(draw, self.second_digits[0], sec_x, start_y, digit_width, digit_height * flip_scale, seg_color, 1.0);
                self.render_digit(draw, self.second_digits[1], sec_x + digit_width + spacing, start_y, digit_width, digit_height * flip_scale, seg_color, 1.0);
            }
        }
    }

    fn render_timer_display(&self, draw: &mut DrawContext, viewport: Rect) {
        // Blue LCD timer display in separate window
        // Viewport is 80x30 for the timer window
        let outer_padding = 3.0;

        // Size to fit the small window
        let digit_height = viewport.height - outer_padding * 2.0;
        let digit_width = digit_height * 0.62;
        let spacing = 2.0;
        let colon_width = digit_width * 0.28;
        let margin = 2.0;

        // Blue color for timer
        let seg_color = Color::rgba(64, 128, 255, 255);

        let total_width = digit_width * 4.0 + spacing * 3.0 + colon_width;

        // Center in the small viewport
        let face_w = viewport.width - outer_padding * 2.0;
        let face_h = viewport.height - outer_padding * 2.0;
        let face_x = outer_padding;
        let face_y = outer_padding;

        // Background face (black)
        draw.rect(face_x, face_y, face_w, face_h, Color::rgba(0, 0, 0, 255));

        let start_x = face_x + margin;
        let start_y = face_y + margin;

        // Render MM:SS
        self.render_digit(draw, self.minute_digits[0], start_x, start_y, digit_width, digit_height, seg_color, 1.0);
        self.render_digit(draw, self.minute_digits[1], start_x + digit_width + spacing, start_y, digit_width, digit_height, seg_color, 1.0);

        // Colon
        let colon_x = start_x + digit_width * 2.0 + spacing * 2.0;
        let dot = digit_width * 0.11;
        draw.rect(colon_x, start_y + digit_height * 0.3, dot, dot, seg_color);
        draw.rect(colon_x, start_y + digit_height * 0.62, dot, dot, seg_color);

        // Seconds
        let second_x = colon_x + colon_width + spacing;
        self.render_digit(draw, self.second_digits[0], second_x, start_y, digit_width, digit_height, seg_color, 1.0);
        self.render_digit(draw, self.second_digits[1], second_x + digit_width + spacing, start_y, digit_width, digit_height, seg_color, 1.0);
    }

    fn render_digit(&self, draw: &mut DrawContext, digit: u8, x: f32, y: f32, width: f32, height: f32, color: Color, alpha: f32) {
        if digit > 9 { return; }
        let segments = SEGMENT_MAP[digit as usize];
        let segment_width = width * 0.8;
        let segment_thickness = width * 0.15;
        let h_offset = width * 0.1;
        let v_segment_height = height * 0.4;
        let bevel = segment_thickness * 0.5;
        let color = Color::new(color.r, color.g, color.b, color.a * alpha);

        if segments[0] { self.render_horizontal_segment(draw, x + h_offset, y, segment_width, segment_thickness, bevel, color); }
        if segments[1] { self.render_vertical_segment(draw, x + width - segment_thickness, y + segment_thickness, v_segment_height, segment_thickness, bevel, color, false); }
        if segments[2] { self.render_vertical_segment(draw, x + width - segment_thickness, y + height * 0.5 + segment_thickness * 0.5, v_segment_height, segment_thickness, bevel, color, true); }
        if segments[3] { self.render_horizontal_segment(draw, x + h_offset, y + height - segment_thickness, segment_width, segment_thickness, bevel, color); }
        if segments[4] { self.render_vertical_segment(draw, x, y + height * 0.5 + segment_thickness * 0.5, v_segment_height, segment_thickness, bevel, color, true); }
        if segments[5] { self.render_vertical_segment(draw, x, y + segment_thickness, v_segment_height, segment_thickness, bevel, color, false); }
        if segments[6] { self.render_middle_segment(draw, x + h_offset, y + height * 0.5 - segment_thickness * 0.5, segment_width, segment_thickness, bevel, color); }
    }

    fn render_horizontal_segment(&self, draw: &mut DrawContext, x: f32, y: f32, width: f32, thickness: f32, bevel: f32, color: Color) {
        let steps = 20;
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let y_pos = y + (t * thickness);
            let distance_from_center = (t - 0.5).abs() * 2.0;
            let x_inset = distance_from_center * bevel;
            let slice_x = x + x_inset;
            let slice_width = width - (2.0 * x_inset);
            let slice_height = thickness / steps as f32 + 0.5;
            if slice_width > 0.0 {
                draw.rect(slice_x, y_pos, slice_width, slice_height, color);
            }
        }
    }

    fn render_vertical_segment(&self, draw: &mut DrawContext, x: f32, y: f32, height: f32, thickness: f32, bevel: f32, color: Color, is_bottom: bool) {
        let steps = 20;
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let x_pos = x + (t * thickness);
            let distance_from_center = (t - 0.5).abs() * 2.0;
            let y_inset_top = if !is_bottom { distance_from_center * bevel } else { 0.0 };
            let y_inset_bottom = if is_bottom { distance_from_center * bevel } else { 0.0 };
            let slice_y = y + y_inset_top;
            let slice_height = height - y_inset_top - y_inset_bottom;
            let slice_width = thickness / steps as f32 + 0.5;
            if slice_height > 0.0 {
                draw.rect(x_pos, slice_y, slice_width, slice_height, color);
            }
        }
    }

    fn render_middle_segment(&self, draw: &mut DrawContext, x: f32, y: f32, width: f32, thickness: f32, bevel: f32, color: Color) {
        let steps = 20;
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let y_pos = y + (t * thickness);
            let distance_from_center = (t - 0.5).abs() * 2.0;
            let x_inset = distance_from_center * bevel * 1.2;
            let slice_x = x + x_inset;
            let slice_width = width - (2.0 * x_inset);
            let slice_height = thickness / steps as f32 + 0.5;
            if slice_width > 0.0 {
                draw.rect(slice_x, y_pos, slice_width, slice_height, color);
            }
        }
    }
}