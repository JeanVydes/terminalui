//! Dropdown menu anchored to a trigger, looking and behaving like the
//! runtime select you get for free from [`super::select::Select`]: the
//! trigger content is composed by the host; the popup is built from items.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::chrome::{draw_box, Chrome, Padding};
use super::icons::Icon;
use super::layout::{row, Align};
use super::menu::{
    first_selectable, next_selectable, popup_height, popup_width, render_row, row_colors,
    MenuEntry, MenuStyle,
};
use super::scroll::{render_scrollbar, Scroll};

const MAX_VISIBLE: usize = 8;

/// Public state of the menu: area, focus, open, highlight and disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the menu would hold focus by the tree.
    pub focused: bool,
    /// Whether the popup is open.
    pub open: bool,
    /// Index of the highlighted entry.
    pub highlight: usize,
    /// First visible entry when the popup overflows.
    pub offset: usize,
    /// Whether interaction is disabled.
    pub disabled: bool,
}

/// A button-like trigger that opens a list of actions underneath it.
/// Activation is reported through [`DropdownMenu::take_activated`].
#[derive(Serialize)]
pub struct DropdownMenu {
    /// Public state: area, focus, open, highlight and disabled.
    pub state: MenuState,
    /// Per-instance overrides; defaults match the theme.
    pub style: MenuStyle,
    /// The entries shown in the popup, in order.
    pub items: Vec<MenuEntry>,
    trigger: Vec<Node>,
    activated: Option<usize>,
    item_hits: Vec<Rect>,
    trigger_hit: Rect,
}

impl DropdownMenu {
    /// Compose the trigger from arbitrary views (usually one `Text`).
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            state: MenuState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                open: false,
                highlight: 0,
                offset: 0,
                disabled: false,
            },
            style: MenuStyle::default(),
            items: Vec::new(),
            trigger: children,
            activated: None,
            item_hits: Vec::new(),
            trigger_hit: Rect::new(0, 0, 0, 0),
        }
    }

    /// Convenience constructor: the trigger is a [`terminalui_core::Text`] view.
    pub fn with_label(label: impl Into<String>) -> Self {
        Self::new(vec![terminalui_core::node(
            "label",
            terminalui_core::Text::new(label),
        )])
    }

    /// Replace the entries shown in the popup.
    pub fn with_items(mut self, items: Vec<MenuEntry>) -> Self {
        self.items = items;
        self.state.highlight = first_selectable(&self.items).unwrap_or(0);
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder shorthand: disable or enable interaction.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    /// Drain the activation queue: the index of the item the user picked,
    /// if any, since the last call.
    pub fn take_activated(&mut self) -> Option<usize> {
        self.activated.take()
    }

    /// The rect the popup renders into, anchored to the trigger row.
    /// Flips above when there is no room below; never leaves the screen.
    fn popup_rect(&self, bounds: Rect) -> Rect {
        let width = popup_width(&self.items).max(self.trigger_hit.width).max(16);
        let height = popup_height(&self.items, MAX_VISIBLE);
        let trigger_bottom = self.trigger_hit.y.saturating_add(self.trigger_hit.height);
        let space_below = bounds.y + bounds.height.saturating_sub(trigger_bottom);
        let space_above = self.trigger_hit.y.saturating_sub(bounds.y);
        let y = if space_below >= height || space_below >= space_above {
            trigger_bottom.min(bounds.y + bounds.height.saturating_sub(height))
        } else {
            self.trigger_hit.y.saturating_sub(height).max(bounds.y)
        };
        let max_x = bounds.x + bounds.width.saturating_sub(width);
        let x = self.trigger_hit.x.min(max_x);
        Rect::new(x, y, width, height)
    }

    fn clamp_offset(&mut self) {
        let visible = MAX_VISIBLE;
        let max_offset = self.items.len().saturating_sub(visible);
        if self.state.highlight < self.state.offset {
            self.state.offset = self.state.highlight;
        } else if self.state.highlight >= self.state.offset + visible {
            self.state.offset = self.state.highlight + 1 - visible;
        }
        self.state.offset = self.state.offset.min(max_offset);
    }

    fn activate(&mut self, index: usize) -> Outcome {
        self.state.highlight = index;
        self.activated = Some(index);
        self.state.open = false;
        Outcome::Consumed
    }

    fn open(&mut self) {
        self.state.open = true;
        self.state.offset = 0;
        self.state.highlight = first_selectable(&self.items).unwrap_or(0);
        self.clamp_offset();
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
}

impl Default for DropdownMenu {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for DropdownMenu {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "dropdown_menu"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "open={}, highlight={}, disabled={}",
            self.state.open, self.state.highlight, self.state.disabled
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((popup_width(&self.items).max(16), 1))
    }

    fn children(&self) -> &[Node] {
        &self.trigger
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.trigger
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = !self.state.disabled && cx.is_focused();
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let background = self.style.background.unwrap_or(theme.background);
        let trigger_chrome = Chrome {
            background: Some(background),
            border: self.style.border.or(Some(super::chrome::Border::default())),
            radius: self.style.radius,
            padding: Padding::default(),
            border_gradient: None,
            fill_gradient: None,
        };
        let trigger = Rect::new(area.x, area.y, area.width, 1);
        let inner = draw_box(buf, trigger, &trigger_chrome, theme);
        let content = Rect::new(inner.x, inner.y, inner.width.saturating_sub(2).max(1), 1);
        row(&mut self.trigger, content, Align::Left, 1, buf, cx, theme);
        let chevron = if self.state.open {
            Icon::ChevronUp.glyph()
        } else {
            Icon::ChevronDown.glyph()
        };
        buf.set_string(
            inner.x + inner.width.saturating_sub(1),
            inner.y,
            chevron,
            Style::default().fg(theme.muted),
        );
        self.trigger_hit = trigger;

        if !self.state.open {
            self.item_hits.clear();
            return;
        }
        let popup = self.popup_rect(buf.area);
        let popup_chrome = super::menu::popup_chrome(&self.style, theme);
        let popup_inner = draw_box(buf, popup, &popup_chrome, theme);
        let visible = (popup_inner.height as usize).min(self.items.len());
        self.clamp_offset();
        let colors = row_colors(&self.style, theme);
        self.item_hits.clear();
        for visible_index in 0..visible {
            let index = self.state.offset + visible_index;
            let Some(entry) = self.items.get(index) else {
                break;
            };
            let y = popup_inner.y + visible_index as u16;
            let highlighted = index == self.state.highlight;
            render_row(buf, popup_inner, y, entry, highlighted, &colors);
            self.item_hits
                .push(Rect::new(popup_inner.x, y, popup_inner.width, 1));
        }
        if self.items.len() > MAX_VISIBLE {
            let style = self.style.scrollbar.clone().unwrap_or_default();
            let mut scroll = Scroll::new(visible, self.items.len());
            scroll.offset = self.state.offset;
            render_scrollbar(buf, popup_inner, &style, theme, &scroll);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        self.state.focused = !self.state.disabled && cx.is_focused();
        if self.state.disabled {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if !self.state.focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Down => {
                        if self.state.open {
                            self.nav(1)
                        } else {
                            self.open();
                            Outcome::Consumed
                        }
                    }
                    KeyCode::Up if self.state.open => self.nav(-1),
                    KeyCode::Home if self.state.open => match first_selectable(&self.items) {
                        Some(first) => {
                            self.state.highlight = first;
                            self.clamp_offset();
                            Outcome::Consumed
                        }
                        None => Outcome::Ignored,
                    },
                    KeyCode::End if self.state.open => {
                        match next_selectable(&self.items, self.items.len(), -1) {
                            Some(last) => {
                                self.state.highlight = last;
                                self.clamp_offset();
                                Outcome::Consumed
                            }
                            None => Outcome::Ignored,
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if self.state.open {
                            let index = self.state.highlight;
                            self.activate(index)
                        } else {
                            self.open();
                            Outcome::Consumed
                        }
                    }
                    KeyCode::Esc if self.state.open => {
                        self.state.open = false;
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
                if in_rect(self.trigger_hit) {
                    if self.state.open {
                        self.state.open = false;
                    } else {
                        self.open();
                    }
                    return Outcome::Consumed;
                }
                if self.state.open {
                    if let Some(index) = self
                        .item_hits
                        .iter()
                        .position(|rect| in_rect(*rect))
                        .map(|offset| self.state.offset + offset)
                    {
                        if matches!(&self.items[index], super::menu::MenuEntry::Item(item) if !item.disabled)
                        {
                            return self.activate(index);
                        }
                    }
                    self.state.open = false;
                    return Outcome::Consumed;
                }
                Outcome::Ignored
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        !self.state.disabled
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        if self.state.disabled {
            return false;
        }
        if self.state.open {
            let index = self.state.highlight;
            self.activate(index);
        } else {
            self.open();
        }
        true
    }
}
