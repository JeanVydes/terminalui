//! Badge: a small status pill for categories, counts and states.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, TextStyle, View};

use super::layout::{row, Align};

/// Semantic significance of the badge. Maps to theme colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BadgeVariant {
    /// No semantic significance; uses the theme secondary color.
    #[default]
    Neutral,
    /// Positive/success state; uses the theme success color.
    Success,
    /// Warning state; uses the theme warning color.
    Warning,
    /// Error/destructive state; uses the theme destructive color.
    Destructive,
}

/// Per-instance overrides of colors and geometry for a badge. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BadgeStyle {
    /// Badge background. Defaults to the variant's color.
    pub background: Option<Color>,
    /// Render the contents in bold. Default true.
    pub bold: Option<bool>,
    /// Horizontal padding in cells. Default 1.
    pub padding_x: Option<u16>,
}

/// Public state of the badge: the last rendered area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BadgeState {
    /// Last rendered area, set during render.
    pub area: Rect,
}

/// A small status pill for categories, counts and states.
#[derive(Serialize)]
pub struct Badge {
    /// Semantic significance of the pill; colors a text with the theme.
    pub variant: BadgeVariant,
    /// Per-instance overrides of colors and geometry.
    pub style: BadgeStyle,
    /// Where the badge rendered in the last pass; the host may read it.
    pub state: BadgeState,
    children: Vec<Node>,
    themed_content: bool,
}

impl Badge {
    /// Convenience constructor: the content is a single [`Text`] view.
    pub fn new(content: impl Into<String>) -> Self {
        let text = Text::new(content);
        let mut badge = Self::with_children(vec![node("content", text)]);
        badge.themed_content = true;
        badge
    }
}

impl Badge {
    /// Compose a badge from arbitrary views.
    pub fn with_children(children: Vec<Node>) -> Self {
        Self {
            variant: BadgeVariant::default(),
            style: BadgeStyle::default(),
            state: BadgeState {
                area: Rect::new(0, 0, 0, 0),
            },
            children,
            themed_content: false,
        }
    }

    /// Builder shorthand: set the variant.
    pub fn with_variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: BadgeStyle) -> Self {
        self.style = style;
        self
    }

    fn background(&self, theme: &Theme) -> Color {
        match self.variant {
            BadgeVariant::Neutral => theme.secondary,
            BadgeVariant::Success => theme.success,
            BadgeVariant::Warning => theme.warning,
            BadgeVariant::Destructive => theme.destructive,
        }
    }

    fn content_width(&self) -> u16 {
        let padding = self.style.padding_x.unwrap_or(1);
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum();
        measured + padding * 2
    }
}

impl Default for Badge {
    fn default() -> Self {
        Self::with_children(Vec::new())
    }
}

impl View for Badge {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "badge"
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((self.content_width(), 1))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        let width = self.content_width().min(area.width);
        let padding = self.style.padding_x.unwrap_or(1);
        let background = self.style.background.unwrap_or(self.background(theme));
        if self.themed_content {
            if let Some(text) = self
                .children
                .first_mut()
                .and_then(|node| node.view.as_any().downcast_mut::<Text>())
            {
                let mut style = TextStyle::default().fg(contrasting_color(background, theme));
                if self.style.bold.unwrap_or(true) {
                    style = style.bold();
                }
                text.set_style(style);
            }
        }
        let rect = Rect::new(area.x, area.y, width.max(1), 1);
        buf.set_style(rect, Style::default().bg(background));
        let inner_x = rect.x + padding.min(width);
        let inner_width = rect.width.saturating_sub(padding * 2).max(1);
        let inner = Rect::new(inner_x, rect.y, inner_width, 1);
        row(&mut self.children, inner, Align::Center, 0, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

fn contrasting_color(background: Color, theme: &Theme) -> Color {
    let background_luminance = rgb(background).map(luminance);
    [theme.background, theme.foreground]
        .into_iter()
        .max_by(|left, right| {
            let left_contrast = background_luminance
                .zip(rgb(*left).map(luminance))
                .map_or(0.0, |(background, foreground)| {
                    contrast(background, foreground)
                });
            let right_contrast = background_luminance
                .zip(rgb(*right).map(luminance))
                .map_or(0.0, |(background, foreground)| {
                    contrast(background, foreground)
                });
            left_contrast.total_cmp(&right_contrast)
        })
        .unwrap_or(theme.foreground)
}

fn contrast(left: f64, right: f64) -> f64 {
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn luminance((red, green, blue): (u8, u8, u8)) -> f64 {
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
}

fn rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(red, green, blue) => Some((red, green, blue)),
        Color::Indexed(index) => indexed_rgb(index),
        Color::Reset => None,
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((205, 49, 49)),
        Color::Green => Some((13, 188, 121)),
        Color::Yellow => Some((229, 229, 16)),
        Color::Blue => Some((36, 114, 200)),
        Color::Magenta => Some((188, 63, 188)),
        Color::Cyan => Some((17, 168, 205)),
        Color::Gray => Some((229, 229, 229)),
        Color::DarkGray => Some((102, 102, 102)),
        Color::LightRed => Some((241, 76, 76)),
        Color::LightGreen => Some((35, 209, 139)),
        Color::LightYellow => Some((245, 245, 67)),
        Color::LightBlue => Some((59, 142, 234)),
        Color::LightMagenta => Some((214, 112, 214)),
        Color::LightCyan => Some((41, 184, 219)),
        Color::White => Some((255, 255, 255)),
    }
}

fn indexed_rgb(index: u8) -> Option<(u8, u8, u8)> {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];
    match index {
        0..=15 => ANSI.get(usize::from(index)).copied(),
        16..=231 => {
            let cube = usize::from(index - 16);
            let component = |value: usize| match value {
                0 => 0,
                value => 55 + 40 * value,
            } as u8;
            Some((
                component(cube / 36),
                component((cube / 6) % 6),
                component(cube % 6),
            ))
        }
        232..=255 => {
            let gray = 8 + 10 * (index - 232);
            Some((gray, gray, gray))
        }
    }
}
