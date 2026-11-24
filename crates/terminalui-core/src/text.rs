//! Plain rich text as a view leaf.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;

use crate::context::Context;
use crate::input::InputEvent;
use crate::outcome::Outcome;
use crate::outline::quoted;
use crate::theme::Theme;
use crate::view::View;

/// Styling for a [`Text`] leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct TextStyle {
    /// Text color.
    pub foreground: Option<Color>,
    /// Background color.
    pub background: Option<Color>,
    /// Render bold.
    pub bold: bool,
    /// Render italic.
    pub italic: bool,
    /// Render with a solid underline.
    pub underline: bool,
}

impl TextStyle {
    /// Builder shorthand: `true` for [`TextStyle::bold`].
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    /// Builder shorthand: set the foreground color.
    pub fn fg(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }
}

/// Plain text as a view leaf.
#[derive(Serialize)]
pub struct Text {
    content: String,
    style: TextStyle,
}

impl From<&str> for Text {
    fn from(content: &str) -> Self {
        Self::new(content)
    }
}

impl From<String> for Text {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl Text {
    /// Creates a text leaf from any `Into<String>`.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::default(),
        }
    }

    /// Replaces the style.
    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }

    /// Replaces the style in place.
    ///
    /// Use this when a theme change must keep the same view instance, so
    /// focus, identity and hit-test state attached to the node survive.
    pub fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }

    /// Replaces the content; use this rather than reaching into the leaf.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.content = text.into();
    }

    /// The current content.
    pub fn text(&self) -> &str {
        &self.content
    }

    fn style(&self, theme: &Theme) -> Style {
        let mut style = Style::default().fg(self.style.foreground.unwrap_or(theme.foreground));
        if let Some(background) = self.style.background {
            style = style.bg(background);
        }
        if self.style.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.style.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.style.underline {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        style
    }
}

impl View for Text {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn kind(&self) -> &'static str {
        "text"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("text={}", quoted(&self.content)))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        let style = self.style(theme);
        for (row, line) in self.content.lines().enumerate() {
            if row >= area.height as usize {
                break;
            }
            let y = area.y + row as u16;
            buf.set_stringn(area.x, y, line, area.width as usize, style);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let mut width = 0u16;
        let mut lines = 0u16;
        for line in self.content.lines() {
            width = width.max(line.chars().count() as u16);
            lines += 1;
        }
        Some((width, lines.max(1)))
    }
}

/// Trim `text` to `limit` characters, marking a truncated value with an
/// ellipsis. The ellipsis is included in `limit`.
pub fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
