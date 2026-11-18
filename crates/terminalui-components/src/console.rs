//! Console: a shell log block.
//!
//! Renders a `$ command` header and the command's output lines on a filled
//! surface, like a terminal inside the UI. Used for tool shell logs, build
//! output and REPL panes. While the command is `Running` the header shows a
//! spinner; when it exits the header reports the exit code.
//!
//! Output beyond `max_visible` rows scrolls; the view clamps its offset and
//! draws a scrollbar when the output overflows.

use crossterm::event::{KeyCode, KeyEventKind, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::chrome::{draw_box, Chrome, Padding, Radius};
use super::scroll::{render_scrollbar, Scroll, ScrollbarStyle};

const DEFAULT_MAX_VISIBLE: usize = 8;
const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
        }
    }

    /// Cap the number of output rows shown at once. Defaults to 8.
    pub fn with_max_visible(mut self, rows: usize) -> Self {
        self.max_visible = rows.max(1);
        self.state.offset = self.clamp_offset(self.state.offset);
        self
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

    fn visible(&self) -> usize {
        self.lines.len().min(self.max_visible)
    }

    fn max_offset(&self) -> usize {
        self.lines.len().saturating_sub(self.visible())
    }

    fn clamp_offset(&self, offset: usize) -> usize {
        offset.min(self.max_offset())
    }

    fn scroll_rows(&mut self, delta: isize) {
        let next = self.state.offset as isize + delta;
        self.state.offset = next.clamp(0, self.max_offset() as isize) as usize;
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
        let rows = self.visible().max(1);
        Some((0, 1 + rows as u16))
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

        let command = theme.foreground;
        let accent = theme.accent;
        let output = self.style.output.unwrap_or(theme.muted);
        let command_color = self.style.command.unwrap_or(command);

        let header_style = Style::default().bg(chrome.background.unwrap_or(theme.secondary));
        buf.set_style(Rect { height: 1, ..inner }, header_style);
        buf.set_stringn(
            inner.x,
            inner.y,
            "$ ",
            2,
            Style::default()
                .fg(accent)
                .bg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        );
        let command_text: String = self
            .command
            .chars()
            .take(inner.width.saturating_sub(6) as usize)
            .collect();
        buf.set_stringn(
            inner.x + 2,
            inner.y,
            &command_text,
            command_text.len(),
            Style::default()
                .fg(command_color)
                .bg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        );

        let status_text = match self.status {
            ConsoleStatus::Running => {
                SPINNER_FRAMES[self.state.frame % SPINNER_FRAMES.len()].to_string()
            }
            ConsoleStatus::Exited { code } => match code {
                Some(0) => "done".to_string(),
                Some(code) => format!("exit {code}"),
                None => "done".to_string(),
            },
        };
        let status_color = match self.status {
            ConsoleStatus::Running => accent,
            ConsoleStatus::Exited { code } => match code {
                Some(0) | None => theme.success,
                Some(_) => theme.destructive,
            },
        };
        let status_width = status_text.chars().count() as u16;
        let status_x = inner.x + inner.width.saturating_sub(status_width);
        buf.set_stringn(
            status_x,
            inner.y,
            &status_text,
            status_width as usize,
            Style::default()
                .fg(status_color)
                .bg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        );

        let output_area = Rect {
            y: inner.y + 1,
            height: inner.height - 1,
            ..inner
        };
        for y in output_area.y..output_area.y + output_area.height {
            buf.set_stringn(
                output_area.x,
                y,
                " ".repeat(output_area.width as usize),
                output_area.width as usize,
                Style::default().bg(theme.secondary),
            );
        }
        if self.lines.is_empty() {
            buf.set_stringn(
                output_area.x,
                output_area.y,
                "waiting for output...",
                output_area.width as usize,
                Style::default()
                    .fg(output)
                    .bg(theme.secondary)
                    .add_modifier(Modifier::ITALIC),
            );
            return;
        }

        let len = self.lines.len();
        let start = len.saturating_sub(self.state.offset + self.visible());
        let end = start + self.visible();
        for (row, index) in (start..end).enumerate() {
            let line = &self.lines[index];
            let truncated: String = line.chars().take(output_area.width as usize).collect();
            buf.set_stringn(
                output_area.x,
                output_area.y + row as u16,
                &truncated,
                truncated.len(),
                Style::default().fg(output).bg(theme.secondary),
            );
        }

        if self.lines.len() > self.max_visible {
            let scroll = Scroll {
                offset: self.max_offset().saturating_sub(self.state.offset),
                viewport: self.max_visible,
                total: self.lines.len(),
            };
            render_scrollbar(buf, output_area, &ScrollbarStyle::default(), theme, &scroll);
        }
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
        self.lines.len() > self.max_visible
    }
}
