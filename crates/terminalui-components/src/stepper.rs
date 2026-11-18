//! A horizontal step indicator (wizard / deploy stages). Steps are
//! host-provided data with one status each; the component owns the glyphs,
//! colors and connectors.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::icons::Icon;

/// Where a step is in the flow. The host drives transitions; this component
/// only renders them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StepStatus {
    /// The step has not run yet.
    #[default]
    Pending,
    /// The step is running.
    Active,
    /// The step finished successfully.
    Done,
    /// The step failed.
    Failed,
}

/// One step of a flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Step {
    /// The label shown next to the step glyph.
    pub label: String,
    /// Where the step is in the flow.
    pub status: StepStatus,
}

impl Step {
    /// Creates a step with the given label, initially `Pending`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: StepStatus::Pending,
        }
    }

    /// Builder shorthand: set the status.
    pub fn with_status(mut self, status: StepStatus) -> Self {
        self.status = status;
        self
    }

    fn glyph(&self) -> &'static str {
        match self.status {
            StepStatus::Pending => Icon::Circle.glyph(),
            StepStatus::Active => Icon::CircleFilled.glyph(),
            StepStatus::Done => Icon::Check.glyph(),
            StepStatus::Failed => Icon::X.glyph(),
        }
    }
}

/// Public state of the stepper: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepperState {
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// Per-instance overrides for a stepper. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StepperStyle {
    /// Pending step color. Defaults to `theme.muted`.
    pub pending: Option<Color>,
    /// Active step color. Defaults to `theme.accent`.
    pub active: Option<Color>,
    /// Done step color. Defaults to `theme.success`.
    pub done: Option<Color>,
    /// Failed step color. Defaults to `theme.destructive`.
    pub failed: Option<Color>,
    /// Connector line color. Defaults to `theme.border`.
    pub connector: Option<Color>,
}

/// The steps for one flow, in order, one row high.
#[derive(Serialize)]
pub struct Stepper {
    /// The steps, in order.
    pub steps: Vec<Step>,
    /// Public state: the last rendered area.
    pub state: StepperState,
    /// Per-instance overrides; defaults match the theme.
    pub style: StepperStyle,
}

impl Stepper {
    /// Creates a stepper with the given steps.
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            state: StepperState {
                area: Rect::new(0, 0, 0, 0),
            },
            style: StepperStyle::default(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: StepperStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the steps wholesale.
    pub fn set_steps(&mut self, steps: Vec<Step>) {
        self.steps = steps;
    }

    /// Index of the first non-`Done` step, if any.
    pub fn current(&self) -> Option<usize> {
        self.steps
            .iter()
            .position(|step| step.status != StepStatus::Done)
    }
}

impl Default for Stepper {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for Stepper {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "stepper"
    }

    fn outline(&self) -> Option<String> {
        let active = self
            .current()
            .map(|index| index.to_string())
            .unwrap_or_else(|| "none".to_string());
        Some(format!("steps={}, current={}", self.steps.len(), active))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let width: u16 = self
            .steps
            .iter()
            .map(|step| 2 + step.label.chars().count() as u16 + 3)
            .sum();
        Some((width, 1))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let pending = self.style.pending.unwrap_or(theme.muted);
        let active = self.style.active.unwrap_or(theme.accent);
        let done = self.style.done.unwrap_or(theme.success);
        let failed = self.style.failed.unwrap_or(theme.destructive);
        let connector = self.style.connector.unwrap_or(theme.border);
        let right = area.x + area.width;
        let mut x = area.x;
        let mut previous_status = StepStatus::Done;

        for (index, step) in self.steps.iter().enumerate() {
            let y = area.y;
            if index > 0 {
                let line_color = match previous_status {
                    StepStatus::Done => connector,
                    _ => theme.muted,
                };
                for _ in 0..3 {
                    if x >= right {
                        return;
                    }
                    buf.set_string(x, y, "─", Style::default().fg(line_color));
                    x += 1;
                }
                if x < right {
                    x += 1;
                }
            }
            let color = match step.status {
                StepStatus::Pending => pending,
                StepStatus::Active => active,
                StepStatus::Done => done,
                StepStatus::Failed => failed,
            };
            let mut style = Style::default().fg(color);
            if step.status == StepStatus::Active {
                style = style.add_modifier(Modifier::BOLD);
            }
            if x >= right {
                return;
            }
            buf.set_string(x, y, step.glyph(), style);
            if x + 1 < right {
                buf.set_string(x + 1, y, " ", style);
                x += 2;
            } else {
                return;
            }
            let remaining = right.saturating_sub(x) as usize;
            let (written, _) = buf.set_stringn(x, y, &step.label, remaining, style);
            x += written;
            if x < right {
                x += 1;
            }
            previous_status = step.status;
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
