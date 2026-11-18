//! Compact outline output of the tree: one line per node, tree connectors,
//! compact state and escaped, truncated string values.

use std::any::Any;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Outcome, Spacer, Stack, Text, Tree, View};

#[derive(Serialize)]
struct Focusable;

#[derive(Serialize)]
struct Stateless;

impl View for Focusable {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "focusable"
    }

    fn render(&mut self, _area: Rect, _buf: &mut Buffer, _cx: &mut Context, _theme: &Theme) {}

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }
}

impl View for Stateless {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "panel"
    }

    fn render(&mut self, _area: Rect, _buf: &mut Buffer, _cx: &mut Context, _theme: &Theme) {}

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

#[test]
fn outline_nests_children_with_connectors() {
    let tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("first", Text::new("one")),
            node(
                "second",
                Stack::horizontal().children(vec![node("deep", Spacer)]),
            ),
            node("last", Text::new("three")),
        ]),
    ));

    assert_eq!(
        tree.inspect_outline(),
        concat!(
            "focus=none\n",
            "root [stack] {direction='vertical'}\n",
            "├─ first [text] {text='one'}\n",
            "├─ second [stack] {direction='horizontal'}\n",
            "│  └─ deep [spacer]\n",
            "└─ last [text] {text='three'}",
        )
    );
}

#[test]
fn outline_reports_the_focused_id() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("ok", Focusable)]),
    ));

    assert!(tree.inspect_outline().starts_with("focus=none\n"));
    assert!(tree.focus("ok"));
    assert!(tree.inspect_outline().starts_with("focus=ok\n"));
}

#[test]
fn outline_escapes_control_characters_and_quotes() {
    let tree = Tree::new(node("root", Text::new("it's \\ back\nnext\ttab")));

    assert_eq!(
        tree.inspect_outline(),
        concat!(
            "focus=none\n",
            "root [text] {text='it\\'s \\\\ back\\nnext\\ttab'}",
        )
    );
}

#[test]
fn outline_truncates_long_values() {
    let tree = Tree::new(node("root", Text::new("a".repeat(60))));
    let outline = tree.inspect_outline();
    let expected = format!("root [text] {{text='{}...'}}", "a".repeat(48));

    assert_eq!(outline, format!("focus=none\n{expected}"));
}

#[test]
fn outline_omits_state_braces_for_stateless_views() {
    let tree = Tree::new(node("root", Stateless));

    assert_eq!(tree.inspect_outline(), "focus=none\nroot [panel]");
}

#[test]
fn outline_omits_state_braces_for_unit_views() {
    let tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("gap", Spacer)]),
    ));
    let outline = tree.inspect_outline();
    assert_eq!(
        outline,
        concat!(
            "focus=none\n",
            "root [stack] {direction='vertical'}\n",
            "└─ gap [spacer]",
        )
    );
}
