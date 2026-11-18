# Quickstart

From an empty directory to an interactive terminal app in a few minutes.
This guide is for users of the library; everything it uses is stable
public API.

## 1. Depend on the facade

The crate is distributed on git, not crates.io:

```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"

[dependencies]
terminalui = { git = "https://github.com/JeanVydes/terminalui" }
```

`terminalui` re-exports the whole toolkit, so this is the only dependency
you need. See `README.md` for the pieces if you want one crate alone.

## 2. Declare the screen

A terminalui app is a tree of typed views. Build it once; change it by
changing state, not by re-mounting widgets.

```rust
use terminalui::{node, Stack, Text, Tree};
use terminalui::components::button::{Button, ButtonVariant};

let root = node("root", Stack::vertical().gap(1).children(vec![
    node("title", Text::new("my app")),
    node("ok", Button::new(ButtonVariant::Primary, "OK")),
]));
let mut tree = Tree::new(root);
```

Ids (`"title"`, `"ok"`) are how the host finds views later:
`tree.find_mut::<Button>("ok")`.

## 3. Run the loop

terminalui owns layout, focus and events; the host owns the loop:

```rust,no_run
use std::time::Duration;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;

let mut last = std::time::Instant::now();
loop {
    let now = std::time::Instant::now();
    let delta = now - last;
    last = now;

    // paint the tree into the frame buffer
    terminal.draw(|frame| {
        let area = frame.area();
        let theme = terminalui::Theme::default();
        frame.buffer_mut().set_style(
            area,
            ratatui::style::Style::default().bg(theme.background),
        );
        tree.render(area, frame.buffer_mut(), &theme);
    })?;

    // translate crossterm events into terminalui events
    if event::poll(Duration::from_millis(16))? {
        let input = terminalui::InputEvent::from(event::read()?);
        if let terminalui::InputEvent::Key(key) = &input {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                break;
            }
        }
        tree.handle_event(&input);
    }

    // drain component state between frames
    if let Some(button) = tree.find_mut::<Button>("ok") {
        if button.take_pressed() {
            // the user activated OK
        }
    }

    tree.tick(delta);
}
```

The full, runnable version of this loop is `crates/terminalui/examples/counter.rs`.

## 4. Focus and input

Focus lives in the tree. `TAB`/`SHIFT-TAB` move it between focusable
views (buttons, inputs, selects, ...), in declaration order. The host
decides nothing about focus mechanics; it only routes events and reads
component state.

Clicks work the same way: every focusable view handles its own hit area;
the tree dispatches `Mouse` events to the view under the cursor.

## 5. Where next

- More shapes: forms, dialogs, wizards, dashboards, toasts:
  `docs/use-cases.md`.
- Writing your own view: `docs/custom-views.md`, with a complete
  worked implementation in `crates/terminalui/examples/meter.rs`.
- Runnable programs: `cargo run --example counter` and friends
  (see the README).
- Reading API docs: `cargo doc --workspace --no-deps`.
