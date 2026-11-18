//! A modal confirmation flow with a second view tree.
//!
//! Run with `cargo run --example dialog`.
//!
//! ENTER on the delete button opens the dialog, TAB moves between the
//! dialog's buttons, ENTER activates the focused one, ESC cancels (or
//! quits when no dialog is open), `q` quits.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::style::Style;

use terminalui::components::button::{Button, ButtonVariant};
use terminalui::components::dialog::Dialog;
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

    let main_root = node(
        "root",
        Stack::vertical().gap(1).children(vec![
            node(
                "title",
                Text::new("dialog example").with_style(TextStyle::default().bold()),
            ),
            node(
                "delete",
                Button::new(ButtonVariant::Destructive, "Delete service"),
            ),
            node("status", Text::new("nothing deleted yet")),
            node("hint", Text::new("enter: activate, esc: cancel/quit")),
        ]),
    );
    let mut main_tree = Tree::new(main_root);

    let dialog_root = node(
        "dialog",
        Dialog::with_default_actions()
            .with_title("Delete service")
            .with_message("This cannot be undone."),
    );
    let mut dialog_tree = Tree::new(dialog_root);

    let mut dialog_open = false;
    let mut last = Instant::now();

    loop {
        let now = Instant::now();
        let delta = now - last;
        last = now;

        let theme = Theme::default();
        terminal.draw(|frame| {
            let area = frame.area();
            frame
                .buffer_mut()
                .set_style(area, Style::default().bg(theme.background));
            main_tree.render(area, frame.buffer_mut(), &theme);
            if dialog_open {
                dialog_tree.render(area, frame.buffer_mut(), &theme);
            }
        })?;

        if event::poll(Duration::from_millis(16))? {
            let input = InputEvent::from(event::read()?);
            if let InputEvent::Key(key) = &input {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc {
                    if dialog_open {
                        set_status(&mut main_tree, "cancelled");
                        dialog_open = false;
                        if let Some(dialog) = dialog_tree.find_mut::<Dialog>("dialog") {
                            dialog.close();
                        }
                        continue;
                    }
                    break;
                }
            }
            if dialog_open {
                dialog_tree.handle_event(&input);
            } else {
                main_tree.handle_event(&input);
            }
        }

        if dialog_open {
            let confirm = dialog_tree
                .find_mut::<Button>("confirm")
                .map(Button::take_pressed)
                .unwrap_or(false);
            let cancel = dialog_tree
                .find_mut::<Button>("cancel")
                .map(Button::take_pressed)
                .unwrap_or(false);
            if confirm || cancel {
                dialog_open = false;
                if let Some(dialog) = dialog_tree.find_mut::<Dialog>("dialog") {
                    dialog.close();
                }
                set_status(
                    &mut main_tree,
                    if confirm {
                        "service deleted"
                    } else {
                        "cancelled"
                    },
                );
            }
        } else {
            let pressed = main_tree
                .find_mut::<Button>("delete")
                .map(Button::take_pressed)
                .unwrap_or(false);
            if pressed {
                dialog_open = true;
                if let Some(dialog) = dialog_tree.find_mut::<Dialog>("dialog") {
                    dialog.open();
                }
                dialog_tree.focus("confirm");
            }
        }

        main_tree.tick(delta);
        if dialog_open {
            dialog_tree.tick(delta);
        }
    }

    terminal::disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture,
        terminal::LeaveAlternateScreen
    )?;
    Ok(())
}

fn set_status(tree: &mut Tree, message: &str) {
    if let Some(status) = tree.find_mut::<Text>("status") {
        status.set_text(message.to_string());
    }
}
