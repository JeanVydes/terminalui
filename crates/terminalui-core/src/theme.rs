use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub muted: Color,
    pub accent: Color,
    pub destructive: Color,
    pub success: Color,
    pub warning: Color,
    pub border: Color,
    pub ring: Color,
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
