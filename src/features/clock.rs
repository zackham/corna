use crate::app::UiEvent;
use crate::gfx::{anim::Timeline, draw::DrawContext, math::{Color, Rect}};
use time::OffsetDateTime;
use log::info;

// Seven-segment display mapping
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

pub struct Clock {
    last_sec: i32,
    flip_timeline: Timeline,
    pulse_timeline: Timeline,
    hour_digits: [u8; 2],
    minute_digits: [u8; 2],
    second_digits: [u8; 2],
    is_pm: bool,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            last_sec: -1,
            flip_timeline: Timeline::new(0.12),
            pulse_timeline: Timeline::new(0.2),
            hour_digits: [0, 0],
            minute_digits: [0, 0],
            second_digits: [0, 0],
            is_pm: false,
        }
    }

    pub fn update(&mut self, _dt: f32, now: f32) {
        if let Ok(time) = OffsetDateTime::now_local() {
            let sec = time.second() as i32;

            if sec != self.last_sec {
                self.last_sec = sec;
                self.flip_timeline.start(now);
                self.pulse_timeline.start(now);

                // Update digits - convert to 12h time
                let mut hour_24 = time.hour() as u8;
                self.is_pm = hour_24 >= 12;
                let hour_12 = {
                    let mut h = hour_24 % 12;
                    if h == 0 { h = 12; }
                    h
                };
                let minute = time.minute() as u8;
                let second = time.second() as u8;

                self.hour_digits = [hour_12 / 10, hour_12 % 10];
                self.minute_digits = [minute / 10, minute % 10];
                self.second_digits = [second / 10, second % 10];
            }
        }

        self.flip_timeline.update(now);
        self.pulse_timeline.update(now);
    }

    pub fn render(&self, draw: &mut DrawContext, viewport: Rect, show_seconds: bool, color_mode: u8, time: f32) {
        self.render_clock(draw, viewport, show_seconds, color_mode, time);
    }

    fn render_clock(&self, draw: &mut DrawContext, viewport: Rect, show_seconds: bool, color_mode: u8, time: f32) {
        // Compact 12h time HH:MM or HH:MM:SS
        let outer_padding = 4.0;

        // Ratios
        let r_w = 0.62;   // digit_width = r_w * dh
        let r_c = 0.28;   // colon_width = r_c * digit_width
        let r_m = 1.5;    // margin = r_m * spacing

        // Fixed inter-glyph spacing
        let spacing = 6.0f32;

        // Compute max digit height by height constraint only (keep height consistent)
        let margin_h = r_m * spacing;
        let mut dh_by_h = viewport.height - outer_padding * 2.0 - margin_h * 2.0;
        if dh_by_h < 0.0 { dh_by_h = 0.0; }

        // Use height constraint for digit size (don't change based on seconds display)
        let digit_height = dh_by_h;

        let digit_width = digit_height * r_w;
        let colon_width = digit_width * r_c;

        let total_width = if show_seconds {
            digit_width * 6.0 + spacing * 7.0 + colon_width * 2.0
        } else {
            digit_width * 4.0 + spacing * 3.0 + colon_width
        };

        // Larger bezel margin around readout
        let mut margin = spacing * r_m;
        if margin < 4.0 { margin = 4.0; }

        // Compute face rect anchored to top-right inside viewport with outer padding
        let face_w = total_width + margin * 2.0;
        let face_h = digit_height + margin * 2.0;
        let face_x = viewport.width - face_w - outer_padding;
        let face_y = outer_padding;

        // Background face (black)
        draw.rect(face_x, face_y, face_w, face_h, Color::rgba(0, 0, 0, 255));

        // Digits start inside bezel
        let start_x = face_x + margin;
        let start_y = face_y + margin;

        let num_digits = if show_seconds { 6 } else { 4 };

        // Render HH with position info
        self.render_digit_with_pos(draw, self.hour_digits[0], start_x, start_y,
            digit_width, digit_height, color_mode, time, 0, num_digits);
        self.render_digit_with_pos(draw, self.hour_digits[1], start_x + digit_width + spacing, start_y,
            digit_width, digit_height, color_mode, time, 1, num_digits);

        // Colon with position color
        let colon_x = start_x + digit_width * 2.0 + spacing * 2.0;
        let dot = digit_width * 0.11;
        let colon_color = self.get_color_for_position(color_mode, time, 2, num_digits, 0);
        draw.rect(colon_x, start_y + digit_height * 0.3, dot, dot, colon_color);
        draw.rect(colon_x, start_y + digit_height * 0.62, dot, dot, colon_color);

        // Minutes with position info
        let minute_x = colon_x + colon_width + spacing;
        self.render_digit_with_pos(draw, self.minute_digits[0], minute_x, start_y,
            digit_width, digit_height, color_mode, time, 2, num_digits);
        self.render_digit_with_pos(draw, self.minute_digits[1], minute_x + digit_width + spacing, start_y,
            digit_width, digit_height, color_mode, time, 3, num_digits);

        // Seconds (if enabled)
        if show_seconds {
            // Second colon with position color
            let colon2_x = minute_x + digit_width * 2.0 + spacing * 2.0;
            let colon2_color = self.get_color_for_position(color_mode, time, 4, num_digits, 0);
            draw.rect(colon2_x, start_y + digit_height * 0.3, dot, dot, colon2_color);
            draw.rect(colon2_x, start_y + digit_height * 0.62, dot, dot, colon2_color);

            // Second digits with position info
            let second_x = colon2_x + colon_width + spacing;
            self.render_digit_with_pos(draw, self.second_digits[0], second_x, start_y,
                digit_width, digit_height, color_mode, time, 4, num_digits);
            self.render_digit_with_pos(draw, self.second_digits[1], second_x + digit_width + spacing, start_y,
                digit_width, digit_height, color_mode, time, 5, num_digits);
        }
    }

    fn render_digit_with_pos(&self, draw: &mut DrawContext, digit: u8, x: f32, y: f32,
                             width: f32, height: f32, color_mode: u8, time: f32,
                             digit_pos: u8, total_digits: u8) {
        if digit > 9 { return; }
        let segments = SEGMENT_MAP[digit as usize];
        let segment_width = width * 0.8;
        let segment_thickness = width * 0.15;
        let h_offset = width * 0.1;
        let v_segment_height = height * 0.4;
        let bevel = segment_thickness * 0.5;

        // Render each segment with its own color based on position
        for (seg_idx, &is_on) in segments.iter().enumerate() {
            if is_on {
                let color = self.get_color_for_position(color_mode, time, digit_pos, total_digits, seg_idx as u8);

                match seg_idx {
                    0 => self.render_horizontal_segment(draw, x + h_offset, y, segment_width, segment_thickness, bevel, color),
                    1 => self.render_vertical_segment(draw, x + width - segment_thickness, y + segment_thickness, v_segment_height, segment_thickness, bevel, color, false),
                    2 => self.render_vertical_segment(draw, x + width - segment_thickness, y + height * 0.5 + segment_thickness * 0.5, v_segment_height, segment_thickness, bevel, color, true),
                    3 => self.render_horizontal_segment(draw, x + h_offset, y + height - segment_thickness, segment_width, segment_thickness, bevel, color),
                    4 => self.render_vertical_segment(draw, x, y + height * 0.5 + segment_thickness * 0.5, v_segment_height, segment_thickness, bevel, color, true),
                    5 => self.render_vertical_segment(draw, x, y + segment_thickness, v_segment_height, segment_thickness, bevel, color, false),
                    6 => self.render_middle_segment(draw, x + h_offset, y + height * 0.5 - segment_thickness * 0.5, segment_width, segment_thickness, bevel, color),
                    _ => {}
                }
            }
        }
    }

    fn get_color_for_position(&self, mode: u8, time: f32, digit_pos: u8, total_digits: u8, segment: u8) -> Color {
        // Calculate position-based phase offset for waves and animations
        let pos_offset = digit_pos as f32 / total_digits as f32;
        let seg_offset = segment as f32 / 7.0;

        match mode {
            0 => Color::rgba(255, 64, 64, 255),      // Classic Red
            1 => Color::rgba(0, 255, 255, 255),      // Cyan
            2 => Color::rgba(64, 255, 64, 255),      // Green
            3 => Color::rgba(255, 191, 0, 255),      // Amber
            4 => Color::rgba(191, 64, 255, 255),     // Purple
            5 => Color::rgba(255, 255, 255, 255),    // White

            6 => {
                // Rainbow Wave - flows across digits
                let hue = (time * 0.2 + pos_offset * 0.5 + seg_offset * 0.05) % 1.0;
                self.hsv_to_rgb(hue, 1.0, 1.0)
            }

            7 => {
                // Cascade Breathing - pulses from left to right
                let phase = time + pos_offset * 0.5;
                let brightness = (phase.sin() * 0.3 + 0.7).max(0.4).min(1.0);
                let val = (255.0 * brightness) as u8;
                Color::rgba(val, val / 4, val / 4, 255)
            }

            8 => {
                // Matrix Rain Effect - segments cascade downward
                let cascade_time = time * 2.0 + digit_pos as f32 * 0.3 + segment as f32 * 0.1;
                let intensity = ((cascade_time % 3.0) - 1.5).abs() / 1.5;
                let green = (64.0 + 191.0 * intensity) as u8;
                let blue = (255.0 * (1.0 - intensity * 0.7)) as u8;
                Color::rgba(0, green, blue, 255)
            }

            9 => {
                // Fire Effect - flickering per segment
                let flicker = (time * 10.0 + digit_pos as f32 * 3.7 + segment as f32 * 5.3).sin();
                let random = ((digit_pos as f32 * 7.3 + segment as f32 * 13.7).sin() * 43758.5453).fract();
                let intensity = (0.7 + flicker * 0.2 + random * 0.1).max(0.5).min(1.0);

                let r = (255.0 * intensity) as u8;
                let g = (191.0 * intensity * 0.7) as u8;
                let b = (64.0 * intensity * 0.2) as u8;
                Color::rgba(r, g, b, 255)
            }

            10 => {
                // Electric Storm - random segment flashes with propagation
                let storm_phase = time * 3.0 + pos_offset * 2.0;
                let flash = ((storm_phase * 7.3 + segment as f32 * 11.1).sin() * 137.5).fract();
                let flash_intensity = if flash > 0.8 { 1.0 } else { 0.6 };

                let base_color = if flash > 0.8 {
                    Color::rgba(255, 255, 255, 255) // White flash
                } else {
                    // Electric blue base with variation
                    let variation = (time * 0.5 + pos_offset).sin() * 0.2 + 0.8;
                    Color::rgba(
                        (100.0 * variation) as u8,
                        (150.0 * variation) as u8,
                        (255.0 * flash_intensity) as u8,
                        255
                    )
                };
                base_color
            }

            _ => Color::rgba(255, 64, 64, 255), // Default to red
        }
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

    fn hsv_to_rgb(&self, h: f32, s: f32, v: f32) -> Color {
        let h = h * 360.0;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color::rgba(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
            255,
        )
    }
}