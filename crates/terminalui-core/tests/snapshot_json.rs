//! JSON shape of the serialized tree: exact field layout, typed tags and
//! the custom constraint encoding.

use ratatui::layout::Constraint;
use serde_json::json;

use terminalui_core::outline::quoted;
use terminalui_core::{node, Stack, Text, Tree};

#[test]
fn tree_serializes_to_the_documented_shape() {
    let tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("label", Text::new("OK"))]),
    ));
    let value = serde_json::from_str::<serde_json::Value>(&tree.to_json().expect("serialize"))
        .expect("valid json");

    assert_eq!(
        value,
        json!({
            "focus": null,
            "root": {
                "hidden": false,
                "id": "root",
                "kind": "stack",
                "view": {
                    "children": [{
                        "hidden": false,
                        "id": "label",
                        "kind": "text",
                        "view": {
                            "content": "OK",
                            "style": {
                                "foreground": null,
                                "background": null,
                                "bold": false,
                                "italic": false,
                                "underline": false,
                            },
                        },
                    }],
                    "direction": "vertical",
                    "gap": 0,
                    "constraints": [],
                },
            },
        })
    );
}

#[test]
fn pretty_json_contains_newlines() {
    let tree = Tree::new(node("root", Text::new("x")));
    let pretty = tree.to_json_pretty().expect("pretty json");
    assert!(pretty.contains('\n'));
}

#[test]
fn constraints_serialize_with_variant_tags() {
    let tree = Tree::new(node(
        "root",
        Stack::vertical()
            .constraints(vec![
                Constraint::Min(3),
                Constraint::Max(9),
                Constraint::Length(5),
                Constraint::Percentage(50),
                Constraint::Ratio(1, 2),
                Constraint::Fill(1),
            ])
            .children(vec![]),
    ));
    let value = serde_json::from_str::<serde_json::Value>(&tree.to_json().expect("serialize"))
        .expect("valid json");

    assert_eq!(
        value["root"]["view"]["constraints"],
        json!([
            { "min": 3 },
            { "max": 9 },
            { "length": 5 },
            { "percentage": 50 },
            { "ratio": [1, 2] },
            { "fill": 1 },
        ])
    );
}

#[test]
fn quoted_values_escape_control_characters_and_quotes() {
    assert_eq!(quoted("plain"), "'plain'");
    assert_eq!(quoted("it's"), "'it\\'s'");
    assert_eq!(quoted("back\\slash"), "'back\\\\slash'");
    assert_eq!(quoted("line\nbreak"), "'line\\nbreak'");
    assert_eq!(quoted("carriage\rreturn"), "'carriage\\rreturn'");
    assert_eq!(quoted("tab\there"), "'tab\\there'");
}

#[test]
fn quoted_values_truncate_past_the_limit() {
    let long = quoted(&"a".repeat(60));
    assert!(long.len() < 60);
    assert!(long.ends_with("...'"));
}
