//! A single-series line chart. The host owns the samples; this view owns
//! where the line, grid and labels land. Compose caption views (symbol,
//! badges) as children; they render in the row above the plot.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::axis::{format_value, nice_domain};
use super::layout::{row, Align};
use super::presentation::{RenderStyle, StyleTuning};

const MAX_HISTORY: usize = 240;
const AXIS_WIDTH: u16 = 8;
/// Ticks on a zero-based value axis.
const AXIS_DIVISIONS: usize = 10;

/// Public state of the chart: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineChartState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the chart would hold focus by the tree. Charts are display-only.
    pub focused: bool,
}

/// Per-instance overrides for a line chart. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LineChartStyle {
    /// Line and point color. Defaults to `theme.accent`.
    pub line: Option<Color>,
    /// Grid dot color. Defaults to `theme.muted`.
    pub grid: Option<Color>,
    /// Axis label color. Defaults to `theme.muted`.
    pub label: Option<Color>,
    /// Latest-value tag background. Defaults to `theme.accent`.
    pub latest: Option<Color>,
}

/// A compact line chart over a single `f64` series. Values are consumed in
/// order; keep feeding [`LineChart::push`] to watch a live value.
#[derive(Serialize)]
pub struct LineChart {
    /// Last rendered area, set during render.
    pub state: LineChartState,
    /// Per-instance overrides; defaults match the theme.
    pub style: LineChartStyle,
    /// Which glyph set paints the line. Defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    /// Shading knobs for non-`Native` presentations.
    pub tuning: StyleTuning,
    /// Caption row, composed by the host.
    children: Vec<Node>,
    series: Vec<f64>,
    domain: Option<(f64, f64)>,
}

impl LineChart {
    /// Creates an empty chart. Series starts empty; the domain auto-fits.
    pub fn new() -> Self {
        Self {
            state: LineChartState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
            },
            style: LineChartStyle::default(),
            presentation: RenderStyle::default(),
            tuning: StyleTuning::default(),
            children: Vec::new(),
            series: Vec::new(),
            domain: None,
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: LineChartStyle) -> Self {
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

    /// Compose a caption row above the plot (symbol, badges, interval, ...).
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    /// Fix the y range. Auto-fits the visible series when unset.
    pub fn with_domain(mut self, low: f64, high: f64) -> Self {
        if low.is_finite() && high.is_finite() && high > low {
            self.domain = Some((low, high));
        }
        self
    }

    /// Replace the series entirely; non-finite values dropped, truncated to `MAX_HISTORY`.
    pub fn set_series(&mut self, series: Vec<f64>) {
        let mut series: Vec<_> = series
            .into_iter()
            .filter(|value| value.is_finite())
            .collect();
        if series.len() > MAX_HISTORY {
            series.drain(..series.len() - MAX_HISTORY);
        }
        self.series = series;
    }

    /// Append one value, dropping the oldest when the history is full.
    pub fn push(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        self.series.push(value);
        if self.series.len() > MAX_HISTORY {
            self.series.remove(0);
        }
    }

    /// Number of samples currently held.
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// `true` when there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    fn domain(&self) -> (f64, f64) {
        if let Some(domain) = self.domain {
            return domain;
        }
        let low = self.series.iter().copied().fold(f64::INFINITY, f64::min);
        let high = self
            .series
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        if high == low {
            let padding = high.abs().mul_add(0.1, 0.1);
            return (low - padding, high + padding);
        }
        nice_domain(low, high, AXIS_DIVISIONS)
    }
}

impl Default for LineChart {
    fn default() -> Self {
        Self::new()
    }
}

impl View for LineChart {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "line_chart"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "points={}, presentation={}",
            self.series.len(),
            quoted(&format!("{:?}", self.presentation))
        ))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.state.area = area;
        buf.set_style(area, Style::default().bg(theme.background));
        let caption_height = u16::from(!self.children.is_empty());
        if caption_height > 0 {
            let caption = Rect::new(area.x, area.y, area.width, 1);
            row(&mut self.children, caption, Align::Left, 2, buf, cx, theme);
        }
        if area.width < 12 || area.height < 5 || self.series.is_empty() {
            return;
        }
        let line_color = self.style.line.unwrap_or(theme.accent);
        let grid_color = self.style.grid.unwrap_or(theme.muted);
        let label_color = self.style.label.unwrap_or(theme.muted);

        let plot_top = area.y + caption_height + 1;
        let axis_y = area.y + area.height.saturating_sub(1);
        let axis_line_y = axis_y.saturating_sub(1);
        if axis_line_y < plot_top {
            return;
        }
        let plot_bottom = axis_line_y.saturating_sub(1);
        let plot_height = (plot_bottom - plot_top + 1).max(1) as f64;
        let plot_width = area.width.saturating_sub(AXIS_WIDTH + 1).max(1);
        let axis_x = area.x + plot_width + 1;
        let (low, high) = self.domain();
        let range = (high - low).max(f64::EPSILON);
        let to_y = |value: f64| -> u16 {
            let normalized = ((value - low) / range).clamp(0.0, 1.0);
            plot_top + ((1.0 - normalized) * (plot_height - 1.0)).round() as u16
        };

        // Gridline count follows the available plot height.
        let divisions = AXIS_DIVISIONS
            .min(plot_bottom.saturating_sub(plot_top) as usize)
            .max(1);
        let axis = (0..=divisions)
            .map(|step| low + (high - low) * step as f64 / divisions as f64)
            .collect::<Vec<_>>();
        for value in &axis {
            let y = to_y(*value).clamp(plot_top, plot_bottom);
            for x in (area.x..area.x + plot_width).step_by(3) {
                buf.set_string(x, y, "·", Style::default().fg(grid_color));
            }
        }
        // Highest value stays at the top, lowest at the bottom.
        let span = plot_bottom.saturating_sub(plot_top);
        for index in 0..=divisions {
            let value = axis[divisions - index];
            let y = plot_top + (index as u16 * span / divisions as u16);
            buf.set_stringn(
                axis_x + 2,
                y,
                format_value(value),
                4,
                Style::default().fg(label_color),
            );
        }
        for y in plot_top..=plot_bottom {
            buf.set_string(axis_x, y, "│", Style::default().fg(grid_color));
        }
        for x in area.x..area.x + plot_width + 1 {
            buf.set_string(x, axis_line_y, "─", Style::default().fg(grid_color));
        }

        // Sampled polyline: one point per column.
        let shaded = self.presentation != RenderStyle::Native;
        let sample_count = self.series.len();
        let mut previous_y: Option<u16> = None;
        for column in 0..plot_width {
            let start = column as usize * sample_count / plot_width as usize;
            let end = ((column as usize + 1) * sample_count / plot_width as usize)
                .max(start + 1)
                .min(sample_count);
            let slice = &self.series[start..end];
            let minimum = to_y(slice.iter().copied().fold(f64::INFINITY, f64::min));
            let maximum = to_y(slice.iter().copied().fold(f64::NEG_INFINITY, f64::max));
            let y = to_y(*slice.last().unwrap_or(&low));
            if let Some(previous_y) = previous_y {
                connect(buf, area.x + column, previous_y, y, line_color);
            }
            for row in minimum.min(maximum)..=minimum.max(maximum) {
                buf.set_string(area.x + column, row, "│", Style::default().fg(line_color));
            }
            let glyph = if shaded {
                let intensity = ((slice[slice.len() - 1] - low) / range).clamp(0.0, 1.0) as f32;
                self.presentation.shade(intensity, &self.tuning)
            } else {
                "●"
            };
            buf.set_string(area.x + column, y, glyph, Style::default().fg(line_color));
            previous_y = Some(y);
        }

        // Latest value tag on the right axis.
        let latest = *self.series.last().unwrap_or(&low);
        let y = to_y(latest).clamp(plot_top, plot_bottom);
        let tag = format!(" {} ", format_value(latest));
        buf.set_stringn(
            axis_x + 2,
            y,
            tag,
            9,
            Style::default()
                .fg(theme.background)
                .bg(self.style.latest.unwrap_or(line_color)),
        );
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

/// Stepped connector between the previous point row and `y`, drawn in the
/// new column so the step reads as a staircase.
fn connect(buf: &mut Buffer, column: u16, previous_y: u16, y: u16, color: Color) {
    let style = Style::default().fg(color);
    let (from, to) = if previous_y <= y {
        (previous_y + 1, y + 1)
    } else {
        (y, previous_y)
    };
    for row in from..to {
        buf.set_string(column, row, "│", style);
    }
}
