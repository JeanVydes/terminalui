//! A category bar chart. The host owns the samples and labels; this view
//! owns where bars, grid and labels land. Values are treated as
//! non-negative; negatives clamp to zero.

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

const AXIS_WIDTH: u16 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BarChartState {
    pub area: Rect,
    pub focused: bool,
}

/// Per-instance overrides for a bar chart. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BarChartStyle {
    /// Bar fill color. Defaults to `theme.primary`.
    pub bar: Option<Color>,
    /// Grid dot color. Defaults to `theme.muted`.
    pub grid: Option<Color>,
    /// Axis label color. Defaults to `theme.muted`.
    pub label: Option<Color>,
}

/// Vertical bars over host-provided values. One x label per bar is drawn
/// in a row under the baseline when [`BarChart::with_labels`] sets them.
#[derive(Serialize)]
pub struct BarChart {
    pub state: BarChartState,
    pub style: BarChartStyle,
    /// Which glyph set paints the bars. Defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    /// Shading knobs for non-`Native` presentations.
    pub tuning: StyleTuning,
    /// Caption row, composed by the host.
    children: Vec<Node>,
    values: Vec<f64>,
    labels: Vec<String>,
}

impl BarChart {
    pub fn new() -> Self {
        Self {
            state: BarChartState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
            },
            style: BarChartStyle::default(),
            presentation: RenderStyle::default(),
            tuning: StyleTuning::default(),
            children: Vec::new(),
            values: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn with_style(mut self, style: BarChartStyle) -> Self {
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

    /// Compose a caption row above the plot (title, badges, ...).
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    /// Replace the values. Negative or non-finite entries clamp to zero.
    pub fn with_values(mut self, values: Vec<f64>) -> Self {
        self.values = values
            .into_iter()
            .map(|value| {
                if value.is_finite() && value > 0.0 {
                    value
                } else {
                    0.0
                }
            })
            .collect();
        self
    }

    /// One label per value, drawn under the baseline. Extra labels are
    /// ignored; missing labels render empty slots.
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }
}

impl Default for BarChart {
    fn default() -> Self {
        Self::new()
    }
}

impl View for BarChart {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "bar_chart"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "bars={}, presentation={}",
            self.values.len(),
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
        let label_row = u16::from(!self.labels.is_empty() && area.height >= 8);
        if self.values.is_empty() || area.height < 4 || area.width < 12 {
            return;
        }
        let bar_color = self.style.bar.unwrap_or(theme.primary);
        let grid_color = self.style.grid.unwrap_or(theme.muted);
        let label_color = self.style.label.unwrap_or(theme.muted);

        let plot_top = area.y + caption_height + 1;
        let labels_y = area.y + area.height - label_row;
        let axis_line_y = labels_y.saturating_sub(1);
        let plot_bottom = axis_line_y.saturating_sub(1);
        if plot_bottom < plot_top {
            return;
        }
        let plot_height = (plot_bottom - plot_top + 1) as f64;
        let plot_width = area.width.saturating_sub(AXIS_WIDTH + 1).max(1);
        let axis_x = area.x + plot_width + 1;
        let max = self.values.iter().copied().fold(0.0_f64, f64::max).max(1.0);
        let shaded = self.presentation != RenderStyle::Native;

        // Dotted gridlines with max/mid labels.
        for step in 0..=2 {
            let value = max * step as f64 / 2.0;
            let y =
                plot_bottom.saturating_sub(((value / max) * (plot_height - 1.0)).round() as u16);
            for x in (area.x..area.x + plot_width).step_by(3) {
                buf.set_string(x, y, "·", Style::default().fg(grid_color));
            }
            buf.set_stringn(
                axis_x + 2,
                y,
                format!("{value:.0}"),
                8,
                Style::default().fg(label_color),
            );
        }
        for y in plot_top..=plot_bottom {
            buf.set_string(axis_x, y, "│", Style::default().fg(grid_color));
        }
        for x in area.x..area.x + plot_width + 1 {
            buf.set_string(x, axis_line_y, "─", Style::default().fg(grid_color));
        }

        let count = self.values.len() as u16;
        let slot = (plot_width / count).max(1);
        let bar_width = slot.saturating_sub(1).max(1);
        for (index, value) in self.values.iter().enumerate() {
            let height = ((*value / max) * (plot_height - 1.0)).round().max(0.0) as u16;
            let x = area.x + index as u16 * slot;
            let top = plot_bottom.saturating_sub(height);
            for row in top..=plot_bottom {
                for column in 0..bar_width {
                    let intensity =
                        ((plot_bottom - row) as f32 / plot_height.max(1.0) as f32).clamp(0.0, 1.0);
                    let glyph = if shaded {
                        self.presentation
                            .shade(0.35 + 0.65 * intensity, &self.tuning)
                    } else {
                        "█"
                    };
                    buf.set_string(x + column, row, glyph, Style::default().fg(bar_color));
                }
            }
            if label_row > 0 {
                if let Some(label) = self.labels.get(index) {
                    buf.set_stringn(
                        x,
                        labels_y,
                        label,
                        slot as usize,
                        Style::default().fg(label_color),
                    );
                }
            }
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
