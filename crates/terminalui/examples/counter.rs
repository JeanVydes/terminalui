//! A minimal interactive example: a counter with minus and plus buttons.
//!
//! Run with `cargo run --example counter`.
//!
//! TAB moves focus between the buttons, ENTER or SPACE activates the
//! focused one, and the buttons can also be clicked with the mouse.
//! `q` quits.

use std::io;
use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::style::Style;
use ratatui::Terminal;

use terminalui::components::button::{Button, ButtonVariant};
use terminalui::input::InputEvent;
use terminalui::theme::Theme;
use terminalui::{node, Stack, Text, TextStyle, Tree};

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(
        io::stdout(),
        terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let root = node(
        "root",
        Stack::vertical().gap(1).children(vec![
            node(
                "title",
                Text::new("counter").with_style(TextStyle::default().bold()),
            ),
            node("count", Text::new("count: 0")),
            node("minus", Button::new(ButtonVariant::Secondary, "-")),
            node("plus", Button::new(ButtonVariant::Primary, "+")),
            node(
                "hint",
                Text::new("tab: move focus, enter: activate, q: quit"),
            ),
        ]),
    );
    let mut tree = Tree::new(root);
    let mut count: i64 = 0;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let theme = Theme::default();
            frame
                .buffer_mut()
                .set_style(area, Style::default().bg(theme.background));
            tree.render(area, frame.buffer_mut(), &theme);
        })?;

        if event::poll(Duration::from_millis(16))? {
            let input = InputEvent::from(event::read()?);
            if let InputEvent::Key(key) = &input {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
            tree.handle_event(&input);
        }

        let plus = tree
            .find_mut::<Button>("plus")
            .map(Button::take_pressed)
            .unwrap_or(false);
        let minus = tree
            .find_mut::<Button>("minus")
            .map(Button::take_pressed)
            .unwrap_or(false);
        if plus || minus {
            count = if plus {
                count.saturating_add(1)
            } else {
                count.saturating_sub(1)
            };
            if let Some(text) = tree.find_mut::<Text>("count") {
                text.set_text(format!("count: {count}"));
            }
        }

        tree.tick(Duration::from_millis(16));
    }

    terminal::disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture,
        terminal::LeaveAlternateScreen
    )?;
    Ok(())
}
