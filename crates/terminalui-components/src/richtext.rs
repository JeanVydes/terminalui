//! RichText: a view that paints composed ratatui [`Line`]s.
//!
//! Terminal apps migrating hand-drawn ratatui code onto a view tree still
//! hold their rendering as [`Line`]s and spans; this view lets them paint
//! those lines directly as one terminalui node instead of translating them
//! back into strings and styles by hand.
//!
//! The view renders each line top to bottom, left to right, truncating at
//! the width it is given. It owns no input and never holds focus.

use crossterm::event::{MouseButton, MouseEventKind};
use std::any::Any;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
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
    /// Clickable ranges in rendered cell columns.
    pub links: Vec<RichTextLink>,
    #[serde(skip)]
    hit_areas: Vec<LinkHitArea>,
    #[serde(skip)]
    activated_link: Option<String>,
}

/// Clickable range on a line, using zero-based line and cell columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RichTextLink {
    /// Zero-based line in [`RichText::lines`].
    pub line: usize,
    /// Inclusive starting cell column, before indentation.
    pub start: usize,
    /// Exclusive ending cell column, before indentation.
    pub end: usize,
    /// Host-defined target returned by [`RichText::take_activated_link`].
    pub target: String,
}

#[derive(Debug, Clone)]
struct LinkHitArea {
    area: Rect,
    target: String,
}

impl RichText {
    /// Creates a view painting the given lines.
    pub fn new(lines: Vec<Line<'static>>) -> Self {
        Self {
            lines,
            indent: 0,
            links: Vec::new(),
            hit_areas: Vec::new(),
            activated_link: None,
        }
    }

    /// Indent every line by this many cells.
    pub fn with_indent(mut self, indent: u16) -> Self {
        self.indent = indent;
        self
    }

    /// Mark a range as a link. Coordinates count terminal cells, so wide
    /// characters occupy two columns. The target is opaque to terminalui.
    pub fn with_link(
        mut self,
        line: usize,
        start: usize,
        end: usize,
        target: impl Into<String>,
    ) -> Self {
        if start < end && line < self.lines.len() {
            self.links.push(RichTextLink {
                line,
                start,
                end,
                target: target.into(),
            });
        }
        self
    }

    /// Take the target activated by the last link click, if any.
    pub fn take_activated_link(&mut self) -> Option<String> {
        self.activated_link.take()
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
        self.hit_areas.clear();
        for (row, line) in self.lines.iter().enumerate() {
            let y = area.y + row as u16;
            if y >= area.y + area.height {
                break;
            }
            let mut x = area.x + self.indent;
            let mut remaining = max_width;
            let mut column = 0usize;
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
                    let cell_width = unicode_width::UnicodeWidthChar::width(ch)
                        .unwrap_or(0)
                        .max(1);
                    let linked = self.links.iter().any(|link| {
                        link.line == row && column < link.end && column + cell_width > link.start
                    });
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_symbol(&ch.to_string());
                        cell.set_style(if linked {
                            style.fg(theme.accent).add_modifier(Modifier::UNDERLINED)
                        } else {
                            style
                        });
                    }
                    let width = cell_width as u16;
                    x += width;
                    remaining = remaining.saturating_sub(width as usize);
                    column = column.saturating_add(cell_width);
                }
            }
            for link in self.links.iter().filter(|link| link.line == row) {
                let visible_width = Self::line_width(line).min(max_width);
                let start = link.start.min(visible_width);
                let end = link.end.min(visible_width);
                if start < end {
                    self.hit_areas.push(LinkHitArea {
                        area: Rect::new(
                            area.x
                                .saturating_add(self.indent)
                                .saturating_add(start as u16),
                            y,
                            (end - start) as u16,
                            1,
                        ),
                        target: link.target.clone(),
                    });
                }
            }
        }
    }

    fn handle_event(&mut self, event: &InputEvent, _cx: &mut Context) -> Outcome {
        let InputEvent::Mouse(mouse) = event else {
            return Outcome::Ignored;
        };
        if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return Outcome::Ignored;
        }
        let position = Position::new(mouse.column, mouse.row);
        if let Some(link) = self
            .hit_areas
            .iter()
            .find(|link| link.area.contains(position))
        {
            self.activated_link = Some(link.target.clone());
            Outcome::Consumed
        } else {
            Outcome::Ignored
        }
    }
}
