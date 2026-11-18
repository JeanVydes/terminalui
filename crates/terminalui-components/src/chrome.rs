//! Shared border/frame abstraction: the box most surfaces draw.
//!
//! Everything that is a "box" composes this instead of hand-drawing glyphs:
//! buttons, inputs, floating boxes, dialogs, toasts, tooltips, the table's
//! frame, checkbox and select.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;

use terminalui_animations::lerp_color;
use terminalui_core::theme::Theme;

/// Border glyph set. Terminals cannot express arbitrary border shapes, so
/// the radius is expressed through the rounded set instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BorderSet {
    /// Plain glyph set: `─ │ ┌ ┐ └ ┘`.
    #[default]
    Plain,
    /// Thick strokes: `━ ┃ ┏ ┓ ┗ ┛`.
    Thick,
    /// Double strokes: `═ ║ ╔ ╗ ╚ ╝`.
    Double,
    /// Rounded corners with plain single-line strokes.
    Rounded,
    /// Dashed strokes: `╌ ╎` with plain corners.
    Dashed,
    /// Blank cells: the border reserves its rectangle but paints nothing.
    Blank,
}

impl BorderSet {
    pub(crate) fn horizontal(self) -> &'static str {
        match self {
            Self::Plain | Self::Rounded => "─",
            Self::Thick => "━",
            Self::Double => "═",
            Self::Dashed => "╌",
            Self::Blank => " ",
        }
    }

    pub(crate) fn vertical(self) -> &'static str {
        match self {
            Self::Plain | Self::Rounded => "│",
            Self::Thick => "┃",
            Self::Double => "║",
            Self::Dashed => "╎",
            Self::Blank => " ",
        }
    }

    pub(crate) fn corners(self) -> (char, char, char, char) {
        match self {
            Self::Plain | Self::Dashed => ('┌', '┐', '└', '┘'),
            Self::Thick => ('┏', '┓', '┗', '┛'),
            Self::Double => ('╔', '╗', '╚', '╝'),
            Self::Rounded => ('╭', '╮', '╰', '╯'),
            Self::Blank => (' ', ' ', ' ', ' '),
        }
    }
}

/// Corner radius; in a terminal, rounded corners are the only radius that
/// exists. With `Radius::Small` the corners render rounded even when the
/// set is `Plain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Radius {
    /// No radius.
    #[default]
    None,
    /// Rounded corners.
    Small,
}

/// A two-color gradient. `angle` is in degrees, clockwise: `0°` left to
/// right, `90°` top to bottom, `180°` right to left, `270°` bottom to top;
/// any diagonal in between works the same way.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Gradient {
    /// Color at one end of the gradient axis.
    pub start: Color,
    /// Color at the other end of the gradient axis.
    pub end: Color,
    /// Travel direction in degrees. Clockwise: `0°` left to right,
    /// `90°` top to bottom, `180°` right to left, `270°` bottom to top.
    /// Any diagonal in between works the same way.
    pub angle: f64,
}

impl Gradient {
    /// Color at a cell, projected from center to center along the axis.
    fn color_at(&self, rect: Rect, x: u16, y: u16) -> Color {
        let angle = self.angle.to_radians();
        let axis_x = angle.cos();
        let axis_y = angle.sin();
        let project = |px: f64, py: f64| px * axis_x + py * axis_y;
        let corners = [
            (rect.x as f64 + 0.5, rect.y as f64 + 0.5),
            ((rect.x + rect.width) as f64 - 0.5, rect.y as f64 + 0.5),
            (rect.x as f64 + 0.5, (rect.y + rect.height) as f64 - 0.5),
            (
                (rect.x + rect.width) as f64 - 0.5,
                (rect.y + rect.height) as f64 - 0.5,
            ),
        ];
        let min_projection = corners
            .iter()
            .map(|&(cx, cy)| project(cx, cy))
            .fold(f64::INFINITY, f64::min);
        let max_projection = corners
            .iter()
            .map(|&(cx, cy)| project(cx, cy))
            .fold(f64::NEG_INFINITY, f64::max);
        let span = (max_projection - min_projection).max(f64::EPSILON);
        let cell_projection = project(x as f64 + 0.5, y as f64 + 0.5);
        let t = ((cell_projection - min_projection) / span).clamp(0.0, 1.0);
        lerp_color(self.start, self.end, t)
    }
}

/// Border of a box: glyph set, color, radius. `color: None` falls back to
/// `theme.border`. A gradient on the border (if any) overrides `color` per
/// cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Border {
    /// Glyph set to paint the border with.
    pub set: BorderSet,
    /// Foreground color of the border glyphs; defaults to `theme.border`.
    pub color: Option<Color>,
    /// Corner radius for this border.
    pub radius: Radius,
}

/// Interior spacing in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Padding {
    /// Cells of padding on the left and right.
    pub horizontal: u16,
    /// Cells of padding on the top and bottom.
    pub vertical: u16,
}

/// Fill, border and padding of a box. Default is "no chrome": transparent,
/// nothing drawn.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Chrome {
    /// Border of the box. `None` = no border, only fill.
    pub border: Option<Border>,
    /// Background fill behind the box.
    pub background: Option<Color>,
    /// Corner radius: rounds corners when set.
    pub radius: Radius,
    /// Inner padding in cells.
    pub padding: Padding,
    /// Gradient applied to the border glyphs, per cell; overrides the
    /// border's `color`. The glyphs keep the border's glyph set.
    pub border_gradient: Option<Gradient>,
    /// Gradient applied to the background fill, per cell; overrides
    /// `background`.
    pub fill_gradient: Option<Gradient>,
}

impl Chrome {
    /// `Chrome` with nothing set: no border, no background, no padding.
    pub fn transparent() -> Self {
        Self::default()
    }

    /// `Chrome` with only a background fill and no border or padding.
    pub fn filled(background: Color) -> Self {
        Self {
            background: Some(background),
            ..Self::default()
        }
    }

    /// `Chrome` with a border (`set` glyph set, `color`, no radius or padding).
    pub fn bordered(set: BorderSet, color: Option<Color>) -> Self {
        Self {
            border: Some(Border {
                set,
                color,
                radius: Radius::None,
            }),
            ..Self::default()
        }
    }

    /// The (glyph set, opaque) border, honoring radius rounding. Returns
    /// the set only; each cell's color comes from the gradient at render
    /// time or from the border's own `color`.
    fn border_set(&self) -> Option<BorderSet> {
        self.border.map(|border| {
            if self.radius == Radius::Small || border.radius == Radius::Small {
                BorderSet::Rounded
            } else {
                border.set
            }
        })
    }

    /// Inner rect content should render into, after border and padding.
    /// A one-cell-high box can only hold its side strokes, so the border
    /// contributes x offsets but no y offset there.
    pub fn inner(&self, rect: Rect) -> Rect {
        let border = u16::from(self.border.is_some());
        let vertical_offset = u16::from(self.border.is_some() && rect.height >= 2);
        let x = rect.x + border + self.padding.horizontal;
        let y = rect.y + vertical_offset + self.padding.vertical.min(rect.height.saturating_sub(1));
        let width = rect
            .width
            .saturating_sub(2 * (border + self.padding.horizontal));
        let height = rect
            .height
            .saturating_sub(2 * (vertical_offset + self.padding.vertical));
        Rect::new(x, y, width.max(1), height.max(1))
    }
}

/// Draw a box for `chrome` inside `rect`. Returns the inner content rect.
pub fn draw_box(buf: &mut Buffer, rect: Rect, chrome: &Chrome, theme: &Theme) -> Rect {
    if let Some(gradient) = &chrome.fill_gradient {
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(gradient.color_at(rect, x, y));
                }
            }
        }
    } else if let Some(background) = chrome.background {
        buf.set_style(rect, Style::default().bg(background));
    }
    if let Some(set) = chrome.border_set() {
        let Some(border) = chrome.border else {
            return chrome.inner(rect);
        };
        let fill_at = |x: u16, y: u16| match &chrome.fill_gradient {
            Some(gradient) => gradient.color_at(rect, x, y),
            None => chrome.background.unwrap_or(theme.background),
        };
        let fg_at = |x: u16, y: u16| match &chrome.border_gradient {
            Some(gradient) => gradient.color_at(rect, x, y),
            None => border.color.unwrap_or(theme.border),
        };
        let paint_cell = |buf: &mut Buffer, x: u16, y: u16, glyph: &str| {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(glyph);
                cell.set_fg(fg_at(x, y));
                cell.set_bg(fill_at(x, y));
            }
        };
        let (tl, tr, bl, br) = set.corners();
        if rect.height >= 2 {
            let bottom = rect.y + rect.height - 1;
            for x in rect.x..rect.x + rect.width {
                paint_cell(buf, x, rect.y, set.horizontal());
                paint_cell(buf, x, bottom, set.horizontal());
            }
        }
        if rect.width >= 2 {
            for y in rect.y..rect.y + rect.height {
                paint_cell(buf, rect.x, y, set.vertical());
                paint_cell(buf, rect.x + rect.width - 1, y, set.vertical());
            }
        }
        if rect.width >= 2 && rect.height >= 2 {
            paint_cell(buf, rect.x, rect.y, &tl.to_string());
            paint_cell(buf, rect.x + rect.width - 1, rect.y, &tr.to_string());
            paint_cell(buf, rect.x, rect.y + rect.height - 1, &bl.to_string());
            paint_cell(
                buf,
                rect.x + rect.width - 1,
                rect.y + rect.height - 1,
                &br.to_string(),
            );
        }
    }
    chrome.inner(rect)
}
