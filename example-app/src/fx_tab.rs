//! The fx tab: every easing/transition in terminalui-animations, looped.

use terminalui::theme::Theme;
use terminalui::{node, Text, TextStyle, Tree};
use terminalui::components::grid::ResizableGrid;
use terminalui::components::panel::Panel;

use crate::common::{breadcrumb, stack_root, strip_node, tabs_node, TAB_FX};
use terminalui::{Direction, Effect};

/// Every animation effect shipped by terminalui-animations, in render order.
pub(crate) fn fx_cells() -> Vec<(&'static str, Effect)> {
    vec![
        ("fade-in", Effect::FadeIn),
        ("fade-out", Effect::FadeOut),
        ("slide-bounce", Effect::slide_bounce(Direction::Left, 14)),
        ("slide-up", Effect::slide_in(Direction::Up, 10)),
        ("slide-out-right", Effect::slide_out(Direction::Right, 12)),
        ("reveal-right", Effect::RevealRight),
        ("reveal-down", Effect::RevealDown),
        ("reveal-left", Effect::RevealLeft),
        ("reveal-up", Effect::RevealUp),
        (
            "sweep-right",
            Effect::SweepRight {
                color: Some(ratatui::style::Color::Rgb(56, 189, 248)),
            },
        ),
        ("sweep-left", Effect::SweepLeft { color: None }),
        ("dissolve", Effect::Dissolve { seed: 42 }),
        ("thanos", Effect::Thanos { seed: 42 }),
        ("fall", Effect::Fall),
    ]
}

fn panel(id: &str, name: &str, theme: &Theme) -> terminalui::Node {
    node(
        id.to_string(),
        Panel::line().children(vec![node(
            format!("{id}-name"),
            Text::new(name).with_style(TextStyle::default().fg(theme.accent).bold()),
        )]),
    )
}

/// Rebuild a fresh looping cell animation, seeded so dissolve/thanos vary.
pub(crate) fn fresh_cell(index: usize, seed: u64) -> crate::common::FxState {
    use terminalui::{Animation, Easing, Effect};
    let effects = fx_cells();
    let effect = effects
        .get(index)
        .map(|(_, effect)| effect.clone())
        .unwrap_or(Effect::FadeIn);
    let effect = match effect {
        Effect::Dissolve { .. } => Effect::Dissolve {
            seed: seed % 97 + 1,
        },
        Effect::Thanos { .. } => Effect::Thanos {
            seed: seed % 97 + 1,
        },
        other => other,
    };
    let duration_ms = 700 + (seed % 5) * 150;
    crate::common::FxState {
        effect,
        anim: Animation::new(
            std::time::Duration::from_millis(duration_ms),
            Easing::EaseOutCubic,
        ),
    }
}

pub(crate) fn build_fx_tree(theme: &Theme) -> Tree {
    let cells = fx_cells();
    let (left, right) = cells.split_at(7);
    let left_panes = left
        .iter()
        .enumerate()
        .map(|(index, (name, _))| panel(&format!("fx-cell-{index}"), name, theme))
        .collect();
    let right_panes = right
        .iter()
        .enumerate()
        .map(|(index, (name, _))| panel(&format!("fx-cell-{}", index + 7), name, theme))
        .collect();
    let root = stack_root(vec![
        node("breadcrumb", breadcrumb(theme, TAB_FX)),
        tabs_node(TAB_FX),
        strip_node(
            "every terminalui-animations effect, looping — tab bar on the left",
            theme,
        ),
        node(
            "fx-grid",
            ResizableGrid::row(vec![1, 1])
                .with_min_sizes(vec![20, 20])
                .panes(vec![
                    node(
                        "fx-left",
                        ResizableGrid::column(vec![1; 7])
                            .with_min_sizes(vec![3; 7])
                            .panes(left_panes),
                    ),
                    node(
                        "fx-right",
                        ResizableGrid::column(vec![1; 7])
                            .with_min_sizes(vec![3; 7])
                            .panes(right_panes),
                    ),
                ]),
        ),
    ]);
    Tree::new(root)
}
