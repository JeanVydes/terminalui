//! Pane: a container that renders exactly one of its children.
//!
//! The host decides which child is visible via [`Pane::set_visible`];
//! every hidden child keeps its state, stays queryable through
//! [`Tree::find_mut`](crate::Tree::find_mut), but receives no input events,
//! no focus slot and no ticks.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;
use std::any::Any;

use crate::context::Context;
use crate::input::InputEvent;
use crate::node::Node;
use crate::outcome::Outcome;
use crate::theme::Theme;
use crate::view::View;

/// A stack of pages, one visible at a time.
#[derive(Serialize)]
pub struct Pane {
    children: Vec<Node>,
    visible: usize,
}

impl Pane {
    /// Creates a pane; the first child starts visible.
    pub fn new(children: Vec<Node>) -> Self {
        let mut pane = Self {
            visible: 0,
            children,
        };
        pane.sync_hidden();
        pane
    }

    /// Switch to the given page index. Out-of-range is ignored.
    pub fn set_visible(&mut self, index: usize) {
        if index < self.children.len() {
            self.visible = index;
            self.sync_hidden();
        }
    }

    /// Which page is visible now.
    pub fn visible(&self) -> usize {
        self.visible
    }

    fn sync_hidden(&mut self) {
        for (index, child) in self.children.iter_mut().enumerate() {
            child.hidden = index != self.visible;
        }
    }

    fn visible_child_mut(&mut self) -> Option<&mut Node> {
        self.children.get_mut(self.visible)
    }

    fn visible_child(&self) -> Option<&Node> {
        self.children.get(self.visible)
    }
}

impl View for Pane {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "pane"
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        // The visible page owns the whole area; stale glyphs from the
        // previous page would bleed through its unpainted cells.
        let blank = " ".repeat(usize::from(area.width));
        for row in area.y..area.y + area.height {
            buf.set_stringn(
                area.x,
                row,
                &blank,
                usize::from(area.width),
                ratatui::style::Style::default().bg(theme.background),
            );
        }
        if let Some(child) = self.visible_child_mut() {
            child.view.render(area, buf, cx, theme);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        // The tree dispatches to the children itself, skipping the hidden
        // ones; a view that dispatched internally as well would deliver
        // every event twice.
        Outcome::Ignored
    }

    fn focusable(&self) -> bool {
        self.visible_child()
            .map(|child| child.view.focusable())
            .unwrap_or(false)
    }
}
