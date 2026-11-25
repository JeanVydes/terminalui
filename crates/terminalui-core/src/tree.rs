//! The root of a rendered view tree.

use std::collections::HashMap;
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
    focus: Option<FocusTarget>,
}

/// Identity of a focusable node. Occurrence keeps duplicate host ids distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FocusTarget {
    id: String,
    occurrence: usize,
}

impl Tree {
    /// Creates a tree from a root [`Node`].
    pub fn new(root: Node) -> Self {
        Self { root, focus: None }
    }

    /// Render the tree into `buf` inside `area`.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let mut cx = Context::new(self.focus_index());
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
        let mut cx = Context::new(self.focus_index());
        dispatch(&mut self.root, event, &mut cx)
    }

    /// Advance time-based state on every view.
    pub fn tick(&mut self, delta: Duration) {
        let mut cx = Context::new(self.focus_index());
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
            .as_ref()
            .and_then(|target| focus_index_for(&self.root, target).map(|_| target.id.as_str()))
    }

    /// Focus the view with this id. Returns true when it exists and is focusable.
    pub fn focus(&mut self, id: &str) -> bool {
        if let Some(target) = first_focus_target(&self.root, id) {
            self.focus = Some(target);
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
        let next = self
            .focus_index()
            .map(|index| (index + 1) % count)
            .unwrap_or(0);
        self.focus = focus_target_at(&self.root, next);
        true
    }

    /// Move focus to the previous focusable view (wraps around).
    pub fn focus_previous(&mut self) -> bool {
        let count = count_focusable(&self.root);
        if count == 0 {
            return false;
        }
        let previous = self
            .focus_index()
            .map(|index| if index == 0 { count - 1 } else { index - 1 })
            .unwrap_or(count - 1);
        self.focus = focus_target_at(&self.root, previous);
        self.focus.is_some()
    }

    /// Activate a view programmatically by id. Returns true when it produced an action.
    pub fn press(&mut self, id: &str) -> bool {
        let Some(target) = first_focus_target(&self.root, id) else {
            return false;
        };
        self.focus = Some(target.clone());
        let Some(index) = focus_index_for(&self.root, &target) else {
            return false;
        };
        let mut rank = 0usize;
        match find_focusable_mut_at(&mut self.root, index, &mut rank) {
            Some(node) => node.view.activate(&Context::new(Some(index))),
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

    fn focus_index(&self) -> Option<usize> {
        self.focus
            .as_ref()
            .and_then(|target| focus_index_for(&self.root, target))
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

fn first_focus_target(node: &Node, id: &str) -> Option<FocusTarget> {
    find_focusable_target(node, id)
}

fn find_focusable_target(node: &Node, id: &str) -> Option<FocusTarget> {
    if node.hidden {
        return None;
    }
    if node.view.focusable() && node.id() == id {
        return Some(FocusTarget {
            id: id.to_owned(),
            occurrence: 0,
        });
    }
    for child in node.view.children() {
        if let Some(target) = find_focusable_target(child, id) {
            return Some(target);
        }
    }
    None
}

fn focus_index_for(node: &Node, target: &FocusTarget) -> Option<usize> {
    let mut counter = 0;
    let mut occurrence = 0;
    find_focus_index(node, target, &mut counter, &mut occurrence)
}

fn find_focus_index(
    node: &Node,
    target: &FocusTarget,
    counter: &mut usize,
    occurrence: &mut usize,
) -> Option<usize> {
    if node.hidden {
        return None;
    }
    if node.view.focusable() {
        if node.id == target.id {
            if *occurrence == target.occurrence {
                return Some(*counter);
            }
            *occurrence += 1;
        }
        *counter += 1;
    }
    for child in node.view.children() {
        if let Some(found) = find_focus_index(child, target, counter, occurrence) {
            return Some(found);
        }
    }
    None
}

fn focus_target_at(node: &Node, index: usize) -> Option<FocusTarget> {
    let mut counter = 0;
    let mut occurrences = HashMap::<String, usize>::new();
    target_at(node, index, &mut counter, &mut occurrences)
}

fn target_at(
    node: &Node,
    index: usize,
    counter: &mut usize,
    occurrences: &mut HashMap<String, usize>,
) -> Option<FocusTarget> {
    if node.hidden {
        return None;
    }
    if node.view.focusable() {
        if *counter == index {
            let occurrence = occurrences.entry(node.id.clone()).or_default();
            return Some(FocusTarget {
                id: node.id.clone(),
                occurrence: *occurrence,
            });
        }
        *counter += 1;
        *occurrences.entry(node.id.clone()).or_default() += 1;
    }
    for child in node.view.children() {
        if let Some(target) = target_at(child, index, counter, occurrences) {
            return Some(target);
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
