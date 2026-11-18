//! JSON shape of the tree: unquoted ids, nested children, stable kinds and
//! the full serialized view state.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Outcome, Stack, Text, Tree, View};

#[derive(Default, Serialize)]
struct FocusProbe;

impl View for FocusProbe {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn kind(&self) -> &'static str {
        "focus_probe"
    }

    fn render(&mut self, _area: Rect, _buf: &mut Buffer, cx: &mut Context, _theme: &Theme) {
        let _ = cx.is_focused();
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }
}

fn sample_tree() -> Tree {
    Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("probe", FocusProbe),
            node(
                "body",
                Stack::horizontal().children(vec![
                    node("dup", Text::new("first")),
                    node("dup", Text::new("second")),
                ]),
            ),
        ]),
    ))
}

fn json(tree: &Tree) -> serde_json::Value {
    let text = tree.to_json().expect("serialization succeeds");
    serde_json::from_str(&text).expect("output is valid JSON")
}

fn children(node: &serde_json::Value) -> &[serde_json::Value] {
    node["view"]["children"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn collect_ids<'a>(node: &'a serde_json::Value, out: &mut Vec<&'a str>) {
    out.push(node["id"].as_str().expect("id is a string"));
    for child in children(node) {
        collect_ids(child, out);
    }
}

fn find_by_id<'a>(node: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    if node["id"] == id {
        return Some(node);
    }
    children(node)
        .iter()
        .find_map(|child| find_by_id(child, id))
}

#[test]
fn ids_and_focus_are_plain_without_quotes() {
    let mut tree = sample_tree();
    assert!(tree.focus("probe"));
    let value = json(&tree);
    assert_eq!(value["focus"], "probe");
    let mut ids = Vec::new();
    collect_ids(&value["root"], &mut ids);
    assert_eq!(ids, ["root", "probe", "body", "dup", "dup"]);
}

#[test]
fn children_are_nested_and_duplicate_ids_stay_distinct() {
    let value = json(&sample_tree());
    let root = &value["root"];
    assert_eq!(root["id"], "root");
    let root_children = children(root);
    assert_eq!(root_children.len(), 2);
    assert_eq!(root_children[0]["id"], "probe");

    let body = &root_children[1];
    assert_eq!(body["id"], "body");
    assert_eq!(body["view"]["direction"], "horizontal");
    let body_children = children(body);
    assert_eq!(body_children.len(), 2);
    assert_eq!(body_children[0]["view"]["content"], "first");
    assert_eq!(body_children[1]["view"]["content"], "second");

    let first = find_by_id(root, "dup").expect("duplicate id resolves");
    assert_eq!(first["view"]["content"], "first");
}

#[test]
fn kinds_are_stable_tags() {
    let value = json(&sample_tree());
    assert_eq!(value["root"]["kind"], "stack");
    assert_eq!(
        find_by_id(&value["root"], "body").map(|node| node["kind"].as_str()),
        Some(Some("stack"))
    );
    assert_eq!(
        find_by_id(&value["root"], "dup").map(|node| node["kind"].as_str()),
        Some(Some("text"))
    );
    assert_eq!(find_by_id(&value["root"], "missing"), None);
}

#[test]
fn serialization_never_fails_on_composite_trees() {
    let tree = sample_tree();
    assert!(tree.to_json().is_ok());
    assert!(tree.to_json_pretty().is_ok());
    let pretty = tree.to_json_pretty().expect("pretty output");
    assert!(pretty.contains('\n'));
}
