# Contributing

This file is for anyone changing terminalui: humans and AI agents alike.
It states the rules that keep the tree green for everyone who depends on
it.

## Ground rules

- Distribution is git-only, not crates.io. There is no release process
  and no versioning ceremony; `main` is what people build against.
- The license is MIT. By contributing you license your work under it.
- The minimum supported Rust is 1.94 (`rust-version` in `Cargo.toml`).
- Never commit `Cargo.lock`; this is a library workspace.

## Setup

```console
git clone https://github.com/JeanVydes/terminalui.git
cd terminalui
cargo build --workspace
```

`rust-toolchain.toml` pins the development toolchain, so `cargo` uses the
pinned version automatically; the MSRV is checked by the pinned toolchain
itself.

## The verification suite

Every change must keep all of these green, in this order:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D missing-docs" cargo doc --workspace --no-deps
```

`cargo test --workspace --all-targets` includes the example builds; a new
example that does not compile breaks CI.

## Where things live

- `crates/terminalui-core`: the view tree, focus, layout, events,
  serialization.
- `crates/terminalui-components`: ready-made views (button, table,
  select, ...). One file per component, named after the component.
- `crates/terminalui-animations`: easings and buffer effects.
- `crates/terminalui`: the facade. Re-exports the other crates; only this
  crate is meant to be depended on directly.

## Adding a component

1. One file in `crates/terminalui-components/src/`, named after the
   component, with a module doc (`//!`) describing what it paints and
   which views it composes.
2. State lives in a `pub struct XxxState` (Serialize), overrides in a
   `pub struct XxxStyle`; both get doc comments on every field.
3. Implement `View`: `kind` (stable, singular), `outline`, `measure`,
   `render`, `handle_event`. Follow the focus rules: `cx.is_focused()`
   exactly once per walk, same order in render and events.
4. Enums are `#[non_exhaustive]`. Booleans that represent states are
   enums. No raw strings where a variant exists.
5. Every public item carries a doc comment; the missing-docs build runs
   with `-D` and will fail otherwise.
6. Public interaction state (e.g. `pressed`) is drained by the host with
   a `take_*` method, never consumed inside the view.
7. Tests go in a sibling file wired with
   `#[cfg(test)] #[path = "..._tests.rs"] mod tests;` or in `tests/`;
   never inline in the component file.
8. `cargo run --example x` examples are the living docs: keep them
   compiling and lint-clean.

## Style

- Plain ASCII punctuation, no em dashes, no emojis.
- Names follow the domain vocabulary; avoid abbreviations.
- No raw `String` where a newtype or enum carries meaning.
- No `unwrap`/`expect`/`panic` in production paths.
- Comments explain the why, never the what; keep them short.
- Match the surrounding code; the smallest change that solves the problem
  is the right one.

## Commits

Short, imperative, present tense, one logical change. Examples:

```
Add breadcrumb component
Fix focus order in horizontal stacks
Docs: cover serialization in quickstart
```

## For agents

If you are an AI agent working in this repository, in addition to the
rules above:

- Read the surrounding code before writing new code; match its
  conventions exactly.
- Do not perform scripted bulk edits across files; edit file by file.
- The `AGENTS.md` in the working tree (not shipped on git) carries the
  full house rules; where it disagrees with this file, `AGENTS.md` wins
  on style, this file wins on process.
- Run the verification suite before reporting done.
- `Tree::inspect_outline` and `Tree::to_json` are the preferred way to
  understand an existing tree: they are machine-readable and stable. See
  `docs/agents.md`.
