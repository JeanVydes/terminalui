//! Vertical and horizontal stacks.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

use crate::context::Context;
use crate::input::InputEvent;
use crate::node::Node;
use crate::outcome::Outcome;
use crate::outline::quoted;
use crate::theme::Theme;
use crate::view::View;

/// Direction the stack lays out its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StackDirection {
    /// Children stack top to bottom.
    Vertical,
    /// Children lay out left to right.
    Horizontal,
}

/// Stack is the primary container: it lays out composable children along
/// one axis with optional constraints and gap.
#[derive(Serialize)]
pub struct Stack {
    children: Vec<Node>,
    direction: StackDirection,
    gap: u16,
    #[serde(serialize_with = "serialize_constraints")]
    constraints: Vec<Constraint>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ConstraintView {
    Min(u16),
    Max(u16),
    Length(u16),
    Percentage(u16),
    Ratio(u32, u32),
    Fill(u16),
}

impl From<Constraint> for ConstraintView {
    fn from(constraint: Constraint) -> Self {
        match constraint {
            Constraint::Min(value) => Self::Min(value),
            Constraint::Max(value) => Self::Max(value),
            Constraint::Length(value) => Self::Length(value),
            Constraint::Percentage(value) => Self::Percentage(value),
            Constraint::Ratio(numerator, denominator) => Self::Ratio(numerator, denominator),
            Constraint::Fill(value) => Self::Fill(value),
        }
    }
}

fn serialize_constraints<S>(constraints: &[Constraint], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut sequence = serializer.serialize_seq(Some(constraints.len()))?;
    for constraint in constraints {
        sequence.serialize_element(&ConstraintView::from(*constraint))?;
    }
    sequence.end()
}

impl Stack {
    /// A vertical stack.
    pub fn vertical() -> Self {
        Self::new(StackDirection::Vertical)
    }

    /// A horizontal stack.
    pub fn horizontal() -> Self {
        Self::new(StackDirection::Horizontal)
    }

    fn new(direction: StackDirection) -> Self {
        Self {
            children: Vec::new(),
            direction,
            gap: 0,
            constraints: Vec::new(),
        }
    }

    /// Sets the children; builder style (consumes and returns `self`).
    pub fn children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    /// Cells between children. Default `0`.
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Layout constraints, one per child. Children without a constraint
    /// share the remaining space with `Fill(1)`.
    pub fn constraints(mut self, constraints: Vec<Constraint>) -> Self {
        self.constraints = constraints;
        self
    }

    fn child_areas(&self, area: Rect) -> Vec<Rect> {
        if self.children.is_empty() {
            return Vec::new();
        }
        let direction = match self.direction {
            StackDirection::Vertical => Direction::Vertical,
            StackDirection::Horizontal => Direction::Horizontal,
        };
        // Measured heights become `Length`: `Fill(1)` for every child makes
        // the layout solver exponential in the number of children.
        let constraints: Vec<Constraint> = (0..self.children.len())
            .map(|index| {
                self.constraints.get(index).copied().unwrap_or_else(|| {
                    self.children[index]
                        .view
                        .measure()
                        .map(|(_, height)| Constraint::Length(height))
                        .unwrap_or(Constraint::Fill(1))
                })
            })
            .collect();
        Layout::default()
            .direction(direction)
            .margin(0)
            .spacing(self.gap)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }
}

impl View for Stack {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn kind(&self) -> &'static str {
        "stack"
    }

    fn outline(&self) -> Option<String> {
        let direction = match self.direction {
            StackDirection::Vertical => "vertical",
            StackDirection::Horizontal => "horizontal",
        };
        Some(format!("direction={}", quoted(direction)))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let areas = self.child_areas(area);
        for (child, child_area) in self.children.iter_mut().zip(areas) {
            // `Layout` may return rects with y beyond the buffer when the
            // children do not fit; an empty child draws nothing but still
            // occupies a focus slot.
            if child_area.width == 0 || child_area.height == 0 {
                if child.view.focusable() {
                    cx.is_focused();
                }
                continue;
            }
            child.view.render(child_area, buf, cx, theme);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
