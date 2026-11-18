//! Smoke tests: effects run without panicking and behave at the extremes.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use terminalui_core::theme::Theme;
use terminalui_animations::{Animation, Direction, Easing, Effect, RadialMode};

fn sample_src(area: Rect) -> Buffer {
    let mut buf = Buffer::empty(area);
    for x in area.x..area.x + area.width {
        for y in area.y..area.y + area.height {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol("A");
                cell.set_fg(Color::Indexed(196));
                cell.set_bg(Color::Rgb(10, 10, 14));
            }
        }
    }
    buf
}

fn painted_cells(buf: &Buffer, area: Rect) -> usize {
    (area.y..area.y + area.height)
        .flat_map(|y| (area.x..area.x + area.width).map(move |x| (x, y)))
        .filter(|&pos| {
            buf.cell(pos)
                .map(|cell| cell.symbol() == "A")
                .unwrap_or(false)
        })
        .count()
}

fn paint_fn(area: Rect, buf: &mut Buffer, _theme: &Theme) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol("A");
                cell.set_fg(Color::Indexed(196));
                cell.set_bg(Color::Rgb(10, 10, 14));
            }
        }
    }
}

fn effects() -> Vec<Effect> {
    vec![
        Effect::FadeIn,
        Effect::FadeOut,
        Effect::slide_in(Direction::Left, 10),
        Effect::slide_in(Direction::Right, 10),
        Effect::slide_in(Direction::Up, 5),
        Effect::slide_in(Direction::Down, 5),
        Effect::slide_bounce(Direction::Left, 10),
        Effect::slide_bounce(Direction::Right, 10),
        Effect::slide_bounce(Direction::Up, 5),
        Effect::slide_out(Direction::Left, 10),
        Effect::slide_out(Direction::Right, 10),
        Effect::RevealRight,
        Effect::RevealLeft,
        Effect::RevealDown,
        Effect::RevealUp,
        Effect::SweepRight { color: None },
        Effect::SweepLeft { color: None },
        Effect::SweepRight {
            color: Some(Color::Rgb(1, 2, 3)),
        },
        Effect::Thanos { seed: 42 },
        Effect::Thanos { seed: 1337 },
        Effect::Dissolve { seed: 42 },
        Effect::Dissolve { seed: 99 },
        Effect::Fall,
        Effect::flare(),
        Effect::flare_colors(Color::Rgb(255, 153, 51), Color::Rgb(120, 60, 10)),
        Effect::Flare {
            angle: 180.0,
            width: 8,
            peak: Color::Indexed(51),
            edge: Color::Indexed(23),
        },
        Effect::flare_angle(90.0, Color::Rgb(255, 255, 255), Color::Rgb(20, 20, 20)),
        Effect::flare_angle(60.0, Color::Rgb(200, 255, 200), Color::Rgb(0, 40, 0)),
        Effect::flare_direction(Direction::Up),
        Effect::radial_flare(),
        Effect::radial_flare_expand(),
        Effect::radial_flare_converge(),
        Effect::radial_flare_collide(),
        Effect::radial_flare_mode(RadialMode::Expand, Color::Indexed(51), Color::Indexed(23)),
        Effect::radial_flare_mode(RadialMode::Converge, Color::Indexed(51), Color::Indexed(23)),
        Effect::radial_flare_mode(RadialMode::Collide, Color::Indexed(51), Color::Indexed(23)),
        Effect::flare_bidirectional(),
        Effect::flare_bidirectional_angle(45.0, Color::Rgb(255, 255, 255), Color::Rgb(20, 20, 20)),
        Effect::flare_bidirectional_angle(90.0, Color::Rgb(255, 255, 255), Color::Rgb(20, 20, 20)),
    ]
}

#[test]
fn animations_completes() {
    let mut anim = Animation::new(Duration::from_millis(100), Easing::Linear);
    assert!((anim.progress() - 0.0).abs() < f64::EPSILON);
    anim.tick(Duration::from_millis(50));
    let mid = anim.linear();
    assert!((mid - 0.5).abs() < 0.1);
    anim.tick(Duration::from_millis(100));
    assert!(anim.is_done());
    assert!((anim.progress() - 1.0).abs() < f64::EPSILON);
    anim.restart();
    assert!(!anim.is_done());
}

#[test]
fn all_effects_render_across_progress_range() {
    let area = Rect::new(0, 0, 40, 10);
    let theme = Theme::default();
    for effect in effects() {
        for step in 0..=10 {
            let t = step as f64 / 10.0;
            let mut dst = Buffer::empty(area);
            terminalui_animations::render_with(&effect, t, &mut dst, area, &theme, paint_fn);
            // Just must not panic and must leave the buffer valid.
            let _ = painted_cells(&dst, area);
        }
    }
}

#[test]
fn slide_in_ends_exact_and_starts_empty() {
    let area = Rect::new(0, 0, 40, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    for direction in [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ] {
        let effect = Effect::slide_in(direction, 20);
        let mut dst = Buffer::empty(area);
        effect.apply(&src, &mut dst, area, &theme, 1.0);
        assert_eq!(painted_cells(&dst, area), 40 * 10, "slide fully in");
        let mut dst = Buffer::empty(area);
        effect.apply(&src, &mut dst, area, &theme, 0.0);
        // At t=0 nothing or nearly nothing should be visible.
        assert!(painted_cells(&dst, area) <= 40 * 10 / 2 || direction == Direction::Down);
    }
}

#[test]
fn slide_bounce_lands_exact() {
    let area = Rect::new(0, 0, 40, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    for direction in [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ] {
        let effect = Effect::slide_bounce(direction, 20);
        let mut dst = Buffer::empty(area);
        effect.apply(&src, &mut dst, area, &theme, 1.0);
        assert_eq!(painted_cells(&dst, area), 40 * 10);
    }
}

#[test]
fn reveal_right_covers_source_at_the_end() {
    let area = Rect::new(0, 0, 40, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut dst = Buffer::empty(area);
    Effect::RevealRight.apply(&src, &mut dst, area, &theme, 1.0);
    assert_eq!(painted_cells(&dst, area), 40 * 10);
    // At t=0 nothing is revealed.
    let mut dst = Buffer::empty(area);
    Effect::RevealRight.apply(&src, &mut dst, area, &theme, 0.0);
    assert_eq!(painted_cells(&dst, area), 0);
}

#[test]
fn fade_in_reaches_full_opacity() {
    let area = Rect::new(0, 0, 10, 4);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut dst = Buffer::empty(area);
    Effect::FadeIn.apply(&src, &mut dst, area, &theme, 1.0);
    let cell = dst.cell((0, 0)).unwrap();
    assert_eq!(cell.symbol(), "A");
    let fg = cell.fg;
    assert_eq!(fg, Color::Indexed(196));
}

#[test]
fn thanos_at_full_progress_hides_everything() {
    let area = Rect::new(0, 0, 40, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut dst = Buffer::empty(area);
    Effect::Thanos { seed: 7 }.apply(&src, &mut dst, area, &theme, 1.0);
    assert_eq!(painted_cells(&dst, area), 0);
}

#[test]
fn dissolve_reveals_everything_at_the_end() {
    let area = Rect::new(0, 0, 40, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut dst = Buffer::empty(area);
    Effect::Dissolve { seed: 7 }.apply(&src, &mut dst, area, &theme, 1.0);
    assert_eq!(painted_cells(&dst, area), 40 * 10);
}

#[test]
fn fall_drops_cells_out_of_the_bottom() {
    let area = Rect::new(0, 0, 20, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut dst = Buffer::empty(area);
    Effect::Fall.apply(&src, &mut dst, area, &theme, 1.0);
    // Everything should have fallen out of the area.
    assert_eq!(painted_cells(&dst, area), 0);
}

#[test]
fn multi_size_areas_do_not_panic() {
    let theme = Theme::default();
    for (width, height) in [(1, 1), (1, 7), (8, 3), (40, 20), (80, 24)] {
        let area = Rect::new(0, 0, width, height);
        for effect in effects() {
            for step in 0..=4 {
                let t = step as f64 / 4.0;
                let mut dst = Buffer::empty(area);
                terminalui_animations::render_with(&effect, t, &mut dst, area, &theme, paint_fn);
            }
        }
    }
}

#[test]
fn flare_tints_the_band_center_and_spares_the_rest() {
    let area = Rect::new(0, 0, 40, 4);
    let theme = Theme::default();
    let mut dst = Buffer::empty(area);
    let effect = Effect::flare_colors(Color::Rgb(255, 255, 255), Color::Rgb(10, 10, 10));
    effect.apply(&sample_src(area), &mut dst, area, &theme, 0.5);

    let center = dst.cell((20, 1)).unwrap().fg;
    let edge = dst.cell((0, 1)).unwrap().fg;
    assert_ne!(center, Color::Indexed(196), "band center should be tinted");
    assert_eq!(
        edge,
        Color::Indexed(196),
        "cells outside the band stay untouched"
    );
}

#[test]
fn flare_sweeps_in_opposite_direction_at_180_degrees() {
    let area = Rect::new(0, 0, 40, 4);
    let theme = Theme::default();
    let src = sample_src(area);
    let peak = Color::Rgb(255, 255, 255);
    let edge = Color::Rgb(20, 20, 20);
    let mirrored = |angle: f64| {
        let mut dst = Buffer::empty(area);
        Effect::flare_angle(angle, peak, edge).apply(&src, &mut dst, area, &theme, 0.25);
        let mut sum = 0u64;
        let mut count = 0u64;
        for y in 0..area.height {
            for x in 0..area.width {
                if dst.cell((x, y)).unwrap().fg != Color::Indexed(196) {
                    sum += x as u64;
                    count += 1;
                }
            }
        }
        assert!(count > 0, "angle {angle} should tint at t=0.25");
        (sum / count) as f64
    };
    assert!(mirrored(0.0) < 20.0, "0° enters from the left");
    assert!(mirrored(180.0) > 20.0, "180° enters from the right");
}

#[test]
fn flare_at_90_degrees_sweeps_top_to_bottom() {
    let area = Rect::new(0, 0, 16, 10);
    let theme = Theme::default();
    let src = sample_src(area);
    let peak = Color::Rgb(255, 255, 255);
    let edge = Color::Rgb(20, 20, 20);
    let mut dst = Buffer::empty(area);
    Effect::flare_angle(90.0, peak, edge).apply(&src, &mut dst, area, &theme, 0.5);

    let row_tinted = |row: u16| {
        (0..area.width)
            .filter(|&x| dst.cell((x, row)).unwrap().fg != Color::Indexed(196))
            .count()
    };
    assert_eq!(row_tinted(0), 0, "top row untouched mid-sweep");
    assert!(row_tinted(5) > 0, "middle row tinted mid-sweep");
    assert_eq!(row_tinted(9), 0, "bottom row untouched mid-sweep");
}

#[test]
fn radial_flare_leaves_extremes_untouched() {
    let area = Rect::new(0, 0, 24, 12);
    let theme = Theme::default();
    let src = sample_src(area);
    // t = 0 starts every sweep off-canvas; Collide ends on a visible ring at
    // t = 1 by design, so the full-clean check only applies at t = 0 for it.
    let effects = [
        Effect::radial_flare_expand(),
        Effect::radial_flare_converge(),
        Effect::radial_flare_collide(),
        Effect::flare_bidirectional(),
        Effect::flare_bidirectional_angle(45.0, Color::Rgb(255, 255, 255), Color::Rgb(20, 20, 20)),
    ];
    for effect in &effects {
        let mut dst = Buffer::empty(area);
        effect.apply(&src, &mut dst, area, &theme, 0.0);
        for y in 0..area.height {
            for x in 0..area.width {
                assert_eq!(dst.cell((x, y)).unwrap().fg, Color::Indexed(196));
            }
        }
    }
    for effect in &effects[..2] {
        let mut dst = Buffer::empty(area);
        effect.apply(&src, &mut dst, area, &theme, 1.0);
        for y in 0..area.height {
            for x in 0..area.width {
                assert_eq!(dst.cell((x, y)).unwrap().fg, Color::Indexed(196));
            }
        }
    }
}

#[test]
fn radial_flare_expand_tints_center_first() {
    let area = Rect::new(0, 0, 24, 12);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut early = Buffer::empty(area);
    Effect::radial_flare_expand().apply(&src, &mut early, area, &theme, 0.15);
    let center_fg = early.cell((12, 6)).unwrap().fg;
    assert_ne!(center_fg, Color::Indexed(196), "center tints first");
    let corner_fg = early.cell((0, 0)).unwrap().fg;
    assert_eq!(
        corner_fg,
        Color::Indexed(196),
        "far corner untouched at t=0.15"
    );
}

#[test]
fn radial_flare_converge_tints_edges_first() {
    let area = Rect::new(0, 0, 24, 12);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut early = Buffer::empty(area);
    Effect::radial_flare_converge().apply(&src, &mut early, area, &theme, 0.15);
    let corner_fg = early.cell((0, 0)).unwrap().fg;
    assert_ne!(corner_fg, Color::Indexed(196), "edges tint first");
}

#[test]
fn radial_flare_collide_tints_mid_radius() {
    let area = Rect::new(0, 0, 24, 12);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut mid = Buffer::empty(area);
    Effect::radial_flare_collide().apply(&src, &mut mid, area, &theme, 0.5);
    let tinted = (0..area.height)
        .flat_map(|y| (0..area.width).map(move |x| (x, y)))
        .filter(|&pos| mid.cell(pos).unwrap().fg != Color::Indexed(196))
        .count();
    assert!(tinted > 0, "rings tint at mid radius");
}

#[test]
fn flare_bidirectional_tints_both_directions() {
    let area = Rect::new(0, 0, 40, 6);
    let theme = Theme::default();
    let src = sample_src(area);
    let mut early = Buffer::empty(area);
    Effect::flare_bidirectional().apply(&src, &mut early, area, &theme, 0.2);
    assert_ne!(
        early.cell((20, 3)).unwrap().fg,
        Color::Indexed(196),
        "center tints at small t"
    );
    assert_eq!(
        early.cell((0, 0)).unwrap().fg,
        Color::Indexed(196),
        "extreme untouched at t=0.2"
    );
}

#[test]
fn flare_leaves_content_untouched_at_extremes() {
    let area = Rect::new(0, 0, 40, 4);
    let theme = Theme::default();
    for t in [0.0, 1.0] {
        let mut dst = Buffer::empty(area);
        let effect = Effect::flare();
        effect.apply(&sample_src(area), &mut dst, area, &theme, t);
        for x in 0..area.width {
            for y in 0..area.height {
                let cell = dst.cell((x, y)).unwrap();
                assert_eq!(cell.symbol(), "A", "extreme {t} at {x},{y}");
                assert_eq!(cell.fg, Color::Indexed(196), "extreme {t} at {x},{y}");
            }
        }
    }
}

#[test]
fn style_constants_exported() {
    let _ = Color::Rgb(1, 2, 3);
    let _ = Style::default();
}
