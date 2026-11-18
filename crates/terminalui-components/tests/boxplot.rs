//! Box plot behavior: rows are sanitized on construction, labels are composed
//! views with their own state in the JSON tree, and the plot state is
//! serialized verbatim.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::Value;

use terminalui_core::theme::Theme;
use terminalui_core::{node, Text, Tree};
use terminalui_components::boxplot::{BoxPlot, BoxPlotRow};
use terminalui_components::presentation::RenderStyle;

fn row(low: f64, q1: f64, median: f64, q3: f64, high: f64) -> BoxPlotRow {
    BoxPlotRow::new(low, q1, median, q3, high)
}

fn json(tree: &Tree) -> Value {
    let text = tree.to_json().expect("serialization succeeds");
    serde_json::from_str(&text).expect("output is valid JSON")
}

fn find<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if node["id"] == id {
        return Some(node);
    }
    node["view"]["children"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .find_map(|child| find(child, id))
}

#[test]
fn rows_are_ordered_and_clamped_on_construction() {
    let row = row(10.0, 8.0, 7.0, 4.0, 0.0);

    assert_eq!(row.low, 0.0);
    assert_eq!(row.high, 10.0);
    assert_eq!(row.q1, 4.0);
    assert_eq!(row.q3, 8.0);
    assert_eq!(row.median, 7.0);
}

#[test]
fn outliers_above_the_whiskers_are_kept_and_non_finite_values_dropped() {
    let row = row(0.0, 2.0, 4.0, 6.0, 8.0).with_outliers(vec![12.0, f64::NAN]);

    assert_eq!(row.outliers, vec![12.0]);
    let plot = BoxPlot::new().with_rows(vec![row]);
    let (low, high) = plot.domain();
    assert!((low + 0.48).abs() < 1e-9);
    assert!((high - 12.48).abs() < 1e-9);
}

#[test]
fn explicit_domain_wins_and_json_reports_rows() {
    let plot = BoxPlot::new()
        .with_rows(vec![row(0.0, 2.0, 4.0, 6.0, 10.0)])
        .with_domain(0.0, 20.0)
        .with_presentation(RenderStyle::Halftone);
    let tree = Tree::new(node("plot", plot));

    let value = json(&tree);
    let plot = find(&value["root"], "plot").expect("plot node");
    assert_eq!(plot["view"]["rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(plot["view"]["presentation"], "halftone");
    assert_eq!(plot["view"]["domain"], serde_json::json!([0.0, 20.0]));
}

#[test]
fn empty_plot_uses_a_unit_domain() {
    let plot = BoxPlot::new();

    assert_eq!(plot.domain(), (0.0, 1.0));
}

#[test]
fn labels_are_composed_children_with_their_own_state() {
    let plot = BoxPlot::new()
        .with_rows(vec![row(0.0, 2.0, 4.0, 6.0, 10.0)])
        .with_labels(vec![node("latency", Text::new("latency"))]);
    let tree = Tree::new(node("plot", plot));

    let value = json(&tree);
    let plot = find(&value["root"], "plot").expect("plot node");
    assert_eq!(plot["view"]["rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(plot["view"]["presentation"], "native");

    let latency = find(&value["root"], "latency").expect("label child");
    assert_eq!(latency["kind"], "text");
    assert_eq!(latency["view"]["content"], "latency");
}

#[test]
fn renders_labels_and_median_readout() {
    let plot = BoxPlot::new()
        .with_rows(vec![row(0.0, 2.0, 4.0, 6.0, 10.0)])
        .with_labels(vec![node("latency", Text::new("latency"))]);
    let mut tree = Tree::new(node("plot", plot));
    let area = Rect::new(0, 0, 60, 10);
    let mut buffer = Buffer::empty(area);

    tree.render(area, &mut buffer, &Theme::default());

    let mut painted = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            painted.push_str(buffer[(x, y)].symbol());
        }
    }
    assert!(painted.contains("latency"));
    assert!(painted.contains("p50 4"));
}

#[test]
fn tiny_areas_are_ignored_without_panicking() {
    for (width, height) in [(1, 1), (10, 2), (23, 4)] {
        let plot = BoxPlot::new().with_rows(vec![row(0.0, 2.0, 4.0, 6.0, 10.0)]);
        let mut tree = Tree::new(node("plot", plot));
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        tree.render(area, &mut buffer, &Theme::default());
    }
}
