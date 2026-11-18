use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::layout::{row, Align};
use super::presentation::{RenderStyle, StyleTuning};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProgressState {
    pub area: Rect,
    /// Eased value in 0.0..=1.0 for smooth rendering.
    pub value: f64,
    pub elapsed: Duration,
}

/// Per-instance overrides for a progress bar. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProgressStyle {
    /// Filled cell color. Defaults to `theme.primary`.
    pub fill: Option<Color>,
    /// Empty track color. Defaults to `theme.muted`.
    pub track: Option<Color>,
    /// Milliseconds between indeterminate sweep frames.
    pub interval_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct Progress {
    /// Target value in 0.0..=1.0. The bar animates toward this smoothly.
    pub value: f64,
    /// Indeterminate mode: a sweeping band instead of a fill.
    pub indeterminate: bool,
    /// Which glyph set draws the track. Defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    /// Shading knobs for non-`Native` presentations. Defaults to a 4-level
    /// halftone ramp at neutral contrast.
    pub tuning: StyleTuning,
    pub style: ProgressStyle,
    pub state: ProgressState,
    children: Vec<Node>,
}

impl Progress {
    /// Convenience constructor: the label is a single [`Text`] view.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.children = vec![node("label", Text::new(label))];
        self
    }
}
impl Progress {
    /// A determinate progress bar. `value` is in 0.0..=1.0.
    pub fn new(value: f64) -> Self {
        Self::bar(value, false)
    }

    /// An indeterminate (sweeping) progress bar.
    pub fn indeterminate_bar() -> Self {
        Self::bar(0.0, true)
    }

    fn bar(value: f64, indeterminate: bool) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            indeterminate,
            presentation: RenderStyle::Native,
            tuning: StyleTuning::default(),
            style: ProgressStyle::default(),
            state: ProgressState {
                area: Rect::new(0, 0, 0, 0),
                value: value.clamp(0.0, 1.0),
                elapsed: Duration::ZERO,
            },
            children: Vec::new(),
        }
    }

    pub fn with_value(mut self, value: f64) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }

    /// Compose a progress bar with an arbitrary view rendered before the bar.
    pub fn with_children(mut self, children: Vec<Node>) -> Self {
        self.children = children;
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

    pub fn with_style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }

    fn sweep_window(&self, width: u16) -> (u16, u16) {
        let interval = self.style.interval_ms.unwrap_or(60) as u128;
        let phase = (self.state.elapsed.as_millis() / interval.max(1)) % (width as u128 + 8);
        let center = phase as u16;
        let half = (width / 6).max(1);
        let start = center.saturating_sub(half);
        let end = (center + half).min(width);
        (start, end)
    }

    fn children_width(&self) -> u16 {
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum();
        if self.children.is_empty() {
            0
        } else {
            measured + 2
        }
    }
}

impl View for Progress {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "progress"
    }

    fn outline(&self) -> Option<String> {
        if self.indeterminate {
            Some("indeterminate=true".to_string())
        } else {
            Some(format!("value={}", self.value))
        }
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let fill = self.style.fill.unwrap_or(theme.primary);
        let track = self.style.track.unwrap_or(theme.muted);
        let full = self.presentation.filled();
        let empty = self.presentation.empty();
        let shaded = self.presentation != RenderStyle::Native;
        let tuning = self.tuning;
        let y = area.y;

        let label_width = self.children_width().min(area.width);
        if !self.children.is_empty() {
            let label_area = Rect::new(area.x, y, label_width.saturating_sub(2), 1);
            row(
                &mut self.children,
                label_area,
                Align::Left,
                1,
                buf,
                cx,
                theme,
            );
        }

        let bar_x = area.x + label_width;
        let bar_width = area.width.saturating_sub(label_width);
        if bar_width == 0 {
            return;
        }
        let filled = (self.state.value * bar_width as f64).round() as u16;
        for offset in 0..bar_width {
            let x = bar_x + offset;
            let is_filled = if self.indeterminate {
                let (start, end) = self.sweep_window(bar_width);
                offset >= start && offset < end
            } else {
                offset < filled
            };
            if shaded {
                let frac = if self.indeterminate {
                    if is_filled {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    (self.state.value * bar_width as f64) - offset as f64
                }
                .clamp(0.0, 1.0) as f32;
                if frac <= 0.0 {
                    buf.set_string(x, y, empty, Style::default().fg(track));
                } else {
                    let glyph = self.presentation.shade(frac, &tuning);
                    buf.set_string(x, y, glyph, Style::default().fg(fill));
                }
            } else {
                let style = if is_filled {
                    Style::default().fg(fill).bg(fill)
                } else {
                    Style::default().fg(track).bg(track)
                };
                buf.set_string(x, y, if is_filled { full } else { empty }, style);
            }
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        self.state.elapsed += delta;
        let target = self.value.clamp(0.0, 1.0);
        if (self.state.value - target).abs() > 0.0001 {
            let easing = 0.18;
            self.state.value += (target - self.state.value) * easing;
        } else {
            self.state.value = target;
        }
        for child in &mut self.children {
            child.view.tick(delta, cx);
        }
    }
}
