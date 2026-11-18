# terminalui-animations

Easings and buffer effects for terminalui: fades, slides, bounces,
dissolves, radial flares, bidirectional flares, and a driver to run them.

Prefer the `terminalui` facade over depending on this crate directly; the
example below shows the facade path. If you depend on `terminalui-animations`
alone, drop the `terminalui::` prefix and use `terminalui_animations::`.

```rust
use std::time::Duration;
use terminalui::{Animation, Easing, Effect, render_with};

let mut anim = Animation::new(Duration::from_millis(600), Easing::EaseOutCubic);
render_with(
    &Effect::FadeIn,
    anim.progress(),
    buf,
    area,
    theme,
    |_area, buf, theme| { /* paint your component */ },
);
```

License: MIT
