use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use terminalui_core::theme::Theme;

pub(crate) fn paint(area: Rect, label: &str, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buf.set_style(area, Style::default().bg(theme.background));
    let axis_y = area.y.saturating_add(area.height.saturating_sub(1));
    let grid = Style::default().fg(theme.border);
    for y in area.y..axis_y {
        for x in (area.x..area.x.saturating_add(area.width)).step_by(4) {
            buf.set_string(x, y, "·", grid);
        }
    }
    if area.width >= 8 {
        buf.set_string(area.x, axis_y, "└", grid);
        for x in area.x.saturating_add(1)..area.x.saturating_add(area.width) {
            buf.set_string(x, axis_y, "─", grid);
        }
    }
    let label_width = u16::try_from(label.chars().count()).unwrap_or(u16::MAX);
    if area.width < label_width.saturating_add(2) || area.height < 3 {
        return;
    }
    let x = area
        .x
        .saturating_add(area.width.saturating_sub(label_width) / 2);
    let y = area.y.saturating_add(area.height / 2);
    buf.set_stringn(
        x,
        y,
        label,
        usize::from(area.width.saturating_sub(x.saturating_sub(area.x))),
        Style::default().fg(theme.muted).bg(theme.background),
    );
}
