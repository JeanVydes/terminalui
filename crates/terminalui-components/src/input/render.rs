//! Painting logic for the input: chrome, text, selection highlight and the
//! configurable caret.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use terminalui_animations::lerp_color;
use terminalui_core::theme::Theme;
use terminalui_core::Context;

use super::super::cursor::CursorShape;
use super::Input;

use super::super::chrome::{draw_box, Border, Chrome, Padding};
use super::super::layout::{row, Align};
use super::super::scroll::{render_scrollbar, Scroll};

pub(super) fn paint(
    input: &mut Input,
    area: Rect,
    buf: &mut Buffer,
    cx: &mut Context,
    theme: &Theme,
) {
    input.state.focused = if input.state.disabled {
        false
    } else {
        cx.is_focused()
    };
    input.state.area = area;
    if area.width == 0 || area.height == 0 {
        return;
    }
    let cursor_color = input.style.cursor.unwrap_or(theme.accent);
    let dimmed = input.state.disabled;
    let default_fg = if dimmed {
        theme.muted
    } else {
        theme.foreground
    };
    let text_color = input.style.foreground.unwrap_or(default_fg);

    let chrome = Chrome {
        background: input.style.background,
        border: input.style.border.map(|border| {
            if input.state.focused {
                Border {
                    color: Some(theme.accent),
                    ..border
                }
            } else {
                border
            }
        }),
        radius: input.style.radius,
        padding: Padding::default(),
        border_gradient: None,
        fill_gradient: None,
    };
    let inner = draw_box(buf, area, &chrome, theme);

    let padding_x = 1u16;
    let text_y = inner.y + inner.height / 2;
    let max_text = inner.width.saturating_sub(padding_x + 1) as usize;
    let value = &input.state.value;

    if value.is_empty() {
        let rect = Rect::new(inner.x + 1, text_y, inner.width.saturating_sub(2), 1);
        row(&mut input.placeholder, rect, Align::Left, 1, buf, cx, theme);
        if input.state.focused && !input.state.disabled {
            let caret_y = if input.multiline { inner.y } else { text_y };
            draw_caret(
                input,
                buf,
                CaretArgs {
                    x: inner.x + 1,
                    y: caret_y,
                    cell_selected: false,
                    theme,
                    accent: cursor_color,
                    text_color,
                },
            );
        }
        return;
    }

    let selected_style = Style::default().bg(theme.primary).fg(theme.background);
    if input.multiline {
        paint_multiline(
            input,
            inner,
            max_text,
            buf,
            theme,
            text_color,
            cursor_color,
            selected_style,
        );
    } else {
        paint_single_line(
            input,
            inner,
            max_text,
            text_y,
            buf,
            theme,
            text_color,
            cursor_color,
            selected_style,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_single_line(
    input: &mut Input,
    inner: Rect,
    max_text: usize,
    text_y: u16,
    buf: &mut Buffer,
    theme: &Theme,
    text_color: Color,
    cursor_color: Color,
    selected_style: Style,
) {
    let chars: Vec<char> = input.state.value.chars().collect();
    let cursor_col = input.cursor_line_col().1;
    let max_scroll = chars.len().saturating_sub(max_text);
    let scroll = if cursor_col >= max_text {
        cursor_col - max_text + 1
    } else if cursor_col < input.state.scroll_x {
        cursor_col
    } else {
        input.state.scroll_x.min(max_scroll)
    };
    input.state.scroll_x = scroll;
    let selection = input.selection();
    let padding_x = 1u16;
    for (visible, index) in (scroll..chars.len()).take(max_text).enumerate() {
        let x = inner.x + padding_x + visible as u16;
        let selected = selection.is_some_and(|(start, end)| start <= index && index < end);
        paint_cell(
            buf,
            x,
            text_y,
            chars[index],
            selected,
            text_color,
            selected_style,
        );
    }
    if !input.state.focused || input.state.disabled {
        return;
    }
    let max_cursor_x = inner.x + inner.width.saturating_sub(1);
    let visible_col = cursor_col.saturating_sub(scroll);
    let cursor_x = (inner.x + padding_x + visible_col as u16).min(max_cursor_x);
    let cell_selected =
        selection.is_some_and(|(start, end)| start <= cursor_col && cursor_col < end);
    draw_caret(
        input,
        buf,
        CaretArgs {
            x: cursor_x,
            y: text_y,
            cell_selected,
            theme,
            accent: cursor_color,
            text_color,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_multiline(
    input: &mut Input,
    inner: Rect,
    max_text: usize,
    buf: &mut Buffer,
    theme: &Theme,
    text_color: Color,
    cursor_color: Color,
    selected_style: Style,
) {
    let inner_height = inner.height as usize;
    let lines: Vec<&str> = input.state.value.split('\n').collect();
    let start = input.state.scroll.min(lines.len().saturating_sub(1));
    let selection = input.selection();
    let mut char_index = input
        .value_slice(0, input.line_start(start))
        .chars()
        .count();
    for (index, line) in lines.iter().enumerate().skip(start) {
        let y = inner.y + (index - start) as u16;
        if y >= inner.y + inner.height {
            break;
        }
        for (visible, (char_offset, ch)) in line.chars().enumerate().take(max_text).enumerate() {
            let x = inner.x + 1 + visible as u16;
            let global = char_index + char_offset;
            let selected = selection.is_some_and(|(start, end)| start <= global && global < end);
            paint_cell(buf, x, y, ch, selected, text_color, selected_style);
        }
        char_index += line.chars().count() + 1;
    }
    let mut scroll = Scroll::new(inner_height, lines.len());
    scroll.offset = input.state.scroll;
    render_scrollbar(buf, inner, &input.style.scrollbar, theme, &scroll);

    if !input.state.focused || input.state.disabled {
        return;
    }
    let (line, col) = input.cursor_line_col();
    if line >= start {
        let y = inner.y + (line - start) as u16;
        let cursor_x = (inner.x + 1 + col as u16).min(inner.x + inner.width.saturating_sub(1));
        let global = input.value_slice(0, input.line_start(line)).chars().count() + col;
        let cell_selected = selection.is_some_and(|(start_, end)| start_ <= global && global < end);
        draw_caret(
            input,
            buf,
            CaretArgs {
                x: cursor_x,
                y,
                cell_selected,
                theme,
                accent: cursor_color,
                text_color,
            },
        );
    }
}

fn paint_cell(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    character: char,
    selected: bool,
    text_color: Color,
    selected_style: Style,
) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(character.to_string().as_str());
        if selected {
            cell.set_style(selected_style);
        } else {
            cell.set_fg(text_color);
        }
    }
}

struct CaretArgs<'a> {
    x: u16,
    y: u16,
    cell_selected: bool,
    theme: &'a Theme,
    accent: Color,
    text_color: Color,
}

fn draw_caret(input: &Input, buf: &mut Buffer, args: CaretArgs<'_>) {
    let style = &input.style.cursor_style;
    let visibility = input.caret_visibility();
    let background = if args.cell_selected {
        args.theme.primary
    } else {
        input.style.background.unwrap_or(args.theme.background)
    };
    let Some(cell) = buf.cell_mut((args.x, args.y)) else {
        return;
    };
    match style.shape {
        CursorShape::Block => {
            cell.set_bg(lerp_color(background, args.accent, visibility));
        }
        CursorShape::Underline => {
            let underline = lerp_color(background, args.accent, visibility);
            cell.set_style(
                Style::default()
                    .underline_color(underline)
                    .add_modifier(Modifier::UNDERLINED),
            );
        }
        CursorShape::Line => {
            let fg = lerp_color(args.text_color, args.accent, visibility);
            cell.set_symbol("▏");
            cell.set_fg(fg);
        }
    }
}
