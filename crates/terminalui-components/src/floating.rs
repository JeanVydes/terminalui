//! FloatingBox: an anchored floating box that composites arbitrary children.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, TextStyle, View};

use super::chrome::{draw_box, Chrome};
use super::layout::column;

/// Screen position for a floating box.
///
/// One of the nine anchors, or free `Coordinates` clamped to keep the box
/// inside its area. The box never owns meaning; the host decides what goes
/// inside and how it is animated (e.g. the `terminalui_animations::Effect` type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FloatingPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    #[default]
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Coordinates {
        x: u16,
        y: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FloatingState {
    pub area: Rect,
}

/// Chrome configuration for the floating box: background, border set,
/// radius and padding. Defaults to a filled, unbordered box.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FloatingStyle {
    pub chrome: Chrome,
}

impl Default for FloatingStyle {
    fn default() -> Self {
        Self {
            chrome: Chrome::filled(Color::Rgb(63, 63, 79)),
        }
    }
}

/// A bordered, floating box anchored inside its area. Content is composed:
/// pass arbitrary views (text, badges, buttons) as children.
#[derive(Serialize)]
pub struct FloatingBox {
    pub width: u16,
    pub position: FloatingPosition,
    pub style: FloatingStyle,
    pub state: FloatingState,
    children: Vec<Node>,
}

impl FloatingBox {
    /// title styling included. The title renders in the accent color.
    pub fn with_title_body(title: impl Into<String>, body: Vec<impl Into<String>>) -> Self {
        let mut children = vec![node(
            "title",
            Text::new(title).with_style(TextStyle::default().bold()),
        )];
        children.extend(
            body.into_iter()
                .enumerate()
                .map(|(index, line)| node(format!("line-{index}"), Text::new(line))),
        );
        Self::new(children)
    }
}
impl FloatingBox {
    /// Compose a floating box from arbitrary views.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            width: 44,
            position: FloatingPosition::default(),
            style: FloatingStyle::default(),
            state: FloatingState {
                area: Rect::new(0, 0, 0, 0),
            },
            children,
        }
    }

    /// Convenience constructor: a title line plus body [`Text`] lines, all bold
    pub fn with_width(mut self, width: u16) -> Self {
        self.width = width.max(8);
        self
    }

    pub fn with_position(mut self, position: FloatingPosition) -> Self {
        self.position = position;
        self
    }

    pub fn set_position(&mut self, position: FloatingPosition) {
        self.position = position;
    }

    pub fn with_style(mut self, style: FloatingStyle) -> Self {
        self.style = style;
        self
    }

    /// Total content height in cells, derived from measured children.
    pub fn content_height(&self) -> u16 {
        let inner: u16 = self
            .children
            .iter()
            .map(|child| child.view.measure().map(|(_, height)| height).unwrap_or(1))
            .sum();
        let vertical = u16::from(self.style.chrome.border.is_some()) * 2;
        inner + self.style.chrome.padding.vertical * 2 + vertical
    }

    /// Resolves the box rect inside `bounds`. Bounds come from the frame
    /// buffer at render time, so a floating box can cover sibling views.
    pub fn box_rect(&self, bounds: Rect) -> Rect {
        let width = self.width.min(bounds.width).max(1);
        let height = self.content_height().min(bounds.height).max(1);
        let (x, y) = match self.position {
            FloatingPosition::TopLeft => (bounds.x, bounds.y),
            FloatingPosition::TopCenter => (bounds.x + (bounds.width - width) / 2, bounds.y),
            FloatingPosition::TopRight => (bounds.x + bounds.width - width, bounds.y),
            FloatingPosition::CenterLeft => (bounds.x, bounds.y + (bounds.height - height) / 2),
            FloatingPosition::Center => (
                bounds.x + (bounds.width - width) / 2,
                bounds.y + (bounds.height - height) / 2,
            ),
            FloatingPosition::CenterRight => (
                bounds.x + bounds.width - width,
                bounds.y + (bounds.height - height) / 2,
            ),
            FloatingPosition::BottomLeft => (bounds.x, bounds.y + bounds.height - height),
            FloatingPosition::BottomCenter => (
                bounds.x + (bounds.width - width) / 2,
                bounds.y + bounds.height - height,
            ),
            FloatingPosition::BottomRight => (
                bounds.x + bounds.width - width,
                bounds.y + bounds.height - height,
            ),
            FloatingPosition::Coordinates { x, y } => {
                let max_x = bounds.x + bounds.width.saturating_sub(width);
                let max_y = bounds.y + bounds.height.saturating_sub(height);
                (x.min(max_x), y.min(max_y))
            }
        };
        Rect::new(x, y, width, height)
    }
}

impl Default for FloatingBox {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for FloatingBox {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "floating_box"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "position={}",
            quoted(&format!("{:?}", self.position))
        ))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let rect = self.box_rect(buf.area);
        let mut chrome = self.style.chrome.clone();
        chrome.background = chrome.background.or(Some(theme.secondary));
        let inner = draw_box(buf, rect, &chrome, theme);
        column(&mut self.children, inner, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
