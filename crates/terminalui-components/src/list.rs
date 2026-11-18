//! List: a selectable list with per-item markers or composed icons.
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::scroll::{render_scrollbar, Scroll, ScrollbarStyle};

/// How each item is prefixed.
#[derive(Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Marker {
    /// No marker, items only.
    None,
    /// A bullet before each item.
    #[default]
    Bullet,
    /// `1.`, `2.`, `3.` decimal numbering.
    Decimal,
    /// `I.`, `II.`, `III.` Roman numerals.
    Roman,
    /// `A.`, `B.`, `C.` lettered.
    Alphabetical,
    /// Composed marker rendered at every row's prefix. Compose any view
    /// (icon, badge, stack of views) with `terminalui_core::node(...)`.
    View(Node),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Index of the highlighted item.
    pub selected: usize,
    /// First visible item when the list overflows.
    pub offset: usize,
    /// Whether the list would hold focus by the tree.
    pub focused: bool,
}

/// Per-instance overrides for a list. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ListStyle {
    /// Normal item text color. Defaults to `theme.foreground`.
    pub foreground: Option<Color>,
    /// Highlighted item text color. Defaults to `theme.background`.
    pub selected_fg: Option<Color>,
    /// Highlighted item background. Defaults to `theme.primary`.
    pub selected_bg: Option<Color>,
    /// Marker color. Defaults to `theme.muted`.
    pub marker: Option<Color>,
    /// Render the highlighted item in bold.
    pub bold: Option<bool>,
    /// Scrollbar drawn when items overflow the visible height. Auto-hides
    /// when everything fits. Default [`ScrollbarStyle::default()`].
    pub scrollbar: Option<ScrollbarStyle>,
}

/// A selectable vertical list. Items are arbitrary views (usually
/// [`Text`]); selection is by index and reported through keyboard or mouse
/// interaction. Hosts that need typed activation read [`List::selected`].
#[derive(Serialize)]
pub struct List {
    /// How each item is prefixed.
    pub marker: Marker,
    /// Per-instance overrides; defaults match the theme.
    pub style: ListStyle,
    /// Public state: selection and visibility.
    pub state: ListState,
    children: Vec<Node>,
    hit_areas: Vec<Rect>,
}

fn roman(index: usize) -> String {
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut remaining = index as i64;
    let mut out = String::new();
    for (value, symbol) in NUMERALS {
        while remaining >= value as i64 {
            out.push_str(symbol);
            remaining -= value as i64;
        }
    }
    out
}

fn marker_for(marker: &Marker, index: usize) -> String {
    match marker {
        Marker::None | Marker::View(_) => String::new(),
        Marker::Bullet => "●".to_string(),
        Marker::Decimal => format!("{}.", index + 1),
        Marker::Roman => format!("{}.", roman(index + 1)),
        Marker::Alphabetical => format!(
            "{}.",
            char::from_u32((b'A' as u32) + (index % 26) as u32).unwrap_or('?')
        ),
    }
}

/// Width reserved for the marker prefix of one row.
fn marker_width(marker: &Marker) -> u16 {
    match marker {
        Marker::None => 0,
        Marker::View(node) => {
            node.view
                .measure()
                .map(|(width, _)| width.max(1))
                .unwrap_or(1)
                + 1
        }
        Marker::Bullet => 2,
        Marker::Decimal | Marker::Roman | Marker::Alphabetical => 3,
    }
}

impl List {
    /// Convenience constructor: every item is a [`Text`] view.
    pub fn with_items(items: Vec<impl Into<String>>) -> Self {
        let children = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| node(format!("item-{index}"), Text::new(item)))
            .collect();
        Self::new(children)
    }
}
impl List {
    /// Compose a list from arbitrary views, one per item.
    /// Compose a list from arbitrary views, one per item.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            marker: Marker::Bullet,
            style: ListStyle::default(),
            state: ListState {
                area: Rect::new(0, 0, 0, 0),
                selected: 0,
                offset: 0,
                focused: false,
            },
            children,
            hit_areas: Vec::new(),
        }
    }

    pub fn with_marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    pub fn with_style(mut self, style: ListStyle) -> Self {
        self.style = style;
        self
    }

    /// The highlighted item index.
    pub fn selected(&self) -> usize {
        self.state.selected
    }

    fn body_height(&self) -> usize {
        self.state.area.height as usize
    }

    fn clamp_view(&mut self) {
        let (selected, offset) = super::scroll::clamp_view(
            self.state.selected,
            self.state.offset,
            self.body_height(),
            self.children.len(),
        );
        self.state.selected = selected;
        self.state.offset = offset;
    }

    fn select(&mut self, index: usize) -> Outcome {
        if self.children.is_empty() {
            return Outcome::Ignored;
        }
        let index = index.min(self.children.len() - 1);
        if index == self.state.selected {
            return Outcome::Ignored;
        }
        self.state.selected = index;
        self.clamp_view();
        Outcome::Consumed
    }
}

impl View for List {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "list"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "selected={}, offset={}",
            self.state.selected, self.state.offset
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
        self.clamp_view();
        let foreground = self.style.foreground.unwrap_or(theme.foreground);
        let selected_fg = self.style.selected_fg.unwrap_or(theme.background);
        let selected_bg = self.style.selected_bg.unwrap_or(theme.primary);
        let marker_color = self.style.marker.unwrap_or(theme.muted);
        let bold = self.style.bold.unwrap_or(true);
        let body = area.height as usize;
        let overflow = self.children.len() > body;
        let row_width = if overflow {
            area.width.saturating_sub(1)
        } else {
            area.width
        };
        self.hit_areas.clear();

        for visible in 0..body {
            let index = self.state.offset + visible;
            if index >= self.children.len() {
                break;
            }
            let y = area.y + visible as u16;
            let selected = index == self.state.selected;
            let marker = marker_for(&self.marker, index);
            let prefix_width = marker_width(&self.marker);
            let row_style = if selected {
                let mut style = Style::default().bg(selected_bg).fg(selected_fg);
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                style
            } else {
                Style::default().fg(foreground)
            };
            let row_rect = Rect::new(area.x, y, row_width, 1);
            buf.set_style(row_rect, row_style);
            if let Marker::View(marker_node) = &mut self.marker {
                let width = prefix_width.saturating_sub(1);
                marker_node
                    .view
                    .render(Rect::new(area.x, y, width, 1), buf, cx, theme);
            } else if !marker.is_empty() {
                buf.set_string(area.x, y, &marker, Style::default().fg(marker_color));
            }
            let child = &mut self.children[index];
            let item_rect = Rect::new(
                area.x + prefix_width,
                y,
                row_width.saturating_sub(prefix_width),
                1,
            );
            child.view.render(item_rect, buf, cx, theme);
            self.hit_areas.push(row_rect);
        }
        if overflow {
            let style = self.style.scrollbar.clone().unwrap_or_default();
            let mut scroll = Scroll::new(body, self.children.len());
            scroll.offset = self.state.offset;
            render_scrollbar(
                buf,
                Rect::new(
                    area.x + area.width.saturating_sub(1),
                    area.y,
                    1,
                    area.height,
                ),
                &style,
                theme,
                &scroll,
            );
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        if self.children.is_empty() {
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
                    KeyCode::End => self.select(self.children.len() - 1),
                    KeyCode::Enter => Outcome::Consumed,
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Outcome::Ignored;
                }
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
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        true
    }
}
