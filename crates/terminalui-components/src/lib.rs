//! Ready-made components for terminalui.
//!
//! Every component is a self-contained view: it owns its geometry, its
//! hit-testing and its input mechanics and reports outcomes through typed
//! events. Nothing here makes decisions for you; the host wires component
//! state to meaning.
//!
//! # Example
//!
//! ```rust
//! use terminalui_core::{node, Stack, Text, Tree};
//! use terminalui_components::{button::{Button, ButtonVariant}, separator::Separator};
//!
//! let root = node("root", Stack::vertical().children(vec![
//!     node("title", Text::new("status")),
//!     node("sep", Separator::new()),
//!     node("ok", Button::new(ButtonVariant::Primary, "Save")),
//! ]));
//! let _tree = Tree::new(root);
//! ```
//!
//! # Customization
//!
//! Components use two layering tiers of configuration:
//!
//! - [`Theme`](terminalui_core::Theme) is the global palette. Pass your own
//!   [`Theme`](terminalui_core::Theme) to every render call by constructing
//!   the tree with it.
//! - Every component exposes an `XxxStyle`, all-`Option`, defaulting to the
//!   theme, plus a `.with_style(...)` builder for chained construction.
//!
//! # Serialization
//!
//! Every component serializes its full state to JSON through
//! [`Tree::to_json`](terminalui_core::Tree::to_json), including private
//! fields and composed children. [`Tree::inspect_outline`](terminalui_core::Tree::inspect_outline)
//! gives a compact one-line-per-node outline for agents and logs.

mod chart_empty;
pub mod chrome;
pub mod cursor;
mod layout;

pub mod autocomplete;
pub mod axis;
pub mod badge;
pub mod barchart;
pub mod boxplot;
pub mod breadcrumb;
pub mod button;
pub mod candlechart;
pub mod checkbox;
pub mod commandpalette;
pub mod console;
pub mod contextmenu;
pub mod dialog;
pub mod dropdown;
pub mod expandable;
pub mod floating;
pub mod form;
pub mod gauge;
pub mod grid;
pub mod icons;
pub mod input;
pub mod kbd;
pub mod linechart;
pub mod linefield;
pub mod list;
pub mod menu;
pub mod overlay;
pub mod pagination;
pub mod panel;
pub mod piechart;
pub mod popover;
pub mod presentation;
pub mod progress;
pub mod radio;
pub mod richtext;
pub mod scroll;
pub mod select;
pub mod separator;
pub mod slider;
pub mod sparkline;
pub mod spinner;
pub mod stat;
pub mod statusbar;
pub mod stepper;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod toast;
pub mod tooltip;
pub mod treeview;
