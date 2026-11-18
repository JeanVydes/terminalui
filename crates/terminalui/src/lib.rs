//! terminalui: the whole toolkit in one crate.
//!
//! This is a facade. It re-exports the crates that make up the workspace so
//! downstream users can depend on a single package:
//!
//! - [`terminalui_core`]: the retained view tree, focus management,
//!   layout and event dispatch, re-exported at the top level.
//! - [`terminalui_animations`]: easings and buffer effects,
//!   re-exported at the top level.
//! - [`terminalui_components`]: ready-made components, available as
//!   [`components`], e.g. `terminalui::components::button::Button`.
//!
//! ```
//! use terminalui::{node, Stack, Text, Tree};
//! use terminalui::components::button::{Button, ButtonVariant};
//!
//! let root = node("root", Stack::vertical().children(vec![
//!     node("ok", Button::new(ButtonVariant::Primary, "OK")),
//! ]));
//! let _tree = Tree::new(root);
//! ```

pub use terminalui_animations::*;
pub use terminalui_core::*;

pub use terminalui_components as components;
