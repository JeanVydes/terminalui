//! Tests for the console shell log.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_components::console::{Console, ConsoleStatus};
use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Outcome, Stack, Tree, View};

fn single(child: terminalui_core::Node) -> Tree {
    Tree::new(node("root", Stack::vertical().children(vec![child])))
}

fn paint(tree: &mut Tree, area: Rect) -> String {
    let theme = Theme::default();
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn console_measures_header_plus_visible_rows() {
    let console = Console::new("cargo test");
    assert_eq!(console.measure(), Some((0, 2)));
    let console = console.set_lines(vec![
        "line one".into(),
        "line two".into(),
        "line three".into(),
    ]);
    assert_eq!(console.measure(), Some((0, 4)));
    let mut console = console.with_max_visible(2);
    assert_eq!(console.measure(), Some((0, 3)));
    console.push_line("line four");
    assert_eq!(console.measure(), Some((0, 3)));
}

#[test]
fn console_renders_command_and_waiting_placeholder() {
    let mut tree = single(node("shell", Console::new("cargo test")));
    let out = paint(&mut tree, Rect::new(0, 0, 40, 2));
    assert!(out.contains("cargo test"), "got: {out}");
    assert!(out.contains("waiting for output..."), "got: {out}");
    assert!(out.contains('⠋'), "got: {out}");
}

#[test]
fn console_reports_exit_status_in_the_header() {
    let console = Console::new("cargo test").set_lines(vec!["ok".into()]);
    assert_eq!(console.status, ConsoleStatus::Running);
    let mut console = console;
    console.set_exit(Some(0));
    assert_eq!(console.status, ConsoleStatus::Exited { code: Some(0) });
    let mut tree = single(node("shell", console));
    let out = paint(&mut tree, Rect::new(0, 0, 40, 2));
    assert!(out.contains("done"), "got: {out}");

    let mut console = Console::new("cargo test").set_lines(vec!["failed".into()]);
    console.set_exit(Some(3));
    let mut tree = single(node("shell", console));
    let out = paint(&mut tree, Rect::new(0, 0, 40, 2));
    assert!(out.contains("exit 3"), "got: {out}");
}

#[test]
fn console_scrolls_output_with_overflow() {
    let lines: Vec<String> = (0..12).map(|n| format!("row {n}")).collect();
    let console = Console::new("yes").with_max_visible(4).set_lines(lines);
    assert_eq!(console.state.offset, 0);
    let mut tree = single(node("shell", console));
    let out = paint(&mut tree, Rect::new(0, 0, 40, 6));
    assert!(out.contains("row 11"), "got: {out}");
    assert!(!out.contains("row 0"), "got: {out}");
}

#[test]
fn long_output_wraps_to_the_rendered_width() {
    let line = "abcdefghijklmnopqrstuvwxyz";
    let console = Console::new("yes").set_lines(vec![line.to_string()]);
    let mut tree = Tree::new(node("shell", console));
    let out = paint(&mut tree, Rect::new(0, 0, 12, 4));
    let rows: Vec<&str> = out.lines().collect();
    assert!(rows[1].contains("abcdefghij"), "got: {out}");
    assert!(rows[2].contains("klmnopqrst"), "got: {out}");
    assert!(rows[3].contains("uvwxyz"), "got: {out}");
}

#[test]
fn wrapped_height_counts_wrapped_rows() {
    let console = Console::new("yes").set_lines(vec!["abcdefghijklmnopqrstuvwxyz".into()]);
    assert_eq!(console.wrapped_height(10), 4);
    assert_eq!(console.wrapped_height(26), 2);
}

#[test]
fn console_outline_reports_running_state() {
    let console = Console::new("cargo build").set_lines(vec!["one".into(), "two".into()]);
    assert_eq!(
        console.outline().as_deref(),
        Some("command='cargo build' status=running rows=2")
    );
}

#[test]
fn expanded_console_measures_every_retained_line() {
    let lines: Vec<String> = (0..5).map(|n| format!("row {n}")).collect();
    let console = Console::new("yes")
        .with_max_visible(2)
        .set_lines(lines)
        .with_expanded(true);
    assert!(console.is_expanded());
    assert_eq!(console.measure(), Some((0, 6)));
    assert_eq!(console.state.offset, 0);
}

#[test]
fn expanded_console_renders_every_line_and_the_live_count() {
    let lines: Vec<String> = (0..12).map(|n| format!("row {n}")).collect();
    let console = Console::new("yes")
        .with_max_visible(4)
        .set_lines(lines)
        .with_expanded(true);
    let mut tree = single(node("shell", console));
    let out = paint(&mut tree, Rect::new(0, 0, 40, 13));
    assert!(out.contains("row 0"), "got: {out}");
    assert!(out.contains("row 11"), "got: {out}");
    assert!(out.contains("12 lines"), "got: {out}");
    assert!(out.contains("[-]"), "got: {out}");
}

#[test]
fn the_header_line_count_follows_new_output() {
    let mut tree = single(node("shell", Console::new("cargo test")));
    let area = Rect::new(0, 0, 40, 3);

    tree.find_mut::<Console>("shell")
        .expect("console")
        .push_line("one");
    let text = paint(&mut tree, area);
    assert!(text.contains("1 lines"), "got: {text}");

    tree.find_mut::<Console>("shell")
        .expect("console")
        .push_line("two");
    let text = paint(&mut tree, area);
    assert!(text.contains("2 lines"), "got: {text}");
    assert!(text.contains("[+]"), "got: {text}");
}

#[test]
fn clicking_the_header_toggles_expansion() {
    let mut tree = single(node("shell", Console::new("yes")));
    let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
    tree.render(Rect::new(0, 0, 40, 3), &mut buffer, &Theme::default());
    let click = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 2,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(tree.handle_event(&click), Outcome::Consumed);
    assert!(tree
        .find::<Console>("shell")
        .expect("console")
        .is_expanded());

    assert_eq!(tree.handle_event(&click), Outcome::Consumed);
    assert!(!tree
        .find::<Console>("shell")
        .expect("console")
        .is_expanded());
}

#[test]
fn enter_toggles_expansion() {
    let mut tree = single(node("shell", Console::new("yes")));
    let enter = InputEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(tree.handle_event(&enter), Outcome::Consumed);
    assert!(tree
        .find::<Console>("shell")
        .expect("console")
        .is_expanded());
}
