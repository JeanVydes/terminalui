//! The application state and orchestration for the terminalui showcase.
//! One composable view tree per tab plus a modal dialog tree; the host
//! wires meaning to component state and components own geometry, input
//! mechanics and visuals.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use terminalui::input::InputEvent;
use terminalui::theme::Theme;
use terminalui::Text;
use terminalui::Tree;
use terminalui::components::boxplot::BoxPlot;
use terminalui::components::breadcrumb::Breadcrumb;
use terminalui::components::button::Button;
use terminalui::components::candlechart::CandleChart;
use terminalui::components::form::Form;
use terminalui::components::grid::ResizableGrid;
use terminalui::components::input::Input;
use terminalui::components::pagination::Pagination;
use terminalui::components::presentation::{RenderStyle, StyleTuning};
use terminalui::components::select::Select;
use terminalui::components::table::Table;
use terminalui::components::tabs::Tabs;

use crate::catalog_tab::build_catalog_tree;
use crate::colors_tab::build_colors_tree;
use crate::common::{
    blit, build_dialog_tree, chart_entrance_fxs, fx_for, id, DialogKind, FxState, FOCUS_DEFAULT,
    PAGE_SIZE, TAB_CATALOG, TAB_DEPLOY, TAB_FX, TAB_MARKET, TAB_NAMES, TAB_OVERVIEW, TAB_STYLES,
};
use crate::data::Service;
use crate::data::{selected_service, services, total_pages};
use crate::fx_tab::build_fx_tree;
use crate::latency::LatencyMonitor;
use crate::market::{InventedMarket, Timeframe};
use crate::market_tab::build_market_tree;
use crate::overview::{build_deploy_tree, build_help_tree, build_overview_tree, table_rows};
use crate::presentations_tab::build_presentations_tree;

/// The three palettes this build ships with; cycled with `t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeKind {
    Dark,
    Ansi16,
    Ansi256,
}

impl ThemeKind {
    fn theme(&self) -> Theme {
        match self {
            ThemeKind::Dark => Theme::default(),
            ThemeKind::Ansi16 => Theme::for_ansi16(),
            ThemeKind::Ansi256 => Theme::for_ansi256(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ThemeKind::Dark => "default",
            ThemeKind::Ansi16 => "ansi16",
            ThemeKind::Ansi256 => "ansi256",
        }
    }
}

pub struct App {
    pub theme: Theme,
    services: Vec<Service>,
    view: Vec<usize>,
    page: usize,
    tab: usize,
    filter_text: String,
    crumb_highlight: usize,
    trees: Vec<Tree>,
    markets: Vec<InventedMarket>,
    tuning: terminalui::components::presentation::StyleTuning,
    dialog: Option<Tree>,
    pending_dialog: Option<DialogKind>,
    pending_values: Option<Vec<(String, String)>>,
    status_message: String,
    pub should_quit: bool,
    pub last_tick: Instant,
    fx: Option<FxState>,
    fx_index: usize,
    chart_fxs: [Option<FxState>; 4],
    fx_cells: Vec<Option<FxState>>,
    theme_kind: ThemeKind,
    fx_seed: u64,
    mouse_position: Option<(u16, u16)>,
    latency: LatencyMonitor,
    styles_time: f64,
    gradient_key: Option<(
        terminalui::components::presentation::RenderStyle,
        terminalui::components::presentation::StyleTuning,
    )>,
}

impl App {
    pub fn new() -> Self {
        let theme = Theme::default();
        let latency = LatencyMonitor::new(&theme);
        let services = services();
        let view: Vec<usize> = (0..services.len()).collect();
        let markets: Vec<InventedMarket> = [
            Timeframe::Secs1,
            Timeframe::Secs5,
            Timeframe::Secs30,
            Timeframe::Min1,
        ]
        .into_iter()
        .map(InventedMarket::new)
        .collect();
        let mut app = Self {
            theme,
            services,
            view,
            page: 0,
            tab: TAB_OVERVIEW,
            filter_text: String::new(),
            crumb_highlight: 2,
            trees: vec![
                build_overview_tree(&theme),
                build_deploy_tree(&theme),
                build_help_tree(&theme),
                build_market_tree(&theme, &markets),
                build_fx_tree(&theme),
                build_presentations_tree(&theme, &latency),
                build_colors_tree(&theme),
                build_catalog_tree(&theme),
            ],
            markets,
            tuning: terminalui::components::presentation::StyleTuning::default(),
            dialog: None,
            pending_dialog: None,
            pending_values: None,
            status_message: String::from("Welcome — press 3 for help"),
            should_quit: false,
            last_tick: Instant::now(),
            fx: None,
            fx_index: 0,
            chart_fxs: [None, None, None, None],
            fx_cells: (0..14)
                .map(|index| Some(crate::fx_tab::fresh_cell(index, 42 + index as u64)))
                .collect(),
            theme_kind: ThemeKind::Dark,
            fx_seed: 42,
            mouse_position: None,
            latency,
            styles_time: 0.0,
            gradient_key: None,
        };
        app.focus_default();
        app
    }

    fn page_slice(&self) -> Vec<usize> {
        let start = self.page * PAGE_SIZE;
        self.view
            .iter()
            .skip(start)
            .take(PAGE_SIZE)
            .copied()
            .collect()
    }

    fn focus_default(&mut self) {
        self.trees[self.tab].focus(&id(FOCUS_DEFAULT[self.tab]));
    }

    /// Compact outline for every tab tree, in tab order.
    pub fn tree_outlines(&self) -> Vec<(&'static str, String)> {
        self.trees
            .iter()
            .enumerate()
            .map(|(index, tree)| (TAB_NAMES[index], tree.inspect_outline()))
            .collect()
    }

    /// Outline dump for a file: one `## <tab>` section per tab, in tab order.
    pub fn tree_outlines_document(&self) -> String {
        let mut document = String::new();
        for (name, outline) in self.tree_outlines() {
            document.push_str("## ");
            document.push_str(name);
            document.push('\n');
            document.push_str(&outline);
            document.push('\n');
        }
        document
    }

    /// Every tab tree as one JSON document, keyed by tab name.
    pub fn tree_json(&self) -> Result<String, serde_json::Error> {
        let mut tabs = serde_json::Map::with_capacity(self.trees.len());
        for (index, tree) in self.trees.iter().enumerate() {
            let value = serde_json::from_str(&tree.to_json()?)?;
            tabs.insert(TAB_NAMES[index].to_string(), value);
        }
        serde_json::to_string_pretty(&serde_json::Value::Object(tabs))
    }

    /// Show the catalog tooltip only while the mouse is over the trigger,
    /// anchored next to it.
    fn sync_hover_tooltip(&mut self) {
        let show = self.mouse_over(&self.trees[TAB_CATALOG], "cat-hover-trigger");
        let trigger_area = self.trees[TAB_CATALOG]
            .find::<Button>(&id("cat-hover-trigger"))
            .map(|button| button.state.area);
        if let Some(tooltip) = self.trees[TAB_CATALOG]
            .find_mut::<terminalui::components::tooltip::Tooltip>(&id("cat-tooltip"))
        {
            if show {
                tooltip.show();
                if let Some(area) = trigger_area {
                    tooltip.set_position(
                        terminalui::components::floating::FloatingPosition::Coordinates {
                            x: area.x,
                            y: area.y + area.height + 1,
                        },
                    );
                }
            } else {
                tooltip.hide();
            }
        }
    }

    /// True when the mouse is over the given view's rendered area.
    fn mouse_over(&self, tree: &terminalui::Tree, target: &str) -> bool {
        let Some((column, row)) = self.mouse_position else {
            return false;
        };
        tree.find::<Button>(&id(target))
            .map(|button| {
                let area = button.state.area;
                column >= area.x
                    && column < area.x.saturating_add(area.width)
                    && row >= area.y
                    && row < area.y.saturating_add(area.height)
            })
            .unwrap_or(false)
    }

    /// Push the current styles-tab controls (per-component preset selects and
    /// tuning sliders) into every component that consumes them. Independent:
    /// chart, progress and gradient each keep their own presentation and knobs.
    /// Runs every frame; idempotent writes only when something changed.
    fn sync_tuner(&mut self) {
        use terminalui::components::slider::Slider;

        let tree = &mut self.trees[TAB_STYLES];
        let slider_value = |tree: &Tree, name: &str| {
            tree.find::<Slider>(&id(name))
                .map(Slider::value)
                .unwrap_or(0.0)
        };
        let tuning = StyleTuning {
            density: (slider_value(tree, "box-density").round() as u8).clamp(2, 6),
            contrast: slider_value(tree, "box-contrast").clamp(0.25, 4.0) as f32,
            dotsize: StyleTuning::default().dotsize,
        };
        let box_preset = tree
            .find::<Select>(&id("preset-box"))
            .and_then(Select::selected)
            .unwrap_or(1);
        let progress_preset = tree
            .find::<Select>(&id("preset-progress"))
            .and_then(Select::selected)
            .unwrap_or(1);
        let gradient_preset = tree
            .find::<Select>(&id("preset-gradient"))
            .and_then(Select::selected)
            .unwrap_or(1);
        let presentation_of = |index: usize| {
            crate::presentations_tab::PRESENTATIONS
                .get(index)
                .map(|(style, _)| *style)
                .unwrap_or(RenderStyle::Halftone)
        };
        let name_of = |index: usize| {
            crate::presentations_tab::PRESENTATIONS
                .get(index)
                .map(|(_, name)| *name)
                .unwrap_or("?")
        };

        let box_presentation = presentation_of(box_preset);
        let progress_presentation = presentation_of(progress_preset);
        let gradient_presentation = presentation_of(gradient_preset);

        if let Some(plot) = tree.find_mut::<BoxPlot>(&id("style-boxplot")) {
            plot.presentation = box_presentation;
            plot.tuning = tuning;
        }
        for slider_id in ["box-density", "box-contrast"] {
            if let Some(slider) = tree.find_mut::<Slider>(&id(slider_id)) {
                slider.presentation = box_presentation;
                slider.tuning = tuning;
            }
        }
        let readouts = [
            ("label-box-density", format!("{}", tuning.density)),
            ("label-box-contrast", format!("{:.2}", tuning.contrast)),
        ];
        for (label_id, text) in readouts {
            if let Some(label) = tree.find_mut::<Text>(&id(label_id)) {
                label.set_text(text);
            }
        }
        if let Some(progress) =
            tree.find_mut::<terminalui::components::progress::Progress>(&id("style-progress"))
        {
            progress.presentation = progress_presentation;
        }
        let gradient_key = (gradient_presentation, StyleTuning::default());
        if self.gradient_key != Some(gradient_key) {
            if let Some(gradient) = tree.find_mut::<Text>(&id("style-gradient")) {
                gradient.set_text(crate::presentations_tab::ramp_gradient(
                    gradient_presentation,
                    &StyleTuning::default(),
                    52,
                ));
            }
            self.gradient_key = Some(gradient_key);
        }
        self.tuning = tuning;
        self.status_message = format!(
            "box:{} p:{} g:{}",
            name_of(box_preset),
            name_of(progress_preset),
            name_of(gradient_preset),
        );
    }

    /// Chart pane rects in declaration order: left column top→bottom, then right.
    fn chart_rects(&mut self, area: Rect) -> [Rect; 4] {
        let id = |name: &str| name.to_string();
        let market = &mut self.trees[TAB_MARKET];
        let outer = market
            .find::<ResizableGrid>(&id("market-grid"))
            .map(|grid| grid.pane_areas(grid.state.area))
            .unwrap_or_else(|| vec![area, area]);
        let left_rect = outer.first().copied().unwrap_or(area);
        let right_rect = outer.get(1).copied().unwrap_or(left_rect);
        let left = market
            .find::<ResizableGrid>(&id("market-left"))
            .map(|grid| grid.pane_areas(grid.state.area))
            .unwrap_or_else(|| vec![left_rect, left_rect]);
        let right = market
            .find::<ResizableGrid>(&id("market-right"))
            .map(|grid| grid.pane_areas(grid.state.area))
            .unwrap_or_else(|| vec![right_rect, right_rect]);
        [
            left.first().copied().unwrap_or(left_rect),
            left.get(1).copied().unwrap_or(left_rect),
            right.first().copied().unwrap_or(right_rect),
            right.get(1).copied().unwrap_or(right_rect),
        ]
    }

    /// FX cell rects in declaration order: left column first, then right.
    fn fx_cell_rects(&mut self, area: Rect) -> [Rect; 14] {
        let id = |name: &str| name.to_string();
        let fx_tree = &mut self.trees[TAB_FX];
        // Each grid reports the area it was rendered into; pane areas are
        // computed from that, not from the host's content rect.
        let outer = fx_tree
            .find::<ResizableGrid>(&id("fx-grid"))
            .map(|grid| grid.pane_areas(grid.state.area))
            .unwrap_or_else(|| vec![area, area]);
        let left_rect = outer.first().copied().unwrap_or(area);
        let right_rect = outer.get(1).copied().unwrap_or(left_rect);
        let left = fx_tree
            .find::<ResizableGrid>(&id("fx-left"))
            .map(|grid| grid.pane_areas(grid.state.area))
            .unwrap_or_else(|| vec![left_rect; 7]);
        let right = fx_tree
            .find::<ResizableGrid>(&id("fx-right"))
            .map(|grid| grid.pane_areas(grid.state.area))
            .unwrap_or_else(|| vec![right_rect; 7]);
        let mut rects = [Rect::new(0, 0, 0, 0); 14];
        for (index, slot) in left.into_iter().take(7).enumerate() {
            rects[index] = slot;
        }
        for (index, slot) in right.into_iter().take(7).enumerate() {
            rects[7 + index] = slot;
        }
        rects
    }

    fn switch_tab(&mut self, tab: usize) {
        self.tab = tab.min(TAB_NAMES.len() - 1);
        for tree in &mut self.trees {
            if let Some(tabs) = tree.find_mut::<Tabs>(&id("tabs")) {
                tabs.set_selected(self.tab);
            }
            if let Some(breadcrumb) = tree.find_mut::<Breadcrumb>(&id("breadcrumb")) {
                breadcrumb.set_segments(vec!["acme", "deploys", TAB_NAMES[self.tab]]);
            }
        }
        self.crumb_highlight = 2;
        if self.tab == TAB_MARKET {
            self.fx = None;
            self.chart_fxs = chart_entrance_fxs();
            self.status_message = String::from("market: rise fade dissolve sweep");
        } else {
            let (effect, name) = fx_for(self.fx_index);
            self.fx_index = self.fx_index.wrapping_add(1);
            self.fx = Some(FxState {
                effect,
                anim: terminalui::Animation::new(
                    Duration::from_millis(650),
                    terminalui::Easing::EaseOutCubic,
                ),
            });
            self.status_message = format!("tab -> {name}");
        }
        self.focus_default();
    }

    fn cycle_theme(&mut self) {
        self.theme_kind = match self.theme_kind {
            ThemeKind::Dark => ThemeKind::Ansi16,
            ThemeKind::Ansi16 => ThemeKind::Ansi256,
            ThemeKind::Ansi256 => ThemeKind::Dark,
        };
        self.theme = self.theme_kind.theme();
        self.status_message = format!("theme: {}", self.theme_kind.name());
    }

    fn input_focused(&self) -> bool {
        let tree = &self.trees[self.tab];
        match tree.focused_id() {
            Some(focused) => tree.find::<Input>(&id(focused)).is_some(),
            None => false,
        }
    }

    fn refresh_page(&mut self) {
        let rows = table_rows(&self.services, &self.page_slice(), self.page);
        if let Some(table) = self.trees[TAB_OVERVIEW].find_mut::<Table>(&id("table")) {
            table.set_rows(rows);
        }
        self.status_message = format!("Page {} of {}", self.page + 1, total_pages(self.view.len()));
    }

    fn apply_filter(&mut self) {
        let needle = self.filter_text.trim().to_lowercase();
        let view: Vec<usize> = if needle.is_empty() {
            (0..self.services.len()).collect()
        } else {
            self.services
                .iter()
                .enumerate()
                .filter(|(_, service)| {
                    service.name.to_lowercase().contains(&needle)
                        || service.status.to_lowercase().contains(&needle)
                        || service.env.to_lowercase().contains(&needle)
                })
                .map(|(index, _)| index)
                .collect()
        };
        let matches = view.len();
        self.view = view;
        self.page = 0;
        let page_count = total_pages(self.view.len());
        if let Some(pagination) = self.trees[TAB_OVERVIEW].find_mut::<Pagination>(&id("pagination"))
        {
            pagination.set_total_pages(page_count);
            pagination.go_to(0);
        }
        let rows = table_rows(&self.services, &self.page_slice(), 0);
        if let Some(table) = self.trees[TAB_OVERVIEW].find_mut::<Table>(&id("table")) {
            table.set_rows(rows);
        }
        self.status_message = format!("{matches} service(s) match");
    }

    fn open_dialog(&mut self, kind: DialogKind) {
        self.pending_dialog = Some(kind);
        let selected = match kind {
            DialogKind::RowDetails => {
                let slice = self.page_slice();
                let selected = self.trees[TAB_OVERVIEW]
                    .find::<Table>(&id("table"))
                    .map(|table| table.selected())
                    .unwrap_or(0);
                Some(selected_service(&self.services, &slice, selected))
            }
            _ => None,
        };
        let theme = self.theme;
        let dialog = build_dialog_tree(kind, self.pending_values.clone(), selected, &theme);
        self.dialog = Some(dialog);
    }

    fn close_dialog(&mut self, primary: bool) {
        self.dialog = None;
        match (self.pending_dialog.take(), primary) {
            (Some(DialogKind::ConfirmQuit), true) => {
                self.should_quit = true;
            }
            (Some(DialogKind::ConfirmQuit), false) => {
                self.status_message = "Staying alive".to_string();
            }
            (Some(DialogKind::DeployQueued), _) => {
                self.status_message = "Deploy queued — watch the rollout".to_string();
            }
            (Some(DialogKind::RowDetails), true) => {
                let slice = self.page_slice();
                let selected = self.trees[TAB_OVERVIEW]
                    .find::<Table>(&id("table"))
                    .map(|table| table.selected())
                    .unwrap_or(0);
                let service = selected_service(&self.services, &slice, selected);
                self.status_message = format!("Rollout ping sent to {}", service.name);
            }
            _ => {
                self.status_message.clear();
            }
        }
    }

    fn drain_dialog_buttons(&mut self) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let confirm = dialog
            .find_mut::<Button>(&id("confirm"))
            .map(|button| button.take_pressed())
            .unwrap_or(false);
        let cancel = dialog
            .find_mut::<Button>(&id("cancel"))
            .map(|button| button.take_pressed())
            .unwrap_or(false);
        if confirm {
            self.close_dialog(true);
        } else if cancel {
            self.close_dialog(false);
        }
    }

    fn drain_tree_buttons(&mut self) {
        let mut about = false;
        let mut quit = false;
        for tree in &mut self.trees {
            if let Some(button) = tree.find_mut::<Button>(&id("about-button")) {
                about |= button.take_pressed();
            }
            if let Some(button) = tree.find_mut::<Button>(&id("quit-button")) {
                quit |= button.take_pressed();
            }
        }
        if about {
            self.open_dialog(DialogKind::About);
        } else if quit {
            self.open_dialog(DialogKind::ConfirmQuit);
        }
    }

    pub fn handle_event(&mut self, event: &InputEvent) {
        if let InputEvent::Mouse(mouse) = event {
            self.mouse_position = Some((mouse.column, mouse.row));
        }
        if self.dialog.is_some() {
            if let InputEvent::Key(key) = event {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc {
                    self.close_dialog(false);
                    return;
                }
            }
            if let Some(dialog) = &mut self.dialog {
                dialog.handle_event(event);
            }
            self.drain_dialog_buttons();
            return;
        }
        if let InputEvent::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }
            match key.code {
                KeyCode::F(number) if (1..=8).contains(&number) => {
                    self.switch_tab(usize::from(number) - 1);
                    return;
                }
                KeyCode::Esc => {
                    self.trees[self.tab].focus(&id("tabs"));
                    return;
                }
                _ => {}
            }
            if !self.input_focused() {
                if key.code == KeyCode::Char('t') {
                    self.cycle_theme();
                    return;
                }
                match key.code {
                    KeyCode::Char(digit @ '1'..='8') => {
                        self.switch_tab(digit as usize - '1' as usize);
                        return;
                    }
                    KeyCode::Char('q') => {
                        self.open_dialog(DialogKind::ConfirmQuit);
                        return;
                    }
                    _ => {}
                }
            }
        }
        let enter_pressed = matches!(
            event,
            InputEvent::Key(key) if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter
        );
        self.trees[self.tab].handle_event(event);
        // Focus follows a left click on the services table, mirroring the
        // keyboard Tab path so Enter can activate the row.
        if let InputEvent::Mouse(mouse) = event {
            if self.tab == TAB_OVERVIEW
                && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            {
                let table_area = self.trees[TAB_OVERVIEW]
                    .find::<Table>(&id("table"))
                    .map(|table| table.state.area);
                if let Some(area) = table_area {
                    let inside = mouse.column >= area.x
                        && mouse.column < area.x.saturating_add(area.width)
                        && mouse.row >= area.y
                        && mouse.row < area.y.saturating_add(area.height);
                    if inside {
                        self.trees[TAB_OVERVIEW].focus(&id("table"));
                    }
                }
            }
        }
        self.drain_tree_buttons();
        self.drain_tree_diffs(enter_pressed);
    }

    /// Synchronize host state with component state owned by the trees.
    fn drain_tree_diffs(&mut self, enter_pressed: bool) {
        // Tab strip selection.
        let selected = self.trees[self.tab]
            .find::<Tabs>(&id("tabs"))
            .map(Tabs::selected);
        let tab = self.tab;
        if let Some(selected) = selected {
            if selected != tab {
                self.switch_tab(selected);
            }
        }

        // Pagination (overview only).
        let page = self.trees[TAB_OVERVIEW]
            .find::<Pagination>(&id("pagination"))
            .map(Pagination::page);
        if let Some(page) = page {
            if page != self.page {
                self.page = page;
                self.refresh_page();
            }
        }

        // Filter text drives the table view.
        let filter = self.trees[TAB_OVERVIEW]
            .find::<Input>(&id("filter"))
            .map(|input| input.state.value.clone());
        if let Some(filter) = filter {
            if filter != self.filter_text {
                self.filter_text = filter;
                self.apply_filter();
            }
        }

        // Breadcrumb ancestor navigation.
        let highlight = self.trees[self.tab]
            .find::<Breadcrumb>(&id("breadcrumb"))
            .map(Breadcrumb::highlight);
        if let Some(highlight) = highlight {
            if highlight != self.crumb_highlight {
                self.crumb_highlight = highlight;
                if highlight < 2 && self.tab != TAB_OVERVIEW {
                    self.switch_tab(TAB_OVERVIEW);
                }
            }
        }

        // Styles tab: sliders and preset select sync every frame.
        if self.tab == TAB_STYLES {
            self.sync_tuner();
        }

        // Catalog tab: the "open a dialog" button opens the About dialog.
        if self.tab == TAB_CATALOG {
            let open = self.trees[TAB_CATALOG]
                .find_mut::<Button>(&id("cat-dialog-button"))
                .map(Button::take_pressed);
            if open == Some(true) {
                self.open_dialog(DialogKind::About);
            }
        }

        // Enter on the table opens row details.
        if enter_pressed && self.tab == TAB_OVERVIEW {
            let table_focused = self.trees[TAB_OVERVIEW].focused_id() == Some("table");
            if table_focused {
                self.open_dialog(DialogKind::RowDetails);
            }
        }

        // Successful form submit opens the queue dialog.
        let submitted = self.trees[TAB_DEPLOY]
            .find_mut::<Form>(&id("form"))
            .and_then(Form::take_submitted);
        if let Some(values) = submitted {
            self.pending_values = Some(values);
            self.open_dialog(DialogKind::DeployQueued);
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        for tree in &mut self.trees {
            tree.tick(delta);
        }
        if let Some(dialog) = &mut self.dialog {
            dialog.tick(delta);
        }
        // The host feeds the markets; the charts render them.
        for index in 0..self.markets.len() {
            let market_tick = self.markets[index].tick(delta);
            let chart_id = id(["chart-1s", "chart-5s", "chart-30s", "chart-1m"][index]);
            if let Some(chart) = self.trees[TAB_MARKET].find_mut::<CandleChart>(&chart_id) {
                if market_tick.new_candle {
                    chart.push_candle(market_tick.candle);
                }
                chart.update_candle(self.markets[index].live_candle());
            }
        }
        if let Some(fx) = &mut self.fx {
            fx.anim.tick(delta);
            if fx.anim.is_done() {
                self.fx = None;
            }
        }
        for slot in &mut self.chart_fxs {
            if let Some(fx) = slot {
                fx.anim.tick(delta);
                if fx.anim.is_done() {
                    *slot = None;
                }
            }
        }
        for index in 0..self.fx_cells.len() {
            let done = self.fx_cells[index]
                .as_mut()
                .map(|fx| {
                    fx.anim.tick(delta);
                    fx.anim.is_done()
                })
                .unwrap_or(false);
            if done {
                self.fx_seed = self.fx_seed.wrapping_add(1);
                self.fx_cells[index] = Some(crate::fx_tab::fresh_cell(index, self.fx_seed));
            }
        }
        self.styles_time += delta.as_secs_f64();
        let rows = self.latency.rows(self.styles_time);
        let caption = self.latency.caption(self.styles_time);
        let tree = &mut self.trees[TAB_STYLES];
        if let Some(plot) = tree.find_mut::<BoxPlot>(&id("style-boxplot")) {
            plot.set_rows(rows);
        }
        if let Some(label) = tree.find_mut::<Text>(&id("box-window")) {
            label.set_text(caption);
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        buf.set_style(area, Style::default().bg(theme.background));
        if area.height == 0 {
            return;
        }
        let content = Rect::new(area.x, area.y, area.width, area.height - 1);
        let status = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        if self.tab == TAB_FX && self.fx_cells.iter().any(|fx| fx.is_some()) {
            let mut scratch = Buffer::empty(content);
            scratch.set_style(content, Style::default().bg(theme.background));
            self.trees[self.tab].render(content, &mut scratch, theme);
            blit(buf, &scratch, content);
            let rects = self.fx_cell_rects(content);
            for (index, fx) in self.fx_cells.iter_mut().enumerate() {
                let Some(fx) = fx else { continue };
                if fx.anim.is_done() {
                    continue;
                }
                // The panel border stays static; only the content animates.
                let rect = rects[index];
                let inner = Rect::new(
                    rect.x.saturating_add(1),
                    rect.y.saturating_add(1),
                    rect.width.saturating_sub(2),
                    rect.height.saturating_sub(2),
                );
                fx.effect
                    .apply(&scratch, buf, inner, theme, fx.anim.progress());
            }
        } else if self.tab == TAB_MARKET && self.chart_fxs.iter().any(|fx| fx.is_some()) {
            let mut scratch = Buffer::empty(content);
            scratch.set_style(content, Style::default().bg(theme.background));
            self.trees[self.tab].render(content, &mut scratch, theme);
            blit(buf, &scratch, content);
            let cells = self.chart_rects(content);
            for (index, fx) in self.chart_fxs.iter_mut().enumerate() {
                let Some(fx) = fx else { continue };
                if fx.anim.is_done() {
                    continue;
                }
                fx.effect
                    .apply(&scratch, buf, cells[index], theme, fx.anim.progress());
            }
        } else if let Some(fx) = self.fx.clone() {
            if !fx.anim.is_done() {
                let mut scratch = Buffer::empty(content);
                scratch.set_style(content, Style::default().bg(theme.background));
                self.trees[self.tab].render(content, &mut scratch, theme);
                fx.effect
                    .apply(&scratch, buf, content, theme, fx.anim.progress());
            } else {
                self.trees[self.tab].render(content, buf, theme);
            }
        } else {
            self.trees[self.tab].render(content, buf, theme);
        }
        if self.tab == TAB_CATALOG {
            self.sync_hover_tooltip();
        }
        if let Some(dialog) = &mut self.dialog {
            dialog.render(area, buf, theme);
        }
        self.render_status(status, buf, theme);
    }

    fn render_status(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        buf.set_style(area, Style::default().bg(theme.secondary));
        let hints = " F1-F8 tabs  t theme  esc tabs  q quit";
        let message = self.status_message.clone();
        let max_hints = area.width.saturating_sub(1) as usize;
        buf.set_stringn(
            area.x,
            area.y,
            hints,
            max_hints,
            Style::default().fg(theme.muted).bg(theme.secondary),
        );
        if message.is_empty() {
            return;
        }
        let width = message.chars().count().min(area.width as usize / 2) as u16;
        let limited: String = message.chars().take(width as usize).collect();
        if area.width > width + 4 {
            buf.set_stringn(
                area.x + area.width.saturating_sub(width + 1),
                area.y,
                limited,
                width as usize,
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
