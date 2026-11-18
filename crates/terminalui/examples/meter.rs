//! A custom view written from scratch against the raw `View` trait.
//!
//! Run with `cargo run --example meter`.
//!
//! TAB focuses the meter, LEFT/RIGHT adjust it, HOME/END jump to the
//! ends, and `q` quits.

use std::any::Any;
use std::io;
use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use serde::Serialize;

use terminalui::{node, Context, InputEvent, Outcome, Text, Theme, Tree, View};

const METER_WIDTH: u16 = 24;
const STEP: u16 = 5;

/// A horizontal bar meter, implemented without any ready-made component.
#[derive(Debug, Clone, Serialize)]
pub struct Meter {
    value: u16,
}

impl Meter {
    pub fn new(value: u16) -> Self {
        Self {
            value: value.min(100),
        }
    }

    pub fn value(&self) -> u16 {
        self.value
    }
}

impl View for Meter {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "meter"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("value={}", self.value))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((METER_WIDTH, 1))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let focused = cx.is_focused();
        let filled = (self.value as usize * METER_WIDTH as usize / 100).min(METER_WIDTH as usize);
        let mut style = Style::default().fg(theme.primary);
        if focused {
            style = style.add_modifier(Modifier::BOLD);
        }
        for column in 0..METER_WIDTH {
            let (glyph, color) = if (column as usize) < filled {
                if focused {
                    ("█", theme.accent)
                } else {
                    ("█", theme.primary)
                }
            } else {
                ("░", theme.muted)
            };
            buf.set_stringn(area.x + column, area.y, glyph, 1, style.fg(color));
        }
        let label = format!(" {value:>3}%", value = self.value);
        buf.set_stringn(
            area.x + METER_WIDTH,
            area.y,
            label,
            u16::MAX as usize,
            style.fg(theme.foreground),
        );
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        if !focused {
            return Outcome::Ignored;
        }
        let InputEvent::Key(key) = event else {
            return Outcome::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Left => {
                self.value = self.value.saturating_sub(STEP);
                Outcome::Consumed
            }
            KeyCode::Right => {
                self.value = (self.value + STEP).min(100);
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.value = 0;
                Outcome::Consumed
            }
            KeyCode::End => {
                self.value = 100;
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}

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
        terminalui::Stack::vertical().gap(1).children(vec![
            node(
                "title",
                Text::new("meter (custom view)")
                    .with_style(terminalui::TextStyle::default().bold()),
            ),
            node(
                "hint",
                Text::new("left/right adjust, home/end jump, tab focus, q quit"),
            ),
            node("meter", Meter::new(40)),
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
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
            tree.handle_event(&input);
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
