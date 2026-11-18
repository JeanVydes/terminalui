//! A hierarchical tree. Nodes are host-provided data with one label each;
//! the component owns expansion, cursor, scroll and hit-testing.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::scroll::{clamp_view, render_scrollbar, Scroll, ScrollbarStyle};

/// One host-provided node of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeNode {
    pub label: String,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            expanded: false,
            children: Vec::new(),
        }
    }

    pub fn branch(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        Self {
            label: label.into(),
            expanded: false,
            children,
        }
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    fn is_branch(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeViewState {
    pub area: Rect,
    /// Index into the visible (flattened) rows.
    pub cursor: usize,
    /// First visible row when the tree overflows.
    pub offset: usize,
    pub focused: bool,
}

/// Per-instance overrides for a tree view. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TreeViewStyle {
    pub label: Option<Color>,
    pub selected_fg: Option<Color>,
    pub selected_bg: Option<Color>,
    /// Disclosure glyph color. Defaults to `theme.muted`.
    pub disclosure: Option<Color>,
    /// Bold the highlighted row. Default true.
    pub bold: Option<bool>,
    pub scrollbar: Option<ScrollbarStyle>,
}

struct FlatRow<'a> {
    path: Vec<usize>,
    depth: usize,
    node: &'a TreeNode,
}

/// A keyboard-navigable tree. Left/Right collapse/expand branches,
/// Enter/Space toggle them, arrows move the cursor, and clicking a
/// branch disclosure toggles it.
#[derive(Serialize)]
pub struct TreeView {
    pub state: TreeViewState,
    pub style: TreeViewStyle,
    nodes: Vec<TreeNode>,
}

impl TreeView {
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        Self {
            state: TreeViewState {
                area: Rect::new(0, 0, 0, 0),
                cursor: 0,
                offset: 0,
                focused: false,
            },
            style: TreeViewStyle::default(),
            nodes,
        }
    }

    pub fn with_style(mut self, style: TreeViewStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the whole data set. Expansion flags come with the data.
    pub fn set_nodes(&mut self, nodes: Vec<TreeNode>) {
        self.nodes = nodes;
        self.state.cursor = 0;
        self.state.offset = 0;
    }

    /// Path of labels from the root to the cursored node, if any.
    pub fn selected_path(&self) -> Option<Vec<String>> {
        let flat = self.flatten();
        let row = flat.get(self.state.cursor)?;
        let mut labels = Vec::with_capacity(row.path.len());
        let mut current = self.nodes.as_slice();
        for &index in &row.path {
            let node = current.get(index)?;
            labels.push(node.label.clone());
            current = &node.children;
        }
        Some(labels)
    }

    /// Label of the cursored node, if the tree is not empty.
    pub fn selected_label(&self) -> Option<&str> {
        self.flatten()
            .get(self.state.cursor)
            .map(|row| row.node.label.as_str())
    }

    fn flatten(&self) -> Vec<FlatRow<'_>> {
        let mut out = Vec::new();
        flatten_into(&self.nodes, Vec::new(), 0, &mut out);
        out
    }

    fn visible_len(&self) -> usize {
        self.flatten().len()
    }

    fn body_height(&self) -> usize {
        self.state.area.height as usize
    }

    fn clamp_view(&mut self) {
        let total = self.visible_len();
        let (cursor, offset) = clamp_view(
            self.state.cursor,
            self.state.offset,
            self.body_height(),
            total,
        );
        self.state.cursor = cursor;
        self.state.offset = offset;
    }

    fn move_cursor(&mut self, index: usize) -> Outcome {
        let total = self.visible_len();
        if total == 0 {
            return Outcome::Ignored;
        }
        self.state.cursor = index.min(total - 1);
        self.clamp_view();
        Outcome::Consumed
    }

    fn node_at_mut(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        let mut current = &mut self.nodes;
        for (depth, &index) in path.iter().enumerate() {
            let node = current.get_mut(index)?;
            if depth + 1 == path.len() {
                return Some(node);
            }
            current = &mut node.children;
        }
        None
    }

    fn toggle_at(&mut self, path: &[usize]) -> Outcome {
        match self.node_at_mut(path) {
            Some(node) if node.is_branch() => {
                node.expanded = !node.expanded;
                self.clamp_view();
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    /// Right expands a collapsed branch; Left collapses it or jumps to the
    /// parent row. Anything else is ignored.
    fn collapse_expand(&mut self, expand: bool) -> Outcome {
        let flat = self.flatten();
        let Some(row) = flat.get(self.state.cursor) else {
            return Outcome::Ignored;
        };
        let path = row.path.clone();
        let is_branch = row.node.is_branch();
        let expanded = row.node.expanded;
        if expand && is_branch && !expanded {
            return self.toggle_at(&path);
        }
        if !expand && is_branch && expanded {
            return self.toggle_at(&path);
        }
        if !expand && path.len() > 1 {
            let parent = &path[..path.len() - 1];
            let flat = self.flatten();
            if let Some(index) = flat.iter().position(|row| row.path == parent) {
                return self.move_cursor(index);
            }
        }
        Outcome::Ignored
    }
}

fn flatten_into<'a>(
    nodes: &'a [TreeNode],
    path: Vec<usize>,
    depth: usize,
    out: &mut Vec<FlatRow<'a>>,
) {
    for (index, node) in nodes.iter().enumerate() {
        let mut row_path = path.clone();
        row_path.push(index);
        out.push(FlatRow {
            path: row_path.clone(),
            depth,
            node,
        });
        if node.expanded {
            flatten_into(&node.children, row_path, depth + 1, out);
        }
    }
}

impl Default for TreeView {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View for TreeView {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "tree_view"
    }

    fn outline(&self) -> Option<String> {
        Some(format!(
            "cursor={}, offset={}",
            self.state.cursor, self.state.offset
        ))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        self.state.focused = cx.is_focused();
        self.state.area = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.clamp_view();
        let label_color = self.style.label.unwrap_or(theme.foreground);
        let selected_fg = self.style.selected_fg.unwrap_or(theme.background);
        let selected_bg = self.style.selected_bg.unwrap_or(theme.primary);
        let disclosure_color = self.style.disclosure.unwrap_or(theme.muted);
        let bold = self.style.bold.unwrap_or(true);
        let flat = self.flatten();
        let visible = self.body_height();
        let overflow = flat.len() > visible;
        let row_width = if overflow {
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        for visible_index in 0..visible {
            let index = self.state.offset + visible_index;
            let Some(row) = flat.get(index) else {
                break;
            };
            let y = area.y + visible_index as u16;
            let indent = area.x + (row.depth as u16) * 2;
            let selected = index == self.state.cursor;
            let row_rect = Rect::new(area.x, y, row_width, 1);
            let mut row_style =
                Style::default().fg(if selected { selected_fg } else { label_color });
            if selected {
                row_style = row_style.bg(selected_bg);
                if bold {
                    row_style = row_style.add_modifier(Modifier::BOLD);
                }
            }
            if selected {
                buf.set_style(row_rect, row_style);
            }
            let disclosure = if row.node.is_branch() {
                if row.node.expanded {
                    "▾"
                } else {
                    "▸"
                }
            } else {
                " "
            };
            let disclosure_style = if selected {
                row_style
            } else {
                Style::default().fg(disclosure_color)
            };
            buf.set_string(indent, y, disclosure, disclosure_style);
            let label_x = indent + 2;
            let label_width = row_width.saturating_sub((row.depth as u16) * 2 + 2);
            buf.set_stringn(label_x, y, &row.node.label, label_width as usize, row_style);
        }
        if overflow {
            let style = self.style.scrollbar.clone().unwrap_or_default();
            let mut scroll = Scroll::new(visible, flat.len());
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
        self.state.focused = cx.is_focused();
        match event {
            InputEvent::Key(key) => {
                if !self.state.focused || key.kind != KeyEventKind::Press {
                    return Outcome::Ignored;
                }
                let page = self.body_height().max(1);
                match key.code {
                    KeyCode::Up => self.move_cursor(self.state.cursor.saturating_sub(1)),
                    KeyCode::Down => self.move_cursor(self.state.cursor + 1),
                    KeyCode::PageUp => self.move_cursor(self.state.cursor.saturating_sub(page)),
                    KeyCode::PageDown => self.move_cursor(self.state.cursor + page),
                    KeyCode::Home => self.move_cursor(0),
                    KeyCode::End => self.move_cursor(usize::MAX),
                    KeyCode::Right => self.collapse_expand(true),
                    KeyCode::Left => self.collapse_expand(false),
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let flat = self.flatten();
                        let Some(row) = flat.get(self.state.cursor) else {
                            return Outcome::Ignored;
                        };
                        let path = row.path.clone();
                        if row.node.is_branch() {
                            self.toggle_at(&path)
                        } else {
                            Outcome::Consumed
                        }
                    }
                    _ => Outcome::Ignored,
                }
            }
            InputEvent::Mouse(mouse) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Outcome::Ignored;
                }
                if mouse.row < self.state.area.y
                    || mouse.row >= self.state.area.y.saturating_add(self.state.area.height)
                    || mouse.column < self.state.area.x
                    || mouse.column >= self.state.area.x.saturating_add(self.state.area.width)
                {
                    return Outcome::Ignored;
                }
                let visible_index = (mouse.row - self.state.area.y) as usize;
                let index = self.state.offset + visible_index;
                let flat = self.flatten();
                let Some(row) = flat.get(index) else {
                    return Outcome::Ignored;
                };
                let indent = self.state.area.x + (row.depth as u16) * 2;
                let path = row.path.clone();
                let is_branch = row.node.is_branch();
                if is_branch && mouse.column >= indent && mouse.column < indent + 2 {
                    return self.toggle_at(&path);
                }
                self.move_cursor(index)
            }
            _ => Outcome::Ignored,
        }
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        let flat = self.flatten();
        let Some(row) = flat.get(self.state.cursor) else {
            return false;
        };
        let path = row.path.clone();
        if row.node.is_branch() {
            self.toggle_at(&path);
        }
        true
    }
}
