//! CandleChart: OHLC candles with capacity, zoom, scroll and a live-edge readout.
use crossterm::event::{KeyCode, KeyEventKind, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::{Serialize, Serializer};
use std::any::Any;
use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::axis::{format_value, nice_max};
use super::layout::{row, Align};
use super::presentation::{Presentation, RenderStyle, StyleTuning};
use super::scroll::{render_scrollbar, Scroll, ScrollbarStyle, ScrollbarVisibility};

#[path = "candlechart_hover.rs"]
mod hover;

/// The glyph set the chart was originally drawn with (candle bodies, wick
/// and a dashed live line). Used for [`RenderStyle::Native`].
const NATIVE: Presentation = Presentation {
    grid: "·",
    wick: "│",
    doji: "━",
    body: "█",
    live: "╌",
};

const MAX_HISTORY: usize = 240;
/// Ticks on a zero-based value axis.
const AXIS_DIVISIONS: usize = 10;

/// Width the caption row actually occupies, so the OHLC stats sit next to it.
/// Min and max of `volume` in the visible slice; the ink ramp for candle
/// bodies and volume bars is normalized against this range so every chart
/// frame spreads the full ramp, not just its dense half.
fn caption_width(children: &[Node]) -> u16 {
    if children.is_empty() {
        return 0;
    }
    let measured: u16 = children
        .iter()
        .filter_map(|child| child.view.measure().map(|(width, _)| width))
        .sum();
    measured + 2 * (children.len() as u16) + 2
}

/// Public state of the chart: area, focus, scroll and zoom.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChartState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the chart would hold focus by the tree. Charts are display-only.
    pub focused: bool,
    /// Candles scrolled back from the live edge. Zero = following the live candle.
    pub scroll_back: usize,
    /// Horizontal zoom multiplier on the candle cell width. 1.0 = default 2 cells per candle.
    pub x_zoom: f64,
    /// Vertical zoom multiplier on the price range. 1.0 = data range fits the plot.
    pub y_zoom: f64,
}

/// One OHLC candle plus its volume.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Candle {
    /// Opening price.
    pub open: f64,
    /// Highest price.
    pub high: f64,
    /// Lowest price.
    pub low: f64,
    /// Closing price.
    pub close: f64,
    /// Traded volume, used by the optional volume band.
    pub volume: f64,
}

impl Candle {
    fn normalized(mut self) -> Option<Self> {
        if ![self.open, self.high, self.low, self.close, self.volume]
            .into_iter()
            .all(f64::is_finite)
        {
            return None;
        }
        let bounds_high = self.open.max(self.close);
        let bounds_low = self.open.min(self.close);
        self.high = self.high.max(bounds_high).max(self.low);
        self.low = self.low.min(bounds_low).min(self.high);
        self.volume = self.volume.max(0.0);
        Some(self)
    }
}

/// Per-instance overrides for a candle chart. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CandleChartStyle {
    /// Bullish candle color. Defaults to `theme.success`.
    pub up: Option<Color>,
    /// Bearish candle color. Defaults to `theme.destructive`.
    pub down: Option<Color>,
    /// Grid and axis glyph color. Defaults to `theme.muted`.
    pub grid: Option<Color>,
    /// Axis label color. Defaults to `theme.muted`.
    pub label: Option<Color>,
    /// Live price line color. Defaults to `theme.accent`.
    pub live: Option<Color>,
    /// History scrollbar drawn on the right edge of the plot when history
    /// exceeds the visible window (or always, if [`ScrollbarVisibility::Always`]).
    /// `None` (default) draws none.
    pub scrollbar: Option<ScrollbarStyle>,
}

/// Closures are not serializable; the label source is reported by presence.
fn serialize_x_label<S>(
    label: &Option<Box<dyn Fn(usize) -> String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(label.is_some())
}

/// A candlestick chart with volume bars, scrollback and zoom.
#[derive(Serialize)]
pub struct CandleChart {
    /// Public state: area, focus, scroll and zoom.
    pub state: ChartState,
    /// Per-instance overrides; defaults match the theme.
    pub style: CandleChartStyle,
    /// How the chart paints itself (Native, Blocks, Ascii, …).
    pub presentation: RenderStyle,
    /// Caption row (symbol, interval, live badge, ...), composed by the host.
    children: Vec<Node>,
    /// Shading knobs for the volume bars when a non-`Native` presentation
    /// is selected. Volume intensity maps onto the ink ramp.
    pub tuning: StyleTuning,
    /// Injected by the host: label for a candle's absolute index (axis tick
    /// every 10 candles). When unset the axis stays unlabeled.
    #[serde(serialize_with = "serialize_x_label")]
    x_label: Option<Box<dyn Fn(usize) -> String>>,
    #[serde(skip)]
    hover_label: Option<Box<dyn Fn(usize) -> String>>,
    series: Vec<Candle>,
    hover_plot: Rect,
    visible_range: (usize, usize),
    hovered: Option<usize>,
    pointer_origin: (i32, i32),
}

impl CandleChart {
    /// Creates an empty chart.
    pub fn new() -> Self {
        Self {
            state: ChartState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                scroll_back: 0,
                x_zoom: 1.0,
                y_zoom: 1.0,
            },
            style: CandleChartStyle::default(),
            presentation: RenderStyle::default(),
            tuning: StyleTuning::default(),
            children: Vec::new(),
            x_label: None,
            hover_label: None,
            series: Vec::new(),
            hover_plot: Rect::default(),
            visible_range: (0, 0),
            hovered: None,
            pointer_origin: (0, 0),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: CandleChartStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder shorthand: pick the presentation.
    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    /// Builder shorthand: set the shading knobs for non-`Native` presentations.
    pub fn with_tuning(mut self, tuning: StyleTuning) -> Self {
        self.tuning = tuning;
        self
    }

    /// Compose the caption row above the plot (symbol, badges, interval, ...).
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    /// Builder shorthand: set the injected x-axis label source (absolute candle index to label).
    pub fn with_x_label(mut self, label: impl Fn(usize) -> String + 'static) -> Self {
        self.set_x_label(label);
        self
    }

    /// Sets or replaces the injected x-axis label source (absolute candle
    /// index → label). Used for axis ticks every 10 candles.
    pub fn set_x_label(&mut self, label: impl Fn(usize) -> String + 'static) {
        self.x_label = Some(Box::new(label));
    }

    /// Sets the full label used by the pointer readout, independently of compact axis labels.
    pub fn set_x_hover_label(&mut self, label: impl Fn(usize) -> String + 'static) {
        self.hover_label = Some(Box::new(label));
    }

    /// Sets the screen origin of this chart when it is rendered into an
    /// off-screen surface that is later translated before display.
    pub fn set_pointer_origin(&mut self, x: i32, y: i32) {
        self.pointer_origin = (x, y);
    }

    /// Replace the whole series. Anchors the view to the live edge.
    pub fn set_series(&mut self, series: Vec<Candle>) {
        let mut series: Vec<_> = series.into_iter().filter_map(Candle::normalized).collect();
        if series.len() > MAX_HISTORY {
            series.drain(..series.len() - MAX_HISTORY);
        }
        self.series = series;
        self.state.scroll_back = 0;
        self.hovered = None;
    }

    /// Replaces the forming (last) candle in place. No-op on an empty series
    /// (equivalent to pushing it).
    pub fn update_candle(&mut self, candle: Candle) {
        let Some(candle) = candle.normalized() else {
            return;
        };
        if let Some(last) = self.series.last_mut() {
            *last = candle;
        } else {
            self.series.push(candle);
        }
    }

    /// Appends a new candle. When the user is scrolled back the same candles
    /// stay on screen; when following the live edge the window tracks forward.
    pub fn push_candle(&mut self, candle: Candle) {
        let Some(candle) = candle.normalized() else {
            return;
        };
        self.series.push(candle);
        if self.series.len() > MAX_HISTORY {
            self.series.remove(0);
            self.state.scroll_back = self.state.scroll_back.saturating_sub(1);
        } else if self.state.scroll_back > 0 {
            self.state.scroll_back += 1;
        }
    }

    /// Number of candles currently held.
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// `true` when there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    fn scroll(&mut self, delta: i32) -> Outcome {
        let max = self.series.len().saturating_sub(1);
        let previous = self.state.scroll_back;
        let next = (self.state.scroll_back as i32 + delta).clamp(0, max as i32) as usize;
        if next == previous {
            return Outcome::Ignored;
        }
        self.state.scroll_back = next;
        Outcome::Consumed
    }

    /// Zoom the horizontal axis, keeping candles visible. Wheel-up on the
    /// time axis expands (more candles), wheel-down zooms in.
    pub fn zoom_x(&mut self, factor: f64) {
        self.state.x_zoom = (self.state.x_zoom * factor).clamp(0.5, 8.0);
    }

    /// Zoom the vertical axis, keeping candles visible. Wheel-up on the
    /// price axis expands the range, wheel-down zooms in.
    pub fn zoom_y(&mut self, factor: f64) {
        self.state.y_zoom = (self.state.y_zoom * factor).clamp(0.1, 20.0);
    }

    fn cell_width(&self) -> u16 {
        (2.0 * self.state.x_zoom).round().clamp(1.0, 16.0) as u16
    }

    fn set_scroll_back(&mut self, target: usize) -> Outcome {
        let previous = self.state.scroll_back;
        let max = self.series.len().saturating_sub(1);
        let next = target.min(max);
        if next == previous {
            return Outcome::Ignored;
        }
        self.state.scroll_back = next;
        Outcome::Consumed
    }
}

impl Default for CandleChart {
    fn default() -> Self {
        Self::new()
    }
}

impl View for CandleChart {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "candle_chart"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "presentation={}, scroll_back={}, x_zoom={}, y_zoom={}",
            quoted(&format!("{:?}", self.presentation)),
            self.state.scroll_back,
            self.state.x_zoom,
            self.state.y_zoom
        ))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        self.state.focused = cx.is_focused();
        self.hover_plot = Rect::default();
        buf.set_style(area, Style::default().bg(theme.background));
        let caption_height = u16::from(!self.children.is_empty());
        if caption_height > 0 {
            let caption = Rect::new(area.x, area.y, area.width, 1);
            row(&mut self.children, caption, Align::Left, 2, buf, cx, theme);
        }
        if self.series.is_empty() {
            super::chart_empty::paint(
                Rect::new(
                    area.x,
                    area.y.saturating_add(caption_height),
                    area.width,
                    area.height.saturating_sub(caption_height),
                ),
                "No data",
                buf,
                theme,
            );
            return;
        }
        if area.width < 12 || area.height < 6 {
            return;
        }
        let up = self.style.up.unwrap_or(theme.success);
        let down = self.style.down.unwrap_or(theme.destructive);
        let grid_color = self.style.grid.unwrap_or(theme.muted);
        let label_color = self.style.label.unwrap_or(theme.muted);
        let live_color = self.style.live.unwrap_or(theme.accent);
        let pres = self.presentation.resolve(NATIVE);

        let total = self.series.len();
        let following = self.state.scroll_back == 0;
        let cell_width = self.cell_width();
        // Reserve the last column for the history scrollbar when one is
        // configured and there is something to indicate.
        let full_plot_width = area.width.saturating_sub(11).max(1);
        let base_visible = (full_plot_width / cell_width).max(1) as usize;
        let want_rail = self
            .style
            .scrollbar
            .as_ref()
            .map(|scrollbar| match scrollbar.visible {
                ScrollbarVisibility::Never => false,
                ScrollbarVisibility::Always => true,
                ScrollbarVisibility::Auto => total > base_visible,
            })
            .unwrap_or(false);
        let plot_width = area
            .width
            .saturating_sub(if want_rail { 12 } else { 11 })
            .max(1);
        let visible_count = (plot_width / cell_width).max(1) as usize;
        let end = total.saturating_sub(self.state.scroll_back).max(1);
        let start = end.saturating_sub(visible_count);
        let series_index = |visible: usize| -> usize { start + visible };

        // Keep ordinary price axes anchored at zero and fit signed data around
        // rounded bounds so negative values remain visible.
        let floor = self.series[start..end]
            .iter()
            .map(|candle| candle.low.min(candle.open).min(candle.close))
            .fold(f64::INFINITY, f64::min);
        let peak = self.series[start..end]
            .iter()
            .map(|candle| candle.high.max(candle.close).max(candle.open))
            .fold(0.0f64, |peak, value| peak.max(value));
        let (mut low, mut high) = if floor < 0.0 {
            super::axis::nice_domain(floor, peak, AXIS_DIVISIONS)
        } else {
            (0.0, nice_max(peak, AXIS_DIVISIONS))
        };
        if low < 0.0 {
            let center = (low + high) / 2.0;
            let range = ((high - low) * self.state.y_zoom).max(0.001);
            low = center - range / 2.0;
            high = center + range / 2.0;
        } else {
            high = (high * self.state.y_zoom).max(0.001);
        }
        let range = (high - low).max(0.001);

        let caption_offset = if self.children.is_empty() { 0 } else { 1 };
        let plot_top = area.y + caption_offset + 1;
        let axis_y = area.y + area.height.saturating_sub(1);
        let axis_line_y = axis_y.saturating_sub(1);
        let plot_bottom = axis_line_y.saturating_sub(3);
        if plot_bottom < plot_top {
            return;
        }
        self.hover_plot = Rect::new(
            area.x,
            plot_top,
            plot_width,
            plot_bottom.saturating_sub(plot_top).saturating_add(1),
        );
        self.visible_range = (start, end);
        let plot_height = (plot_bottom - plot_top + 1) as f64;
        let to_y = |value: f64| -> u16 {
            let normalized = ((value - low) / range).clamp(0.0, 1.0);
            plot_top + ((1.0 - normalized) * (plot_height - 1.0).max(0.0)).round() as u16
        };

        let axis_x = area.x + plot_width + 1;
        let inner_width = plot_width.saturating_sub(1);

        let shaded = self.presentation != RenderStyle::Native;
        let tuning = self.tuning;

        // Header stats row (data-derived, not composed content).
        let visible_slice = &self.series[start..end];
        let last = visible_slice[visible_slice.len() - 1];
        let first_open = visible_slice[0].open;
        let reference_close = last.close;
        let change = (first_open != 0.0)
            .then(|| (reference_close - first_open) / first_open * 100.0)
            .filter(|change| change.is_finite());
        let change_color = if change.is_some_and(|change| change >= 0.0) {
            up
        } else {
            down
        };
        let stats_x = area.x + caption_width(&self.children);
        let stats_y = area.y;
        let header = format!(
            "O {} H {} L {} C {} {}",
            format_value(last.open),
            format_value(last.high),
            format_value(last.low),
            format_value(reference_close),
            change.map_or_else(|| "n/a".to_owned(), |change| format!("{change:+.2}%")),
        );
        if caption_height > 0 && area.width >= 44 {
            buf.set_stringn(
                stats_x,
                stats_y,
                header,
                inner_width as usize,
                Style::default().fg(change_color),
            );
        } else if caption_height > 0 && area.width >= 24 {
            buf.set_stringn(
                stats_x,
                stats_y,
                change.map_or_else(|| "n/a".to_owned(), |change| format!("{change:+.2}%")),
                8,
                Style::default().fg(change_color),
            );
        }

        // Gridline count follows the plot height and shares the labeled ticks.
        let divisions = AXIS_DIVISIONS
            .min(plot_bottom.saturating_sub(plot_top) as usize)
            .max(1);
        let axis: Vec<f64> = (0..=divisions)
            .map(|step| low + (step as f64 / divisions as f64) * range)
            .collect();
        for value in &axis {
            let y = to_y(*value).clamp(plot_top, plot_bottom);
            for x in (area.x..area.x + plot_width).step_by(3) {
                buf.set_string(x, y, pres.grid, Style::default().fg(grid_color));
            }
        }
        let span = plot_bottom.saturating_sub(plot_top);
        for index in 0..=divisions {
            let value = axis[divisions - index];
            let y = plot_top + (index as u16 * span / divisions as u16);
            buf.set_stringn(
                axis_x + 2,
                y,
                format_value(value),
                8,
                Style::default().fg(label_color),
            );
        }

        // Axes.
        for y in plot_top..=plot_bottom {
            buf.set_string(axis_x, y, "│", Style::default().fg(grid_color));
        }
        let axis_line_width = (plot_width + 1).min(area.width);
        for x in area.x..area.x + axis_line_width {
            buf.set_string(x, axis_line_y, "─", Style::default().fg(grid_color));
        }
        // X-axis labels, injected by the host, every 10 candles.
        if let Some(x_label) = &self.x_label {
            for visible in (0..(end - start)).step_by(10) {
                let index = start + visible;
                let x = area.x + visible as u16 * cell_width;
                buf.set_stringn(
                    x,
                    axis_y,
                    x_label(index),
                    5,
                    Style::default().fg(label_color),
                );
            }
        }

        // Candles.
        let wick_x_offset = cell_width / 2;
        for visible in 0..(end - start) {
            let index = series_index(visible);
            let candle = self.series[index];
            let x = area.x + visible as u16 * cell_width;
            let color = if candle.close >= candle.open {
                up
            } else {
                down
            };
            let intensity = (candle.volume / 2.0).clamp(0.0, 1.0) as f32;
            let candle_body = if shaded {
                self.presentation.shade(intensity, &tuning)
            } else {
                pres.body
            };
            let candle_marker = if shaded {
                self.presentation.shade(intensity, &tuning)
            } else {
                pres.doji
            };
            let y_high = to_y(candle.high);
            let y_low = to_y(candle.low);
            let y_top = to_y(candle.open.max(candle.close));
            let y_bottom = to_y(candle.open.min(candle.close));
            for y in y_high..=y_low {
                buf.set_string(x + wick_x_offset, y, pres.wick, Style::default().fg(color));
            }
            if y_top == y_bottom {
                for dx in 0..cell_width {
                    buf.set_string(x + dx, y_top, candle_marker, Style::default().fg(color));
                }
            } else {
                for y in y_top..=y_bottom {
                    for dx in 0..cell_width {
                        buf.set_string(x + dx, y, candle_body, Style::default().fg(color));
                    }
                }
            }
        }

        // History scrollbar, one cell wide, riding on the right edge.
        if want_rail {
            let scrollbar = self.style.scrollbar.clone().unwrap_or_default();
            let rail_height = plot_bottom.saturating_sub(plot_top).saturating_add(1);
            let scroll = Scroll::scaled(rail_height as usize, start, visible_count, total);
            let rail = Rect::new(
                area.x + area.width.saturating_sub(1),
                plot_top,
                1,
                rail_height,
            );
            render_scrollbar(buf, rail, &scrollbar, theme, &scroll);
        }

        // Volume bars, two rows above the time axis.
        let volume_base = axis_line_y.saturating_sub(1);
        if volume_base > plot_bottom {
            for visible in 0..(end - start) {
                let index = series_index(visible);
                let candle = self.series[index];
                let x = area.x + visible as u16 * cell_width;
                let bars = ((candle.volume * 1.5).round() as i64).clamp(0, 2) as usize;
                let color = if candle.close >= candle.open {
                    up
                } else {
                    down
                };
                let intensity = (candle.volume / 2.0).clamp(0.0, 1.0) as f32;
                for step in 0..bars {
                    let glyph = if shaded {
                        self.presentation.shade(intensity, &tuning)
                    } else {
                        pres.body
                    };
                    buf.set_string(
                        x,
                        volume_base - step as u16,
                        glyph,
                        Style::default().fg(color),
                    );
                }
            }
        }

        // Live price line with dashes and a price tag at the axis.
        if following {
            let live_price = self.series[total - 1].close;
            let live_y = to_y(live_price).clamp(plot_top, plot_bottom);
            for x in (area.x..area.x + plot_width).step_by(3) {
                buf.set_string(x, live_y, pres.live, Style::default().fg(live_color));
            }
            let tag = format!(" {} ", format_value(live_price));
            buf.set_stringn(
                axis_x + 2,
                live_y,
                tag,
                9,
                Style::default().fg(theme.background).bg(live_color),
            );
            buf.set_string(axis_x, live_y, "◄", Style::default().fg(live_color));
        }
        hover::paint(self, buf, theme, &to_y, plot_top, plot_bottom, cell_width);
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        // Wheel is direct manipulation and works without the keyboard focus,
        // but only over our own area: the chart body scrolls history, the
        // price axis zooms the range, the time axis zooms the candles.
        // Anything outside our area is left for the rest of the tree.
        if let InputEvent::Mouse(mouse) = event {
            if mouse.kind == MouseEventKind::Moved {
                return hover::handle_move(self, *mouse);
            }
            if matches!(
                mouse.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            ) {
                let area = self.state.area;
                let inside = mouse.row >= area.y
                    && mouse.row < area.y.saturating_add(area.height)
                    && mouse.column >= area.x
                    && mouse.column < area.x.saturating_add(area.width);
                if !inside || area.height < 6 || area.width < 12 {
                    return Outcome::Ignored;
                }
                let up = matches!(mouse.kind, MouseEventKind::ScrollUp);
                let axis_y = area.y + area.height.saturating_sub(1);
                let axis_x = area.x + area.width.saturating_sub(11);
                if mouse.row == axis_y {
                    self.zoom_x(if up { 0.8 } else { 1.25 });
                    return Outcome::Consumed;
                }
                if mouse.column >= axis_x {
                    self.zoom_y(if up { 1.25 } else { 0.8 });
                    return Outcome::Consumed;
                }
                return match mouse.kind {
                    MouseEventKind::ScrollDown => self.scroll(3),
                    MouseEventKind::ScrollUp => self.scroll(-3),
                    _ => Outcome::Ignored,
                };
            }
        }
        let focused = cx.is_focused();
        self.state.focused = focused;
        if !focused {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Left => self.scroll(1),
                    KeyCode::Right => self.scroll(-1),
                    KeyCode::Home => self.set_scroll_back(usize::MAX),
                    KeyCode::End => self.set_scroll_back(0),
                    _ => Outcome::Ignored,
                }
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        true
    }
}
