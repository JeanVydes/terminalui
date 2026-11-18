//! `View::tick` forwards to children by default, so containers only
//! override it when they own animation.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Outcome, Stack, Tree, View};

#[derive(Default, serde::Serialize)]
struct TickProbe {
    ticks: u32,
}

impl View for TickProbe {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn render(&mut self, _area: Rect, _buf: &mut Buffer, _cx: &mut Context, _theme: &Theme) {}

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }

    fn tick(&mut self, _delta: Duration, _cx: &mut Context) {
        self.ticks += 1;
    }
}

#[test]
fn composite_ticks_nested_children_by_default() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![node(
            "middle",
            Stack::horizontal().children(vec![node("probe", TickProbe::default())]),
        )]),
    ));

    let delta = Duration::from_millis(16);
    tree.tick(delta);
    tree.tick(delta);

    let ticks = tree.find::<TickProbe>("probe").map(|probe| probe.ticks);
    assert_eq!(ticks, Some(2));
}
