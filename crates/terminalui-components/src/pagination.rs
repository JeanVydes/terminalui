//! Pagination: a page pager with jump buttons and a current-page indicator.
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::icons::Icon;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaginationState {
    pub area: Rect,
    pub focused: bool,
    /// Current page, zero-based.
    pub page: usize,
}

/// Per-instance overrides for pagination. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PaginationStyle {
    /// Current page background. Defaults to `theme.primary`.
    pub selected_bg: Option<Color>,
    /// Current page number color. Defaults to `theme.background`.
    pub selected_fg: Option<Color>,
    /// Active arrow and page number color. Defaults to `theme.foreground`.
    pub foreground: Option<Color>,
    /// Disabled arrow/ellipsis color. Defaults to `theme.muted`.
    pub muted: Option<Color>,
    /// Bold the current page. Default true.
    pub bold_selected: Option<bool>,
    /// Below this width the compact `‹ 3/9 ›` layout is used. Default 14.
    pub compact_width: Option<u16>,
}

/// A pager. One [`Text`] child is composed per page; the visible window
/// follows the current page. Hosts read [`Pagination::page`] to know which
/// page is active.
#[derive(Serialize)]
pub struct Pagination {
    pub total_pages: usize,
    /// Consecutive page numbers shown around the current page. Default 5.
    pub max_visible_pages: usize,
    pub state: PaginationState,
    pub style: PaginationStyle,
    children: Vec<Node>,
    hit_areas: Vec<(HitTarget, Rect)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum HitTarget {
    Previous,
    Next,
    Page(usize),
}

impl Pagination {
    /// `total_pages` is clamped to at least 1.
    pub fn new(total_pages: usize) -> Self {
        let total_pages = total_pages.max(1);
        let children = (0..total_pages)
            .map(|index| node(format!("page-{index}"), Text::new((index + 1).to_string())))
            .collect();
        Self {
            total_pages,
            max_visible_pages: 5,
            state: PaginationState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                page: 0,
            },
            style: PaginationStyle::default(),
            children,
            hit_areas: Vec::new(),
        }
    }

    pub fn with_page(total_pages: usize, page: usize) -> Self {
        let mut pagination = Self::new(total_pages);
        pagination.go_to(page);
        pagination
    }

    pub fn with_style(mut self, style: PaginationStyle) -> Self {
        self.style = style;
        self
    }

    /// How many consecutive page numbers to show. Clamped to at least 1.
    pub fn with_max_visible_pages(mut self, max_visible_pages: usize) -> Self {
        self.max_visible_pages = max_visible_pages.max(1);
        self
    }

    /// Returns the current page, zero-based.
    pub fn page(&self) -> usize {
        self.state.page
    }

    pub fn go_to(&mut self, page: usize) {
        self.state.page = page.min(self.total_pages - 1);
    }

    /// Rebuild the per-page children to match a new page count; clamp the
    /// current page. Used when the host's data set shrinks or grows.
    pub fn set_total_pages(&mut self, total_pages: usize) {
        let total_pages = total_pages.max(1);
        self.total_pages = total_pages;
        self.children = (0..total_pages)
            .map(|index| node(format!("page-{index}"), Text::new((index + 1).to_string())))
            .collect();
        self.state.page = self.state.page.min(total_pages - 1);
    }

    fn step(&self, forward: bool) -> usize {
        if forward {
            (self.state.page + 1).min(self.total_pages - 1)
        } else {
            self.state.page.saturating_sub(1)
        }
    }

    fn change(&mut self, page: usize) {
        self.state.page = page.min(self.total_pages - 1);
    }

    /// Visible page slots: a window of `max_visible_pages` consecutive
    /// pages around the current page, plus first/last anchors, with `None`
    /// marking an ellipsis where whole pages are hidden.
    fn visible_pages(&self) -> Vec<Option<usize>> {
        let total = self.total_pages;
        let page = self.state.page;
        let window = self.max_visible_pages.max(1);
        if total <= window {
            return (0..total).map(Some).collect();
        }
        let start = page.saturating_sub(window / 2).min(total - window);
        let end = start + window;
        let mut pages: Vec<Option<usize>> = Vec::new();
        if start <= 2 {
            pages.extend((0..start).map(Some));
        } else {
            pages.push(Some(0));
            pages.push(None);
        }
        pages.extend((start..end).map(Some));
        let tail = total - end;
        if tail <= 2 {
            pages.extend((end..total).map(Some));
        } else {
            pages.push(None);
            pages.push(Some(total - 1));
        }
        pages
    }

    fn hit_test(&self, column: u16, row: u16) -> Option<HitTarget> {
        let position = Position::new(column, row);
        self.hit_areas
            .iter()
            .find(|(_, rect)| rect.contains(position))
            .map(|(target, _)| *target)
    }
}

impl View for Pagination {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "pagination"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "page={}, total_pages={}",
            self.state.page, self.total_pages
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
        self.hit_areas.clear();
        let selected_bg = self.style.selected_bg.unwrap_or(theme.primary);
        let selected_fg = self.style.selected_fg.unwrap_or(theme.background);
        let foreground = self.style.foreground.unwrap_or(theme.foreground);
        let muted = self.style.muted.unwrap_or(theme.muted);
        let bold_selected = self.style.bold_selected.unwrap_or(true);
        let compact_width = self.style.compact_width.unwrap_or(14);
        let y = area.y + area.height.saturating_sub(1) / 2;

        let compact = area.width < compact_width;
        let mut x = area.x;
        let prev_style = if self.state.page == 0 {
            Style::default().fg(muted)
        } else {
            Style::default().fg(foreground)
        };
        buf.set_string(x, y, format!(" {} ", Icon::ChevronLeft.glyph()), prev_style);
        self.hit_areas
            .push((HitTarget::Previous, Rect::new(x, y, 3, 1)));
        x += 3;

        if compact {
            let label = format!(" {}/{} ", self.state.page + 1, self.total_pages);
            buf.set_string(x, y, &label, Style::default().fg(foreground));
            x += label.chars().count() as u16;
        } else {
            for slot in self.visible_pages() {
                match slot {
                    Some(index) => {
                        let width = self.children[index]
                            .view
                            .measure()
                            .map(|(w, _)| w)
                            .unwrap_or(1)
                            + 2;
                        if x + width > area.x + area.width.saturating_sub(3) {
                            break;
                        }
                        let selected = index == self.state.page;
                        let cell = Rect::new(x, y, width, 1);
                        if selected {
                            let mut style = Style::default().bg(selected_bg).fg(selected_fg);
                            if bold_selected {
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            buf.set_style(cell, style);
                        }
                        let inner = Rect::new(x + 1, y, width.saturating_sub(2), 1);
                        self.children[index].view.render(inner, buf, cx, theme);
                        self.hit_areas.push((HitTarget::Page(index), cell));
                        x = x.saturating_add(width);
                    }
                    None => {
                        let label = format!(" {} ", Icon::Ellipsis.glyph());
                        buf.set_string(x, y, &label, Style::default().fg(muted));
                        x += label.chars().count() as u16;
                    }
                }
            }
        }

        let next_style = if self.state.page + 1 >= self.total_pages {
            Style::default().fg(muted)
        } else {
            Style::default().fg(foreground)
        };
        buf.set_string(
            x,
            y,
            format!(" {} ", Icon::ChevronRight.glyph()),
            next_style,
        );
        self.hit_areas
            .push((HitTarget::Next, Rect::new(x, y, 3, 1)));
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        match event {
            InputEvent::Key(key) => {
                if !focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Left => {
                        self.change(self.step(false));
                        Outcome::Consumed
                    }
                    KeyCode::Right => {
                        self.change(self.step(true));
                        Outcome::Consumed
                    }
                    KeyCode::Home => {
                        self.change(0);
                        Outcome::Consumed
                    }
                    KeyCode::End => {
                        self.change(self.total_pages - 1);
                        Outcome::Consumed
                    }
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                let is_click = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
                if !is_click {
                    return Outcome::Ignored;
                }
                match self.hit_test(mouse.column, mouse.row) {
                    Some(HitTarget::Previous) => {
                        self.change(self.step(false));
                        Outcome::Consumed
                    }
                    Some(HitTarget::Next) => {
                        self.change(self.step(true));
                        Outcome::Consumed
                    }
                    Some(HitTarget::Page(index)) => {
                        self.change(index);
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
        self.change(self.step(true));
        true
    }
}
