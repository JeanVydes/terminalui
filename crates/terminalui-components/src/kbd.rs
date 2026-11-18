//! A keyboard key cap. Rendered as a single-row pill: background, bold
//! foreground, bracketed label. Compose several into hint rows with a
//! horizontal stack plus [`terminalui_core::Text`] views.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

/// Public state of the key cap: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KbdState {
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// Per-instance overrides for a key cap. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct KbdStyle {
    /// Pill background. Defaults to `theme.secondary`.
    pub background: Option<Color>,
    /// Key color. Defaults to `theme.foreground`.
    pub foreground: Option<Color>,
}

/// A single key cap. Pass the key text (e.g. `"Ctrl"`, `"Enter"`, `"F1"`);
/// for combined modifiers compose the pill yourself with separate [`Kbd`]s
/// plus [`terminalui_core::Text`].
#[derive(Serialize)]
pub struct Kbd {
    /// The key text, e.g. `Ctrl`, `Enter`, `F1`.
    pub key: String,
    /// Where the cap rendered in the last pass.
    pub state: KbdState,
    /// Per-instance overrides; defaults match the theme.
    pub style: KbdStyle,
}

impl Kbd {
    /// Creates a key cap with the given key text.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            state: KbdState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: KbdStyle::default(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: KbdStyle) -> Self {
        self.style = style;
        self
    }
}

impl View for Kbd {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "kbd"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("key={}", quoted(&self.key)))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let width = self.key.chars().count() as u16 + 4;
        Some((width, 1))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let background = self.style.background.unwrap_or(theme.secondary);
        let foreground = self.style.foreground.unwrap_or(theme.foreground);
        let style = Style::default()
            .bg(background)
            .fg(foreground)
            .add_modifier(Modifier::BOLD);
        buf.set_stringn(
            area.x,
            area.y,
            format!("[ {} ]", self.key),
            area.width as usize,
            style,
        );
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
