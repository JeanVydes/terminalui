//! A tiny animation driver: a finite duration plus an easing curve.

use std::time::Duration;

use crate::easing::Easing;

/// A finite one-way animation.
///
/// `tick` is fed a delta each frame; the animation completes when
/// `elapsed >= duration`. `progress` reports eased progress in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Animation {
    duration: Duration,
    easing: Easing,
    elapsed: Duration,
}

impl Animation {
    /// Starts an animation at t = 0.
    pub fn new(duration: Duration, easing: Easing) -> Self {
        Self {
            duration: duration.max(Duration::from_millis(1)),
            easing,
            elapsed: Duration::ZERO,
        }
    }

    /// The easing curve the animation reports progress with.
    pub fn easing(&self) -> Easing {
        self.easing
    }

    /// Total duration of the animation.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Linear (un-eased) progress in `[0.0, 1.0]`.
    pub fn linear(&self) -> f64 {
        (self.elapsed.as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// Eased progress in `[0.0, 1.0]`.
    pub fn progress(&self) -> f64 {
        self.easing.apply(self.linear())
    }

    /// Whether the animation has completed.
    pub fn is_done(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Advances the animation by `delta`. Returns `true` if it is still running.
    pub fn tick(&mut self, delta: Duration) -> bool {
        if self.is_done() {
            return false;
        }
        self.elapsed = (self.elapsed + delta).min(self.duration);
        !self.is_done()
    }

    /// Restarts the animation from t = 0.
    pub fn restart(&mut self) {
        self.elapsed = Duration::ZERO;
    }
}
