//! An anchored popup for interactive content. Unlike the display-only
//! [`super::tooltip::Tooltip`], a popover is built to hold composed views
//! with behavior (buttons, inputs): children receive input through the
//! usual tree dispatch, and the popover stays until the host hides it.
//!
//! Optionally draws a caret on the edge nearest the trigger.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::any::Any;

use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::floating::{FloatingBox, FloatingPosition};

/// Which edge the caret is drawn on and which way it points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PopoverArrow {
    /// No caret. Default.
    #[default]
    None,
    /// Caret on the top edge, pointing up at the trigger.
    Top,
    /// Caret on the bottom edge, pointing down at the trigger.
    Bottom,
}

/// Public state of the popover: area, visibility and arrow edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PopoverState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the popover is visible.
    pub visible: bool,
    /// Which edge the caret sits on.
    pub arrow: PopoverArrow,
}

/// A popover anchored at a coordinate. The content is composed: pass
/// arbitrary views to [`Popover::with_children`]; host decides what the
/// popover means and when it closes (e.g. `show`/`hide`/`toggle` on the
/// trigger's events).
#[derive(Serialize)]
pub struct Popover {
    /// Public state: area, visibility and arrow edge.
    pub state: PopoverState,
    floating: FloatingBox,
}

impl Popover {
    /// Convenience constructor: the content is a single [`Text`] view.
    pub fn new(text: impl Into<String>, x: u16, y: u16) -> Self {
        let text = text.into();
        let width = (text.chars().count() as u16 + 4).clamp(16, 60);
        Self::with_children(vec![node("content", Text::new(text))], x, y).with_width(width)
    }

    /// Compose a popover from arbitrary views.
    pub fn with_children(children: Vec<Node>, x: u16, y: u16) -> Self {
        Self {
            state: PopoverState {
                area: Rect::new(0, 0, 0, 0),
                visible: true,
                arrow: PopoverArrow::None,
            },
            floating: FloatingBox::new(children)
                .with_position(FloatingPosition::Coordinates { x, y }),
        }
    }

    /// Builder shorthand: anchor the popover to a floating position.
    pub fn with_position(self, position: FloatingPosition) -> Self {
        let mut popover = self;
        popover.set_position(position);
        popover
    }

    /// Move the popover to a floating position.
    pub fn set_position(&mut self, position: FloatingPosition) {
        self.floating.set_position(position);
    }

    /// Builder shorthand: set the box width in cells.
    pub fn with_width(mut self, width: u16) -> Self {
        self.floating = self.floating.with_width(width);
        self
    }

    /// Caret on the given edge, e.g. `Top` for a popover opened below
    /// its trigger.
    pub fn with_arrow(mut self, arrow: PopoverArrow) -> Self {
        self.state.arrow = arrow;
        self
    }

    /// Show the popover.
    pub fn show(&mut self) {
        self.state.visible = true;
    }

    /// Hide the popover.
    pub fn hide(&mut self) {
        self.state.visible = false;
    }

    /// Toggle the visibility of the popover.
    pub fn toggle(&mut self) {
        self.state.visible = !self.state.visible;
    }

    /// Whether the popover is currently visible.
    pub fn visible(&self) -> bool {
        self.state.visible
    }
}

impl View for Popover {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "popover"
    }

    fn outline(&self) -> Option<String> {
        let caret = match self.state.arrow {
            PopoverArrow::None => "none",
            PopoverArrow::Top => "top",
            PopoverArrow::Bottom => "bottom",
        };
        Some(format!("visible={}, arrow={}", self.state.visible, caret))
    }

    fn children(&self) -> &[Node] {
        if self.state.visible {
            self.floating.children()
        } else {
            &[]
        }
    }

    fn children_mut(&mut self) -> &mut [Node] {
        if self.state.visible {
            self.floating.children_mut()
        } else {
            &mut []
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if !self.state.visible {
            return;
        }
        self.floating.render(area, buf, cx, theme);
        let rect = self.floating.box_rect(buf.area);
        let (glyph, edge_y) = match self.state.arrow {
            PopoverArrow::None => return,
            PopoverArrow::Top => ("▲", rect.y),
            PopoverArrow::Bottom => ("▼", rect.y + rect.height - 1),
        };
        let caret_x = rect.x + rect.width / 2;
        let border = self.floating.style.chrome.border.unwrap_or_default();
        let color = border.color.unwrap_or(theme.border);
        let style = ratatui::style::Style::default().fg(color);
        let background = match self.floating.style.chrome.background {
            Some(background) => background,
            None => theme.secondary,
        };
        buf.set_string(caret_x, edge_y, glyph, style.bg(background));
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        if !self.state.visible {
            return Outcome::Ignored;
        }
        self.floating.handle_event(event, cx)
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        if self.state.visible {
            self.floating.tick(delta, cx);
        }
    }
}
