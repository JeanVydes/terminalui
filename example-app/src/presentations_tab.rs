//! The styles tab: a live latency box plot, every `RenderStyle` preset and
//! the tuning knobs that reshape the kit's ink ramps, all independent.

use ratatui::layout::Constraint;
use terminalui::theme::Theme;
use terminalui::{node, Stack, Text, TextStyle, Tree};
use terminalui::components::boxplot::{BoxPlot, BoxPlotStyle};
use terminalui::components::chrome::{BorderSet, Chrome, Padding};
use terminalui::components::panel::Panel;
use terminalui::components::presentation::{RenderStyle, StyleTuning};
use terminalui::components::progress::Progress;
use terminalui::components::select::Select;
use terminalui::components::slider::Slider;
use terminalui::components::spinner::Spinner;

use crate::common::{breadcrumb, muted, strip_node, TAB_STYLES};
use crate::latency::LatencyMonitor;

/// Every presentation variant shipped by the kit, in display order.
pub(crate) const PRESENTATIONS: &[(RenderStyle, &str)] = &[
    (RenderStyle::Native, "native"),
    (RenderStyle::Halftone, "halftone"),
    (RenderStyle::Ascii, "ascii"),
    (RenderStyle::DenseAscii, "dense-ascii"),
    (RenderStyle::Numbers, "numbers"),
    (RenderStyle::Binary, "binary"),
    (RenderStyle::Letters, "letters"),
    (RenderStyle::Blocks, "blocks"),
    (RenderStyle::Squares, "squares"),
    (RenderStyle::Lines, "lines"),
    (RenderStyle::Dither, "dither"),
];

pub(crate) fn ramp_gradient(presentation: RenderStyle, tuning: &StyleTuning, width: u16) -> String {
    let width = width.max(2) as usize;
    (0..width)
        .map(|index| {
            let intensity = index as f32 / (width - 1) as f32;
            presentation.shade(intensity, tuning)
        })
        .collect()
}

fn preset_select(id: &str, index: usize) -> terminalui::Node {
    let options: Vec<terminalui::Node> = PRESENTATIONS
        .iter()
        .enumerate()
        .map(|(index, (_style, name))| node(format!("{id}-opt-{index}"), Text::new(*name)))
        .collect();
    node(id.to_string(), Select::new(options).with_selected(index))
}

/// One labelled row: a fixed-width label, free content and a preset select.
fn preset_row(
    theme: &Theme,
    row_id: &str,
    label: &str,
    select_id: &str,
    preset_index: usize,
    content: terminalui::Node,
) -> terminalui::Node {
    node(
        row_id.to_string(),
        Stack::horizontal()
            .gap(2)
            .constraints(vec![
                Constraint::Length(14),
                Constraint::Fill(1),
                Constraint::Length(18),
            ])
            .children(vec![
                node(
                    format!("label-{row_id}"),
                    Text::new(label).with_style(muted(theme)),
                ),
                content,
                preset_select(select_id, preset_index),
            ]),
    )
}

/// The slider rows that reshape the hero plot's ink. Ids are scoped by
/// `prefix`; the host reads the values and writes the readout labels.
struct TunerRow {
    prefix: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
    value_text: String,
}

fn tuner_row(theme: &Theme, config: TunerRow) -> terminalui::Node {
    let TunerRow {
        prefix,
        label,
        min,
        max,
        step,
        value,
        value_text,
    } = config;
    let slider_id = format!("{prefix}-{}", label.replace(' ', "-"));
    let value_id = format!("label-{slider_id}");
    node(
        format!("row-{slider_id}"),
        Stack::horizontal()
            .gap(2)
            .constraints(vec![
                Constraint::Length(14),
                Constraint::Fill(1),
                Constraint::Length(7),
            ])
            .children(vec![
                node(
                    format!("caption-{slider_id}"),
                    Text::new(label).with_style(muted(theme)),
                ),
                node(
                    slider_id.clone(),
                    Slider::new(min, max, step).with_value(value).with_style(
                        terminalui::components::slider::SliderStyle {
                            filled: Some(theme.primary),
                            track: None,
                            thumb: Some(theme.accent),
                            disabled_fill: None,
                        },
                    ),
                ),
                node(
                    value_id,
                    Text::new(value_text).with_style(TextStyle::default().fg(theme.accent)),
                ),
            ]),
    )
}

fn tuner_rows(theme: &Theme, tuning: StyleTuning) -> Vec<terminalui::Node> {
    vec![
        tuner_row(
            theme,
            TunerRow {
                prefix: "box",
                label: "density",
                min: 2.0,
                max: 6.0,
                step: 1.0,
                value: tuning.density as f64,
                value_text: format!("{:.0}", tuning.density),
            },
        ),
        tuner_row(
            theme,
            TunerRow {
                prefix: "box",
                label: "contrast",
                min: 0.5,
                max: 3.0,
                step: 0.25,
                value: tuning.contrast.into(),
                value_text: format!("{:.2}", tuning.contrast),
            },
        ),
    ]
}

pub(crate) fn build_presentations_tree(theme: &Theme, latency: &LatencyMonitor) -> Tree {
    let tuning = StyleTuning::default();
    let gradient = ramp_gradient(RenderStyle::Halftone, &StyleTuning::default(), 52);
    let spinner_nodes: Vec<terminalui::Node> = PRESENTATIONS
        .iter()
        .enumerate()
        .map(|(index, (style, _name))| {
            node(
                format!("style-spinner-{index}"),
                Spinner::new().with_presentation(*style),
            )
        })
        .collect();

    let plot = BoxPlot::new()
        .with_rows(latency.rows(0.0))
        .with_labels(latency.labels())
        .with_domain(0.0, 600.0)
        .with_unit("ms")
        .with_presentation(RenderStyle::Blocks)
        .with_tuning(tuning)
        .with_style(BoxPlotStyle::default());

    let plot_row = preset_row(
        theme,
        "box-row",
        "presentation",
        "preset-box",
        7,
        node(
            "box-hint",
            Text::new("ink for the plot and the track").with_style(muted(theme)),
        ),
    );
    let progress_row = preset_row(
        theme,
        "progress-row",
        "progress",
        "preset-progress",
        1,
        node(
            "style-progress",
            Progress::new(0.62)
                .with_label("62%")
                .with_presentation(RenderStyle::Halftone)
                .with_tuning(StyleTuning::default()),
        ),
    );
    let gradient_row = preset_row(
        theme,
        "gradient-row",
        "gradient",
        "preset-gradient",
        1,
        node(
            "style-gradient",
            Text::new(gradient).with_style(TextStyle::default().fg(theme.foreground)),
        ),
    );
    let spinner_row = node(
        "style-spinners",
        Stack::horizontal()
            .gap(2)
            .constraints(vec![Constraint::Length(14), Constraint::Fill(1)])
            .children(vec![
                node(
                    "spinners-label",
                    Text::new("all presets").with_style(muted(theme)),
                ),
                node(
                    "spinners",
                    Stack::horizontal().gap(3).children(spinner_nodes),
                ),
            ]),
    );

    let mut hero_chrome = Chrome::bordered(BorderSet::Plain, None);
    hero_chrome.padding = Padding {
        horizontal: 1,
        vertical: 0,
    };
    let hero = node(
        "style-box-panel",
        Panel::new().with_chrome(hero_chrome).children(vec![node(
            "box-body",
            Stack::vertical()
                .constraints(vec![Constraint::Min(1), Constraint::Length(1)])
                .children(vec![
                    node("style-boxplot", plot),
                    node(
                        "box-legend",
                        Text::new(
                            "box = p25-p75 · rail = p50 · whiskers = p5-p95 · dots = outliers",
                        )
                        .with_style(muted(theme)),
                    ),
                ]),
        )]),
    );

    let mut children = vec![
        node("breadcrumb", breadcrumb(theme, TAB_STYLES)),
        strip_node(
            "live latency boxes · one preset and two knobs, no repeated controls",
            theme,
        ),
        node(
            "window",
            Stack::horizontal()
                .gap(2)
                .constraints(vec![Constraint::Length(14), Constraint::Fill(1)])
                .children(vec![
                    node("window-label", Text::new("window").with_style(muted(theme))),
                    node(
                        "box-window",
                        Text::new(latency.caption(0.0)).with_style(muted(theme)),
                    ),
                ]),
        ),
        hero,
        plot_row,
    ];
    children.extend(tuner_rows(theme, tuning));
    children.push(progress_row);
    children.push(gradient_row);
    children.push(spinner_row);

    let mut constraints = vec![
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(14),
    ];
    for _ in 4..children.len() {
        constraints.push(Constraint::Length(1));
    }

    let root = node(
        "root",
        Stack::vertical()
            .constraints(constraints)
            .children(children),
    );
    Tree::new(root)
}
