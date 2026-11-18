# Use cases

Recipes for common terminal-UI shapes, each pointing at the pieces that
build it. Every snippet assumes `terminalui` as the dependency (see the
README). All examples listed here live under `crates/terminalui/examples/`
and run with `cargo run --example <name>`.

## A single screen with a few widgets

Declare the whole screen as one tree; nothing is fetched at runtime, the
tree is the layout. Use `Stack` for vertical or horizontal flow and give
each child an id.

```rust
let root = node("root", Stack::vertical().gap(1).children(vec![
    node("title", Text::new("deploy").with_style(TextStyle::default().bold())),
    node("status", Badge::new("healthy").with_variant(BadgeVariant::Success)),
    node("go", Button::new(ButtonVariant::Primary, "Deploy")),
]));
```

Example: `counter.rs`.

## A form with validation

`Form` owns the label/input rows, marks required fields and buffers the
submitted pairs for the host. `ENTER` submits; `Form::take_submitted`
drains the outbox once per successful validation.

Example: `form.rs`.

## A modal confirmation

Keep the dialog as a second tree with its own focus. Render it after the
main tree so it covers it, route events to it only while open, and drain
its buttons with `take_pressed`. `ESC` should cancel; confirm/cancel are
just buttons.

Example: `dialog.rs`.

## A wizard or stepped flow

`Stepper` renders the steps (pending/active/done/failed); the host drives
them with `set_steps` or by replacing `Step::status`. Pair it with a
`Form` per step and a `Button` row; each step is a child view the tree
already knows how to lay out.

## Live dashboards and ticking data

Components that display data expose `set_*` methods for incremental
updates instead of rebuilding the tree: `Table::set_rows`-style pushes
on charts (`CandleChart::update_candle`, `LineChart::push`), `Spinner`
and `Progress` animate from `tick`. The host pushes a value per frame;
the view owns easing and glyph selection.

## Entrance animations

Render the tree into a scratch `Buffer`, then composite it with
`Effect::apply(scratch, frame, area, theme, progress)` driven by an
`Animation` ticked with the frame delta. `SPACE` replays.

Example: `animation.rs`.

## Notifications

`Toast` is a floating box with a kind (info/success/warning/destructive),
a TTL and the same composed-children contract as everything else. Tick it
every frame; read `expired()` to know when to drop it from the tree.

## Logs and streaming output

A host appends `Text` children or backing data to a scrollable view and
replaces the visible window per frame. The tree reconciles what changed;
no re-mount, no flicker. For no-op rebuilds, push new data through the
component instead of re-declaring nodes.

## Resizable splits

`ResizableGrid` gives two panes with a keyboard-resizable divider; nest two
of them for a grid with four cells. The host reads `state.divider` on
every frame; the grid owns dragging, min sizes and hit-testing.

## System status chrome

`StatusBar` pins segments to the left and right edges; `Kbd` renders key
caps; `Breadcrumb` renders composed segments; `Tabs` with a
`Tabs::with_style` highlight. Compose them at the bottom of the root
stack like any other child.

## Custom views

When a component does not exist, implement `View` for a new leaf and drop
it in the tree; no registration step needed. See
[custom-views.md](custom-views.md) and `meter.rs`.
