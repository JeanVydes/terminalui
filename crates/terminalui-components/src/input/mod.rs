//! Text input with an optional edit mode and a configurable caret.
//!
//! Edit mode (`edit_mode`, on by default) adds the familiar editing half:
//! Shift+arrows extends a selection, Ctrl+A selects everything, Ctrl+X/C/V
//! cut/copy/paste, Ctrl+L selects the current line, backspace/delete remove
//! a selection when there is one, and paste replaces the selection. Turn it
//! off with `.edit_mode(false)` to get the minimal original behavior.
//!
//! The caret is configurable via [`CursorStyle`] (shape, blink mode, blink
//! speed, opacity, easing). System clipboard access goes through OSC 52:
//! `copy` and `cut` write to the terminal's clipboard, and `paste` from the
//! system uses the terminal's bracketed-paste path (`Ctrl+Shift+V` or your
//! terminal's paste key) via [`terminalui_core::input::InputEvent::Paste`].
//! Ctrl+V pastes from the internal copy buffer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use serde::Serialize;
use std::any::Any;
use std::time::Duration;

pub use super::cursor::{CursorMode, CursorShape, CursorStyle};
use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

pub use super::chrome::{Border, Radius};

use super::scroll::ScrollbarStyle;

mod edit;
mod render;

/// Public state of the input: area, text, cursor, focus and scroll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InputState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// The current text content.
    pub value: String,
    /// Cursor position in chars, counted across newlines.
    pub cursor: usize,
    /// Whether the input would hold focus by the tree.
    pub focused: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
    /// First visible line in multi-line mode.
    pub scroll: usize,
    /// Horizontal scroll in single-line mode when the text overflows.
    pub scroll_x: usize,
    /// Selection anchor (char index). The cursor is the moving edge;
    /// when anchor and cursor coincide there is no selection.
    pub anchor: Option<usize>,
}

/// Per-instance overrides for an input. Defaults match the theme.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InputStyle {
    /// Background of the box. Defaults to `theme.background`.
    pub background: Option<Color>,
    /// Text color. Defaults to `theme.foreground` (muted when disabled).
    pub foreground: Option<Color>,
    /// Placeholder color. Defaults to `theme.muted`.
    pub placeholder: Option<Color>,
    /// Caret color (accent used by the caret). Defaults to `theme.accent`.
    pub cursor: Option<Color>,
    /// Border of the box. Default: none.
    pub border: Option<Border>,
    /// Corner radius. Rounded corners when set.
    pub radius: Radius,
    /// Scrollbar for multi-line mode. Defaults to auto-visibility.
    pub scrollbar: ScrollbarStyle,
    /// Caret presentation.
    pub cursor_style: CursorStyle,
}

impl Default for InputStyle {
    fn default() -> Self {
        Self {
            background: None,
            foreground: None,
            placeholder: None,
            cursor: None,
            border: None,
            radius: Radius::None,
            scrollbar: ScrollbarStyle::default(),
            cursor_style: CursorStyle::default(),
        }
    }
}

/// A single- or multi-line text input.
///
/// The host decides when this view is focused (a tree can have many
/// focusable inputs); events drive cursor, selection and editing.
#[derive(Serialize)]
pub struct Input {
    /// Public state: area, text, cursor, focus and scroll.
    pub state: InputState,
    /// Per-instance overrides; defaults match the theme.
    pub style: InputStyle,
    /// Whether the input renders and edits multiple lines.
    pub multiline: bool,
    /// Edit mode: selection with Shift+arrows, Ctrl+A/C/X/V, Ctrl+L, and
    /// selection-aware deletion/paste. Default on, as standard everywhere.
    pub edit_mode: bool,
    /// Internal copy buffer for Ctrl+C/Ctrl+X, drained by Ctrl+V.
    /// Serialized as part of the view for transparency, though it is
    /// ephemeral.
    pub clipboard: Option<String>,
    /// Pulse/blink timer for the caret.
    #[serde(skip)]
    pub blink: terminalui_animations::Animation,
    /// Current caret phase (visible vs hidden) for blink/pulse modes.
    #[serde(skip)]
    pub blink_on: bool,
    /// Serialized as `children` so every node exposes a uniform child key.
    #[serde(rename = "children")]
    placeholder: Vec<Node>,
}

impl Input {
    /// Builder shorthand: set the placeholder text shown when empty.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = vec![node("placeholder", Text::new(placeholder))];
        self
    }

    /// Replace the composed placeholder with a plain text one.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = vec![node("placeholder", Text::new(placeholder))];
    }

    /// Creates an empty single-line input.
    pub fn new() -> Self {
        Self {
            state: InputState {
                area: Rect::new(0, 0, 0, 0),
                value: String::new(),
                cursor: 0,
                focused: false,
                disabled: false,
                scroll: 0,
                scroll_x: 0,
                anchor: None,
            },
            style: InputStyle::default(),
            multiline: false,
            edit_mode: true,
            clipboard: None,
            blink: terminalui_animations::Animation::new(
                Duration::from_millis(530),
                terminalui_animations::Easing::EaseInOutSine,
            ),
            blink_on: true,
            placeholder: Vec::new(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: InputStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the line mode; multi-line inputs wrap and scroll vertically.
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Composed placeholder: any views, usually an icon plus a text.
    pub fn with_placeholder_children(mut self, children: Vec<Node>) -> Self {
        self.placeholder = children;
        self
    }

    /// Enable or disable the edit mode (selection, clipboard ops and
    /// advanced cursor keys). Default on.
    pub fn edit_mode(mut self, edit_mode: bool) -> Self {
        self.edit_mode = edit_mode;
        self
    }

    /// Builder shorthand: set the caret style.
    pub fn with_cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.style.cursor_style = cursor_style;
        self
    }

    /// Replace the whole text; cursor moves to the end.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.state.value = value.into();
        self.state.cursor = self.state.value.chars().count();
        self.state.anchor = None;
        self.state.scroll = 0;
        self.state.scroll_x = 0;
    }

    /// Clear the text, selection and scroll state.
    pub fn clear(&mut self) {
        self.state.value.clear();
        self.state.cursor = 0;
        self.state.anchor = None;
        self.state.scroll = 0;
        self.state.scroll_x = 0;
    }

    /// The ordered selection range in chars, if any.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.state.anchor?;
        let (start, end) = if anchor <= self.state.cursor {
            (anchor, self.state.cursor)
        } else {
            (self.state.cursor, anchor)
        };
        (start != end).then_some((start, end))
    }

    /// The selected text, if any.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection()?;
        Some(
            self.state
                .value
                .chars()
                .skip(start)
                .take(end - start)
                .collect(),
        )
    }

    fn line_count(&self) -> usize {
        self.state.value.split('\n').count().max(1)
    }

    /// Cursor position as `(line_index, column)` for multi-line mode.
    pub(crate) fn cursor_line_col(&self) -> (usize, usize) {
        let mut line = 0usize;
        let mut col = 0usize;
        for (index, ch) in self.state.value.chars().enumerate() {
            if index == self.state.cursor {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    pub(crate) fn line_len(&self, line: usize) -> usize {
        self.state
            .value
            .split('\n')
            .nth(line)
            .map(|candidate| candidate.chars().count())
            .unwrap_or(0)
    }

    pub(crate) fn value_slice(&self, start: usize, len: usize) -> String {
        self.state.value.chars().skip(start).take(len).collect()
    }

    /// Caret visibility in `0..=1`: mode visibility (with blink phase) times
    /// the configured opacity.
    pub(crate) fn caret_visibility(&self) -> f64 {
        use super::cursor::CursorMode;
        let mode = self.style.cursor_style.mode;
        let phase = match mode {
            CursorMode::Off => 1.0,
            CursorMode::Blink => {
                if self.blink_on {
                    1.0
                } else {
                    0.0
                }
            }
            CursorMode::Pulse => {
                let progress = self.blink.progress();
                if self.blink_on {
                    1.0 - progress
                } else {
                    progress
                }
            }
        };
        phase * self.style.cursor_style.opacity
    }

    pub(crate) fn line_start(&self, line: usize) -> usize {
        self.state
            .value
            .split('\n')
            .take(line)
            .map(|candidate| candidate.chars().count() + 1)
            .sum()
    }

    fn clamp_view(&mut self) {
        let (line, _) = self.cursor_line_col();
        if line < self.state.scroll {
            self.state.scroll = line;
            return;
        }
        let height = self.state.area.height as usize;
        if height > 0 && line >= self.state.scroll + height {
            self.state.scroll = line + 1 - height;
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Input {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "input"
    }

    fn outline(&self) -> Option<String> {
        let selected = self
            .selection()
            .map(|(start, end)| (end - start).to_string());
        match selected {
            Some(count) => Some(format!(
                "value={}, cursor={}, disabled={}, selected={}",
                quoted(&self.state.value),
                self.state.cursor,
                self.state.disabled,
                count
            )),
            None => Some(format!(
                "value={}, cursor={}, disabled={}",
                quoted(&self.state.value),
                self.state.cursor,
                self.state.disabled
            )),
        }
    }

    fn children(&self) -> &[Node] {
        &self.placeholder
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.placeholder
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        render::paint(self, area, buf, cx, theme);
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        edit::handle_event(self, event, cx)
    }

    fn tick(&mut self, delta: Duration, cx: &mut Context) {
        let focused = cx.is_focused();
        edit::tick_blink(self, delta, focused);
    }

    fn focusable(&self) -> bool {
        !self.state.disabled
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        !self.state.disabled
    }
}
