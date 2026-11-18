# terminalui-core

The view-tree runtime. Retained-mode rendering for the terminal on top of
ratatui: typed views, focus order, event dispatch, layout, and JSON-capable
serialization of the whole tree.

Prefer the `terminalui` facade over depending on this crate directly; the
example below shows the facade path. If you depend on `terminalui-core`
alone, drop the `terminalui::` prefix and use `terminalui_core::`.

```rust
use terminalui::{node, Stack, Text, Tree};

let root = node("root", Stack::vertical().children(vec![
    node("title", Text::new("hello")),
]));
let _tree = Tree::new(root);
```

License: MIT
