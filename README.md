# terminalui

Terminal UIs as a typed, composable view tree, built on ratatui.

## Install

```toml
[dependencies]
terminalui = { git = "https://github.com/JeanVydes/terminalui" }
```

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

## Examples

All examples ship with the repo and run with `cargo run --example <name>`:

```console
counter    # minimal interactive counter
meter      # a custom view written against the View trait
form       # required-field form with submit handling
dialog     # modal confirmation with a second view tree
animation  # entrance animation with Effect
```

## Docs

- `docs/quickstart.md`: from a git dependency to a running app.
- `docs/use-cases.md`: recipes for dashboards, forms, modals, wizards, toasts and more.
- `docs/custom-views.md`: implement the `View` trait to add your own views.
- `docs/agents.md`: machine-readable tree inspection for agents and tools.
- `CONTRIBUTING.md`: verification suite, component conventions, style.

Requires Rust 1.94 or newer.

## License

MIT. See [LICENSE](LICENSE).
