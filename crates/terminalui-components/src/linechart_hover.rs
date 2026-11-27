use crossterm::event::MouseEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::Outcome;
use unicode_width::UnicodeWidthStr;

use super::super::axis::format_value;
use super::super::presentation::Presentation;
use super::LineChart;

pub(super) fn handle_event(chart: &mut LineChart, event: &InputEvent) -> Outcome {
    let InputEvent::Mouse(mouse) = event else {
        return Outcome::Ignored;
    };
    if mouse.kind != MouseEventKind::Moved {
        return Outcome::Ignored;
    }
    let plot = chart.plot_area;
    let next = if mouse.column >= plot.x
        && mouse.column < plot.x.saturating_add(plot.width)
        && mouse.row >= plot.y
        && mouse.row < plot.y.saturating_add(plot.height)
    {
        let column = mouse.column - plot.x;
        chart
            .sample_columns
            .get(usize::from(column))
            .copied()
            .flatten()
            .map(|sample| (sample, column))
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
    chart: &LineChart,
    buf: &mut Buffer,
    theme: &Theme,
    to_y: &impl Fn(f64) -> u16,
    plot_top: u16,
    plot_bottom: u16,
) {
    let Some((sample, column)) = chart.hovered else {
        return;
    };
    let Some(value) = chart.series.get(sample).copied().flatten() else {
        return;
    };
    let x = chart.plot_area.x.saturating_add(column);
    let y = to_y(value).clamp(plot_top, plot_bottom);
    let glyphs = chart.presentation.resolve(Presentation {
        grid: "·",
        wick: "│",
        doji: "●",
        body: "●",
        live: "╌",
    });
    let crosshair = Style::default().fg(theme.primary);
    for row in plot_top..=plot_bottom {
        buf.set_string(x, row, chart.presentation.hairline().1, crosshair);
    }
    buf.set_string(x, y, glyphs.doji, crosshair);

    let label = chart
        .x_labels
        .get(sample)
        .cloned()
        .unwrap_or_else(|| format!("sample {}", sample + 1));
    let text = format!("{label}  {}", format_value(value));
    let text_width = UnicodeWidthStr::width(text.as_str());
    let width = u16::try_from(text_width.saturating_add(2))
        .unwrap_or(u16::MAX)
        .min(chart.plot_area.width);
    if width < 4 {
        return;
    }
    let right = chart.plot_area.x.saturating_add(chart.plot_area.width);
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
    let tooltip_style = Style::default().fg(theme.foreground).bg(theme.secondary);
    buf.set_style(tooltip, Style::default().bg(theme.secondary));
    buf.set_stringn(
        tooltip_x.saturating_add(1),
        tooltip_y,
        &text,
        usize::from(width.saturating_sub(2)),
        tooltip_style,
    );
}
