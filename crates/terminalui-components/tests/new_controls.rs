//! Functional tests for the newer components: radio group, switch, tree view,
//! line chart, bar chart, sparkline and stat.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Tree};
use terminalui_components::linechart::LineChart;
use terminalui_components::radio::RadioGroup;
use terminalui_components::sparkline::Sparkline;
use terminalui_components::stat::{Stat, StatTone};
use terminalui_components::switch::Switch;
use terminalui_components::treeview::{TreeNode, TreeView};

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn click(column: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn radio_group_moves_cursor_and_picks_with_keys() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "radio",
            RadioGroup::with_options(vec!["alpha", "beta", "gamma"]),
        )]),
    ));
    assert!(tree.focus("radio"));
    tree.handle_event(&key(KeyCode::Down));
    tree.handle_event(&key(KeyCode::Down));
    tree.handle_event(&key(KeyCode::Enter));
    let selected = tree
        .find::<RadioGroup>("radio")
        .and_then(RadioGroup::selected);
    assert_eq!(selected, Some(2));
    tree.handle_event(&key(KeyCode::Enter));
    let selected = tree
        .find::<RadioGroup>("radio")
        .and_then(RadioGroup::selected);
    assert_eq!(selected, Some(2), "re-Enter keeps the same option");
}

#[test]
fn radio_group_click_picks_the_option_under_the_mouse() {
    let area = Rect::new(0, 0, 20, 10);
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "radio",
            RadioGroup::with_options(vec!["alpha", "beta"]),
        )]),
    ));
    let theme = Theme::default();
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    tree.handle_event(&click(2, 1));
    let selected = tree
        .find::<RadioGroup>("radio")
        .and_then(RadioGroup::selected);
    assert_eq!(selected, Some(1));
}

#[test]
fn switch_toggles_with_space_and_click() {
    let area = Rect::new(0, 0, 20, 4);
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("switch", Switch::with_label("watch"))]),
    ));
    assert!(tree.focus("switch"));
    let theme = Theme::default();
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    assert_eq!(tree.find::<Switch>("switch").map(Switch::on), Some(false));
    tree.handle_event(&key(KeyCode::Char(' ')));
    assert_eq!(tree.find::<Switch>("switch").map(Switch::on), Some(true));
    tree.handle_event(&click(1, 0));
    assert_eq!(tree.find::<Switch>("switch").map(Switch::on), Some(false));
}

#[test]
fn tree_view_expands_and_collapses_branches() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "tree",
            TreeView::new(vec![TreeNode::branch(
                "parent",
                vec![TreeNode::leaf("child-a"), TreeNode::leaf("child-b")],
            )]),
        )]),
    ));
    assert!(tree.focus("tree"));
    let before = tree.find::<TreeView>("tree").unwrap();
    assert_eq!(before.selected_label(), Some("parent"));
    tree.handle_event(&key(KeyCode::Right));
    tree.handle_event(&key(KeyCode::Down));
    let inside = tree.find::<TreeView>("tree").unwrap();
    assert_eq!(inside.selected_label(), Some("child-a"));
    assert_eq!(
        inside.selected_path().map(|path| path.len()),
        Some(2),
        "path descends into the branch"
    );
    tree.handle_event(&key(KeyCode::Left));
    tree.handle_event(&key(KeyCode::Left));
    let collapsed = tree.find::<TreeView>("tree").unwrap();
    assert_eq!(collapsed.selected_label(), Some("parent"));
}

#[test]
fn tree_view_serializes_nodes_and_expansion() {
    let tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "tree",
            TreeView::new(vec![TreeNode::branch(
                "parent",
                vec![TreeNode::leaf("child")],
            )
            .with_expanded(true)]),
        )]),
    ));
    let text = tree.to_json().expect("serialization succeeds");
    assert!(text.contains(r#""kind":"tree_view""#));
    assert!(text.contains(r#""label":"parent""#));
    assert!(text.contains(r#""expanded":true"#));
}

#[test]
fn charts_reports_kinds_and_series() {
    let mut line = LineChart::new();
    line.set_series(vec![1.0, 2.0, 1.5]);
    let spark = Sparkline::new(vec![0.0, 1.0, 0.5]);
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("line", line),
            node("spark", spark),
            node(
                "stat",
                Stat::new("deploys", "12").with_delta("+3", StatTone::Positive),
            ),
        ]),
    ));
    let text = tree.to_json().expect("serialization succeeds");
    assert!(text.contains(r#""kind":"line_chart""#));
    assert!(text.contains(r#""kind":"sparkline""#));
    assert!(text.contains(r#""kind":"stat""#));
    assert!(text.contains(r#""series":[1.0,2.0,1.5]"#));

    let area = Rect::new(0, 0, 40, 6);
    let theme = Theme::default();
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let painted: String = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(painted.contains("12"), "stat value renders: {painted}");
    assert!(painted.contains("▲"), "stat delta arrow renders: {painted}");
}
