//! A one-row status bar. Composed views flow from the left and from the
//! right edge, separated by a muted hairline. Focusable children see events
//! through the normal tree walk like anywhere else.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

const SEPARATOR: &str = " │ ";
/// Width assumed for children that do not implement `measure()`.
const DEFAULT_UNMEASURED: u16 = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusBarState {
    pub area: Rect,
}

/// Per-instance overrides for a status bar. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StatusBarStyle {
    /// Bar background. Defaults to `theme.secondary`.
    pub background: Option<Color>,
    /// Separator color. Defaults to `theme.muted`.
    pub separator: Option<Color>,
}

/// One row of segments pinned to the left and right edges of the bar.
/// The trailing [`StatusBar::right`] children render flush to the right.
#[derive(Serialize)]
pub struct StatusBar {
    pub state: StatusBarState,
    pub style: StatusBarStyle,
    children: Vec<Node>,
    /// Number of trailing children pinned to the right edge.
    right_count: usize,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            state: StatusBarState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: StatusBarStyle::default(),
            children: Vec::new(),
            right_count: 0,
        }
    }

    /// Append composed views to the left-pinned cluster.
    pub fn left(mut self, children: Vec<Node>) -> Self {
        let split = self.children.len() - self.right_count;
        let tail: Vec<Node> = self.children.split_off(split);
        self.children.extend(children);
        self.children.extend(tail);
        self
    }

    /// Append composed views to the right-pinned cluster.
    pub fn right(mut self, children: Vec<Node>) -> Self {
        self.right_count += children.len();
        self.children.extend(children);
        self
    }

    pub fn with_style(mut self, style: StatusBarStyle) -> Self {
        self.style = style;
        self
    }

    fn left_count(&self) -> usize {
        self.children.len() - self.right_count
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

fn width_of(node: &Node) -> u16 {
    node.view
        .measure()
        .map(|(width, _)| width)
        .unwrap_or(DEFAULT_UNMEASURED)
}

impl View for StatusBar {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "status_bar"
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let width: u16 = self
            .children
            .iter()
            .map(|node| width_of(node) + SEPARATOR.chars().count() as u16)
            .sum();
        Some((width.max(1), 1))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let background = self.style.background.unwrap_or(theme.secondary);
        let separator = self.style.separator.unwrap_or(theme.muted);
        buf.set_style(area, Style::default().bg(background));

        let separator_style = Style::default().fg(separator).bg(background);
        let count = self.left_count();
        let right_count = self.right_count;
        let sep = SEPARATOR.chars().count() as u16;
        let right_total: u16 = self.children[count..].iter().map(width_of).sum::<u16>()
            + sep.saturating_mul(right_count.saturating_sub(1) as u16);
        let protected = (area.x + area.width).saturating_sub(right_total);

        let mut x = area.x;
        for (index, child) in self.children[..count].iter_mut().enumerate() {
            let width = width_of(child).min(protected.saturating_sub(x));
            if width == 0 {
                break;
            }
            child
                .view
                .render(Rect::new(x, area.y, width, 1), buf, cx, theme);
            x += width;
            if index + 1 < count {
                buf.set_string(x, area.y, SEPARATOR, separator_style);
                x += sep;
            }
        }

        let mut right_x = protected;
        for (index, child) in self.children[count..].iter_mut().enumerate() {
            let width = width_of(child);
            child
                .view
                .render(Rect::new(right_x, area.y, width, 1), buf, cx, theme);
            right_x += width;
            if index + 1 < right_count {
                buf.set_string(right_x, area.y, SEPARATOR, separator_style);
                right_x += sep;
            }
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
