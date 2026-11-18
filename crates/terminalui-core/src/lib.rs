//! terminalui-core: retained-mode view tree for the terminal.
//!
//! [`Tree`] is the root object. A tree is a nested structure of [`Node`]s,
//! each wrapping a [`View`]. Views own their own geometry, focus semantics
//! and input handling; the host owns rendering, timing and the meaning of
//! events.
//!
//! A tree renders into a ratatui buffer and handles input events. Nothing
//! is painted outside of `Tree::render` and no event escapes unhandled
//! bubbling through `Tree::handle_event`.
//!
//! # Example
//!
//! Build a tree, find a node, check focus semantics:
//!
//! ```rust
//! use terminalui_core::{node, Stack, Text, Tree};
//!
//! let root = node("root", Stack::vertical().children(vec![
//!     node("title", Text::new("do not repeat me")),
//!     node("again", Text::new("up up down down")),
//! ]));
//!
//! let mut tree = Tree::new(root);
//! assert!(tree.find::<Text>("title").is_some());
//! assert!(tree.find::<Text>("body").is_none());
//! ```
//!
//! # Programmatic control
//!
//! Views do not expose side-channel APIs. The host drives behavior through
//! the tree:
//!
//! - [`Tree::find`] and [`Tree::find_all`] locate nodes by id and type.
//! - [`Tree::focus`] moves keyboard focus in pre-order.
//! - [`Tree::press`] activates a view by id.
//! - [`Tree::visit_mut`] applies a closure to every node sharing an id.
//! - [`Tree::tick`] advances time-based components (spinners, toasts,
//!   progress bars, charts) by a delta.
//!
//! # Serialization
//!
//! [`Tree::to_json`] and [`Tree::to_json_pretty`] produce a complete machine
//! readable representation of the tree: every public and private field a
//! view owns is serialized, children included. [`Tree::inspect_outline`]
//! produces a compact tree-shaped outline for agents and logs.

pub mod context;
pub mod input;
mod node;
mod outcome;
pub mod outline;
mod spacer;
mod stack;
mod text;
pub mod theme;
mod tree;
mod view;

pub use context::Context;
pub use input::InputEvent;
pub use node::{node, Node};
pub use outcome::Outcome;
pub use spacer::Spacer;
pub use stack::{Stack, StackDirection};
pub use text::{Text, TextStyle};
pub use theme::Theme;
pub use tree::Tree;
pub use view::View;
