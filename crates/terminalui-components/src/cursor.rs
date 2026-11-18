//! Cursor presentation for text inputs: shape, visibility mode, opacity and
//! animation curve. Everything is configurable; the defaults match the
//! classic steady block cursor.

use serde::Serialize;
use terminalui_animations::Easing;

/// Shape of the caret cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    /// Inverted cell over the character (classic block).
    #[default]
    Block,
    /// Underline drawn on the cell, keeping the character visible.
    Underline,
    /// Vertical bar (left eighth-block) before the character.
    Line,
}

/// Blink behavior of the caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CursorMode {
    /// No cycling: always fully visible.
    Off,
    /// Hard blink, alternating visible/invisible every half period.
    #[default]
    Blink,
    /// Smooth breathing fade through the configured easing curve.
    Pulse,
}

/// Per-instance configuration of the caret. All fields are settable; use
/// the `with_*` builders for ergonomic chaining.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CursorStyle {
    /// Shape of the caret cell.
    pub shape: CursorShape,
    /// Visibility behavior of the caret.
    pub mode: CursorMode,
    /// Half-period of the blink in milliseconds.
    pub blink_ms: u64,
    /// Caret opacity in `0..=1`. `1.0` fully opaque, `0.0` invisible.
    pub opacity: f64,
    /// Curve used by [`CursorMode::Pulse`]; ignored otherwise.
    pub easing: Easing,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Block,
            mode: CursorMode::Off,
            blink_ms: 530,
            opacity: 1.0,
            easing: Easing::EaseInOutSine,
        }
    }
}

impl CursorStyle {
    /// Builder shorthand: set the caret shape.
    pub fn with_shape(mut self, shape: CursorShape) -> Self {
        self.shape = shape;
        self
    }

    /// Builder shorthand: set the visibility mode.
    pub fn with_mode(mut self, mode: CursorMode) -> Self {
        self.mode = mode;
        self
    }

    /// Builder shorthand: set the blink half-period in milliseconds.
    pub fn with_blink_ms(mut self, blink_ms: u64) -> Self {
        self.blink_ms = blink_ms;
        self
    }

    /// Builder shorthand: set the opacity in `0..=1`.
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Builder shorthand: set the pulse easing curve.
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}
