//! A fuzzy-ish command palette: the host hands us a list of commands, we
//! own the filter field and the filtered list. Activation is reported
//! through [`CommandPalette::take_activated`].
//!
//! Give it the full screen as its area (last child of the root stack) so it
//! can dim everything behind it. Open/close with [`CommandPalette::toggle`];
//! the host decides the keybinding (usually Ctrl+P) and moves tree focus.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::chrome::{draw_box, Chrome, Padding};
use super::linefield::LineField;
use super::menu::{row_colors, MenuStyle};
use super::scroll::{clamp_view, render_scrollbar, Scroll};

const MAX_VISIBLE: usize = 8;

/// One command offered by the host. `detail` is muted text right-aligned on
/// the row (a group, a path, a shortcut). Disabled commands render muted and
/// are skipped by navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Command {
    /// Displayed label; what fuzzy matching runs over.
    pub label: String,
    /// Muted right column: a group, a path, a shortcut.
    pub detail: Option<String>,
    /// Muted and skipped by keyboard navigation when disabled.
    pub disabled: bool,
}

impl Command {
    /// Creates a command with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            disabled: false,
        }
    }

    /// Builder shorthand: set the muted right-column detail.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Builder shorthand: disable or enable the command.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Public state of the palette: area, focus, open, highlight and scroll offset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaletteState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the field would hold focus by the tree.
    pub focused: bool,
    /// Whether the palette is open.
    pub open: bool,
    /// Index of the highlighted command in the filtered view.
    pub highlight: usize,
    /// First visible command when the list overflows.
    pub offset: usize,
}

/// The palette. Rendered over everything when open; inert when closed.
#[derive(Serialize)]
pub struct CommandPalette {
    /// Public state: area, focus, open, highlight and offset.
    pub state: PaletteState,
    /// Per-instance overrides; defaults match the theme.
    pub style: MenuStyle,
    /// The commands offered by the palette.
    pub commands: Vec<Command>,
    field: LineField,
    #[serde(skip)]
    activated: Option<usize>,
    #[serde(skip)]
    item_hits: Vec<Rect>,
}

impl CommandPalette {
    /// Creates a palette with the given commands. Starts closed, not focused, filter empty.
    pub fn new(commands: Vec<Command>) -> Self {
        Self {
            state: PaletteState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                open: false,
                highlight: 0,
                offset: 0,
            },
            style: MenuStyle::default(),
            commands,
            field: LineField::new(),
            activated: None,
            item_hits: Vec::new(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Open the palette and reset the filter.
    pub fn open(&mut self) {
        self.state.open = true;
        self.state.offset = 0;
        self.field.clear();
        self.clamp_highlight();
    }

    /// Close the palette.
    pub fn close(&mut self) {
        self.state.open = false;
    }

    /// Toggle the palette.
    pub fn toggle(&mut self) {
        if self.state.open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Whether the palette is currently open.
    pub fn is_open(&self) -> bool {
        self.state.open
    }

    /// Drain the activation queue: the index into [`CommandPalette::commands`]
    /// the user picked, if any, since the last call.
    pub fn take_activated(&mut self) -> Option<usize> {
        self.activated.take()
    }

    fn matches(&self) -> Vec<usize> {
        let needle = self.field.value.trim().to_lowercase();
        if needle.is_empty() {
            return self
                .commands
                .iter()
                .enumerate()
                .map(|(index, _)| index)
                .collect();
        }
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, command)| {
                command.label.to_lowercase().contains(&needle)
                    || command
                        .detail
                        .as_ref()
                        .map(|detail| detail.to_lowercase().contains(&needle))
                        .unwrap_or(false)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn selectable(&self, matches: &[usize]) -> Vec<usize> {
        matches
            .iter()
            .enumerate()
            .filter(|(_, index)| !self.commands[**index].disabled)
            .map(|(index, _)| index)
            .collect()
    }

    fn clamp_highlight(&mut self) {
        let matches = self.matches();
        let total = matches.len();
        self.state.highlight = self.state.highlight.min(total.saturating_sub(1));
        let visible = MAX_VISIBLE.min(total);
        let (highlight, offset) =
            clamp_view(self.state.highlight, self.state.offset, visible, total);
        self.state.highlight = highlight;
        self.state.offset = offset;
    }

    fn activate_highlighted(&mut self) -> Outcome {
        let matches = self.matches();
        let Some(&index) = matches.get(self.state.highlight) else {
            return Outcome::Ignored;
        };
        if self.commands[index].disabled {
            return Outcome::Ignored;
        }
        self.activated = Some(index);
        self.close();
        Outcome::Consumed
    }

    fn nav(&mut self, step: isize) -> Outcome {
        let matches = self.matches();
        if matches.is_empty() {
            return Outcome::Ignored;
        }
        let selectable = self.selectable(&matches);
        if selectable.is_empty() {
            return Outcome::Ignored;
        }
        let current = self.state.highlight;
        let position = selectable
            .iter()
            .position(|&selected| selected == current)
            .unwrap_or(0);
        let next =
            selectable[(position as isize + step).rem_euclid(selectable.len() as isize) as usize];
        self.state.highlight = next;
        self.clamp_highlight();
        Outcome::Consumed
    }

    fn panel_rect(&self, area: Rect) -> Rect {
        let width = area.width.saturating_sub(4).clamp(16, 72);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height / 8).max(1);
        let rows = (self.matches().len()).clamp(1, MAX_VISIBLE) as u16;
        let height = rows + 4;
        Rect::new(x, y, width, height)
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for CommandPalette {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "command_palette"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "open={}, matches={}, highlight={}",
            self.state.open,
            self.matches().len(),
            self.state.highlight
        ))
    }

    fn focusable(&self) -> bool {
        self.state.open
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.state.area = area;
        let focused = cx.is_focused();
        self.state.focused = self.state.open && focused;
        if !self.state.open {
            self.item_hits.clear();
            return;
        }
        dim(buf, area, theme);
        let panel = self.panel_rect(area);
        let chrome = Chrome {
            background: Some(self.style.background.unwrap_or(theme.secondary)),
            border: Some(super::chrome::Border::default()),
            radius: super::chrome::Radius::None,
            padding: Padding::default(),
            border_gradient: None,
            fill_gradient: None,
        };
        let inner = draw_box(buf, panel, &chrome, theme);
        render_field(buf, inner, &self.field, theme);
        let list_top = inner.y + 1;
        let list_height = inner.height.saturating_sub(2) as usize;
        let matches = self.matches();
        self.clamp_highlight();
        let colors = row_colors(&self.style, theme);
        self.item_hits.clear();
        if matches.is_empty() {
            buf.set_string(
                inner.x + 1,
                list_top,
                "no matches",
                Style::default().fg(theme.muted),
            );
        } else {
            for visible in 0..list_height {
                let match_index = self.state.offset + visible;
                let Some(&index) = matches.get(match_index) else {
                    break;
                };
                let command = &self.commands[index];
                let y = list_top + visible as u16;
                let highlighted = match_index == self.state.highlight;
                let row_style = if highlighted {
                    Style::default()
                        .bg(colors.highlight_bg)
                        .fg(colors.highlight_fg)
                        .add_modifier(Modifier::BOLD)
                } else if command.disabled {
                    Style::default().fg(theme.muted)
                } else {
                    Style::default().fg(colors.label)
                };
                let row = Rect::new(inner.x, y, inner.width, 1);
                if highlighted {
                    buf.set_style(row, row_style);
                }
                buf.set_stringn(
                    inner.x + 1,
                    y,
                    &command.label,
                    inner.width.saturating_sub(2) as usize,
                    row_style,
                );
                if let Some(detail) = &command.detail {
                    let width = detail.chars().count() as u16;
                    if width + 2 <= inner.width {
                        let style = if highlighted {
                            row_style
                        } else {
                            Style::default().fg(theme.muted)
                        };
                        buf.set_string(inner.x + inner.width - width - 1, y, detail, style);
                    }
                }
                self.item_hits.push(row);
            }
            if matches.len() > MAX_VISIBLE {
                let style = super::scroll::ScrollbarStyle::default();
                let mut scroll = Scroll::new(list_height, matches.len());
                scroll.offset = self.state.offset;
                let bar = Rect::new(
                    inner.x + inner.width.saturating_sub(1),
                    list_top,
                    1,
                    list_height as u16,
                );
                render_scrollbar(buf, bar, &style, theme, &scroll);
            }
        }
        let footer = "↑↓ navigate   enter run   esc close".to_string();
        let footer_y = inner.y + inner.height - 1;
        buf.set_string(
            inner.x + 1,
            footer_y,
            footer,
            Style::default().fg(theme.muted),
        );
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = self.state.open && focused;
        if !self.state.focused {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Up => self.nav(-1),
                    KeyCode::Down => self.nav(1),
                    KeyCode::Home => {
                        self.field.move_home();
                        Outcome::Consumed
                    }
                    KeyCode::End => {
                        self.field.move_end();
                        Outcome::Consumed
                    }
                    KeyCode::Enter => self.activate_highlighted(),
                    KeyCode::Esc => {
                        self.close();
                        Outcome::Consumed
                    }
                    KeyCode::Char(character) => {
                        self.field.insert(character);
                        self.state.highlight = 0;
                        self.clamp_highlight();
                        Outcome::Consumed
                    }
                    KeyCode::Backspace => {
                        self.field.backspace();
                        self.state.highlight = 0;
                        self.clamp_highlight();
                        Outcome::Consumed
                    }
                    KeyCode::Delete => {
                        self.field.delete();
                        self.clamp_highlight();
                        Outcome::Consumed
                    }
                    KeyCode::Left => {
                        self.field.move_left();
                        Outcome::Consumed
                    }
                    KeyCode::Right => {
                        self.field.move_right();
                        Outcome::Consumed
                    }
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Paste(text) => {
                self.field.paste(text);
                self.state.highlight = 0;
                self.clamp_highlight();
                Outcome::Consumed
            }
            InputEvent::Mouse(mouse) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Outcome::Ignored;
                }
                let position = Position::new(mouse.column, mouse.row);
                for (offset, rect) in self.item_hits.iter().enumerate() {
                    if rect.contains(position) {
                        self.state.highlight = self.state.offset + offset;
                        return self.activate_highlighted();
                    }
                }
                let panel = self.panel_rect(self.state.area);
                if panel.contains(position) {
                    Outcome::Consumed
                } else {
                    self.close();
                    Outcome::Consumed
                }
            }
            _ => Outcome::Ignored,
        }
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        let _ = self.activate_highlighted();
        true
    }
}

fn dim(buf: &mut Buffer, area: Rect, theme: &Theme) {
    let background = match theme.background {
        Color::Rgb(red, green, blue) => Color::Rgb(
            (red as u32 * 40 / 100) as u8,
            (green as u32 * 40 / 100) as u8,
            (blue as u32 * 40 / 100) as u8,
        ),
        _ => theme.muted,
    };
    for y in area.y..area.y + area.height {
        buf.set_string(
            area.x,
            y,
            " ".repeat(area.width as usize),
            Style::default().bg(background),
        );
    }
}

fn render_field(buf: &mut Buffer, inner: Rect, field: &LineField, theme: &Theme) {
    let y = inner.y;
    let prompt_style = Style::default().fg(theme.muted);
    buf.set_string(inner.x + 1, y, "> ", prompt_style);
    let text_x = inner.x + 3;
    let text_width = inner.width.saturating_sub(4) as usize;
    if field.value.is_empty() {
        buf.set_string(
            text_x,
            y,
            "Search commands…",
            Style::default().fg(theme.muted),
        );
        let cursor_style = Style::default().fg(theme.background).bg(theme.accent);
        buf.set_string(text_x, y, " ", cursor_style);
        return;
    }
    let characters: Vec<char> = field.value.chars().collect();
    let cursor = field.cursor.min(characters.len());
    let start = if characters.len() <= text_width {
        0
    } else {
        characters.len() - text_width
    };
    let end = (start + text_width).min(characters.len());
    for (column, character) in characters[start..end].iter().enumerate() {
        buf.set_string(
            text_x + column as u16,
            y,
            character.to_string(),
            Style::default().fg(theme.foreground),
        );
    }
    let cursor_column = text_x + (cursor - start) as u16;
    let cursor_style = Style::default().fg(theme.background).bg(theme.accent);
    if cursor < characters.len() {
        buf.set_string(
            cursor_column,
            y,
            characters[cursor].to_string(),
            cursor_style,
        );
    } else if cursor_column < inner.x + inner.width - 1 {
        buf.set_string(cursor_column, y, " ", cursor_style);
    }
}
