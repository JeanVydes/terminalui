# terminalui-animations

Easings and buffer effects for terminalui: fades, slides, bounces,
dissolves, radial flares, bidirectional flares, and a driver to run them.

```rust
use std::time::Duration;
use terminalui_animations::{Animation, Effect, render_with};

let mut anim = Animation::new(Duration::from_millis(600), terminalui_animations::Easing::EaseOutCubic);
terminalui_animations::render_with(
    &Effect::FadeIn,
    anim.progress(),
    buf,
    area,
    theme,
    |_area, buf, theme| { /* paint your component */ },
);
```

License: MIT
