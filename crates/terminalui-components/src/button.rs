use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::chrome::{draw_box, Border, BorderSet, Chrome, Padding, Radius};
use super::layout::{row, Align};
use super::spinner::Spinner;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Destructive,
    Ghost,
    Outline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ButtonState {
    pub area: Rect,
    pub focused: bool,
    pub disabled: bool,
    pub loading: bool,
    /// Set when the button is activated (key, click or programmatic press);
    /// drained by the host with [`Button::take_pressed`].
    pub pressed: bool,
    /// True while the primary mouse button is held down on the button.
    /// Purely visual (pressed look); activation is reported on release of
    /// the click as usual.
    pub active: bool,
}

/// Per-instance overrides for a button. Defaults match the theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ButtonStyle {
    /// Fill background. `None` = the variant decides (Ghost/Outline: transparent).
    pub background: Option<Color>,
    /// Focus ring / border highlight color. Defaults to `theme.ring`.
    pub focused_ring: Option<Color>,
    /// Horizontal padding in cells. Default 1.
    pub padding_x: Option<u16>,
    /// Border of the button. `None` = no border (filled button).
    pub border: Option<Border>,
    /// Corner radius. Rounded corners when set.
    pub radius: Radius,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            background: None,
            focused_ring: None,
            padding_x: None,
            border: None,
            radius: Radius::None,
        }
    }
}

const SPINNER_SLOT: &str = "__spinner";

#[derive(Serialize)]
pub struct Button {
    pub variant: ButtonVariant,
    pub state: ButtonState,
    pub style: ButtonStyle,
    children: Vec<Node>,
}

impl Button {
    /// Convenience constructor: the content is a single [`Text`] view.
    pub fn new(variant: ButtonVariant, label: impl Into<String>) -> Self {
        Self::with_children(variant, vec![node("label", Text::new(label))])
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.state.loading = loading;
        if loading {
            self.state.active = false;
        }
        match loading {
            true => {
                let has_spinner = self
                    .children
                    .iter()
                    .any(|child| child.view.as_any_ref().is::<Spinner>());
                if !has_spinner {
                    self.children.insert(0, node(SPINNER_SLOT, Spinner::new()));
                }
            }
            false => {
                self.children
                    .retain(|child| !child.view.as_any_ref().is::<Spinner>());
            }
        }
    }
}

impl Button {
    /// Compose a button from arbitrary views.
    pub fn with_children(variant: ButtonVariant, children: Vec<Node>) -> Self {
        Self {
            variant,
            state: ButtonState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                disabled: false,
                loading: false,
                pressed: false,
                active: false,
            },
            style: ButtonStyle::default(),
            children,
        }
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.state.disabled = disabled;
        if disabled {
            self.state.active = false;
        }
    }

    /// Programmatic activation (e.g. `tree.press(id)`).
    pub fn press(&mut self) -> bool {
        let ok = !self.state.disabled && !self.state.loading;
        if ok {
            self.state.pressed = true;
        }
        ok
    }

    /// Drains the activation flag. Returns true once per activation.
    pub fn take_pressed(&mut self) -> bool {
        std::mem::take(&mut self.state.pressed)
    }

    fn hit(&self, column: u16, row: u16) -> bool {
        let area = self.state.area;
        column >= area.x
            && column < area.x.saturating_add(area.width)
            && row >= area.y
            && row < area.y.saturating_add(area.height)
    }

    fn focusable_now(&self) -> bool {
        !self.state.disabled && !self.state.loading
    }

    fn background(&self, theme: &Theme) -> Color {
        match self.variant {
            ButtonVariant::Primary => theme.primary,
            ButtonVariant::Secondary => theme.secondary,
            ButtonVariant::Destructive => theme.destructive,
            ButtonVariant::Ghost => theme.background,
            ButtonVariant::Outline => theme.background,
        }
    }

    /// Hover/active shade of a filled variant: lighten RGB fills so the
    /// emphasized state reads as the same hue, brighter. Non-RGB fills fall
    /// back to the accent color, which contrasts in every shipped palette.
    fn emphasized_fill(fill: Color, accent: Color) -> Color {
        match fill {
            Color::Rgb(red, green, blue) => Color::Rgb(
                red.saturating_add(38),
                green.saturating_add(38),
                blue.saturating_add(38),
            ),
            _ => accent,
        }
    }

    fn content_width(&self) -> u16 {
        let padding = self.style.padding_x.unwrap_or(1);
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum::<u16>()
            + self.children.len().saturating_sub(1) as u16;
        let border = u16::from(self.style.border.is_some()) * 2;
        measured + padding * 2 + border
    }
}

impl View for Button {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "button"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "pressed={}, disabled={}, loading={}",
            self.state.pressed, self.state.disabled, self.state.loading
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((self.content_width(), 1))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = if self.focusable_now() {
            cx.is_focused()
        } else {
            false
        };
        self.state.area = area;
        let ring = self.style.focused_ring.unwrap_or(theme.ring);
        let emphasized = self.state.focused || self.state.active;
        let is_outline = matches!(self.variant, ButtonVariant::Outline);
        let is_ghost = matches!(self.variant, ButtonVariant::Ghost);
        let transparent_fill = is_ghost || is_outline;
        let emphasized_filled =
            !transparent_fill && emphasized && self.style.border.is_none() && !self.state.disabled;
        let background = if transparent_fill {
            self.style.background
        } else if emphasized_filled {
            let base = self.style.background.unwrap_or(self.background(theme));
            Some(Self::emphasized_fill(base, theme.accent))
        } else {
            match self.style.background {
                Some(color) => Some(color),
                None if self.state.disabled => Some(theme.muted),
                None => Some(self.background(theme)),
            }
        };
        let border = if let Some(host) = self.style.border {
            let color = match host.color {
                Some(color) => Some(if self.state.disabled {
                    theme.muted
                } else if emphasized {
                    ring
                } else {
                    color
                }),
                None => Some(if self.state.disabled {
                    theme.muted
                } else if emphasized {
                    ring
                } else {
                    theme.border
                }),
            };
            Some(Border { color, ..host })
        } else if is_outline {
            let color = if self.state.disabled {
                theme.muted
            } else if emphasized {
                ring
            } else {
                theme.border
            };
            Some(Border {
                set: BorderSet::Plain,
                color: Some(color),
                radius: self.style.radius,
            })
        } else if is_ghost && emphasized {
            Some(Border {
                set: BorderSet::Plain,
                color: Some(ring),
                radius: self.style.radius,
            })
        } else {
            None
        };
        let chrome = Chrome {
            background,
            border,
            radius: self.style.radius,
            padding: Padding {
                horizontal: self.style.padding_x.unwrap_or(1),
                vertical: 0,
            },
            border_gradient: None,
            fill_gradient: None,
        };
        let width = self.content_width().min(area.width);
        let x = area.x + area.width.saturating_sub(width) / 2;
        let rect = Rect::new(x, area.y, width.max(1), 1);
        let inner = draw_box(buf, rect, &chrome, theme);
        row(&mut self.children, inner, Align::Center, 1, buf, cx, theme);
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        self.state.focused = if self.focusable_now() {
            cx.is_focused()
        } else {
            false
        };
        if self.state.disabled || self.state.loading {
            self.state.active = false;
            return Outcome::Ignored;
        }
        if let InputEvent::Mouse(mouse) = event {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.hit(mouse.column, mouse.row) {
                        self.state.pressed = true;
                        self.state.active = true;
                        return Outcome::Consumed;
                    }
                }
                MouseEventKind::Up(MouseButton::Left) if self.state.active => {
                    self.state.active = false;
                    return Outcome::Consumed;
                }
                MouseEventKind::Up(MouseButton::Left) => {}
                _ => {}
            }
            return Outcome::Ignored;
        }
        if !self.state.focused {
            return Outcome::Ignored;
        }
        let InputEvent::Key(key) = event else {
            return Outcome::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        if key.code == KeyCode::Enter || key.code == KeyCode::Char(' ') {
            self.state.pressed = true;
            return Outcome::Consumed;
        }
        Outcome::Ignored
    }

    fn focusable(&self) -> bool {
        self.focusable_now()
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.press()
    }
}
