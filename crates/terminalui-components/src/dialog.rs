//! Dialog: a modal overlay with a centered content box. Host-composed content.
use std::any::Any;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Stack, Text, TextStyle, View};

use super::button::{Button, ButtonVariant};
use super::chrome::{draw_box, Border, BorderSet, Chrome, Padding, Radius};
use super::layout::column;
use super::overlay::Overlay;

/// Per-instance overrides for a dialog. Defaults match the theme.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DialogStyle {
    /// Chrome of the content box: background, border, radius and padding.
    pub chrome: Chrome,
    /// Content box width as a percentage of the screen area (1-100). Default 60.
    pub width_percent: Option<u16>,
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self {
            chrome: Chrome {
                background: None,
                border: Some(Border {
                    set: BorderSet::Plain,
                    color: None,
                    radius: Radius::None,
                }),
                radius: Radius::None,
                padding: Padding {
                    horizontal: 1,
                    vertical: 1,
                },
                border_gradient: None,
                fill_gradient: None,
            },
            width_percent: None,
        }
    }
}

/// Public state of the dialog: whether it is open and where it rendered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DialogState {
    /// Whether the dialog is currently presented.
    pub is_open: bool,
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// A modal dialog: an overlay plus a centered content box. The content is
/// composed: any views the host passes as children render inside the box,
/// stacked vertically.
#[derive(Serialize)]
pub struct Dialog {
    /// Public state: whether the dialog is open and where it rendered.
    pub state: DialogState,
    /// The dimming overlay behind the content box.
    pub overlay: Overlay,
    /// Per-instance overrides; defaults match the theme.
    pub style: DialogStyle,
    children: Vec<Node>,
}

impl Dialog {
    /// A dialog with default confirm and cancel buttons.
    pub fn with_default_actions() -> Self {
        let actions = node(
            "actions",
            Stack::horizontal().gap(2).children(vec![
                node("cancel", Button::new(ButtonVariant::Secondary, "Cancel")),
                node("confirm", Button::new(ButtonVariant::Primary, "Confirm")),
            ]),
        );
        Self::new(vec![actions])
    }

    /// Insert a bold title as the first child.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        let text = Text::new(title).with_style(TextStyle::default().bold());
        self.children.insert(0, node("title", text));
        self
    }

    /// Insert a message right after the title (or as the first child).
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        let index = self
            .children
            .iter()
            .position(|child| child.id == "title")
            .map(|index| index + 1)
            .unwrap_or(0);
        self.children
            .insert(index, node("message", Text::new(message)));
        self
    }
}
impl Dialog {
    /// Compose a dialog from arbitrary views.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            state: DialogState {
                is_open: false,
                area: Rect::new(0, 0, 0, 0),
            },
            overlay: Overlay::new(),
            style: DialogStyle::default(),
            children,
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: DialogStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the content children entirely.
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    /// Open the dialog.
    pub fn open(&mut self) {
        self.state.is_open = true;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.state.is_open = false;
    }

    /// Whether the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.state.is_open
    }

    fn content_height(&self) -> u16 {
        let inner: u16 = self
            .children
            .iter()
            .map(|child| child.view.measure().map(|(_, height)| height).unwrap_or(1))
            .sum();
        let vertical = u16::from(self.style.chrome.border.is_some()) * 2;
        inner + self.style.chrome.padding.vertical * 2 + vertical
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::with_default_actions()
    }
}

impl View for Dialog {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "dialog"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("open={}", self.state.is_open))
    }

    fn children(&self) -> &[Node] {
        if self.state.is_open {
            &self.children
        } else {
            &[]
        }
    }

    fn children_mut(&mut self) -> &mut [Node] {
        if self.state.is_open {
            &mut self.children
        } else {
            &mut []
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if !self.state.is_open {
            return;
        }
        self.overlay.render(area, buf, cx, theme);
        let width_percent = self.style.width_percent.unwrap_or(60).clamp(1, 100) as u32;
        let content_width = ((area.width as u32 * width_percent) / 100) as u16;
        let content_width = content_width.max(10).min(area.width);
        let content_height = self.content_height().min(area.height).max(3);
        let content_x = area.x + area.width.saturating_sub(content_width) / 2;
        let content_y = area.y + area.height.saturating_sub(content_height) / 2;
        let content = Rect::new(content_x, content_y, content_width, content_height);
        let mut chrome = self.style.chrome.clone();
        chrome.background = chrome.background.or(Some(theme.secondary));
        chrome.border = chrome.border.map(|border| Border {
            color: border.color.or(Some(theme.border_set)),
            ..border
        });
        let inner = draw_box(buf, content, &chrome, theme);
        column(&mut self.children, inner, buf, cx, theme);
    }

    fn handle_event(&mut self, event: &InputEvent, _cx: &mut Context) -> Outcome {
        if !self.state.is_open {
            return Outcome::Ignored;
        }
        if let InputEvent::Key(key) = event {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc {
                self.state.is_open = false;
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        if !self.state.is_open {
            return;
        }
        self.overlay.tick(delta, cx);
        for child in &mut self.children {
            child.view.tick(delta, cx);
        }
    }
}
