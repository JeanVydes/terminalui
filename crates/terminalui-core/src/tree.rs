//! The root of a rendered view tree.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::context::Context;
use crate::input::InputEvent;
use crate::node::Node;
use crate::outcome::Outcome;
use crate::outline::tree_outline;
use crate::theme::Theme;
use crate::view::View;

/// Rendered view tree. Owns the root view and the tracked focus slot.
pub struct Tree {
    root: Node,
    focus: Option<usize>,
}

impl Tree {
    /// Creates a tree from a root [`Node`].
    pub fn new(root: Node) -> Self {
        Self { root, focus: None }
    }

    /// Render the tree into `buf` inside `area`.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let mut cx = Context::new(self.focus);
        self.root.view.render(area, buf, &mut cx, theme);
    }

    /// Feed an input event into the tree. Tab/BackTab move focus. Anything
    /// else is delivered to every view with focus counting identical to render.
    pub fn handle_event(&mut self, event: &InputEvent) -> Outcome {
        if let InputEvent::Key(key) = event {
            use crossterm::event::KeyCode as Code;
            match key.code {
                Code::Tab => {
                    self.focus_next();
                    return Outcome::Consumed;
                }
                Code::BackTab => {
                    self.focus_previous();
                    return Outcome::Consumed;
                }
                _ => {}
            }
        }
        let mut cx = Context::new(self.focus);
        dispatch(&mut self.root, event, &mut cx)
    }

    /// Advance time-based state on every view.
    pub fn tick(&mut self, delta: Duration) {
        let mut cx = Context::new(self.focus);
        self.root.view.tick(delta, &mut cx);
    }

    /// Ids of focusable views in preorder.
    pub fn focusable_ids(&self) -> Vec<&str> {
        let mut out = Vec::new();
        collect_focusable(&self.root, &mut out);
        out
    }

    /// Every view in the tree that matches both the id and type, in preorder.
    ///
    /// Ids are not required to be unique: several nodes may share an id, and
    /// this is how to reach all of them. Pair with [`Tree::visit_mut`] when a
    /// mutable traversal is needed.
    pub fn find_all<T: View>(&self, id: &str) -> Vec<&T> {
        let mut out = Vec::new();
        collect_all(&self.root, id, &mut out);
        out
    }

    /// Visit every node in the tree, preorder, read-only.
    pub fn visit(&self, mut f: impl FnMut(&Node)) {
        walk(&self.root, &mut f);
    }

    /// Visit every node in the tree, preorder, mutable. The escape hatch for
    /// operations that do not fit [`Tree::find`]/[`Tree::find_mut`]/[`Tree::find_all`],
    /// e.g. disabling all buttons at once:
    ///
    /// ```ignore
    /// tree.visit_mut(|node| {
    ///     if let Some(button) = node.view.as_any_mut().downcast_mut::<Button>() {
    ///         button.set_disabled(true);
    ///     }
    /// });
    /// ```
    pub fn visit_mut(&mut self, mut f: impl FnMut(&mut Node)) {
        walk_mut(&mut self.root, &mut f);
    }

    /// Currently focused view, by id.
    pub fn focused_id(&self) -> Option<&str> {
        self.focus
            .and_then(|index| find_focusable_id(&self.root, index, &mut 0usize))
    }

    /// Focus the view with this id. Returns true when it exists and is focusable.
    pub fn focus(&mut self, id: &str) -> bool {
        let mut counter = 0usize;
        if let Some(index) = find_focusable_index(&self.root, id, &mut counter) {
            self.focus = Some(index);
            true
        } else {
            false
        }
    }

    /// Move focus to the next focusable view (wraps around).
    pub fn focus_next(&mut self) -> bool {
        let count = count_focusable(&self.root);
        if count == 0 {
            return false;
        }
        let next = self.focus.map(|index| (index + 1) % count).unwrap_or(0);
        self.focus = Some(next);
        true
    }

    /// Move focus to the previous focusable view (wraps around).
    pub fn focus_previous(&mut self) -> bool {
        let count = count_focusable(&self.root);
        if count == 0 {
            return false;
        }
        let previous = self
            .focus
            .map(|index| if index == 0 { count - 1 } else { index - 1 })
            .unwrap_or(count - 1);
        self.focus = Some(previous);
        true
    }

    /// Activate a view programmatically by id. Returns true when it produced an action.
    pub fn press(&mut self, id: &str) -> bool {
        let mut counter = 0usize;
        let Some(index) = find_focusable_index(&self.root, id, &mut counter) else {
            return false;
        };
        self.focus = Some(index);
        let mut rank = 0usize;
        match find_focusable_mut_at(&mut self.root, index, &mut rank) {
            Some(node) => node.view.activate(&Context::new(self.focus)),
            None => false,
        }
    }

    /// Typed query for a view by id.
    pub fn find<T: View>(&self, id: &str) -> Option<&T> {
        let node = find_node_ref(&self.root, id)?;
        node.view.as_any_ref().downcast_ref::<T>()
    }

    /// Mutable typed query for a view by id.
    pub fn find_mut<T: View>(&mut self, id: &str) -> Option<&mut T> {
        let node = find_node_mut(&mut self.root, id)?;
        node.view.as_any().downcast_mut::<T>()
    }

    /// Machine-readable full state of the tree as compact JSON.
    ///
    /// Serializes the real views, so every public and private field a view
    /// owns is included.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.serializable())
    }

    /// Machine-readable full state of the tree as indented JSON.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.serializable())
    }

    /// Compact one-line-per-node outline of the tree.
    ///
    /// Cheaper to read than [`Tree::to_json`] when the caller only needs
    /// structure, ids, kinds and a summary of state.
    pub fn inspect_outline(&self) -> String {
        tree_outline(&self.root, self.focused_id())
    }

    fn serializable(&self) -> SerializableTree<'_> {
        SerializableTree {
            focus: self.focused_id(),
            root: &self.root,
        }
    }
}

#[derive(serde::Serialize)]
struct SerializableTree<'a> {
    focus: Option<&'a str>,
    root: &'a Node,
}

fn count_focusable(node: &Node) -> usize {
    if node.hidden {
        return 0;
    }
    usize::from(node.view.focusable())
        + node
            .view
            .children()
            .iter()
            .map(count_focusable)
            .sum::<usize>()
}

fn collect_focusable<'a>(node: &'a Node, out: &mut Vec<&'a str>) {
    if node.hidden {
        return;
    }
    if node.view.focusable() {
        out.push(&node.id);
    }
    for child in node.view.children() {
        collect_focusable(child, out);
    }
}

fn find_focusable_mut_at<'a>(
    node: &'a mut Node,
    index: usize,
    counter: &mut usize,
) -> Option<&'a mut Node> {
    if node.hidden {
        return None;
    }
    if node.view.focusable() {
        if *counter == index {
            return Some(node);
        }
        *counter += 1;
    }
    for child in node.view.children_mut() {
        if let Some(found) = find_focusable_mut_at(child, index, counter) {
            return Some(found);
        }
    }
    None
}

fn collect_all<'a, T: View>(node: &'a Node, id: &str, out: &mut Vec<&'a T>) {
    if node.id == id {
        if let Some(view) = node.view.as_any_ref().downcast_ref::<T>() {
            out.push(view);
        }
    }
    for child in node.view.children() {
        collect_all(child, id, out);
    }
}

fn walk_mut(node: &mut Node, f: &mut dyn FnMut(&mut Node)) {
    f(node);
    for child in node.view.children_mut() {
        walk_mut(child, f);
    }
}

fn find_focusable_index(node: &Node, id: &str, counter: &mut usize) -> Option<usize> {
    if node.hidden {
        return None;
    }
    if node.view.focusable() {
        if node.id() == id {
            return Some(*counter);
        }
        *counter += 1;
    }
    for child in node.view.children() {
        if let Some(index) = find_focusable_index(child, id, counter) {
            return Some(index);
        }
    }
    None
}

fn find_focusable_id<'a>(node: &'a Node, index: usize, counter: &mut usize) -> Option<&'a str> {
    if node.hidden {
        return None;
    }
    if node.view.focusable() {
        if *counter == index {
            return Some(&node.id);
        }
        *counter += 1;
    }
    for child in node.view.children() {
        if let Some(found) = find_focusable_id(child, index, counter) {
            return Some(found);
        }
    }
    None
}

fn find_node_ref<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
    if node.id() == id {
        return Some(node);
    }
    for child in node.view.children() {
        if let Some(found) = find_node_ref(child, id) {
            return Some(found);
        }
    }
    None
}

fn find_node_mut<'a>(node: &'a mut Node, id: &str) -> Option<&'a mut Node> {
    if node.id() == id {
        return Some(node);
    }
    for child in node.view.children_mut() {
        if let Some(found) = find_node_mut(child, id) {
            return Some(found);
        }
    }
    None
}

fn walk(node: &Node, f: &mut dyn FnMut(&Node)) {
    f(node);
    for child in node.view.children() {
        walk(child, f);
    }
}

fn dispatch(node: &mut Node, event: &InputEvent, cx: &mut Context) -> Outcome {
    let own = node.view.handle_event(event, cx);
    let mut consumed = own == Outcome::Consumed;
    for child in node.view.children_mut() {
        if child.hidden {
            continue;
        }
        if dispatch(child, event, cx) == Outcome::Consumed {
            consumed = true;
        }
    }
    if consumed {
        Outcome::Consumed
    } else {
        Outcome::Ignored
    }
}
