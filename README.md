# terminalui

A retained-mode, composable view tree for the terminal, built on ratatui.

Terminal UI apps are usually a handful of stateful widgets and a dispatch
loop glued together by hand. terminalui gives them a real tree: typed views,
focus management, events, layout, and ready-made components, so the structure
of a screen can be declared once and reused.

## Install

Add `terminalui` to your `Cargo.toml`:

```toml
[dependencies]
terminalui = "0.1"
```

`terminalui` re-exports the full toolkit: the view-tree runtime, the
animations crate and the component library. Depending on it is enough.

## Example

```rust
use terminalui::{node, Stack, Text, Tree};
use terminalui::components::button::{Button, ButtonVariant};

let root = node("root", Stack::vertical()
    .gap(1)
    .children(vec![
        node("title", Text::new("terminalui")),
        node("ok", Button::new(ButtonVariant::Primary, "OK")),
    ]));
let mut tree = Tree::new(root);
```

The crates under the facade, in case you want to depend on one alone:

| Crate | Purpose |
|---|---|
| `terminalui-core` | View tree, focus, layout, events, serialization |
| `terminalui-animations` | Easings and buffer effects (fade, slide, dissolve, radial flare) |
| `terminalui-components` | Buttons, inputs, tables, selects, charts, dialogs, and the rest |

## Requirements

- Rust 2021 or newer (MSRV 1.88)
- A terminal with truecolor support is recommended but not required

## Repository

- Source: <https://github.com/JeanVydes/terminalui>
- Example app: `cargo run -p example-app`

## License

MIT. See `LICENSE`.
