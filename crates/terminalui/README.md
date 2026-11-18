# terminalui

The facade crate for the terminalui toolkit. Re-exports the three crates
that make it up so you can depend on a single package.

Distributed on git, tracking `main`:

```toml
[dependencies]
terminalui = { git = "https://github.com/JeanVydes/terminalui" }
```

It re-exports:

- `terminalui-core`: the retained view tree, focus, layout and events.
- `terminalui-animations`: easings and buffer effects.
- `terminalui-components`: ready-made components.

```rust
use terminalui::{node, Stack, Text, Tree};
use terminalui::components::button::{Button, ButtonVariant};

let root = node("root", Stack::vertical().children(vec![
    node("ok", Button::new(ButtonVariant::Primary, "OK")),
]));
let _tree = Tree::new(root);
```

License: MIT
