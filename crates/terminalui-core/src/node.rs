//! The one and only node in the tree: an id plus a view.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// A node binds a view to a host-chosen id.
///
/// The id is used for programmatic queries (`Tree::focus`, `Tree::press`,
/// `Tree::find`), never by the views themselves. The id may be shared by
/// several nodes; queries filter by id and type.
pub struct Node {
    /// Host-chosen id. It may be shared by several nodes; queries filter by id and type.
    pub id: String,
    /// The view itself.
    pub view: Box<dyn crate::view::View>,
    /// Hidden nodes keep their state and stay queryable, but they get no
    /// events, no focus slot and no ticks. Containers set this to model
    /// visibility without unmounting their children.
    pub(crate) hidden: bool,
}

impl Node {
    /// The node's host-chosen id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Whether this node is excluded from rendering, focus and event walks.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Set visibility without unmounting the view or discarding its state.
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut node = serializer.serialize_struct("Node", 4)?;
        node.serialize_field("id", &self.id)?;
        node.serialize_field("kind", self.view.kind())?;
        node.serialize_field("view", &ErasedView(&*self.view))?;
        node.serialize_field("hidden", &self.hidden)?;
        node.end()
    }
}

struct ErasedView<'a>(&'a dyn crate::view::View);

impl Serialize for ErasedView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        erased_serde::serialize(self.0, serializer)
    }
}

/// Constructs a [`Node`] from an id and a view value.
///
/// The view is boxed and registered under the id; the tree builder and leaf
/// renderers consume it.
pub fn node<V: crate::view::View + 'static>(id: impl Into<String>, view: V) -> Node {
    Node {
        id: id.into(),
        view: Box::new(view),
        hidden: false,
    }
}
