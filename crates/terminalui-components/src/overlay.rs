//! Overlay: a modal scrim that consumes pointer events underneath it and dims content.
use std::any::Any;

use crossterm::event::MouseEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::Context;
use terminalui_core::{Outcome, View};

/// Public state of the overlay: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OverlayState {
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// Per-instance overrides for an overlay. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OverlayStyle {
    /// Dim-layer background. Defaults to `theme.muted`.
    pub background: Option<Color>,
}

fn dim(theme: &Theme) -> Color {
    match theme.background {
        Color::Rgb(red, green, blue) => Color::Rgb(
            (red as u32 * 40 / 100) as u8,
            (green as u32 * 40 / 100) as u8,
            (blue as u32 * 40 / 100) as u8,
        ),
        _ => Color::Black,
    }
}

/// A modal scrim: blocks and dims everything underneath it.
#[derive(Serialize)]
pub struct Overlay {
    /// Public state: the last rendered area.
    pub state: OverlayState,
    /// Per-instance overrides; defaults match the theme.
    pub style: OverlayStyle,
}

impl Overlay {
    /// Creates an empty overlay.
    pub fn new() -> Self {
        Self {
            state: OverlayState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: OverlayStyle::default(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: OverlayStyle) -> Self {
        self.style = style;
        self
    }

    /// `true` when the cell at `(column, row)` is inside the overlay.
    pub fn is_hit(&self, column: u16, row: u16) -> bool {
        let area = self.state.area;
        column >= area.x
            && column < area.x.saturating_add(area.width)
            && row >= area.y
            && row < area.y.saturating_add(area.height)
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Overlay {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "overlay"
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let background = self.style.background.unwrap_or_else(|| dim(theme));
        // A real modal scrim: blank every cell under the overlay rather than
        // tint the background. The dimmed app must not bleed through.
        for y in area.y..area.y + area.height {
            buf.set_string(
                area.x,
                y,
                " ".repeat(area.width as usize),
                Style::default().bg(background),
            );
        }
    }

    fn handle_event(&mut self, event: &InputEvent, _cx: &mut Context) -> Outcome {
        let InputEvent::Mouse(mouse) = event else {
            return Outcome::Ignored;
        };
        let is_press = matches!(mouse.kind, MouseEventKind::Down(_));
        if is_press && self.is_hit(mouse.column, mouse.row) {
            Outcome::Consumed
        } else {
            Outcome::Ignored
        }
    }
}
