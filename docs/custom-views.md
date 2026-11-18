# Writing custom views

Every view in terminalui, from `Text` to `Dialog`, implements the same
contract: the `View` trait (see `crates/terminalui-core/src/view.rs`).
Ready-made
components are not special; they are just views that already exist. When
none of them fits, implement `View` yourself.

The minimal custom view is a display-only leaf: it paints something, never
holds focus, and never handles events. The example below is `Spacer`, the
smallest real view in the codebase.

```rust
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use serde::Serialize;

use terminalui::{node, Context, InputEvent, Outcome, Theme, Tree, View};

#[derive(Serialize)]
pub struct Spacer;

impl View for Spacer {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }
    fn kind(&self) -> &'static str {
        "spacer"
    }
    fn render(&mut self, _area: Rect, _buf: &mut Buffer, _cx: &mut Context, _theme: &Theme) {}
    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
```

## The contract

### `kind`

A stable string tag for the outline dump and the JSON serialization
(`Tree::inspect_outline`, `Tree::to_json`). Pick something short and
singular: `"meter"`, `"sparkline"`, `"marquee"`.

### Serialization

`View` requires `Serialize`. Every field gets serialized; anything that
must not be serialized (hit-area caches, `item_hits`, callbacks) needs
`#[serde(skip)]`. This is why every public state struct in the component
library derives `Serialize` and every component has a `kind`.

### `render`

Called once per frame with the rectangle the view owns and the shared
`Buffer` (see `ratatui::buffer::Buffer`). Paint cells with `buf.set_stringn`,
`buf.set_string` or `Buffer::set_style`; read colors from
`&Theme` (`theme.primary`, `theme.muted`, ...) so the view follows the
user's palette. The view never clears `area` itself: either paint every
cell you own, or let the background show through.

### `measure`

Returns the intrinsic size in cells, width first, or `None` to fill
whatever the parent gives you. In a `Stack::vertical`, a child that
measures `Some((width, n))` takes exactly `n` rows; a child without a
measurement splits the remaining rows equally with every other child.
`Stack::horizontal` uses the width the same way. Return honest sizes:
components that guess leave gaps or overlap their siblings.

### `outline`

A one-line summary shown by `Tree::inspect_outline`, e.g.
`Text::outline` returns `text="the label"`. Optional but strongly
recommended; it is what makes trees debuggable without a debugger.

## Interactive views

Focusable views must increment the focus counter exactly once per
render and per event, and in the same order both times.

```rust
fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
    self.state.focused = cx.is_focused();
    // ...
}

fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
    self.state.focused = cx.is_focused();
    if !self.state.focused {
        return Outcome::Ignored;
    }
    // ...
}

fn focusable(&self) -> bool {
    true
}
```

Rules:

- `cx.is_focused()` is the preorder rank of the focused leaf in the tree.
  Every focusable view calls it exactly once per walk; no more, no less.
- Focus flows in preorder DFS order, so `Stack` ordering is tab order.
- `handle_event` must not consume focus-bearing events it does not own;
  return `Outcome::Ignored` so the tree can move focus to the next view.
- `Outcome::Consumed` claims the event; nothing else (including the
  tree) sees it.
- When the view is activated by keyboard, mouse click or `tree.press`,
  follow the components' pattern: set a `pressed` flag in public state
  and let the host drain it with a `take_pressed`-style method. Never
  hold host state inside the view beyond its own interaction state.

## Composing views

A view that owns children implements `children()` and `children_mut()`;
`Spacer` and `Text` return empty slices because they are leaves.
`Stack` returns its `Vec<Node>`. Push children through
`terminalui::node("id", view)` so the tree can find them by id.

As a rule of thumb: views own geometry, input mechanics and visuals.
The host owns meaning. A button view knows how to render a pressed
state; it does not know what the press confirms.

## Following the components

Read the components in `crates/terminalui-components/src` before writing
a new one; most have a simpler cousin. Small examples to study first:

- `switch.rs`: minimal focusable leaf with mouse and keyboard.
- `kbd.rs`: display-only leaf with `measure` and `outline`.
- `floating.rs`: composing a `Chrome` over arbitrary children.

`crates/terminalui/examples/meter.rs` is a complete custom view (a
horizontal bar) written against this contract alone; `cargo run
--example meter` runs it.
