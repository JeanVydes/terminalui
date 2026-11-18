# terminalui for agents

This page is for AI agents that build, drive or inspect terminalui apps.
Everything here is stable, public API.

## Why this exists

Suggesting changes to a running TUI by reading pixels is slow and
fragile. terminalui trees can explain themselves: every view carries a
stable `kind` tag, the host chooses machine-meaningful node ids, and the
tree can serialize its structure and state at any moment.

## Inspecting a tree

`Tree::inspect_outline()` returns a compact one-line-per-node outline.
Format: `<id> [<kind>] {<outline state>}`, values quoted and truncated to
48 characters, focus on the first line.

```text
focus=none
root [stack] {direction='vertical'}
├─ title [text] {text='counter'}
├─ count [text] {text='count: 0'}
├─ minus [button] {pressed=false, disabled=false, loading=false}
└─ plus [button] {pressed=false, disabled=false, loading=false}
```

`Tree::to_json()` / `to_json_pretty()` produce the full machine-readable
state: `{ "focus": <id|null>, "root": <Node> }` where each node is
`{ "id": ..., "kind": ..., "view": { ...view fields, "children": [...] } }`
(children live inside the view's own object). Use the JSON
when you need values; use the outline when you need structure.

## Stable conventions

- `focus` is the id of the focused view, or `null`.
- `kind` strings are stable across releases (`text`, `button`, `stack`,
  `spacer`, ...). They are the `View::kind()` return value.
- Node ids are chosen by the host; in generated apps prefer short,
  kebab-or-snake ids that name the role (`count`, `delete-button`).
- Activation state is drained, not read-and-reset: buttons expose
  `take_pressed()`, menus and palettes expose `take_activated()`.
  Consume exactly once per event loop iteration.
- `outline` output is lossy by design (48 chars, one line); never parse
  it as a general protocol. Use JSON for values.

## Driving a tree headlessly

No terminal is required to exercise the tree:

```rust
let mut tree = Tree::new(root);
// dispatch events directly
tree.handle_event(&InputEvent::Key(KeyEvent {
    code: KeyCode::Enter,
    kind: KeyEventKind::Press,
    modifiers: KeyModifiers::NONE,
}));
// read state
assert!(tree.find_mut::<Button>("ok").unwrap().take_pressed());
// advance animations
tree.tick(Duration::from_millis(16));
```

Rendering is only needed when you want pixels; serialization, focus
queries and event dispatch all work without a frame.

## Gotchas

- Focus is assigned by preorder rank. If you insert a focusable view
  before an existing one, every `cx.is_focused()` decision after it
  shifts; ids are the stable reference, indices are not.
- `handle_event` is root-to-leaf in the tree: a view that returns
  `Consumed` stops the dispatch.
- Two focus walks (render and event) must count focusables in the same
  order, or focus lands on the wrong view.
- `Tree::press(id)` activates programmatically without synthesizing a
  keyboard event.

## Contributing as an agent

See `CONTRIBUTING.md` for the verification suite (fmt, clippy with
`-D warnings`, tests, missing-docs) and the style rules it enforces.
In particular, docs must describe the present tense of the code, never
its history.
