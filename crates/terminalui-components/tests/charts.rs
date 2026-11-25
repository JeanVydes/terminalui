use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use terminalui_components::axis::{format_value, nice_domain, nice_max};
use terminalui_components::candlechart::{Candle, CandleChart};
use terminalui_components::linechart::{LineChart, LineChartStyle};
use terminalui_components::scroll::{ScrollbarStyle, ScrollbarVisibility};
use terminalui_core::theme::Theme;
use terminalui_core::{node, Text, Tree};

fn render(view: impl terminalui_core::View + 'static, area: Rect) -> Buffer {
    let mut tree = Tree::new(node("chart", view));
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &Theme::default());
    buffer
}

fn painted(buffer: &Buffer, area: Rect) -> String {
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn line_chart_keeps_negative_values_in_its_auto_domain() {
    let chart = LineChart::new().with_style(LineChartStyle {
        line: Some(ratatui::style::Color::Red),
        ..LineChartStyle::default()
    });
    let mut chart = chart;
    chart.set_series(vec![-12.0, 4.0]);
    let area = Rect::new(0, 0, 48, 14);
    let output = painted(&render(chart, area), area);

    assert!(output.contains("-12"));
    assert!(output.contains("●"));
}

#[test]
fn small_line_values_get_a_small_axis_range() {
    let mut chart = LineChart::new();
    chart.set_series(vec![0.01, 0.08]);
    let area = Rect::new(0, 0, 48, 14);
    let output = painted(&render(chart, area), area);

    assert!(output.contains("0.08"));
    assert!(!output.contains("10"));
}

#[test]
fn line_chart_downsampling_preserves_a_narrow_peak() {
    let mut values = vec![0.0; 100];
    values[51] = 100.0;
    let chart = LineChart::new().with_style(LineChartStyle {
        line: Some(ratatui::style::Color::Red),
        ..LineChartStyle::default()
    });
    let mut chart = chart;
    chart.set_series(values);
    let area = Rect::new(0, 0, 48, 14);
    let buffer = render(chart, area);

    assert_eq!(buffer[(20, 2)].fg, ratatui::style::Color::Red);
}

#[test]
fn line_chart_history_keeps_the_newest_samples() {
    let mut chart = LineChart::new();
    chart.set_series((0..300).map(f64::from).collect());
    let tree = Tree::new(node("chart", chart));
    let serialized: serde_json::Value =
        serde_json::from_str(&tree.to_json().expect("chart serializes"))
            .expect("serialized chart is valid JSON");
    let series = &serialized["root"]["view"]["series"];
    assert_eq!(series.as_array().map(Vec::len), Some(240));
    assert_eq!(series[0], 60.0);
    assert_eq!(series[239], 299.0);
}

#[test]
fn zero_open_candle_shows_unavailable_change_without_non_finite_text() {
    let candle = Candle {
        open: 0.0,
        high: 2.0,
        low: -1.0,
        close: 1.0,
        volume: 1.0,
    };
    let chart = CandleChart::new().with_children(vec![node("symbol", Text::new("TEST"))]);
    let mut chart = chart;
    chart.set_series(vec![candle]);
    let area = Rect::new(0, 0, 64, 14);
    let output = painted(&render(chart, area), area);

    assert!(output.contains("n/a"));
    assert!(!output.contains("NaN"));
    assert!(!output.contains("inf"));
}

#[test]
fn candle_chart_drops_non_finite_data_and_normalizes_ohlc_bounds() {
    let valid = Candle {
        open: 5.0,
        high: 4.0,
        low: 6.0,
        close: 7.0,
        volume: f64::NAN,
    };
    let invalid = Candle {
        volume: 1.0,
        ..valid
    };
    let mut chart = CandleChart::new();
    chart.set_series(vec![valid, invalid]);

    assert_eq!(chart.len(), 1);
    let serialized = Tree::new(node("chart", chart))
        .to_json()
        .expect("chart serializes");
    assert!(serialized.contains("\"high\":7.0"));
    assert!(serialized.contains("\"low\":5.0"));
}

#[test]
fn candle_chart_reserves_the_scrollbar_column() {
    let candles = (0..100)
        .map(|index| Candle {
            open: 1.0 + f64::from(index),
            high: 2.0 + f64::from(index),
            low: 0.5 + f64::from(index),
            close: 1.5 + f64::from(index),
            volume: 1.0,
        })
        .collect();
    let mut chart =
        CandleChart::new().with_style(terminalui_components::candlechart::CandleChartStyle {
            scrollbar: Some(ScrollbarStyle {
                visible: ScrollbarVisibility::Always,
                ..ScrollbarStyle::default()
            }),
            ..Default::default()
        });
    chart.set_series(candles);
    let area = Rect::new(0, 0, 60, 15);
    let buffer = render(chart, area);

    assert!((3..12).any(|y| buffer[(59, y)].fg != Theme::default().muted));
}

#[test]
fn bar_chart_keeps_excess_categories_inside_its_plot() {
    let chart = terminalui_components::barchart::BarChart::new()
        .with_values((1..=60).map(f64::from).collect());
    let area = Rect::new(0, 0, 24, 10);

    render(chart, area);
}

#[test]
fn bar_chart_scales_decimal_values_to_their_own_range() {
    let chart = terminalui_components::barchart::BarChart::new().with_values(vec![0.1]);
    let area = Rect::new(0, 0, 24, 10);
    let buffer = render(chart, area);

    assert_eq!(buffer[(0, 1)].symbol(), "█");
}

#[test]
fn axis_labels_compact_small_decimal_values() {
    assert_eq!(format_value(0.05), "0.05");
    assert_eq!(nice_max(0.1, 10), 0.1);
    assert_eq!(nice_domain(-2.4, 3.1, 5), (-4.0, 4.0));
    assert_eq!(nice_domain(0.02, 0.08, 6), (0.02, 0.08));
}
