//! Edit mode for the Input: selection, clipboard operations and the
//! configurable caret.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use terminalui_components::input::Input;
use terminalui_core::input::InputEvent;
use terminalui_core::{node, Stack, Tree};

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, modifiers))
}

fn shift(code: KeyCode) -> InputEvent {
    key_mod(code, KeyModifiers::SHIFT)
}

fn ctrl(code: KeyCode) -> InputEvent {
    key_mod(code, KeyModifiers::CONTROL)
}

fn input_tree() -> Tree {
    Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("input", Input::new().with_placeholder("type"))]),
    ))
}

fn press(tree: &mut Tree, event: InputEvent) {
    tree.handle_event(&event);
}

fn type_text(tree: &mut Tree, text: &str) {
    for ch in text.chars() {
        press(tree, key_mod(KeyCode::Char(ch), KeyModifiers::NONE));
    }
}

#[test]
fn shift_arrows_extend_selection_spanning_chars() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "hello");
    press(&mut tree, key(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    let input = tree.find::<Input>("input").unwrap();
    assert_eq!(input.selection(), Some((2, 4)));
    assert_eq!(input.selected_text().as_deref(), Some("ll"));
}

#[test]
fn plain_arrows_collapse_the_selection() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "abc");
    press(&mut tree, key(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    assert_eq!(
        tree.find::<Input>("input").unwrap().selection(),
        Some((0, 2))
    );
    press(&mut tree, key(KeyCode::Left));
    assert_eq!(tree.find::<Input>("input").unwrap().selection(), None);
    assert_eq!(tree.find::<Input>("input").unwrap().state.cursor, 0);
}

#[test]
fn ctrl_a_selects_all_and_delete_clears_it() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "wipe me");
    press(&mut tree, ctrl(KeyCode::Char('a')));
    assert_eq!(
        tree.find::<Input>("input").unwrap().selection(),
        Some((0, 7))
    );
    press(&mut tree, key(KeyCode::Delete));
    assert_eq!(tree.find::<Input>("input").unwrap().state.value, "");
    assert_eq!(tree.find::<Input>("input").unwrap().state.cursor, 0);
}

#[test]
fn ctrl_c_keeps_selection_and_ctrl_x_removes_it() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "copy me");
    press(&mut tree, ctrl(KeyCode::Char('a')));
    press(&mut tree, ctrl(KeyCode::Char('c')));
    assert_eq!(tree.find::<Input>("input").unwrap().state.value, "copy me");
    assert_eq!(
        tree.find::<Input>("input").unwrap().clipboard.as_deref(),
        Some("copy me")
    );
    press(&mut tree, ctrl(KeyCode::Char('x')));
    assert_eq!(tree.find::<Input>("input").unwrap().state.value, "");
    press(&mut tree, ctrl(KeyCode::Char('v')));
    assert_eq!(tree.find::<Input>("input").unwrap().state.value, "copy me");
}

#[test]
fn paste_event_replaces_what_is_selected() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "keep this");
    press(&mut tree, ctrl(KeyCode::Char('a')));
    tree.handle_event(&InputEvent::Paste("swapped".to_string()));
    assert_eq!(tree.find::<Input>("input").unwrap().state.value, "swapped");
}

#[test]
fn plain_typing_replaces_what_is_selected() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "abcd");
    press(&mut tree, key(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Left));
    press(&mut tree, key_mod(KeyCode::Char('X'), KeyModifiers::NONE));
    assert_eq!(tree.find::<Input>("input").unwrap().state.value, "Xd");
}

#[test]
fn ctrl_l_selects_the_current_line() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("input", Input::new().multiline(true))]),
    ));
    assert!(tree.focus("input"));
    type_text(&mut tree, "line one\nline two\nline three");
    press(&mut tree, ctrl(KeyCode::Char('l')));
    assert_eq!(
        tree.find::<Input>("input")
            .unwrap()
            .selected_text()
            .as_deref(),
        Some("line three")
    );
}

#[test]
fn shift_home_selects_to_line_start() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "hello world");
    press(&mut tree, shift(KeyCode::Home));
    assert_eq!(
        tree.find::<Input>("input")
            .unwrap()
            .selected_text()
            .as_deref(),
        Some("hello world")
    );
}

#[test]
fn edit_mode_off_keeps_plain_editing_without_selection() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("input", Input::new().edit_mode(false))]),
    ));
    assert!(tree.focus("input"));
    type_text(&mut tree, "ab");
    press(&mut tree, shift(KeyCode::Left));
    press(&mut tree, shift(KeyCode::Right));
    assert_eq!(tree.find::<Input>("input").unwrap().selection(), None);
    assert_eq!(tree.find::<Input>("input").unwrap().state.cursor, 2);
}

#[test]
fn outline_reports_selection_when_active() {
    let mut tree = input_tree();
    assert!(tree.focus("input"));
    type_text(&mut tree, "ab");
    press(&mut tree, shift(KeyCode::Left));
    let outline = tree.inspect_outline();
    assert!(outline.contains("selected=1"), "outline: {outline}");
}
