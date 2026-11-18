//! Smoke tests: every component must render and handle events without
//! panicking, even in tiny areas. All interaction goes through `Tree`,
//! the public entry point.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use terminalui_components::badge::{Badge, BadgeVariant};
use terminalui_components::boxplot::{BoxPlot, BoxPlotRow};
use terminalui_components::breadcrumb::Breadcrumb;
use terminalui_components::button::{Button, ButtonVariant};
use terminalui_components::candlechart::{Candle, CandleChart};
use terminalui_components::dialog::Dialog;
use terminalui_components::expandable::Expandable;
use terminalui_components::form::{Form, FormField};
use terminalui_components::grid::ResizableGrid;
use terminalui_components::icons::Icon;
use terminalui_components::list::{List, Marker};
use terminalui_components::pagination::Pagination;
use terminalui_components::presentation::RenderStyle;
use terminalui_components::progress::Progress;
use terminalui_components::spinner::Spinner;
use terminalui_components::table::{Table, TableCell, TableRow};
use terminalui_components::tabs::{Tab, Tabs};
use terminalui_components::toast::{Toast, ToastKind};
use terminalui_components::tooltip::Tooltip;
use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Stack, Text, Tree};

fn id(name: &str) -> String {
    name.to_string()
}

fn json(tree: &Tree) -> serde_json::Value {
    serde_json::from_str(&tree.to_json().expect("serialization succeeds")).expect("valid json")
}

fn find<'a>(node: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
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

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn click(column: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn wheel(column: u16, row: u16, up: bool) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: if up {
            MouseEventKind::ScrollUp
        } else {
            MouseEventKind::ScrollDown
        },
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn theme() -> Theme {
    Theme::default()
}

fn focus(tree: &mut Tree, name: &str) {
    assert!(tree.focus(&id(name)), "focus failed for {name}");
}

/// Render a tree at a matrix of sizes; it must never panic.
fn render_tree(tree: &mut Tree) {
    let theme = theme();
    for (width, height) in [(1, 1), (8, 2), (20, 3), (40, 8), (80, 24)] {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        tree.render(area, &mut buffer, &theme);
    }
}

fn buffer_text(buffer: &Buffer, area: Rect) -> String {
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn render_text(tree: &mut Tree, area: Rect) -> String {
    let theme = theme();
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    buffer_text(&buffer, area)
}

fn type_text(tree: &mut Tree, text: &str) {
    for character in text.chars() {
        tree.handle_event(&key(KeyCode::Char(character)));
    }
}

fn every_size() -> [(u16, u16); 5] {
    [(1, 1), (8, 2), (20, 3), (40, 8), (80, 24)]
}

#[test]
fn badge_renders_all_variants() {
    for variant in [
        BadgeVariant::Neutral,
        BadgeVariant::Success,
        BadgeVariant::Warning,
        BadgeVariant::Destructive,
    ] {
        let mut tree = Tree::new(node("badge", Badge::new("Deployed").with_variant(variant)));
        render_tree(&mut tree);
    }
}

#[test]
fn icons_have_glyphs() {
    for icon in [
        Icon::Check,
        Icon::ChevronLeft,
        Icon::Ellipsis,
        Icon::Warning,
    ] {
        assert!(!icon.glyph().is_empty());
    }
}

#[test]
fn breadcrumb_renders_segments_and_moves_highlight() {
    let mut tree = Tree::new(node(
        "breadcrumb",
        Breadcrumb::with_segments(vec!["Home", "Docs", "Button"]),
    ));
    let area = Rect::new(0, 0, 60, 3);
    let text = render_text(&mut tree, area);
    assert!(text.contains("Home"), "missing First segment: {text}");
    assert!(text.contains("Docs"), "missing second segment: {text}");
    assert!(text.contains("Button"), "missing last segment: {text}");
    focus(&mut tree, "breadcrumb");
    tree.handle_event(&key(KeyCode::Left));
    let highlight = tree
        .find::<Breadcrumb>(&id("breadcrumb"))
        .map(|breadcrumb| breadcrumb.highlight());
    assert_eq!(highlight, Some(1));
    tree.handle_event(&key(KeyCode::End));
    let highlight = tree
        .find::<Breadcrumb>(&id("breadcrumb"))
        .map(|breadcrumb| breadcrumb.highlight());
    assert_eq!(highlight, Some(2));
}

#[test]
fn tabs_skip_disabled_tabs() {
    let mut tree = Tree::new(node(
        "tabs",
        Tabs::new(vec![
            node("tab-0", Tab::with_title("Overview")),
            node("tab-1", Tab::disabled("Soon")),
            node("tab-2", Tab::with_title("Billing")),
        ]),
    ));
    render_tree(&mut tree);
    focus(&mut tree, "tabs");
    tree.handle_event(&key(KeyCode::Right));
    let selected = tree.find::<Tabs>(&id("tabs")).map(Tabs::selected);
    assert_eq!(selected, Some(2));
    tree.handle_event(&key(KeyCode::Left));
    let selected = tree.find::<Tabs>(&id("tabs")).map(Tabs::selected);
    assert_eq!(selected, Some(0));
}

#[test]
fn pagination_changes_page() {
    let mut tree = Tree::new(node("pagination", Pagination::with_page(9, 0)));
    render_tree(&mut tree);
    focus(&mut tree, "pagination");
    tree.handle_event(&key(KeyCode::Right));
    assert_eq!(
        tree.find::<Pagination>(&id("pagination"))
            .map(Pagination::page),
        Some(1)
    );
}

#[test]
fn pagination_changes_page_on_mouse_click() {
    let mut tree = Tree::new(node("pagination", Pagination::new(9)));
    let area = Rect::new(0, 0, 60, 1);
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    let click = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 6,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    tree.handle_event(&click);
    assert_eq!(
        tree.find::<Pagination>(&id("pagination"))
            .map(Pagination::page),
        Some(1)
    );
}

#[test]
fn pagination_max_visible_pages_is_configurable() {
    let pagination = Pagination::new(100).with_max_visible_pages(3);
    assert_eq!(pagination.max_visible_pages, 3);
    let pagination = Pagination::new(100).with_max_visible_pages(1);
    assert_eq!(pagination.max_visible_pages, 1);
    let mut tree = Tree::new(node("pagination", pagination));
    render_tree(&mut tree);
}

fn sample_services_table() -> Table {
    let rows = (0..12).map(|index| {
        node(
            format!("row-{index}"),
            TableRow::body(vec![
                node(
                    format!("cell-{index}-name"),
                    TableCell::text(format!("service-{index}")),
                ),
                node(
                    format!("cell-{index}-value"),
                    TableCell::text(format!("value-{index}")),
                ),
            ]),
        )
    });
    let mut table = Table::new().with_header(node(
        "header",
        TableRow::header(vec![
            node("header-name", TableCell::text("NAME")),
            node("header-value", TableCell::text("VALUE")),
        ]),
    ));
    table.set_rows(rows.collect());
    table
}

#[test]
fn table_selects_row_with_keyboard() {
    let mut tree = Tree::new(node("table", sample_services_table()));
    let area = Rect::new(0, 0, 60, 10);
    let text = render_text(&mut tree, area);
    assert!(text.contains("NAME"), "missing header: {text}");
    assert!(text.contains("service-0"), "missing row: {text}");
    focus(&mut tree, "table");
    tree.handle_event(&key(KeyCode::Down));
    assert_eq!(
        tree.find::<Table>(&id("table")).map(Table::selected),
        Some(1)
    );
    tree.handle_event(&key(KeyCode::PageDown));
    let selected = tree
        .find::<Table>(&id("table"))
        .map(Table::selected)
        .unwrap();
    assert!(selected >= 1);
}

#[test]
fn table_renders_cells_side_by_side() {
    let mut tree = Tree::new(node("table", sample_services_table()));
    let area = Rect::new(0, 0, 60, 10);
    let text = render_text(&mut tree, area);
    let header_line = text.lines().next().unwrap_or("");
    assert!(header_line.contains("NAME"), "missing NAME: {header_line}");
    assert!(
        header_line.contains("VALUE"),
        "missing VALUE: {header_line}"
    );
}

#[test]
fn dialog_confirm_button_drains_pressed() {
    let mut tree = Tree::new(node(
        "dialog",
        Dialog::with_default_actions()
            .with_title("Deploy")
            .with_message("Ship it?"),
    ));
    tree.find_mut::<Dialog>(&id("dialog"))
        .unwrap()
        .state
        .is_open = true;
    focus(&mut tree, "confirm");
    tree.handle_event(&key(KeyCode::Enter));
    let pressed = tree
        .find_mut::<Button>(&id("confirm"))
        .map(Button::take_pressed);
    assert_eq!(pressed, Some(true));
}

#[test]
fn dialog_closes_on_escape() {
    let mut tree = Tree::new(node("dialog", Dialog::with_default_actions()));
    let _ = tree.find_mut::<Dialog>(&id("dialog")).map(|dialog| {
        dialog.state.is_open = true;
    });
    tree.handle_event(&key(KeyCode::Esc));
    let open = tree
        .find::<Dialog>(&id("dialog"))
        .map(|dialog| dialog.state.is_open);
    assert_eq!(open, Some(false));
}

#[test]
fn grid_resizes_with_keyboard() {
    let area = Rect::new(0, 0, 40, 5);
    let mut tree = Tree::new(node("grid", ResizableGrid::row(vec![1, 1])));
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    focus(&mut tree, "grid");
    let before = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area))
        .unwrap();
    assert_eq!(before[0].width + before[1].width + 1, 40);
    tree.handle_event(&key(KeyCode::Right));
    let after = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area))
        .unwrap();
    assert_eq!(after[0].width, before[0].width + 1);
}

#[test]
fn grid_wheel_resizes_without_focus() {
    let area = Rect::new(0, 0, 40, 5);
    let mut tree = Tree::new(node("grid", ResizableGrid::row(vec![1, 1])));
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    let before = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area)[0].width)
        .unwrap();
    let divider = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.divider_positions(area)[0])
        .unwrap();
    tree.handle_event(&wheel(divider, 1, true));
    let after = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area)[0].width)
        .unwrap();
    assert!(after > before);
}

#[test]
fn grid_drag_resizes_pane() {
    let area = Rect::new(0, 0, 40, 5);
    let mut tree = Tree::new(node("grid", ResizableGrid::row(vec![1, 1])));
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    let before = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area)[0].width)
        .unwrap();
    let divider = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.divider_positions(area)[0])
        .unwrap();
    tree.handle_event(&click(divider, 2));
    tree.handle_event(&InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: divider + 3,
        row: 2,
        modifiers: KeyModifiers::NONE,
    }));
    let after = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area)[0].width)
        .unwrap();
    assert!(after >= before + 3);
}

fn sample_series(len: usize) -> Vec<Candle> {
    (0..len)
        .map(|index| {
            let wave = (index as f64 * 0.3).sin() * 3.0;
            Candle {
                open: 100.0 + wave,
                high: 100.0 + wave + 1.5,
                low: 100.0 + wave - 1.5,
                close: 100.0 + wave + (index % 3) as f64 - 1.0,
                volume: 1.0 + (index % 5) as f64 * 0.2,
            }
        })
        .collect()
}

#[test]
fn candlechart_renders_all_presentations() {
    for presentation in [
        RenderStyle::Native,
        RenderStyle::Halftone,
        RenderStyle::Ascii,
        RenderStyle::DenseAscii,
        RenderStyle::Numbers,
        RenderStyle::Binary,
        RenderStyle::Letters,
        RenderStyle::Blocks,
        RenderStyle::Squares,
        RenderStyle::Lines,
        RenderStyle::Dither,
    ] {
        let mut chart = CandleChart::new().with_presentation(presentation);
        chart.set_series(sample_series(60));
        let mut tree = Tree::new(node("chart", chart));
        render_tree(&mut tree);
    }
}

#[test]
fn candlechart_scrolls_history_with_keys() {
    let mut chart = CandleChart::new();
    chart.set_series(sample_series(60));
    let mut tree = Tree::new(node("chart", chart));
    focus(&mut tree, "chart");
    tree.handle_event(&key(KeyCode::Left));
    assert_eq!(
        tree.find::<CandleChart>(&id("chart"))
            .map(|chart| chart.state.scroll_back),
        Some(1)
    );
    // Pushing a new candle keeps the same candles on screen.
    let pushed = tree.find_mut::<CandleChart>(&id("chart")).unwrap();
    pushed.push_candle(Candle {
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        volume: 1.0,
    });
    assert_eq!(
        tree.find::<CandleChart>(&id("chart"))
            .map(|chart| chart.state.scroll_back),
        Some(2)
    );
    // set_series anchors back to the live edge.
    let series = sample_series(10);
    let chart = tree.find_mut::<CandleChart>(&id("chart")).unwrap();
    chart.set_series(series);
    assert_eq!(
        tree.find::<CandleChart>(&id("chart"))
            .map(|chart| chart.state.scroll_back),
        Some(0)
    );
}

#[test]
fn form_validates_required_fields_and_submits() {
    let mut tree = Tree::new(node(
        "form",
        Form::new(vec![
            node("name", FormField::text("name", "Name").required()),
            node(
                "email",
                FormField::text("email", "Email").placeholder("you@example.com"),
            ),
        ]),
    ));
    focus(&mut tree, "name:input");
    // Enter validates before any text is present: no submission.
    tree.handle_event(&key(KeyCode::Enter));
    let submitted = tree
        .find_mut::<Form>(&id("form"))
        .and_then(Form::take_submitted);
    assert_eq!(submitted, None);
    type_text(&mut tree, "Ada");
    tree.handle_event(&key(KeyCode::Enter));
    let submitted = tree
        .find_mut::<Form>(&id("form"))
        .and_then(Form::take_submitted);
    assert_eq!(
        submitted,
        Some(vec![
            ("name".to_string(), "Ada".to_string()),
            ("email".to_string(), String::new())
        ])
    );
}

#[test]
fn form_shows_custom_error_message() {
    let mut tree = Tree::new(node(
        "form",
        Form::new(vec![node(
            "name",
            FormField::text("name", "Name").required_with_message("obligatorio"),
        )]),
    ));
    focus(&mut tree, "name:input");
    tree.handle_event(&key(KeyCode::Enter));
    let area = Rect::new(0, 0, 40, 6);
    let text = render_text(&mut tree, area);
    assert!(text.contains("obligatorio"), "missing error row: {text}");
}

#[test]
fn list_marks_selection() {
    let mut tree = Tree::new(node(
        "list",
        List::with_items(vec!["one", "two", "three"]).with_marker(Marker::Decimal),
    ));
    focus(&mut tree, "list");
    tree.handle_event(&key(KeyCode::Down));
    assert_eq!(tree.find::<List>(&id("list")).map(List::selected), Some(1));
    let area = Rect::new(0, 0, 40, 10);
    let text = render_text(&mut tree, area);
    assert!(text.contains("1."), "missing decimal marker: {text}");
}

#[test]
fn expandable_toggles_on_enter() {
    let mut tree = Tree::new(node(
        "expandable",
        Expandable::with_lines("Title", vec!["body line"]),
    ));
    focus(&mut tree, "expandable");
    let area = Rect::new(0, 0, 40, 10);
    let collapsed = render_text(&mut tree, area);
    assert!(!collapsed.contains("body line"));
    tree.press(&id("expandable"));
    let expanded = render_text(&mut tree, area);
    assert!(
        expanded.contains("body line"),
        "missing body row: {expanded}"
    );
}

#[test]
fn spinner_advances_frame_on_tick() {
    let mut tree = Tree::new(node("spinner", Spinner::new().with_label("working")));
    let area = Rect::new(0, 0, 40, 3);
    let theme = theme();
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    tree.tick(std::time::Duration::from_millis(200));
    let after = tree
        .find::<Spinner>(&id("spinner"))
        .map(|spinner| spinner.state.frame);
    assert!(matches!(after, Some(frame) if frame > 0));
}

#[test]
fn progress_eases_toward_target_on_tick() {
    let mut tree = Tree::new(node("progress", Progress::new(0.2)));
    tree.tick(std::time::Duration::from_millis(50));
    let eased = tree.find::<Progress>(&id("progress")).unwrap();
    assert!(eased.state.value > 0.0);
}

#[test]
fn toast_expires_after_ttl() {
    let mut tree = Tree::new(node(
        "toast",
        Toast::new(ToastKind::Info, "hello").with_ttl(std::time::Duration::from_secs(1)),
    ));
    tree.tick(std::time::Duration::from_secs(2));
    let expired = tree.find::<Toast>(&id("toast")).map(Toast::expired);
    assert_eq!(expired, Some(true));
}

#[test]
fn tooltip_renders_and_hides() {
    let mut tree = Tree::new(node("tooltip", Tooltip::new("hint", 2, 4)));
    let area = Rect::new(0, 0, 40, 10);
    let shown = render_text(&mut tree, area);
    assert!(shown.contains("hint"), "missing tooltip content: {shown}");
    tree.find_mut::<Tooltip>(&id("tooltip")).unwrap().hide();
    let hidden = render_text(&mut tree, area);
    assert!(!hidden.contains("hint"));
}

#[test]
fn outline_reports_all_nodes() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("title", Text::new("Hello")),
            node("button", Button::new(ButtonVariant::Primary, "OK")),
        ]),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 40, 4);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let outline = tree.inspect_outline();
    assert!(outline.contains("title [text]"));
    assert!(outline.contains("button [button]"));
}

#[test]
fn tree_cycles_focus_in_preorder() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("first", Button::new(ButtonVariant::Primary, "one")),
            node("second", Button::new(ButtonVariant::Secondary, "two")),
        ]),
    ));
    assert_eq!(tree.focused_id(), None);
    tree.handle_event(&key(KeyCode::Tab));
    assert_eq!(tree.focused_id(), Some("first"));
    tree.handle_event(&key(KeyCode::Tab));
    assert_eq!(tree.focused_id(), Some("second"));
    tree.handle_event(&key(KeyCode::Tab));
    assert_eq!(tree.focused_id(), Some("first"));
    tree.handle_event(&key(KeyCode::BackTab));
    assert_eq!(tree.focused_id(), Some("second"));
}

#[test]
fn every_component_renders_at_every_size() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("badge", Badge::new("b")),
            node("button", Button::new(ButtonVariant::Primary, "go")),
            node("spinner", Spinner::new().with_label("working")),
            node("progress", Progress::new(0.5).with_label("halfway")),
            node("list", List::with_items(vec!["one", "two"])),
            node(
                "radio",
                terminalui_components::radio::RadioGroup::with_options(vec!["a", "b"]),
            ),
            node(
                "switch",
                terminalui_components::switch::Switch::with_label("on"),
            ),
            node(
                "tree",
                terminalui_components::treeview::TreeView::new(vec![
                    terminalui_components::treeview::TreeNode::branch(
                        "parent",
                        vec![terminalui_components::treeview::TreeNode::leaf("child")],
                    )
                    .with_expanded(true),
                ]),
            ),
            node(
                "linechart",
                terminalui_components::linechart::LineChart::new()
                    .with_children(vec![node("caption", Text::new("p50"))]),
            ),
            node(
                "barchart",
                terminalui_components::barchart::BarChart::new().with_values(vec![1.0, 2.0]),
            ),
            node(
                "sparkline",
                terminalui_components::sparkline::Sparkline::new(vec![1.0, 3.0, 2.0]),
            ),
            node(
                "stat",
                terminalui_components::stat::Stat::new("services", "12")
                    .with_delta("+2", terminalui_components::stat::StatTone::Positive),
            ),
            node(
                "dropdown",
                terminalui_components::dropdown::DropdownMenu::new(vec![node(
                    "trigger",
                    Text::new("open menu"),
                )])
                .with_items(vec![
                    terminalui_components::menu::MenuEntry::item("first"),
                    terminalui_components::menu::MenuEntry::separator(),
                    terminalui_components::menu::MenuEntry::item("second"),
                ]),
            ),
            node(
                "contextmenu",
                terminalui_components::contextmenu::ContextMenu::new(vec![
                    terminalui_components::menu::MenuEntry::item("open"),
                ]),
            ),
            node(
                "palette",
                terminalui_components::commandpalette::CommandPalette::new(vec![
                    terminalui_components::commandpalette::Command::new("run"),
                ]),
            ),
            node(
                "autocomplete",
                terminalui_components::autocomplete::Autocomplete::new(vec!["alpha".to_string()]),
            ),
            node("kbd", terminalui_components::kbd::Kbd::new("Enter")),
            node(
                "stepper",
                terminalui_components::stepper::Stepper::new(vec![
                    terminalui_components::stepper::Step::new("plan")
                        .with_status(terminalui_components::stepper::StepStatus::Done),
                    terminalui_components::stepper::Step::new("build")
                        .with_status(terminalui_components::stepper::StepStatus::Active),
                    terminalui_components::stepper::Step::new("ship"),
                ]),
            ),
            node(
                "statusbar",
                terminalui_components::statusbar::StatusBar::new()
                    .left(vec![node("bar-left", Text::new("main"))])
                    .right(vec![node(
                        "bar-kbd",
                        terminalui_components::kbd::Kbd::new("q"),
                    )]),
            ),
            node(
                "popover",
                terminalui_components::popover::Popover::new("tip", 2, 2),
            ),
            node(
                "gauge",
                terminalui_components::gauge::Gauge::new(0.6)
                    .with_children(vec![node("caption", Text::new("cpu"))]),
            ),
        ]),
    ));
    let theme = theme();
    for (width, height) in every_size() {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        tree.render(area, &mut buffer, &theme);
        tree.tick(std::time::Duration::from_millis(16));
    }
}

#[test]
fn candlechart_renders_with_caption_at_minimal_heights() {
    for height in [6u16, 7, 8, 9, 10, 12] {
        let chart = CandleChart::new()
            .with_children(vec![terminalui_core::node("symbol", Text::new("KVX/USD"))]);
        let mut tree = Tree::new(terminalui_core::node("chart", chart));
        let theme = theme();
        let area = Rect::new(0, 0, 40, height);
        let mut buffer = Buffer::empty(area);
        let id = &"chart".to_string();
        tree.find_mut::<CandleChart>(id)
            .unwrap()
            .set_series(sample_series(60));
        tree.render(area, &mut buffer, &theme);
    }
}

#[test]
fn candlechart_wheel_scrolls_only_the_chart_under_the_mouse() {
    let left = CandleChart::new();
    let right = CandleChart::new();
    let mut tree = Tree::new(terminalui_core::node(
        "root",
        terminalui_core::Stack::horizontal().children(vec![
            terminalui_core::node("left", left),
            terminalui_core::node("right", right),
        ]),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 80, 8);
    let mut buffer = Buffer::empty(area);
    for (id, series) in [
        (&"left".to_string(), sample_series(60)),
        (&"right".to_string(), sample_series(60)),
    ] {
        tree.find_mut::<CandleChart>(id).unwrap().set_series(series);
    }
    tree.render(area, &mut buffer, &theme);
    let wheel = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 1,
        row: 4,
        modifiers: KeyModifiers::NONE,
    });
    tree.handle_event(&wheel);
    let left_back = tree.find::<CandleChart>("left").unwrap().state.scroll_back;
    let right_back = tree.find::<CandleChart>("right").unwrap().state.scroll_back;
    assert_eq!(left_back, 3);
    assert_eq!(right_back, 0);
}

#[test]
fn candlechart_wheel_on_price_axis_zooms_range_not_scroll() {
    let mut tree = Tree::new(terminalui_core::node("chart", CandleChart::new()));
    let theme = theme();
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    let id = &"chart".to_string();
    tree.find_mut::<CandleChart>(id)
        .unwrap()
        .set_series(sample_series(60));
    tree.render(area, &mut buffer, &theme);
    let wheel = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 35,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    tree.handle_event(&wheel);
    let zoom = tree.find::<CandleChart>(id).unwrap().state.y_zoom;
    assert_eq!(zoom, 1.25);
    let scroll_back = tree.find::<CandleChart>(id).unwrap().state.scroll_back;
    assert_eq!(scroll_back, 0);
}

#[test]
fn candlechart_wheel_on_time_axis_zooms_candle_width() {
    let mut tree = Tree::new(terminalui_core::node("chart", CandleChart::new()));
    let theme = theme();
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    let id = &"chart".to_string();
    tree.find_mut::<CandleChart>(id)
        .unwrap()
        .set_series(sample_series(60));
    tree.render(area, &mut buffer, &theme);
    let wheel = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 10,
        row: 9,
        modifiers: KeyModifiers::NONE,
    });
    tree.handle_event(&wheel);
    let zoom = tree.find::<CandleChart>(id).unwrap().state.x_zoom;
    assert_eq!(zoom, 0.8);
    let scroll_back = tree.find::<CandleChart>(id).unwrap().state.scroll_back;
    assert_eq!(scroll_back, 0);
}

#[test]
fn grid_resize_divider_programmatically() {
    let area = Rect::new(0, 0, 40, 5);
    let mut tree = Tree::new(terminalui_core::node(
        "grid",
        ResizableGrid::row(vec![1, 1]),
    ));
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    let before = tree
        .find::<ResizableGrid>(&id("grid"))
        .map(|grid| grid.pane_areas(area))
        .unwrap();
    let grid = tree.find_mut::<ResizableGrid>(&id("grid")).unwrap();
    assert!(grid.resize_divider(0, 5));
    let after = grid.pane_areas(area);
    assert_eq!(after[0].width, before[0].width + 5);
}

#[test]
fn grid_set_divider_size_clamps_to_minimum() {
    let area = Rect::new(0, 0, 40, 5);
    let mut tree = Tree::new(terminalui_core::node(
        "grid",
        ResizableGrid::row(vec![1, 1]),
    ));
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    let grid = tree.find_mut::<ResizableGrid>(&id("grid")).unwrap();
    assert!(grid.set_divider_size(0, 10));
    let panes = grid.pane_areas(area);
    assert_eq!(panes[0].width, 10);
    assert!(grid.set_divider_size(0, 0));
    let panes = grid.pane_areas(area);
    assert_eq!(panes[0].width, 3);
}

#[test]
fn grid_set_weights_rescales_panes() {
    let area = Rect::new(0, 0, 40, 5);
    let mut tree = Tree::new(terminalui_core::node(
        "grid",
        ResizableGrid::row(vec![1, 3]),
    ));
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    tree.render(area, &mut buffer, &theme);
    let grid = tree.find_mut::<ResizableGrid>(&id("grid")).unwrap();
    grid.set_weights(vec![3, 1]);
    let panes = grid.pane_areas(area);
    assert_eq!(panes[0].width, 30);
}

#[test]
fn table_overflow_draws_scrollbar_on_right_edge() {
    let mut tree = Tree::new(node("table", sample_services_table()));
    let area = Rect::new(0, 0, 60, 8);
    let text = render_text(&mut tree, area);
    let last_column: String = text
        .lines()
        .filter_map(|line| line.chars().nth(59))
        .collect();
    assert!(
        last_column.contains('█') || last_column.contains('░'),
        "expected a scrollbar on the right edge: {last_column}"
    );
}

#[test]
fn table_without_overflow_hides_scrollbar() {
    let rows = (0..2).map(|index| {
        node(
            format!("row-{index}"),
            TableRow::body(vec![node(
                format!("cell-{index}"),
                TableCell::text(format!("service-{index}")),
            )]),
        )
    });
    let mut table = Table::new();
    table.set_rows(rows.collect());
    let mut tree = Tree::new(node("table", table));
    let text = render_text(&mut tree, Rect::new(0, 0, 40, 20));
    let last_column: String = text
        .lines()
        .filter_map(|line| line.chars().nth(39))
        .collect();
    assert!(
        !last_column.contains('█') && !last_column.contains('░'),
        "scrollbar should auto-hide: {last_column}"
    );
}

#[test]
fn list_overflow_draws_scrollbar_on_right_edge() {
    let mut tree = Tree::new(node(
        "list",
        List::with_items(
            (0..30)
                .map(|index| format!("item-{index}"))
                .collect::<Vec<_>>(),
        ),
    ));
    let area = Rect::new(0, 0, 40, 6);
    let text = render_text(&mut tree, area);
    let last_column: String = text
        .lines()
        .filter_map(|line| line.chars().nth(39))
        .collect();
    assert!(
        last_column.contains('█') || last_column.contains('░'),
        "expected a scrollbar on the right edge: {last_column}"
    );
}

#[test]
fn separator_resolves_hairline_from_presentation() {
    let mut tree = Tree::new(node(
        "separator",
        terminalui_components::separator::Separator::new().with_presentation(RenderStyle::Ascii),
    ));
    let text = render_text(&mut tree, Rect::new(0, 0, 20, 1));
    assert_eq!(text.trim(), "-".repeat(20));
}

#[test]
fn panel_line_border_uses_rounded_or_plain_glyphs() {
    let mut tree = Tree::new(node(
        "panel",
        terminalui_components::panel::Panel::line().children(vec![node("label", Text::new("hi"))]),
    ));
    let text = render_text(&mut tree, Rect::new(0, 0, 20, 4));
    assert!(text.contains('┌'), "missing corner: {text}");
    assert!(text.contains('┘'), "missing corner: {text}");
}

#[test]
fn grid_divider_resolves_hairline_from_presentation() {
    let grid = ResizableGrid::row(vec![1, 1]).with_style(terminalui_components::grid::GridStyle {
        presentation: Some(RenderStyle::Ascii),
        ..Default::default()
    });
    let mut tree = Tree::new(node("grid", grid));
    let text = render_text(&mut tree, Rect::new(0, 0, 20, 4));
    assert!(text.contains('|'), "ascii divider missing: {text}");
    assert!(!text.contains('│'), "native divider leaked: {text}");
}

#[test]
fn tabs_separator_resolves_hairline_from_presentation() {
    let mut tabs = Tabs::with_titles(vec!["one", "two"]);
    let style = terminalui_components::tabs::TabsStyle {
        presentation: Some(RenderStyle::Ascii),
        ..Default::default()
    };
    tabs = tabs.with_style(style);
    let mut tree = Tree::new(node("tabs", tabs));
    let text = render_text(&mut tree, Rect::new(0, 0, 20, 2));
    assert!(text.contains('-'), "ascii separator missing: {text}");
    assert!(!text.contains('─'), "native separator leaked: {text}");
}

#[test]
fn dashed_chrome_border_uses_dashed_glyphs() {
    let chrome = terminalui_components::chrome::Chrome::bordered(
        terminalui_components::chrome::BorderSet::Dashed,
        None,
    );
    let panel = terminalui_components::panel::Panel::new()
        .with_chrome(chrome)
        .children(vec![node("label", Text::new("hi"))]);
    let mut tree = Tree::new(node("panel", panel));
    let text = render_text(&mut tree, Rect::new(0, 0, 20, 3));
    assert!(text.contains('╌'), "dashed horizontal missing: {text}");
    assert!(text.contains('╎'), "dashed vertical missing: {text}");
}

#[test]
fn input_multiline_accepts_typed_text() {
    let input = terminalui_components::input::Input::new().multiline(true);
    let mut tree = Tree::new(node("input", input));
    focus(&mut tree, "input");
    for character in "alpha bravo".chars() {
        tree.handle_event(&key(KeyCode::Char(character)));
    }
    tree.handle_event(&key(KeyCode::Enter));
    for character in "juliett kilo".chars() {
        tree.handle_event(&key(KeyCode::Char(character)));
    }
    let value = tree
        .find::<terminalui_components::input::Input>(&id("input"))
        .map(|input| input.state.value.clone());
    assert_eq!(value.as_deref(), Some("alpha bravo\njuliett kilo"));
}

#[test]
fn multiline_input_receives_keys_after_many_focus_siblings() {
    use terminalui_components::breadcrumb::Breadcrumb;
    use terminalui_components::checkbox::Checkbox;
    use terminalui_core::{Stack, Text};

    let buttons = Stack::horizontal().gap(1).children(vec![
        node("primary", Button::new(ButtonVariant::Primary, "Primary")),
        node(
            "secondary",
            Button::new(ButtonVariant::Secondary, "Secondary"),
        ),
        node(
            "destructive",
            Button::new(ButtonVariant::Destructive, "Del"),
        ),
        node("ghost", Button::new(ButtonVariant::Ghost, "Ghost")),
        node("outline", Button::new(ButtonVariant::Outline, "Outline")),
        node("loading", Button::new(ButtonVariant::Primary, "Loading")),
    ]);
    let root = Stack::vertical().children(vec![
        node(
            "breadcrumb",
            Breadcrumb::with_segments(vec!["acme", "deploys", "catalog"]),
        ),
        node("buttons", buttons),
        node("checkbox-on", Checkbox::with_label("On").with_checked(true)),
        node("checkbox-off", Checkbox::with_label("Off")),
        node(
            "multiline",
            terminalui_components::input::Input::new().multiline(true),
        ),
        node("after", Text::new("tail")),
    ]);
    let mut tree = Tree::new(node("root", root));
    assert!(tree.focus(&id("multiline")));
    for character in "z".chars() {
        tree.handle_event(&key(KeyCode::Char(character)));
    }
    let value = tree
        .find::<terminalui_components::input::Input>(&id("multiline"))
        .map(|input| input.state.value.clone());
    assert_eq!(value.as_deref(), Some("z"));
}

#[test]
fn disabled_button_does_not_consume_focus_rank() {
    let mut tree = Tree::new(node(
        "root",
        Stack::horizontal().children(vec![
            node("disabled", Button::new(ButtonVariant::Secondary, "Nope")),
            node("enabled", Button::new(ButtonVariant::Primary, "Yes")),
        ]),
    ));
    tree.find_mut::<Button>(&id("disabled"))
        .unwrap()
        .set_disabled(true);
    assert!(tree.focus(&id("enabled")));
    tree.handle_event(&key(KeyCode::Enter));
    let pressed = tree
        .find_mut::<Button>(&id("enabled"))
        .map(Button::take_pressed);
    assert_eq!(pressed, Some(true));
}

#[test]
fn outline_variant_draws_border_when_idle() {
    let mut tree = Tree::new(node(
        "outline",
        Button::new(ButtonVariant::Outline, "Outline"),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 30, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let text = buffer_text(&buffer, area);
    assert!(text.contains('│'), "idle outline border missing: {text}");
}

#[test]
fn disabled_filled_button_uses_muted_fill() {
    let mut button = Button::new(ButtonVariant::Primary, "Nope");
    button.set_disabled(true);
    let mut tree = Tree::new(node("button", button));
    let theme = theme();
    let area = Rect::new(0, 0, 20, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let muted_cells = (0..area.width)
        .filter(|x| buffer[(*x, 0)].bg == theme.muted)
        .count();
    assert!(muted_cells > 0, "disabled fill should be theme.muted");
}

#[test]
fn held_mouse_button_emphasizes_without_focus() {
    let mut tree = Tree::new(node("primary", Button::new(ButtonVariant::Primary, "Go")));
    let theme = theme();
    let area = Rect::new(0, 0, 20, 1);
    tree.render(area, &mut Buffer::empty(area), &theme);
    let idle_buffer = {
        let mut buffer = Buffer::empty(area);
        tree.render(area, &mut buffer, &theme);
        buffer
    };
    let idle_cell = idle_buffer[(10, 0)].clone();
    tree.handle_event(&InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }));
    let held_buffer = {
        let mut buffer = Buffer::empty(area);
        tree.render(area, &mut buffer, &theme);
        buffer
    };
    let held_cell = held_buffer[(10, 0)].clone();
    assert_ne!(
        idle_cell.bg, held_cell.bg,
        "held button should paint a different fill"
    );
    tree.handle_event(&InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 10,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }));
    let released_buffer = {
        let mut buffer = Buffer::empty(area);
        tree.render(area, &mut buffer, &theme);
        buffer
    };
    assert_eq!(
        released_buffer[(10, 0)].bg,
        idle_cell.bg,
        "released button should repaint idle fill"
    );
}

#[test]
fn loading_button_is_not_focusable() {
    let mut button = Button::new(ButtonVariant::Primary, "Load");
    button.set_loading(true);
    let mut tree = Tree::new(node("button", button));
    assert!(
        !tree.focus(&id("button")),
        "loading button should not be focusable"
    );
}

#[test]
fn disabled_button_ignores_enter() {
    let mut button = Button::new(ButtonVariant::Primary, "Nope");
    button.set_disabled(true);
    let mut tree = Tree::new(node("button", button));
    tree.handle_event(&key(KeyCode::Enter));
    let pressed = tree
        .find_mut::<Button>(&id("button"))
        .map(Button::take_pressed);
    assert_eq!(pressed, Some(false));
}

#[test]
fn chart_draws_history_scrollbar_when_configured() {
    let series: Vec<Candle> = (0..120)
        .map(|index| Candle {
            open: 100.0 + index as f64,
            high: 112.0 + index as f64,
            low: 96.0 + index as f64,
            close: 104.0 + index as f64,
            volume: (index % 3) as f64,
        })
        .collect();
    let mut chart =
        CandleChart::new().with_style(terminalui_components::candlechart::CandleChartStyle {
            scrollbar: Some(terminalui_components::scroll::ScrollbarStyle {
                visible: terminalui_components::scroll::ScrollbarVisibility::Always,
                ..Default::default()
            }),
            ..Default::default()
        });
    chart.set_series(series);
    let mut tree = Tree::new(node("chart", chart));
    let area = Rect::new(0, 0, 80, 16);
    let text = render_text(&mut tree, area);
    let last_column: String = text
        .lines()
        .filter_map(|line| line.chars().nth(79))
        .collect();
    assert!(
        last_column.contains('█') || last_column.contains('░') || last_column.contains('●'),
        "expected a history scrollbar on the right edge: {last_column}"
    );
}

#[test]
fn chart_auto_scrollbar_hides_when_all_candles_fit() {
    let series: Vec<Candle> = (0..3)
        .map(|index| Candle {
            open: 100.0,
            high: 112.0,
            low: 96.0,
            close: 104.0 + index as f64,
            volume: 1.0,
        })
        .collect();
    let mut chart =
        CandleChart::new().with_style(terminalui_components::candlechart::CandleChartStyle {
            scrollbar: Some(terminalui_components::scroll::ScrollbarStyle::default()),
            ..Default::default()
        });
    chart.set_series(series);
    let mut tree = Tree::new(node("chart", chart));
    let area = Rect::new(0, 0, 80, 16);
    let text = render_text(&mut tree, area);
    let last_column: String = text
        .lines()
        .filter_map(|line| line.chars().nth(79))
        .collect();
    assert!(
        !last_column.contains('█') && !last_column.contains('░') && !last_column.contains('●'),
        "scrollbar should auto-hide when all candles fit: {last_column}"
    );
}

#[test]
fn halftone_dots_plot_values_across_density() {
    let tuning = terminalui_components::presentation::StyleTuning {
        density: 2,
        contrast: 1.0,
        dotsize: terminalui_components::presentation::DotSize::Medium,
    };
    let style = RenderStyle::Halftone;
    assert_eq!(style.shade(0.0, &tuning), " ");
    assert_eq!(style.shade(1.0, &tuning), "●");
    assert_eq!(style.shade(0.49, &tuning), " ");
    assert_eq!(style.shade(0.5, &tuning), "●");
}

#[test]
fn halftone_density_two_picks_ends_of_the_large_ramp() {
    let tuning = terminalui_components::presentation::StyleTuning {
        density: 2,
        contrast: 1.0,
        dotsize: terminalui_components::presentation::DotSize::Large,
    };
    let style = RenderStyle::Halftone;
    assert_eq!(style.shade(1.0, &tuning), "◉");
    assert_eq!(style.shade(0.0, &tuning), " ");
}

#[test]
fn halftone_higher_density_lights_weaker_cells() {
    // Density is an ink-coverage knob: at density 6 a weak cell lights up
    // where at density 2 it stays dark.
    let style = RenderStyle::Halftone;
    let sparse = terminalui_components::presentation::StyleTuning {
        density: 2,
        contrast: 1.0,
        dotsize: terminalui_components::presentation::DotSize::Medium,
    };
    let dense = terminalui_components::presentation::StyleTuning {
        density: 6,
        ..sparse
    };
    assert_eq!(style.shade(0.15, &sparse), " ");
    assert_ne!(style.shade(0.15, &dense), " ");
    assert_eq!(style.shade(0.0, &dense), " ");
    assert_eq!(style.shade(1.0, &dense), "●");
}

#[test]
fn contrast_above_one_leans_more_ink() {
    let style = RenderStyle::Halftone;
    let base = terminalui_components::presentation::StyleTuning::default();
    let heavy = terminalui_components::presentation::StyleTuning {
        contrast: 3.0,
        ..base
    };
    let sparse = terminalui_components::presentation::StyleTuning {
        contrast: 0.5,
        ..base
    };
    let low_ink = |tuning| style.shade(0.25, &tuning);
    // A 4-level ramp at low intensity must lean toward sparser glyphs as
    // contrast drops.
    assert_eq!(low_ink(sparse), " ");
    assert!(low_ink(heavy) != " ");
}

#[test]
fn squares_picks_square_cap_glyph() {
    let tuning = terminalui_components::presentation::StyleTuning {
        density: 4,
        contrast: 1.0,
        dotsize: terminalui_components::presentation::DotSize::Small,
    };
    assert_eq!(RenderStyle::Squares.shade(1.0, &tuning), "▪");
    let large = terminalui_components::presentation::StyleTuning {
        dotsize: terminalui_components::presentation::DotSize::Large,
        ..tuning
    };
    assert_eq!(RenderStyle::Squares.shade(1.0, &large), "■");
}

#[test]
fn native_shading_is_filled_or_space() {
    let tuning = terminalui_components::presentation::StyleTuning::default();
    assert_eq!(
        RenderStyle::Native.shade(1.0, &tuning),
        RenderStyle::Native.filled()
    );
    assert_eq!(RenderStyle::Native.shade(0.0, &tuning), " ");
}

#[test]
fn native_shading_is_unaffected_by_contrast() {
    let tuning = terminalui_components::presentation::StyleTuning {
        contrast: 3.0,
        ..Default::default()
    };
    assert_eq!(
        RenderStyle::Native.shade(1.0, &tuning),
        RenderStyle::Native.filled()
    );
}

#[test]
fn intensity_above_one_is_clamped() {
    let tuning = terminalui_components::presentation::StyleTuning::default();
    assert_eq!(
        RenderStyle::Halftone.shade(5.0, &tuning),
        RenderStyle::Halftone.shade(1.0, &tuning)
    );
}

#[test]
fn halftone_progress_bar_paints_dotted_fill() {
    let bar = Progress::new(0.62)
        .with_presentation(RenderStyle::Halftone)
        .with_tuning(terminalui_components::presentation::StyleTuning {
            density: 6,
            contrast: 1.0,
            dotsize: terminalui_components::presentation::DotSize::Medium,
        });
    let mut tree = Tree::new(node("progress", bar));
    let theme = theme();
    let area = Rect::new(0, 0, 40, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let text = buffer_text(&buffer, area);
    assert!(
        text.contains('●') || text.contains('•') || text.contains('·'),
        "expected dotted fill, not block fill: {text}"
    );
    assert!(
        !text.contains(RenderStyle::Halftone.filled()),
        "a halftone presentation must not paint the native filled glyph: {text}"
    );
}

#[test]
fn native_progress_bar_keeps_block_fill() {
    let mut tree = Tree::new(node("progress", Progress::new(0.6)));
    let theme = theme();
    let area = Rect::new(0, 0, 40, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let text = buffer_text(&buffer, area);
    assert!(
        text.contains(RenderStyle::Native.filled()),
        "native fill glyph missing: {text}"
    );
}

#[test]
fn chart_volume_bars_shade_with_halftone() {
    let series: Vec<Candle> = (0..60)
        .map(|index| Candle {
            open: 100.0,
            high: 103.0,
            low: 97.0,
            close: 102.0,
            volume: if index % 7 == 0 { 2.0 } else { 0.2 },
        })
        .collect();
    let mut chart = CandleChart::new()
        .with_presentation(RenderStyle::Halftone)
        .with_tuning(terminalui_components::presentation::StyleTuning {
            density: 5,
            contrast: 1.0,
            dotsize: terminalui_components::presentation::DotSize::Medium,
        });
    chart.set_series(series);
    let mut tree = Tree::new(node("chart", chart));
    let theme = theme();
    let area = Rect::new(0, 0, 80, 16);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let text = buffer_text(&buffer, area);
    assert!(
        text.contains('●') || text.contains('•'),
        "volume bars should be drawn with dot glyphs: {text}"
    );
}

#[test]
fn slider_renders_thumb_and_fill() {
    let mut tree = Tree::new(node(
        "slider",
        terminalui_components::slider::Slider::new(0.0, 100.0, 1.0).with_value(50.0),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 20, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let text = buffer_text(&buffer, area);
    assert!(text.contains('●'), "thumb missing: {text}");
    assert!(text.contains('█'), "filled track missing: {text}");
    assert!(text.contains('░'), "empty track missing: {text}");
}

#[test]
fn slider_snaps_to_step_and_clamps() {
    let mut slider = terminalui_components::slider::Slider::new(0.0, 100.0, 10.0);
    slider.set_value(37.0);
    assert_eq!(slider.value(), 40.0);
    slider.set_value(-50.0);
    assert_eq!(slider.value(), 0.0);
    slider.set_value(5000.0);
    assert_eq!(slider.value(), 100.0);
}

#[test]
fn slider_keys_move_value_by_step() {
    let mut tree = Tree::new(node(
        "slider",
        terminalui_components::slider::Slider::new(0.0, 100.0, 10.0).with_value(50.0),
    ));
    focus(&mut tree, "slider");
    tree.handle_event(&key(KeyCode::Left));
    assert_eq!(
        tree.find::<terminalui_components::slider::Slider>(&id("slider"))
            .map(terminalui_components::slider::Slider::value),
        Some(40.0)
    );
    tree.handle_event(&key(KeyCode::End));
    assert_eq!(
        tree.find::<terminalui_components::slider::Slider>(&id("slider"))
            .map(terminalui_components::slider::Slider::value),
        Some(100.0)
    );
    tree.handle_event(&key(KeyCode::Home));
    assert_eq!(
        tree.find::<terminalui_components::slider::Slider>(&id("slider"))
            .map(terminalui_components::slider::Slider::value),
        Some(0.0)
    );
}

#[test]
fn slider_mouse_click_sets_value() {
    let mut tree = Tree::new(node(
        "slider",
        terminalui_components::slider::Slider::new(0.0, 100.0, 1.0),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 20, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    tree.handle_event(&click(19, 0));
    assert_eq!(
        tree.find::<terminalui_components::slider::Slider>(&id("slider"))
            .map(terminalui_components::slider::Slider::value),
        Some(100.0)
    );
    tree.handle_event(&click(0, 0));
    assert_eq!(
        tree.find::<terminalui_components::slider::Slider>(&id("slider"))
            .map(terminalui_components::slider::Slider::value),
        Some(0.0)
    );
}

#[test]
fn slider_disabled_ignores_input() {
    let mut slider = terminalui_components::slider::Slider::new(0.0, 100.0, 10.0);
    slider.set_disabled(true);
    let mut tree = Tree::new(node("slider", slider));
    assert!(
        !tree.focus(&id("slider")),
        "disabled slider should not be focusable"
    );
    tree.handle_event(&click(10, 0));
    let value = tree
        .find::<terminalui_components::slider::Slider>(&id("slider"))
        .map(terminalui_components::slider::Slider::value);
    assert_eq!(value, Some(0.0));
}

#[test]
fn slider_halftone_presentation_uses_dot_ramp() {
    let mut tree = Tree::new(node(
        "slider",
        terminalui_components::slider::Slider::new(0.0, 10.0, 1.0)
            .with_value(5.0)
            .with_presentation(RenderStyle::Halftone)
            .with_tuning(terminalui_components::presentation::StyleTuning {
                density: 5,
                contrast: 1.0,
                dotsize: terminalui_components::presentation::DotSize::Medium,
            }),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 20, 1);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let text = buffer_text(&buffer, area);
    assert!(
        text.contains('●'),
        "halftone track should use dot glyphs: {text}"
    );
    assert!(
        !text.contains('█'),
        "halftone track must not use the native block fill: {text}"
    );
}

#[test]
fn select_enter_selects_highlighted_option() {
    let options = vec![
        node("opt-a", Text::new("alpha")),
        node("opt-b", Text::new("beta")),
        node("opt-c", Text::new("gamma")),
    ];
    let tree_node = node(
        "select",
        terminalui_components::select::Select::new(options).with_selected(1),
    );
    let mut tree = Tree::new(tree_node);
    assert!(tree.focus(&id("select")), "select must be focusable");
    tree.handle_event(&key(KeyCode::Down));
    tree.handle_event(&key(KeyCode::Down));
    tree.handle_event(&key(KeyCode::Enter));
    let selected = tree
        .find::<terminalui_components::select::Select>(&id("select"))
        .and_then(terminalui_components::select::Select::selected);
    assert_eq!(selected, Some(2));
}

#[test]
fn json_reports_agent_meaningful_state() {
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("tabs", {
                let mut tabs = Tabs::with_titles(vec!["one", "two"]);
                tabs.set_selected(1);
                tabs
            }),
            node("badge", Badge::new("hot")),
            node("crumb", Breadcrumb::with_segments(vec!["a", "b", "c"])),
            node("pagination", Pagination::with_page(9, 3)),
            node("table", {
                let rows = (0..3).map(|index| {
                    node(
                        format!("row-{index}"),
                        TableRow::body(vec![node(
                            format!("cell-{index}"),
                            TableCell::text(format!("row-{index}")),
                        )]),
                    )
                });
                let mut table = Table::new();
                table.set_rows(rows.collect());
                table
            }),
        ]),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let value = json(&tree);
    let root = &value["root"];
    let snap = |id: &str| find(root, id).cloned();
    assert_eq!(
        snap("tabs").map(|node| node["view"]["state"]["selected"].clone()),
        Some(serde_json::json!(1))
    );
    assert_eq!(
        snap("badge").map(|node| node["kind"].clone()),
        Some(serde_json::json!("badge"))
    );
    assert_eq!(
        snap("crumb").map(|node| node["view"]["state"]["highlight"].clone()),
        Some(serde_json::json!(2))
    );
    let pagination = snap("pagination").expect("pagination node");
    assert_eq!(pagination["view"]["state"]["page"], 3);
    assert_eq!(pagination["view"]["total_pages"], 9);
    let table = snap("table").expect("table node");
    assert_eq!(table["view"]["state"]["selected"], 0);
    assert_eq!(table["view"]["children"].as_array().map(Vec::len), Some(3));
}

#[test]
fn json_covers_every_component_with_its_real_kind() {
    use terminalui_components::candlechart::{Candle, CandleChart};
    use terminalui_components::form::Form;
    use terminalui_components::panel::Panel;
    use terminalui_components::separator::Separator;

    let mut chart = CandleChart::new();
    chart.set_series(
        (1..10)
            .map(|index| Candle {
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 102.0,
                volume: index as f64,
            })
            .collect(),
    );
    let mut tree = Tree::new(node(
        "root",
        Stack::vertical().children(vec![
            node("breadcrumb", Breadcrumb::with_segments(vec!["a", "b"])),
            node("button", Button::new(ButtonVariant::Primary, "OK")),
            node("badge", Badge::new("new")),
            node(
                "boxplot",
                BoxPlot::new().with_rows(vec![BoxPlotRow::new(0.0, 2.0, 4.0, 6.0, 8.0)]),
            ),
            node(
                "checkbox",
                terminalui_components::checkbox::Checkbox::with_label("c"),
            ),
            node("dialog", Dialog::with_default_actions()),
            node("expandable", Expandable::with_lines("ex", vec!["a"])),
            node("chart", chart),
            node(
                "floating",
                terminalui_components::floating::FloatingBox::new(vec![node(
                    "content",
                    Text::new("body"),
                )]),
            ),
            node("form", Form::with_fields(vec![("name", "Name")])),
            node("grid", ResizableGrid::row(vec![1, 1])),
            node("input", terminalui_components::input::Input::new()),
            node("list", List::with_items(vec!["a", "b"])),
            node("overlay", terminalui_components::overlay::Overlay::new()),
            node("pagination", Pagination::new(5)),
            node(
                "panel",
                Panel::line().children(vec![node("child", Text::new("inside"))]),
            ),
            node("progress", Progress::new(0.5)),
            node(
                "select",
                terminalui_components::select::Select::new(vec![
                    node("a", Text::new("x")),
                    node("b", Text::new("y")),
                ]),
            ),
            node("separator", Separator::new()),
            node(
                "slider",
                terminalui_components::slider::Slider::new(0.0, 10.0, 1.0),
            ),
            node("spinner", Spinner::new()),
            node("tabs", Tabs::with_titles(vec!["one", "two"])),
            node("table", {
                let mut table = Table::new();
                table.set_rows(vec![node(
                    "row-0",
                    TableRow::body(vec![node("cell-0", TableCell::text("x"))]),
                )]);
                table
            }),
            node("toast", Toast::new(ToastKind::Info, "msg")),
            node("tooltip", Tooltip::new("tip", 0, 0)),
            node(
                "radio",
                terminalui_components::radio::RadioGroup::with_options(vec!["a", "b"]),
            ),
            node(
                "switch",
                terminalui_components::switch::Switch::with_label("s"),
            ),
            node(
                "tree",
                terminalui_components::treeview::TreeView::new(vec![
                    terminalui_components::treeview::TreeNode::branch(
                        "parent",
                        vec![terminalui_components::treeview::TreeNode::leaf("child")],
                    ),
                ]),
            ),
            node(
                "linechart",
                terminalui_components::linechart::LineChart::new(),
            ),
            node(
                "barchart",
                terminalui_components::barchart::BarChart::new().with_values(vec![1.0, 2.0]),
            ),
            node(
                "sparkline",
                terminalui_components::sparkline::Sparkline::new(vec![1.0, 2.0, 3.0]),
            ),
            node(
                "stat",
                terminalui_components::stat::Stat::new("services", "12"),
            ),
            node(
                "dropdown",
                terminalui_components::dropdown::DropdownMenu::new(vec![node(
                    "trigger",
                    Text::new("open menu"),
                )])
                .with_items(vec![terminalui_components::menu::MenuEntry::item("first")]),
            ),
            node(
                "contextmenu",
                terminalui_components::contextmenu::ContextMenu::new(vec![
                    terminalui_components::menu::MenuEntry::item("open"),
                ]),
            ),
            node(
                "palette",
                terminalui_components::commandpalette::CommandPalette::new(vec![
                    terminalui_components::commandpalette::Command::new("run"),
                ]),
            ),
            node(
                "autocomplete",
                terminalui_components::autocomplete::Autocomplete::new(vec!["alpha".to_string()]),
            ),
            node("kbd", terminalui_components::kbd::Kbd::new("Enter")),
            node(
                "stepper",
                terminalui_components::stepper::Stepper::new(vec![
                    terminalui_components::stepper::Step::new("plan")
                        .with_status(terminalui_components::stepper::StepStatus::Done),
                    terminalui_components::stepper::Step::new("build")
                        .with_status(terminalui_components::stepper::StepStatus::Active),
                    terminalui_components::stepper::Step::new("ship"),
                ]),
            ),
            node(
                "statusbar",
                terminalui_components::statusbar::StatusBar::new()
                    .left(vec![node("bar-left", Text::new("main"))])
                    .right(vec![node(
                        "bar-kbd",
                        terminalui_components::kbd::Kbd::new("q"),
                    )]),
            ),
            node(
                "popover",
                terminalui_components::popover::Popover::new("tip", 2, 2),
            ),
            node(
                "gauge",
                terminalui_components::gauge::Gauge::new(0.6)
                    .with_children(vec![node("caption", Text::new("cpu"))]),
            ),
        ]),
    ));
    let theme = theme();
    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    tree.render(area, &mut buffer, &theme);
    let value = json(&tree);
    let mut fallback = Vec::new();
    collect_fallback_kinds(&value["root"], &mut fallback);
    assert!(
        fallback.is_empty(),
        "these nodes fell back to the default 'view' kind: {fallback:?}"
    );
}

fn collect_fallback_kinds(node: &serde_json::Value, out: &mut Vec<String>) {
    if node["kind"] == "view" {
        out.push(node["id"].as_str().unwrap_or("?").to_string());
    }
    if let Some(children) = node["view"]["children"].as_array() {
        for child in children {
            collect_fallback_kinds(child, out);
        }
    }
}
