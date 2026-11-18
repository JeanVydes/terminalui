//! Event outcome shared by every view.

/// Whether an event was claimed by the view that handled it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Outcome {
    /// The view ignored the event; it bubbles on.
    #[default]
    Ignored,
    /// The view consumed the event; nothing else sees it.
    Consumed,
}

impl Outcome {
    /// `true` if the view claimed the event (`Consumed`).
    pub fn is_consumed(self) -> bool {
        matches!(self, Outcome::Consumed)
    }

    /// `true` if the view let the event continue (`Ignored`).
    pub fn is_ignored(self) -> bool {
        matches!(self, Outcome::Ignored)
    }

    /// `Outcome::Consumed`; lets a view claim an event in one move.
    pub fn consumed(self) -> Self {
        Outcome::Consumed
    }
}
