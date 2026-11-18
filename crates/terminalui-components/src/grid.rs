use super::presentation::RenderStyle;
use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use serde::Serialize;
use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

const DEFAULT_MIN_SIZE: u16 = 3;

/// Axis along which the panes are laid out.
///
/// `Row` places panes side by side with vertical dividers (`│`).
/// `Column` stacks panes with horizontal dividers (`─`).
/// Nesting two grids gives a two-dimensional resizable grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GridAxis {
    Row,
    Column,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GridState {
    pub area: Rect,
    pub focused: bool,
    /// Divider the keyboard resizes. Zero-based, at most `panes - 2`.
    pub divider: usize,
    /// Divider being dragged with the mouse, if any.
    pub dragging: Option<usize>,
}

/// Per-instance overrides for a grid. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GridStyle {
    /// Divider color when idle. Defaults to `theme.border`.
    pub divider: Option<Color>,
    /// Divider color when focused/dragging. Defaults to `theme.ring`.
    pub active: Option<Color>,
    /// Glyph for vertical dividers (`Row` axis). Default `"│"`.
    pub row_char: Option<String>,
    /// Glyph for horizontal dividers (`Column` axis). Default `"─"`.
    pub column_char: Option<String>,
    /// Resolve divider glyphs from this presentation when `row_char`/`column_char`
    /// are unset. Default [`RenderStyle::Native`].
    pub presentation: Option<RenderStyle>,
}

#[derive(Serialize)]
pub struct ResizableGrid {
    pub axis: GridAxis,
    /// Relative weights of each pane.
    pub weights: Vec<u16>,
    /// Minimum size (cells) of each pane.
    pub min_sizes: Vec<u16>,
    pub state: GridState,
    pub style: GridStyle,
    /// Anchor of the active mouse drag.
    drag_anchor: u16,
    /// One composed view per pane, laid out in order. Children beyond
    /// `weights.len()` are not rendered.
    children: Vec<terminalui_core::Node>,
}

impl ResizableGrid {
    fn with_axis(axis: GridAxis, weights: Vec<u16>) -> Self {
        let weights = if weights.is_empty() {
            vec![1, 1]
        } else {
            weights
        };
        let panes = weights.len();
        Self {
            axis,
            weights,
            min_sizes: vec![DEFAULT_MIN_SIZE; panes],
            state: GridState {
                area: Rect::new(0, 0, 0, 0),
                focused: false,
                divider: 0,
                dragging: None,
            },
            style: GridStyle::default(),
            drag_anchor: 0,
            children: Vec::new(),
        }
    }

    pub fn with_style(mut self, style: GridStyle) -> Self {
        self.style = style;
        self
    }

    /// Panes laid out horizontally (side by side).
    pub fn row(weights: Vec<u16>) -> Self {
        Self::with_axis(GridAxis::Row, weights)
    }

    /// Panes laid out vertically (stacked).
    pub fn column(weights: Vec<u16>) -> Self {
        Self::with_axis(GridAxis::Column, weights)
    }

    /// Compose the pane contents, one child per pane in declaration order.
    pub fn panes(mut self, children: Vec<terminalui_core::Node>) -> Self {
        self.children = children;
        self
    }

    pub fn with_min_sizes(mut self, min_sizes: Vec<u16>) -> Self {
        self.min_sizes = min_sizes
            .into_iter()
            .take(self.weights.len())
            .chain(std::iter::repeat(DEFAULT_MIN_SIZE))
            .take(self.weights.len())
            .collect();
        self
    }

    /// Current pane lengths (in cells) derived from the weights and the
    /// current area. Defaults to the zero-sized area before the first render.
    pub fn pane_lengths(&self) -> Vec<u16> {
        self.pane_lengths_for(self.axis_length(self.state.area))
    }

    /// Resize the divider at `index` by `delta` cells. Positive `delta`
    /// grows the left/top pane at the expense of the next one. Returns
    /// false when the index is out of range or the minimums forbid movement.
    /// This is the same operation the keyboard and the mouse wheel perform.
    pub fn resize_divider(&mut self, index: usize, delta: i32) -> bool {
        self.resize_divider_at(index, delta)
    }

    /// Set the left/top pane of divider `index` to exactly `size` cells.
    /// Clamped to the per-pane minimums.
    pub fn set_divider_size(&mut self, index: usize, size: u16) -> bool {
        let axis_len = self.axis_length(self.state.area);
        if axis_len == 0 || index >= self.divider_count() {
            return false;
        }
        let current = self.pane_lengths_for(axis_len);
        let current_size = current.get(index).copied().unwrap_or(0);
        self.resize_divider_at(index, size as i32 - current_size as i32)
    }

    /// Replace the pane weights and rescale the layout. Sizes stay clamped
    /// to the configured per-pane minimums.
    pub fn set_weights(&mut self, weights: Vec<u16>) {
        let weights = if weights.is_empty() {
            vec![1, 1]
        } else {
            weights
        };
        let panes = weights.len();
        self.weights = weights;
        let previous = std::mem::take(&mut self.min_sizes);
        self.min_sizes = previous
            .into_iter()
            .take(panes)
            .chain(std::iter::repeat(DEFAULT_MIN_SIZE))
            .take(panes)
            .collect();
        self.state.divider = self.state.divider.min(panes.saturating_sub(2));
        self.state.dragging = None;
    }

    /// Computes the rectangle of each pane for a given area.
    ///
    /// Children should render into these rects; the grid only draws the
    /// dividers between them.
    pub fn pane_areas(&self, area: Rect) -> Vec<Rect> {
        let lengths = self.pane_lengths_for(self.axis_length(area));
        let mut rects = Vec::with_capacity(lengths.len());
        match self.axis {
            GridAxis::Row => {
                let mut x = area.x;
                for length in lengths {
                    rects.push(Rect::new(x, area.y, length, area.height));
                    x += length + 1;
                }
            }
            GridAxis::Column => {
                let mut y = area.y;
                for length in lengths {
                    rects.push(Rect::new(area.x, y, area.width, length));
                    y += length + 1;
                }
            }
        }
        rects
    }

    /// Absolute position (column for `Row`, row for `Column`) of each divider.
    pub fn divider_positions(&self, area: Rect) -> Vec<u16> {
        let mut positions = Vec::with_capacity(self.weights.len().saturating_sub(1));
        match self.axis {
            GridAxis::Row => {
                let mut x = area.x;
                let lengths = self.pane_lengths_for(area.width);
                for (index, length) in lengths.iter().enumerate() {
                    if index + 1 < lengths.len() {
                        positions.push(x + length);
                    }
                    x += length + 1;
                }
            }
            GridAxis::Column => {
                let mut y = area.y;
                let lengths = self.pane_lengths_for(area.height);
                for (index, length) in lengths.iter().enumerate() {
                    if index + 1 < lengths.len() {
                        positions.push(y + length);
                    }
                    y += length + 1;
                }
            }
        }
        positions
    }

    fn axis_length(&self, area: Rect) -> u16 {
        match self.axis {
            GridAxis::Row => area.width,
            GridAxis::Column => area.height,
        }
    }

    fn axis_position(&self, event_column: u16, event_row: u16) -> u16 {
        match self.axis {
            GridAxis::Row => event_column,
            GridAxis::Column => event_row,
        }
    }

    fn divider_count(&self) -> usize {
        self.weights.len().saturating_sub(1)
    }

    /// Current length of each pane, derived from weights and `axis_len`.
    fn pane_lengths_for(&self, axis_len: u16) -> Vec<u16> {
        allocate(
            axis_len,
            &self.weights,
            &self.min_sizes,
            self.divider_count() as u16,
        )
    }

    /// Moves the boundary of `divider` by `delta` cells (positive grows the
    /// left/top pane at the expense of the next one). Returns true if moved.
    fn resize_divider_at(&mut self, divider: usize, delta: i32) -> bool {
        let axis_len = self.axis_length(self.state.area);
        if axis_len == 0 || divider >= self.divider_count() {
            return false;
        }
        let mut sizes = self.pane_lengths_for(axis_len);
        let next = divider + 1;
        let min_left = self.min_sizes[divider] as i32;
        let min_right = self.min_sizes[next] as i32;
        let left = sizes[divider] as i32;
        let right = sizes[next] as i32;
        let delta = delta.clamp(-(left - min_left), right - min_right);
        if delta == 0 {
            return false;
        }
        sizes[divider] = (left + delta) as u16;
        sizes[next] = (right - delta) as u16;
        // Sum is unchanged, so storing sizes as weights makes the next layout
        // reproduce exactly these sizes.
        self.weights = sizes;
        true
    }

    fn hit_divider(&self, pos: u16) -> Option<usize> {
        self.divider_positions(self.state.area)
            .iter()
            .enumerate()
            .find(|(_, position)| pos.abs_diff(**position) <= 1)
            .map(|(index, _)| index)
    }
}

/// Distributes `axis_len - dividers` cells proportionally to weights, then
/// enforces per-pane minimums by stealing from the largest donors.
fn allocate(axis_len: u16, weights: &[u16], min_sizes: &[u16], dividers: u16) -> Vec<u16> {
    let n = weights.len();
    if n == 0 {
        return Vec::new();
    }
    let total_min: u16 = min_sizes.iter().take(n).sum();
    let avail = axis_len.saturating_sub(dividers);
    if total_min > avail {
        // Not enough room for minimums: spread evenly.
        let base = avail / n as u16;
        let mut sizes = vec![base; n];
        for size in sizes.iter_mut().take((avail % n as u16) as usize) {
            *size += 1;
        }
        return sizes;
    }
    let weight_sum: u32 = weights.iter().map(|weight| *weight as u32).sum();
    let avail_u32 = avail as u32;
    let mut sizes: Vec<u16> = weights
        .iter()
        .map(|weight| ((avail_u32 * *weight as u32) / weight_sum) as u16)
        .collect();
    let assigned: u32 = sizes.iter().map(|size| *size as u32).sum();
    let mut remainder = avail_u32 - assigned;
    let mut index = 0usize;
    while remainder > 0 {
        sizes[index % n] += 1;
        remainder -= 1;
        index += 1;
    }
    // Enforce minimums by stealing from the largest donors.
    while let Some(deficit) = (0..n).find(|&index| sizes[index] < min_sizes[index]) {
        let need = min_sizes[deficit] - sizes[deficit];
        let Some(donor) = (0..n)
            .filter(|&index| index != deficit)
            .max_by_key(|&index| sizes[index].saturating_sub(min_sizes[index]))
        else {
            break;
        };
        let take = need.min(sizes[donor].saturating_sub(min_sizes[donor]));
        if take == 0 {
            sizes[deficit] = min_sizes[deficit];
            break;
        }
        sizes[donor] -= take;
        sizes[deficit] += take;
    }
    sizes
}

impl View for ResizableGrid {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn kind(&self) -> &'static str {
        "grid"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "axis={}, panes={}, focused_divider={}",
            quoted(&format!("{:?}", self.axis)),
            self.weights.len(),
            self.state.divider
        ))
    }

    fn children(&self) -> &[terminalui_core::Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [terminalui_core::Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        self.state.focused = cx.is_focused();
        let positions = self.divider_positions(area);
        let divider_color = self.style.divider.unwrap_or(theme.border);
        let active_color = self.style.active.unwrap_or(theme.ring);
        let (hairline_h, hairline_v) = self.style.presentation.unwrap_or_default().hairline();
        let row_char = self
            .style
            .row_char
            .clone()
            .unwrap_or_else(|| hairline_v.to_string());
        let column_char = self
            .style
            .column_char
            .clone()
            .unwrap_or_else(|| hairline_h.to_string());
        for (index, position) in positions.iter().enumerate() {
            let active = self.state.dragging == Some(index)
                || (self.state.focused && self.state.divider == index);
            let style = if active {
                Style::default().fg(active_color)
            } else {
                Style::default().fg(divider_color)
            };
            match self.axis {
                GridAxis::Row => {
                    let divider = Rect::new(*position, area.y, 1, area.height);
                    buf.set_style(divider, style);
                    for y in area.y..area.y + area.height {
                        buf.set_string(*position, y, &row_char, style);
                    }
                }
                GridAxis::Column => {
                    let divider = Rect::new(area.x, *position, area.width, 1);
                    buf.set_style(divider, style);
                    let line = column_char
                        .chars()
                        .cycle()
                        .take(area.width as usize)
                        .collect::<String>();
                    buf.set_string(area.x, *position, line, style);
                }
            }
        }
        let panes = self.pane_areas(area);
        for (index, child) in self.children.iter_mut().enumerate() {
            let Some(pane) = panes.get(index).copied() else {
                break;
            };
            child.view.render(pane, buf, cx, theme);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, cx: &mut Context) -> Outcome {
        let focused = cx.is_focused();
        self.state.focused = focused;
        // Direct manipulation never needs the keyboard focus: wheel over a
        // divider resizes it, and press/drag/release captures the mouse.
        if let InputEvent::Mouse(mouse) = event {
            match mouse.kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let position = self.axis_position(mouse.column, mouse.row);
                    let Some(divider) = self.hit_divider(position) else {
                        return Outcome::Ignored;
                    };
                    let delta = if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                        2
                    } else {
                        -2
                    };
                    return match self.resize_divider(divider, delta) {
                        true => Outcome::Consumed,
                        false => Outcome::Ignored,
                    };
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let position = self.axis_position(mouse.column, mouse.row);
                    if let Some(index) = self.hit_divider(position) {
                        self.state.divider = index;
                        self.state.dragging = Some(index);
                        self.drag_anchor = position;
                    }
                    return Outcome::Ignored;
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    let Some(index) = self.state.dragging else {
                        return Outcome::Ignored;
                    };
                    let position = self.axis_position(mouse.column, mouse.row);
                    let delta = position as i32 - self.drag_anchor as i32;
                    if delta == 0 {
                        return Outcome::Ignored;
                    }
                    return match self.resize_divider(index, delta) {
                        true => {
                            self.drag_anchor = position;
                            Outcome::Consumed
                        }
                        false => Outcome::Ignored,
                    };
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.state.dragging = None;
                    return Outcome::Ignored;
                }
                _ => {}
            }
        }
        if !focused {
            return Outcome::Ignored;
        }
        match event {
            InputEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                let last_divider = self.divider_count();
                match key.code {
                    KeyCode::Left if self.axis == GridAxis::Row => {
                        if self.resize_divider_at(self.state.divider, -1) {
                            Outcome::Consumed
                        } else {
                            Outcome::Ignored
                        }
                    }
                    KeyCode::Right if self.axis == GridAxis::Row => {
                        if self.resize_divider_at(self.state.divider, 1) {
                            Outcome::Consumed
                        } else {
                            Outcome::Ignored
                        }
                    }
                    KeyCode::Up if self.axis == GridAxis::Column => {
                        if self.resize_divider_at(self.state.divider, -1) {
                            Outcome::Consumed
                        } else {
                            Outcome::Ignored
                        }
                    }
                    KeyCode::Down if self.axis == GridAxis::Column => {
                        if self.resize_divider_at(self.state.divider, 1) {
                            Outcome::Consumed
                        } else {
                            Outcome::Ignored
                        }
                    }
                    KeyCode::Char('[') => {
                        self.state.divider = self.state.divider.saturating_sub(1);
                        Outcome::Ignored
                    }
                    KeyCode::Char(']') => {
                        if last_divider > 0 {
                            self.state.divider = (self.state.divider + 1).min(last_divider - 1);
                        }
                        Outcome::Ignored
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
        true
    }
}
