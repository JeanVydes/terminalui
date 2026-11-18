//! Host-side latency windows for the presentation lab.
//!
//! The monitor invents deterministic, smoothly interpolated distributions.
//! It owns what the numbers mean; the box plot only draws them.

use ratatui::style::Color;

use terminalui::theme::Theme;
use terminalui::{node, Node, Text, TextStyle};

use terminalui::components::boxplot::BoxPlotRow;

use crate::market::Rng;

struct ServiceSpec {
    name: &'static str,
    base: f64,
    spread: f64,
}

const SERVICES: [ServiceSpec; 6] = [
    ServiceSpec {
        name: "auth",
        base: 38.0,
        spread: 0.30,
    },
    ServiceSpec {
        name: "search",
        base: 120.0,
        spread: 0.45,
    },
    ServiceSpec {
        name: "checkout",
        base: 210.0,
        spread: 0.35,
    },
    ServiceSpec {
        name: "profile",
        base: 60.0,
        spread: 0.25,
    },
    ServiceSpec {
        name: "images",
        base: 160.0,
        spread: 0.60,
    },
    ServiceSpec {
        name: "payments",
        base: 260.0,
        spread: 0.30,
    },
];

const WINDOW_SECONDS: f64 = 1.5;

pub(crate) struct LatencyMonitor {
    colors: [Color; 6],
}

impl LatencyMonitor {
    pub(crate) fn new(theme: &Theme) -> Self {
        Self {
            colors: [
                theme.accent,
                theme.success,
                theme.warning,
                theme.destructive,
                theme.primary,
                Color::Rgb(210, 168, 255),
            ],
        }
    }

    /// One box per service: p5, p25, p50, p75 and p95 of the current window
    /// blended into the next one, plus the window's outliers.
    pub(crate) fn rows(&self, seconds: f64) -> Vec<BoxPlotRow> {
        let phase = seconds.max(0.0) * WINDOW_SECONDS;
        let window = phase.floor();
        let blend = smoothstep(phase - window);
        SERVICES
            .iter()
            .enumerate()
            .map(|(index, service)| {
                let previous = quantiles(service, index, window);
                let next = quantiles(service, index, window + 1.0);
                let blended: Vec<f64> = previous
                    .iter()
                    .zip(next.iter())
                    .map(|(from, to)| from + (to - from) * blend)
                    .collect();
                let outliers = outlier_values(window, index, blended[4]);
                BoxPlotRow::new(blended[0], blended[1], blended[2], blended[3], blended[4])
                    .with_outliers(outliers)
                    .with_color(self.colors[index])
            })
            .collect()
    }

    /// One composed label per row, colored like its box.
    pub(crate) fn labels(&self) -> Vec<Node> {
        SERVICES
            .iter()
            .zip(self.colors.iter())
            .map(|(service, color)| {
                node(
                    format!("box-label-{}", service.name),
                    Text::new(service.name).with_style(TextStyle::default().fg(*color)),
                )
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn caption(&self, seconds: f64) -> String {
        let window = (seconds.max(0.0) * WINDOW_SECONDS).floor() as u64;
        format!("window {:03} · 500 samples per service", window % 1000)
    }
}

fn quantiles(service: &ServiceSpec, index: usize, window: f64) -> [f64; 5] {
    let seed = index as f64;
    let drift = 1.0
        + 0.35 * (noise2(window * 0.4, seed * 7.0, 2) - 0.5)
        + if index == 4 {
            0.5 * (noise2(window * 0.3, 3.0, 8) - 0.6).max(0.0)
        } else {
            0.0
        };
    let median = service.base * drift;
    let spread = service.spread * (0.8 + 0.4 * noise2(window * 0.5, seed, 6));
    [
        median * (-2.2 * spread).exp(),
        median * (-0.7 * spread).exp(),
        median,
        median * (0.75 * spread).exp(),
        median * (2.4 * spread).exp(),
    ]
}

fn outlier_values(window: f64, index: usize, high: f64) -> Vec<f64> {
    let mut rng = Rng::new(window.max(0.0) as u64 * 31 + index as u64 + 7);
    let count = 1 + (rng.next() * 3.0) as usize;
    (0..count)
        .map(|_| high * (1.15 + rng.next() * 0.9))
        .filter(|value| *value <= 600.0)
        .collect()
}

fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn noise2(x: f64, y: f64, seed: u64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = smoothstep(x - x0);
    let fy = smoothstep(y - y0);
    let x0 = x0 as i64;
    let y0 = y0 as i64;
    let top = lerp(hash(x0, y0, seed), hash(x0 + 1, y0, seed), fx);
    let bottom = lerp(hash(x0, y0 + 1, seed), hash(x0 + 1, y0 + 1, seed), fx);
    lerp(top, bottom, fy)
}

fn lerp(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

fn hash(x: i64, y: i64, seed: u64) -> f64 {
    let mut value = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    value ^= value >> 33;
    value = value.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    value ^= value >> 33;
    (value >> 11) as f64 / (1u64 << 53) as f64
}
