//! terminalui-animations: easings and buffer effects for terminalui.
//!
//! Two layers:
//!
//! - [`Animation`]: a finite, easing-driven timer. Feed it a `delta` each
//!   frame; it reports eased progress in `[0.0, 1.0]` and reports completion
//!   through [`Animation::is_done`].
//! - [`Effect`] / [`render_with`]: buffer effects (slide, bounce, fade,
//!   reveal, thanos dust, dissolve, fall, flare, radial flare, bidirectional
//!   flare) that paint your component into a scratch buffer and composite it
//!   with the effect applied.
//!
//! An [`Effect`] does not own time. Drive it with an [`Animation`] you
//! construct, tick and restart; the effect only owns what the cells look
//! like for a given progress.
//!
//! # Example
//!
//! ```rust
//! use std::time::Duration;
//! use ratatui::layout::Rect;
//! use ratatui::buffer::Buffer;
//! use ratatui::style::Style;
//! use terminalui_core::theme::Theme;
//! use terminalui_animations::{Animation, Easing, Effect, render_with};
//!
//! let area = Rect::new(0, 0, 20, 4);
//! let mut buffer = Buffer::empty(area);
//! let theme = Theme::default();
//! let mut anim = Animation::new(Duration::from_millis(600), Easing::EaseOutCubic);
//! anim.tick(Duration::from_millis(300));
//!
//! render_with(
//!     &Effect::FadeIn,
//!     anim.progress(),
//!     &mut buffer,
//!     area,
//!     &theme,
//!     |_area, buf, _theme| { buf.set_string(0, 0, "hello", Style::default()); },
//! );
//! ```

pub mod animation;
pub mod easing;
pub mod effects;

pub use animation::Animation;
pub use easing::{ease_out_bounce, Easing};
pub use effects::{lerp_color, render_with, Direction, Effect, RadialMode};
