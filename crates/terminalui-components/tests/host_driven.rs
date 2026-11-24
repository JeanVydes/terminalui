//! Host-driven control of components that usually receive focus from the tree.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use terminalui_components::autocomplete::Autocomplete;
use terminalui_components::form::{Form, FormField};
use terminalui_components::tabs::Tabs;
use terminalui_components::treeview::{TreeNode, TreeView};
use terminalui_core::{node, Outcome, View};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn autocomplete_can_be_driven_by_the_host() {
    let mut field = Autocomplete::new(vec!["openrouter".into(), "ollama".into()]);
    assert_eq!(
        field.handle_key(&key(KeyCode::Char('o'))),
        Outcome::Consumed
    );
    assert_eq!(field.value(), "o");
    assert_eq!(field.filtered(), vec!["openrouter", "ollama"]);

    field.set_value("oll");
    assert_eq!(field.value(), "oll");
    assert_eq!(field.filtered(), vec!["ollama"]);

    assert_eq!(
        field.handle_key(&key(KeyCode::Backspace)),
        Outcome::Consumed
    );
    assert_eq!(field.value(), "ol");
}

#[test]
fn treeview_exposes_visible_rows_and_can_be_driven_by_the_host() {
    let branch = TreeNode::branch(
        "openrouter",
        vec![TreeNode::leaf("one"), TreeNode::leaf("two")],
    );
    let mut tree = TreeView::new(vec![branch, TreeNode::leaf("ollama")]);

    assert_eq!(
        tree.rows()
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        vec!["openrouter", "ollama"]
    );

    assert_eq!(tree.handle_key(&key(KeyCode::Right)), Outcome::Consumed);
    assert_eq!(
        tree.rows()
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        vec!["openrouter", "one", "two", "ollama"]
    );

    assert_eq!(tree.handle_key(&key(KeyCode::Down)), Outcome::Consumed);
    assert_eq!(tree.selected_row().map(|row| row.label), Some("one".into()));
    assert_eq!(tree.handle_key(&key(KeyCode::Down)), Outcome::Consumed);
    assert_eq!(tree.selected_row().map(|row| row.label), Some("two".into()));

    tree.set_cursor(0);
    assert_eq!(tree.toggle_selected(), Outcome::Consumed);
    assert_eq!(tree.rows().len(), 2);
}

#[test]
fn form_accepts_host_validation_errors() {
    let mut form = Form::new(vec![node(
        "key",
        FormField::text("key", "api key").required(),
    )]);
    form.set_error("key", Some("invalid key"));
    let values = form.values();
    assert_eq!(values.len(), 1);

    let outline = form
        .children()
        .first()
        .and_then(|child| child.view.outline())
        .unwrap_or_default();
    assert!(outline.contains("invalid key"), "{outline}");
    form.set_error("key", None::<String>);
    let outline = form
        .children()
        .first()
        .and_then(|child| child.view.outline())
        .unwrap_or_default();
    assert!(!outline.contains("invalid key"), "{outline}");
}

#[test]
fn tabs_can_be_stepped_by_the_host() {
    let mut tabs = Tabs::with_titles(vec!["chat", "models", "settings"]);
    tabs.step_selection(true);
    assert_eq!(tabs.selected(), 1);
    tabs.step_selection(false);
    assert_eq!(tabs.selected(), 0);
    tabs.step_selection(false);
    assert_eq!(tabs.selected(), 2);
}
