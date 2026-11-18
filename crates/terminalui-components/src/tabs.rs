//! Tabs: a horizontal tab strip; selected driven by `selected`.
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, View};

use super::layout::{row, Align};
use super::presentation::RenderStyle;

/// A single tab. Its content is composed (usually a [`Text`] view).
#[derive(Serialize)]
pub struct Tab {
    /// Whether the tab is skipped by navigation and renders muted.
    pub disabled: bool,
    children: Vec<Node>,
}

impl Tab {
    /// Compose a tab from arbitrary views.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            disabled: false,
            children,
        }
    }

    fn content_width(&self) -> u16 {
        let measured: u16 = self
            .children
            .iter()
            .filter_map(|child| child.view.measure().map(|(width, _)| width))
            .sum();
        measured + 2
    }
}

impl View for Tab {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "tab"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("disabled={}", self.disabled))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((self.content_width(), 1))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let inner = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1);
        row(&mut self.children, inner, Align::Center, 0, buf, cx, theme);
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

/// Public state of the tabs: area, focus and selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TabsState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the tabs would hold focus by the tree.
    pub focused: bool,
    /// Index of the selected tab.
    pub selected: usize,
}

/// Per-instance overrides for tabs. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TabsStyle {
    /// Highlight background of the selected tab. Defaults to `theme.muted`.
    pub highlight: Option<Color>,
    /// Underline color of the selected tab. Defaults to `theme.accent`.
    pub indicator: Option<Color>,
    /// Cells between tabs. Default 1.
    pub gap: Option<u16>,
    /// Separator glyph drawn on the bottom row. Default `"─"`.
    pub separator: Option<String>,
    /// Separator color. Defaults to `theme.border`.
    pub separator_color: Option<Color>,
    /// Resolve the separator line from this presentation when `separator` is
    /// unset. Default [`RenderStyle::Native`].
    pub presentation: Option<RenderStyle>,
}

/// A tab strip. The selected tab is underlined with accent color; hosts read
/// [`Tabs::selected`] to know which tab is active.
#[derive(Serialize)]
pub struct Tabs {
    /// Per-instance overrides; defaults match the theme.
    pub style: TabsStyle,
    /// Public state: area, focus and selection.
    pub state: TabsState,
    children: Vec<Node>,
    hit_areas: Vec<Rect>,
}

impl Tab {
    /// Convenience constructor: the tab title is a single [`Text`] view.
    pub fn with_title(title: impl Into<String>) -> Self {
        Self::new(vec![node("title", Text::new(title))])
    }

    /// Convenience constructor: a disabled text tab.
    pub fn disabled(title: impl Into<String>) -> Self {
        let mut tab = Self::with_title(title);
        tab.disabled = true;
        tab
    }
}

impl Tabs {
    /// Convenience constructor: each title becomes a [`Tab`] with a [`Text`] child.
    pub fn with_titles(titles: Vec<impl Into<String>>) -> Self {
        let children = titles
            .into_iter()
            .enumerate()
            .map(|(index, title)| node(format!("tab-{index}"), Tab::with_title(title)))
            .collect();
        Self::new(children)
    }
}

impl Tabs {
    /// Compose a tab strip from [`Tab`] nodes.
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            style: TabsStyle::default(),
            state: TabsState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                selected: 0,
            },
            children,
            hit_areas: Vec::new(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: TabsStyle) -> Self {
        self.style = style;
        self
    }

    /// The selected tab index.
    pub fn selected(&self) -> usize {
        self.state.selected
    }

    /// Select a tab by index. Ignored when out of range or disabled.
    pub fn set_selected(&mut self, index: usize) {
        if self.is_disabled(index) || index >= self.children.len() {
            return;
        }
        self.state.selected = index;
    }

    fn is_disabled(&self, index: usize) -> bool {
        self.children
            .get(index)
            .and_then(|child| child.view.as_any_ref().downcast_ref::<Tab>())
            .map(|tab| tab.disabled)
            .unwrap_or(false)
    }

    fn step(&self, from: usize, forward: bool) -> usize {
        if self.children.is_empty() {
            return 0;
        }
        for delta in 1..=self.children.len() {
            let candidate = if forward {
                (from + delta) % self.children.len()
            } else {
                (from + self.children.len() - delta % self.children.len()) % self.children.len()
            };
            if !self.is_disabled(candidate) {
                return candidate;
            }
        }
        from
    }
}

impl View for Tabs {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "tabs"
    }

    fn outline(&self) -> Option<String> {
        Some(format!("selected={}", self.state.selected))
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
        let highlight = self.style.highlight.unwrap_or(theme.muted);
        let indicator = self.style.indicator.unwrap_or(theme.accent);
        let gap = self.style.gap.unwrap_or(1);
        let separator = self.style.separator.clone().unwrap_or_else(|| {
            self.style
                .presentation
                .unwrap_or_default()
                .hairline()
                .0
                .to_string()
        });
        let separator_color = self.style.separator_color.unwrap_or(theme.border);
        let y = area.y;
        let mut x = area.x;
        for (index, child) in self.children.iter_mut().enumerate() {
            let width = child.view.measure().map(|(width, _)| width).unwrap_or(0);
            if width == 0 || x + width > area.x + area.width {
                break;
            }
            let cell = Rect::new(x, y, width, 1);
            if index == self.state.selected {
                buf.set_style(cell, Style::default().bg(highlight));
            }
            child.view.render(cell, buf, cx, theme);
            self.hit_areas.push(cell);
            x = x.saturating_add(width + gap);
        }
        if area.height >= 2 {
            let separator_y = area.y + area.height - 1;
            buf.set_string(
                area.x,
                separator_y,
                separator.repeat(area.width as usize),
                Style::default().fg(separator_color),
            );
            if let Some(cell) = self.hit_areas.get(self.state.selected) {
                buf.set_string(
                    cell.x,
                    separator_y,
                    separator.repeat(cell.width as usize),
                    Style::default().fg(indicator),
                );
            }
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        if self.children.is_empty() {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if !focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Left => {
                        self.state.selected = self.step(self.state.selected, false);
                        Outcome::Consumed
                    }
                    KeyCode::Right => {
                        self.state.selected = self.step(self.state.selected, true);
                        Outcome::Consumed
                    }
                    KeyCode::Home => {
                        self.state.selected = self.step(0, true);
                        Outcome::Consumed
                    }
                    KeyCode::End => {
                        self.state.selected = self.step(self.children.len() - 1, false);
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
                let hit = self
                    .hit_areas
                    .iter()
                    .position(|rect| rect.contains(Position::new(mouse.column, mouse.row)));
                match hit {
                    Some(index) if !self.is_disabled(index) => {
                        self.state.selected = index;
                        Outcome::Consumed
                    }
                    _ => Outcome::Ignored,
                }
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.state.selected = self.step(self.state.selected, true);
        true
    }
}
