//! Shared menu primitives: items, separators, the popup chrome and row
//! rendering both dropdown and context menus use.
//!
//! Not a component on its own; [`super::dropdown::DropdownMenu`] and
//! [`super::contextmenu::ContextMenu`] are built from these.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;

use terminalui_core::theme::Theme;

use super::chrome::{Border, Chrome, Padding, Radius};
use super::scroll::ScrollbarStyle;

/// One selectable entry's content. A menu item shows a label, an optional
/// glyph to its left (`icon`) and an optional shortcut to its right.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuItem {
    /// The label shown on the row.
    pub label: String,
    /// Optional glyph shown to the left of the label.
    pub icon: Option<String>,
    /// Optional shortcut hint shown to the right.
    pub shortcut: Option<String>,
    /// Muted and skipped by navigation when disabled.
    pub disabled: bool,
}

impl MenuItem {
    /// Creates a menu item with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            disabled: false,
        }
    }

    /// Builder shorthand: set the icon glyph.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Builder shorthand: set the shortcut hint.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Builder shorthand: disable or enable the item.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A menu entry: a selectable item or a visual separator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MenuEntry {
    /// A selectable item.
    Item(MenuItem),
    /// A visual separator; not selectable.
    Separator,
}

impl MenuEntry {
    /// Creates an item entry with the given label.
    pub fn item(label: impl Into<String>) -> Self {
        Self::Item(MenuItem::new(label))
    }

    /// Creates a separator entry.
    pub fn separator() -> Self {
        Self::Separator
    }
}

/// Per-instance overrides shared by the dropdown and context menus.
/// Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MenuStyle {
    /// Trigger/inline background. Defaults to `theme.background`.
    pub background: Option<Color>,
    /// Popup background. Defaults to `theme.secondary`.
    pub open_background: Option<Color>,
    /// Border of the trigger and the popup. Defaults to a plain border.
    pub border: Option<Border>,
    /// Corner radius of the trigger and the popup. Default: none.
    pub radius: Radius,
    /// Highlighted row background. Defaults to `theme.primary`.
    pub highlight_bg: Option<Color>,
    /// Highlighted row foreground. Defaults to `theme.background`.
    pub highlight_fg: Option<Color>,
    /// Icon color. Defaults to `theme.muted`.
    pub icon: Option<Color>,
    /// Label color. Defaults to `theme.foreground`.
    pub label: Option<Color>,
    /// Shortcut and separator color. Defaults to `theme.muted`.
    pub shortcut: Option<Color>,
    /// Scrollbar drawn when entries overflow the popup. Default: none.
    pub scrollbar: Option<ScrollbarStyle>,
}

/// Colors for a highlighted row, resolved per render pass.
pub(crate) struct RowColors {
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub icon: Color,
    pub label: Color,
    pub shortcut: Color,
}

pub(crate) fn row_colors(style: &MenuStyle, theme: &Theme) -> RowColors {
    RowColors {
        highlight_bg: style.highlight_bg.unwrap_or(theme.primary),
        highlight_fg: style.highlight_fg.unwrap_or(theme.background),
        icon: style.icon.unwrap_or(theme.muted),
        label: style.label.unwrap_or(theme.foreground),
        shortcut: style.shortcut.unwrap_or(theme.muted),
    }
}

/// Popup box: filled background and a border (plain by default).
pub(crate) fn popup_chrome(style: &MenuStyle, theme: &Theme) -> Chrome {
    Chrome {
        background: Some(style.open_background.unwrap_or(theme.secondary)),
        border: style.border.or(Some(Border::default())),
        radius: style.radius,
        padding: Padding::default(),
        border_gradient: None,
        fill_gradient: None,
    }
}

/// Width of one rendered row, padding included. Separators get a fixed
/// minimum so the popup never collapses to nothing.
pub(crate) fn row_width(entry: &MenuEntry) -> u16 {
    match entry {
        MenuEntry::Separator => 8,
        MenuEntry::Item(item) => {
            let icon = if item.icon.is_some() { 2 } else { 0 };
            let label = item.label.chars().count() as u16;
            let shortcut = item
                .shortcut
                .as_ref()
                .map(|shortcut| shortcut.chars().count() as u16 + 2)
                .unwrap_or(0);
            1 + icon + label + shortcut + 1
        }
    }
}

/// Max popup width for these entries.
pub(crate) fn popup_width(entries: &[MenuEntry]) -> u16 {
    entries.iter().map(row_width).max().unwrap_or(16).max(16)
}

/// Height of a popup showing at most `max_visible` entries, plus borders.
pub(crate) fn popup_height(entries: &[MenuEntry], max_visible: usize) -> u16 {
    entries.len().clamp(1, max_visible) as u16 + 2
}

/// Next selectable index from `from`, stepping by `step` and wrapping.
/// Skips separators and disabled items. `from` is inclusive only when the
/// direction is negative.
pub(crate) fn next_selectable(entries: &[MenuEntry], from: usize, step: isize) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }
    let len = entries.len() as isize;
    let mut index = from as isize;
    for _ in 0..entries.len() {
        index = (index + step).rem_euclid(len);
        if matches!(&entries[index as usize], MenuEntry::Item(item) if !item.disabled) {
            return Some(index as usize);
        }
    }
    None
}

/// First selectable index, if any.
pub(crate) fn first_selectable(entries: &[MenuEntry]) -> Option<usize> {
    entries
        .iter()
        .position(|entry| matches!(entry, MenuEntry::Item(item) if !item.disabled))
}

/// Render one row, or a separator, at `y`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_row(
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    entry: &MenuEntry,
    highlighted: bool,
    colors: &RowColors,
) {
    let row = Rect::new(inner.x, y, inner.width, 1);
    match entry {
        MenuEntry::Separator => {
            let line: String = "─".repeat(inner.width as usize);
            buf.set_string(inner.x, y, line, Style::default().fg(colors.shortcut));
        }
        MenuEntry::Item(item) => {
            let row_style = if highlighted {
                Style::default()
                    .bg(colors.highlight_bg)
                    .fg(colors.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.label)
            };
            if highlighted {
                buf.set_style(row, row_style);
            }
            let mut x = inner.x + 1;
            if let Some(icon) = &item.icon {
                let style = if highlighted {
                    row_style
                } else {
                    Style::default().fg(colors.icon)
                };
                buf.set_string(x, y, icon, style);
                x += 2;
            }
            buf.set_stringn(
                x,
                y,
                &item.label,
                inner.width.saturating_sub(2) as usize,
                row_style,
            );
            if let Some(shortcut) = &item.shortcut {
                let width = shortcut.chars().count() as u16;
                if width + 2 <= inner.width {
                    let style = if highlighted {
                        row_style
                    } else {
                        Style::default().fg(colors.shortcut)
                    };
                    buf.set_string(inner.x + inner.width - width - 1, y, shortcut, style);
                }
            }
        }
    }
}
