//! A proportional circle for category composition.

use std::any::Any;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::layout::{row, Align};
use super::presentation::{RenderStyle, StyleTuning};

/// One category in a pie chart.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PieSlice {
    /// Category label shown in the legend.
    label: String,
    /// Non-negative amount represented by this slice.
    value: f64,
    /// Color assigned by the host.
    color: Color,
}

impl PieSlice {
    /// Creates a labeled slice. Invalid and negative values are normalized to zero.
    pub fn new(label: impl Into<String>, value: f64, color: Color) -> Self {
        Self {
            label: label.into(),
            value: if value.is_finite() && value > 0.0 {
                value
            } else {
                0.0
            },
            color,
        }
    }

    /// The category label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The normalized non-negative amount.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The color supplied by the host.
    pub fn color(&self) -> Color {
        self.color
    }
}

/// Last rendered geometry of a pie chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PieChartState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the chart would hold focus by the tree. Charts are display-only.
    pub focused: bool,
}

/// A pie chart whose values, labels and colors are supplied by its host.
#[derive(Serialize)]
pub struct PieChart {
    /// Last rendered area.
    pub state: PieChartState,
    children: Vec<Node>,
    slices: Vec<PieSlice>,
    presentation: RenderStyle,
    tuning: StyleTuning,
}

impl PieChart {
    /// Creates an empty chart.
    pub fn new() -> Self {
        Self {
            state: PieChartState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
            },
            children: Vec::new(),
            slices: Vec::new(),
            presentation: RenderStyle::Native,
            tuning: StyleTuning::default(),
        }
    }

    /// Replaces all categories.
    pub fn set_slices(&mut self, slices: Vec<PieSlice>) {
        self.slices = slices
            .into_iter()
            .map(|slice| PieSlice::new(slice.label, slice.value, slice.color))
            .collect();
    }

    /// Builder shorthand for replacing all categories.
    pub fn with_slices(mut self, slices: Vec<PieSlice>) -> Self {
        self.set_slices(slices);
        self
    }

    /// Chooses the glyph family used to draw the slices.
    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    /// Replaces the glyph family used to draw the slices.
    pub fn set_presentation(&mut self, presentation: RenderStyle) {
        self.presentation = presentation;
    }

    /// Tunes density, contrast and halftone dot size for alternate glyphs.
    pub fn with_tuning(mut self, tuning: StyleTuning) -> Self {
        self.tuning = tuning.clamped();
        self
    }

    /// Replaces the density, contrast and dot size used by alternate glyphs.
    pub fn set_tuning(&mut self, tuning: StyleTuning) {
        self.tuning = tuning.clamped();
    }

    /// Composes a caption row above the chart.
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
        self
    }

    fn scale(&self) -> f64 {
        self.slices
            .iter()
            .map(|slice| slice.value)
            .fold(0.0, f64::max)
    }

    fn total_scaled(&self, scale: f64) -> f64 {
        if scale == 0.0 {
            return 0.0;
        }
        self.slices.iter().map(|slice| slice.value / scale).sum()
    }

    fn color_at(&self, angle: f64) -> Option<Color> {
        let scale = self.scale();
        let total = self.total_scaled(scale);
        if total <= 0.0 {
            return None;
        }
        let position = (angle / std::f64::consts::TAU).rem_euclid(1.0) * total;
        let mut cumulative = 0.0;
        self.slices.iter().find_map(|slice| {
            cumulative += slice.value / scale;
            (slice.value > 0.0 && position < cumulative).then_some(slice.color)
        })
    }

    fn near_slice_edge(&self, angle: f64, radius: f64) -> bool {
        let scale = self.scale();
        let total = self.total_scaled(scale);
        if total <= 0.0 || radius <= 0.0 {
            return false;
        }
        let position = (angle / std::f64::consts::TAU).rem_euclid(1.0) * total;
        let mut boundary = 0.0;
        self.slices
            .iter()
            .filter(|slice| slice.value > 0.0)
            .any(|slice| {
                boundary += slice.value / scale;
                let direct = (position - boundary).abs();
                direct.min(total - direct) * std::f64::consts::TAU * radius < 0.65
            })
    }
}

impl Default for PieChart {
    fn default() -> Self {
        Self::new()
    }
}

impl View for PieChart {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "pie_chart"
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
        let body_y = area.y.saturating_add(caption_height);
        let body_height = area.height.saturating_sub(caption_height);
        if body_height == 0 || self.slices.is_empty() {
            return;
        }

        let desired_legend = area.width / 3;
        let circle_width = body_height
            .saturating_mul(2)
            .min(area.width.saturating_sub(desired_legend));
        let legend_width = area.width.saturating_sub(circle_width);
        let circle_height = body_height.min(circle_width.div_ceil(2));
        let circle = Rect::new(area.x, body_y, circle_width, circle_height);
        self.paint_circle(circle, buf, theme);
        self.paint_legend(
            Rect::new(
                area.x.saturating_add(circle_width),
                body_y,
                legend_width,
                body_height,
            ),
            buf,
            theme,
        );
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

impl PieChart {
    fn paint_circle(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let diameter = f64::from(area.width.min(area.height.saturating_mul(2)));
        let radius = diameter / 2.0;
        let center_x = f64::from(area.x) + f64::from(area.width) / 2.0;
        let center_y_pixels = f64::from(area.y) * 2.0 + f64::from(area.height);
        for y in 0..area.height {
            for x in 0..area.width {
                let top = self.pixel_color(area, x, y, 0, center_x, center_y_pixels, radius);
                let bottom = self.pixel_color(area, x, y, 1, center_x, center_y_pixels, radius);
                if self.presentation != RenderStyle::Native {
                    if let Some((color, glyph)) = top.or(bottom) {
                        buf.set_string(area.x + x, area.y + y, glyph, Style::default().fg(color));
                    }
                    continue;
                }
                match (top, bottom) {
                    (Some((top, top_glyph)), Some((bottom, bottom_glyph)))
                        if top == bottom && top_glyph == bottom_glyph =>
                    {
                        buf.set_string(area.x + x, area.y + y, top_glyph, Style::default().fg(top));
                    }
                    (Some((top, _)), Some((bottom, _))) => {
                        buf.set_string(
                            area.x + x,
                            area.y + y,
                            "▀",
                            Style::default().fg(top).bg(bottom),
                        );
                    }
                    (Some((color, glyph)), None) => {
                        buf.set_string(
                            area.x + x,
                            area.y + y,
                            if glyph == "█" { "▀" } else { glyph },
                            Style::default().fg(color).bg(theme.background),
                        );
                    }
                    (None, Some((color, glyph))) => {
                        buf.set_string(
                            area.x + x,
                            area.y + y,
                            if glyph == "█" { "▄" } else { glyph },
                            Style::default().fg(color).bg(theme.background),
                        );
                    }
                    (None, None) => {}
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn pixel_color(
        &self,
        area: Rect,
        x: u16,
        y: u16,
        half: u16,
        center_x: f64,
        center_y_pixels: f64,
        radius: f64,
    ) -> Option<(Color, &'static str)> {
        let pixel_x = f64::from(area.x + x) + 0.5;
        let pixel_y = f64::from(area.y + y) * 2.0 + f64::from(half) + 0.5;
        let dx = pixel_x - center_x;
        let dy = pixel_y - center_y_pixels;
        if dx.mul_add(dx, dy * dy) > radius * radius {
            return None;
        }
        let angle = dy.atan2(dx).rem_euclid(std::f64::consts::TAU);
        if self.near_slice_edge(angle, radius) {
            return None;
        }
        let radial = (dx.mul_add(dx, dy * dy).sqrt() / radius).clamp(0.0, 1.0) as f32;
        let color = self.color_at(angle)?;
        let intensity = if radial < 0.78 {
            1.0
        } else {
            ((1.0 - radial) / 0.22).clamp(0.0, 1.0)
        };
        let glyph = self.presentation.shade(intensity, &self.tuning);
        Some((color, glyph))
    }

    fn paint_legend(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width < 8 || area.height == 0 {
            return;
        }
        let scale = self.scale();
        let total = self.total_scaled(scale);
        if total <= 0.0 {
            buf.set_stringn(
                area.x,
                area.y,
                "no data",
                usize::from(area.width),
                Style::default().fg(theme.muted),
            );
            return;
        }
        for (index, slice) in self
            .slices
            .iter()
            .filter(|slice| slice.value > 0.0)
            .enumerate()
        {
            let Some(y) = area.y.checked_add(u16::try_from(index).unwrap_or(u16::MAX)) else {
                break;
            };
            if y >= area.y.saturating_add(area.height) {
                break;
            }
            let percent = (slice.value / scale) / total * 100.0;
            buf.set_string(area.x, y, "●", Style::default().fg(slice.color));
            let label_width = usize::from(if area.width >= 20 {
                area.width.saturating_sub(10)
            } else {
                area.width.saturating_sub(2)
            });
            buf.set_stringn(
                area.x + 2,
                y,
                &slice.label,
                label_width,
                Style::default().fg(theme.foreground),
            );
            if area.width >= 20 {
                let value_x = area.x + area.width.saturating_sub(7);
                buf.set_stringn(
                    value_x,
                    y,
                    format!("{percent:>5.1}%"),
                    7,
                    Style::default().fg(theme.muted),
                );
            }
        }
    }
}
