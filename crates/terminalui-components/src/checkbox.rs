//! Checkbox: a labelled toggle with [x] / [ ] states and an optional border.
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::chrome::{draw_box, Border, Chrome, Padding, Radius};
use super::layout::{row, Align};

/// Per-instance overrides for a checkbox. Defaults match the theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckboxStyle {
    /// Glyph color when checked. Defaults to `theme.success`.
    pub checked: Option<Color>,
    /// Glyph color when unchecked. Defaults to `theme.foreground`.
    pub foreground: Option<Color>,
    /// Box background. Defaults to none (transparent).
    pub background: Option<Color>,
    /// Box border. Default: none.
    pub border: Option<Border>,
    /// Radius of the box. Default: none.
    pub radius: Radius,
    /// Glyph for the checked state. Default `[x]`.
    pub checked_glyph: char,
    /// Glyph for the unchecked state. Default `[ ]`.
    pub unchecked_glyph: char,
}

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self {
            checked: None,
            foreground: None,
            background: None,
            border: None,
            radius: Radius::None,
            checked_glyph: 'x',
            unchecked_glyph: ' ',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckboxState {
    pub area: Rect,
    pub checked: bool,
    pub focused: bool,
    pub disabled: bool,
}

#[derive(Serialize)]
pub struct Checkbox {
    pub state: CheckboxState,
    pub style: CheckboxStyle,
    children: Vec<Node>,
}

impl Checkbox {
    /// Convenience: a single text label.
    pub fn with_label(text: impl Into<String>) -> Self {
        Self::new(vec![node("label", Text::new(text))])
    }
}

impl Checkbox {
    /// Compose a checkbox from arbitrary views as its label.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            state: CheckboxState {
                area: Rect::new(0, 0, 0, 0),
                checked: false,
                focused: false,
                disabled: false,
            },
            style: CheckboxStyle::default(),
            children,
        }
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    pub fn with_style(mut self, style: CheckboxStyle) -> Self {
        self.style = style;
        self
    }

    pub fn is_checked(&self) -> bool {
        self.state.checked
    }

    /// Flip the checked state.
    pub fn toggle(&mut self) -> bool {
        if self.state.disabled {
            return self.state.checked;
        }
        self.state.checked = !self.state.checked;
        self.state.checked
    }
}

impl View for Checkbox {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "checkbox"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "on={}, disabled={}",
            self.state.checked, self.state.disabled
        ))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = if self.state.disabled {
            false
        } else {
            cx.is_focused()
        };
        self.state.area = area;
        let chrome = Chrome {
            background: self.style.background,
            border: self.style.border.map(|border| {
                if self.state.focused {
                    Border {
                        color: Some(theme.accent),
                        ..border
                    }
                } else {
                    border
                }
            }),
            radius: self.style.radius,
            padding: Padding::default(),
            border_gradient: None,
            fill_gradient: None,
        };
        let inner = draw_box(buf, area, &chrome, theme);
        let checked_color = self.style.checked.unwrap_or(theme.success);
        let fg = self.style.foreground.unwrap_or(theme.foreground);
        let y = inner.y + inner.height / 2;
        let box_color = if self.state.disabled {
            theme.muted
        } else if self.state.checked {
            checked_color
        } else {
            fg
        };
        let glyph = if self.state.checked {
            self.style.checked_glyph
        } else {
            self.style.unchecked_glyph
        };
        buf.set_string(
            inner.x,
            y,
            format!("[{glyph}]"),
            Style::default().fg(box_color),
        );
        let content = Rect::new(inner.x + 4, y, inner.width.saturating_sub(4), 1);
        row(&mut self.children, content, Align::Left, 1, buf, cx, theme);
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        self.state.focused = if self.state.disabled {
            false
        } else {
            cx.is_focused()
        };
        if self.state.disabled {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Mouse(mouse) => {
                let is_click = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
                let inside = mouse.column >= self.state.area.x
                    && mouse.column < self.state.area.x.saturating_add(self.state.area.width)
                    && mouse.row >= self.state.area.y
                    && mouse.row < self.state.area.y.saturating_add(self.state.area.height);
                if is_click && inside {
                    self.toggle();
                    return Outcome::Consumed;
                }
                Outcome::Ignored
            }
            InputEvent::Key(key) => {
                if !self.state.focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.toggle();
                    return Outcome::Consumed;
                }
                Outcome::Ignored
            }
            _ => Outcome::Ignored,
        }
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum();
        let gaps = (self.children.len().saturating_sub(1)) as u16;
        Some((2 + 2 + measured + gaps, 1))
    }

    fn focusable(&self) -> bool {
        !self.state.disabled
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.toggle();
        true
    }
}
