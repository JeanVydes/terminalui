//! Measured children keep their intrinsic size along the stack axis.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_components::button::{Button, ButtonVariant};
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Tree};

#[test]
fn horizontal_stack_renders_full_button_labels() {
    let mut tree = Tree::new(node(
        "actions",
        Stack::horizontal().gap(2).children(vec![
            node("cancel", Button::new(ButtonVariant::Secondary, "cancel")),
            node("save", Button::new(ButtonVariant::Primary, "save")),
        ]),
    ));
    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &Theme::default());

    let rendered: String = (0..area.width)
        .map(|x| {
            buffer
                .cell((x, 0))
                .map_or(" ", ratatui::buffer::Cell::symbol)
        })
        .collect();
    assert!(
        rendered.contains("cancel"),
        "the cancel button was clipped: {rendered:?}"
    );
    assert!(
        rendered.contains("save"),
        "the save button was clipped: {rendered:?}"
    );
}
