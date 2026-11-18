//! Overview, new deploy and help tab trees.

use ratatui::layout::Constraint;
use ratatui::style::Color;

use terminalui::components::badge::{Badge, BadgeVariant};
use terminalui::components::button::{Button, ButtonVariant};
use terminalui::components::form::{Form, FormField};
use terminalui::components::grid::ResizableGrid;
use terminalui::components::icons::Icon;
use terminalui::components::input::Input;
use terminalui::components::list::{List, Marker};
use terminalui::components::pagination::Pagination;
use terminalui::components::table::{Table, TableCell, TableRow};
use terminalui::theme::Theme;
use terminalui::{node, Stack, Text, TextStyle, Tree};

use crate::common::{
    breadcrumb, muted, stack_root, strip_node, tabs_node, PAGE_SIZE, TAB_DEPLOY, TAB_HELP,
    TAB_OVERVIEW,
};
use crate::data::{health_counts, services, status_variant, total_pages, Service, TOTAL_SERVICES};
fn table_header() -> terminalui::Node {
    node(
        "table-header",
        TableRow::header(vec![
            node("header-service", TableCell::text("SERVICE")),
            node("header-env", TableCell::text("ENV")),
            node("header-version", TableCell::text("VERSION")),
            node("header-status", TableCell::text("STATUS")),
        ]),
    )
}

pub(crate) fn table_rows(
    services: &[Service],
    window: &[usize],
    page: usize,
) -> Vec<terminalui::Node> {
    window
        .iter()
        .enumerate()
        .map(|(index, &service_index)| {
            let service = &services[service_index];
            let slot = page * PAGE_SIZE + index;
            node(
                format!("row-{slot}"),
                TableRow::body(vec![
                    node(format!("cell-{slot}-name"), TableCell::text(&service.name)),
                    node(format!("cell-{slot}-env"), TableCell::text(&service.env)),
                    node(
                        format!("cell-{slot}-version"),
                        TableCell::text(&service.version),
                    ),
                    node(
                        format!("cell-{slot}-status"),
                        TableCell::new(vec![node(
                            format!("cell-{slot}-badge"),
                            Badge::new(&service.status)
                                .with_variant(status_variant(&service.status)),
                        )]),
                    ),
                ]),
            )
        })
        .collect()
}

fn table(services: &[Service], window: &[usize], theme: &Theme) -> Table {
    let mut table = Table::new().with_header(table_header());
    table.style.selected_bg = Some(Color::Rgb(15, 118, 110));
    table.style.selected_fg = Some(Color::Rgb(224, 242, 254));
    table.set_rows(table_rows(services, window, 0));
    let _ = theme;
    table
}

pub(crate) fn build_overview_tree(theme: &Theme) -> Tree {
    let services = services();
    let window: Vec<usize> = (0..PAGE_SIZE).collect();
    let table = table(&services, &window, theme);
    let mut filter = Input::new();
    filter.set_placeholder("Type to filter services…");
    let (healthy, degraded, down) = health_counts(&services);
    let root = stack_root(vec![
        node("breadcrumb", breadcrumb(theme, TAB_OVERVIEW)),
        tabs_node(TAB_OVERVIEW),
        strip_node(
            "Services — drag the sidebar divider to resize (or arrow keys when focused)",
            theme,
        ),
        node(
            "grid",
            ResizableGrid::row(vec![1, 3])
                .with_min_sizes(vec![14, 22])
                .panes(vec![
                    node(
                        "sidebar",
                        Stack::vertical()
                            .constraints(vec![
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Min(1),
                            ])
                            .gap(1)
                            .children(vec![
                                node(
                                    "filter-label",
                                    Text::new("FILTER").with_style(muted(theme).bold()),
                                ),
                                node("filter", filter),
                                node(
                                    "health",
                                    Stack::vertical().gap(0).children(vec![
                                        node(
                                            "health-ok",
                                            Badge::new(format!("{healthy} healthy"))
                                                .with_variant(BadgeVariant::Success),
                                        ),
                                        node(
                                            "health-degraded",
                                            Badge::new(format!("{degraded} degraded"))
                                                .with_variant(BadgeVariant::Warning),
                                        ),
                                        node(
                                            "health-down",
                                            Badge::new(format!("{down} down"))
                                                .with_variant(BadgeVariant::Destructive),
                                        ),
                                    ]),
                                ),
                            ]),
                    ),
                    node(
                        "main",
                        Stack::vertical()
                            .constraints(vec![Constraint::Min(1), Constraint::Length(1)])
                            .children(vec![
                                node("table", table),
                                node("pagination", Pagination::new(total_pages(TOTAL_SERVICES))),
                            ]),
                    ),
                ]),
        ),
    ]);
    Tree::new(root)
}

pub(crate) fn build_deploy_tree(theme: &Theme) -> Tree {
    let service = FormField::text("service", "Service")
        .placeholder("web-api-00")
        .required();
    let image = FormField::text("image", "Image")
        .placeholder("registry.example.com/web-api-00")
        .required();
    let tag = FormField::text("tag", "Tag")
        .placeholder("v2.14.1")
        .required();
    let replicas = FormField::text("replicas", "Replicas").placeholder("3");
    let root = stack_root(vec![
        node("breadcrumb", breadcrumb(theme, TAB_DEPLOY)),
        tabs_node(TAB_DEPLOY),
        strip_node(
            "Fill the form — Enter submits, Esc goes back to the tab bar",
            theme,
        ),
        node(
            "form",
            Form::new(vec![
                node("service", service),
                node("image", image),
                node("tag", tag),
                node("replicas", replicas),
            ]),
        ),
    ]);
    Tree::new(root)
}

pub(crate) fn build_help_tree(theme: &Theme) -> Tree {
    let shortcuts = [
        "F1-F8    jump to a tab from anywhere",
        "1 - 8    jump to a tab (outside text fields)",
        "TAB      cycle focus across components",
        "ARROWS   move within the focused component",
        "[ ]      select a divider, resize the grid",
        "ENTER    activate / confirm",
        "ESC      back to the tab bar",
        "Q        quit (with confirmation)",
    ];
    let icons_line: String = [
        Icon::Check,
        Icon::X,
        Icon::ChevronLeft,
        Icon::ChevronRight,
        Icon::ChevronUp,
        Icon::ChevronDown,
        Icon::ArrowLeft,
        Icon::ArrowRight,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::Plus,
        Icon::Minus,
        Icon::Search,
        Icon::Info,
        Icon::Warning,
        Icon::Error,
        Icon::Star,
        Icon::Circle,
        Icon::Ellipsis,
        Icon::Home,
    ]
    .iter()
    .map(|icon| icon.glyph())
    .collect::<Vec<_>>()
    .join(" ");
    let root = stack_root(vec![
        node("breadcrumb", breadcrumb(theme, TAB_HELP)),
        tabs_node(TAB_HELP),
        strip_node("terminalui-components — copyable TUI widgets", theme),
        node(
            "help-body",
            Stack::vertical()
                .gap(0)
                .constraints(vec![
                    Constraint::Length(8),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .children(vec![
                    node(
                        "keys",
                        List::with_items(shortcuts.to_vec()).with_marker(Marker::None),
                    ),
                    node("keys-spacer", Text::new("")),
                    node("icon-label", Text::new("icon set").with_style(muted(theme))),
                    node(
                        "icons",
                        Text::new(icons_line).with_style(TextStyle::default().fg(theme.accent)),
                    ),
                    node(
                        "help-actions",
                        Stack::horizontal().gap(2).children(vec![
                            node(
                                "about-button",
                                Button::new(ButtonVariant::Secondary, "About terminalui"),
                            ),
                            node(
                                "quit-button",
                                Button::new(ButtonVariant::Destructive, "Quit demo"),
                            ),
                        ]),
                    ),
                ]),
        ),
    ]);
    Tree::new(root)
}
