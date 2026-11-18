//! Normalized easing curves for animation timing.
//!
//! Every curve maps linear progress `t ∈ [0, 1]` onto eased progress in the
//! same range. Curves are pure and allocation-free.

use std::f64;

/// The easings shipped out of the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Easing {
    /// No easing.
    #[default]
    Linear,
    /// Accelerating from zero velocity (t²).
    EaseInQuad,
    /// Decelerating to zero velocity (t² mirror).
    EaseOutQuad,
    /// S-curve (t² mirror blend).
    EaseInOutQuad,
    /// Acceleration from zero velocity (t³).
    EaseInCubic,
    /// Deceleration to zero velocity (t³ mirror).
    EaseOutCubic,
    /// S-curve (t³ mirror blend).
    EaseInOutCubic,
    /// Acceleration from zero velocity (sine).
    EaseInSine,
    /// Deceleration to zero velocity (sine).
    EaseOutSine,
    /// S-curve (sine).
    EaseInOutSine,
    /// Strong accel at start.
    EaseInExpo,
    /// Strong decel at end.
    EaseOutExpo,
    /// S-curve (exponential).
    EaseInOutExpo,
    /// Overshoots past 1.0 before settling.
    EaseOutBack,
    /// Rubber-band: oscillates around 1.0 before settling.
    EaseOutElastic,
    /// Bounces on arrival.
    EaseOutBounce,
    /// Generic ease-in (alias of EaseInCubic).
    EaseIn,
    /// Generic ease-out (alias of EaseOutCubic).
    EaseOut,
    /// Generic ease-in-out (alias of EaseInOutCubic).
    EaseInOut,
}

impl Easing {
    /// Applies the easing to linear progress `t`. `t` is clamped to `[0, 1]`.
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseInSine => 1.0 - (t * f64::consts::FRAC_PI_2).cos(),
            Easing::EaseOutSine => (t * f64::consts::FRAC_PI_2).sin(),
            Easing::EaseInOutSine => -((t * f64::consts::PI).cos() - 1.0) / 2.0,
            Easing::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0f64.powf(10.0 * (t - 1.0))
                }
            }
            Easing::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0f64.powf(-10.0 * t)
                }
            }
            Easing::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Easing::EaseOutBack => {
                const C1: f64 = 1.70158;
                const C3: f64 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            Easing::EaseOutElastic => {
                const C4: f64 = (2.0 * f64::consts::PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2.0f64.powf(-10.0 * t) * (t * 10.0 - 0.75).sin() * C4 + 1.0
                }
            }
            Easing::EaseOutBounce => ease_out_bounce(t),
            Easing::EaseIn => Easing::EaseInCubic.apply(t),
            Easing::EaseOut => Easing::EaseOutCubic.apply(t),
            Easing::EaseInOut => Easing::EaseInOutCubic.apply(t),
        }
    }
}

/// Classic `easeOutBounce`.
pub fn ease_out_bounce(t: f64) -> f64 {
    const N1: f64 = 7.5625;
    const D1: f64 = 2.75;
    let t = t.clamp(0.0, 1.0);
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}
