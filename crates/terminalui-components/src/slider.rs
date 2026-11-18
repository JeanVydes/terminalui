//! A single-value horizontal slider.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::presentation::{RenderStyle, StyleTuning};

/// Public state of the slider: area, focus and disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SliderState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the slider would hold focus by the tree.
    pub focused: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
}

/// Per-instance overrides for a slider. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SliderStyle {
    /// Filled track color. Defaults to `theme.primary`.
    pub filled: Option<Color>,
    /// Track color. Defaults to `theme.muted`.
    pub track: Option<Color>,
    /// Thumb glyph color. Defaults to `theme.accent`.
    pub thumb: Option<Color>,
    /// Filled track color when disabled. Defaults to `theme.muted`.
    pub disabled_fill: Option<Color>,
}

/// A horizontal slider over `[min, max]` in `step` increments. Mouse click
/// and drag set the value from the pointer position; when focused, Left and
/// Right step by `step` (Home/End jump to the ends, PageUp/PageDown step by
/// ten). The host reads [`Slider::value`] every frame; there is no outbox.
#[derive(Serialize)]
pub struct Slider {
    /// Public state: area, focus and disabled.
    pub state: SliderState,
    /// Per-instance overrides; defaults match the theme.
    pub style: SliderStyle,
    /// Which glyph set draws the track. Defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    /// Shading knobs for non-`Native` track presentations.
    pub tuning: StyleTuning,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
}

impl Slider {
    /// Creates a slider over `[min, max]` in `step` increments, starting at `min`.
    pub fn new(min: f64, max: f64, step: f64) -> Self {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        let step = if step.is_finite() && step > 0.0 {
            step
        } else {
            (max - min).max(1.0) / 100.0
        };
        Self {
            state: SliderState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                disabled: false,
            },
            style: SliderStyle::default(),
            presentation: RenderStyle::Native,
            tuning: StyleTuning::default(),
            min,
            max,
            step,
            value: min,
        }
    }

    /// The current value, clamped to `[min, max]` and snapped to `step`.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Normalized position in `0.0..=1.0`.
    pub fn fraction(&self) -> f64 {
        let span = self.max - self.min;
        if span <= 0.0 {
            0.0
        } else {
            ((self.value - self.min) / span).clamp(0.0, 1.0)
        }
    }

    /// Snap `value` to `[min, max]` and the step grid. Returns `true` when the value changed.
    pub fn set_value(&mut self, value: f64) -> bool {
        let snapped = if value.is_finite() {
            let steps = ((value - self.min) / self.step).round();
            (self.min + steps * self.step).clamp(self.min, self.max)
        } else {
            self.min
        };
        if snapped == self.value {
            return false;
        }
        self.value = snapped;
        true
    }

    /// Builder shorthand: set the value; snapped to the step grid.
    pub fn with_value(mut self, value: f64) -> Self {
        self.set_value(value);
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder shorthand: pick the glyph set.
    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    /// Builder shorthand: set the shading knobs for non-`Native` presentations.
    pub fn with_tuning(mut self, tuning: StyleTuning) -> Self {
        self.tuning = tuning;
        self
    }

    /// Builder shorthand: disable or enable interaction.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state.disabled = disabled;
    }

    fn value_at(&self, column: u16) -> f64 {
        let area = self.state.area;
        if area.width <= 1 {
            return self.max;
        }
        let offset = column.saturating_sub(area.x).min(area.width - 1) as f64;
        self.min + (offset / f64::from(area.width - 1)) * (self.max - self.min)
    }

    fn thumb_column(&self) -> u16 {
        let area = self.state.area;
        if area.width <= 1 {
            return area.x;
        }
        area.x + ((self.fraction() * f64::from(area.width - 1)).round() as u16).min(area.width - 1)
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new(0.0, 100.0, 1.0)
    }
}

impl View for Slider {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "slider"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "value={}, min={}, max={}, disabled={}",
            self.value, self.min, self.max, self.state.disabled
        ))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        self.state.focused = if self.state.disabled {
            false
        } else {
            cx.is_focused()
        };
        let fill = if self.state.disabled {
            self.style.disabled_fill.unwrap_or(theme.muted)
        } else {
            self.style.filled.unwrap_or(theme.primary)
        };
        let track = self.style.track.unwrap_or(theme.muted);
        let thumb = self.style.thumb.unwrap_or(theme.accent);
        let thumb_x = self.thumb_column();
        let full = self.presentation.filled();
        let empty = self.presentation.empty();
        let shaded = self.presentation != RenderStyle::Native;
        let tuning = self.tuning;
        for offset in 0..area.width {
            let x = area.x + offset;
            if x == thumb_x {
                buf.set_string(x, area.y, "●", Style::default().fg(thumb));
                continue;
            }
            let before = x < thumb_x;
            if shaded {
                let glyph = if before {
                    self.presentation.shade(1.0, &tuning)
                } else {
                    empty
                };
                let style = if before {
                    Style::default().fg(fill)
                } else {
                    Style::default().fg(track)
                };
                buf.set_string(x, area.y, glyph, style);
            } else {
                let style = if before {
                    Style::default().fg(fill).bg(fill)
                } else {
                    Style::default().fg(track).bg(track)
                };
                buf.set_string(x, area.y, if before { full } else { empty }, style);
            }
        }
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
        if let InputEvent::Mouse(mouse) = event {
            let inside = mouse.column >= self.state.area.x
                && mouse.column < self.state.area.x.saturating_add(self.state.area.width)
                && mouse.row >= self.state.area.y
                && mouse.row < self.state.area.y.saturating_add(self.state.area.height);
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) if inside => {
                    let value = self.value_at(mouse.column);
                    self.set_value(value);
                    return Outcome::Consumed;
                }
                MouseEventKind::Drag(MouseButton::Left)
                    if mouse.column >= self.state.area.x
                        && mouse.column
                            < self.state.area.x.saturating_add(self.state.area.width) =>
                {
                    let value = self.value_at(mouse.column);
                    self.set_value(value);
                    return Outcome::Consumed;
                }
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
        match key.code {
            KeyCode::Left | KeyCode::Down => {
                self.set_value(self.value - self.step);
                Outcome::Consumed
            }
            KeyCode::Right | KeyCode::Up => {
                self.set_value(self.value + self.step);
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.set_value(self.value + self.step * 10.0);
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.set_value(self.value - self.step * 10.0);
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.set_value(self.min);
                Outcome::Consumed
            }
            KeyCode::End => {
                self.set_value(self.max);
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        !self.state.disabled
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.set_value(self.value + self.step);
        true
    }
}
