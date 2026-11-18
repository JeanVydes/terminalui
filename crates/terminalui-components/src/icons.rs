//! Unicode icon glyphs.
//!
//! Plain Unicode so they render in any terminal without patched fonts.
//! Most glyphs occupy a single cell, though `Warning` and `Info` may render
//! double-width in some terminals.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Icon {
    Check,
    X,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Plus,
    Minus,
    /// Vim-style search glyph.
    Search,
    Info,
    Warning,
    Error,
    Question,
    Star,
    StarOutline,
    Circle,
    CircleFilled,
    Dot,
    Ellipsis,
    Home,
    Menu,
    ExternalLink,
}

impl Icon {
    pub fn glyph(self) -> &'static str {
        match self {
            Icon::Check => "\u{2713}",
            Icon::X => "\u{2717}",
            Icon::ChevronLeft => "\u{2039}",
            Icon::ChevronRight => "\u{203a}",
            Icon::ChevronUp => "\u{25b4}",
            Icon::ChevronDown => "\u{25be}",
            Icon::ArrowLeft => "\u{2190}",
            Icon::ArrowRight => "\u{2192}",
            Icon::ArrowUp => "\u{2191}",
            Icon::ArrowDown => "\u{2193}",
            Icon::Plus => "+",
            Icon::Minus => "\u{2212}",
            Icon::Search => "/",
            Icon::Info => "\u{2139}",
            Icon::Warning => "\u{26a0}",
            Icon::Error => "\u{2716}",
            Icon::Question => "?",
            Icon::Star => "\u{2605}",
            Icon::StarOutline => "\u{2606}",
            Icon::Circle => "\u{25cb}",
            Icon::CircleFilled => "\u{25cf}",
            Icon::Dot => "\u{00b7}",
            Icon::Ellipsis => "\u{2026}",
            Icon::Home => "\u{2302}",
            Icon::Menu => "\u{2630}",
            Icon::ExternalLink => "\u{2197}",
        }
    }
}

impl std::fmt::Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.glyph())
    }
}
