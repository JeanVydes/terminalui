//! The single composable trait of the library.

use std::any::Any;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::context::Context;
use crate::input::InputEvent;
use crate::node::Node;
use crate::outcome::Outcome;
use crate::theme::Theme;

/// A composable view in the tree.
///
/// Implement this trait to add behavior to the tree. Views own their
/// geometry, input handling and focus semantics; the host drives rendering,
/// timing and the meaning of events.
///
/// Views are type-erased through `Any`. If you need to reach a concrete view
/// from outside the tree (e.g. in a test), use [`View::as_any_ref`] with a
/// typed query.
pub trait View: Any + erased_serde::Serialize {
    /// Type erasure hook. Required for typed queries through the tree.
    fn as_any(&mut self) -> &mut dyn Any;
    /// Read-only type erasure hook. Required for typed queries through the tree.
    fn as_any_ref(&self) -> &dyn Any;

    /// Stable tag used by the outline and JSON output.
    fn kind(&self) -> &'static str {
        "view"
    }

    /// Compact state shown by [`crate::Tree::inspect_outline`].
    ///
    /// Returns only the fields that help an agent understand the node at a
    /// glance. Full state lives in the serialized view.
    fn outline(&self) -> Option<String> {
        None
    }

    /// Intrinsic size in cells, if the view knows it. Width first. Used by
    /// composite views to lay out children; `None` means "fill what the
    /// parent gives me".
    fn measure(&self) -> Option<(u16, u16)> {
        None
    }

    /// Draw the view inside `area`. Focusable views increment the counter in `cx`.
    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme);

    /// Handle an input event. Return `Consumed` to stop bubbling. Focusable
    /// views increment the counter in `cx` and must do so in the same order
    /// as in `render`.
    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome;

    /// Advance view-internal state by `delta`. The default forwards the tick
    /// to every child, so containers only override when they own animation.
    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        for child in self.children_mut() {
            if child.hidden {
                continue;
            }
            child.view.tick(delta, cx);
        }
    }

    /// Children of this view, in declaration order.
    fn children(&self) -> &[Node] {
        &[]
    }

    /// Mutable children of this view, in declaration order.
    fn children_mut(&mut self) -> &mut [Node] {
        &mut []
    }

    /// Whether this view can hold keyboard focus.
    fn focusable(&self) -> bool {
        false
    }

    /// Programmatic activation (e.g. `tree.press(id)`). Defaults to `false`.
    /// Views may override to provide richer semantics.
    fn activate(&mut self, _cx: &Context) -> bool {
        false
    }
}
