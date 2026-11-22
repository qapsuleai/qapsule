//! The glyph spec of a story (K30 channels): freshness, strength and ledger mark.

use super::*;

/// Bars after the last transition at which freshness is one half.
pub const FRESHNESS_HALF_BARS: f64 = qp_schema::params::system::GLYPH_FRESHNESS_HALF_BARS_DEFAULT;

/// Glyph parameters of a story at `now_bar` (K30 channels).
///
/// - confidence: `realized / n` of `ledger_at_birth` when `n >= min_ledger_n` (and
///   `n > 0`), else `None`;
/// - freshness: `h / (h + age)` with `h =` [`FRESHNESS_HALF_BARS`] and `age = now_bar -
///   last_bar` (0 if negative): 1 on the transition bar, 0.5 after `h` bars, → 0;
/// - phase from the stage; confirmations = number of families (clamped to 1–5);
///   server mark from the trust path.
#[must_use]
pub fn glyph_spec(story: &Story, now_bar: u64, min_ledger_n: u32) -> GlyphSpec {
    let lab = story.ledger_at_birth;
    let confidence = if lab.n >= min_ledger_n && lab.n > 0 {
        Some(f64::from(lab.realized) / f64::from(lab.n))
    } else {
        None
    };
    #[allow(clippy::cast_precision_loss)]
    let age = now_bar.saturating_sub(story.last_bar) as f64;
    let freshness = FRESHNESS_HALF_BARS / (FRESHNESS_HALF_BARS + age);
    let confirmations = u8::try_from(story.families.len().clamp(1, 5)).unwrap_or(5);
    GlyphSpec {
        kind: story.kind,
        direction: story.direction,
        strength: story.strength,
        confidence,
        phase: GlyphPhase::from(story.stage),
        freshness,
        confirmations,
        server: story.trust == Trust::Server,
        validity: None,
    }
}

/// [`glyph_spec`] with the story's validity today (R4, K42/A28), which the caller
/// computes from the bars ([`crate::sentence::validity_now`]): the glyph then says what
/// the headline says.
#[must_use]
pub fn glyph_spec_at(
    story: &Story,
    now_bar: u64,
    min_ledger_n: u32,
    validity: Validity,
) -> GlyphSpec {
    GlyphSpec {
        validity: Some(validity),
        ..glyph_spec(story, now_bar, min_ledger_n)
    }
}

#[cfg(test)]
mod params_binding {
    // RG1/K3: the constant reads `qp_schema::params` and keeps its old literal.
    #[test]
    fn capsule_freshness_half_bars_binds_registry() {
        assert_eq!(super::FRESHNESS_HALF_BARS.to_bits(), 5.0f64.to_bits());
    }
}
