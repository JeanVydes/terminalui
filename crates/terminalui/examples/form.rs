//! A small settings form with required fields and submit handling.
//!
//! Run with `cargo run --example form`.
//!
//! TAB moves between fields, ENTER submits, ESC quits. Empty required
//! fields are marked on submit; fix them and press ENTER again.

use std::io;
use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;

use terminalui::components::form::{Form, FormField};
use terminalui::InputEvent;
use terminalui::{node, Stack, Text, TextStyle, Theme, Tree};

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(
        io::stdout(),
        terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    let form = Form::new(vec![
        node("name", FormField::text("name", "Name").required()),
        node(
            "email",
            FormField::text("email", "Email").required_with_message("email is required"),
        ),
        node(
            "nickname",
            FormField::text("nickname", "Nickname (optional)"),
        ),
    ]);

    let root = node(
        "root",
        Stack::vertical().gap(1).children(vec![
            node(
                "title",
                Text::new("form example").with_style(TextStyle::default().bold()),
            ),
            node("form", form),
            node("status", Text::new("not submitted yet")),
            node(
                "hint",
                Text::new("tab: next field, enter: submit, esc: quit"),
            ),
        ]),
    );
    let mut tree = Tree::new(root);

    loop {
        let theme = Theme::default();
        terminal.draw(|frame| {
            let area = frame.area();
            frame
                .buffer_mut()
                .set_style(area, ratatui::style::Style::default().bg(theme.background));
            tree.render(area, frame.buffer_mut(), &theme);
        })?;

        if event::poll(Duration::from_millis(16))? {
            let input = InputEvent::from(event::read()?);
            if let InputEvent::Key(key) = &input {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc {
                    break;
                }
            }
            tree.handle_event(&input);
        }

        let submitted = tree.find_mut::<Form>("form").and_then(Form::take_submitted);
        if let Some(values) = submitted {
            let joined = values
                .iter()
                .map(|(name, value)| format!("{name}={value:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(status) = tree.find_mut::<Text>("status") {
                status.set_text(format!("submitted: {joined}"));
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
