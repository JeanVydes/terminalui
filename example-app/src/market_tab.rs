//! The market tab: four candle charts with composed captions.

use terminalui::theme::Theme;
use terminalui::{node, Text, TextStyle, Tree};
use terminalui::components::badge::{Badge, BadgeVariant};
use terminalui::components::candlechart::CandleChart;
use terminalui::components::grid::ResizableGrid;

use crate::common::{breadcrumb, stack_root, strip_node, tabs_node, TAB_MARKET};
use crate::market::{InventedMarket, Timeframe};

fn chart_caption(theme: &Theme, timeframe: Timeframe) -> Vec<terminalui::Node> {
    vec![
        node(
            format!("symbol-{}", timeframe.label()),
            Text::new("KVX/USD").with_style(TextStyle::default().bold().fg(theme.foreground)),
        ),
        node(
            format!("interval-{}", timeframe.label()),
            Badge::new(timeframe.label()).with_variant(BadgeVariant::Neutral),
        ),
    ]
}

/// X-axis labels injected by the host: HH:MM (or HH:MM:SS for sub-minute
/// timeframes) derived from the candle index relative to a 09:30 open.
fn x_label_for(timeframe: Timeframe) -> impl Fn(usize) -> String {
    let step = timeframe.duration().as_secs();
    move |index| {
        let total = 9 * 60 * 60 + 30 * 60 + step * (index as u64 + 1);
        let hours = (total / 3600) % 24;
        let minutes = (total / 60) % 60;
        let seconds = total % 60;
        if step >= 60 {
            format!("{hours:02}:{minutes:02}")
        } else {
            format!("{minutes:02}:{seconds:02}")
        }
    }
}

fn chart_node(
    theme: &Theme,
    markets: &[InventedMarket],
    index: usize,
    timeframe: Timeframe,
) -> terminalui::Node {
    let mut chart = CandleChart::new()
        .with_children(chart_caption(theme, timeframe))
        .with_x_label(x_label_for(timeframe));
    chart.set_series(markets[index].series.clone());
    node(format!("chart-{}", timeframe.label()), chart)
}

pub(crate) fn build_market_tree(theme: &Theme, markets: &[InventedMarket]) -> Tree {
    let chart = |index, timeframe| chart_node(theme, markets, index, timeframe);
    let root = stack_root(vec![
        node("breadcrumb", breadcrumb(theme, TAB_MARKET)),
        tabs_node(TAB_MARKET),
        strip_node(
            "KVX/USD · 1s · 5s · 30s · 1m — drag the grid dividers to resize",
            theme,
        ),
        node(
            "market-grid",
            ResizableGrid::row(vec![1, 1])
                .with_min_sizes(vec![20, 20])
                .panes(vec![
                    node(
                        "market-left",
                        ResizableGrid::column(vec![1, 1])
                            .with_min_sizes(vec![5, 5])
                            .panes(vec![chart(0, Timeframe::Secs1), chart(1, Timeframe::Secs5)]),
                    ),
                    node(
                        "market-right",
                        ResizableGrid::column(vec![1, 1])
                            .with_min_sizes(vec![5, 5])
                            .panes(vec![chart(2, Timeframe::Secs30), chart(3, Timeframe::Min1)]),
                    ),
                ]),
        ),
    ]);
    Tree::new(root)
}
