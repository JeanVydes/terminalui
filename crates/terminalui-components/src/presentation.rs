//! Shared presentation presets: how "ink" maps to glyphs.
//!
//! Some components (charts, spinners, meters…) draw with glyphs. A
//! `RenderStyle` chooses the glyph set so the same component can render as
//! phosphor blocks, ASCII art, raw numbers and so on. It only affects glyph
//! choice, never colors.
//!
//! `RenderStyle::Native` is the default: each component resolves it to the
//! glyph set its author drew it with. The rest are shared presets.

use serde::Serialize;

/// Dot glyph a halftone ramp may reach at full intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DotSize {
    /// Caps at `•`.
    Small,
    /// Caps at `●`.
    #[default]
    Medium,
    /// Caps at `◉`.
    Large,
}

/// Knob set for the shading ramps. `density` and `dotsize` only affect
/// [`RenderStyle::Halftone`] and [`RenderStyle::Squares`]; `contrast` remaps
/// every shading family except [`RenderStyle::Native`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct StyleTuning {
    /// Halftone dot coverage. 2..=6: low values show few dots, high values
    /// literally paint more cells into the ramp. Also picks the ramp length.
    pub density: u8,
    /// Gamma applied to the normalized intensity before picking a level.
    /// Above 1.0 the gradient saturates early (more ink); below 1.0 it leans
    /// sparse. Must stay inside 0.25..=4.0.
    pub contrast: f32,
    /// Largest glyph a halftone (or square) ramp may reach.
    pub dotsize: DotSize,
}

impl StyleTuning {
    pub const fn clamped(mut self) -> Self {
        if self.density < 2 {
            self.density = 2;
        }
        if self.density > 6 {
            self.density = 6;
        }
        if !(self.contrast >= 0.25 && self.contrast <= 4.0) {
            self.contrast = 1.0;
        }
        self
    }
}

impl Default for StyleTuning {
    fn default() -> Self {
        Self {
            density: 4,
            contrast: 1.0,
            dotsize: DotSize::Medium,
        }
    }
}

/// The built-in presentation choices. `Native` is the default and means
/// "whatever the component was originally rendered with".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RenderStyle {
    /// Whatever glyph set the component's author used natively.
    #[default]
    Native,
    /// Sparse/dense dots.
    Halftone,
    /// Ramp of punctuation.
    Ascii,
    /// `%$@` ramp.
    DenseAscii,
    /// `6 9 8 0` digits.
    Numbers,
    /// `0`/`1` bits.
    Binary,
    /// `A H M W` letterforms.
    Letters,
    /// `░▒▓█` block ramp.
    Blocks,
    /// `·▪■` square ramp.
    Squares,
    /// `╌─━═` line ramp.
    Lines,
    /// `░▞▚` dither pattern.
    Dither,
}

/// Hairline (thin line) glyphs for this presentation: horizontal and vertical strokes.
impl RenderStyle {
    /// Hairline strokes for separators, dividers, axes and panel borders.
    pub fn hairline(self) -> (&'static str, &'static str) {
        match self {
            RenderStyle::Native
            | RenderStyle::Lines
            | RenderStyle::Blocks
            | RenderStyle::Squares => ("─", "│"),
            RenderStyle::Ascii | RenderStyle::Binary | RenderStyle::Numbers => ("-", "|"),
            RenderStyle::DenseAscii => (".", ":"),
            RenderStyle::Letters => ("-", "l"),
            RenderStyle::Halftone => ("·", "·"),
            RenderStyle::Dither => (":", ":"),
        }
    }
}

/// Resolved glyph choices for a render style.
///
/// `grid` is the background grid/axis; `wick` draws wicks and other thin
/// strokes; `doji` marks a flat candle; `body` fills candle bodies; `live`
/// draws the live price line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    pub grid: &'static str,
    pub wick: &'static str,
    pub doji: &'static str,
    pub body: &'static str,
    pub live: &'static str,
}

const FRAMES_BLOCKS: &[&str] = &[
    "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
];
const FRAMES_HASH: &[&str] = &["-", "\\", "|", "/"];
const FRAMES_DOTS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const FRAMES_SQUARES: &[&str] = &["◐", "◓", "◑", "◒"];
const FRAMES_NUMBERS: &[&str] = &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
const FRAMES_BINARY: &[&str] = &["0", "1"];
const FRAMES_LETTERS: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];
const FRAMES_DITHER: &[&str] = &["░", "▒", "▓", "█", "▓", "▒"];

/// Scrollbar glyphs resolved from a style. `track` paints the rail,
/// `thumb` the position indicator, `cap_top`/`cap_bottom` round the thumb
/// when [`Radius::Small`](crate::chrome::Radius) is set and the thumb spans
/// at least three cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbarGlyphs {
    pub track: &'static str,
    pub thumb: &'static str,
    pub cap_top: &'static str,
    pub cap_bottom: &'static str,
}

/// Checkbox glyphs resolved from a style. `checked` fills the box when on,
/// `unchecked` draws the empty box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxGlyphs {
    pub checked: &'static str,
    pub unchecked: &'static str,
}

impl RenderStyle {
    /// Scrollbar glyphs for this presentation. Native resolves to the
    /// default light rail and solid thumb.
    pub fn scrollbar(self) -> ScrollbarGlyphs {
        let (track, thumb, cap_top, cap_bottom) = match self {
            RenderStyle::Native | RenderStyle::Blocks => ("░", "█", "▄", "▀"),
            RenderStyle::Halftone => ("·", "●", "◖", "◗"),
            RenderStyle::Ascii => ("-", "#", "=", "="),
            RenderStyle::DenseAscii => (".", "@", "@", "@"),
            RenderStyle::Numbers => ("0", "8", "8", "8"),
            RenderStyle::Binary => ("0", "1", "1", "1"),
            RenderStyle::Letters => ("i", "M", "M", "M"),
            RenderStyle::Squares => ("·", "■", "▄", "▀"),
            RenderStyle::Lines => ("╌", "━", "━", "━"),
            RenderStyle::Dither => ("░", "▓", "▄", "▀"),
        };
        ScrollbarGlyphs {
            track,
            thumb,
            cap_top,
            cap_bottom,
        }
    }

    /// Checkbox glyphs for this presentation.
    pub fn checkbox(self) -> CheckboxGlyphs {
        let (checked, unchecked) = match self {
            RenderStyle::Native | RenderStyle::Ascii | RenderStyle::Binary => ("[x]", "[ ]"),
            RenderStyle::Halftone => ("(●)", "( )"),
            RenderStyle::DenseAscii => ("[@]", "[ ]"),
            RenderStyle::Numbers => ("[8]", "[ ]"),
            RenderStyle::Letters => ("[M]", "[ ]"),
            RenderStyle::Blocks => ("[█]", "[ ]"),
            RenderStyle::Squares => ("[■]", "[ ]"),
            RenderStyle::Lines => ("[─]", "[ ]"),
            RenderStyle::Dither => ("[▓]", "[ ]"),
        };
        CheckboxGlyphs { checked, unchecked }
    }
}

impl RenderStyle {
    /// Resolve a choice into glyphs. [`RenderStyle::Native`] (the default)
    /// resolves to `native`, the glyph set the component's author drew with.
    pub fn resolve(self, native: Presentation) -> Presentation {
        match self {
            RenderStyle::Native => native,
            RenderStyle::Halftone => Presentation {
                grid: "·",
                wick: "•",
                doji: "•",
                body: "●",
                live: "·",
            },
            RenderStyle::Ascii => Presentation {
                grid: ".",
                wick: "*",
                doji: "*",
                body: "@",
                live: "#",
            },
            RenderStyle::DenseAscii => Presentation {
                grid: ".",
                wick: "$",
                doji: "$",
                body: "@",
                live: "%",
            },
            RenderStyle::Numbers => Presentation {
                grid: "0",
                wick: "6",
                doji: "6",
                body: "8",
                live: "9",
            },
            RenderStyle::Binary => Presentation {
                grid: "0",
                wick: "1",
                doji: "1",
                body: "1",
                live: "0",
            },
            RenderStyle::Letters => Presentation {
                grid: "A",
                wick: "H",
                doji: "H",
                body: "W",
                live: "M",
            },
            RenderStyle::Blocks => Presentation {
                grid: "░",
                wick: "▒",
                doji: "▒",
                body: "█",
                live: "▒",
            },
            RenderStyle::Squares => Presentation {
                grid: "·",
                wick: "▪",
                doji: "▪",
                body: "■",
                live: "▪",
            },
            RenderStyle::Lines => Presentation {
                grid: "╌",
                wick: "─",
                doji: "━",
                body: "═",
                live: "╌",
            },
            RenderStyle::Dither => Presentation {
                grid: "░",
                wick: "▞",
                doji: "▚",
                body: "█",
                live: "▚",
            },
        }
    }

    /// Glyph cycle for spinners and loaders in this style.
    ///
    /// `Native` is the braille-dot pattern.
    pub fn frames(self) -> &'static [&'static str] {
        match self {
            RenderStyle::Native | RenderStyle::Halftone => FRAMES_DOTS,
            RenderStyle::Lines | RenderStyle::DenseAscii | RenderStyle::Ascii => FRAMES_HASH,
            RenderStyle::Numbers => FRAMES_NUMBERS,
            RenderStyle::Binary => FRAMES_BINARY,
            RenderStyle::Letters => FRAMES_LETTERS,
            RenderStyle::Dither => FRAMES_DITHER,
            RenderStyle::Blocks => FRAMES_BLOCKS,
            RenderStyle::Squares => FRAMES_SQUARES,
        }
    }

    /// Glyph used to fill a progress bar's filled portion.
    pub fn filled(self) -> &'static str {
        match self {
            RenderStyle::Native | RenderStyle::Blocks => "█",
            RenderStyle::Halftone | RenderStyle::Dither => "▓",
            RenderStyle::Lines => "━",
            RenderStyle::DenseAscii | RenderStyle::Ascii => "#",
            RenderStyle::Numbers => "8",
            RenderStyle::Binary => "1",
            RenderStyle::Letters => "M",
            RenderStyle::Squares => "■",
        }
    }

    /// Glyph for a data point marker (outliers, scatter dots).
    pub fn marker(self) -> &'static str {
        match self {
            RenderStyle::Native => "•",
            RenderStyle::Halftone => "●",
            RenderStyle::Ascii => "*",
            RenderStyle::DenseAscii => "$",
            RenderStyle::Numbers => "9",
            RenderStyle::Binary => "1",
            RenderStyle::Letters => "W",
            RenderStyle::Blocks => "▪",
            RenderStyle::Squares => "■",
            RenderStyle::Lines => "━",
            RenderStyle::Dither => "▚",
        }
    }

    /// Faint gridline glyph for chart backgrounds.
    pub fn grid(self) -> &'static str {
        match self {
            RenderStyle::Native
            | RenderStyle::Blocks
            | RenderStyle::Lines
            | RenderStyle::Squares => "┊",
            RenderStyle::Halftone
            | RenderStyle::Numbers
            | RenderStyle::Binary
            | RenderStyle::Letters => "·",
            RenderStyle::Ascii | RenderStyle::DenseAscii => ":",
            RenderStyle::Dither => "░",
        }
    }

    /// Glyph used to draw a progress bar's track.
    pub fn empty(self) -> &'static str {
        match self {
            RenderStyle::Native | RenderStyle::Blocks => "░",
            RenderStyle::Halftone | RenderStyle::Dither => "░",
            RenderStyle::Lines => "─",
            RenderStyle::DenseAscii | RenderStyle::Ascii => ".",
            RenderStyle::Numbers => "0",
            RenderStyle::Binary => "0",
            RenderStyle::Letters => "i",
            RenderStyle::Squares => "□",
        }
    }
}

impl RenderStyle {
    /// Map a per-cell intensity (0..=1) onto the ink ramp of this family.
    /// `Native` picks between `empty` and `filled`; every other family maps
    /// onto a dot/glyph ramp. `density` and `dotsize` apply to Halftone and
    /// Squares; `contrast` applies to every shading family except `Native`.
    pub fn shade(self, intensity: f32, tuning: &StyleTuning) -> &'static str {
        let tuning = tuning.clamped();
        let u = intensity.clamp(0.0, 1.0).powf(1.0 / tuning.contrast);
        // Higher density lifts more cells into the ink ramp: at density 2
        // (factor = 1.0) only the strongest tones render dots; at density 6
        // almost every cell lights up.
        let u = (u * (1.0 + (tuning.density.clamp(2, 6) as f32 - 2.0) * 0.35)).min(1.0);
        match self {
            RenderStyle::Native => {
                if u >= 0.5 {
                    self.filled()
                } else {
                    " "
                }
            }
            RenderStyle::Halftone => {
                let ramp: &[&'static str] = match tuning.dotsize {
                    DotSize::Small => &[" ", "·", "∙", "•"],
                    DotSize::Medium => &[" ", "·", "•", "●"],
                    DotSize::Large => &[" ", "·", "•", "●", "◉"],
                };
                let levels = (tuning.density as usize).min(ramp.len());
                let pick = |index: usize| -> &'static str {
                    let last = ramp.len() - 1;
                    ramp[index * last / (levels - 1)]
                };
                let index = (u * (levels - 1) as f32).round() as usize;
                pick(index.min(levels - 1))
            }
            RenderStyle::Squares => {
                let ramp: &[&'static str] = match tuning.dotsize {
                    DotSize::Small => &[" ", "·", "▪"],
                    _ => &[" ", "▪", "■"],
                };
                ramp[(u * (ramp.len() - 1) as f32).round() as usize]
            }
            RenderStyle::Ascii => pick_ramp(u, &[" ", ".", ":", "#", "@"]),
            RenderStyle::DenseAscii => pick_ramp(u, &[" ", ".", "$", "@"]),
            RenderStyle::Numbers => pick_ramp(u, &[" ", "0", "6", "8"]),
            RenderStyle::Binary => {
                if u >= 0.5 {
                    "1"
                } else {
                    " "
                }
            }
            RenderStyle::Letters => pick_ramp(u, &[" ", "A", "H", "M", "W"]),
            RenderStyle::Blocks => pick_ramp(u, &[" ", "░", "▒", "▓", "█"]),
            RenderStyle::Lines => pick_ramp(u, &[" ", "╌", "─", "━", "═"]),
            RenderStyle::Dither => pick_ramp(u, &[" ", "░", "▞", "▓"]),
        }
    }
}

fn pick_ramp(u: f32, ramp: &[&'static str]) -> &'static str {
    let index = (u * (ramp.len() - 1) as f32).round() as usize;
    ramp[index.min(ramp.len() - 1)]
}
