# terminalui

**A standalone Rust toolkit for terminal interfaces.** terminalui builds on
ratatui and provides a typed view tree for layout, focus, keyboard and mouse
input, reusable components, and animations. Use it to build terminal
applications in Rust.

## Add the dependency

```toml
[dependencies]
terminalui = { git = "https://github.com/JeanVydes/terminalui", branch = "main" }
```

The facade crate re-exports the core view tree, animation effects, and
components. You can also depend on the individual crates when you only need a
part of the toolkit.

## Build a view tree

```rust
use terminalui::{node, Stack, Text, Tree};
use terminalui::components::button::{Button, ButtonVariant};

let root = node(
    "root",
    Stack::vertical().gap(1).children(vec![
        node("title", Text::new("My terminal app")),
        node("confirm", Button::new(ButtonVariant::Primary, "Continue")),
    ]),
);
let mut tree = Tree::new(root);
```

Render the tree from your ratatui frame and forward input events to it. The
tree handles layout, focus order, and dispatch to the view under the cursor.
Your application owns the event loop and its domain state.

## Components and animations

The library includes inputs, buttons, forms, tables, menus, dialogs, overlays,
toasts, progress indicators, charts, scroll views, and other terminal
components. Compose them with the same view tree. Stateful components expose
methods for updating their displayed data between frames.

Use the animation crate for transitions and effects that operate on ratatui
buffers. Tick views and animations from your application loop.

## Runnable examples

Run an example from the workspace root with:

```sh
cargo run -p terminalui --example counter
```

Examples include a counter, a custom view, a form, a dialog, and an entrance
animation.

## Documentation

- [Quickstart](docs/quickstart.md)
- [Use cases](docs/use-cases.md)
- [Custom views](docs/custom-views.md)
- [Tree inspection for agents and tools](docs/agents.md)
- [Contributing](CONTRIBUTING.md)

Requires Rust 1.94 or newer.

## License

MIT. See [LICENSE](LICENSE).
