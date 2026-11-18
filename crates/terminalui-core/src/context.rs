//! Per-call context for render/event walks of the tree. Each walker counts
//! focusables in the same (preorder DFS) order so focus assignment is stable.

/// Passed to every view during one operation. `is_focused` is cheap to call
/// and must be called exactly once by every focusable view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    /// Preorder rank of the focused leaf, if any.
    focus_index: Option<usize>,
    /// Running count of focusable views visited so far.
    counter: usize,
}

impl Context {
    pub(crate) fn new(focus_index: Option<usize>) -> Self {
        Self {
            focus_index,
            counter: 0,
        }
    }

    /// Registers this view as the next focusable leaf. Returns `true` when it
    /// holds the focus slot. Focusable views must call this exactly once per
    /// render/event/tick.
    pub fn is_focused(&mut self) -> bool {
        let focused = self.focus_index == Some(self.counter);
        self.counter += 1;
        focused
    }
}
