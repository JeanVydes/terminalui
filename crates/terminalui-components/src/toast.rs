use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, TextStyle, View};

use super::floating::{FloatingBox, FloatingPosition};

/// Toast significance. Maps to a color via the theme, like a badge variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToastState {
    pub area: Rect,
    pub elapsed: Duration,
}

/// A transient notification anchored inside its area. The content is composed:
/// pass arbitrary views to [`Toast::with_children`].
#[derive(Serialize)]
pub struct Toast {
    pub kind: ToastKind,
    pub ttl: Duration,
    pub state: ToastState,
    floating: FloatingBox,
}

impl Toast {
    /// Convenience constructor: a bold kind title plus the message as [`Text`] views.
    pub fn new(kind: ToastKind, message: impl Into<String>) -> Self {
        let children = vec![
            node(
                "title",
                Text::new(Self::kind_title(kind)).with_style(TextStyle::default().bold()),
            ),
            node("message", Text::new(message)),
        ];
        Self::with_children(kind, children)
    }

    /// Compose a toast from arbitrary views.
    pub fn with_children(kind: ToastKind, children: Vec<Node>) -> Self {
        Self {
            kind,
            ttl: Duration::from_secs(4),
            state: ToastState {
                area: Rect::new(0, 0, 0, 0),
                elapsed: Duration::ZERO,
            },
            floating: FloatingBox::new(children).with_position(FloatingPosition::BottomRight),
        }
    }
}
impl Toast {
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn with_position(mut self, position: FloatingPosition) -> Self {
        self.floating = self.floating.with_position(position);
        self
    }

    pub fn with_width(mut self, width: u16) -> Self {
        self.floating = self.floating.with_width(width);
        self
    }

    /// Whether the toast's time-to-live has elapsed.
    pub fn expired(&self) -> bool {
        self.state.elapsed >= self.ttl
    }

    fn kind_title(kind: ToastKind) -> &'static str {
        match kind {
            ToastKind::Info => "Info",
            ToastKind::Success => "Success",
            ToastKind::Warning => "Warning",
            ToastKind::Destructive => "Error",
        }
    }
}

impl View for Toast {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "toast"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "kind={}, expired={}",
            quoted(&format!("{:?}", self.kind)),
            self.expired()
        ))
    }

    fn children(&self) -> &[Node] {
        self.floating.children()
    }

    fn children_mut(&mut self) -> &mut [Node] {
        self.floating.children_mut()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let accent = match self.kind {
            ToastKind::Info => theme.accent,
            ToastKind::Success => theme.success,
            ToastKind::Warning => theme.warning,
            ToastKind::Destructive => theme.destructive,
        };
        self.floating.style.chrome.border = Some(super::chrome::Border {
            set: super::chrome::BorderSet::Plain,
            color: Some(accent),
            radius: super::chrome::Radius::None,
        });
        self.floating.render(area, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        self.state.elapsed += delta;
        self.floating.tick(delta, cx);
    }
}
