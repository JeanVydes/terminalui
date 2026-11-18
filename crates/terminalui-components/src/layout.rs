//! Measure-based placement of a view's children.
//!! Children that impl ement [`View::measure`] get their intrinsic width;
//! children that do not fill the remaining space.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Left,
    Center,
}

/// Render children in a single row at `area`, left aligned or centered.
pub(crate) fn row(
    children: &mut [Node],
    area: Rect,
    align: Align,
    gap: u16,
    buf: &mut Buffer,
    cx: &mut Context,
    theme: &Theme,
) {
    let right = area.x + area.width;
    let measured: u16 = children
        .iter()
        .filter_map(|child| child.view.measure().map(|(width, _)| width))
        .sum();
    let gaps = gap * (children.len().saturating_sub(1)) as u16;
    let total = measured + gaps;
    let mut x = match align {
        Align::Left => area.x,
        Align::Center => area.x + area.width.saturating_sub(total) / 2,
    };
    for child in children {
        if x >= right {
            break;
        }
        let remaining = right - x;
        let width = child
            .view
            .measure()
            .map(|(width, _)| width.min(remaining))
            .unwrap_or(remaining);
        let cell = Rect::new(x, area.y, width.max(1), 1);
        child.view.render(cell, buf, cx, theme);
        x = x.saturating_add(width + gap);
    }
}

/// Render children stacked vertically, each with its measured height.
/// Children that do not measure consume one row.
pub(crate) fn column(
    children: &mut [Node],
    area: Rect,
    buf: &mut Buffer,
    cx: &mut Context,
    theme: &Theme,
) {
    let bottom = area.y + area.height;
    let mut y = area.y;
    for child in children {
        if y >= bottom {
            break;
        }
        let remaining = bottom - y;
        let height = child
            .view
            .measure()
            .map(|(_, height)| height.min(remaining))
            .unwrap_or(1)
            .max(1);
        let cell = Rect::new(area.x, y, area.width, height);
        child.view.render(cell, buf, cx, theme);
        y = y.saturating_add(height);
    }
}
