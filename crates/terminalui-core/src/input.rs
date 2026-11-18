use crossterm::event::{Event, KeyEvent, MouseEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize { width: u16, height: u16 },
    FocusGained,
    FocusLost,
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
