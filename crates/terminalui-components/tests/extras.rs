//! Functional tests for kbd, stepper, status bar, popover and gauge.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_components::gauge::Gauge;
use terminalui_components::kbd::Kbd;
use terminalui_components::popover::{Popover, PopoverArrow};
use terminalui_components::statusbar::StatusBar;
use terminalui_components::stepper::{Step, StepStatus, Stepper};
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Text, Tree, View};

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

fn single(child: terminalui_core::Node) -> Tree {
    Tree::new(node("root", Stack::vertical().children(vec![child])))
}

#[test]
fn kbd_renders_brackets_and_measures() {
    let mut tree = single(node("kbd", Kbd::new("Enter")));
    let area = Rect::new(0, 0, 40, 1);
    assert_eq!(
        tree.find::<Kbd>("kbd").map(View::measure),
        Some(Some((9, 1)))
    );
    let painted = painted(&mut tree, area);
    assert!(painted.contains("[ Enter ]"), "got: {painted}");
    let outline = tree.find::<Kbd>("kbd").map(View::outline);
    assert_eq!(outline, Some(Some("key='Enter'".to_string())));
}

#[test]
fn stepper_walks_the_flow() {
    let steps = vec![
        Step::new("plan").with_status(StepStatus::Done),
        Step::new("build").with_status(StepStatus::Active),
        Step::new("ship"),
    ];
    let tree = single(node("steps", Stepper::new(steps)));
    let stepper = tree.find::<Stepper>("steps").unwrap();
    assert_eq!(stepper.current(), Some(1));
    assert_eq!(stepper.outline().unwrap(), "steps=3, current=1");
    assert_eq!(stepper.steps[0].status, StepStatus::Done);
}

#[test]
fn stepper_shows_every_glyph() {
    let mut tree = single(node(
        "steps",
        Stepper::new(vec![
            Step::new("a").with_status(StepStatus::Done),
            Step::new("b").with_status(StepStatus::Active),
            Step::new("c").with_status(StepStatus::Failed),
            Step::new("d"),
        ]),
    ));
    let area = Rect::new(0, 0, 80, 2);
    let painted = painted(&mut tree, area);
    assert!(painted.contains('✓'), "done glyph: {painted}");
    assert!(painted.contains('●'), "active glyph: {painted}");
    assert!(painted.contains('✗'), "failed glyph: {painted}");
    assert!(painted.contains('○'), "pending glyph: {painted}");
}

#[test]
fn status_bar_pins_left_and_right() {
    let mut tree = Tree::new(node(
        "bar",
        StatusBar::new()
            .left(vec![node("hint", Text::new("main"))])
            .right(vec![node("quit", Kbd::new("q"))]),
    ));
    let area = Rect::new(0, 0, 40, 2);
    let painted: Vec<String> = painted(&mut tree, area)
        .lines()
        .map(str::to_string)
        .collect();
    let first = &painted[0];
    assert!(first.starts_with("main"), "left pinned: {first}");
    assert!(first.ends_with("[ q ]"), "right pinned: {first}");
}

#[test]
fn popover_renders_arrow_when_open() {
    let mut tree = single(node(
        "pop",
        Popover::new("tip", 4, 2).with_arrow(PopoverArrow::Top),
    ));
    let area = Rect::new(0, 0, 40, 8);
    let rendered = painted(&mut tree, area);
    assert!(rendered.contains("▲"), "caret drawn: {rendered}");
    let pop = tree.find_mut::<Popover>("pop").unwrap();
    pop.hide();
    let hidden = painted(&mut tree, area);
    assert!(!hidden.contains("▲"), "hidden popover draws nothing");
    assert!(!hidden.contains("tip"), "hidden popover hides content");
}

#[test]
fn gauge_renders_bar_and_readout() {
    let mut tree = single(node("gauge", Gauge::new(0.6)));
    let area = Rect::new(0, 0, 40, 1);
    let painted = painted(&mut tree, area);
    assert!(painted.contains("60%"), "readout: {painted}");
    assert_eq!(
        tree.find::<Gauge>("gauge").map(View::outline),
        Some(Some("value=0.60, presentation='Native'".to_string()))
    );
}

#[test]
fn gauge_warns_past_threshold() {
    use terminalui_components::gauge::GaugeStyle;
    let style = GaugeStyle {
        warn_above: Some(0.5),
        ..GaugeStyle::default()
    };
    let mut tree = single(node("gauge", Gauge::new(0.8).with_style(style)));
    let area = Rect::new(0, 0, 40, 1);
    let painted = painted(&mut tree, area);
    assert!(painted.contains("80%"), "readout: {painted}");
}

#[test]
fn gauge_uses_presentation_glyphs() {
    use terminalui_components::presentation::RenderStyle;
    let mut tree = single(node(
        "gauge",
        Gauge::new(1.0).with_presentation(RenderStyle::Blocks),
    ));
    let painted = painted(&mut tree, Rect::new(0, 0, 40, 1));
    assert!(
        painted.contains('█'),
        "full fill uses blocks glyphs: {painted}"
    );
    let outline = tree.find::<Gauge>("gauge").map(View::outline);
    assert_eq!(
        outline,
        Some(Some("value=1.00, presentation='Blocks'".to_string()))
    );
}

#[test]
fn new_components_serialize_to_json() {
    let tree = single(node(
        "root",
        Stack::vertical().children(vec![
            node("kbd", Kbd::new("F1")),
            node("steps", Stepper::new(vec![Step::new("one")])),
            node(
                "bar",
                StatusBar::new().left(vec![node("bar-left", Text::new("x"))]),
            ),
            node("pop", Popover::new("x", 0, 0)),
            node("gauge", Gauge::new(0.1)),
        ]),
    ));
    let text = tree.to_json().expect("serialization succeeds");
    for kind in ["kbd", "stepper", "status_bar", "popover", "gauge"] {
        assert!(
            text.contains(&format!("\"kind\":\"{kind}\"")),
            "missing {kind}"
        );
    }
}
