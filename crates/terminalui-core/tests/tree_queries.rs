//! Duplicated ids are a supported feature: `find_all` and `visit_mut`
//! address every node sharing an id, and `press` activates the first
//! focusable match.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Text, Tree, View};

#[derive(Default, serde::Serialize)]
struct Probe {
    activations: u32,
}

impl View for Probe {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn render(
        &mut self,
        _area: Rect,
        _buf: &mut Buffer,
        cx: &mut terminalui_core::Context,
        _theme: &Theme,
    ) {
        let _ = cx.is_focused();
    }

    fn handle_event(
        &mut self,
        _event: &InputEvent,
        _cx: &mut terminalui_core::Context,
    ) -> terminalui_core::Outcome {
        terminalui_core::Outcome::Ignored
    }

    fn tick(&mut self, _delta: Duration, _cx: &mut terminalui_core::Context) {}

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &terminalui_core::Context) -> bool {
        self.activations += 1;
        true
    }
}

fn two_confirms() -> Tree {
    Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("confirm", Text::new("label only")),
            node("confirm", Probe::default()),
            node("confirm", Probe::default()),
        ]),
    ))
}

#[test]
fn find_all_returns_every_view_with_the_id() {
    let tree = two_confirms();
    let found = tree.find_all::<Probe>("confirm");
    assert_eq!(found.len(), 2);
}

#[test]
fn press_activates_the_first_focusable_match() {
    let mut tree = two_confirms();
    let area = Rect::new(0, 0, 20, 4);
    let mut buffer = Buffer::empty(area);
    let theme = Theme::default();
    tree.render(area, &mut buffer, &theme);
    assert!(tree.press("confirm"));
    let probes = tree.find_all::<Probe>("confirm");
    let activations: u32 = probes.iter().map(|probe| probe.activations).sum();
    assert_eq!(activations, 1);
}

#[test]
fn visit_mut_applies_to_all_matching_views() {
    let mut tree = two_confirms();
    let mut visited = 0usize;
    tree.visit_mut(|node| {
        if node.id() == "confirm" && node.view.as_any().is::<Probe>() {
            visited += 1;
        }
    });
    assert_eq!(visited, 2);
}

#[test]
fn duplicated_focusables_are_all_reachable_by_tab() {
    let mut tree = two_confirms();
    let first = tree.handle_event(&key(crossterm::event::KeyCode::Tab));
    let second = tree.handle_event(&key(crossterm::event::KeyCode::Tab));
    let third = tree.handle_event(&key(crossterm::event::KeyCode::Tab));
    let _ = (first, second, third);
    assert_eq!(
        tree.focusable_ids().into_iter().collect::<Vec<_>>(),
        vec![&"confirm".to_string(), &"confirm".to_string()]
    );
    let _ = tree.focused_id();
}

fn key(code: crossterm::event::KeyCode) -> InputEvent {
    InputEvent::Key(crossterm::event::KeyEvent::new(
        code,
        crossterm::event::KeyModifiers::NONE,
    ))
}
