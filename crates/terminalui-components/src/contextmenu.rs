//! A context menu opened at a mouse coordinate. While closed it renders
//! nothing and has no focus; the host opens it on right-click with
//! [`ContextMenu::open_at`].
//!
//! Give it the full screen as its area (last child of the root stack) so it
//! can anchor anywhere and paint over everything else.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::chrome::draw_box;
use super::menu::{
    first_selectable, next_selectable, popup_height, popup_width, render_row, row_colors,
    MenuEntry, MenuStyle,
};
use super::scroll::{render_scrollbar, Scroll};

const MAX_VISIBLE: usize = 8;

/// Public state of the context menu: area, focus, open, highlight, offset, disabled, position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the menu would hold focus by the tree.
    pub focused: bool,
    /// Whether the menu is open.
    pub open: bool,
    /// Index of the highlighted entry.
    pub highlight: usize,
    /// First visible entry when the popup overflows.
    pub offset: usize,
    /// Whether interaction is disabled.
    pub disabled: bool,
    /// Anchor point set by [`ContextMenu::open_at`].
    pub position: (u16, u16),
}

/// A right-click style menu at the mouse position. Activation is reported
/// through [`ContextMenu::take_activated`].
#[derive(Serialize)]
pub struct ContextMenu {
    /// Public state: area, focus, open, highlight, offset, disabled, position.
    pub state: ContextState,
    /// Per-instance overrides; defaults match the theme.
    pub style: MenuStyle,
    /// The entries shown in the popup, in order.
    pub items: Vec<MenuEntry>,
    activated: Option<usize>,
    item_hits: Vec<Rect>,
}

impl ContextMenu {
    /// Creates a context menu with the given entries.
    pub fn new(items: Vec<MenuEntry>) -> Self {
        Self {
            state: ContextState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                open: false,
                highlight: 0,
                offset: 0,
                disabled: false,
                position: (0, 0),
            },
            style: MenuStyle::default(),
            items,
            activated: None,
            item_hits: Vec::new(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Open the menu anchored at `x`, `y` (a mouse coordinate).
    pub fn open_at(&mut self, x: u16, y: u16) {
        self.state.open = true;
        self.state.position = (x, y);
        self.state.highlight = first_selectable(&self.items).unwrap_or(0);
        self.state.offset = 0;
    }

    /// Close the menu.
    pub fn close(&mut self) {
        self.state.open = false;
    }

    /// Whether the menu is currently open.
    pub fn is_open(&self) -> bool {
        self.state.open
    }

    /// Drain the activation queue: the item index the user picked,
    /// if any, since the last call.
    pub fn take_activated(&mut self) -> Option<usize> {
        self.activated.take()
    }

    fn popup_rect(&self, bounds: Rect) -> Rect {
        let width = popup_width(&self.items).max(12);
        let height = popup_height(&self.items, MAX_VISIBLE);
        let (x, y) = self.state.position;
        let x = x.min(bounds.x + bounds.width.saturating_sub(width));
        let y = if y + height > bounds.y + bounds.height {
            y.saturating_sub(height)
        } else {
            y
        };
        Rect::new(x.max(bounds.x), y.max(bounds.y), width, height)
    }

    fn clamp_offset(&mut self) {
        let max_offset = self.items.len().saturating_sub(MAX_VISIBLE);
        if self.state.highlight < self.state.offset {
            self.state.offset = self.state.highlight;
        } else if self.state.highlight >= self.state.offset + MAX_VISIBLE {
            self.state.offset = self.state.highlight + 1 - MAX_VISIBLE;
        }
        self.state.offset = self.state.offset.min(max_offset);
    }

    fn activate(&mut self, index: usize) -> Outcome {
        self.state.highlight = index;
        self.activated = Some(index);
        self.state.open = false;
        Outcome::Consumed
    }

    fn nav(&mut self, step: isize) -> Outcome {
        match next_selectable(&self.items, self.state.highlight, step) {
            Some(index) => {
                self.state.highlight = index;
                self.clamp_offset();
                Outcome::Consumed
            }
            None => Outcome::Ignored,
        }
    }

    fn select_index(&mut self, index: usize) -> Outcome {
        match self.items.get(index) {
            Some(MenuEntry::Item(item)) if !item.disabled => self.activate(index),
            _ => Outcome::Ignored,
        }
    }
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for ContextMenu {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "context_menu"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "open={}, highlight={}",
            self.state.open, self.state.highlight
        ))
    }

    fn focusable(&self) -> bool {
        self.state.open && !self.state.disabled
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let focused = cx.is_focused();
        self.state.focused = self.state.open && focused;
        if !self.state.open {
            self.item_hits.clear();
            return;
        }
        let popup = self.popup_rect(area);
        let chrome = super::menu::popup_chrome(&self.style, theme);
        let inner = draw_box(buf, popup, &chrome, theme);
        let visible = (inner.height as usize).min(self.items.len());
        self.clamp_offset();
        let colors = row_colors(&self.style, theme);
        self.item_hits.clear();
        for visible_index in 0..visible {
            let index = self.state.offset + visible_index;
            let Some(entry) = self.items.get(index) else {
                break;
            };
            let y = inner.y + visible_index as u16;
            let highlighted = index == self.state.highlight;
            render_row(buf, inner, y, entry, highlighted, &colors);
            self.item_hits.push(Rect::new(inner.x, y, inner.width, 1));
        }
        if self.items.len() > MAX_VISIBLE {
            let style = self.style.scrollbar.clone().unwrap_or_default();
            let mut scroll = Scroll::new(visible, self.items.len());
            scroll.offset = self.state.offset;
            render_scrollbar(buf, inner, &style, theme, &scroll);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = self.state.open && focused;
        if !self.state.open || self.state.disabled {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if !self.state.focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Down => self.nav(1),
                    KeyCode::Up => self.nav(-1),
                    KeyCode::Home => match first_selectable(&self.items) {
                        Some(first) => {
                            self.state.highlight = first;
                            self.clamp_offset();
                            Outcome::Consumed
                        }
                        None => Outcome::Ignored,
                    },
                    KeyCode::End => match next_selectable(&self.items, self.items.len(), -1) {
                        Some(last) => {
                            self.state.highlight = last;
                            self.clamp_offset();
                            Outcome::Consumed
                        }
                        None => Outcome::Ignored,
                    },
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let index = self.state.highlight;
                        self.select_index(index)
                    }
                    KeyCode::Esc => {
                        self.close();
                        Outcome::Consumed
                    }
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Outcome::Ignored;
                }
                let in_rect = |rect: Rect| rect.contains(Position::new(mouse.column, mouse.row));
                if let Some(index) = self
                    .item_hits
                    .iter()
                    .position(|rect| in_rect(*rect))
                    .map(|offset| self.state.offset + offset)
                {
                    return self.select_index(index);
                }
                self.close();
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        if !self.state.open {
            return false;
        }
        let index = self.state.highlight;
        let _ = self.select_index(index);
        true
    }
}
