//! A bordered container view over the shared chrome abstraction.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::chrome::{draw_box, BorderSet, Chrome};

/// A structural container with optional border and fill.
///
/// Border glyphs come from [`Chrome`] (see [`BorderSet`]); the emphasized
/// variant paints the border in `theme.ring` instead of `theme.border`.
#[derive(Serialize)]
pub struct Panel {
    children: Vec<Node>,
    chrome: Chrome,
    emphasized: bool,
}

impl Panel {
    /// Creates an empty panel with transparent chrome.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            chrome: Chrome::transparent(),
            emphasized: false,
        }
    }

    /// Compose the panel from arbitrary views.
    pub fn children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    /// Fully customized chrome: border set, fill, radius, padding.
    pub fn with_chrome(mut self, chrome: Chrome) -> Self {
        self.chrome = chrome;
        self
    }

    /// A plain border in the theme's border color.
    pub fn line() -> Self {
        Self::new().with_chrome(Chrome::bordered(BorderSet::Plain, None))
    }

    /// A plain border in the theme's ring (focus accent) color.
    pub fn highlighted() -> Self {
        let mut panel = Self::new().with_chrome(Chrome::bordered(BorderSet::Plain, None));
        panel.emphasized = true;
        panel
    }

    /// Fill the panel with `color` instead of `theme.background`.
    pub fn background(mut self, color: Color) -> Self {
        self.chrome.background = Some(color);
        self
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Panel {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn kind(&self) -> &'static str {
        "panel"
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let mut chrome = self.chrome.clone();
        if self.emphasized {
            if let Some(border) = chrome.border.as_mut() {
                if border.color.is_none() {
                    border.color = Some(theme.ring);
                }
            }
        }
        if chrome.background.is_none() {
            chrome.background = Some(theme.background);
        }
        let inner = draw_box(buf, area, &chrome, theme);
        for child in &mut self.children {
            child.view.render(inner, buf, cx, theme);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
