//! A filler view consuming the remaining space along an axis.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;

use crate::context::Context;
use crate::input::InputEvent;
use crate::outcome::Outcome;
use crate::theme::Theme;
use crate::view::View;

#[derive(Serialize)]
pub struct Spacer;

impl Spacer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Spacer {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }
    fn kind(&self) -> &'static str {
        "spacer"
    }
    fn render(&mut self, _area: Rect, _buf: &mut Buffer, _cx: &mut Context, _theme: &Theme) {}
    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
