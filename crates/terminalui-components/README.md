# terminalui-components

Ready-made components: buttons, inputs, selects, tables, lists, tabs,
toasts, charts, dialogs, menus, and more. Each component is a plain view
you drop into any terminalui tree; nothing is generated.

Prefer the `terminalui` facade over depending on this crate directly; the
example below shows the facade path. If you depend on `terminalui-components`
alone, drop the `terminalui::components::` prefix and use `terminalui_components::`.

```rust
use terminalui::{node, Stack, Text, Tree};
use terminalui::components::button::{Button, ButtonVariant};

let root = node("root", Stack::vertical().children(vec![
    node("title", Text::new("hello")),
    node("ok", Button::new(ButtonVariant::Primary, "OK")),
]));
let _tree = Tree::new(root);
```

License: MIT
