pub mod clock;
pub mod pomodoro;

use crate::app::UiEvent;
use crate::gfx::{draw::DrawContext, math::Rect};

pub trait Feature {
    fn name(&self) -> &'static str;
    fn desired_expanded_size(&self) -> (u32, u32);
    fn update(&mut self, dt: f32, now: f32);
    fn handle_event(&mut self, event: UiEvent) -> bool;
    fn render(&self, draw: &mut DrawContext, viewport: Rect);
}