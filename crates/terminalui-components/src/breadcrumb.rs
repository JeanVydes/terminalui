//! Breadcrumb: the trail from root to current inside the current area.
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::icons::Icon;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BreadcrumbState {
    pub area: Rect,
    pub focused: bool,
    /// Index into the composed segments used by keyboard navigation.
    pub highlight: usize,
}

/// Per-instance overrides for a breadcrumb. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BreadcrumbStyle {
    /// Color of the current (last) segment. Defaults to `theme.foreground`.
    pub foreground: Option<Color>,
    /// Color of the ancestor segments. Defaults to `theme.accent`.
    pub accent: Option<Color>,
    /// Separator color. Defaults to `theme.muted`.
    pub muted: Option<Color>,
    /// Bold the current segment. Default true.
    pub bold_current: Option<bool>,
    /// Separator between segments. Default `" › "`.
    pub separator: Option<String>,
}

/// A navigation path rendered as a row of composed segments separated by a
/// glyph. Segments are arbitrary views (usually [`terminalui_core::Text`]).
#[derive(Serialize)]
pub struct Breadcrumb {
    pub style: BreadcrumbStyle,
    pub state: BreadcrumbState,
    children: Vec<terminalui_core::Node>,
    hit_areas: Vec<Rect>,
}

impl Breadcrumb {
    /// Convenience constructor: every segment is a [`terminalui_core::Text`] view.
    pub fn with_segments(segments: Vec<impl Into<String>>) -> Self {
        use terminalui_core::{node, Text};
        let children = segments
            .into_iter()
            .enumerate()
            .map(|(index, segment)| node(format!("segment-{index}"), Text::new(segment)))
            .collect();
        Self::new(children)
    }

    /// Rebuild every segment as a [`terminalui_core::Text`] view.
    pub fn set_segments(&mut self, segments: Vec<impl Into<String>>) {
        use terminalui_core::{node, Text};
        self.children = segments
            .into_iter()
            .enumerate()
            .map(|(index, segment)| node(format!("segment-{index}"), Text::new(segment)))
            .collect();
        self.state.highlight = self.children.len().saturating_sub(1);
    }
}
impl Breadcrumb {
    /// Compose a breadcrumb from arbitrary views, one per segment.
    pub fn new(children: Vec<terminalui_core::Node>) -> Self {
        let highlight = children.len().saturating_sub(1);
        Self {
            style: BreadcrumbStyle::default(),
            state: BreadcrumbState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                highlight,
            },
            children,
            hit_areas: Vec::new(),
        }
    }

    pub fn with_style(mut self, style: BreadcrumbStyle) -> Self {
        self.style = style;
        self
    }

    /// The segment the keyboard highlight is on.
    pub fn highlight(&self) -> usize {
        self.state.highlight
    }

    fn hit_test(&self, column: u16, row: u16) -> Option<usize> {
        let position = Position::new(column, row);
        self.hit_areas
            .iter()
            .position(|rect| rect.contains(position))
    }
}

impl View for Breadcrumb {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "breadcrumb"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("highlight={}", self.state.highlight))
    }

    fn children(&self) -> &[terminalui_core::Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [terminalui_core::Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let focused = cx.is_focused();
        self.state.focused = focused;
        self.state.area = area;
        self.hit_areas.clear();
        if self.children.is_empty() {
            return;
        }
        let y = area.y + area.height.saturating_sub(1) / 2;
        let last = self.children.len() - 1;
        let foreground = self.style.foreground.unwrap_or(theme.foreground);
        let accent = self.style.accent.unwrap_or(theme.accent);
        let muted = self.style.muted.unwrap_or(theme.muted);
        let bold_current = self.style.bold_current.unwrap_or(true);
        let separator = self
            .style
            .separator
            .clone()
            .unwrap_or_else(|| format!(" {} ", Icon::ChevronRight.glyph()));
        let right = area.x + area.width;
        let mut x = area.x;
        for (index, child) in self.children.iter_mut().enumerate() {
            if x >= right {
                break;
            }
            let remaining = right - x;
            let width = child
                .view
                .measure()
                .map(|(width, _)| width.min(remaining))
                .unwrap_or(remaining)
                .max(1);
            let is_current = index == last;
            let highlight = self.state.focused && index == self.state.highlight;
            let mut style = Style::default().fg(if is_current { foreground } else { accent });
            if is_current && bold_current {
                style = style.add_modifier(Modifier::BOLD);
            }
            if highlight && !is_current {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            buf.set_style(Rect::new(x, y, width, 1), style);
            let cell = Rect::new(x, y, width, 1);
            child.view.render(cell, buf, cx, theme);
            self.hit_areas.push(cell);
            x = x.saturating_add(width);
            if !is_current {
                let sep: String = separator
                    .chars()
                    .take((right.saturating_sub(x)) as usize)
                    .collect();
                buf.set_string(x, y, &sep, Style::default().fg(muted));
                x = x.saturating_add(sep.chars().count() as u16);
            }
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        if self.children.is_empty() {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if !focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Left => {
                        self.state.highlight = self.state.highlight.saturating_sub(1);
                        Outcome::Consumed
                    }
                    KeyCode::Right => {
                        self.state.highlight =
                            (self.state.highlight + 1).min(self.children.len() - 1);
                        Outcome::Consumed
                    }
                    KeyCode::Home => {
                        self.state.highlight = 0;
                        Outcome::Consumed
                    }
                    KeyCode::End => {
                        self.state.highlight = self.children.len() - 1;
                        Outcome::Consumed
                    }
                    KeyCode::Enter => Outcome::Consumed,
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                let is_click = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
                if !is_click {
                    return Outcome::Ignored;
                }
                match self.hit_test(mouse.column, mouse.row) {
                    Some(index) => {
                        self.state.highlight = index;
                        Outcome::Consumed
                    }
                    None => Outcome::Ignored,
                }
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        true
    }
}
