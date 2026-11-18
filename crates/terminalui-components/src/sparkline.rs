//! A one-row inline chart. No axes, no labels: just the shape of a series,
//! drawn with block glyphs in a single cell row.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::presentation::RenderStyle;

const BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const ASCII_RAMP: [&str; 8] = [".", ":", "-", "=", "+", "*", "#", "@"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SparklineState {
    pub area: Rect,
}

/// Per-instance overrides for a sparkline. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SparklineStyle {
    /// Glyph color. Defaults to `theme.accent`.
    pub line: Option<Color>,
}

/// A tiny inline chart for table cells, stats and captions. The host feeds
/// values in order; the last `width` of them are drawn.
#[derive(Serialize)]
pub struct Sparkline {
    pub state: SparklineState,
    pub style: SparklineStyle,
    /// `Native` draws block glyphs; `Ascii` draws an ASCII ramp.
    pub presentation: RenderStyle,
    series: Vec<f64>,
}

impl Sparkline {
    pub fn new(series: Vec<f64>) -> Self {
        Self {
            state: SparklineState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: SparklineStyle::default(),
            presentation: RenderStyle::Native,
            series: series
                .into_iter()
                .filter(|value| value.is_finite())
                .collect(),
        }
    }

    pub fn with_style(mut self, style: SparklineStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    pub fn len(&self) -> usize {
        self.series.len()
    }

    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    fn glyph(presentation: RenderStyle, value: f64, low: f64, range: f64) -> &'static str {
        let ramp = match presentation {
            RenderStyle::Ascii | RenderStyle::DenseAscii => &ASCII_RAMP,
            _ => &BLOCKS,
        };
        let index = ((value - low) / range * (BLOCKS.len() - 1) as f64)
            .round()
            .clamp(0.0, (BLOCKS.len() - 1) as f64) as usize;
        ramp[index]
    }
}

impl Default for Sparkline {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for Sparkline {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "sparkline"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("points={}", self.series.len()))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((self.series.len().min(u16::MAX as usize) as u16, 1))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 || self.series.is_empty() {
            return;
        }
        let color = self.style.line.unwrap_or(theme.accent);
        let start = self.series.len().saturating_sub(area.width as usize);
        let visible = &self.series[start..];
        let low = visible.iter().copied().fold(f64::INFINITY, f64::min);
        let high = visible.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = (high - low).abs().max(f64::EPSILON);
        let y = area.y;
        for (column, value) in visible.iter().enumerate() {
            buf.set_string(
                area.x + column as u16,
                y,
                Self::glyph(self.presentation, *value, low, range),
                Style::default().fg(color),
            );
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
