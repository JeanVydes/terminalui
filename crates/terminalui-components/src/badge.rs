//! Badge: a small status pill for categories, counts and states.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, TextStyle, View};

use super::layout::{row, Align};

/// Semantic significance of the badge. Maps to theme colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BadgeVariant {
    /// No semantic significance; uses the theme secondary color.
    #[default]
    Neutral,
    /// Positive/success state; uses the theme success color.
    Success,
    /// Warning state; uses the theme warning color.
    Warning,
    /// Error/destructive state; uses the theme destructive color.
    Destructive,
}

/// Per-instance overrides of colors and geometry for a badge. Defaults match the theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BadgeStyle {
    /// Badge background. Defaults to the variant's color.
    pub background: Option<Color>,
    /// Render the contents in bold. Default true.
    pub bold: Option<bool>,
    /// Horizontal padding in cells. Default 1.
    pub padding_x: Option<u16>,
}

/// Public state of the badge: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BadgeState {
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// A small status pill for categories, counts and states.
#[derive(Serialize)]
pub struct Badge {
    /// Semantic significance of the pill; colors a text with the theme.
    pub variant: BadgeVariant,
    /// Per-instance overrides of colors and geometry.
    pub style: BadgeStyle,
    /// Where the badge rendered in the last pass; the host may read it.
    pub state: BadgeState,
    children: Vec<Node>,
}

impl Badge {
    /// Convenience constructor: the content is a single [`Text`] view.
    pub fn new(content: impl Into<String>) -> Self {
        let text = Text::new(content).with_style(TextStyle::default().fg(Color::White).bold());
        Self::with_children(vec![node("content", text)])
    }
}

impl Badge {
    /// Compose a badge from arbitrary views.
    pub fn with_children(children: Vec<Node>) -> Self {
        Self {
            variant: BadgeVariant::default(),
            style: BadgeStyle::default(),
            state: BadgeState {
                area: Rect::new(0, 0, 0, 0),
            },
            children,
        }
    }

    /// Builder shorthand: set the variant.
    pub fn with_variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: BadgeStyle) -> Self {
        self.style = style;
        self
    }

    fn background(&self, theme: &Theme) -> Color {
        match self.variant {
            BadgeVariant::Neutral => theme.secondary,
            BadgeVariant::Success => theme.success,
            BadgeVariant::Warning => theme.warning,
            BadgeVariant::Destructive => theme.destructive,
        }
    }

    fn content_width(&self) -> u16 {
        let padding = self.style.padding_x.unwrap_or(1);
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum();
        measured + padding * 2
    }
}

impl Default for Badge {
    fn default() -> Self {
        Self::with_children(Vec::new())
    }
}

impl View for Badge {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "badge"
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((self.content_width(), 1))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let width = self.content_width().min(area.width);
        let padding = self.style.padding_x.unwrap_or(1);
        let background = self.style.background.unwrap_or(self.background(theme));
        let rect = Rect::new(area.x, area.y, width.max(1), 1);
        buf.set_style(rect, Style::default().bg(background));
        let inner_x = rect.x + padding.min(width);
        let inner_width = rect.width.saturating_sub(padding * 2).max(1);
        let inner = Rect::new(inner_x, rect.y, inner_width, 1);
        row(&mut self.children, inner, Align::Center, 0, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
