//! Horizontal box-and-whisker plots for pre-aggregated distributions.
//!
//! Rows are pure statistics supplied by the host. Labels are composed views
//! rendered in the left gutter, so a host can use a `Text`, a badge or any
//! other component as a row label. The component never reads time or random
//! state: animation is the host replacing the rows.

use std::any::Any;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::presentation::{RenderStyle, StyleTuning};

const ROW_HEIGHT: u16 = 3;
const AXIS_HEIGHT: u16 = 2;
const VALUE_COLUMN: u16 = 9;

/// One horizontal box: whisker range, quartiles, median and optional
/// outliers. Values are kept ordered on construction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoxPlotRow {
    pub low: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub high: f64,
    pub outliers: Vec<f64>,
    /// Box, whisker and cap ink. `None` uses the plot band color.
    pub color: Option<Color>,
}

impl BoxPlotRow {
    pub fn new(low: f64, q1: f64, median: f64, q3: f64, high: f64) -> Self {
        let mut row = Self {
            low,
            q1,
            median,
            q3,
            high,
            outliers: Vec::new(),
            color: None,
        };
        row.sanitize();
        row
    }

    pub fn with_outliers(mut self, outliers: Vec<f64>) -> Self {
        self.outliers = outliers
            .into_iter()
            .filter(|value| value.is_finite())
            .collect();
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    fn sanitize(&mut self) {
        let finite = |value: f64| if value.is_finite() { value } else { 0.0 };
        let low = finite(self.low).min(finite(self.high));
        let high = finite(self.low).max(finite(self.high));
        let mut q1 = finite(self.q1).clamp(low, high);
        let mut q3 = finite(self.q3).clamp(low, high);
        if q1 > q3 {
            std::mem::swap(&mut q1, &mut q3);
        }
        self.low = low;
        self.high = high;
        self.q1 = q1;
        self.q3 = q3;
        self.median = finite(self.median).clamp(q1, q3);
        self.outliers.retain(|value| value.is_finite());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct BoxPlotState {
    pub area: Rect,
}

/// Per-instance overrides for a box plot. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BoxPlotStyle {
    /// Axis, grid and tick color. Defaults to `theme.muted`.
    pub axis: Option<Color>,
    /// Default box and whisker color. Defaults to `theme.primary`.
    pub band: Option<Color>,
    /// Median rail color. Defaults to `theme.foreground`.
    pub median: Option<Color>,
    /// Whisker and cap color. Defaults to the row color.
    pub whisker: Option<Color>,
    /// Outlier dot color. Defaults to `theme.warning`.
    pub outlier: Option<Color>,
    /// `p50` readout color. Defaults to `theme.foreground`.
    pub value: Option<Color>,
}

/// A horizontal box plot. The host owns the samples; this view owns where
/// the boxes, whiskers, medians, outliers, grid and labels land.
#[derive(Serialize)]
pub struct BoxPlot {
    pub state: BoxPlotState,
    pub style: BoxPlotStyle,
    pub presentation: RenderStyle,
    pub tuning: StyleTuning,
    rows: Vec<BoxPlotRow>,
    /// One composed view per row, serialized as `children` for uniformity.
    #[serde(rename = "children")]
    labels: Vec<Node>,
    domain: Option<(f64, f64)>,
    unit: Option<&'static str>,
}

impl BoxPlot {
    pub fn new() -> Self {
        Self {
            state: BoxPlotState::default(),
            style: BoxPlotStyle::default(),
            presentation: RenderStyle::default(),
            tuning: StyleTuning::default(),
            rows: Vec::new(),
            labels: Vec::new(),
            domain: None,
            unit: None,
        }
    }

    pub fn with_rows(mut self, rows: Vec<BoxPlotRow>) -> Self {
        self.set_rows(rows);
        self
    }

    /// One label view per row, rendered in the left gutter.
    pub fn with_labels(mut self, labels: Vec<Node>) -> Self {
        self.labels = labels;
        self
    }

    /// Fix the horizontal domain. Auto-fits the rows when unset.
    pub fn with_domain(mut self, low: f64, high: f64) -> Self {
        if low.is_finite() && high.is_finite() && high > low {
            self.domain = Some((low, high));
        }
        self
    }

    /// Unit suffix drawn at the end of the axis, e.g. `"ms"`.
    pub fn with_unit(mut self, unit: &'static str) -> Self {
        self.unit = Some(unit);
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

    pub fn with_style(mut self, style: BoxPlotStyle) -> Self {
        self.style = style;
        self
    }

    pub fn set_rows(&mut self, rows: Vec<BoxPlotRow>) {
        self.rows = rows;
        for row in &mut self.rows {
            row.sanitize();
        }
    }

    pub fn rows(&self) -> &[BoxPlotRow] {
        &self.rows
    }

    /// Effective horizontal domain, explicit or fitted to the rows.
    pub fn domain(&self) -> (f64, f64) {
        if let Some(domain) = self.domain {
            return domain;
        }
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for row in &self.rows {
            low = low.min(row.low);
            high = high.max(row.high);
            for outlier in &row.outliers {
                low = low.min(*outlier);
                high = high.max(*outlier);
            }
        }
        if !low.is_finite() || !high.is_finite() || high <= low {
            return (0.0, 1.0);
        }
        let padding = (high - low) * 0.04;
        (low - padding, high + padding)
    }

    fn geometry(&self, area: Rect) -> Option<PlotGeometry> {
        let label_width = self
            .labels
            .iter()
            .filter_map(|label| label.view.measure().map(|(width, _)| width))
            .max()
            .unwrap_or(0)
            .min(area.width / 3);
        let label_width = if label_width > 0 { label_width + 2 } else { 0 };
        let plot_width = area.width.saturating_sub(label_width + VALUE_COLUMN + 1);
        let body_height = area.height.saturating_sub(AXIS_HEIGHT);
        if plot_width < 8 || body_height == 0 || self.rows.is_empty() {
            return None;
        }
        let row_count = self.rows.len() as u16;
        let available = body_height / row_count;
        let row_height = if available >= ROW_HEIGHT {
            available.min(6)
        } else if body_height >= row_count * ROW_HEIGHT {
            ROW_HEIGHT
        } else {
            1
        };
        let visible = if row_height == 1 {
            body_height.min(row_count).max(1)
        } else {
            row_count
        };
        let chart_height = visible * row_height + AXIS_HEIGHT;
        let row_top = area.y + area.height.saturating_sub(chart_height) / 2;
        Some(PlotGeometry {
            area,
            plot_x: area.x + label_width,
            plot_width,
            label_width,
            row_top,
            row_height,
            visible,
            axis_line_y: row_top + visible * row_height,
            axis_y: row_top + visible * row_height + 1,
            domain: self.domain(),
        })
    }
}

impl Default for BoxPlot {
    fn default() -> Self {
        Self::new()
    }
}

impl View for BoxPlot {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "box_plot"
    }

    fn outline(&self) -> Option<String> {
        let (domain_min, domain_max) = self.domain();
        Some(format!(
            "rows={}, presentation={}, domain_min={}, domain_max={}",
            self.rows.len(),
            quoted(&format!("{:?}", self.presentation)),
            domain_min,
            domain_max
        ))
    }

    fn children(&self) -> &[Node] {
        &self.labels
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.labels
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        buf.set_style(area, Style::default().bg(theme.background));
        if area.width < 24 || area.height < 4 {
            return;
        }
        let Some(geometry) = self.geometry(area) else {
            return;
        };
        let colors = PlotColors::resolve(&self.style, theme);
        draw_axis(&geometry, self.presentation, self.unit, &colors, buf);
        let tuning = self.tuning;
        for (index, row) in self.rows.iter().enumerate() {
            if index as u16 >= geometry.visible {
                break;
            }
            let top = geometry.row_top + index as u16 * geometry.row_height;
            draw_row(
                row,
                self.labels.get_mut(index),
                top,
                &geometry,
                self.presentation,
                &tuning,
                &colors,
                buf,
                cx,
                theme,
            );
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

struct PlotGeometry {
    area: Rect,
    plot_x: u16,
    plot_width: u16,
    label_width: u16,
    row_top: u16,
    row_height: u16,
    visible: u16,
    axis_line_y: u16,
    axis_y: u16,
    domain: (f64, f64),
}

impl PlotGeometry {
    fn x_of(&self, value: f64) -> u16 {
        let span = (self.domain.1 - self.domain.0).max(f64::EPSILON);
        let normalized = ((value - self.domain.0) / span).clamp(0.0, 1.0);
        self.plot_x + (normalized * f64::from(self.plot_width - 1)).round() as u16
    }
}

#[derive(Clone, Copy)]
struct PlotColors {
    axis: Color,
    band: Color,
    median: Color,
    whisker: Color,
    outlier: Color,
    value: Color,
}

impl PlotColors {
    fn resolve(style: &BoxPlotStyle, theme: &Theme) -> Self {
        Self {
            axis: style.axis.unwrap_or(theme.muted),
            band: style.band.unwrap_or(theme.primary),
            median: style.median.unwrap_or(theme.foreground),
            whisker: style.whisker.unwrap_or(theme.muted),
            outlier: style.outlier.unwrap_or(theme.warning),
            value: style.value.unwrap_or(theme.foreground),
        }
    }
}

fn draw_axis(
    geometry: &PlotGeometry,
    presentation: RenderStyle,
    unit: Option<&'static str>,
    colors: &PlotColors,
    buf: &mut Buffer,
) {
    let (horizontal, _vertical) = presentation.hairline();
    let grid = presentation.grid();
    let line_style = Style::default().fg(colors.axis);
    for x in geometry.plot_x..geometry.plot_x + geometry.plot_width {
        buf.set_string(x, geometry.axis_line_y, horizontal, line_style);
    }
    let max_ticks = (geometry.plot_width / 12).clamp(2, 8) as usize;
    let (ticks, step) = nice_ticks(geometry.domain, max_ticks);
    for tick in ticks {
        let x = geometry.x_of(tick);
        for y in geometry.row_top..geometry.axis_line_y {
            buf.set_string(x, y, grid, Style::default().fg(colors.axis));
        }
        let label = format_tick(tick, step);
        let width = label.chars().count().min(geometry.plot_width as usize) as u16;
        let start = x
            .saturating_sub(width.saturating_sub(1))
            .max(geometry.plot_x);
        buf.set_stringn(start, geometry.axis_y, label, width as usize, line_style);
    }
    if let Some(unit) = unit {
        let width = unit.chars().count().min(geometry.area.width as usize) as u16;
        let x = geometry.area.x + geometry.area.width - width;
        buf.set_stringn(x, geometry.axis_y, unit, width as usize, line_style);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row(
    row: &BoxPlotRow,
    label: Option<&mut Node>,
    top: u16,
    geometry: &PlotGeometry,
    presentation: RenderStyle,
    tuning: &StyleTuning,
    colors: &PlotColors,
    buf: &mut Buffer,
    cx: &mut Context,
    theme: &Theme,
) {
    let middle = top + geometry.row_height / 2;
    let band = row.color.unwrap_or(colors.band);
    if let Some(label) = label {
        let width = geometry.label_width.saturating_sub(1);
        label
            .view
            .render(Rect::new(geometry.area.x, middle, width, 1), buf, cx, theme);
    }
    draw_value(row, middle, geometry, colors, buf);
    draw_whiskers(row, middle, geometry, presentation, band, colors, buf);
    draw_box(row, top, geometry, presentation, tuning, band, buf);
    draw_median(row, top, geometry, presentation, colors, buf);
    draw_outliers(row, middle, geometry, presentation, colors, buf);
}

fn draw_value(
    row: &BoxPlotRow,
    y: u16,
    geometry: &PlotGeometry,
    colors: &PlotColors,
    buf: &mut Buffer,
) {
    let text = format!("p50 {:.0}", row.median);
    let width = text.chars().count().min(VALUE_COLUMN as usize) as u16;
    let x = geometry.area.x + geometry.area.width - width;
    buf.set_stringn(
        x,
        y,
        text,
        width as usize,
        Style::default().fg(colors.value),
    );
}

fn draw_whiskers(
    row: &BoxPlotRow,
    y: u16,
    geometry: &PlotGeometry,
    presentation: RenderStyle,
    band: Color,
    colors: &PlotColors,
    buf: &mut Buffer,
) {
    let (horizontal, vertical) = presentation.hairline();
    let cap = if presentation == RenderStyle::Native {
        None
    } else {
        Some(vertical)
    };
    let style = Style::default().fg(if row.color.is_some() {
        band
    } else {
        colors.whisker
    });
    let low = geometry.x_of(row.low);
    let q1 = geometry.x_of(row.q1);
    let q3 = geometry.x_of(row.q3);
    let high = geometry.x_of(row.high);
    for x in low..q1 {
        buf.set_string(x, y, horizontal, style);
    }
    for x in q3.saturating_add(1)..=high {
        buf.set_string(x, y, horizontal, style);
    }
    let (left_cap, right_cap) = match cap {
        Some(cap) => (cap, cap),
        None => ("├", "┤"),
    };
    buf.set_string(low, y, left_cap, style);
    buf.set_string(high, y, right_cap, style);
}

fn draw_box(
    row: &BoxPlotRow,
    top: u16,
    geometry: &PlotGeometry,
    presentation: RenderStyle,
    tuning: &StyleTuning,
    band: Color,
    buf: &mut Buffer,
) {
    let q1 = geometry.x_of(row.q1);
    let q3 = geometry.x_of(row.q3);
    let height = geometry.row_height;
    let shaded = presentation != RenderStyle::Native;
    let half = (q3.saturating_sub(q1) as f32 / 2.0).max(1.0);
    let median_x = geometry.x_of(row.median) as f32;
    let edge = |x: u16| {
        if x == q1 {
            Some("▐")
        } else if x == q3 {
            Some("▌")
        } else {
            None
        }
    };
    let layer = |offset: u16| {
        if offset == 0 || offset + 1 == height {
            0.55
        } else if offset == height / 2 {
            1.0
        } else {
            0.85
        }
    };
    let intensity = |x: u16, layer: f32| {
        let distance = (x as f32 - median_x).abs() / half;
        (layer * (1.0 - 0.25 * distance.min(1.0))).clamp(0.55, 1.0)
    };
    for x in q1..=q3 {
        for offset in 0..height {
            let glyph = if shaded {
                presentation.shade(intensity(x, layer(offset)), tuning)
            } else if let Some(edge) = edge(x) {
                edge
            } else if offset == 0 {
                "▄"
            } else if offset + 1 == height {
                "▀"
            } else {
                "█"
            };
            buf.set_string(x, top + offset, glyph, Style::default().fg(band));
        }
    }
}

fn draw_median(
    row: &BoxPlotRow,
    top: u16,
    geometry: &PlotGeometry,
    presentation: RenderStyle,
    colors: &PlotColors,
    buf: &mut Buffer,
) {
    let x = geometry.x_of(row.median);
    let style = Style::default().fg(colors.median);
    let height = geometry.row_height;
    for offset in 0..height {
        let glyph = if presentation == RenderStyle::Native {
            if offset == 0 {
                "┬"
            } else if offset + 1 == height {
                "┴"
            } else {
                "┃"
            }
        } else {
            presentation.hairline().1
        };
        buf.set_string(x, top + offset, glyph, style);
    }
}

fn draw_outliers(
    row: &BoxPlotRow,
    y: u16,
    geometry: &PlotGeometry,
    presentation: RenderStyle,
    colors: &PlotColors,
    buf: &mut Buffer,
) {
    let glyph = presentation.marker();
    for value in &row.outliers {
        let x = geometry.x_of(*value);
        buf.set_string(x, y, glyph, Style::default().fg(colors.outlier));
    }
}

/// Rounded tick step (1, 2 or 5 times a power of ten) and its ticks.
fn nice_ticks(domain: (f64, f64), max_ticks: usize) -> (Vec<f64>, f64) {
    let (low, high) = domain;
    let span = high - low;
    if !span.is_finite() || span <= 0.0 {
        return (vec![low], 1.0);
    }
    let raw = span / max_ticks.max(2) as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let step = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    } * magnitude;
    let mut ticks = Vec::new();
    let mut tick = (low / step).ceil() * step;
    while tick <= high + step * 1e-6 && ticks.len() < 32 {
        ticks.push(tick);
        tick += step;
    }
    (ticks, step)
}

fn format_tick(value: f64, step: f64) -> String {
    if step >= 1.0 {
        format!("{value:.0}")
    } else if step >= 0.1 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}
