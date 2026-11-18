//! The one and only node in the tree: an id plus a view.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Wraps any view in the tree together with the id the host knows it by.
///
/// The id is a `String` (the same shape as an HTML `id`); it is only used
/// for programmatic queries (`Tree::focus`, `Tree::press`, `Tree::find`, …),
/// never by the views themselves.
/// A node binds a [`View`](crate::View) to a host-chosen id.
pub struct Node {
    /// Host-chosen id. It may be shared by several nodes; queries filter by id and type.
    pub id: String,
    /// The view itself.
    pub view: Box<dyn crate::view::View>,
}

impl Node {
    /// The node's host-chosen id.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut node = serializer.serialize_struct("Node", 3)?;
        node.serialize_field("id", &self.id)?;
        node.serialize_field("kind", self.view.kind())?;
        node.serialize_field("view", &ErasedView(&*self.view))?;
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
    }
}
