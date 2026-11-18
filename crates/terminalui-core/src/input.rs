//! Input events delivered to the tree.
use crossterm::event::{Event, KeyEvent, MouseEvent};

/// Input event delivered to the tree.
///
/// This is the crate's stable boundary: every raw crossterm event maps to
/// an `InputEvent`, views consume them, and generic code goes through
/// [`Tree::handle_event`](crate::Tree::handle_event).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputEvent {
    /// A key press.
    Key(KeyEvent),
    /// A mouse event (click, scroll, motion, ...).
    Mouse(MouseEvent),
    /// The terminal was resized.
    Resize {
        /// New width in cells.
        width: u16,
        /// New height in cells.
        height: u16,
    },
    /// The terminal regained focus.
    FocusGained,
    /// The terminal lost focus.
    FocusLost,
    /// Bracketed paste carrying the pasted text.
    Paste(String),
}

impl From<Event> for InputEvent {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key) => InputEvent::Key(key),
            Event::Mouse(mouse) => InputEvent::Mouse(mouse),
            Event::Resize(width, height) => InputEvent::Resize { width, height },
            Event::FocusGained => InputEvent::FocusGained,
            Event::FocusLost => InputEvent::FocusLost,
            Event::Paste(pasted) => InputEvent::Paste(pasted),
        }
    }
}
