//! Tooltip: a floating hint anchored at a point; display-only.
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::floating::{FloatingBox, FloatingPosition};

/// Public state of the tooltip: area and visibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TooltipState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the tooltip is visible.
    pub visible: bool,
}

/// A floating hint anchored at a coordinate, typically a cell under a
/// hovered view. The content is composed: pass arbitrary views to
/// [`Tooltip::with_children`]. The host decides what the hint says and when
/// it shows. Tooltips are display-only.
#[derive(Serialize)]
pub struct Tooltip {
    /// Public state: area and visibility.
    pub state: TooltipState,
    floating: FloatingBox,
}

impl Tooltip {
    /// Convenience constructor: the content is a single [`Text`] view.
    pub fn new(text: impl Into<String>, x: u16, y: u16) -> Self {
        let text = text.into();
        let width = (text.chars().count() as u16 + 4).clamp(12, 60);
        Self::with_children(vec![node("content", Text::new(text))], x, y).with_width(width)
    }
}
impl Tooltip {
    /// Compose a tooltip from arbitrary views.
    pub fn with_children(children: Vec<Node>, x: u16, y: u16) -> Self {
        Self {
            state: TooltipState {
                area: Rect::new(0, 0, 0, 0),
                visible: true,
            },
            floating: FloatingBox::new(children)
                .with_position(FloatingPosition::Coordinates { x, y }),
        }
    }

    /// Builder shorthand: anchor the tooltip to a floating position.
    pub fn with_position(mut self, position: FloatingPosition) -> Self {
        self.set_position(position);
        self
    }

    /// Move the tooltip to a floating position.
    pub fn set_position(&mut self, position: FloatingPosition) {
        self.floating.set_position(position);
    }

    /// Builder shorthand: set the box width in cells.
    pub fn with_width(mut self, width: u16) -> Self {
        self.floating = self.floating.with_width(width);
        self
    }

    /// Show the tooltip.
    pub fn show(&mut self) {
        self.state.visible = true;
    }

    /// Hide the tooltip.
    pub fn hide(&mut self) {
        self.state.visible = false;
    }

    /// Whether the tooltip is currently visible.
    pub fn visible(&self) -> bool {
        self.state.visible
    }
}

impl View for Tooltip {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "tooltip"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("visible={}", self.state.visible))
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
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        if !self.state.visible {
            return Outcome::Ignored;
        }
        self.floating.handle_event(event, cx)
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        self.floating.tick(delta, cx);
    }
}
