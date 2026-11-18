//! A single-value meter. The host hands over the target fraction (0..=1):
//! one row showing where the value sits between empty and full, with an
//! optional warn color past a threshold.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::layout::{row, Align};
use super::presentation::{RenderStyle, StyleTuning};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GaugeState {
    pub area: Rect,
}

/// Per-instance overrides for a gauge. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct GaugeStyle {
    /// Filled cells color. Defaults to `theme.primary`.
    pub fill: Option<Color>,
    /// Empty track color. Defaults to `theme.muted`.
    pub track: Option<Color>,
    /// Fill color once `warn_above` is reached. Defaults to `theme.warning`.
    pub warn: Option<Color>,
    /// Threshold in 0..=1 at which the fill turns warn-colored.
    pub warn_above: Option<f64>,
}

/// A horizontal bar gauge over `0.0..=1.0`. Compose a caption (label,
/// units) as children; they render in the row above the bar.
#[derive(Serialize)]
pub struct Gauge {
    /// Target fraction, clamped to 0..=1.
    pub value: f64,
    pub state: GaugeState,
    pub style: GaugeStyle,
    /// Which glyph set paints the bar. Defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    /// Shading knobs for non-`Native` presentations.
    pub tuning: StyleTuning,
    children: Vec<Node>,
}

impl Gauge {
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            state: GaugeState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: GaugeStyle::default(),
            presentation: RenderStyle::Native,
            tuning: StyleTuning::default(),
            children: Vec::new(),
        }
    }

    pub fn with_value(mut self, value: f64) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }

    pub fn with_style(mut self, style: GaugeStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    pub fn with_tuning(mut self, tuning: StyleTuning) -> Self {
        self.tuning = tuning;
        self
    }

    /// Compose a caption row above the bar (label, units, ...).
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl View for Gauge {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "gauge"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "value={:.2}, presentation={}",
            self.value,
            quoted(&format!("{:?}", self.presentation))
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((24, 1 + u16::from(!self.children.is_empty())))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let caption_height = u16::from(!self.children.is_empty());
        if caption_height > 0 {
            let caption = Rect::new(area.x, area.y, area.width, 1);
            row(&mut self.children, caption, Align::Left, 2, buf, cx, theme);
        }
        let bar_y = area.y + caption_height;
        if bar_y >= area.y + area.height {
            return;
        }
        let warn = self
            .style
            .warn_above
            .map(|threshold| self.value >= threshold)
            .unwrap_or(false);
        let fill = if warn {
            self.style.warn.unwrap_or(theme.warning)
        } else {
            self.style.fill.unwrap_or(theme.primary)
        };
        let track = self.style.track.unwrap_or(theme.muted);
        let readout = format!("{:>3}%", (self.value * 100.0).round() as u16);
        let readout_width = readout.chars().count() as u16;
        let bar_width = area.width.saturating_sub(readout_width + 1);
        let filled = ((bar_width as f64) * self.value).round() as u16;
        let shaded = self.presentation != RenderStyle::Native;
        // Native keeps the original solid look; shaded families ramp the
        // ink across the fill so the bar carries gradient detail.
        let filled_ratio = |index: u16| -> f32 {
            if filled == 0 {
                0.0
            } else {
                0.55 + 0.45 * (index as f32 / (filled.max(1) - 1).max(1) as f32)
            }
        };
        for x in 0..bar_width {
            let is_filled = x < filled;
            let (glyph, color) = match (is_filled, shaded) {
                (true, true) => (
                    self.presentation
                        .shade(filled_ratio(x).clamp(0.0, 1.0), &self.tuning),
                    fill,
                ),
                (true, false) => ("█", fill),
                (false, true) => (self.presentation.shade(0.15, &self.tuning), track),
                (false, false) => ("░", track),
            };
            buf.set_string(area.x + x, bar_y, glyph, Style::default().fg(color));
        }
        let readout_width = readout.chars().count();
        buf.set_stringn(
            area.x + bar_width + 1,
            bar_y,
            &readout,
            readout_width,
            Style::default().fg(theme.foreground),
        );
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
