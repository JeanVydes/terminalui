//! A text field with a filtering suggestion popup. Free text allowed,
//! suggestions are just that: suggestions. The popup opens once the field is
//! focused and filtered while typing; Up/Down move the highlight, Enter
//! applies it, Esc closes the popup and keeps the text.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::chrome::{draw_box, Chrome, Padding, Radius};
use super::linefield::LineField;
use super::menu::MenuStyle;
use super::scroll::{clamp_view, render_scrollbar, Scroll};

const MAX_VISIBLE: usize = 5;

/// Public state of the inline field and the popup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutocompleteState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the inline field is focused by the tree.
    pub focused: bool,
    /// Whether the suggestion popup is open.
    pub open: bool,
    /// Index of the highlighted suggestion inside the filtered view.
    pub highlight: usize,
    /// First visible suggestion index when the list overflows.
    pub offset: usize,
}

/// An inline single-line input with a suggestion dropdown under it. The
/// host feeds the candidate list; the component filters it while typing.
#[derive(Serialize)]
pub struct Autocomplete {
    /// The public state of the inline field and the popup visibility.
    pub state: AutocompleteState,
    /// Per-instance overrides; defaults match the theme.
    pub style: MenuStyle,
    /// The candidate suggestions shown in the popup.
    pub suggestions: Vec<String>,
    field: LineField,
    #[serde(skip)]
    item_hits: Vec<Rect>,
}

impl Autocomplete {
    /// Creates an autocomplete with the given suggestions. Starts empty, closed, not focused.
    pub fn new(suggestions: Vec<String>) -> Self {
        Self {
            state: AutocompleteState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                open: false,
                highlight: 0,
                offset: 0,
            },
            style: MenuStyle::default(),
            suggestions,
            field: LineField::new(),
            item_hits: Vec::new(),
        }
    }

    /// Replace the style.
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the candidate list. Keeps the current text and highlight.
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.clamp_highlight();
    }

    /// The inline text currently in the field.
    pub fn value(&self) -> &str {
        &self.field.value
    }

    /// Clear the field, its selection and its suggestion highlight.
    pub fn clear(&mut self) {
        self.field.clear();
        self.clamp_highlight();
    }

    /// Replace the inline text, keeping the suggestion list.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.field.set(value.into());
        self.state.highlight = 0;
        self.clamp_highlight();
    }

    /// Suggestions matching the current text, in their original order.
    pub fn filtered(&self) -> Vec<String> {
        self.matches()
            .into_iter()
            .map(|index| self.suggestions[index].clone())
            .collect()
    }

    /// Handle a key press without requiring tree focus. Hosts that own key
    /// dispatch can drive the field directly.
    pub fn handle_key(&mut self, key: &KeyEvent) -> Outcome {
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Up if self.state.open => self.nav(-1),
            KeyCode::Down => {
                if self.state.open {
                    self.nav(1)
                } else {
                    self.state.open = true;
                    self.clamp_highlight();
                    Outcome::Consumed
                }
            }
            KeyCode::Home => {
                self.field.move_home();
                Outcome::Consumed
            }
            KeyCode::End => {
                self.field.move_end();
                Outcome::Consumed
            }
            KeyCode::Enter => {
                if self.state.open {
                    self.apply_highlight()
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Esc if self.state.open => {
                self.state.open = false;
                Outcome::Consumed
            }
            KeyCode::Char(character) => {
                self.field.insert(character);
                self.state.open = true;
                self.state.highlight = 0;
                self.clamp_highlight();
                Outcome::Consumed
            }
            KeyCode::Backspace => {
                self.field.backspace();
                self.state.open = true;
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

    fn matches(&self) -> Vec<usize> {
        let needle = self.field.value.trim().to_lowercase();
        if needle.is_empty() {
            return (0..self.suggestions.len()).collect();
        }
        self.suggestions
            .iter()
            .enumerate()
            .filter(|(_, suggestion)| suggestion.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect()
    }

    fn clamp_highlight(&mut self) {
        let total = self.matches().len();
        self.state.highlight = self.state.highlight.min(total.saturating_sub(1));
        let visible = MAX_VISIBLE.min(total);
        let (highlight, offset) =
            clamp_view(self.state.highlight, self.state.offset, visible, total);
        self.state.highlight = highlight;
        self.state.offset = offset;
    }

    fn nav(&mut self, step: isize) -> Outcome {
        let total = self.matches().len();
        if total == 0 {
            return Outcome::Ignored;
        }
        let mut index = self.state.highlight as isize;
        index = (index + step).rem_euclid(total as isize);
        self.state.highlight = index as usize;
        self.clamp_highlight();
        Outcome::Consumed
    }

    fn apply_highlight(&mut self) -> Outcome {
        let matches = self.matches();
        let Some(&index) = matches.get(self.state.highlight) else {
            return Outcome::Ignored;
        };
        self.field.set(self.suggestions[index].clone());
        self.state.open = false;
        Outcome::Consumed
    }

    fn popup_rect(&self, bounds: Rect) -> Rect {
        let width = self
            .suggestions
            .iter()
            .map(|suggestion| suggestion.chars().count() as u16 + 2)
            .max()
            .unwrap_or(12)
            .max(self.state.area.width)
            .min(bounds.width);
        let height = (self.matches().len().clamp(1, MAX_VISIBLE)) as u16 + 2;
        let trigger_bottom = self.state.area.y + self.state.area.height;
        let below = bounds.y + bounds.height.saturating_sub(trigger_bottom);
        let y = if below >= height {
            trigger_bottom
        } else {
            self.state.area.y.saturating_sub(height).max(bounds.y)
        };
        let x = self
            .state
            .area
            .x
            .min(bounds.x + bounds.width.saturating_sub(width));
        Rect::new(x, y, width, height)
    }
}

impl Default for Autocomplete {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for Autocomplete {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "autocomplete"
    }

    fn outline(&self) -> Option<String> {
        let value = self.field.value.replace('\n', " ");
        Some(format!(
            "value={}, open={}, matches={}",
            terminalui_core::outline::quoted(&value),
            self.state.open,
            self.matches().len()
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((24, 1))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = cx.is_focused();
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let background = self.style.background.unwrap_or(theme.background);
        let chrome = Chrome {
            background: Some(background),
            border: None,
            radius: Radius::None,
            padding: Padding::default(),
            border_gradient: None,
            fill_gradient: None,
        };
        let inner = draw_box(buf, area, &chrome, theme);
        render_field(buf, inner, &self.field, theme);

        if !self.state.open || !self.state.focused {
            self.item_hits.clear();
            return;
        }
        let bounds = buf.area;
        let popup = self.popup_rect(bounds);
        let popup_chrome = Chrome {
            background: Some(theme.secondary),
            border: Some(super::chrome::Border::default()),
            radius: Radius::None,
            padding: Padding::default(),
            border_gradient: None,
            fill_gradient: None,
        };
        let popup_inner = draw_box(buf, popup, &popup_chrome, theme);
        let matches = self.matches();
        self.clamp_highlight();
        let visible = (popup_inner.height as usize).min(matches.len());
        let highlight_bg = self.style.highlight_bg.unwrap_or(theme.primary);
        let highlight_fg = self.style.highlight_fg.unwrap_or(theme.background);
        let suggestion = self.style.label.unwrap_or(theme.foreground);
        self.item_hits.clear();
        for visible_index in 0..visible {
            let match_index = self.state.offset + visible_index;
            let Some(&index) = matches.get(match_index) else {
                break;
            };
            let y = popup_inner.y + visible_index as u16;
            let highlighted = match_index == self.state.highlight;
            let row = Rect::new(popup_inner.x, y, popup_inner.width, 1);
            let row_style = if highlighted {
                Style::default()
                    .bg(highlight_bg)
                    .fg(highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(suggestion)
            };
            if highlighted {
                buf.set_style(row, row_style);
            }
            buf.set_stringn(
                popup_inner.x + 1,
                y,
                &self.suggestions[index],
                popup_inner.width.saturating_sub(2) as usize,
                row_style,
            );
            self.item_hits.push(row);
        }
        if matches.len() > MAX_VISIBLE {
            let style = super::scroll::ScrollbarStyle::default();
            let mut scroll = Scroll::new(visible, matches.len());
            scroll.offset = self.state.offset;
            let rail = Rect::new(
                popup_inner.x + popup_inner.width.saturating_sub(1),
                popup_inner.y,
                1,
                popup_inner.height,
            );
            render_scrollbar(buf, rail, &style, theme, &scroll);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        self.state.focused = cx.is_focused();
        if !self.state.focused {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => self.handle_key(key),
            InputEvent::Paste(text) => {
                self.field.paste(text);
                self.state.open = true;
                self.state.highlight = 0;
                self.clamp_highlight();
                Outcome::Consumed
            }
            InputEvent::Mouse(mouse) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Outcome::Ignored;
                }
                let position = Position::new(mouse.column, mouse.row);
                if self.state.open {
                    for (offset, rect) in self.item_hits.iter().enumerate() {
                        if rect.contains(position) {
                            self.state.highlight = self.state.offset + offset;
                            return self.apply_highlight();
                        }
                    }
                }
                if self.state.area.contains(position) {
                    self.state.open = true;
                    self.clamp_highlight();
                    return Outcome::Consumed;
                }
                self.state.open = false;
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        if self.state.open {
            self.apply_highlight();
        } else {
            self.state.open = true;
            self.clamp_highlight();
        }
        true
    }
}

fn render_field(buf: &mut Buffer, inner: Rect, field: &LineField, theme: &Theme) {
    let y = inner.y;
    let text_width = inner.width.saturating_sub(2) as usize;
    if field.value.is_empty() {
        let cursor_style = Style::default().fg(theme.background).bg(theme.accent);
        buf.set_string(inner.x, y, " ", cursor_style);
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
            inner.x + column as u16,
            y,
            character.to_string(),
            Style::default().fg(theme.foreground),
        );
    }
    let cursor_column = inner.x + (cursor - start) as u16;
    let cursor_style = Style::default().fg(theme.background).bg(theme.accent);
    if cursor < characters.len() {
        buf.set_string(
            cursor_column,
            y,
            characters[cursor].to_string(),
            cursor_style,
        );
    } else if cursor_column < inner.x + inner.width {
        buf.set_string(cursor_column, y, " ", cursor_style);
    }
}
