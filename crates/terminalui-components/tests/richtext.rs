//! Tests for the RichText view.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use terminalui_components::richtext::RichText;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Tree, View};

fn painted(tree: &mut Tree, area: Rect) -> String {
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
fn rich_text_measures_and_paints_lines() {
    let lines = vec![
        Line::from(vec![
            Span::styled("bold ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("plain"),
        ]),
        Line::raw("second row"),
    ];
    let rich = RichText::new(lines);
    assert_eq!(rich.measure(), Some((10, 2)));

    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("rich", rich)]),
    ));
    let out = painted(&mut tree, Rect::new(0, 0, 30, 2));
    assert!(out.contains("bold plain"), "got: {out}");
    assert!(out.contains("second row"), "got: {out}");
}

#[test]
fn rich_text_indents_every_line() {
    let rich = RichText::new(vec![Line::raw("indented")]).with_indent(4);
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node("rich", rich)]),
    ));
    let out = painted(&mut tree, Rect::new(0, 0, 30, 1));
    assert!(out.starts_with("    indented"), "got: {out}");
}
