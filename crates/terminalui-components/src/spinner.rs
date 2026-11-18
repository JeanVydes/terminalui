//! Spinner: a rotating glyph for indeterminate or busy states.
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::layout::{row, Align};
use super::presentation::RenderStyle;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpinnerState {
    pub area: Rect,
    pub frame: usize,
    pub elapsed: Duration,
}

/// Per-instance overrides for a spinner. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SpinnerStyle {
    /// Glyph color. Defaults to `theme.accent`.
    pub color: Option<Color>,
    /// Milliseconds between frames.
    pub interval_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct Spinner {
    /// Which glyph set to draw. Defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    pub style: SpinnerStyle,
    pub state: SpinnerState,
    children: Vec<Node>,
}

impl Spinner {
    /// Convenience constructor: the label is a single [`Text`] view.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.children = vec![node("label", Text::new(label))];
        self
    }
}
impl Spinner {
    pub fn new() -> Self {
        Self {
            presentation: RenderStyle::Native,
            style: SpinnerStyle::default(),
            state: SpinnerState {
                area: Rect::new(0, 0, 0, 0),
                frame: 0,
                elapsed: Duration::ZERO,
            },
            children: Vec::new(),
        }
    }

    /// Compose a spinner with arbitrary views next to the glyph.
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    pub fn with_style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    fn interval_ms(&self) -> u64 {
        self.style.interval_ms.unwrap_or(90)
    }

    fn children_width(&self) -> u16 {
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum();
        measured + self.children.len().min(1) as u16
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Spinner {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "spinner"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("frame={}", self.state.frame))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let glyph = 2u16;
        let gap = if self.children.is_empty() { 0 } else { 1 };
        Some((glyph + gap + self.children_width(), 1))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let color = self.style.color.unwrap_or(theme.accent);
        let frames = self.presentation.frames();
        let glyph = frames[self.state.frame % frames.len()];
        let y = area.y + area.height.saturating_sub(1) / 2;
        buf.set_string(area.x, y, glyph, Style::default().fg(color));
        if self.children.is_empty() {
            return;
        }
        let inner = Rect::new(area.x + 2, y, area.width.saturating_sub(2), 1);
        row(&mut self.children, inner, Align::Left, 1, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        self.state.elapsed += delta;
        let interval = Duration::from_millis(self.interval_ms());
        let mut guard = 0usize;
        while self.state.elapsed >= interval && guard < 2048 {
            self.state.elapsed -= interval;
            self.state.frame = (self.state.frame + 1) % self.presentation.frames().len().max(1);
            guard += 1;
        }
        for child in &mut self.children {
            child.view.tick(delta, cx);
        }
    }
}
