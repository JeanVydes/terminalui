//! The catalog tab: every component in the kit, one showcase page.

use ratatui::layout::Constraint;
use ratatui::style::Color;

use terminalui::theme::Theme;
use terminalui::Spacer;
use terminalui::{node, Stack, Text, TextStyle, Tree};
use terminalui::components::badge::{Badge, BadgeVariant};
use terminalui::components::button::{Button, ButtonVariant};
use terminalui::components::checkbox::{Checkbox, CheckboxStyle};
use terminalui::components::chrome::{Border, BorderSet, Chrome, Padding, Radius};
use terminalui::components::expandable::Expandable;
use terminalui::components::floating::FloatingBox;
use terminalui::components::icons::Icon;
use terminalui::components::input::Input;
use terminalui::components::list::{List, Marker};
use terminalui::components::progress::Progress;
use terminalui::components::select::Select;
use terminalui::components::separator::Separator;
use terminalui::components::spinner::Spinner;
use terminalui::components::table::{Table, TableCell, TableRow};
use terminalui::components::toast::{Toast, ToastKind};
use terminalui::components::tooltip::Tooltip;

use crate::common::{breadcrumb, strip_node, tabs_node, TAB_CATALOG};

fn section(theme: &Theme, title: &str) -> terminalui::Node {
    node(
        format!("section-{title}"),
        Text::new(format!("── {title}")).with_style(TextStyle::default().fg(theme.muted).bold()),
    )
}

fn mini_table() -> Table {
    let rows = (0..2)
        .map(|index| {
            node(
                format!("cat-row-{index}"),
                TableRow::body(vec![
                    node(
                        format!("cat-name-{index}"),
                        TableCell::text(format!("service-{index}")),
                    ),
                    node(format!("cat-status-{index}"), TableCell::text("healthy")),
                ]),
            )
        })
        .collect();
    let mut table = Table::new().with_header(node(
        "cat-header",
        TableRow::header(vec![
            node("cat-header-name", TableCell::text("SERVICE")),
            node("cat-header-status", TableCell::text("STATUS")),
        ]),
    ));
    table.set_rows(rows);
    table
}

pub(crate) fn build_catalog_tree(theme: &Theme) -> Tree {
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
        Icon::Question,
        Icon::Star,
        Icon::Circle,
        Icon::Dot,
        Icon::Ellipsis,
        Icon::Home,
        Icon::Menu,
        Icon::ExternalLink,
    ]
    .iter()
    .map(|icon| icon.glyph())
    .collect::<Vec<_>>()
    .join(" ");
    let root = node(
        "root",
        Stack::vertical()
            .constraints(vec![
                Constraint::Length(1), // breadcrumb
                Constraint::Length(2), // tabs
                Constraint::Length(1), // strip
                Constraint::Length(1), // actions
                Constraint::Length(1), // buttons
                Constraint::Length(1), // bordered buttons
                Constraint::Length(1), // inputs
                Constraint::Length(4), // multiline input
                Constraint::Length(1), // checkboxes + select
                Constraint::Length(1), // status
                Constraint::Length(1), // spinners + progress
                Constraint::Length(1), // badges
                Constraint::Length(1), // data display
                Constraint::Min(4),    // table + floating
                Constraint::Length(3), // list + expandable
                Constraint::Length(1), // icons
                Constraint::Length(1), // feedback
                Constraint::Length(1), // hover trigger + dialog button
                Constraint::Length(1), // toast
                Constraint::Length(1), // tooltip
                Constraint::Length(1), // structure
                Constraint::Length(1), // separators
                Constraint::Length(1), // separators
                Constraint::Length(1), // separators
                Constraint::Length(1), // breadcrumb row
                Constraint::Fill(1),   // trailing slack
            ])
            .children(vec![
                node("breadcrumb", breadcrumb(theme, TAB_CATALOG)),
                tabs_node(TAB_CATALOG),
                strip_node(
                    "the whole kit on one page — Tab to focus, Enter to activate",
                    theme,
                ),
                section(theme, "actions"),
                node(
                    "cat-buttons",
                    Stack::horizontal().gap(1).children(vec![
                        node(
                            "cat-button-primary",
                            Button::new(ButtonVariant::Primary, "Primary"),
                        ),
                        node(
                            "cat-button-secondary",
                            Button::new(ButtonVariant::Secondary, "Secondary"),
                        ),
                        node(
                            "cat-button-destructive",
                            Button::new(ButtonVariant::Destructive, "Del"),
                        ),
                        node(
                            "cat-button-ghost",
                            Button::new(ButtonVariant::Ghost, "Ghost"),
                        ),
                        node(
                            "cat-button-outline",
                            Button::new(ButtonVariant::Outline, "Outline"),
                        ),
                        {
                            let mut loading = Button::new(ButtonVariant::Primary, "Loading");
                            loading.set_loading(true);
                            node("cat-button-loading", loading)
                        },
                        {
                            let mut disabled = Button::new(ButtonVariant::Secondary, "Disabled");
                            disabled.set_disabled(true);
                            node("cat-button-disabled", disabled)
                        },
                    ]),
                ),
                node(
                    "cat-buttons-border",
                    Stack::horizontal().gap(1).children(vec![
                        {
                            let mut primary = Button::new(ButtonVariant::Secondary, "Rounded");
                            primary =
                                primary.with_style(terminalui::components::button::ButtonStyle {
                                    background: None,
                                    focused_ring: None,
                                    padding_x: Some(2),
                                    border: Some(Border {
                                        set: BorderSet::Plain,
                                        color: Some(Color::Rgb(125, 211, 252)),
                                        radius: Radius::Small,
                                    }),
                                    radius: Radius::Small,
                                });
                            node("cat-button-rounded", primary)
                        },
                        {
                            let mut thick = Button::new(ButtonVariant::Secondary, "Thick");
                            thick = thick.with_style(terminalui::components::button::ButtonStyle {
                                background: None,
                                focused_ring: None,
                                padding_x: Some(2),
                                border: Some(Border {
                                    set: BorderSet::Thick,
                                    color: Some(theme.accent),
                                    radius: Radius::None,
                                }),
                                radius: Radius::None,
                            });
                            node("cat-button-thick", thick)
                        },
                        {
                            let mut dashed = Button::new(ButtonVariant::Secondary, "Dashed");
                            dashed =
                                dashed.with_style(terminalui::components::button::ButtonStyle {
                                    background: None,
                                    focused_ring: None,
                                    padding_x: Some(2),
                                    border: Some(Border {
                                        set: BorderSet::Dashed,
                                        color: Some(theme.destructive),
                                        radius: Radius::None,
                                    }),
                                    radius: Radius::None,
                                });
                            node("cat-button-dashed", dashed)
                        },
                        {
                            let mut double = Button::new(ButtonVariant::Secondary, "Double");
                            double =
                                double.with_style(terminalui::components::button::ButtonStyle {
                                    background: None,
                                    focused_ring: None,
                                    padding_x: Some(2),
                                    border: Some(Border {
                                        set: BorderSet::Double,
                                        color: Some(theme.success),
                                        radius: Radius::None,
                                    }),
                                    radius: Radius::None,
                                });
                            node("cat-button-double", double)
                        },
                    ]),
                ),
                section(theme, "inputs"),
                node(
                    "cat-checkboxes-select",
                    Stack::horizontal()
                        .gap(2)
                        .constraints(vec![Constraint::Fill(1), Constraint::Length(30)])
                        .children(vec![
                            node(
                                "cat-checkboxes",
                                Stack::horizontal().gap(2).children(vec![
                                    node(
                                        "cat-checkbox-on",
                                        Checkbox::with_label("On").with_checked(true),
                                    ),
                                    node("cat-checkbox-off", Checkbox::with_label("Off")),
                                    node(
                                        "cat-checkbox-rounded",
                                        Checkbox::with_label("Rounded").with_style(CheckboxStyle {
                                            checked: Some(theme.accent),
                                            foreground: None,
                                            background: None,
                                            border: Some(Border {
                                                set: BorderSet::Rounded,
                                                color: Some(theme.muted),
                                                radius: Radius::Small,
                                            }),
                                            radius: Radius::Small,
                                            checked_glyph: 'x',
                                            unchecked_glyph: ' ',
                                        }),
                                    ),
                                ]),
                            ),
                            node(
                                "cat-select",
                                Select::new(vec![
                                    node("opt-prod", Text::new("production")),
                                    node("opt-staging", Text::new("staging")),
                                    node("opt-preview", Text::new("preview")),
                                    node("opt-canary", Text::new("canary")),
                                ])
                                .with_selected(0),
                            ),
                        ]),
                ),
                node(
                    "cat-multiline",
                    Input::new()
                        .multiline(true)
                        .with_style(terminalui::components::input::InputStyle {
                            background: None,
                            foreground: None,
                            placeholder: None,
                            cursor: None,
                            border: Some(Border {
                                set: BorderSet::Rounded,
                                color: Some(Color::Rgb(125, 211, 252)),
                                radius: Radius::Small,
                            }),
                            radius: Radius::Small,
                            scrollbar: terminalui::components::scroll::ScrollbarStyle::default(),
                            cursor_style: terminalui::components::cursor::CursorStyle::default(),
                        })
                        .with_placeholder("type lines, Enter inserts a newline"),
                ),
                section(theme, "status"),
                node(
                    "cat-spinners-progress",
                    Stack::horizontal().gap(2).children(vec![
                        node("cat-spinner-1", Spinner::new()),
                        node(
                            "cat-spinner-2",
                            Spinner::new().with_presentation(
                                terminalui::components::presentation::RenderStyle::Blocks,
                            ),
                        ),
                        node(
                            "cat-spinner-3",
                            Spinner::new().with_presentation(
                                terminalui::components::presentation::RenderStyle::Halftone,
                            ),
                        ),
                        node("cat-progress-det", Progress::new(0.75).with_label("75%")),
                        node("cat-progress-low", Progress::new(0.25).with_label("25%")),
                        node(
                            "cat-progress-halftone",
                            Progress::new(0.62)
                                .with_label("62%")
                                .with_presentation(
                                    terminalui::components::presentation::RenderStyle::Halftone,
                                )
                                .with_tuning(terminalui::components::presentation::StyleTuning {
                                    density: 6,
                                    contrast: 1.0,
                                    dotsize: terminalui::components::presentation::DotSize::Medium,
                                }),
                        ),
                    ]),
                ),
                node(
                    "cat-badges",
                    Stack::horizontal().gap(1).children(vec![
                        node("cat-badge-neutral", Badge::new("Neutral")),
                        node(
                            "cat-badge-success",
                            Badge::new("Success").with_variant(BadgeVariant::Success),
                        ),
                        node(
                            "cat-badge-warning",
                            Badge::new("Warning").with_variant(BadgeVariant::Warning),
                        ),
                        node(
                            "cat-badge-destructive",
                            Badge::new("Failed").with_variant(BadgeVariant::Destructive),
                        ),
                    ]),
                ),
                section(theme, "data display"),
                node(
                    "cat-row-table",
                    Stack::horizontal()
                        .gap(2)
                        .constraints(vec![Constraint::Length(34), Constraint::Min(20)])
                        .children(vec![
                            node("cat-table", mini_table()),
                            node(
                                "cat-floating",
                                FloatingBox::with_title_body(
                                    "FloatingBox",
                                    vec!["composes any views", "into one surface"],
                                )
                                .with_width(28),
                            ),
                        ]),
                ),
                node(
                    "cat-list-expandable",
                    Stack::horizontal()
                        .gap(2)
                        .constraints(vec![Constraint::Length(30), Constraint::Min(20)])
                        .children(vec![
                            node(
                                "cat-list",
                                List::with_items(vec!["alpha", "beta", "gamma", "delta"])
                                    .with_marker(Marker::Decimal),
                            ),
                            node(
                                "cat-expandable",
                                Expandable::with_lines(
                                    "Expandable",
                                    vec!["click to toggle", "this content"],
                                )
                                .with_style(
                                    terminalui::components::expandable::ExpandableStyle {
                                        header: None,
                                        chevron: Some(theme.accent),
                                        highlight: None,
                                        bold: None,
                                    },
                                ),
                            ),
                        ]),
                ),
                node(
                    "cat-icons",
                    Text::new(icons_line).with_style(TextStyle::default().fg(theme.accent)),
                ),
                section(theme, "feedback"),
                node(
                    "cat-feedback-row",
                    Stack::horizontal().gap(2).children(vec![
                        node(
                            "cat-hover-trigger",
                            Button::new(ButtonVariant::Ghost, "hover me for tip"),
                        ),
                        node(
                            "cat-dialog-button",
                            Button::new(ButtonVariant::Secondary, "Open a dialog"),
                        ),
                    ]),
                ),
                node(
                    "cat-toast",
                    Toast::new(ToastKind::Success, "deploy complete — nothing is on fire")
                        .with_ttl(std::time::Duration::from_secs(3600))
                        .with_position(
                            terminalui::components::floating::FloatingPosition::BottomRight,
                        ),
                ),
                {
                    let mut tooltip = Tooltip::new("I only appear on hover", 0, 0);
                    tooltip.hide();
                    node("cat-tooltip", tooltip)
                },
                section(theme, "structure"),
                node("cat-sep", Separator::new()),
                node(
                    "cat-sep-rounded",
                    Separator::new().with_chrome(Chrome {
                        border: Some(Border {
                            set: BorderSet::Plain,
                            color: Some(theme.muted),
                            radius: Radius::Small,
                        }),
                        background: None,
                        radius: Radius::Small,
                        padding: Padding::default(),
                        border_gradient: None,
                        fill_gradient: None,
                    }),
                ),
                node(
                    "cat-sep-dashed",
                    Separator::new()
                        .with_chrome(Chrome::bordered(BorderSet::Dashed, Some(theme.muted))),
                ),
                node(
                    "cat-crumb",
                    terminalui::components::breadcrumb::Breadcrumb::new(vec![
                        node(
                            "crumb-home",
                            Stack::horizontal().gap(0).children(vec![
                                node("icon", Text::new(Icon::Home.glyph())),
                                node("label", Text::new("acme")),
                            ]),
                        ),
                        node(
                            "crumb-deploys",
                            Stack::horizontal().gap(0).children(vec![
                                node("icon", Text::new(Icon::Menu.glyph())),
                                node("label", Text::new("deploys")),
                            ]),
                        ),
                        node(
                            "crumb-current",
                            Stack::horizontal().gap(0).children(vec![
                                node("icon", Text::new(Icon::Star.glyph())),
                                node("label", Text::new("overview")),
                            ]),
                        ),
                    ]),
                ),
                node("catalog-slack", Spacer::new()),
            ]),
    );
    Tree::new(root)
}
