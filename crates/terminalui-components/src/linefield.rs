//! Shared single-line editing state: value, cursor and the operations both
//! the command palette and the autocomplete need. Char-indexed cursor,
//! mirroring [`super::input::Input`].

use serde::Serialize;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LineField {
    pub value: String,
    pub cursor: usize,
}

impl LineField {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.chars().count();
    }

    pub(crate) fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    fn byte_at(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(byte, _)| byte)
            .unwrap_or(self.value.len())
    }

    pub(crate) fn insert(&mut self, character: char) {
        if character == '\n' || character == '\r' {
            return;
        }
        let byte = self.byte_at(self.cursor);
        self.value.insert(byte, character);
        self.cursor += 1;
    }

    pub(crate) fn paste(&mut self, text: &str) {
        for character in text.chars() {
            self.insert(character);
        }
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.byte_at(self.cursor - 1);
        let end = self.byte_at(self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
    }

    pub(crate) fn delete(&mut self) {
        let start = self.byte_at(self.cursor);
        if start == self.value.len() {
            return;
        }
        let end = self.byte_at(self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub(crate) fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.value.chars().count());
    }

    pub(crate) fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.cursor = self.value.chars().count();
    }
}
