//! A proportional circle for category composition.

use std::any::Any;

use crossterm::event::MouseEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};
use unicode_width::UnicodeWidthStr;

use super::axis::format_value;
use super::layout::{row, Align};
use super::presentation::{DotSize, RenderStyle, StyleTuning};

const BRAILLE_DOTS: [[u8; 2]; 4] = [[0, 3], [1, 4], [2, 5], [6, 7]];
const BRAILLE_ORDER: [u8; 8] = [0, 7, 2, 5, 1, 6, 3, 4];
const BRAILLE_RANK: [usize; 8] = [0, 4, 2, 6, 7, 3, 5, 1];

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
    pointer_origin: (i32, i32),
    hovered: Option<(usize, u16, u16)>,
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
            pointer_origin: (0, 0),
            hovered: None,
        }
    }

    /// Replaces all categories.
    pub fn set_slices(&mut self, slices: Vec<PieSlice>) {
        self.slices = slices
            .into_iter()
            .map(|slice| PieSlice::new(slice.label, slice.value, slice.color))
            .collect();
        self.hovered = None;
    }

    /// Sets the screen origin of this chart when it is rendered into an
    /// off-screen surface that is later translated before display.
    pub fn set_pointer_origin(&mut self, x: i32, y: i32) {
        self.pointer_origin = (x, y);
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
                direct.min(total - direct) * std::f64::consts::TAU * radius < 0.18
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
        if self.slices.is_empty() {
            super::chart_empty::paint(
                Rect::new(area.x, body_y, area.width, body_height),
                "No data",
                buf,
                theme,
            );
            return;
        }
        if body_height == 0 {
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
        self.paint_hover(buf, theme);
    }

    fn handle_event(&mut self, event: &InputEvent, _cx: &mut Context) -> Outcome {
        let InputEvent::Mouse(mouse) = event else {
            return Outcome::Ignored;
        };
        if mouse.kind != MouseEventKind::Moved {
            return Outcome::Ignored;
        }
        let area = self.state.area;
        let x = i32::from(mouse.column) - self.pointer_origin.0;
        let y = i32::from(mouse.row) - self.pointer_origin.1;
        let next = self.slice_at(area, x, y);
        let next = next.map(|index| (index, x as u16, y as u16));
        if next == self.hovered {
            return Outcome::Ignored;
        }
        let had_hover = self.hovered.is_some();
        self.hovered = next;
        if next.is_some() || had_hover {
            Outcome::Consumed
        } else {
            Outcome::Ignored
        }
    }
}

impl PieChart {
    fn slice_at(&self, area: Rect, x: i32, y: i32) -> Option<usize> {
        if self.slices.is_empty() || area.width == 0 || area.height == 0 {
            return None;
        }
        let caption_height = u16::from(!self.children.is_empty());
        let body_y = area.y.saturating_add(caption_height);
        let body_height = area.height.saturating_sub(caption_height);
        let circle_width = body_height
            .saturating_mul(2)
            .min(area.width.saturating_sub(area.width / 3));
        let circle_height = body_height.min(circle_width.div_ceil(2));
        if circle_width == 0 || circle_height == 0 {
            return None;
        }
        let center_x = f64::from(area.x) + f64::from(circle_width) / 2.0;
        let center_y = f64::from(body_y) * 2.0 + f64::from(circle_height);
        let diameter = f64::from(circle_width.min(circle_height.saturating_mul(2)));
        let radius = diameter / 2.0;
        let dx = x as f64 + 0.5 - center_x;
        let dy = y as f64 * 2.0 + 1.0 - center_y;
        if radius == 0.0 || dx.hypot(dy) > radius {
            return None;
        }
        let scale = self.scale();
        let total = self.total_scaled(scale);
        if total <= 0.0 {
            return None;
        }
        let angle = dy.atan2(dx).rem_euclid(std::f64::consts::TAU);
        let position = angle / std::f64::consts::TAU * total;
        let mut cumulative = 0.0;
        self.slices.iter().position(|slice| {
            cumulative += slice.value / scale;
            slice.value > 0.0 && position < cumulative
        })
    }

    fn paint_hover(&self, buf: &mut Buffer, theme: &Theme) {
        let Some((index, pointer_x, pointer_y)) = self.hovered else {
            return;
        };
        let Some(slice) = self.slices.get(index) else {
            return;
        };
        let scale = self.scale();
        let total = self.total_scaled(scale);
        if total <= 0.0 {
            return;
        }
        let percent = slice.value / scale / total * 100.0;
        let text = format!(
            "{} · {} · {:.1}%",
            slice.label,
            format_value(slice.value),
            percent
        );
        let text_width = UnicodeWidthStr::width(text.as_str());
        let width = u16::try_from(text_width.saturating_add(2))
            .unwrap_or(u16::MAX)
            .min(self.state.area.width);
        if width < 6 {
            return;
        }
        let area = self.state.area;
        let x = pointer_x;
        let y = pointer_y;
        let right = area.right();
        let tooltip_x = if x.saturating_add(width).saturating_add(1) < right {
            x.saturating_add(1)
        } else {
            x.saturating_sub(width)
        }
        .clamp(area.x, right.saturating_sub(width));
        let tooltip_y = if y.saturating_add(1) < area.bottom() {
            y.saturating_add(1)
        } else {
            y.saturating_sub(1).max(area.y)
        };
        let tooltip = Rect::new(tooltip_x, tooltip_y, width, 1);
        buf.set_style(tooltip, Style::default().bg(theme.secondary));
        buf.set_stringn(
            tooltip_x.saturating_add(1),
            tooltip_y,
            &text,
            usize::from(width.saturating_sub(2)),
            Style::default().fg(theme.foreground).bg(theme.secondary),
        );
    }

    fn paint_circle(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if self.presentation == RenderStyle::Braille {
            self.paint_braille_circle(area, buf);
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

    fn paint_braille_circle(&self, area: Rect, buf: &mut Buffer) {
        let diameter = f64::from(area.width.min(area.height.saturating_mul(2)));
        let radius = diameter / 2.0;
        let center_x = f64::from(area.x) + f64::from(area.width) / 2.0;
        let center_y = f64::from(area.y) * 2.0 + f64::from(area.height);
        let density = usize::from(self.tuning.density.clamp(2, 6));
        let size_bonus = match self.tuning.dotsize {
            DotSize::Small => 0,
            DotSize::Medium => 1,
            DotSize::Large => 2,
        };
        let active_dots = (density + 2 + size_bonus).min(BRAILLE_ORDER.len());

        for y in 0..area.height {
            for x in 0..area.width {
                let mut dot_colors = [None; 8];
                for (dot_y, row) in BRAILLE_DOTS.iter().enumerate() {
                    for (dot_x, dot) in row.iter().enumerate() {
                        let pixel_x = f64::from(area.x + x) + (dot_x as f64 + 0.5) / 2.0;
                        let pixel_y = f64::from(area.y + y) * 2.0 + (dot_y as f64 + 0.5) / 2.0;
                        dot_colors[usize::from(*dot)] =
                            self.halftone_dot(pixel_x - center_x, pixel_y - center_y, radius);
                    }
                }

                let Some(color) = dominant_color(&dot_colors) else {
                    continue;
                };
                let cell_x = usize::from(area.x + x);
                let cell_y = usize::from(area.y + y);
                let phase = (cell_x + cell_y * 3) % BRAILLE_ORDER.len();
                let mut mask = 0_u8;
                for (dot, dot_color) in dot_colors.iter().enumerate() {
                    if *dot_color != Some(color) {
                        continue;
                    }
                    let rank = BRAILLE_RANK[dot];
                    if (rank + phase) % BRAILLE_ORDER.len() < active_dots {
                        mask |= 1 << dot;
                    }
                }
                if mask != 0 {
                    let glyph = char::from_u32(0x2800 + u32::from(mask)).unwrap_or(' ');
                    buf.set_string(
                        area.x + x,
                        area.y + y,
                        glyph.to_string(),
                        Style::default().fg(color),
                    );
                }
            }
        }
    }

    fn halftone_dot(&self, dx: f64, dy: f64, radius: f64) -> Option<Color> {
        let distance = dx.hypot(dy);
        if distance > radius || radius == 0.0 {
            return None;
        }
        let angle = dy.atan2(dx).rem_euclid(std::f64::consts::TAU);
        if self.near_slice_edge(angle, radius) {
            return None;
        }
        let radial = (distance / radius).clamp(0.0, 1.0) as f32;
        let intensity = if radial < 0.78 {
            1.0
        } else {
            ((1.0 - radial) / 0.22).clamp(0.0, 1.0)
        };
        (RenderStyle::Braille.shade(intensity, &self.tuning) != " ")
            .then(|| self.color_at(angle))
            .flatten()
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

fn dominant_color(colors: &[Option<Color>; 8]) -> Option<Color> {
    let mut selected = None;
    let mut selected_count = 0;
    for color in colors.iter().flatten().copied() {
        let count = colors
            .iter()
            .filter(|candidate| **candidate == Some(color))
            .count();
        if count > selected_count {
            selected = Some(color);
            selected_count = count;
        }
    }
    selected
}
