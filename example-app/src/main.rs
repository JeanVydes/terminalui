//! terminalui showcase — the deploy dashboard exercises every component
//! in `terminalui-components`:
//! breadcrumb, tabs, badge, input (filter), table, pagination, form, button,
//! dialog (modal tree), icons, chart, resizable grid, list, expandable,
//! floating box, spinner, progress and toast. Tabs 5-8 demo every animation
//! effect, every presentation, every color in the theme and the full
//! component catalog.
//!
//! The app is one composable view tree per tab plus a modal dialog tree.
//! The host wires meaning to component state; components only own
//! geometry, input mechanics and visuals.
//!
//! Keys:
//!   F1 / F2 / F3 / F4 / F5 / F6 / F7 / F8  jump to a tab from anywhere
//!   1 / 2 / 3 / 4 / 5 / 6 / 7 / 8        jump to a tab (outside text fields)
//!   TAB / SHIFT-TAB    cycle focus across components in the active tree
//!   ARROWS / [ ]       resize the resizable grids (overview sidebar, market 2x2)
//!   T                  cycle theme: default / ansi16 / ansi256
//!   ESC                back to the tab bar; closes an open dialog
//!   Q                  quit (asks for confirmation)
//!
//! Run with `--dump-json FILE` to write every tab tree as JSON, or
//! `--dump-outline FILE` to write the compact outlines of every tab tree,
//! and exit without starting the terminal.

mod app;
mod catalog_tab;
mod colors_tab;
mod common;
mod data;
mod fx_tab;
mod latency;
mod market;
mod market_tab;
mod overview;
mod presentations_tab;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture,
};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use terminalui::input::InputEvent;

use app::App;

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            terminal::EnterAlternateScreen,
            EnableMouseCapture,
            // EnableMouseCapture only asks for clicks (?1000h) and drags
            // (?1002h). Hover tracking needs any-motion (?1003h) too.
            crossterm::style::Print("\x1b[?1003h"),
            EnableFocusChange,
            EnableBracketedPaste
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableBracketedPaste,
            DisableFocusChange,
            crossterm::style::Print("\x1b[?1003l"),
            DisableMouseCapture,
            terminal::LeaveAlternateScreen
        );
    }
}

enum DumpFormat {
    Json,
    Outline,
}

struct DumpRequest {
    format: DumpFormat,
    path: std::path::PathBuf,
}

/// Parse `--dump-json FILE` / `--dump-outline FILE` out of the argument list.
/// Unknown arguments are ignored so the interactive app keeps its loose parsing.
fn dump_requests(args: &[String]) -> Result<Vec<DumpRequest>, String> {
    let mut requests = Vec::new();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let format = match arg.as_str() {
            "--dump-json" => DumpFormat::Json,
            "--dump-outline" => DumpFormat::Outline,
            _ => continue,
        };
        let Some(path) = iter.next() else {
            return Err(format!("{arg} needs a FILE argument"));
        };
        requests.push(DumpRequest {
            format,
            path: std::path::PathBuf::from(path),
        });
    }
    Ok(requests)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let requests = dump_requests(&args).map_err(io::Error::other)?;
    if !requests.is_empty() {
        let app = App::new();
        for request in requests {
            let document = match request.format {
                DumpFormat::Json => app.tree_json().map_err(io::Error::other)?,
                DumpFormat::Outline => app.tree_outlines_document(),
            };
            std::fs::write(&request.path, document).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("cannot write {}: {error}", request.path.display()),
                )
            })?;
            println!("wrote {}", request.path.display());
        }
        return Ok(());
    }
    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let theme = app.theme;
            app.render(area, frame.buffer_mut(), &theme);
        })?;
        if crossterm::event::poll(Duration::from_millis(16))? {
            let event = crossterm::event::read()?;
            let input = InputEvent::from(event);
            app.handle_event(&input);
        }
        let now = Instant::now();
        let delta = now - app.last_tick;
        app.tick(delta);
        app.last_tick = now;
        if app.should_quit {
            break;
        }
    }
    Ok(())
}
