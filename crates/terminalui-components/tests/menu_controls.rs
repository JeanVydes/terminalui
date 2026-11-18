//! Functional tests for the menu family: dropdown, context menu, command
//! palette and autocomplete.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Tree, View};
use terminalui_components::autocomplete::Autocomplete;
use terminalui_components::commandpalette::{Command, CommandPalette};
use terminalui_components::contextmenu::ContextMenu;
use terminalui_components::dropdown::DropdownMenu;
use terminalui_components::menu::{MenuEntry, MenuItem};

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn char_key(character: char) -> InputEvent {
    InputEvent::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
}

fn click(column: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn char_keys(text: &str) -> Vec<InputEvent> {
    text.chars().map(char_key).collect()
}

fn menu_items() -> Vec<MenuEntry> {
    vec![
        MenuEntry::item("deploy"),
        MenuEntry::separator(),
        MenuEntry::Item(MenuItem::new("rollback").disabled(true)),
        MenuEntry::item("restart"),
    ]
}

fn press(tree: &mut Tree, key: InputEvent) {
    tree.handle_event(&key);
}

#[test]
fn dropdown_opens_navigates_and_activates() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "menu",
            DropdownMenu::with_label("Actions").with_items(menu_items()),
        )]),
    ));
    assert!(tree.focus("menu"));
    let theme = Theme::default();
    let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 10));
    tree.render(Rect::new(0, 0, 40, 10), &mut buffer, &theme);

    press(&mut tree, key(KeyCode::Char(' ')));
    press(&mut tree, key(KeyCode::Down));
    press(&mut tree, key(KeyCode::Enter));
    let activated = tree
        .find_mut::<DropdownMenu>("menu")
        .and_then(DropdownMenu::take_activated);
    assert_eq!(activated, Some(3), "skips the separator and disabled item");
}

#[test]
fn dropdown_escape_closes_without_activating() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "menu",
            DropdownMenu::with_label("Actions").with_items(menu_items()),
        )]),
    ));
    assert!(tree.focus("menu"));
    press(&mut tree, key(KeyCode::Enter));
    press(&mut tree, key(KeyCode::Esc));
    let out = tree
        .find_mut::<DropdownMenu>("menu")
        .map(|menu| menu.state.open);
    assert_eq!(out, Some(false));
    let activated = tree
        .find_mut::<DropdownMenu>("menu")
        .and_then(DropdownMenu::take_activated);
    assert_eq!(activated, None);
}

#[test]
fn context_menu_opens_at_mouse_position_and_activates() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("ctx", ContextMenu::new(menu_items()))]),
    ));
    tree.find_mut::<ContextMenu>("ctx").unwrap().open_at(10, 5);
    assert!(tree.focus("ctx"));
    let theme = Theme::default();
    let area = Rect::new(0, 0, 60, 12);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    assert!(tree.find::<ContextMenu>("ctx").unwrap().is_open());

    press(&mut tree, key(KeyCode::Down));
    press(&mut tree, key(KeyCode::Enter));
    let activated = tree
        .find_mut::<ContextMenu>("ctx")
        .and_then(ContextMenu::take_activated);
    assert_eq!(activated, Some(3));
    assert!(!tree.find::<ContextMenu>("ctx").unwrap().is_open());
}

#[test]
fn context_menu_click_outside_closes() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("ctx", ContextMenu::new(menu_items()))]),
    ));
    tree.find_mut::<ContextMenu>("ctx").unwrap().open_at(5, 3);
    let theme = Theme::default();
    let area = Rect::new(0, 0, 60, 12);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    assert!(tree.focus("ctx"));
    press(&mut tree, click(50, 11));
    assert!(!tree.find::<ContextMenu>("ctx").unwrap().is_open());
}

#[test]
fn palette_filters_and_activates_the_match() {
    let commands = vec![
        Command::new("deploy frontend").detail("prod"),
        Command::new("deploy backend").detail("prod"),
        Command::new("rollback").detail("prod"),
        Command::new("view logs").detail("observability"),
    ];
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("palette", CommandPalette::new(commands))]),
    ));
    tree.find_mut::<CommandPalette>("palette").unwrap().open();
    assert!(tree.focus("palette"));
    for event in char_keys("deploy") {
        press(&mut tree, event);
    }
    press(&mut tree, key(KeyCode::Down));
    press(&mut tree, key(KeyCode::Enter));
    let activated = tree
        .find_mut::<CommandPalette>("palette")
        .and_then(CommandPalette::take_activated);
    assert_eq!(activated, Some(1));
    assert!(!tree.find::<CommandPalette>("palette").unwrap().is_open());
}

#[test]
fn palette_escape_keeps_nothing_pending() {
    let commands = vec![Command::new("noop")];
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("palette", CommandPalette::new(commands))]),
    ));
    tree.find_mut::<CommandPalette>("palette").unwrap().open();
    assert!(tree.focus("palette"));
    for event in char_keys("noop") {
        press(&mut tree, event);
    }
    press(&mut tree, key(KeyCode::Esc));
    let pending = tree
        .find_mut::<CommandPalette>("palette")
        .map(|p| p.is_open());
    assert_eq!(pending, Some(false));
    let activated = tree
        .find_mut::<CommandPalette>("palette")
        .and_then(CommandPalette::take_activated);
    assert_eq!(activated, None);
}

#[test]
fn autocomplete_opens_filters_and_applies() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "ac",
            Autocomplete::new(vec![
                "alpha".to_string(),
                "beta".to_string(),
                "gamma".to_string(),
            ]),
        )]),
    ));
    assert!(tree.focus("ac"));
    let theme = Theme::default();
    let area = Rect::new(0, 0, 30, 6);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);

    for event in char_keys("al") {
        press(&mut tree, event);
    }
    let outline = tree.find::<Autocomplete>("ac").map(|ac| ac.outline());
    assert_eq!(
        outline,
        Some(Some("value='al', open=true, matches=1".to_string()))
    );
    press(&mut tree, key(KeyCode::Enter));
    let value = tree
        .find::<Autocomplete>("ac")
        .map(|ac| ac.value().to_string());
    assert_eq!(value.as_deref(), Some("alpha"));
}

#[test]
fn autocomplete_escape_keeps_typed_text() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "ac",
            Autocomplete::new(vec!["alpha".to_string(), "alpine".to_string()]),
        )]),
    ));
    assert!(tree.focus("ac"));
    for event in char_keys("al") {
        press(&mut tree, event);
    }
    press(&mut tree, key(KeyCode::Esc));
    let value = tree
        .find::<Autocomplete>("ac")
        .map(|ac| ac.value().to_string());
    assert_eq!(value.as_deref(), Some("al"));
    let open = tree.find::<Autocomplete>("ac").map(|ac| ac.state.open);
    assert_eq!(open, Some(false));
}
