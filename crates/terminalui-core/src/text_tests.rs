//! Style mutation keeps the text leaf instance intact.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::{Context, Text, TextStyle, View};
use crate::Theme;

#[test]
fn set_style_updates_the_rendered_color_without_replacing_the_text() {
    let mut text = Text::new("ready").with_style(TextStyle::default().fg(Color::Red));
    let area = Rect::new(0, 0, 8, 1);
    let theme = Theme::default();

    let mut buffer = Buffer::empty(area);
    text.render(area, &mut buffer, &mut Context::new(None), &theme);
    assert_eq!(buffer[(0, 0)].fg, Color::Red);

    text.set_style(TextStyle::default().fg(Color::Green).bold());
    let mut buffer = Buffer::empty(area);
    text.render(area, &mut buffer, &mut Context::new(None), &theme);
    assert_eq!(buffer[(0, 0)].fg, Color::Green);
    assert!(buffer[(0, 0)]
        .modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(text.text(), "ready");
}

#[test]
fn clip_keeps_short_text_and_marks_truncated_text() {
    assert_eq!(super::clip("ready", 8), "ready");
    assert_eq!(super::clip("hello world", 5), "hell…");
    assert_eq!(super::clip("hello", 0), "…");
}
