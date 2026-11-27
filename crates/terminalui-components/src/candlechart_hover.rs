use crossterm::event::MouseEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use terminalui_core::theme::Theme;
use terminalui_core::Outcome;
use unicode_width::UnicodeWidthStr;

use super::super::axis::format_value;
use super::{CandleChart, NATIVE};

pub(super) fn handle_move(chart: &mut CandleChart, mouse: MouseEvent) -> Outcome {
    let plot = chart.hover_plot;
    let column = i32::from(mouse.column) - chart.pointer_origin.0;
    let row = i32::from(mouse.row) - chart.pointer_origin.1;
    let next = if column >= i32::from(plot.x)
        && column < i32::from(plot.x.saturating_add(plot.width))
        && row >= i32::from(plot.y)
        && row < i32::from(plot.y.saturating_add(plot.height))
    {
        let column = u16::try_from(column).unwrap_or_default();
        let visible = usize::from(column.saturating_sub(plot.x) / chart.cell_width());
        let index = chart.visible_range.0.saturating_add(visible);
        (index < chart.visible_range.1).then_some(index)
    } else {
        None
    };
    if next == chart.hovered {
        return Outcome::Ignored;
    }
    let had_hover = chart.hovered.is_some();
    chart.hovered = next;
    if next.is_some() || had_hover {
        Outcome::Consumed
    } else {
        Outcome::Ignored
    }
}

pub(super) fn paint(
    chart: &CandleChart,
    buf: &mut Buffer,
    theme: &Theme,
    to_y: &impl Fn(f64) -> u16,
    plot_top: u16,
    plot_bottom: u16,
    cell_width: u16,
) {
    let Some(index) = chart
        .hovered
        .filter(|index| *index >= chart.visible_range.0 && *index < chart.visible_range.1)
    else {
        return;
    };
    let Some(candle) = chart.series.get(index).copied() else {
        return;
    };
    let visible = index.saturating_sub(chart.visible_range.0);
    let x = chart
        .hover_plot
        .x
        .saturating_add(
            u16::try_from(visible)
                .unwrap_or(u16::MAX)
                .saturating_mul(cell_width),
        )
        .saturating_add(cell_width / 2);
    let y = to_y(candle.close).clamp(plot_top, plot_bottom);
    let glyphs = chart.presentation.resolve(NATIVE);
    let crosshair = Style::default().fg(theme.primary);
    for row in plot_top..=plot_bottom {
        buf.set_string(x, row, chart.presentation.hairline().1, crosshair);
    }
    buf.set_string(x, y, glyphs.doji, crosshair);
    let label = chart
        .hover_label
        .as_ref()
        .or(chart.x_label.as_ref())
        .map_or_else(|| format!("candle {}", index + 1), |label| label(index));
    let text = format!(
        "{label}  O {} H {} L {} C {}",
        format_value(candle.open),
        format_value(candle.high),
        format_value(candle.low),
        format_value(candle.close),
    );
    let width = u16::try_from(UnicodeWidthStr::width(text.as_str()).saturating_add(2))
        .unwrap_or(u16::MAX)
        .min(chart.hover_plot.width);
    if width < 8 {
        return;
    }
    let right = chart.hover_plot.x.saturating_add(chart.hover_plot.width);
    let tooltip_x = if x.saturating_add(width).saturating_add(1) < right {
        x.saturating_add(1)
    } else {
        x.saturating_sub(width)
    };
    let tooltip_y = if y > plot_top {
        y - 1
    } else {
        y.saturating_add(1).min(plot_bottom)
    };
    let tooltip = Rect::new(tooltip_x, tooltip_y, width, 1);
    buf.set_style(tooltip, Style::default().bg(theme.secondary));
    buf.set_stringn(
        tooltip_x.saturating_add(1),
        tooltip_y,
        &text,
        usize::from(width.saturating_sub(2)),
        Style::default().fg(theme.foreground).bg(theme.secondary),
    );
}
