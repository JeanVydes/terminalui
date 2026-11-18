//! Animation primitives and buffer effects for terminalui.
//!
//! Two layers:
//!
//! - [`Animation`]: a finite, easing-driven timer. Feed it a `delta` each
//!   frame; it reports eased progress in `[0.0, 1.0]`.
//! - [`Effect`] / [`render_with`]: buffer effects (slide, bounce, fade,
//!   left-to-right reveal, thanos dust, dissolve, fall, reflective flare,
//!   radial flare, bidirectional flare) that paint your component into a
//!   scratch buffer and composite it with the effect applied.
//!
//! ```ignore
//! use std::time::Duration;
//! use terminalui_animations::{Animation, Effect, Direction};
//!
//! let mut anim = Animation::new(Duration::from_millis(600), terminalui_animations::Easing::EaseOutBounce);
//! anim.tick(delta);
//! terminalui_animations::render_with(
//!     &Effect::slide_bounce(Direction::Left, 12),
//!     anim.progress(),
//!     buf, area, theme,
//!     |_area, buf, theme| { /* render component into buf here */ },
//! );
//! ```

pub mod animation;
pub mod easing;
pub mod effects;

pub use animation::Animation;
pub use easing::{ease_out_bounce, Easing};
pub use effects::{lerp_color, render_with, Direction, Effect, RadialMode};
