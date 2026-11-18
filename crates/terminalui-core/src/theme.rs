//! Global palette used by every view in the tree.
use ratatui::style::Color;

/// Global palette. Every color a view paints with comes from here;
/// override per-instance via `Style` fields or pass your own instance
/// to [`Context`](crate::Context) consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Window background color (default `#18181E`).
    pub background: Color,
    /// Default text color (default `#EEEEF4`).
    pub foreground: Color,
    /// Brand primary (default `#5865F2`).
    pub primary: Color,
    /// Secondary surfaces (default `#3F3F4F`).
    pub secondary: Color,
    /// Subdued surfaces/text (default `#303038`).
    pub muted: Color,
    /// Accent for highlights and focus rings (default `#38BDF8`).
    pub accent: Color,
    /// Destructive actions (default `#EF4444`).
    pub destructive: Color,
    /// Success/green (default `#22C55E`).
    pub success: Color,
    /// Warnings/amber (default `#F59E0B`).
    pub warning: Color,
    /// Border/hairline color (default `#3E3E48`).
    pub border: Color,
    /// Focus ring color; matches `primary` by default.
    pub ring: Color,
    /// Border glyph-set color override; matches `border` by default.
    pub border_set: Color,
}

fn default_theme() -> Theme {
    Theme {
        background: Color::Rgb(24, 24, 30),
        foreground: Color::Rgb(238, 238, 244),
        primary: Color::Rgb(88, 101, 242),
        secondary: Color::Rgb(63, 63, 79),
        muted: Color::Rgb(48, 48, 56),
        accent: Color::Rgb(56, 189, 248),
        destructive: Color::Rgb(239, 68, 68),
        success: Color::Rgb(34, 197, 94),
        warning: Color::Rgb(245, 158, 11),
        border: Color::Rgb(62, 62, 72),
        ring: Color::Rgb(88, 101, 242),
        border_set: Color::Rgb(62, 62, 72),
    }
}

impl Default for Theme {
    fn default() -> Self {
        default_theme()
    }
}

impl Theme {
    /// 16-color ANSI palette.
    pub fn for_ansi16() -> Self {
        Self {
            background: Color::Black,
            foreground: Color::White,
            primary: Color::Blue,
            secondary: Color::DarkGray,
            muted: Color::DarkGray,
            accent: Color::Cyan,
            destructive: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            border: Color::DarkGray,
            ring: Color::Blue,
            border_set: Color::DarkGray,
        }
    }

    /// 256-color palette.
    pub fn for_ansi256() -> Self {
        Self {
            background: Color::Indexed(234),
            foreground: Color::Indexed(231),
            primary: Color::Indexed(12),
            secondary: Color::Indexed(239),
            muted: Color::Indexed(239),
            accent: Color::Indexed(51),
            destructive: Color::Indexed(196),
            success: Color::Indexed(34),
            warning: Color::Indexed(178),
            border: Color::Indexed(239),
            ring: Color::Indexed(12),
            border_set: Color::Indexed(239),
        }
    }
}
