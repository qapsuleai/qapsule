//! Bar fixture parsing and a deterministic synthetic bar generator.
//!
//! Since R1 the bar type and the fixture format are `qp-schema`'s (`Bar`, `Series`).

use qp_schema::{Bar, Instrument, Series, SeriesData, SeriesSource};

use crate::RenderError;

/// Parses a bar series document (`fixtures/bars/*.json`: a `qp_schema::Series` with
/// embedded bars) and validates it with `qp_schema::validate_bars`.
///
/// # Errors
/// Returns an error for malformed JSON, a series without bars, an empty bar list,
/// non-finite values, `h < max(o, c)` / `l > min(o, c)`, or timestamps that are not
/// strictly increasing.
pub fn parse_bars(json: &str) -> Result<Series, RenderError> {
    qp_schema::parse_bar_series(json).map_err(RenderError::new)
}

/// The bars of a series, or an error if it carries none.
pub(crate) fn bars_of(series: &Series) -> Result<&[Bar], RenderError> {
    series
        .bars()
        .ok_or_else(|| RenderError::new("series has no embedded bars"))
}

/// Minimal 64-bit LCG (Knuth MMIX constants). Integer-only, so identical on every target.
pub(crate) struct Lcg(pub(crate) u64);

impl Lcg {
    pub(crate) fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Truncation intended: take the high 32 bits.
        (self.0 >> 32) as u32
    }

    /// Uniform in [-1, 1). Built from an integer, then one exact division.
    pub(crate) fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 2_147_483_648.0 - 1.0
    }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

const DAY_MS: i64 = 86_400_000;
/// 2025-01-02 00:00:00 UTC (a Thursday).
const START_MS: i64 = 1_735_776_000_000;

/// Generates `n` deterministic daily bars (weekdays only) from `seed`.
///
/// Uses only integer arithmetic and IEEE basic float operations (`+ - * /`, `round`),
/// which are correctly rounded on every target, so the output is identical everywhere.
#[must_use]
pub fn synthetic_bars(n: usize, seed: u64) -> Series {
    let mut rng = Lcg(seed);
    let mut bars = Vec::with_capacity(n);
    let mut close = 100.0_f64;
    let mut day = 0_i64;
    for _ in 0..n {
        // Skip weekends: day 0 is a Thursday, so (day + 3) % 7 gives Mon=0..Sun=6.
        while (day + 3).rem_euclid(7) >= 5 {
            day += 1;
        }
        let open = round2(close * (1.0 + 0.004 * rng.unit()));
        let drift = 0.0004;
        let c = round2(open * (1.0 + drift + 0.018 * rng.unit()));
        let hi = round2(open.max(c) * (1.0 + 0.008 * (rng.unit() + 1.0)));
        let lo = round2(open.min(c) * (1.0 - 0.008 * (rng.unit() + 1.0)));
        let vol = round2(1_000_000.0 * (1.5 + rng.unit()));
        bars.push(Bar {
            t: START_MS + day * DAY_MS,
            o: open,
            h: hi,
            l: lo,
            c,
            v: vol,
        });
        close = c;
        day += 1;
    }
    Series {
        id: "sample".to_owned(),
        instrument: Some(Instrument {
            symbol: "SAMPLE".to_owned(),
            exchange: None,
            calendar: None,
        }),
        interval: Some("1d".to_owned()),
        source: SeriesSource::Inline,
        data: Some(SeriesData::Bars { bars }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_is_valid_and_stable() {
        let a = synthetic_bars(500, 7);
        let b = synthetic_bars(500, 7);
        assert_eq!(a, b);
        qp_schema::validate_bars(a.bars().unwrap()).unwrap();
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_bars("{").is_err());
        let empty = r#"{"id":"x","source":{"kind":"inline"},"data":{"kind":"bars","bars":[]}}"#;
        assert!(parse_bars(empty).is_err());
        let bad = r#"{"id":"x","source":{"kind":"inline"},"data":{"kind":"bars","bars":[{"t":1,"o":1,"h":0.5,"l":0.4,"c":1,"v":1}]}}"#;
        assert!(parse_bars(bad).is_err());
        let no_bars = r#"{"id":"x","source":{"kind":"inline"}}"#;
        assert!(parse_bars(no_bars).is_err());
    }
}
