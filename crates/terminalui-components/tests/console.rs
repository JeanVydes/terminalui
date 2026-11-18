//! Tests for the console shell log.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_components::console::{Console, ConsoleStatus};
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Tree, View};

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
fn console_outline_reports_running_state() {
    let console = Console::new("cargo build").set_lines(vec!["one".into(), "two".into()]);
    assert_eq!(
        console.outline().as_deref(),
        Some("command='cargo build' status=running rows=2")
    );
}
