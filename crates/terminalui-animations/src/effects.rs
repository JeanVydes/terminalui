//! Buffer effects: slide, fade, reveal, wipe, dissolve, thanos dust, fall.
//!
//! Effects transform the contents of one `Buffer` region into another. The
//! usual pattern is: paint your component into a scratch buffer, then call
//! [`render_with`] with a progress value `0.0..=1.0` driven by an
//! [`crate::Animation`]:
//!
//! ```ignore
//! terminalui_animations::render_with(
//!     &Effect::slide_in(Direction::Left, 12),
//!     anim.progress(),
//!     buf, area, theme,
//!     |_area, buf, theme| { /* render your component here */ },
//! );
//! ```

use std::f64::consts::FRAC_PI_2;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use terminalui_core::theme::Theme;

/// Cardinal directions for slide effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    /// Slide starts on the left edge and lands on the right.
    Left,
    /// Slide starts on the right edge and lands on the left.
    Right,
    /// Slide starts on the top edge and lands on the bottom.
    Up,
    /// Slide starts on the bottom edge and lands on the top.
    Down,
}

/// Mode for a radial flare: where the ring comes from and where it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RadialMode {
    /// Ring expands from the center of the area toward the edges.
    #[default]
    Expand,
    /// Ring contracts from the edges toward the center.
    Converge,
    /// One ring expands from the center while another contracts from the
    /// edges; they meet at mid-radius.
    Collide,
}

/// An effect applied to a buffer region.
///
/// Most variants take no arguments beyond their configuration fields;
/// timing lives in your [`crate::Animation`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Effect {
    /// Fades from the theme background to the painted content.
    FadeIn,
    /// Fades from the painted content to the theme background.
    FadeOut,
    /// Slides the content into place from `direction`.
    SlideIn {
        /// Where the slide starts from.
        direction: Direction,
        /// How far it travels from `direction`, in cells.
        distance: u16,
        /// Easing applied to the slide motion.
        easing: crate::Easing,
    },
    /// Slides the content out toward `direction`.
    SlideOut {
        /// Where the slide ends up.
        direction: Direction,
        /// How far it travels to `direction`, in cells.
        distance: u16,
        /// Easing applied to the slide motion.
        easing: crate::Easing,
    },
    /// Reveals the content from left to right.
    RevealRight,
    /// Reveals the content from right to left.
    RevealLeft,
    /// Reveals the content from top to bottom.
    RevealDown,
    /// Reveals the content from bottom to top.
    RevealUp,
    /// Sweep reveal with a bright leading edge over the content.
    SweepRight {
        /// Color of the leading edge; defaults to `theme.accent`.
        color: Option<Color>,
    },
    /// Sweep reveal with a bright leading edge, right to left.
    SweepLeft {
        /// Color of the leading edge; defaults to `theme.accent`.
        color: Option<Color>,
    },
    /// Thanos-style disintegration: cells turn to ash that drifts up and fades.
    Thanos {
        /// The deterministic seed driving the cell assignment.
        seed: u64,
    },
    /// Dissolve in/out: scattered cells vanish and reappear.
    Dissolve {
        /// The deterministic seed driving the cell assignment.
        seed: u64,
    },
    /// Cells rain downward and out of the area.
    Fall,
    /// A bright reflective band sweeping left to right (or mirrored with
    /// `Direction::Right`). Every cell outside the band renders untouched;
    /// cells inside blend toward the tint with a parabolic falloff peaking
    /// at the band's center.
    ///
    /// Timing lives in your [`crate::Animation`]; the effect only owns the
    /// look. Works on anything that paints into a buffer: text, boxes,
    /// charts, whole panels.
    Flare {
        /// Travel direction in degrees, clockwise from the right edge:
        /// `0°` sweeps left to right, `90°` top to bottom, `180°` right to
        /// left, `270°` bottom to top. Any angle works; the band stays
        /// perpendicular to this axis.
        angle: f64,
        /// Thickness of the bright band in cells.
        width: u16,
        /// Tint at the center of the band (the hot spot).
        peak: Color,
        /// Tint at the soft outer edge of the band.
        edge: Color,
    },
    /// A bright ring of light sweeping over the painted content, like a
    /// flare seen on a curved surface. The ring only brightens what it
    /// touches, with a parabolic falloff peaking at the ring's center.
    ///
    /// Timing lives in your [`crate::Animation`]; the effect only owns the
    /// look. Works on anything that paints into a buffer.
    RadialFlare {
        /// Which radial behavior to use (expand, converge, collide).
        mode: RadialMode,
        /// Thickness of the bright ring in cells.
        width: u16,
        /// Tint at the center of the ring (the hot spot).
        peak: Color,
        /// Tint at the soft outer edge of the ring.
        edge: Color,
    },
    /// Two bands leaving the center in opposite directions along `angle`.
    /// Same falloff and palette as [`Effect::Flare`], but the sweep is
    /// bidirectional from the center instead of crossing the area once.
    ///
    /// Timing lives in your [`crate::Animation`].
    FlareBidir {
        /// Travel direction in degrees (see [`Effect::Flare`]).
        angle: f64,
        /// Thickness of the bright bands in cells.
        width: u16,
        /// Tint at the center of the bands (the hot spot).
        peak: Color,
        /// Tint at the soft outer edge of the bands.
        edge: Color,
    },
}

impl Effect {
    /// Slide in from `direction` with a smooth ease-out.
    pub fn slide_in(direction: Direction, distance: u16) -> Self {
        Self::SlideIn {
            direction,
            distance,
            easing: crate::Easing::EaseOutCubic,
        }
    }

    /// Slide in from `direction` with a bounce landing.
    pub fn slide_bounce(direction: Direction, distance: u16) -> Self {
        Self::SlideIn {
            direction,
            distance,
            easing: crate::Easing::EaseOutBounce,
        }
    }

    /// Slide out toward `direction` with a smooth ease-in.
    pub fn slide_out(direction: Direction, distance: u16) -> Self {
        Self::SlideOut {
            direction,
            distance,
            easing: crate::Easing::EaseInCubic,
        }
    }

    /// A flare sweeping left to right with default colors (a warm white-gold
    /// shimmer fading to a soft gold at the edges).
    pub fn flare() -> Self {
        Self::flare_colors(Color::Rgb(255, 255, 240), Color::Rgb(255, 224, 130))
    }

    /// A flare sweeping left to right with explicit colors.
    pub fn flare_colors(peak: Color, edge: Color) -> Self {
        Self::Flare {
            angle: 0.0,
            width: 12,
            peak,
            edge,
        }
    }

    /// A flare sweeping in the given direction, in degrees (0° left to
    /// right, 90° top to bottom, 180° right to left, 270° bottom to top).
    pub fn flare_angle(angle: f64, peak: Color, edge: Color) -> Self {
        Self::Flare {
            angle,
            width: 12,
            peak,
            edge,
        }
    }

    /// A flare sweeping in the given [`Direction`].
    pub fn flare_direction(direction: Direction) -> Self {
        let angle = match direction {
            Direction::Left => 0.0,
            Direction::Down => 90.0,
            Direction::Right => 180.0,
            Direction::Up => 270.0,
        };
        Self::flare_angle(angle, Color::Rgb(255, 255, 240), Color::Rgb(255, 224, 130))
    }

    /// A radial ring sweeping out from the center (or collapsing into it,
    /// or colliding mid-way; see [`RadialMode`]), with warm default colors.
    pub fn radial_flare() -> Self {
        Self::radial_flare_mode(
            RadialMode::Expand,
            Color::Rgb(255, 255, 240),
            Color::Rgb(255, 224, 130),
        )
    }

    /// A radial ring sweeping out from the center toward the edges.
    pub fn radial_flare_expand() -> Self {
        Self::radial_flare_mode(
            RadialMode::Expand,
            Color::Rgb(255, 255, 240),
            Color::Rgb(255, 224, 130),
        )
    }

    /// A radial ring collapsing from the edges toward the center.
    pub fn radial_flare_converge() -> Self {
        Self::radial_flare_mode(
            RadialMode::Converge,
            Color::Rgb(255, 255, 240),
            Color::Rgb(255, 224, 130),
        )
    }

    /// One ring expanding from the center while another contracts from the
    /// edges; they meet at mid-radius.
    pub fn radial_flare_collide() -> Self {
        Self::radial_flare_mode(
            RadialMode::Collide,
            Color::Rgb(255, 255, 240),
            Color::Rgb(255, 224, 130),
        )
    }

    /// A radial ring with an explicit mode and colors.
    pub fn radial_flare_mode(mode: RadialMode, peak: Color, edge: Color) -> Self {
        Self::RadialFlare {
            mode,
            width: 10,
            peak,
            edge,
        }
    }

    /// Two bands leaving the center in opposite directions along the given
    /// angle, with warm default colors.
    pub fn flare_bidirectional() -> Self {
        Self::flare_bidirectional_angle(0.0, Color::Rgb(255, 255, 240), Color::Rgb(255, 224, 130))
    }

    /// Two bands leaving the center in opposite directions along `angle`.
    pub fn flare_bidirectional_angle(angle: f64, peak: Color, edge: Color) -> Self {
        Self::FlareBidir {
            angle,
            width: 12,
            peak,
            edge,
        }
    }

    /// Apply the effect, copying `src` (sized to `area`) into `dst`,
    /// transformed according to `progress` in `[0.0, 1.0]`.
    pub fn apply(&self, src: &Buffer, dst: &mut Buffer, area: Rect, theme: &Theme, progress: f64) {
        clear(dst, area, theme.background);
        match self {
            Effect::FadeIn => fade(src, dst, area, theme.background, progress),
            Effect::FadeOut => fade(src, dst, area, theme.background, 1.0 - progress),
            Effect::SlideIn {
                direction,
                distance,
                easing,
            } => {
                let eased = easing.apply(progress);
                let offset = ((1.0 - eased) * *distance as f64).round() as i32;
                slide_by(src, dst, area, *direction, -offset);
            }
            Effect::SlideOut {
                direction,
                distance,
                easing,
            } => {
                let eased = easing.apply(progress);
                let offset = (eased * *distance as f64).round() as i32;
                slide_by(src, dst, area, *direction, offset);
            }
            Effect::RevealRight => reveal(src, dst, area, progress, true, false),
            Effect::RevealLeft => reveal(src, dst, area, progress, true, true),
            Effect::RevealDown => reveal(src, dst, area, progress, false, false),
            Effect::RevealUp => reveal(src, dst, area, progress, false, true),
            Effect::SweepRight { color } => {
                sweep(src, dst, area, theme, progress, true, *color);
            }
            Effect::SweepLeft { color } => {
                sweep(src, dst, area, theme, progress, false, *color);
            }
            Effect::Thanos { seed } => thanos(src, dst, area, theme, progress, *seed),
            Effect::Dissolve { seed } => dissolve(src, dst, area, theme, progress, *seed),
            Effect::Fall => fall(src, dst, area, theme, progress),
            Effect::Flare {
                angle,
                width,
                peak,
                edge,
            } => flare(
                src,
                dst,
                area,
                progress,
                FlareParams {
                    angle: *angle,
                    width: *width,
                    peak: *peak,
                    edge: *edge,
                },
            ),
            Effect::RadialFlare {
                mode,
                width,
                peak,
                edge,
            } => radial_flare(
                src,
                dst,
                area,
                progress,
                RadialParams {
                    mode: *mode,
                    width: *width,
                    peak: *peak,
                    edge: *edge,
                },
            ),
            Effect::FlareBidir {
                angle,
                width,
                peak,
                edge,
            } => flare_bidirectional(
                src,
                dst,
                area,
                progress,
                BidirectionalParams {
                    angle: *angle,
                    width: *width,
                    peak: *peak,
                    edge: *edge,
                },
            ),
        }
    }
}

/// Paint `paint` into `dst` at `area`, transformed by `effect` at `progress`.
pub fn render_with(
    effect: &Effect,
    progress: f64,
    dst: &mut Buffer,
    area: Rect,
    theme: &Theme,
    paint: impl FnOnce(Rect, &mut Buffer, &Theme),
) {
    let mut src = Buffer::empty(area);
    paint(area, &mut src, theme);
    effect.apply(&src, dst, area, theme, progress);
}

fn clear(dst: &mut Buffer, area: Rect, background: Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = dst.cell_mut((x, y)) {
                cell.set_symbol(" ");
                cell.set_fg(background);
                cell.set_bg(background);
                cell.modifier = Modifier::empty();
            }
        }
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Linear interpolate between two colors. RGB twins only; named colors
/// snap at the midpoint (like the rest of the color interpolation code).
pub fn lerp_color(from: Color, to: Color, t: f64) -> Color {
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            lerp(r1 as f64, r2 as f64, t).round() as u8,
            lerp(g1 as f64, g2 as f64, t).round() as u8,
            lerp(b1 as f64, b2 as f64, t).round() as u8,
        ),
        // ANSI named colors have no interpolation; snap at the midpoint.
        _ => {
            if t < 0.5 {
                from
            } else {
                to
            }
        }
    }
}

fn copy_cell(src: &Buffer, dst: &mut Buffer, x: u16, y: u16) {
    let Some(s) = src.cell((x, y)).cloned() else {
        return;
    };
    if let Some(d) = dst.cell_mut((x, y)) {
        *d = s;
    }
}

fn slide_by(src: &Buffer, dst: &mut Buffer, area: Rect, direction: Direction, offset: i32) {
    let (dx, dy) = match direction {
        Direction::Left => (-offset, 0),
        Direction::Right => (offset, 0),
        Direction::Up => (0, -offset),
        Direction::Down => (0, offset),
    };
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let Ok(nx) = (x as i32 + dx).try_into() else {
                continue;
            };
            let Ok(ny) = (y as i32 + dy).try_into() else {
                continue;
            };
            if nx >= area.x && nx < area.x + area.width && ny >= area.y && ny < area.y + area.height
            {
                copy_cell(src, dst, nx, ny);
            }
        }
    }
}

fn fade(src: &Buffer, dst: &mut Buffer, area: Rect, background: Color, t: f64) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let Some(s) = src.cell((x, y)) else {
                continue;
            };
            if let Some(d) = dst.cell_mut((x, y)) {
                d.set_symbol(s.symbol());
                d.modifier = s.modifier;
                d.set_fg(lerp_color(background, s.fg, t));
                d.set_bg(lerp_color(background, s.bg, t));
            }
        }
    }
}

fn reveal(src: &Buffer, dst: &mut Buffer, area: Rect, t: f64, horizontal: bool, reverse: bool) {
    let t = t.clamp(0.0, 1.0);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let visible = if horizontal {
                let fx = (x - area.x) as f64 / area.width.max(1) as f64;
                if reverse {
                    fx >= 1.0 - t
                } else {
                    fx < t
                }
            } else {
                let fy = (y - area.y) as f64 / area.height.max(1) as f64;
                if reverse {
                    fy >= 1.0 - t
                } else {
                    fy < t
                }
            };
            if visible {
                copy_cell(src, dst, x, y);
            }
        }
    }
}

fn sweep(
    src: &Buffer,
    dst: &mut Buffer,
    area: Rect,
    theme: &Theme,
    t: f64,
    left_to_right: bool,
    color: Option<Color>,
) {
    reveal(src, dst, area, t, true, !left_to_right);
    if t <= 0.0 || t >= 1.0 {
        return;
    }
    let edge_color = color.unwrap_or(theme.accent);
    let edge = (t * area.width as f64).round() as i32;
    for y in area.y..area.y + area.height {
        for offset in 0..2i32 {
            let position = if left_to_right {
                area.x as i32 + edge - 1 - offset
            } else {
                area.x as i32 + area.width as i32 - 1 - edge + offset
            };
            if position >= area.x as i32 && (position as u16) < area.x + area.width {
                if let Some(d) = dst.cell_mut((position as u16, y)) {
                    d.set_fg(edge_color);
                    d.set_bg(theme.background);
                }
            }
        }
    }
}

fn flare_intensity(distance: f64, half: f64) -> f64 {
    if distance >= half {
        return 0.0;
    }
    let cosine = (distance / half * FRAC_PI_2).cos();
    cosine * cosine
}

fn tint_cell(cell: &mut ratatui::buffer::Cell, intensity: f64, peak: Color, edge: Color) {
    if intensity <= 0.0 {
        return;
    }
    let tint = lerp_color(edge, peak, intensity);
    cell.set_fg(lerp_color(cell.fg, tint, intensity * 0.9));
    cell.set_bg(lerp_color(cell.bg, tint, intensity * 0.65));
}

struct FlareParams {
    angle: f64,
    width: u16,
    peak: Color,
    edge: Color,
}

fn flare(src: &Buffer, dst: &mut Buffer, area: Rect, t: f64, params: FlareParams) {
    let t = t.clamp(0.0, 1.0);
    let angle = params.angle.to_radians();
    let axis_x = angle.cos();
    let axis_y = angle.sin();
    // Project each cell center onto the travel axis; the band is a slice of
    // that axis sweeping from one side of the canvas to the other.
    let half = (params.width as f64 / 2.0).max(1.0);
    let project = |x: f64, y: f64| x * axis_x + y * axis_y;
    let corners = [
        (area.x as f64 + 0.5, area.y as f64 + 0.5),
        ((area.x + area.width) as f64 - 0.5, area.y as f64 + 0.5),
        (area.x as f64 + 0.5, (area.y + area.height) as f64 - 0.5),
        (
            (area.x + area.width) as f64 - 0.5,
            (area.y + area.height) as f64 - 0.5,
        ),
    ];
    let min_projection = corners
        .iter()
        .map(|&(x, y)| project(x, y))
        .fold(f64::INFINITY, f64::min);
    let max_projection = corners
        .iter()
        .map(|&(x, y)| project(x, y))
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (max_projection - min_projection) + half * 2.0;
    let position = min_projection - half + t * span;
    // Baseline copy; the band only tints what it touches.
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            copy_cell(src, dst, x, y);
        }
    }
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell_projection = project(x as f64 + 0.5, y as f64 + 0.5);
            let distance = (cell_projection - position).abs();
            if distance >= half {
                continue;
            }
            let intensity = flare_intensity(distance, half);
            let Some(cell) = dst.cell_mut((x, y)) else {
                continue;
            };
            tint_cell(cell, intensity, params.peak, params.edge);
        }
    }
}

struct RadialParams {
    mode: RadialMode,
    width: u16,
    peak: Color,
    edge: Color,
}

struct BidirectionalParams {
    angle: f64,
    width: u16,
    peak: Color,
    edge: Color,
}

/// Two bright bands leaving the center in opposite directions along `angle`.
/// The sweep is bidirectional from the center; each band fades outward with
/// the same parabolic falloff as the plain flare.
fn flare_bidirectional(
    src: &Buffer,
    dst: &mut Buffer,
    area: Rect,
    t: f64,
    params: BidirectionalParams,
) {
    let t = t.clamp(0.0, 1.0);
    let angle = params.angle.to_radians();
    let axis_x = angle.cos();
    let axis_y = angle.sin();
    let half = (params.width as f64 / 2.0).max(1.0);
    let center_x = area.x as f64 + area.width as f64 / 2.0;
    let center_y = area.y as f64 + area.height as f64 / 2.0;
    let corners = [
        (area.x as f64 + 0.5, area.y as f64 + 0.5),
        ((area.x + area.width) as f64 - 0.5, area.y as f64 + 0.5),
        (area.x as f64 + 0.5, (area.y + area.height) as f64 - 0.5),
        (
            (area.x + area.width) as f64 - 0.5,
            (area.y + area.height) as f64 - 0.5,
        ),
    ];
    let project = |x: f64, y: f64| x * axis_x + y * axis_y;
    let center_projection = project(center_x, center_y);
    let max_outward = corners
        .iter()
        .map(|&(x, y)| (project(x, y) - center_projection).abs())
        .fold(0.0_f64, f64::max);
    let span = max_outward + half * 2.0;
    // Start before the center so t = 0 is invisible and t = 1 has exited.
    let position = -half + t * span;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            copy_cell(src, dst, x, y);
        }
    }
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let projection = project(x as f64 + 0.5, y as f64 + 0.5);
            let distance = ((projection - center_projection).abs() - position).abs();
            if distance >= half {
                continue;
            }
            let intensity = flare_intensity(distance, half);
            let Some(cell) = dst.cell_mut((x, y)) else {
                continue;
            };
            tint_cell(cell, intensity, params.peak, params.edge);
        }
    }
}

/// A bright ring sweeping over the painted content, radial from the center
/// of the area. `Expand` grows outward, `Converge` collapses inward,
/// `Collide` expands one ring while another contracts; they meet at
/// mid-radius at t = 1.
fn radial_flare(src: &Buffer, dst: &mut Buffer, area: Rect, t: f64, params: RadialParams) {
    let t = t.clamp(0.0, 1.0);
    let half = (params.width as f64 / 2.0).max(1.0);
    let center_x = area.x as f64 + area.width as f64 / 2.0;
    let center_y = area.y as f64 + area.height as f64 / 2.0;
    let dx = (center_x - area.x as f64).max((area.x + area.width) as f64 - center_x);
    let dy = (center_y - area.y as f64).max((area.y + area.height) as f64 - center_y);
    let max_radius = dx.hypot(dy);
    let span_single = max_radius + half * 2.0;
    let ring_out = -half + t * (max_radius / 2.0 + half);
    let ring_in = max_radius + half - t * (max_radius / 2.0 + half);
    let pos_single = match params.mode {
        RadialMode::Expand => -half + t * span_single,
        RadialMode::Converge => max_radius + half - t * span_single,
        RadialMode::Collide => -half,
    };
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            copy_cell(src, dst, x, y);
        }
    }
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cx = x as f64 + 0.5 - center_x;
            let cy = y as f64 + 0.5 - center_y;
            let radius = (cx * cx + cy * cy).sqrt();
            let intensity = match params.mode {
                RadialMode::Expand | RadialMode::Converge => {
                    flare_intensity((radius - pos_single).abs(), half)
                }
                RadialMode::Collide => flare_intensity((radius - ring_out).abs(), half)
                    .max(flare_intensity((radius - ring_in).abs(), half)),
            };
            if intensity <= 0.0 {
                continue;
            }
            let Some(cell) = dst.cell_mut((x, y)) else {
                continue;
            };
            tint_cell(cell, intensity, params.peak, params.edge);
        }
    }
}

fn hash01(x: u16, y: u16, seed: u64) -> f64 {
    let mut z = seed
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((x as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((y as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

const ASH_HIGH: f64 = 0.25;
const MAX_SPAWN: f64 = 0.55;
const PARTICLE_LIFE: f64 = 0.45;

fn thanos(src: &Buffer, dst: &mut Buffer, area: Rect, theme: &Theme, t: f64, seed: u64) {
    // Phase 1: cells whose spawn time hasn't arrived are drawn normally.
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let Some(s) = src.cell((x, y)) else {
                continue;
            };
            if s.symbol() == " " {
                continue;
            }
            let spawn = ASH_HIGH + MAX_SPAWN * hash01(x, y, seed);
            if t < spawn {
                copy_cell(src, dst, x, y);
            }
        }
    }
    // Phase 2: ash particles.
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let Some(s) = src.cell((x, y)) else {
                continue;
            };
            if s.symbol() == " " {
                continue;
            }
            let spawn = ASH_HIGH + MAX_SPAWN * hash01(x, y, seed);
            if t < spawn {
                continue;
            }
            let age = (t - spawn) / PARTICLE_LIFE;
            if age >= 1.0 {
                continue;
            }
            // Drift up and to the sides, decelerating.
            let drift = hash01(x, y, seed ^ 0xD0FD) - 0.5;
            let rise = hash01(x, y, seed ^ 0xBEEF) * 2.0 + 1.5;
            let dx = drift * age * (area.width as f64) * 0.5;
            let dy = -(rise * age) + 1.6 * age * age;
            let px = x as f64 + dx;
            let py = y as f64 + dy;
            if px >= area.x as f64
                && px < (area.x + area.width) as f64
                && py >= area.y as f64
                && py < (area.y + area.height) as f64
            {
                let px = px.round() as u16;
                let py = py.round() as u16;
                let alpha = 1.0 - age;
                let tint = if age < 0.25 {
                    // Hot ember: tint toward orange.
                    lerp_color(s.fg, Color::Rgb(255, 166, 87), age / 0.25)
                } else {
                    // Cool ash gray.
                    lerp_color(
                        Color::Rgb(255, 166, 87),
                        Color::Rgb(140, 140, 140),
                        ((age - 0.25) / 0.6).min(1.0),
                    )
                };
                let glyph = if age < 0.33 {
                    "•"
                } else if age < 0.66 {
                    "·"
                } else {
                    "∙"
                };
                if let Some(d) = dst.cell_mut((px, py)) {
                    d.set_symbol(glyph);
                    d.set_fg(lerp_color(tint, theme.background, 1.0 - alpha));
                    d.set_bg(theme.background);
                    d.modifier = Modifier::empty();
                }
            }
        }
    }
}

fn dissolve(src: &Buffer, dst: &mut Buffer, area: Rect, theme: &Theme, t: f64, seed: u64) {
    let threshold = t * 1.35; // head start so t=1 reveals everything
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let Some(s) = src.cell((x, y)) else {
                continue;
            };
            if s.symbol() == " " {
                continue;
            }
            let appear = hash01(x, y, seed);
            if appear <= threshold {
                let dim = if (threshold - appear) < 0.12 {
                    0.4
                } else {
                    1.0
                };
                if let Some(cell) = dst.cell_mut((x, y)) {
                    cell.set_symbol(s.symbol());
                    cell.modifier = s.modifier;
                    cell.set_fg(lerp_color(theme.background, s.fg, dim));
                    cell.set_bg(lerp_color(theme.background, s.bg, dim));
                }
            }
        }
    }
}

fn fall(src: &Buffer, dst: &mut Buffer, area: Rect, theme: &Theme, t: f64) {
    let t2 = t * t;
    let drop = t2 * (area.height + 6) as f64;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let Some(s) = src.cell((x, y)) else {
                continue;
            };
            if s.symbol() == " " {
                continue;
            }
            let sway = (hash01(x, y, 0xFA11) - 0.5) * 2.0 * t;
            let ny = y as f64 + drop;
            let nx = x as f64 + sway;
            if ny >= area.y as f64
                && ny < (area.y + area.height) as f64
                && nx >= area.x as f64
                && nx < (area.x + area.width) as f64
            {
                let nx = nx.round() as u16;
                let ny = ny.round() as u16;
                if let Some(d) = dst.cell_mut((nx, ny)) {
                    d.set_symbol(s.symbol());
                    d.modifier = s.modifier;
                    // Fade toward the background as it falls.
                    d.set_fg(lerp_color(s.fg, theme.background, t2));
                    d.set_bg(lerp_color(s.bg, theme.background, t2));
                }
            }
        }
    }
}
