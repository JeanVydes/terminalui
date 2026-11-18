use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::layout::{row, Align};
use super::presentation::RenderStyle;
use super::scroll::{render_scrollbar, Scroll, ScrollbarStyle};

/// A single cell. Children can be any views (text, badges, buttons, ...).
/// `weight` drives the column width allocated by the parent row.
#[derive(Serialize)]
pub struct TableCell {
    pub weight: u16,
    children: Vec<Node>,
}

impl TableCell {
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            weight: 1,
            children,
        }
    }

    /// Convenience constructor: a single [`terminalui_core::Text`] child.
    pub fn text(content: impl Into<String>) -> Self {
        Self::new(vec![terminalui_core::node(
            "cell",
            terminalui_core::Text::new(content),
        )])
    }

    pub fn with_weight(mut self, weight: u16) -> Self {
        self.weight = weight.max(1);
        self
    }
}

impl View for TableCell {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "table_cell"
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        row(&mut self.children, area, Align::Left, 1, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

/// A row of cells laid out by weight. The first composed row passed through
/// [`Table::with_header`] renders as a styled header.
#[derive(Serialize)]
pub struct TableRow {
    pub is_header: bool,
    children: Vec<Node>,
}

impl TableRow {
    /// A body row.
    pub fn body(children: Vec<Node>) -> Self {
        Self {
            is_header: false,
            children,
        }
    }

    /// A header row.
    pub fn header(children: Vec<Node>) -> Self {
        Self {
            is_header: true,
            children,
        }
    }

    fn cell_weights(&self) -> Vec<u16> {
        self.children
            .iter()
            .map(|child| {
                child
                    .view
                    .as_any_ref()
                    .downcast_ref::<TableCell>()
                    .map(|cell| cell.weight)
                    .unwrap_or(1)
            })
            .collect()
    }

    fn cell_rects(&self, area: Rect) -> Vec<Rect> {
        let weights = self.cell_weights();
        let total: u16 = weights.iter().sum::<u16>().max(1);
        let mut rects = Vec::with_capacity(weights.len());
        let mut x = area.x;
        let mut remaining_width = area.width;
        let mut remaining_weight = total;
        for weight in weights {
            let width = if remaining_weight <= weight {
                remaining_width
            } else {
                ((area.width as u32 * weight as u32) / total as u32) as u16
            };
            let width = width.max(1).min(remaining_width);
            rects.push(Rect::new(x, area.y, width, area.height));
            x = x.saturating_add(width);
            remaining_width = remaining_width.saturating_sub(width);
            remaining_weight = remaining_weight.saturating_sub(weight);
        }
        rects
    }
}

impl View for TableRow {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "table_row"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("is_header={}", self.is_header))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let rects = self.cell_rects(area);
        if self.is_header {
            buf.set_style(
                area,
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.muted)
                    .add_modifier(Modifier::BOLD),
            );
        }
        for (child, cell_rect) in self.children.iter_mut().zip(rects) {
            child.view.render(cell_rect, buf, cx, theme);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableState {
    pub area: Rect,
    pub focused: bool,
    pub selected: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TableStyle {
    /// Selected row background. Defaults to `theme.primary`.
    pub selected_bg: Option<Color>,
    /// Selected row foreground. Defaults to `theme.background`.
    pub selected_fg: Option<Color>,
    /// Border color used for the header underline. Defaults to `theme.border`.
    pub border: Option<Color>,
    /// Resolve the header underline glyph from this presentation. Default
    /// [`RenderStyle::Native`].
    pub presentation: Option<RenderStyle>,
    /// Scrollbar drawn when the body overflows the visible rows. Auto-hides
    /// when everything fits. Default [`ScrollbarStyle::default()`].
    pub scrollbar: Option<ScrollbarStyle>,
}

/// A scrollable table. Rows and cells are composed views: build
/// [`TableRow`]s containing [`TableCell`]s and add them as children.
#[derive(Serialize)]
pub struct Table {
    pub state: TableState,
    pub style: TableStyle,
    children: Vec<Node>,
    has_header: bool,
    hit_areas: Vec<Rect>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            state: TableState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                selected: 0,
                offset: 0,
            },
            style: TableStyle::default(),
            children: Vec::new(),
            has_header: false,
            hit_areas: Vec::new(),
        }
    }

    pub fn with_style(mut self, style: TableStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the header row.
    pub fn with_header(mut self, header: Node) -> Self {
        if self.has_header {
            self.children.remove(0);
        } else {
            self.has_header = true;
        }
        self.children.insert(0, header);
        self
    }

    /// Replace the body rows, preserving the header when present.
    pub fn set_rows(&mut self, rows: Vec<Node>) {
        let keep = if self.has_header { 1 } else { 0 };
        self.children.truncate(keep);
        self.children.extend(rows);
        self.clamp_view();
    }

    /// Append body rows.
    pub fn push_row(&mut self, row: Node) {
        self.children.push(row);
        self.clamp_view();
    }

    /// The highlighted body row index.
    pub fn selected(&self) -> usize {
        self.state.selected
    }

    fn body(&self) -> &[Node] {
        if self.has_header {
            &self.children[1..]
        } else {
            &self.children
        }
    }

    fn body_height(&self) -> usize {
        let reserved = if self.has_header { 2 } else { 0 };
        (self.state.area.height as usize).saturating_sub(reserved)
    }

    fn clamp_view(&mut self) {
        let (selected, offset) = super::scroll::clamp_view(
            self.state.selected,
            self.state.offset,
            self.body_height(),
            self.body().len(),
        );
        self.state.selected = selected;
        self.state.offset = offset;
    }

    fn select(&mut self, index: usize) -> Outcome {
        if self.body().is_empty() {
            return Outcome::Ignored;
        }
        let index = index.min(self.body().len() - 1);
        if index == self.state.selected {
            return Outcome::Ignored;
        }
        self.state.selected = index;
        self.clamp_view();
        Outcome::Consumed
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Table {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "table"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "selected={}, rows={}, offset={}",
            self.state.selected,
            self.body().len(),
            self.state.offset
        ))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        self.state.focused = cx.is_focused();
        if area.width < 1 || area.height == 0 {
            return;
        }
        self.clamp_view();
        let selected_bg = self.style.selected_bg.unwrap_or(theme.primary);
        let selected_fg = self.style.selected_fg.unwrap_or(theme.background);
        let border = self.style.border.unwrap_or(theme.border);
        let overflow = self.body().len() > self.body_height();
        let row_width = if overflow {
            area.width.saturating_sub(1)
        } else {
            area.width
        };
        let mut y = area.y;
        let mut body_index = 0usize;
        let mut body_top = area.y;
        let mut body_bottom = y;
        self.hit_areas.clear();

        for child in &mut self.children {
            let is_header = child
                .view
                .as_any_ref()
                .downcast_ref::<TableRow>()
                .map(|row| row.is_header)
                .unwrap_or(false);
            if is_header {
                if y >= area.y + area.height {
                    break;
                }
                let cell = Rect::new(area.x, y, area.width, 1);
                child.view.render(cell, buf, cx, theme);
                y = y.saturating_add(1);
                if y < area.y + area.height {
                    let (hairline_h, _) = self.style.presentation.unwrap_or_default().hairline();
                    buf.set_string(
                        area.x,
                        y,
                        hairline_h.repeat(area.width as usize),
                        Style::default().fg(border),
                    );
                    y = y.saturating_add(1);
                }
                body_top = y;
                continue;
            }
            if body_index < self.state.offset {
                body_index += 1;
                continue;
            }
            if y >= area.y + area.height {
                break;
            }
            let selected = body_index == self.state.selected;
            let cell = Rect::new(area.x, y, row_width, 1);
            if selected {
                buf.set_style(cell, Style::default().bg(selected_bg).fg(selected_fg));
            }
            child.view.render(cell, buf, cx, theme);
            self.hit_areas.push(cell);
            body_index += 1;
            y = y.saturating_add(1);
            body_bottom = y;
        }
        if overflow && body_bottom > body_top {
            let style = self.style.scrollbar.clone().unwrap_or_default();
            let mut scroll = Scroll::new(
                body_bottom.saturating_sub(body_top) as usize,
                self.body().len(),
            );
            scroll.offset = self.state.offset;
            let rail = Rect::new(
                area.x + area.width.saturating_sub(1),
                body_top,
                1,
                body_bottom.saturating_sub(body_top),
            );
            render_scrollbar(buf, rail, &style, theme, &scroll);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        if self.body().is_empty() {
            return Outcome::Ignored;
        }
        let body = self.body_height().max(1);
        match event {
            InputEvent::Key(key) => {
                if !focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Up => self.select(self.state.selected.saturating_sub(1)),
                    KeyCode::Down => self.select(self.state.selected + 1),
                    KeyCode::PageUp => self.select(self.state.selected.saturating_sub(body)),
                    KeyCode::PageDown => self.select(self.state.selected + body),
                    KeyCode::Home => self.select(0),
                    KeyCode::End => self.select(self.body().len() - 1),
                    KeyCode::Enter => Outcome::Consumed,
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => self.select(self.state.selected.saturating_sub(3)),
                MouseEventKind::ScrollDown => self.select(self.state.selected + 3),
                MouseEventKind::Down(MouseButton::Left) => {
                    let hit = self
                        .hit_areas
                        .iter()
                        .position(|rect| rect.contains(Position::new(mouse.column, mouse.row)));
                    match hit {
                        Some(index) => {
                            self.state.selected = self.state.offset + index;
                            self.clamp_view();
                            Outcome::Consumed
                        }
                        None => Outcome::Ignored,
                    }
                }
                _ => Outcome::Ignored,
            },
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        true
    }
}
