//! Select: a dropdown trigger that opens a list of options.
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Node, Outcome, View};

use super::chrome::{draw_box, Border, BorderSet, Chrome, Padding, Radius};
use super::icons::Icon;
use super::scroll::{render_scrollbar, Scroll, ScrollbarStyle};

/// Per-instance overrides for a select. Defaults match the theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectStyle {
    /// Background of the trigger and the popup. Defaults to `theme.background`.
    pub background: Option<Color>,
    /// Trigger background when the popup is open. Defaults to `theme.secondary`.
    pub open_background: Option<Color>,
    /// Border of the trigger and the popup. Defaults to a line border.
    pub border: Option<Border>,
    /// Corner radius of the trigger and the popup. Default: none.
    pub radius: Radius,
    /// Highlighted option background inside the popup. Defaults to `theme.primary`.
    pub highlight_bg: Option<Color>,
    /// Highlighted option foreground. Defaults to `theme.background`.
    pub highlight_fg: Option<Color>,
    /// Chevron color. Defaults to `theme.muted`.
    pub chevron: Option<Color>,
    /// Disabled foreground. Defaults to `theme.muted`.
    pub disabled: Option<Color>,
    /// Maximum options visible in the popup before it scrolls. Default 8.
    pub max_visible: u16,
    /// Scrollbar for the popup. Defaults to auto visibility.
    pub scrollbar: ScrollbarStyle,
}

impl Default for SelectStyle {
    fn default() -> Self {
        Self {
            background: None,
            open_background: None,
            border: Some(Border {
                set: BorderSet::Plain,
                color: None,
                radius: Radius::None,
            }),
            radius: Radius::None,
            highlight_bg: None,
            highlight_fg: None,
            chevron: None,
            disabled: None,
            max_visible: 8,
            scrollbar: ScrollbarStyle::default(),
        }
    }
}

/// Public state of the select: area, focus, disabled, open and highlight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Whether the select would hold focus by the tree.
    pub focused: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
    /// Whether the dropdown is open.
    pub open: bool,
    /// Index of the highlighted option while the popup is open.
    pub highlight: usize,
    /// First visible option index (scroll offset).
    pub offset: usize,
}

/// A select dropdown with composed option views.
#[derive(Serialize)]
pub struct Select {
    /// Composed options; each one is arbitrary views (usually text + icon).
    /// Serialized as `children` so every node exposes a uniform child key.
    #[serde(rename = "children")]
    pub options: Vec<Node>,
    /// Index of the selected option. `None` when nothing is selected yet.
    pub selected: Option<usize>,
    /// Public state: area, focus, disabled, open and highlight.
    pub state: SelectState,
    /// Per-instance overrides; defaults match the theme.
    pub style: SelectStyle,
    trigger_hit: Rect,
    option_hits: Vec<Rect>,
}

impl Select {
    /// Compose a select from arbitrary option views.
    pub fn new(options: Vec<Node>) -> Self {
        Self {
            options,
            selected: None,
            state: SelectState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                disabled: false,
                open: false,
                highlight: 0,
                offset: 0,
            },
            style: SelectStyle::default(),
            trigger_hit: Rect::new(0, 0, 0, 0),
            option_hits: Vec::new(),
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: SelectStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder shorthand: disable or enable interaction.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    /// Builder shorthand: pre-select the given option index.
    pub fn with_selected(mut self, selected: usize) -> Self {
        if let Some(index) = (selected < self.options.len()).then_some(selected) {
            self.selected = Some(index);
            self.state.highlight = index;
        }
        self
    }

    /// The selected option index, if any.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Select an option by index. `None` clears the selection.
    pub fn set_selected(&mut self, selected: Option<usize>) {
        self.selected = selected.filter(|index| *index < self.options.len());
        self.state.highlight = self.selected.unwrap_or(0);
    }

    /// Replace the composed options entirely.
    pub fn set_options(&mut self, options: Vec<Node>) {
        self.options = options;
        self.selected = self.selected.filter(|index| *index < self.options.len());
        self.state.highlight = self.selected.unwrap_or(0);
        self.state.offset = 0;
    }

    /// Whether the dropdown is open.
    pub fn is_open(&self) -> bool {
        self.state.open
    }

    /// Toggle the dropdown; a disabled or empty select stays closed.
    pub fn toggle_open(&mut self) {
        if self.state.disabled || self.options.is_empty() {
            return;
        }
        self.state.open = !self.state.open;
        if self.state.open {
            self.state.highlight = self.selected.unwrap_or(0);
            self.clamp_popup();
        }
    }

    /// Open the dropdown; a disabled or empty select stays closed.
    pub fn open(&mut self) {
        if self.state.disabled || self.options.is_empty() {
            return;
        }
        self.state.open = true;
        self.state.highlight = self.selected.unwrap_or(0);
        self.clamp_popup();
    }

    /// Close the dropdown.
    pub fn close(&mut self) {
        self.state.open = false;
    }

    fn option_width(&self) -> u16 {
        self.options
            .iter()
            .filter_map(|node| node.view.measure())
            .map(|(width, _)| width)
            .max()
            .unwrap_or(8)
            .max(8)
    }

    fn popup_height(&self) -> u16 {
        let visible = self.options.len().min(self.style.max_visible as usize) as u16;
        let vertical = u16::from(self.style.border.is_some()) * 2;
        visible + vertical
    }

    fn chrome(&self, theme: &Theme) -> Chrome {
        Chrome {
            background: Some(self.style.background.unwrap_or(theme.background)),
            border: self.style.border,
            radius: self.style.radius,
            padding: Padding::default(),
            border_gradient: None,
            fill_gradient: None,
        }
    }

    fn popup_chrome(&self, theme: &Theme) -> Chrome {
        Chrome {
            background: Some(self.style.open_background.unwrap_or(theme.secondary)),
            ..self.chrome(theme)
        }
    }

    fn clamp_popup(&mut self) {
        let max_offset = self
            .options
            .len()
            .saturating_sub(self.style.max_visible as usize);
        let highlight = self.state.highlight;
        if highlight < self.state.offset {
            self.state.offset = highlight;
        } else if highlight >= self.state.offset + self.style.max_visible as usize {
            self.state.offset = highlight + 1 - self.style.max_visible as usize;
        }
        self.state.offset = self.state.offset.min(max_offset);
    }

    /// The rect the popup renders into, anchored to the trigger. `bounds`
    /// are the frame buffer dimensions: the popup may cover sibling views,
    /// but never leaves the screen.
    fn popup_rect(&self, bounds: Rect) -> Rect {
        let width = self.option_width().max(self.trigger_hit.width).max(8);
        let height = self.popup_height();
        let trigger_bottom = self.trigger_hit.y.saturating_add(self.trigger_hit.height);
        let space_below = bounds.y + bounds.height.saturating_sub(trigger_bottom);
        let space_above = self.trigger_hit.y.saturating_sub(bounds.y);
        let y = if space_below >= height || space_below >= space_above {
            trigger_bottom.min(bounds.y + bounds.height.saturating_sub(height))
        } else {
            self.trigger_hit.y.saturating_sub(height).max(bounds.y)
        };
        let max_x = bounds.x + bounds.width.saturating_sub(width);
        let x = self.trigger_hit.x.min(max_x);
        Rect::new(x, y, width, height)
    }
}

impl View for Select {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "select"
    }

    fn outline(&self) -> Option<String> {
        let selected = self
            .selected
            .map(|index| index.to_string())
            .unwrap_or_else(|| "none".to_string());
        Some(format!(
            "selected={selected}, open={}, disabled={}",
            self.state.open, self.state.disabled
        ))
    }

    fn children(&self) -> &[Node] {
        &self.options
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.options
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let width = self.option_width().max(8) + 4;
        Some((width, 1))
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = if self.state.disabled {
            false
        } else {
            cx.is_focused()
        };
        self.state.area = area;
        let disabled = self.state.disabled;
        let highlight_bg = self.style.highlight_bg.unwrap_or(theme.primary);
        let highlight_fg = self.style.highlight_fg.unwrap_or(theme.background);
        let chevron_color = self.style.chevron.unwrap_or(theme.muted);
        let chrome = self.chrome(theme);
        let trigger = Rect::new(area.x, area.y, area.width, 1);
        let inner = draw_box(buf, trigger, &chrome, theme);
        let trigger_bg = if self.state.open {
            self.style.open_background.unwrap_or(theme.secondary)
        } else {
            self.style.background.unwrap_or(theme.background)
        };
        buf.set_style(inner, Style::default().bg(trigger_bg));
        let chevron = if self.state.open {
            Icon::ChevronUp.glyph()
        } else {
            Icon::ChevronDown.glyph()
        };
        let chevron_x = inner.x + inner.width.saturating_sub(1);
        buf.set_string(
            chevron_x,
            inner.y,
            chevron,
            Style::default().fg(if disabled { theme.muted } else { chevron_color }),
        );
        // Trigger content: the selected option, or nothing when unselected.
        let content = Rect::new(inner.x, inner.y, inner.width.saturating_sub(2).max(1), 1);
        let selected_view = self.selected.filter(|index| *index < self.options.len());
        if let Some(index) = selected_view {
            // Render the selected option's view in the trigger.
            if let Some(node) = self.options.get_mut(index) {
                node.view.render(content, buf, cx, theme);
            }
        }
        self.trigger_hit = trigger;

        if !self.state.open {
            self.option_hits.clear();
            return;
        }
        // Popup.
        let popup = self.popup_rect(buf.area);
        let popup_inner = draw_box(buf, popup, &self.popup_chrome(theme), theme);
        let scroll = Scroll::new(popup_inner.height as usize, self.options.len());
        let max_offset = scroll.max_offset();
        self.state.offset = self.state.offset.min(max_offset);
        let scroll_offset = self.state.offset;
        self.option_hits.clear();
        let visible = popup_inner.height;
        for visible_index in 0..visible as usize {
            let option_index = scroll_offset + visible_index;
            if option_index >= self.options.len() {
                break;
            }
            let y = popup_inner.y + visible_index as u16;
            let row = Rect::new(popup_inner.x, y, popup_inner.width, 1);
            let highlighted = option_index == self.state.highlight;
            if highlighted {
                buf.set_style(row, Style::default().bg(highlight_bg).fg(highlight_fg));
            }
            if let Some(node) = self.options.get_mut(option_index) {
                node.view.render(row, buf, cx, theme);
            }
            self.option_hits.push(row);
        }
        // Scrollbar.
        let mut xscroll = Scroll::new(visible as usize, self.options.len());
        xscroll.offset = scroll_offset;
        render_scrollbar(buf, popup_inner, &self.style.scrollbar, theme, &xscroll);
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        self.state.focused = if self.state.disabled {
            false
        } else {
            cx.is_focused()
        };
        if self.state.disabled {
            return Outcome::Ignored;
        }
        let max_highlight = self.options.len().saturating_sub(1);
        match event {
            InputEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                if !self.state.focused {
                    return Outcome::Ignored;
                }
                match key.code {
                    KeyCode::Down => {
                        if self.state.open {
                            self.state.highlight = (self.state.highlight + 1).min(max_highlight);
                            self.clamp_popup();
                        } else {
                            self.open();
                        }
                        Outcome::Consumed
                    }
                    KeyCode::Up => {
                        if self.state.open {
                            self.state.highlight = self.state.highlight.saturating_sub(1);
                            self.clamp_popup();
                        }
                        Outcome::Consumed
                    }
                    KeyCode::Home if self.state.open => {
                        self.state.highlight = 0;
                        self.clamp_popup();
                        Outcome::Consumed
                    }
                    KeyCode::End if self.state.open => {
                        self.state.highlight = max_highlight;
                        self.clamp_popup();
                        Outcome::Consumed
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if self.state.open {
                            self.selected = Some(self.state.highlight);
                            self.state.open = false;
                        } else {
                            self.open();
                        }
                        Outcome::Consumed
                    }
                    KeyCode::Esc if self.state.open => {
                        self.state.open = false;
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
                let in_rect = |rect: Rect| rect.contains(Position::new(mouse.column, mouse.row));
                if in_rect(self.trigger_hit) {
                    self.toggle_open();
                    return Outcome::Consumed;
                }
                if self.state.open {
                    if let Some(index) = self
                        .option_hits
                        .iter()
                        .position(|rect| in_rect(*rect))
                        .map(|offset| self.state.offset + offset)
                    {
                        if index < self.options.len() {
                            self.selected = Some(index);
                            self.state.highlight = index;
                            self.state.open = false;
                            return Outcome::Consumed;
                        }
                    }
                    self.state.open = false;
                    return Outcome::Consumed;
                }
                Outcome::Ignored
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        !self.state.disabled
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.toggle_open();
        true
    }
}
