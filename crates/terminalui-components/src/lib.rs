//! Copyable components for terminalui.
//!
//! Every component is a plain view you can drop in any terminalui tree;
//! no code generation, no CLI, no copying step.
//!
//! Customization:
//!
//! - `terminalui_core::theme::Theme` is the global palette; pass your own instance
//!   to every render call to reskin an entire app.
//! - Every component exposes a per-instance `style` field
//!   (`XxxStyle`, all-`Option` fields defaulting to the theme) plus a
//!   `.with_style(...)` builder for chained construction.

pub mod chrome;
pub mod cursor;
mod layout;

pub mod autocomplete;
pub mod badge;
pub mod barchart;
pub mod boxplot;
pub mod breadcrumb;
pub mod button;
pub mod candlechart;
pub mod checkbox;
pub mod commandpalette;
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
mod linefield;
pub mod list;
pub mod menu;
pub mod overlay;
pub mod pagination;
pub mod panel;
pub mod popover;
pub mod presentation;
pub mod progress;
pub mod radio;
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
