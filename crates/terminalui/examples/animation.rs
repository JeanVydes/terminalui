//! Entrance animations driven by terminalui-animations.
//!
//! Run with `cargo run --example animation`.
//!
//! A panel slides in from the right with an ease-out curve. SPACE replays
//! the animation, `q` quits.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Style;

use terminalui::{node, Stack, Text, TextStyle, Theme, Tree};
use terminalui::{Animation, Direction, Easing, Effect, InputEvent};

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(
        io::stdout(),
        terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    let root = node(
        "root",
        Stack::vertical().gap(1).children(vec![
            node(
                "title",
                Text::new("terminalui").with_style(TextStyle::default().bold()),
            ),
            node(
                "message",
                Text::new("animated entrance, restart with space"),
            ),
            node("hint", Text::new("space: replay, q: quit")),
        ]),
    );
    let mut tree = Tree::new(root);

    let effect = Effect::SlideIn {
        direction: Direction::Right,
        distance: 24,
        easing: Easing::EaseOutCubic,
    };
    let mut anim = Animation::new(Duration::from_millis(650), Easing::EaseOutCubic);
    let mut last = Instant::now();

    loop {
        let now = Instant::now();
        let delta = now - last;
        last = now;

        let theme = Theme::default();
        terminal.draw(|frame| {
            let area = frame.area();
            let mut scratch = Buffer::empty(area);
            scratch.set_style(area, Style::default().bg(theme.background));
            tree.render(area, &mut scratch, &theme);
            effect.apply(&scratch, frame.buffer_mut(), area, &theme, anim.progress());
        })?;

        if event::poll(Duration::from_millis(16))? {
            let input = InputEvent::from(event::read()?);
            if let InputEvent::Key(key) = &input {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(' ') => anim.restart(),
                        _ => {}
                    }
                }
            }
        }

        anim.tick(delta);
        tree.tick(delta);
    }

    terminal::disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture,
        terminal::LeaveAlternateScreen
    )?;
    Ok(())
}
