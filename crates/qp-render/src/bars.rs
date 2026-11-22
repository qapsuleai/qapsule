//! Bar fixture parsing and a deterministic synthetic bar generator.
//!
//! Since R1 the bar type and the fixture format are `qp-schema`'s (`Bar`, `Series`).

use qp_schema::Series;

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

#[cfg(test)]
mod tests {
    use super::*;

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
