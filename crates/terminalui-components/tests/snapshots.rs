//! Composed views serialize their own state: every field a view owns shows
//! up in the JSON tree, private ones included, children kept as real nodes.

use std::time::Duration;

use serde_json::Value;

use terminalui_core::{node, Stack, Text, Tree};
use terminalui_components::button::{Button, ButtonVariant};
use terminalui_components::checkbox::Checkbox;
use terminalui_components::form::FormField;
use terminalui_components::input::Input;
use terminalui_components::list::List;
use terminalui_components::select::Select;
use terminalui_components::slider::Slider;
use terminalui_components::table::{Table, TableCell, TableRow};
use terminalui_components::toast::{Toast, ToastKind};

fn tree() -> Tree {
    let mut button = Button::new(ButtonVariant::Primary, "Save");
    button.set_disabled(true);
    button.set_loading(true);

    let mut checkbox = Checkbox::with_label("Remember");
    checkbox.state.checked = true;
    checkbox.state.disabled = true;

    let mut input = Input::new().with_placeholder("search");
    input.set_value("abc");
    input.state.disabled = true;

    let mut slider = Slider::new(0.0, 10.0, 1.0);
    slider.set_value(4.0);
    slider.set_disabled(true);

    let mut select = Select::new(vec![node("a", Text::new("alpha"))]).with_selected(0);
    select.state.disabled = true;

    let mut list = List::with_items(vec!["a", "b", "c"]);
    list.state.selected = 2;
    list.state.offset = 1;

    let mut table = Table::new();
    table.set_rows(vec![
        node(
            "row-0",
            TableRow::body(vec![node("cell-0", TableCell::text("one"))]),
        ),
        node(
            "row-1",
            TableRow::body(vec![node("cell-1", TableCell::text("two"))]),
        ),
    ]);

    let mut field = FormField::text("email", "Email");
    field.set_error(Some("required"));

    Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("button", button),
            node("checkbox", checkbox),
            node("input", input),
            node("slider", slider),
            node("select", select),
            node("list", list),
            node("table", table),
            node("field", field),
            node(
                "toast",
                Toast::new(ToastKind::Info, "saved").with_ttl(Duration::ZERO),
            ),
        ]),
    ))
}

fn json(tree: &Tree) -> Value {
    let text = tree.to_json().expect("serialization succeeds");
    serde_json::from_str(&text).expect("output is valid JSON")
}

fn children(node: &Value) -> &[Value] {
    node["view"]["children"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn find<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if node["id"] == id {
        return Some(node);
    }
    children(node).iter().find_map(|child| find(child, id))
}

#[test]
fn control_state_is_serialized_verbatim() {
    let value = json(&tree());
    let root = &value["root"];

    let button = find(root, "button").expect("button node");
    assert_eq!(button["kind"], "button");
    assert_eq!(button["view"]["variant"], "primary");
    assert_eq!(button["view"]["state"]["pressed"], false);
    assert_eq!(button["view"]["state"]["disabled"], true);
    assert_eq!(button["view"]["state"]["loading"], true);

    let checkbox = find(root, "checkbox").expect("checkbox node");
    assert_eq!(checkbox["view"]["state"]["checked"], true);
    assert_eq!(checkbox["view"]["state"]["disabled"], true);

    let input = find(root, "input").expect("input node");
    assert_eq!(input["view"]["state"]["value"], "abc");
    assert_eq!(input["view"]["state"]["cursor"], 3);
    assert_eq!(input["view"]["state"]["disabled"], true);

    let slider = find(root, "slider").expect("slider node");
    assert_eq!(slider["view"]["value"], 4.0);
    assert_eq!(slider["view"]["min"], 0.0);
    assert_eq!(slider["view"]["max"], 10.0);
    assert_eq!(slider["view"]["state"]["disabled"], true);

    let select = find(root, "select").expect("select node");
    assert_eq!(select["view"]["selected"], 0);
    assert_eq!(select["view"]["state"]["open"], false);
    assert_eq!(select["view"]["state"]["disabled"], true);
}

#[test]
fn containers_keep_indices_and_children() {
    let value = json(&tree());
    let root = &value["root"];

    let list = find(root, "list").expect("list node");
    assert_eq!(list["view"]["state"]["selected"], 2);
    assert_eq!(list["view"]["state"]["offset"], 1);

    let table = find(root, "table").expect("table node");
    assert_eq!(children(table).len(), 2);
    assert_eq!(table["view"]["state"]["selected"], 0);

    let field = find(root, "field").expect("field node");
    assert_eq!(field["view"]["name"], "email");
    assert_eq!(field["view"]["required"], false);
    let error = children(field)
        .iter()
        .find(|child| child["id"] == "error")
        .expect("error row is a composed child");
    assert_eq!(error["kind"], "text");

    let toast = find(root, "toast").expect("toast node");
    assert_eq!(toast["kind"], "toast");
    assert_eq!(toast["view"]["kind"], "info");
    assert_eq!(toast["view"]["ttl"]["secs"], 0);

    let cell = find(root, "cell-0").expect("cell node");
    assert_eq!(cell["kind"], "table_cell");
    assert_eq!(cell["view"]["weight"], 1);

    let option = find(root, "a").expect("option node");
    assert_eq!(option["kind"], "text", "option content stays a child node");
}

#[test]
fn outline_shows_compact_state_for_components() {
    let outline = tree().inspect_outline();
    assert!(outline.contains("button [button] {pressed=false, disabled=true, loading=true}"));
    assert!(outline.contains("toast [toast] {kind='Info', expired=true}"));
    assert!(outline.contains("field [form_field] {name='email', required=false, error='required'}"));
    assert!(outline.contains("row-0 [table_row] {is_header=false}"));
}
