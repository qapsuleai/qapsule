//! Bar fixture format and a deterministic synthetic bar generator.
//!
//! R0 note: this is a private, minimal format owned by `qp-render`. Unifying it with
//! `qp-schema`'s series type is a later-round job (see `SPIKE.md`).

use serde::{Deserialize, Serialize};

use crate::RenderError;

/// One OHLCV bar. `t` is the bar open time in epoch milliseconds (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    pub t: i64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
}

/// The `fixtures/bars/*.json` file format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BarsFile {
    pub symbol: String,
    pub interval: String,
    pub bars: Vec<Bar>,
}

/// Parses a bars JSON document and validates it.
///
/// # Errors
/// Returns an error for malformed JSON, an empty bar list, non-finite values,
/// `h < max(o, c)` / `l > min(o, c)`, or timestamps that are not strictly increasing.
pub fn parse_bars(json: &str) -> Result<BarsFile, RenderError> {
    let file: BarsFile = serde_json::from_str(json)
        .map_err(|e| RenderError::new(format!("invalid bars JSON: {e}")))?;
    validate(&file)?;
    Ok(file)
}

fn validate(file: &BarsFile) -> Result<(), RenderError> {
    if file.bars.is_empty() {
        return Err(RenderError::new("bars file has no bars"));
    }
    let mut prev_t: Option<i64> = None;
    for (i, b) in file.bars.iter().enumerate() {
        let vals = [b.o, b.h, b.l, b.c, b.v];
        if vals.iter().any(|v| !v.is_finite()) {
            return Err(RenderError::new(format!("bar {i}: non-finite value")));
        }
        if b.h < b.o.max(b.c) || b.l > b.o.min(b.c) || b.l > b.h {
            return Err(RenderError::new(format!("bar {i}: inconsistent OHLC")));
        }
        if let Some(p) = prev_t {
            if b.t <= p {
                return Err(RenderError::new(format!(
                    "bar {i}: timestamp not increasing"
                )));
            }
        }
        prev_t = Some(b.t);
    }
    Ok(())
}

/// Minimal 64-bit LCG (Knuth MMIX constants). Integer-only, so identical on every target.
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Truncation intended: take the high 32 bits.
        (self.0 >> 32) as u32
    }

    /// Uniform in [-1, 1). Built from an integer, then one exact division.
    fn unit(&mut self) -> f64 {
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
pub fn synthetic_bars(n: usize, seed: u64) -> BarsFile {
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
    BarsFile {
        symbol: "SAMPLE".to_owned(),
        interval: "1d".to_owned(),
        bars,
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
        validate(&a).unwrap();
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_bars("{").is_err());
        assert!(parse_bars(r#"{"symbol":"X","interval":"1d","bars":[]}"#).is_err());
        let bad =
            r#"{"symbol":"X","interval":"1d","bars":[{"t":1,"o":1,"h":0.5,"l":0.4,"c":1,"v":1}]}"#;
        assert!(parse_bars(bad).is_err());
    }
}
