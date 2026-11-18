//! An on/off switch rendered as a sliding knob in a track. Children compose
//! the label; the switch owns the track and the knob.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

/// Public state of the switch: area, on, focus and disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwitchState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the switch is currently on.
    pub on: bool,
    /// Whether the switch would hold focus by the tree.
    pub focused: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
}

/// Per-instance overrides for a switch. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SwitchStyle {
    /// Filled track color when on. Defaults to `theme.success`.
    pub on: Option<Color>,
    /// Track color when off. Defaults to `theme.muted`.
    pub off: Option<Color>,
    /// Knob color. Defaults to `theme.foreground`.
    pub knob: Option<Color>,
}

/// A toggling switch. Label is composed: pass arbitrary views to
/// [`Switch::new`]; the common case is a single [`Text`] view.
#[derive(Serialize)]
pub struct Switch {
    /// Public state: area, on, focus and disabled.
    pub state: SwitchState,
    /// Per-instance overrides; defaults match the theme.
    pub style: SwitchStyle,
    children: Vec<Node>,
}

impl Switch {
    /// Convenience constructor: the label is a single [`Text`] view.
    pub fn with_label(label: impl Into<String>) -> Self {
        Self::new(vec![node("label", Text::new(label))])
    }

    /// Compose a switch from arbitrary views rendered after the track.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            state: SwitchState {
                area: Rect::new(0, 0, 0, 0),
                on: false,
                focused: false,
                disabled: false,
            },
            style: SwitchStyle::default(),
            children,
        }
    }

    /// Builder shorthand: set whether the switch starts on.
    pub fn with_on(mut self, on: bool) -> Self {
        self.state.on = on;
        self
    }

    /// Builder shorthand: disable or enable interaction.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: SwitchStyle) -> Self {
        self.style = style;
        self
    }

    /// Toggle the switch. Returns the new state.
    pub fn toggle(&mut self) -> bool {
        self.state.on = !self.state.on;
        self.state.on
    }

    /// Whether the switch is on.
    pub fn on(&self) -> bool {
        self.state.on
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for Switch {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "switch"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "on={}, disabled={}",
            self.state.on, self.state.disabled
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let mut width = 0u16;
        for child in &self.children {
            width = width.max(child.view.measure().map(|(w, _)| w).unwrap_or(0));
        }
        Some((8 + width, 1))
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
        let y = area.y;
        let muted = theme.muted;
        let track = if self.state.disabled {
            muted
        } else if self.state.on {
            self.style.on.unwrap_or(theme.success)
        } else {
            self.style.off.unwrap_or(muted)
        };
        let knob = if self.state.disabled {
            muted
        } else if self.state.focused {
            theme.accent
        } else {
            self.style.knob.unwrap_or(theme.foreground)
        };
        const TRACK_WIDTH: u16 = 6;
        let track_style = Style::default().fg(track);
        let knob_style = Style::default().fg(knob);
        for x in 0..TRACK_WIDTH {
            buf.set_string(area.x + x, y, "─", track_style);
        }
        let knob_x = if self.state.on {
            area.x + TRACK_WIDTH - 1
        } else {
            area.x
        };
        buf.set_string(knob_x, y, "●", knob_style);
        let content = Rect::new(
            area.x + TRACK_WIDTH + 2,
            y,
            area.width.saturating_sub(TRACK_WIDTH + 2),
            1,
        );
        for child in &mut self.children {
            child.view.render(content, buf, cx, theme);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        self.state.focused = !self.state.disabled && cx.is_focused();
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

    fn focusable(&self) -> bool {
        !self.state.disabled
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        if self.state.disabled {
            return false;
        }
        self.toggle();
        true
    }
}
