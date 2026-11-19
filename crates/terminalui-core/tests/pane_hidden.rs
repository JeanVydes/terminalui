//! A hidden pane child keeps its state and stays queryable, but it gets no
//! events, no focus slot and no ticks until it becomes visible.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Outcome, Pane, Tree, View};

#[derive(Default, serde::Serialize)]
struct Probe {
    events: u32,
    ticks: u32,
    payload: String,
}

impl View for Probe {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn render(&mut self, _area: Rect, _buf: &mut Buffer, _cx: &mut Context, _theme: &Theme) {}

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        self.events += 1;
        Outcome::Consumed
    }

    fn tick(&mut self, _delta: Duration, _cx: &mut Context) {
        self.ticks += 1;
    }

    fn focusable(&self) -> bool {
        true
    }
}

fn page(id: &str) -> terminalui_core::Node {
    node(id, Probe::default())
}

fn sample_key() -> InputEvent {
    InputEvent::Key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::NONE,
    ))
}

#[test]
fn hidden_pages_keep_state_but_receive_no_events() {
    let mut tree = Tree::new(node("root", Pane::new(vec![page("first"), page("second")])));

    tree.handle_event(&sample_key());
    tree.tick(Duration::from_millis(16));

    let second = tree
        .find::<Probe>("second")
        .expect("hidden page stays queryable");
    assert_eq!(second.events, 0);
    assert_eq!(second.ticks, 0);

    let first = tree.find::<Probe>("first").expect("visible page is found");
    assert_eq!(first.events, 1);
    assert_eq!(first.ticks, 1);
}

#[test]
fn hidden_pages_do_not_shift_focus_indexes() {
    let mut tree = Tree::new(node(
        "root",
        Pane::new(vec![page("first"), page("second"), page("third")]),
    ));

    assert_eq!(tree.focusable_ids(), ["root", "first"]);
    assert!(tree.focus("first"));
    assert_eq!(tree.focused_id(), Some("first"));

    let root = tree.find_mut::<Pane>("root").expect("root pane");
    root.set_visible(2);
    assert!(tree.focus("third"));
    assert_eq!(tree.focused_id(), Some("third"));

    tree.handle_event(&sample_key());
    let third = tree
        .find::<Probe>("third")
        .expect("visible page is focusable");
    assert_eq!(third.events, 1);
    let second = tree
        .find::<Probe>("second")
        .expect("hidden page stays queryable");
    assert_eq!(second.events, 0);
}

#[test]
fn tree_queries_reach_hidden_pages() {
    let mut tree = Tree::new(node("root", Pane::new(vec![page("first"), page("second")])));

    {
        let second = tree
            .find_mut::<Probe>("second")
            .expect("hidden page is queryable mutably");
        second.payload = "data".to_string();
    }

    if let Some(pane) = tree.find_mut::<Pane>("root") {
        pane.set_visible(1);
    }

    let second = tree
        .find::<Probe>("second")
        .expect("second page becomes visible");
    assert_eq!(second.payload, "data");
    assert!(tree.focusable_ids().contains(&"second"));
    assert!(!tree.focusable_ids().contains(&"first"));
}
