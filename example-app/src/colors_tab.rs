//! The colors tab: every color in the theme, swatched with its role.
//! `t` cycles the three palettes shipped with the crate.

use ratatui::style::Color;

use terminalui::theme::Theme;
use terminalui::{node, Stack, Text, TextStyle, Tree};

use crate::common::{breadcrumb, muted, stack_root, strip_node, tabs_node, TAB_COLORS};

struct Swatch {
    name: &'static str,
    role: &'static str,
    color: fn(&Theme) -> Color,
}

const SWATCHES: &[Swatch] = &[
    Swatch {
        name: "background",
        role: "app background",
        color: |t| t.background,
    },
    Swatch {
        name: "foreground",
        role: "default text",
        color: |t| t.foreground,
    },
    Swatch {
        name: "primary",
        role: "filled actions, selected rows",
        color: |t| t.primary,
    },
    Swatch {
        name: "secondary",
        role: "secondary surfaces",
        color: |t| t.secondary,
    },
    Swatch {
        name: "muted",
        role: "hints, table header, tab strip",
        color: |t| t.muted,
    },
    Swatch {
        name: "accent",
        role: "emphasis, spinner, live price",
        color: |t| t.accent,
    },
    Swatch {
        name: "destructive",
        role: "destructive actions, down candles",
        color: |t| t.destructive,
    },
    Swatch {
        name: "success",
        role: "positive, up candles",
        color: |t| t.success,
    },
    Swatch {
        name: "warning",
        role: "degraded state",
        color: |t| t.warning,
    },
    Swatch {
        name: "border",
        role: "hairlines, gridlines",
        color: |t| t.border,
    },
    Swatch {
        name: "ring",
        role: "focus ring",
        color: |t| t.ring,
    },
    Swatch {
        name: "border_set",
        role: "dialog/box borders",
        color: |t| t.border_set,
    },
];

fn swatch_row(theme: &Theme, swatch: &Swatch) -> terminalui::Node {
    let color = (swatch.color)(theme);
    node(
        format!("swatch-{}", swatch.name),
        Stack::horizontal()
            .constraints(vec![
                ratatui::layout::Constraint::Length(12),
                ratatui::layout::Constraint::Length(16),
                ratatui::layout::Constraint::Length(20),
                ratatui::layout::Constraint::Min(10),
            ])
            .gap(1)
            .children(vec![
                node(
                    "block",
                    Text::new("            ").with_style({
                        let mut style = TextStyle::default().fg(color);
                        style.background = Some(color);
                        style
                    }),
                ),
                node(
                    "name",
                    Text::new(swatch.name).with_style(TextStyle::default().fg(theme.accent).bold()),
                ),
                node(
                    "rgb",
                    Text::new(format!("{color:?}")).with_style(muted(theme)),
                ),
                node("role", Text::new(swatch.role).with_style(muted(theme))),
            ]),
    )
}

pub(crate) fn build_colors_tree(theme: &Theme) -> Tree {
    let rows: Vec<terminalui::Node> = SWATCHES
        .iter()
        .map(|swatch| swatch_row(theme, swatch))
        .collect();
    let root = stack_root(vec![
        node("breadcrumb", breadcrumb(theme, TAB_COLORS)),
        tabs_node(TAB_COLORS),
        strip_node(
            "every color the kit paints with — press t to cycle default / ansi16 / ansi256",
            theme,
        ),
        node("colors", Stack::vertical().gap(0).children(rows)),
    ]);
    Tree::new(root)
}
