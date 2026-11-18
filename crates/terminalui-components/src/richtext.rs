//! RichText: a view that paints composed ratatui [`Line`]s.
//!
//! Terminal apps migrating hand-drawn ratatui code onto a view tree still
//! hold their rendering as [`Line`]s and spans; this view lets them paint
//! those lines directly as one terminalui node instead of translating them
//! back into strings and styles by hand.
//!
//! The view renders each line top to bottom, left to right, truncating at
//! the width it is given. It owns no input and never holds focus.

use std::any::Any;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use serde::Serialize;
use unicode_width::UnicodeWidthStr;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

/// A block of ratatui lines, painted as one terminalui view.
#[derive(Serialize)]
pub struct RichText {
    /// The lines, in display order.
    #[serde(skip)]
    pub lines: Vec<Line<'static>>,
    /// Indent applied to every line, in cells.
    pub indent: u16,
}

impl RichText {
    /// Creates a view painting the given lines.
    pub fn new(lines: Vec<Line<'static>>) -> Self {
        Self { lines, indent: 0 }
    }

    /// Indent every line by this many cells.
    pub fn with_indent(mut self, indent: u16) -> Self {
        self.indent = indent;
        self
    }

    /// The displayed width of one line, accounting for wide glyphs.
    fn line_width(line: &Line<'_>) -> usize {
        line.spans
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum()
    }
}

impl View for RichText {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "rich_text"
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let width = self.lines.iter().map(Self::line_width).max().unwrap_or(0) as u16;
        Some((width + self.indent * 2, self.lines.len().max(1) as u16))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let max_width = area.width.saturating_sub(self.indent) as usize;
        for (row, line) in self.lines.iter().enumerate() {
            let y = area.y + row as u16;
            if y >= area.y + area.height {
                break;
            }
            let mut x = area.x + self.indent;
            let mut remaining = max_width;
            for span in &line.spans {
                if remaining == 0 {
                    break;
                }
                let text = span.content.as_ref();
                let style = Style::default().patch(span.style);
                for ch in text.chars() {
                    if remaining == 0 {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_symbol(&ch.to_string());
                        cell.set_style(style);
                    }
                    let width = unicode_width::UnicodeWidthChar::width(ch)
                        .unwrap_or(0)
                        .max(1) as u16;
                    x += width;
                    remaining = remaining.saturating_sub(width as usize);
                }
            }
        }
        let _ = theme;
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
