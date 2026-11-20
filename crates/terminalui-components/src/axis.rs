//! Shared numeric axis helpers for charts.
//!
//! The domain starts at zero and ends at a rounded bound above the data, so a
//! series peaking at 859k reads against a 0..1m axis in 100k steps rather than
//! against its own min and max.

/// Choose a rounded upper bound for a zero-based axis with `divisions` steps.
pub fn nice_max(max: f64, divisions: usize) -> f64 {
    let divisions = divisions.max(1) as f64;
    if !max.is_finite() || max <= 0.0 {
        return divisions;
    }
    let rough = max / divisions;
    if !rough.is_finite() || rough <= 0.0 {
        return divisions;
    }
    let magnitude = 10f64.powf(rough.log10().floor());
    let normalized = rough / magnitude;
    let step = [1.0, 2.0, 3.0, 5.0, 10.0]
        .into_iter()
        .find(|candidate| *candidate >= normalized)
        .unwrap_or(10.0);
    (step * magnitude * divisions).max(divisions)
}

/// A short, human-readable axis label: `1m`, `250k`, `12`, `0.5`.
pub fn format_value(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude == 0.0 {
        return "0".to_owned();
    }
    if magnitude >= 1_000_000.0 {
        compact(value / 1_000_000.0, "m")
    } else if magnitude >= 1_000.0 {
        compact(value / 1_000.0, "k")
    } else if magnitude >= 1.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn compact(value: f64, suffix: &str) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if (rounded - rounded.round()).abs() < f64::EPSILON {
        format!("{}{suffix}", rounded.round() as i64)
    } else {
        format!("{rounded:.1}{suffix}")
    }
}

/// The values painted on an axis of `divisions` steps from zero to `high`.
pub fn ticks(high: f64, divisions: usize) -> Vec<f64> {
    (0..=divisions)
        .map(|step| high * step as f64 / divisions.max(1) as f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{format_value, nice_max, ticks};

    #[test]
    fn a_series_peaking_at_859k_gets_a_one_million_axis() {
        let high = nice_max(859_000.0, 10);
        assert_eq!(high, 1_000_000.0);
        assert_eq!(ticks(high, 10)[1], 100_000.0);
    }

    #[test]
    fn a_series_peaking_at_294k_gets_a_three_hundred_k_axis() {
        assert_eq!(nice_max(294_000.0, 10), 300_000.0);
    }

    #[test]
    fn small_and_missing_values_still_get_an_axis() {
        assert_eq!(nice_max(0.0, 10), 10.0);
        assert_eq!(nice_max(f64::NAN, 10), 10.0);
        assert!(nice_max(7.0, 10) >= 7.0);
    }

    #[test]
    fn labels_are_short() {
        assert_eq!(format_value(1_000_000.0), "1m");
        assert_eq!(format_value(250_000.0), "250k");
        assert_eq!(format_value(12.0), "12");
        assert_eq!(format_value(0.0), "0");
    }
}
