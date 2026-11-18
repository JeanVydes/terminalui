//! terminalui: the whole toolkit in one crate.
//!
//! This is a facade. Everything public from the three crates that make up
//! the workspace is re-exported here, so a single dependency gives you
//! the complete toolkit:
//!
//! - [`terminalui-core`](terminalui_core): retained-mode view tree,
//!   focus management, layout engine and event dispatch. Re-exported
//!   at the top level.
//! - [`terminalui-animations`](terminalui_animations): easings and buffer
//!   effects (fade, slide, reveal, sweep, dissolve, radial flares).
//!   Re-exported at the top level.
//! - [`terminalui-components`](terminalui_components): ready-made components
//!   (buttons, inputs, selects, tables, charts, dialogs, menus and more).
//!   Available as [`components`], e.g. `terminalui::components::button::Button`.
//!
//! # Example
//!
//! ```rust
//! use terminalui::{node, Stack, Text, Tree};
//! use terminalui::components::button::{Button, ButtonVariant};
//!
//! let root = node("root", Stack::vertical().children(vec![
//!     node("title", Text::new("hello")),
//!     node("ok", Button::new(ButtonVariant::Primary, "OK")),
//! ]));
//!
//! let mut tree = Tree::new(root);
//! assert!(tree.find::<Button>("ok").is_some());
//! ```

pub use terminalui_animations::*;
pub use terminalui_core::*;

pub use terminalui_components as components;
