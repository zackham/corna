pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[derive(Debug, Clone)]
pub struct Timeline {
    pub start_time: f32,
    pub duration: f32,
    pub current_time: f32,
}

impl Timeline {
    pub fn new(duration: f32) -> Self {
        Self {
            start_time: 0.0,
            duration,
            current_time: 0.0,
        }
    }

    pub fn start(&mut self, now: f32) {
        self.start_time = now;
        self.current_time = now;
    }

    pub fn update(&mut self, now: f32) {
        self.current_time = now;
    }

    pub fn progress(&self) -> f32 {
        let elapsed = self.current_time - self.start_time;
        (elapsed / self.duration).min(1.0).max(0.0)
    }

    pub fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }

    pub fn eased_progress(&self) -> f32 {
        ease_in_out(self.progress())
    }
}