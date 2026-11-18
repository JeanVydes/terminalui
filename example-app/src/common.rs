//! Shared helpers for the terminalui showcase: tab constants and small
//! constructors used by every tab tree.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Color;

use terminalui::theme::Theme;
use terminalui::{node, Stack, Text, TextStyle, Tree};
use terminalui::components::breadcrumb::Breadcrumb;
use terminalui::components::button::{Button, ButtonVariant};
use terminalui::components::dialog::Dialog;
use terminalui::components::tabs::Tabs;

pub(crate) const TAB_OVERVIEW: usize = 0;
pub(crate) const TAB_DEPLOY: usize = 1;
pub(crate) const TAB_HELP: usize = 2;
pub(crate) const TAB_MARKET: usize = 3;
pub(crate) const TAB_FX: usize = 4;
pub(crate) const TAB_STYLES: usize = 5;
pub(crate) const TAB_COLORS: usize = 6;
pub(crate) const TAB_CATALOG: usize = 7;
pub(crate) const TAB_NAMES: [&str; 8] = [
    "overview",
    "new deploy",
    "help",
    "market",
    "fx",
    "styles",
    "colors",
    "catalog",
];
pub(crate) const FOCUS_DEFAULT: [&str; 8] = [
    "filter",
    "service:input",
    "about-button",
    "chart-1s",
    "tabs",
    "preset-box",
    "colors",
    "cat-button-primary",
];

pub(crate) const PAGE_SIZE: usize = 7;

pub(crate) fn fx_for(index: usize) -> (terminalui::Effect, &'static str) {
    use terminalui::{Direction, Effect};
    match index % 8 {
        0 => (Effect::slide_bounce(Direction::Left, 18), "slide-bounce"),
        1 => (Effect::slide_in(Direction::Right, 18), "slide-in"),
        2 => (Effect::RevealRight, "reveal-right"),
        3 => (
            Effect::SweepRight {
                color: Some(Color::Rgb(255, 166, 87)),
            },
            "sweep",
        ),
        4 => (Effect::FadeIn, "fade-in"),
        5 => (Effect::Dissolve { seed: 42 }, "dissolve"),
        6 => (Effect::slide_bounce(Direction::Up, 12), "slide-bounce-up"),
        _ => (Effect::SweepLeft { color: None }, "sweep-left"),
    }
}

/// One entrance animation per market chart: rise, fade, dissolve, sweep.
pub(crate) fn chart_entrance_fxs() -> [Option<FxState>; 4] {
    use terminalui::{Animation, Direction, Easing, Effect};
    let entrance = |effect: Effect| {
        Some(FxState {
            effect,
            anim: Animation::new(Duration::from_millis(650), Easing::EaseOutCubic),
        })
    };
    [
        entrance(Effect::slide_in(Direction::Up, 10)),
        entrance(Effect::FadeIn),
        entrance(Effect::Dissolve { seed: 42 }),
        entrance(Effect::SweepRight { color: None }),
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FxState {
    pub effect: terminalui::Effect,
    pub anim: terminalui::Animation,
}

pub(crate) fn muted(theme: &Theme) -> TextStyle {
    TextStyle::default().fg(theme.muted)
}

pub(crate) fn blit(dst: &mut Buffer, src: &Buffer, area: Rect) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let Some(cell) = src.cell((x, y)).cloned() else {
                continue;
            };
            if let Some(dst_cell) = dst.cell_mut((x, y)) {
                *dst_cell = cell;
            }
        }
    }
}

pub(crate) fn id(name: &str) -> String {
    name.to_string()
}

pub(crate) fn stack_root(children: Vec<terminalui::Node>) -> terminalui::Node {
    node(
        "root",
        Stack::vertical()
            .constraints(vec![
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .children(children),
    )
}

pub(crate) fn breadcrumb(theme: &Theme, tab: usize) -> Breadcrumb {
    let _ = theme;
    Breadcrumb::with_segments(vec!["acme", "deploys", crate::common::TAB_NAMES[tab]])
}

pub(crate) fn tabs_node(initial: usize) -> terminalui::Node {
    let mut tabs = Tabs::with_titles(TAB_NAMES.to_vec());
    tabs.set_selected(initial);
    node("tabs", tabs)
}

pub(crate) fn strip_node(text: &str, theme: &Theme) -> terminalui::Node {
    node("strip", Text::new(text).with_style(muted(theme)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialogKind {
    RowDetails,
    DeployQueued,
    About,
    ConfirmQuit,
}

pub(crate) fn build_dialog_tree(
    kind: DialogKind,
    values: Option<Vec<(String, String)>>,
    selected: Option<&crate::data::Service>,
    theme: &Theme,
) -> Tree {
    let get = |name: &str| {
        values
            .as_ref()
            .and_then(|pairs| pairs.iter().find(|(key, _)| key == name))
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    let (title, message, confirm, cancel, confirm_variant) = match kind {
        DialogKind::RowDetails => {
            let (name, env, version, status) = match selected {
                Some(service) => (
                    service.name.clone(),
                    service.env.clone(),
                    service.version.clone(),
                    service.status.clone(),
                ),
                None => (
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                ),
            };
            (
                name,
                format!(
                    "environment: {env}\nversion:    {version}\nstatus:     {status}\n\nSend a rollout ping?"
                ),
                "Send ping".to_string(),
                "Close".to_string(),
                ButtonVariant::Primary,
            )
        }
        DialogKind::DeployQueued => {
            let replicas = get("replicas");
            (
                "Deploy queued".to_string(),
                format!(
                    "service:  {}\nimage:    {}:{}\nreplicas: {}\n\nA rollout will start shortly.",
                    get("service"),
                    get("image"),
                    get("tag"),
                    if replicas.trim().is_empty() {
                        "3 (default)".to_string()
                    } else {
                        replicas
                    },
                ),
                "OK".to_string(),
                "Close".to_string(),
                ButtonVariant::Primary,
            )
        }
        DialogKind::About => (
            "terminalui deploys 0.1.0".to_string(),
            "A dummy app built with terminalui-components.\n\nCopyable TUI widgets for\nratatui + crossterm.".to_string(),
            "OK".to_string(),
            "Close".to_string(),
            ButtonVariant::Primary,
        ),
        DialogKind::ConfirmQuit => (
            "Quit terminalui deploys?".to_string(),
            "Any unsaved state will be lost.".to_string(),
            "Quit".to_string(),
            "Stay".to_string(),
            ButtonVariant::Destructive,
        ),
    };
    let actions = node(
        "actions",
        Stack::horizontal().gap(2).children(vec![
            node("cancel", Button::new(ButtonVariant::Secondary, cancel)),
            node("confirm", Button::new(confirm_variant, confirm)),
        ]),
    );
    let mut dialog = Dialog::new(vec![actions])
        .with_title(title)
        .with_message(message);
    dialog.state.is_open = true;
    let _ = theme;
    let mut tree = Tree::new(node("dialog", dialog));
    tree.focus(&crate::common::id("confirm"));
    tree
}
