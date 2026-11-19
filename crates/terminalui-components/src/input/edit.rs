//! Edit mode: key handling, selection, clipboard and the caret blink/pulse
//! timer. Kept separate from painting; all state changes flow through here.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

use terminalui_core::input::InputEvent;
use terminalui_core::Context;

use super::Input;

pub(super) fn handle_event(
    input: &mut Input,
    event: &InputEvent,
    cx: &mut Context,
) -> terminalui_core::Outcome {
    let focused = if input.state.disabled {
        false
    } else {
        cx.is_focused()
    };
    input.state.focused = focused;
    if input.state.disabled {
        return terminalui_core::Outcome::Ignored;
    }
    match event {
        InputEvent::Key(key) => handle_key(input, key, focused),
        InputEvent::Paste(text) => {
            if focused {
                insert_str(input, text);
                input.clamp_view();
                terminalui_core::Outcome::Consumed
            } else {
                terminalui_core::Outcome::Ignored
            }
        }
        _ => terminalui_core::Outcome::Ignored,
    }
}

fn handle_key(input: &mut Input, key: &KeyEvent, focused: bool) -> terminalui_core::Outcome {
    if key.kind != KeyEventKind::Press {
        return terminalui_core::Outcome::Ignored;
    }
    if focused && apply_key(input, key) {
        terminalui_core::Outcome::Consumed
    } else {
        terminalui_core::Outcome::Ignored
    }
}

pub(super) fn tick_blink(input: &mut Input, delta: Duration, focused: bool) {
    if !focused || input.state.disabled {
        return;
    }
    use super::CursorMode;
    let mode = input.style.cursor_style.mode;
    if mode == CursorMode::Off {
        return;
    }
    input.blink.tick(delta);
    if input.blink.is_done() {
        input.blink_on = !input.blink_on;
        input.blink.restart();
    }
}

/// Extend the selection to `target`, anchoring at the old cursor if there is
/// no anchor yet.
fn extend_to(input: &mut Input, target: usize) {
    if input.state.anchor.is_none() {
        input.state.anchor = Some(input.state.cursor);
    }
    input.state.cursor = target;
}

/// Collapse the selection onto one of its edges, removing the anchor.
fn collapse(input: &mut Input, to_start: bool) {
    if let Some((start, end)) = input.selection() {
        input.state.cursor = if to_start { start } else { end };
    }
    input.state.anchor = None;
}

fn chars_len(input: &Input) -> usize {
    input.state.value.chars().count()
}

fn delete_selection(input: &mut Input) -> bool {
    let Some((start, end)) = input.selection() else {
        return false;
    };
    let mut chars: Vec<char> = input.state.value.chars().collect();
    chars.drain(start..end);
    input.state.value = chars.into_iter().collect();
    input.state.cursor = start;
    input.state.anchor = None;
    true
}

fn insert_str(input: &mut Input, text: &str) {
    delete_selection(input);
    let chars: Vec<char> = input.state.value.chars().collect();
    let position = input.state.cursor.min(chars.len());
    let mut chars = chars;
    chars.splice(position..position, text.chars());
    input.state.value = chars.into_iter().collect();
    input.state.cursor = position + text.chars().count();
    input.state.anchor = None;
}

fn select_all(input: &mut Input) {
    input.state.anchor = Some(0);
    input.state.cursor = chars_len(input);
}

fn select_current_line(input: &mut Input) {
    let (line, _) = input.cursor_line_col();
    let start = input.line_start(line);
    input.state.anchor = Some(start);
    input.state.cursor = start + input.line_len(line);
}

fn copy_selection(input: &mut Input) -> bool {
    let Some((start, end)) = input.selection() else {
        return false;
    };
    let text: String = input
        .state
        .value
        .chars()
        .skip(start)
        .take(end - start)
        .collect();
    let _ = terminalui_core::osc52_write(&text);
    input.clipboard = Some(text);
    true
}

fn cut_selection(input: &mut Input) -> bool {
    if !copy_selection(input) {
        return false;
    }
    delete_selection(input);
    true
}

fn paste_internal(input: &mut Input) -> bool {
    let Some(text) = input.clipboard.clone() else {
        return false;
    };
    insert_str(input, &text);
    true
}

fn apply_key(input: &mut Input, key: &KeyEvent) -> bool {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let code = key.code;

    if input.edit_mode && control {
        match code {
            KeyCode::Char('a') => {
                select_all(input);
                input.clamp_view();
                return true;
            }
            KeyCode::Char('l') => {
                select_current_line(input);
                input.clamp_view();
                return true;
            }
            KeyCode::Char('c') => return copy_selection(input),
            KeyCode::Char('x') => {
                let cut = cut_selection(input);
                input.clamp_view();
                return cut;
            }
            KeyCode::Char('v') => {
                let pasted = paste_internal(input);
                input.clamp_view();
                return pasted;
            }
            _ => {}
        }
    }

    match code {
        KeyCode::Backspace => {
            if delete_selection(input) {
                input.clamp_view();
                return true;
            }
            if input.state.cursor > 0 {
                let chars: Vec<char> = input.state.value.chars().collect();
                let mut chars = chars;
                chars.remove(input.state.cursor - 1);
                input.state.value = chars.into_iter().collect();
                input.state.cursor -= 1;
                input.clamp_view();
                return true;
            }
            false
        }
        KeyCode::Delete => {
            if delete_selection(input) {
                input.clamp_view();
                return true;
            }
            let chars: Vec<char> = input.state.value.chars().collect();
            if input.state.cursor < chars.len() {
                let mut chars = chars;
                chars.remove(input.state.cursor);
                input.state.value = chars.into_iter().collect();
                input.clamp_view();
                return true;
            }
            false
        }
        KeyCode::Left | KeyCode::Right if shift => {
            let target = match code {
                KeyCode::Left => input.state.cursor.saturating_sub(1),
                _ => (input.state.cursor + 1).min(chars_len(input)),
            };
            extend_to(input, target);
            input.clamp_view();
            true
        }
        KeyCode::Left => {
            if input.selection().is_some() {
                collapse(input, true);
            } else {
                input.state.cursor = input.state.cursor.saturating_sub(1);
            }
            input.clamp_view();
            true
        }
        KeyCode::Right => {
            if input.selection().is_some() {
                collapse(input, false);
            } else {
                input.state.cursor = (input.state.cursor + 1).min(chars_len(input));
            }
            input.clamp_view();
            true
        }
        KeyCode::Up | KeyCode::Down if input.multiline && shift => {
            let delta = if code == KeyCode::Up { -1i32 } else { 1i32 };
            move_line_extended(input, delta);
            input.clamp_view();
            true
        }
        KeyCode::Up | KeyCode::Down if input.multiline => {
            collapse(input, code == KeyCode::Up);
            move_line(input, if code == KeyCode::Up { -1i32 } else { 1i32 });
            input.clamp_view();
            true
        }
        KeyCode::Home => {
            let target = if input.multiline {
                input.line_start(input.cursor_line_col().0)
            } else {
                0
            };
            if shift {
                extend_to(input, target);
            } else {
                collapse(input, true);
                input.state.cursor = target;
            }
            input.clamp_view();
            true
        }
        KeyCode::End => {
            let target = if input.multiline {
                let (line, _) = input.cursor_line_col();
                input.line_start(line) + input.line_len(line)
            } else {
                chars_len(input)
            };
            if shift {
                extend_to(input, target);
            } else {
                collapse(input, false);
                input.state.cursor = target;
            }
            input.clamp_view();
            true
        }
        KeyCode::Enter if input.multiline => {
            insert_str(input, "\n");
            input.clamp_view();
            true
        }
        KeyCode::Char(character) => {
            if control {
                return false;
            }
            insert_str(input, &character.to_string());
            input.clamp_view();
            true
        }
        _ => false,
    }
}

fn move_line(input: &mut Input, delta: i32) {
    let (line, col) = input.cursor_line_col();
    let target = line as i32 + delta;
    if target < 0 || target >= input.line_count() as i32 {
        return;
    }
    let target = target as usize;
    let line_start = input.line_start(target);
    let target_col = col.min(input.line_len(target));
    input.state.cursor = line_start + target_col;
}

fn move_line_extended(input: &mut Input, delta: i32) {
    let anchor = input.state.anchor.unwrap_or(input.state.cursor);
    move_line(input, delta);
    input.state.anchor = Some(anchor);
}
