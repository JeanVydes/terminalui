//! Tests for the chrome box machinery: Blank glyph sets, gradients and
//! the shared gradient projections.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use terminalui_components::chrome::{
    draw_box, Border, BorderSet, Chrome, Gradient, Padding, Radius,
};
use terminalui_core::theme::Theme;

fn theme() -> Theme {
    Theme::default()
}

#[test]
fn blank_border_reserves_the_rect_without_drawing_glyphs() {
    let area = Rect::new(0, 0, 10, 4);
    let chrome = Chrome {
        background: Some(Color::Rgb(10, 10, 20)),
        border: Some(Border {
            set: BorderSet::Blank,
            color: None,
            radius: Radius::None,
        }),
        radius: Radius::None,
        padding: Padding::default(),
        border_gradient: None,
        fill_gradient: None,
    };
    let theme = theme();
    let mut buffer = Buffer::empty(area);
    let inner = draw_box(&mut buffer, area, &chrome, &theme);

    assert_eq!(inner, Rect::new(1, 1, 8, 2), "border offsets still apply");
    for y in 0..area.height {
        for x in 0..area.width {
            assert_eq!(buffer[(x, y)].symbol(), " ", "cell {x},{y} is blank");
            assert_eq!(
                buffer[(x, y)].bg,
                Color::Rgb(10, 10, 20),
                "cell {x},{y} keeps the fill"
            );
        }
    }
}

#[test]
fn border_gradient_tints_cells_along_the_axis() {
    let area = Rect::new(0, 0, 8, 4);
    let chrome = Chrome {
        background: Some(Color::Rgb(0, 0, 0)),
        border: Some(Border {
            set: BorderSet::Plain,
            color: None,
            radius: Radius::None,
        }),
        radius: Radius::None,
        padding: Padding::default(),
        border_gradient: Some(Gradient {
            start: Color::Rgb(255, 0, 0),
            end: Color::Rgb(0, 0, 255),
            angle: 0.0,
        }),
        fill_gradient: None,
    };
    let theme = theme();
    let mut buffer = Buffer::empty(area);
    draw_box(&mut buffer, area, &chrome, &theme);

    let left = buffer[(0, 1)].fg;
    let right = buffer[(7, 1)].fg;
    let middle = buffer[(3, 0)].fg;
    assert_ne!(left, right, "0° sweeps left to right");
    assert_ne!(left, middle);
    assert_ne!(right, middle);
    assert_eq!(buffer[(0, 0)].symbol(), "┌");
}

#[test]
fn border_gradient_at_90_degrees_sweeps_top_to_bottom() {
    let area = Rect::new(0, 0, 6, 5);
    let chrome = Chrome {
        background: Some(Color::Rgb(0, 0, 0)),
        border: Some(Border {
            set: BorderSet::Plain,
            color: None,
            radius: Radius::None,
        }),
        radius: Radius::None,
        padding: Padding::default(),
        border_gradient: Some(Gradient {
            start: Color::Rgb(255, 0, 0),
            end: Color::Rgb(0, 0, 255),
            angle: 90.0,
        }),
        fill_gradient: None,
    };
    let theme = theme();
    let mut buffer = Buffer::empty(area);
    draw_box(&mut buffer, area, &chrome, &theme);

    let top = buffer[(2, 0)].fg;
    let bottom = buffer[(2, 4)].fg;
    assert_ne!(top, bottom, "90° sweeps top to bottom");
}

#[test]
fn fill_gradient_replaces_the_solid_fill_and_tints_every_cell() {
    let area = Rect::new(0, 0, 6, 4);
    let chrome = Chrome {
        background: Some(Color::Rgb(200, 200, 200)),
        border: None,
        radius: Radius::None,
        padding: Padding::default(),
        border_gradient: None,
        fill_gradient: Some(Gradient {
            start: Color::Rgb(0, 0, 0),
            end: Color::Rgb(255, 255, 255),
            angle: 45.0,
        }),
    };
    let theme = theme();
    let mut buffer = Buffer::empty(area);
    draw_box(&mut buffer, area, &chrome, &theme);

    // No cell keeps the old solid background.
    for y in 0..area.height {
        for x in 0..area.width {
            assert_ne!(
                buffer[(x, y)].bg,
                Color::Rgb(200, 200, 200),
                "cell {x},{y} uses the gradient"
            );
        }
    }
}

#[test]
fn themed_chrome_uses_theme_defaults_and_respects_overrides() {
    let theme = Theme::default();
    let padding = Padding {
        horizontal: 2,
        vertical: 1,
    };

    let defaults = Chrome::themed(&theme, None, None, padding);
    let border = defaults.border.unwrap_or_default();
    assert_eq!(border.set, BorderSet::Rounded);
    assert_eq!(border.radius, Radius::Small);
    assert_eq!(border.color, Some(theme.border));
    assert_eq!(defaults.background, None);
    assert_eq!(defaults.padding, padding);

    let border_color = Color::Red;
    let background = Color::Blue;
    let overrides = Chrome::themed(&theme, Some(border_color), Some(background), padding);
    assert_eq!(
        overrides.border.unwrap_or_default().color,
        Some(border_color)
    );
    assert_eq!(overrides.background, Some(background));
}

#[test]
fn opaque_fills_erase_underlying_glyphs_but_transparent_boxes_preserve_them() {
    let area = Rect::new(0, 0, 8, 4);
    let rect = Rect::new(2, 1, 4, 2);
    for gradient in [false, true] {
        let mut buffer = Buffer::empty(area);
        for y in 0..area.height {
            buffer.set_string(0, y, "XXXXXXXX", ratatui::style::Style::default());
        }
        let mut chrome = Chrome::filled(Color::Black);
        if gradient {
            chrome.fill_gradient = Some(Gradient {
                start: Color::Black,
                end: Color::White,
                angle: 0.0,
            });
        }
        draw_box(&mut buffer, rect, &chrome, &theme());
        assert_eq!(buffer[(2, 1)].symbol(), " ");
        assert_eq!(buffer[(5, 2)].symbol(), " ");
        assert_eq!(buffer[(1, 1)].symbol(), "X");
        assert_eq!(buffer[(6, 1)].symbol(), "X");
    }
    let mut buffer = Buffer::empty(area);
    buffer.set_string(0, 1, "XXXXXXXX", ratatui::style::Style::default());
    draw_box(&mut buffer, rect, &Chrome::default(), &theme());
    assert_eq!(buffer[(2, 1)].symbol(), "X");
}

#[test]
fn floating_toast_hides_underlying_text_only_within_its_box() {
    use terminalui_components::toast::{Toast, ToastKind};
    use terminalui_core::{node, Tree};
    let area = Rect::new(0, 0, 70, 12);
    let mut buffer = Buffer::empty(area);
    for y in 0..area.height {
        buffer.set_string(0, y, "X".repeat(70), ratatui::style::Style::default());
    }
    let mut tree = Tree::new(node("toast", Toast::new(ToastKind::Info, "saved")));
    tree.render(area, &mut buffer, &theme());
    assert_eq!(buffer[(0, 11)].symbol(), "X");
    assert_eq!(buffer[(40, 10)].symbol(), " ");
}

#[test]
fn dialog_can_leave_its_surroundings_visible() {
    use terminalui_components::dialog::Dialog;
    use terminalui_core::{node, Tree};
    let area = Rect::new(0, 0, 80, 20);
    let mut buffer = Buffer::empty(area);
    for y in 0..area.height {
        buffer.set_string(0, y, "X".repeat(80), ratatui::style::Style::default());
    }
    let mut dialog = Dialog::new(Vec::new())
        .without_overlay()
        .with_title("Hello");
    dialog.open();
    let mut tree = Tree::new(node("dialog", dialog));
    tree.render(area, &mut buffer, &theme());
    assert_eq!(buffer[(0, 0)].symbol(), "X");
    assert_eq!(buffer[(35, 9)].symbol(), " ");
}
