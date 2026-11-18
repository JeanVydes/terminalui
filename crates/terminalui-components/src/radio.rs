//! Single-choice radio group. Options are composed views (usually [`Text`]);
//! the group owns where the glyph goes and which option is active.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use super::scroll::clamp_view;
use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

/// Public state of the group: area, cursor, offset, focus and disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RadioState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Option the keyboard and mouse point at.
    pub cursor: usize,
    /// First visible option when the list overflows.
    pub offset: usize,
    /// Whether the group would hold focus by the tree.
    pub focused: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
}

/// Per-instance overrides for a radio group. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RadioStyle {
    /// Highlighted option background. Defaults to `theme.muted`.
    pub highlight_bg: Option<Color>,
    /// Highlighted option text color. Defaults to `theme.foreground`.
    pub highlight_fg: Option<Color>,
    /// Selected glyph color. Defaults to `theme.primary`.
    pub on: Option<Color>,
    /// Unselected glyph color. Defaults to `theme.foreground`.
    pub off: Option<Color>,
    /// Bold the highlighted option. Default true.
    pub bold: Option<bool>,
}

/// A vertical group of single-choice options. The active option is
/// reported through [`RadioGroup::selected`].
#[derive(Serialize)]
pub struct RadioGroup {
    /// Public state: area, cursor, offset, focus and disabled.
    pub state: RadioState,
    /// Per-instance overrides; defaults match the theme.
    pub style: RadioStyle,
    /// Active option, if any has been picked.
    pub selected: Option<usize>,
    children: Vec<Node>,
    hit_areas: Vec<Rect>,
}

impl RadioGroup {
    /// Convenience constructor: every option is a [`Text`] view.
    pub fn with_options(options: Vec<impl Into<String>>) -> Self {
        let children = options
            .into_iter()
            .enumerate()
            .map(|(index, option)| node(format!("option-{index}"), Text::new(option)))
            .collect();
        Self::new(children)
    }

    /// Compose a group from arbitrary views, one per option.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            state: RadioState {
                area: Rect::new(0, 0, 0, 0),
                cursor: 0,
                offset: 0,
                focused: false,
                disabled: false,
            },
            style: RadioStyle::default(),
            selected: None,
            children,
            hit_areas: Vec::new(),
        }
    }

    /// Builder shorthand: pre-select the given option index.
    pub fn with_selected(mut self, selected: usize) -> Self {
        if selected < self.children.len() {
            self.selected = Some(selected);
            self.state.cursor = selected;
        }
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: RadioStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder shorthand: disable or enable interaction.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    /// The active option, if any.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    fn body_height(&self) -> usize {
        self.state.area.height as usize
    }

    fn clamp_view(&mut self) {
        let (cursor, offset) = clamp_view(
            self.state.cursor,
            self.state.offset,
            self.body_height(),
            self.children.len(),
        );
        self.state.cursor = cursor;
        self.state.offset = offset;
    }

    fn pick(&mut self, index: usize) -> Outcome {
        if self.children.is_empty() {
            return Outcome::Ignored;
        }
        let index = index.min(self.children.len() - 1);
        self.state.cursor = index;
        self.clamp_view();
        self.selected = Some(index);
        Outcome::Consumed
    }

    fn move_cursor(&mut self, index: usize) -> Outcome {
        if self.children.is_empty() {
            return Outcome::Ignored;
        }
        self.state.cursor = index.min(self.children.len() - 1);
        self.clamp_view();
        Outcome::Consumed
    }
}

impl Default for RadioGroup {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for RadioGroup {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "radio_group"
    }

    fn outline(&self) -> Option<String> {
        let selected = self
            .selected
            .map(|index| index.to_string())
            .unwrap_or_else(|| "none".to_string());
        Some(format!(
            "selected={}, offset={}, disabled={}",
            selected, self.state.offset, self.state.disabled
        ))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = !self.state.disabled && cx.is_focused();
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.clamp_view();
        let highlight_bg = self.style.highlight_bg.unwrap_or(theme.muted);
        let highlight_fg = self.style.highlight_fg.unwrap_or(theme.foreground);
        let on_color = self.style.on.unwrap_or(theme.primary);
        let off_color = self.style.off.unwrap_or(theme.foreground);
        let bold = self.style.bold.unwrap_or(true);
        self.hit_areas.clear();

        let visible = self.body_height();
        for visible_index in 0..visible {
            let index = self.state.offset + visible_index;
            let Some(child) = self.children.get_mut(index) else {
                break;
            };
            let y = area.y + visible_index as u16;
            let active = self.selected == Some(index);
            let highlighted = index == self.state.cursor && self.state.focused;
            let mut row_style = Style::default();
            if highlighted {
                row_style = row_style.bg(highlight_bg).fg(highlight_fg);
                if bold {
                    row_style = row_style.add_modifier(Modifier::BOLD);
                }
            }
            let row_rect = Rect::new(area.x, y, area.width, 1);
            buf.set_style(row_rect, row_style);
            let glyph_color = if self.state.disabled {
                theme.muted
            } else if active {
                on_color
            } else {
                off_color
            };
            let glyph = if active { "(●)" } else { "( )" };
            buf.set_string(area.x, y, glyph, Style::default().fg(glyph_color));
            let content = Rect::new(area.x + 4, y, area.width.saturating_sub(4), 1);
            child.view.render(content, buf, cx, theme);
            self.hit_areas.push(row_rect);
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
                    KeyCode::Up => {
                        let index = self.state.cursor.saturating_sub(1);
                        self.move_cursor(index)
                    }
                    KeyCode::Down => self.move_cursor(self.state.cursor + 1),
                    KeyCode::Home => self.move_cursor(0),
                    KeyCode::End => self.move_cursor(usize::MAX),
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let index = self.state.cursor;
                        self.pick(index)
                    }
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Outcome::Ignored;
                }
                let hit = self
                    .hit_areas
                    .iter()
                    .position(|rect| rect.contains(Position::new(mouse.column, mouse.row)));
                match hit {
                    Some(index) => self.pick(index),
                    None => Outcome::Ignored,
                }
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
        let index = self.state.cursor;
        self.pick(index);
        true
    }
}
