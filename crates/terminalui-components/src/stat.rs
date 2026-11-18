//! A single big number with an optional delta. Display-only: the host owns
//! the strings, this view owns the geometry and the trend coloring.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

/// Meaning of the delta arrow: is the direction good (`Positive`, success
/// color) or bad (`Negative`, destructive color), or neither (`Neutral`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StatTone {
    /// Neither good nor bad; neutral muted rendering.
    #[default]
    Neutral,
    /// The direction is good; success color.
    Positive,
    /// The direction is bad; destructive color.
    Negative,
}

/// A delta chip: text plus its tone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatDelta {
    /// Delta text, e.g. `+2 services`.
    pub text: String,
    /// Meaning of the arrow direction.
    pub tone: StatTone,
}

/// Public state of the tile: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatState {
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// Per-instance overrides for a stat tile. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StatStyle {
    /// Label color. Defaults to `theme.muted`.
    pub label: Option<Color>,
    /// Value color. Defaults to `theme.foreground`.
    pub value: Option<Color>,
    /// Delta text color when positive. Defaults to `theme.success`.
    pub positive: Option<Color>,
    /// Delta text color when negative. Defaults to `theme.destructive`.
    pub negative: Option<Color>,
    /// Delta text color when neutral. Defaults to `theme.muted`.
    pub neutral: Option<Color>,
}

/// A stat tile: a small muted label above a bold value, with an optional
/// delta arrow and text to the right of the value.
#[derive(Serialize)]
pub struct Stat {
    /// The muted caption above the value.
    pub label: String,
    /// The big number or string.
    pub value: String,
    /// Optional delta chip shown to the right of the value.
    pub delta: Option<StatDelta>,
    /// Public state: the last rendered area.
    pub state: StatState,
    /// Per-instance overrides; defaults match the theme.
    pub style: StatStyle,
}

impl Stat {
    /// Creates a stat tile with the given label and value.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            delta: None,
            state: StatState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: StatStyle::default(),
        }
    }

    /// Attach a delta, e.g. `("+2 services", StatTone::Positive)`.
    pub fn with_delta(mut self, text: impl Into<String>, tone: StatTone) -> Self {
        self.delta = Some(StatDelta {
            text: text.into(),
            tone,
        });
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: StatStyle) -> Self {
        self.style = style;
        self
    }
}

impl View for Stat {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "stat"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("value={}", quoted(&self.value)))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let label = self.label.chars().count();
        let delta = self
            .delta
            .as_ref()
            .map(|delta| delta.text.chars().count() + 2)
            .unwrap_or(0);
        let width = self.value.chars().count() + delta + 1;
        Some((label.max(width) as u16, 2))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let label_color = self.style.label.unwrap_or(theme.muted);
        let value_color = self.style.value.unwrap_or(theme.foreground);
        buf.set_stringn(
            area.x,
            area.y,
            &self.label,
            area.width as usize,
            Style::default().fg(label_color),
        );
        if area.height < 2 {
            return;
        }
        let value_y = area.y + 1;
        let value_style = Style::default()
            .fg(value_color)
            .add_modifier(Modifier::BOLD);
        buf.set_stringn(
            area.x,
            value_y,
            &self.value,
            area.width as usize,
            value_style,
        );
        if let Some(delta) = &self.delta {
            let (arrow, color) = match delta.tone {
                StatTone::Positive => ("▲", self.style.positive.unwrap_or(theme.success)),
                StatTone::Negative => ("▼", self.style.negative.unwrap_or(theme.destructive)),
                StatTone::Neutral => ("•", self.style.neutral.unwrap_or(theme.muted)),
            };
            let text_len = delta.text.chars().count();
            let value_len = self.value.chars().count() as u16;
            let delta_x = area.x + value_len + 1;
            if delta_x < area.x + area.width {
                let remaining = area.width.saturating_sub(delta_x - area.x + 1) as usize;
                buf.set_stringn(
                    delta_x,
                    value_y,
                    format!("{arrow} {}", delta.text),
                    remaining.max(1).min(text_len + 2),
                    Style::default().fg(color),
                );
            }
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
