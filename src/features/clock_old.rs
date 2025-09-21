use crate::app::UiEvent;
use crate::gfx::{anim::Timeline, draw::DrawContext, math::{Color, Rect}};
use time::OffsetDateTime;
use log::info;

// Seven-segment display mapping
// Each digit has 7 segments: A(top), B(top-right), C(bottom-right), D(bottom),
// E(bottom-left), F(top-left), G(middle)
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

    pub fn render(&self, draw: &mut DrawContext, viewport: Rect, show_seconds: bool) {
        self.render_collapsed(draw, viewport, show_seconds);
    }

    fn render_collapsed(&self, draw: &mut DrawContext, viewport: Rect, show_seconds: bool) {
        info!("===== CLOCK RENDER START (collapsed) =====");
        info!("Viewport: x={}, y={}, width={}, height={}",
              viewport.x, viewport.y, viewport.width, viewport.height);

        // Compact 12h time HH:MM with blinking colon, pinned to top-right
        let outer_padding = 6.0;

        // Ratios
        let r_w = 0.62;   // digit_width = r_w * dh
        let r_c = 0.28;   // colon_width = r_c * digit_width
        let r_m = 1.5;    // margin = r_m * spacing

        // Fixed inter-glyph spacing
        let spacing = 6.0f32;

        // Compute max digit height by width/height constraints so face fits viewport
        // Height constraint: dh <= H - 2*outer_padding - 2*margin
        let margin_h = r_m * spacing;
        let mut dh_by_h = viewport.height - outer_padding * 2.0 - margin_h * 2.0;
        if dh_by_h < 0.0 { dh_by_h = 0.0; }

        // Width constraint: r_w*(4+ r_c)*dh + 3*spacing + 2*margin <= W - outer_padding
        let margin_w = r_m * spacing;
        let avail_w = viewport.width - outer_padding;
        let denom = r_w * (4.0 + r_c);
        let mut dh_by_w = (avail_w - 3.0 * spacing - 2.0 * margin_w) / denom;
        if dh_by_w < 0.0 { dh_by_w = 0.0; }

        let digit_height = dh_by_h.min(dh_by_w);

        let digit_width = digit_height * r_w;
        let colon_width = digit_width * r_c;

        // Classic seven-seg red
        let seg_color = Color::rgba(255, 64, 64, 255);

        let total_width = digit_width * 4.0 + spacing * 3.0 + colon_width;

        // Larger bezel margin around readout
        let mut margin = spacing * r_m;
        if margin < 4.0 { margin = 4.0; }

        // Compute face rect anchored to top-right inside viewport with outer padding
        let face_w = total_width + margin * 2.0;
        let face_h = digit_height + margin * 2.0;
        // Fix: viewport.x is 0, need to position relative to viewport width
        let face_x = viewport.width - face_w - outer_padding;
        let face_y = outer_padding;

        info!("Clock sizing: digit_height={}, digit_width={}, total_width={}",
              digit_height, digit_width, total_width);
        info!("Clock face dimensions: width={}, height={}", face_w, face_h);
        info!("FINAL CLOCK POSITION: x={}, y={}", face_x, face_y);
        info!("Clock rect: ({}, {}) to ({}, {})",
              face_x, face_y, face_x + face_w, face_y + face_h);

        // Background face (black)
        draw.rect(face_x, face_y, face_w, face_h, Color::rgba(0, 0, 0, 255));

        // Digits start inside bezel
        let start_x = face_x + margin;
        let start_y = face_y + margin;

        // Render HH
        self.render_digit(draw, self.hour_digits[0], start_x, start_y, digit_width, digit_height, seg_color, 1.0);
        self.render_digit(draw, self.hour_digits[1], start_x + digit_width + spacing, start_y, digit_width, digit_height, seg_color, 1.0);

        // Colon (always visible)
        let colon_x = start_x + digit_width * 2.0 + spacing * 2.0;
        let dot = digit_width * 0.11;
        draw.rect(colon_x, start_y + digit_height * 0.3, dot, dot, seg_color);
        draw.rect(colon_x, start_y + digit_height * 0.62, dot, dot, seg_color);

        // Minutes
        let minute_x = colon_x + colon_width + spacing;
        self.render_digit(draw, self.minute_digits[0], minute_x, start_y, digit_width, digit_height, seg_color, 1.0);
        self.render_digit(draw, self.minute_digits[1], minute_x + digit_width + spacing, start_y, digit_width, digit_height, seg_color, 1.0);
    }

    fn render_expanded(&self, draw: &mut DrawContext, viewport: Rect) {
        // Larger HH:MM:SS with tight black bezel, pinned to top-right
        let outer_padding = 8.0;

        // Ratios
        let r_w = 0.64;  // digit_width = r_w * dh
        let r_c = 0.30;  // colon_width = r_c * digit_width
        let r_m = 1.8;   // margin = r_m * spacing

        // Fixed inter-glyph spacing
        let spacing = 6.0f32;

        // Compute max digit height by width/height so face fits viewport
        // Height: dh <= H - 2*outer_padding - 2*margin
        let margin_h = r_m * spacing;
        let mut dh_by_h = viewport.height - outer_padding * 2.0 - margin_h * 2.0;
        if dh_by_h < 0.0 { dh_by_h = 0.0; }

        // Width: r_w*(6 + 2*r_c)*dh + 7*spacing + 2*margin <= W - outer_padding
        let margin_w = r_m * spacing;
        let avail_w = viewport.width - outer_padding;
        let denom = r_w * (6.0 + 2.0 * r_c);
        let mut dh_by_w = (avail_w - 7.0 * spacing - 2.0 * margin_w) / denom;
        if dh_by_w < 0.0 { dh_by_w = 0.0; }

        let digit_height = dh_by_h.min(dh_by_w);

        let digit_width = digit_height * r_w;
        let colon_width = digit_width * r_c;

        let total_width = digit_width * 6.0 + spacing * 7.0 + colon_width * 2.0;

        let base_color = Color::rgba(255, 64, 64, 255);
        let pulse_alpha = if self.pulse_timeline.is_complete() {
            1.0
        } else {
            1.0 + self.pulse_timeline.eased_progress() * 0.3
        };

        let flip_progress = self.flip_timeline.eased_progress();

        // Background face (black) with larger margin/bezel, anchored top-right
        let mut margin = spacing * r_m;
        if margin < 5.0 { margin = 5.0; }
        let face_w = total_width + margin * 2.0;
        let face_h = digit_height + margin * 2.0;
        // Fix: position relative to viewport width, not viewport.x
        let face_x = viewport.width - face_w - outer_padding;
        let face_y = outer_padding;
        draw.rect(face_x, face_y, face_w, face_h, Color::rgba(0, 0, 0, 255));

        let start_x = face_x + margin;
        let start_y = face_y + margin;

        // Render HH:MM:SS in 12h time
        self.render_digit(draw, self.hour_digits[0], start_x, start_y, digit_width, digit_height, base_color, pulse_alpha);
        self.render_digit(draw, self.hour_digits[1], start_x + digit_width + spacing, start_y, digit_width, digit_height, base_color, pulse_alpha);

        // First colon (always visible)
        let colon1_x = start_x + digit_width * 2.0 + spacing * 2.0;
        let dot = digit_width * 0.12;
        draw.rect(colon1_x, start_y + digit_height * 0.3, dot, dot, base_color);
        draw.rect(colon1_x, start_y + digit_height * 0.6, dot, dot, base_color);

        // Minutes
        let minute_x = colon1_x + colon_width + spacing;
        self.render_digit(draw, self.minute_digits[0], minute_x, start_y, digit_width, digit_height, base_color, pulse_alpha);
        self.render_digit(draw, self.minute_digits[1], minute_x + digit_width + spacing, start_y, digit_width, digit_height, base_color, pulse_alpha);

        // Second colon (always visible)
        let colon2_x = minute_x + digit_width * 2.0 + spacing * 2.0;
        let dot = digit_width * 0.12;
        draw.rect(colon2_x, start_y + digit_height * 0.3, dot, dot, base_color);
        draw.rect(colon2_x, start_y + digit_height * 0.6, dot, dot, base_color);

        // Seconds (same size as other digits, no animation)
        let second_x = colon2_x + colon_width + spacing;
        self.render_digit(draw, self.second_digits[0], second_x, start_y, digit_width, digit_height, base_color, pulse_alpha);
        self.render_digit(draw, self.second_digits[1], second_x + digit_width + spacing, start_y, digit_width, digit_height, base_color, pulse_alpha);
    }

    fn render_digit(&self, draw: &mut DrawContext, digit: u8, x: f32, y: f32, width: f32, height: f32, color: Color, alpha: f32) {
        if digit > 9 {
            return;
        }

        let segments = SEGMENT_MAP[digit as usize];
        let segment_width = width * 0.8;
        let segment_thickness = width * 0.15;
        let h_offset = width * 0.1;
        let v_segment_height = height * 0.4;

        // Bevel size for angled ends
        let bevel = segment_thickness * 0.5;

        let color = Color::new(color.r, color.g, color.b, color.a * alpha);

        // A - top horizontal (trapezoid)
        if segments[0] {
            self.render_horizontal_segment(draw, x + h_offset, y, segment_width, segment_thickness, bevel, color);
        }

        // B - top right vertical (trapezoid)
        if segments[1] {
            self.render_vertical_segment(draw, x + width - segment_thickness, y + segment_thickness, v_segment_height, segment_thickness, bevel, color, false);
        }

        // C - bottom right vertical (trapezoid)
        if segments[2] {
            self.render_vertical_segment(draw, x + width - segment_thickness, y + height * 0.5 + segment_thickness * 0.5, v_segment_height, segment_thickness, bevel, color, true);
        }

        // D - bottom horizontal (trapezoid)
        if segments[3] {
            self.render_horizontal_segment(draw, x + h_offset, y + height - segment_thickness, segment_width, segment_thickness, bevel, color);
        }

        // E - bottom left vertical (trapezoid)
        if segments[4] {
            self.render_vertical_segment(draw, x, y + height * 0.5 + segment_thickness * 0.5, v_segment_height, segment_thickness, bevel, color, true);
        }

        // F - top left vertical (trapezoid)
        if segments[5] {
            self.render_vertical_segment(draw, x, y + segment_thickness, v_segment_height, segment_thickness, bevel, color, false);
        }

        // G - middle horizontal (double trapezoid for middle segment)
        if segments[6] {
            self.render_middle_segment(draw, x + h_offset, y + height * 0.5 - segment_thickness * 0.5, segment_width, segment_thickness, bevel, color);
        }
    }

    fn render_horizontal_segment(&self, draw: &mut DrawContext, x: f32, y: f32, width: f32, thickness: f32, bevel: f32, color: Color) {
        // Draw horizontal segment as a hexagon shape (trapezoid with angled ends)
        // Split into many thin horizontal slices for smooth edges
        let steps = 20; // More steps for smoother appearance

        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let y_pos = y + (t * thickness);

            // Calculate inset for each horizontal slice
            // Create a hexagonal profile: angled at ends
            let distance_from_center = (t - 0.5).abs() * 2.0; // 0 at center, 1 at edges
            let x_inset = distance_from_center * bevel;

            let slice_x = x + x_inset;
            let slice_width = width - (2.0 * x_inset);
            let slice_height = thickness / steps as f32 + 0.5; // Slight overlap to avoid gaps

            if slice_width > 0.0 {
                draw.rect(slice_x, y_pos, slice_width, slice_height, color);
            }
        }
    }

    fn render_vertical_segment(&self, draw: &mut DrawContext, x: f32, y: f32, height: f32, thickness: f32, bevel: f32, color: Color, is_bottom: bool) {
        // Draw vertical segment with proper tapering
        let steps = 20; // More steps for smoother appearance

        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let x_pos = x + (t * thickness);

            // Calculate vertical inset based on position
            let distance_from_center = (t - 0.5).abs() * 2.0;

            // Taper differently for top vs bottom segments
            let y_inset_top = if !is_bottom { distance_from_center * bevel } else { 0.0 };
            let y_inset_bottom = if is_bottom { distance_from_center * bevel } else { 0.0 };

            let slice_y = y + y_inset_top;
            let slice_height = height - y_inset_top - y_inset_bottom;
            let slice_width = thickness / steps as f32 + 0.5; // Slight overlap

            if slice_height > 0.0 {
                draw.rect(x_pos, slice_y, slice_width, slice_height, color);
            }
        }
    }

    fn render_middle_segment(&self, draw: &mut DrawContext, x: f32, y: f32, width: f32, thickness: f32, bevel: f32, color: Color) {
        // Middle segment has a hexagonal/diamond shape
        let steps = 20;

        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let y_pos = y + (t * thickness);

            // Create diamond shape: narrow at top and bottom, wide in middle
            let distance_from_center = (t - 0.5).abs() * 2.0;
            let x_inset = distance_from_center * bevel * 1.2; // Slightly more pronounced for middle segment

            let slice_x = x + x_inset;
            let slice_width = width - (2.0 * x_inset);
            let slice_height = thickness / steps as f32 + 0.5;

            if slice_width > 0.0 {
                draw.rect(slice_x, y_pos, slice_width, slice_height, color);
            }
        }
    }
}