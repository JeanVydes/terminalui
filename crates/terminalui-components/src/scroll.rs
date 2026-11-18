//! Generic scrollable region support: math, scrollbar state and rendering.
//!
//! Used by row lists, tables, multi-line inputs, the console-style chart and
//! anything else whose content does not fit its viewport. The style of the
//! scrollbar (visible/hidden, colors, glyphs, radius) is shared too.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;

use super::chrome::Radius;
use super::presentation::RenderStyle;

/// Clamp `selected` to `total` and adjust `offset` so the selected row
/// stays visible in a viewport of `body` rows. Returns `(selected, offset)`.
pub fn clamp_view(selected: usize, offset: usize, body: usize, total: usize) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }
    let selected = selected.min(total - 1);
    if body == 0 {
        return (selected, offset);
    }
    let offset = if selected < offset {
        selected
    } else if selected >= offset + body {
        selected + 1 - body
    } else {
        offset
    }
    .min(total - 1);
    (selected, offset)
}

/// When to draw the scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ScrollbarVisibility {
    /// Draw only when the content overflows the viewport.
    #[default]
    Auto,
    /// Always draw the scrollbar, even without overflow.
    Always,
    /// Never draw the scrollbar.
    Never,
}

/// Style of a vertical scrollbar drawn at the right edge of its area.
/// The glyph set comes from [`RenderStyle`]; nothing is drawn with a
/// hardcoded glyph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScrollbarStyle {
    /// When the scrollbar is drawn.
    pub visible: ScrollbarVisibility,
    /// Presentation for the glyphs; defaults to [`RenderStyle::Native`].
    pub presentation: RenderStyle,
    /// Track (background) color. Defaults to `theme.muted`.
    pub track_color: Option<Color>,
    /// Thumb color. Defaults to `theme.foreground`.
    pub thumb_color: Option<Color>,
    /// `Small` draws rounded caps on a tall-enough thumb.
    pub radius: Radius,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            visible: ScrollbarVisibility::Auto,
            presentation: RenderStyle::Native,
            track_color: None,
            thumb_color: None,
            radius: Radius::None,
        }
    }
}

fn should_draw(style: &ScrollbarStyle, total: usize, viewport: usize) -> bool {
    match style.visible {
        ScrollbarVisibility::Always => true,
        ScrollbarVisibility::Never => false,
        ScrollbarVisibility::Auto => total > viewport,
    }
}

/// Page-offset scroll model shared by lists, tables and the console input.
///
/// `offset` is the index of the first visible row, `viewport` the visible
/// row count, `total` the total rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Scroll {
    /// Index of the first visible row.
    pub offset: usize,
    /// Number of visible rows.
    pub viewport: usize,
    /// Total number of rows.
    pub total: usize,
}

impl Scroll {
    /// Creates a scroll model at offset 0 with the given viewport and total.
    pub fn new(viewport: usize, total: usize) -> Self {
        Self {
            offset: 0,
            viewport,
            total,
        }
    }

    /// A view whose `viewport`/`total`/`offset` are in content units (e.g.
    /// candles), rendered onto a `track`-cell rail. The thumb size and
    /// position stay proportional to `window / total`.
    pub fn scaled(track: usize, offset: usize, window: usize, total: usize) -> Self {
        let track = track.max(1);
        let window = window.max(1);
        let scaled_total = total.max(window) * track / window;
        let mut scroll = Self::new(track, scaled_total.max(track));
        scroll.offset = offset.min(total.saturating_sub(window)) * track / window;
        scroll
    }

    /// Builder shorthand: set the offset; clamps to the max offset afterwards.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.clamp();
        self.offset = offset.min(self.max_offset());
        self
    }

    /// The largest legal offset: total minus viewport.
    pub fn max_offset(&self) -> usize {
        self.total.saturating_sub(self.viewport)
    }

    /// Clamp the offset to the valid range.
    pub fn clamp(&mut self) {
        if self.total == 0 {
            self.offset = 0;
            return;
        }
        self.offset = self.offset.min(self.max_offset());
    }

    /// Thumb height and offset in cells, for drawing the scrollbar.
    pub fn thumb(&self) -> (u16, u16) {
        if self.viewport == 0 {
            return (0, 0);
        }
        let height = self.viewport as u16;
        if self.total <= self.viewport {
            return (0, height);
        }
        let size = ((self.viewport * self.viewport) / self.total).max(1) as u16;
        let track = height.saturating_sub(size);
        let top = if self.max_offset() == 0 {
            0
        } else {
            ((self.offset as u64 * track as u64) / self.max_offset() as u64) as u16
        };
        (top, size)
    }
}

/// Draw the scrollbar for `scroll` at the right edge of `area` (one cell).
/// Does nothing when the style says to hide it.
pub fn render_scrollbar(
    buf: &mut Buffer,
    area: Rect,
    style: &ScrollbarStyle,
    theme: &terminalui_core::theme::Theme,
    scroll: &Scroll,
) {
    let track = style.track_color.unwrap_or(theme.muted);
    let thumb = style.thumb_color.unwrap_or(theme.accent);
    if !should_draw(style, scroll.total, scroll.viewport) || area.height == 0 {
        return;
    }
    let x = area.x + area.width.saturating_sub(1);
    let glyphs = style.presentation.scrollbar();
    for y in area.y..area.y + area.height {
        buf.set_string(x, y, glyphs.track, Style::default().fg(track));
    }
    let (top, size) = scroll.thumb();
    for offset in 0..size {
        let glyph = match style.radius {
            Radius::Small if size >= 3 && offset == 0 => glyphs.cap_top,
            Radius::Small if size >= 3 && offset == size - 1 => glyphs.cap_bottom,
            _ => glyphs.thumb,
        };
        buf.set_string(x, area.y + top + offset, glyph, Style::default().fg(thumb));
    }
}
