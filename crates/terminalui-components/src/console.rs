//! Console: a shell log block.
//!
//! Renders a `$ command` header and the command's output lines on a filled
//! surface, like a terminal inside the UI. Used for tool shell logs, build
//! output and REPL panes. While the command is `Running` the header shows a
//! spinner; when it exits the header reports the exit code.
//!
//! Output beyond `max_visible` rows scrolls; the view clamps its offset and
//! draws a scrollbar when the output overflows. The header shows the live line
//! count and toggles between the collapsed window and every retained line.

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;
use unicode_width::UnicodeWidthChar;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::chrome::{draw_box, Chrome, Padding, Radius};
use super::scroll::{render_scrollbar, Scroll, ScrollbarStyle};

const DEFAULT_MAX_VISIBLE: usize = 8;
const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut rows = Vec::new();
    let mut row = String::new();
    let mut cells = 0usize;
    for character in line.chars() {
        let size = character.width().unwrap_or(0).max(1);
        if cells + size > width && !row.is_empty() {
            rows.push(std::mem::take(&mut row));
            cells = 0;
        }
        row.push(character);
        cells += size;
    }
    if !row.is_empty() {
        rows.push(row);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

struct HeaderSlots {
    status_x: u16,
    status_text: String,
    status_color: Color,
    count_x: u16,
    count_text: String,
    marker_x: u16,
    marker_text: &'static str,
    show_details: bool,
}

/// Status of a console block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ConsoleStatus {
    /// The command is still running; the header animates a spinner.
    #[default]
    Running,
    /// The command finished.
    Exited {
        /// The exit code, when the host knows it.
        code: Option<i32>,
    },
}

/// Public state of the console: area, scroll offset and the spinner frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsoleState {
    /// Last rendered area, set during render.
    pub area: Rect,
    /// Index of the first visible output row when the output overflows.
    pub offset: usize,
    /// Current spinner frame; advanced once per tick while `Running`.
    pub frame: usize,
    /// Time spent in [`ConsoleStatus::Running`]; drives the spinner.
    pub elapsed: std::time::Duration,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            area: Rect::new(0, 0, 0, 0),
            offset: 0,
            frame: 0,
            elapsed: std::time::Duration::ZERO,
        }
    }
}

/// Per-instance overrides for a console. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ConsoleStyle {
    /// Header command color. Defaults to `theme.foreground`.
    pub command: Option<Color>,
    /// Output text color. Defaults to `theme.muted`.
    pub output: Option<Color>,
    /// Chrome override. Defaults to a filled, rounded surface box.
    pub chrome: Option<Chrome>,
}

/// A shell log block: `$ command` header, output rows on a filled surface.
#[derive(Serialize)]
pub struct Console {
    /// The command as typed.
    pub command: String,
    /// Run state; `Running` animates the header spinner.
    pub status: ConsoleStatus,
    /// Public state: area, scroll offset and spinner frame.
    pub state: ConsoleState,
    /// Per-instance overrides; defaults match the theme.
    pub style: ConsoleStyle,
    lines: Vec<String>,
    max_visible: usize,
    expanded: bool,
    #[serde(skip)]
    content_width: usize,
}

impl Console {
    /// Creates a console with the given command, initially [`ConsoleStatus::Running`].
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            status: ConsoleStatus::Running,
            state: ConsoleState::default(),
            style: ConsoleStyle::default(),
            lines: Vec::new(),
            max_visible: DEFAULT_MAX_VISIBLE,
            expanded: false,
            content_width: 0,
        }
    }

    /// Cap the number of output rows shown at once. Defaults to 8.
    pub fn with_max_visible(mut self, rows: usize) -> Self {
        self.max_visible = rows.max(1);
        self.state.offset = self.clamp_offset(self.state.offset);
        self
    }

    /// Choose whether every retained line is shown instead of a window.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self.state.offset = self.clamp_offset(self.state.offset);
        self
    }

    /// Whether every retained line is shown.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Height the console needs at `width`, counting wrapped output rows.
    ///
    /// Hosts that lay the console out before rendering should use this instead
    /// of [`View::measure`], which reports the last rendered width.
    pub fn wrapped_height(&self, width: u16) -> u16 {
        let rows = self.visible_rows(usize::from(width)).max(1);
        u16::try_from(rows.saturating_add(1)).unwrap_or(u16::MAX)
    }

    /// Flip between the collapsed window and the full output.
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        self.state.offset = self.clamp_offset(self.state.offset);
    }

    /// Replace the output lines entirely; scroll resets to the tail.
    pub fn set_lines(mut self, lines: Vec<String>) -> Self {
        self.lines = lines;
        self.state.offset = 0;
        self
    }

    /// Builder shorthand: set the max visible rows.
    pub fn with_style(mut self, style: ConsoleStyle) -> Self {
        self.style = style;
        self
    }

    /// Append a line to the output; trims nothing.
    pub fn push_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        self.state.offset = self.clamp_offset(self.state.offset);
    }

    /// Mark the command finished with an exit code; `None` when unknown.
    pub fn set_exit(&mut self, code: Option<i32>) {
        self.status = ConsoleStatus::Exited { code };
    }

    /// `true` while the command is still running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, ConsoleStatus::Running)
    }

    /// The output lines currently held.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    fn wrap_width(&self) -> usize {
        self.content_width
    }

    fn wrapped_lines(&self, width: usize) -> Vec<String> {
        if width == 0 {
            return self.lines.clone();
        }
        self.lines
            .iter()
            .flat_map(|line| wrap_line(line, width))
            .collect()
    }

    fn visible_rows(&self, width: usize) -> usize {
        let total = if self.lines.is_empty() {
            0
        } else {
            self.wrapped_lines(width).len()
        };
        if self.expanded {
            total
        } else {
            total.min(self.max_visible)
        }
    }

    fn max_offset(&self) -> usize {
        self.wrapped_lines(self.wrap_width())
            .len()
            .saturating_sub(self.visible_rows(self.wrap_width()))
    }

    fn clamp_offset(&self, offset: usize) -> usize {
        offset.min(self.max_offset())
    }

    fn scroll_rows(&mut self, delta: isize) {
        let next = self.state.offset as isize + delta;
        self.state.offset = next.clamp(0, self.max_offset() as isize) as usize;
    }

    fn status_label(&self, theme: &Theme) -> (String, Color) {
        match self.status {
            ConsoleStatus::Running => (
                SPINNER_FRAMES[self.state.frame % SPINNER_FRAMES.len()].to_string(),
                theme.accent,
            ),
            ConsoleStatus::Exited { code } => (
                match code {
                    Some(0) | None => "done".to_string(),
                    Some(code) => format!("exit {code}"),
                },
                match code {
                    Some(0) | None => theme.success,
                    Some(_) => theme.destructive,
                },
            ),
        }
    }

    fn header_slots(&self, inner: Rect, theme: &Theme) -> HeaderSlots {
        let (status_text, status_color) = self.status_label(theme);
        let count_text = format!("{} lines", self.lines.len());
        let marker_text = if self.expanded { "[-]" } else { "[+]" };
        let gap = 1u16;
        let status_width = u16::try_from(status_text.chars().count()).unwrap_or(u16::MAX);
        let count_width = u16::try_from(count_text.chars().count()).unwrap_or(u16::MAX);
        let details_width = status_width
            .saturating_add(gap)
            .saturating_add(count_width)
            .saturating_add(gap)
            .saturating_add(3);
        let show_details = inner.width >= details_width.saturating_add(3);
        let status_x = if show_details {
            inner.x + inner.width.saturating_sub(details_width)
        } else {
            inner.x + inner.width.saturating_sub(status_width)
        };
        HeaderSlots {
            status_x,
            status_text,
            status_color,
            count_x: status_x + status_width + gap,
            count_text,
            marker_x: status_x + status_width + gap + count_width + gap,
            marker_text,
            show_details,
        }
    }

    fn paint_slot(
        buf: &mut Buffer,
        x: u16,
        y: u16,
        text: &str,
        color: Color,
        background: Color,
        bold: bool,
    ) {
        let mut style = Style::default().fg(color).bg(background);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_stringn(x, y, text, text.len(), style);
    }

    fn paint_header(&self, inner: Rect, buf: &mut Buffer, theme: &Theme, background: Color) {
        let slots = self.header_slots(inner, theme);
        buf.set_style(Rect { height: 1, ..inner }, Style::default().bg(background));
        Self::paint_slot(buf, inner.x, inner.y, "$ ", theme.accent, background, true);
        self.paint_command(inner, buf, theme, background, &slots);
        Self::paint_slot(
            buf,
            slots.status_x,
            inner.y,
            &slots.status_text,
            slots.status_color,
            background,
            true,
        );
        if !slots.show_details {
            return;
        }
        Self::paint_slot(
            buf,
            slots.count_x,
            inner.y,
            &slots.count_text,
            theme.muted,
            background,
            false,
        );
        Self::paint_slot(
            buf,
            slots.marker_x,
            inner.y,
            slots.marker_text,
            theme.accent,
            background,
            false,
        );
    }

    fn paint_command(
        &self,
        inner: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        background: Color,
        slots: &HeaderSlots,
    ) {
        let command_width = slots.status_x.saturating_sub(inner.x + 2).saturating_sub(1);
        let command_text: String = self
            .command
            .chars()
            .take(usize::from(command_width))
            .collect();
        Self::paint_slot(
            buf,
            inner.x + 2,
            inner.y,
            &command_text,
            self.style.command.unwrap_or(theme.foreground),
            background,
            true,
        );
    }

    fn paint_output(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let output = self.style.output.unwrap_or(theme.muted);
        Self::paint_output_background(area, buf, theme.secondary);
        if self.lines.is_empty() {
            Self::paint_placeholder(area, buf, output, theme.secondary);
            return;
        }
        self.paint_lines(area, buf, output, theme.secondary);
        self.paint_scrollbar(area, buf, theme);
    }

    fn paint_output_background(area: Rect, buf: &mut Buffer, background: Color) {
        for y in area.y..area.y + area.height {
            buf.set_stringn(
                area.x,
                y,
                " ".repeat(area.width as usize),
                area.width as usize,
                Style::default().bg(background),
            );
        }
    }

    fn paint_placeholder(area: Rect, buf: &mut Buffer, output: Color, background: Color) {
        buf.set_stringn(
            area.x,
            area.y,
            "waiting for output...",
            area.width as usize,
            Style::default()
                .fg(output)
                .bg(background)
                .add_modifier(Modifier::ITALIC),
        );
    }

    fn paint_lines(&self, area: Rect, buf: &mut Buffer, output: Color, background: Color) {
        let width = usize::from(area.width);
        let wrapped = self.wrapped_lines(width);
        let visible_rows = self.visible_rows(width);
        let shown = visible_rows.min(usize::from(area.height));
        let start = wrapped
            .len()
            .saturating_sub(self.state.offset + visible_rows);
        for (row, text) in wrapped.iter().skip(start).take(shown).enumerate() {
            buf.set_stringn(
                area.x,
                area.y + row as u16,
                text,
                text.len(),
                Style::default().fg(output).bg(background),
            );
        }
    }

    fn paint_scrollbar(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if self.expanded {
            return;
        }
        let width = usize::from(area.width);
        let total = self.wrapped_lines(width).len();
        if total <= self.max_visible {
            return;
        }
        let scroll = Scroll {
            offset: total
                .saturating_sub(self.visible_rows(width))
                .saturating_sub(self.state.offset),
            viewport: self.max_visible,
            total,
        };
        render_scrollbar(buf, area, &ScrollbarStyle::default(), theme, &scroll);
    }
}

impl View for Console {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "console"
    }

    fn outline(&self) -> Option<String> {
        let status = match self.status {
            ConsoleStatus::Running => "running".to_string(),
            ConsoleStatus::Exited { code } => match code {
                Some(code) => format!("exit-{code}"),
                None => "exited".to_string(),
            },
        };
        Some(format!(
            "command={} status={} rows={}",
            quoted(&self.command),
            status,
            self.lines.len()
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        Some((
            0,
            self.wrapped_height(u16::try_from(self.content_width).unwrap_or(u16::MAX)),
        ))
    }

    fn children(&self) -> &[terminalui_core::Node] {
        &[]
    }

    fn children_mut(&mut self) -> &mut [terminalui_core::Node] {
        &mut []
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        self.state.area = area;
        self.state.offset = self.clamp_offset(self.state.offset);

        let chrome = self.style.chrome.clone().unwrap_or(Chrome {
            border: None,
            background: Some(theme.secondary),
            radius: Radius::Small,
            padding: Padding {
                horizontal: 1,
                vertical: 0,
            },
            border_gradient: None,
            fill_gradient: None,
        });
        let inner = draw_box(buf, area, &chrome, theme);
        if inner.height == 0 || inner.width < 4 {
            return;
        }
        self.content_width = usize::from(inner.width);
        self.state.offset = self.clamp_offset(self.state.offset);

        self.paint_header(
            inner,
            buf,
            theme,
            chrome.background.unwrap_or(theme.secondary),
        );
        let output_area = Rect {
            y: inner.y + 1,
            height: inner.height - 1,
            ..inner
        };
        self.paint_output(output_area, buf, theme);
    }

    fn tick(&mut self, delta: std::time::Duration, _cx: &mut Context) {
        if self.is_running() {
            self.state.elapsed = self.state.elapsed.saturating_add(delta);
            self.state.frame =
                (self.state.elapsed.as_millis() / 80) as usize % SPINNER_FRAMES.len();
        }
    }

    fn handle_event(&mut self, event: &InputEvent, _cx: &mut Context) -> Outcome {
        let InputEvent::Key(key) = event else {
            if let InputEvent::Mouse(mouse) = event {
                let header_click = mouse.kind == MouseEventKind::Down(MouseButton::Left)
                    && self.state.area.width > 0
                    && mouse.row == self.state.area.y
                    && mouse.column >= self.state.area.x
                    && mouse.column < self.state.area.x + self.state.area.width;
                if header_click {
                    self.toggle_expanded();
                    return Outcome::Consumed;
                }
                if mouse.kind == MouseEventKind::ScrollUp {
                    self.scroll_rows(-3);
                    return Outcome::Consumed;
                }
                if mouse.kind == MouseEventKind::ScrollDown {
                    self.scroll_rows(3);
                    return Outcome::Consumed;
                }
            }
            return Outcome::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Enter => {
                self.toggle_expanded();
                Outcome::Consumed
            }
            KeyCode::Up => {
                self.scroll_rows(-1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.scroll_rows(1);
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.scroll_rows(-(self.max_visible as isize));
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.scroll_rows(self.max_visible as isize);
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.state.offset = self.max_offset();
                Outcome::Consumed
            }
            KeyCode::End => {
                self.state.offset = 0;
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        self.wrapped_lines(self.wrap_width()).len() > self.max_visible
    }
}
