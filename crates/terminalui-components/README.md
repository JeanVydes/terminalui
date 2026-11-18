# terminalui-components

Ready-made components: buttons, inputs, selects, tables, lists, tabs,
toasts, charts, dialogs, menus, and more. Each component is a plain view
you drop into any terminalui tree; nothing is generated.

```rust
use terminalui::{node, Stack, Text};
use terminalui_components::button::{Button, ButtonVariant};

let root = node("root", Stack::vertical().children(vec![
    node("title", Text::new("hello")),
    node("ok", Button::new(ButtonVariant::Primary, "OK")),
]));
```

License: MIT
