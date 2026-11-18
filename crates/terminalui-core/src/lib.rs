//! terminalui: a terminal UI library built around a composable view tree.
//!
//! The root entry point is [`Tree`]. Build a tree of [`Node`]s wrapping
//! any [`View`], render into a ratatui buffer, and feed input events.
//!
//! ```ignore
//! use terminalui_core::{node, Node, Stack, Theme, Tree, Text};
//!
//! let root = node("root", Stack::vertical()
//!     .gap(1)
//!     .children(vec![
//!         node("title", Text::new("Hello")),
//!         node("ok", Button::new("OK")),
//!     ]));
//! let mut tree = Tree::new(root);
//! ```

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

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {}
}
