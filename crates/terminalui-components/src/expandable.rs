use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, TextStyle, View};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExpandableState {
    pub area: Rect,
    pub expanded: bool,
    pub focused: bool,
}

/// Per-instance overrides for an expandable. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ExpandableStyle {
    /// Header color. Defaults to `theme.foreground`.
    pub header: Option<Color>,
    /// Chevron color. Defaults to `theme.accent`.
    pub chevron: Option<Color>,
    /// Header background when focused. Defaults to `theme.muted`.
    pub highlight: Option<Color>,
    /// Render the header in bold. Default true.
    pub bold: Option<bool>,
}

/// A collapsible section. The first child is the always-visible header row;
/// the remaining children form the body, visible only when expanded.
#[derive(Serialize)]
pub struct Expandable {
    pub style: ExpandableStyle,
    pub state: ExpandableState,
    children: Vec<Node>,
}

impl Expandable {
    /// Convenience constructor: a title [`Text`] header.
    pub fn with_title(title: impl Into<String>) -> Self {
        Self::new(vec![node("header", Text::new(title))])
    }

    /// Convenience constructor: a title header plus body [`Text`] lines.
    pub fn with_lines(title: impl Into<String>, lines: Vec<impl Into<String>>) -> Self {
        let header = node(
            "header",
            Text::new(title).with_style(TextStyle::default().bold()),
        );
        let body = lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| node(format!("line-{index}"), Text::new(line)));
        Self::new(vec![header]).with_body(body.collect())
    }
}
impl Expandable {
    /// Compose an expandable from arbitrary views. The first child is the
    /// header; the rest collapse with the body.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            style: ExpandableStyle::default(),
            state: ExpandableState {
                area: Rect::new(0, 0, 0, 0),
                expanded: false,
                focused: false,
            },
            children,
        }
    }

    /// Append body views to the composed children.
    pub fn with_body(mut self, body: Vec<Node>) -> Self {
        self.children.extend(body);
        self
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.state.expanded = expanded;
        self
    }

    pub fn with_style(mut self, style: ExpandableStyle) -> Self {
        self.style = style;
        self
    }

    /// Toggle the expanded state. Returns the new state.
    pub fn toggle(&mut self) -> bool {
        self.state.expanded = !self.state.expanded;
        self.state.expanded
    }

    /// Whether the body is visible.
    pub fn expanded(&self) -> bool {
        self.state.expanded
    }
}

impl View for Expandable {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "expandable"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("expanded={}", self.state.expanded))
    }

    fn children(&self) -> &[Node] {
        if self.state.expanded {
            &self.children
        } else {
            &self.children[..1.min(self.children.len())]
        }
    }

    fn children_mut(&mut self) -> &mut [Node] {
        if self.state.expanded {
            &mut self.children
        } else {
            let end = 1.min(self.children.len());
            &mut self.children[..end]
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let focused = cx.is_focused();
        self.state.focused = focused;
        if self.children.is_empty() || area.height == 0 {
            return;
        }
        let header_fg = self.style.header.unwrap_or(theme.foreground);
        let chevron_color = self.style.chevron.unwrap_or(theme.accent);
        let highlight = self.style.highlight.unwrap_or(theme.muted);
        let bold = self.style.bold.unwrap_or(true);
        let chevron = if self.state.expanded { "▼" } else { "▶" };
        if focused {
            buf.set_style(
                Rect::new(area.x, area.y, area.width, 1),
                Style::default().bg(highlight),
            );
        }
        buf.set_string(area.x, area.y, chevron, Style::default().fg(chevron_color));
        let header_rect = Rect::new(area.x + 2, area.y, area.width.saturating_sub(2), 1);
        let mut header_style = Style::default().fg(header_fg);
        if bold {
            header_style = header_style.add_modifier(Modifier::BOLD);
        }
        buf.set_style(header_rect, header_style);
        self.children[0].view.render(header_rect, buf, cx, theme);

        if self.state.expanded {
            let body_area = Rect::new(
                area.x + 2,
                area.y + 1,
                area.width.saturating_sub(2),
                area.height.saturating_sub(1),
            );
            let mut offset = 0usize;
            for child in self.children.iter_mut().skip(1) {
                if offset >= body_area.height as usize {
                    break;
                }
                let child_height = child
                    .view
                    .measure()
                    .map(|(_, height)| height.max(1))
                    .unwrap_or(1)
                    .min((body_area.height as usize - offset) as u16);
                let cell = Rect::new(
                    body_area.x,
                    body_area.y + offset as u16,
                    body_area.width,
                    child_height,
                );
                child.view.render(cell, buf, cx, theme);
                offset += child_height as usize;
            }
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        match event {
            InputEvent::Key(key) => {
                if !focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.toggle();
                        Outcome::Consumed
                    }
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                let is_click = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
                if !is_click {
                    return Outcome::Ignored;
                }
                let inside_header = mouse.row == self.state.area.y
                    && mouse.column >= self.state.area.x
                    && mouse.column < self.state.area.x + self.state.area.width;
                if inside_header {
                    self.toggle();
                    return Outcome::Consumed;
                }
                Outcome::Ignored
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.toggle();
        true
    }
}
