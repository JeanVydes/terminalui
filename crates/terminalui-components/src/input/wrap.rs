//! Soft wrapping: logical lines split into visual rows that fit the width.
//!
//! The multiline input paints visual rows, not logical ones. Every row
//! carries the global char offset of its first cell so selection and caret
//! math stay in the same coordinate space as [`crate::components::input::InputState::cursor`].

/// A visual row: the global char offset of its first cell and its text.
pub(super) struct VisualRow {
    /// Global char offset of the row's first cell.
    pub(super) global: usize,
    /// The row's text, at most `width` chars.
    pub(super) text: String,
}

/// Split `value` into rows of at most `width` chars.
///
/// Wraps at char boundaries. An empty line yields one empty row. The newline
/// itself occupies one global char offset between lines.
pub(super) fn visual_rows(value: &str, width: usize) -> Vec<VisualRow> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut global = 0usize;
    for line in value.split('\n') {
        let mut row = String::new();
        let mut row_global = global;
        for ch in line.chars() {
            if row.chars().count() >= width {
                rows.push(VisualRow {
                    global: row_global,
                    text: std::mem::take(&mut row),
                });
                row_global = global;
            }
            row.push(ch);
            global += 1;
        }
        rows.push(VisualRow {
            global: row_global,
            text: row,
        });
        // The newline takes exactly one global offset.
        global += 1;
    }
    rows
}

/// Where the caret sits in visual coordinates.
///
/// A caret at the end of a full row belongs to the next row's first cell.
pub(super) fn visual_cursor(rows: &[VisualRow], width: usize, cursor: usize) -> (usize, usize) {
    let width = width.max(1);
    for (index, row) in rows.iter().enumerate() {
        let len = row.text.chars().count();
        let is_last = index + 1 == rows.len();
        if cursor < row.global {
            break;
        }
        if cursor < row.global + len || (cursor == row.global + len && (is_last || len < width)) {
            return (index, cursor - row.global);
        }
    }
    match rows.last() {
        Some(row) => (rows.len().saturating_sub(1), row.text.chars().count()),
        None => (0, 0),
    }
}
