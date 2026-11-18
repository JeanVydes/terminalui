//! The invented market feed: a seeded random walk per timeframe.
//! The host feeds it into the charts; the charts only render.

use std::time::Duration;

use terminalui::components::candlechart::Candle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Timeframe {
    Secs1,
    Secs5,
    Secs30,
    Min1,
}

impl Timeframe {
    pub(crate) fn duration(self) -> Duration {
        match self {
            Timeframe::Secs1 => Duration::from_secs(1),
            Timeframe::Secs5 => Duration::from_secs(5),
            Timeframe::Secs30 => Duration::from_secs(30),
            Timeframe::Min1 => Duration::from_secs(60),
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Timeframe::Secs1 => "1s",
            Timeframe::Secs5 => "5s",
            Timeframe::Secs30 => "30s",
            Timeframe::Min1 => "1m",
        }
    }

    fn seed(self) -> u64 {
        77 ^ (self.duration().as_secs().wrapping_mul(2_654_435_761))
    }
}

/// xorshift64* PRNG: deterministic invented markets.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

const HISTORY: usize = 240;

/// Builds the seeded pre-history plus the currently forming candle.
fn build_series(seed: u64) -> Vec<Candle> {
    let mut rng = Rng(seed ^ 0x9E37_79B9_7F4A_7C15);
    let mut raw = Vec::with_capacity(HISTORY);
    let mut price = 0.0_f64;
    let mut drift = 0.0_f64;
    for _ in 0..HISTORY {
        drift = drift * 0.85 + (rng.next() - 0.5) * 0.45;
        price += drift + (rng.next() - 0.5) * 1.1;
        raw.push(price);
    }
    let fix = (raw[HISTORY - 1] - raw[0]) / HISTORY as f64;
    let closes: Vec<f64> = raw
        .iter()
        .enumerate()
        .map(|(index, value)| 100.0 + value - fix * index as f64)
        .collect();
    let mut series: Vec<Candle> = closes
        .iter()
        .enumerate()
        .map(|(index, &close)| {
            let open = if index == 0 {
                close - 0.3
            } else {
                closes[index - 1]
            };
            let high = open.max(close) + rng.next() * 1.8;
            let low = open.min(close) - rng.next() * 1.8;
            let volume = 0.3 + (close - open).abs() * 0.25 + rng.next() * 0.4;
            Candle {
                open,
                high,
                low,
                close,
                volume,
            }
        })
        .collect();
    let last_close = series.last().map(|candle| candle.close).unwrap_or(100.0);
    series.push(Candle {
        open: last_close,
        high: last_close,
        low: last_close,
        close: last_close,
        volume: 0.05,
    });
    series
}

pub(crate) struct MarketTick {
    pub new_candle: bool,
    pub candle: Candle,
}

pub(crate) struct InventedMarket {
    pub timeframe: Timeframe,
    pub series: Vec<Candle>,
    pub rng: Rng,
    pub elapsed: Duration,
}

impl InventedMarket {
    pub(crate) fn new(timeframe: Timeframe) -> Self {
        let seed = timeframe.seed();
        Self {
            timeframe,
            series: build_series(seed),
            rng: Rng(seed ^ 0x2545_F491_4F6C_DD1D),
            elapsed: Duration::ZERO,
        }
    }

    pub(crate) fn live_candle(&self) -> Candle {
        self.series.last().copied().unwrap_or(Candle {
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 0.0,
        })
    }

    pub(crate) fn tick(&mut self, delta: Duration) -> MarketTick {
        self.elapsed += delta;
        let timeframe = self.timeframe.duration();
        let mut new_candle = false;
        while self.elapsed >= timeframe {
            self.elapsed -= timeframe;
            let last_close = self
                .series
                .last()
                .map(|candle| candle.close)
                .unwrap_or(100.0);
            self.series.push(Candle {
                open: last_close,
                high: last_close,
                low: last_close,
                close: last_close,
                volume: 0.02,
            });
            if self.series.len() > HISTORY + 1 {
                self.series.remove(0);
            }
            new_candle = true;
        }
        let dt = delta.as_secs_f64();
        let volatility = 1.9 * (timeframe.as_secs_f64() / 5.0).sqrt();
        let step = (self.rng.next() - 0.5) * 2.0 * volatility * dt.sqrt();
        if let Some(candle) = self.series.last_mut() {
            candle.close += step;
            candle.high = candle.high.max(candle.close);
            candle.low = candle.low.min(candle.close);
            candle.volume += self.rng.next() * 2.5 * dt;
        }
        MarketTick {
            new_candle,
            candle: self.live_candle(),
        }
    }
}
