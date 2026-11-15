//! The M1 glyph set (SPEC "Glif kümesi"): the core set `K` and the channel pairs.
//!
//! `K` = 12 kind × direction combinations (SPEC says 13; its own list has 12), each with phase `final`, strength 0.6,
//! confidence 0.6, one confirmation, freshness 1, no server mark. A channel pair is two
//! glyphs that differ in exactly one channel, built from every `K` member:
//!
//! | channel | values (all pairs among them) |
//! |---------|-------------------------------|
//! | `direction` | the directions of the same kind present in `K` |
//! | `phase` | candidate, final, realized, failed, retracted |
//! | `strength` | 0.2, 0.9 |
//! | `confirmations` | 1, 2, 3 |
//! | `confidence` | none, 0.3, 0.8 |
//! | `freshness` | 1, 0.3 |
//! | `server` | off, on |

use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};

/// The kind × direction combinations of `K`, in canonical order.
///
/// SPEC lists "devam↑↓, dönüş↑↓, kırılım↑↓, sıkışma·, aşırılık↑↓, akış↑↓· (13)"; that list
/// has **12** members (2+2+2+1+2+3). The harness uses the list, i.e. 12 (README, "Sapmalar").
pub const K_KIND_DIRS: [(MeaningKind, Direction); 12] = [
    (MeaningKind::Continuation, Direction::Up),
    (MeaningKind::Continuation, Direction::Down),
    (MeaningKind::Reversal, Direction::Up),
    (MeaningKind::Reversal, Direction::Down),
    (MeaningKind::Breakout, Direction::Up),
    (MeaningKind::Breakout, Direction::Down),
    (MeaningKind::Squeeze, Direction::Neutral),
    (MeaningKind::Extreme, Direction::Up),
    (MeaningKind::Extreme, Direction::Down),
    (MeaningKind::Flow, Direction::Up),
    (MeaningKind::Flow, Direction::Down),
    (MeaningKind::Flow, Direction::Neutral),
];

/// All phases, canonical order.
pub const PHASES: [GlyphPhase; 5] = [
    GlyphPhase::Candidate,
    GlyphPhase::Final,
    GlyphPhase::Realized,
    GlyphPhase::Failed,
    GlyphPhase::Retracted,
];

/// Channel names, report order.
pub const CHANNELS: [&str; 7] = [
    "direction",
    "phase",
    "strength",
    "confirmations",
    "confidence",
    "freshness",
    "server",
];

/// `up` / `down` / `neutral`.
#[must_use]
pub fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

/// `candidate` / `final` / `realized` / `failed` / `retracted`.
#[must_use]
pub fn phase_name(p: GlyphPhase) -> &'static str {
    match p {
        GlyphPhase::Candidate => "candidate",
        GlyphPhase::Final => "final",
        GlyphPhase::Realized => "realized",
        GlyphPhase::Failed => "failed",
        GlyphPhase::Retracted => "retracted",
    }
}

/// Reading-test strength class: `low` below 0.5, else `high`.
#[must_use]
pub fn strength_class(s: f64) -> &'static str {
    if s < 0.5 {
        "low"
    } else {
        "high"
    }
}

/// A `K` member: the base glyph of a kind × direction.
#[must_use]
pub fn base(kind: MeaningKind, direction: Direction) -> GlyphSpec {
    GlyphSpec {
        kind,
        direction,
        strength: 0.6,
        confidence: Some(0.6),
        phase: GlyphPhase::Final,
        freshness: 1.0,
        confirmations: 1,
        server: false,
    }
}

/// The kind × direction pairs of `K`.
#[must_use]
pub fn k_kind_dirs() -> Vec<(MeaningKind, Direction)> {
    K_KIND_DIRS.to_vec()
}

/// The core set `K` (12 glyphs).
#[must_use]
pub fn core_set() -> Vec<GlyphSpec> {
    k_kind_dirs().into_iter().map(|(k, d)| base(k, d)).collect()
}

/// Canonical, human-readable id of a spec (`breakout.up|final|s0.6|c0.6|n1|f1|-`).
#[must_use]
pub fn spec_id(s: &GlyphSpec) -> String {
    format!(
        "{}.{}|{}|s{}|c{}|n{}|f{}|{}",
        s.kind.name(),
        dir_name(s.direction),
        phase_name(s.phase),
        s.strength,
        s.confidence
            .map_or_else(|| "none".to_owned(), |c| c.to_string()),
        s.confirmations,
        s.freshness,
        if s.server { "srv" } else { "-" }
    )
}

/// Short id of a `K` member (`breakout.up`).
#[must_use]
pub fn kd_id(s: &GlyphSpec) -> String {
    format!("{}.{}", s.kind.name(), dir_name(s.direction))
}

/// Two glyphs differing in one channel.
#[derive(Debug, Clone)]
pub struct ChannelPair {
    /// Channel name (one of [`CHANNELS`]).
    pub channel: &'static str,
    /// First glyph.
    pub a: GlyphSpec,
    /// Second glyph.
    pub b: GlyphSpec,
}

fn pairs_of(channel: &'static str, variants: &[GlyphSpec], out: &mut Vec<ChannelPair>) {
    for i in 0..variants.len() {
        for j in i + 1..variants.len() {
            out.push(ChannelPair {
                channel,
                a: variants[i].clone(),
                b: variants[j].clone(),
            });
        }
    }
}

/// Every channel pair, deterministic order (channel, then `K` order).
#[must_use]
pub fn channel_pairs() -> Vec<ChannelPair> {
    let k = core_set();
    let mut out = Vec::new();
    // direction: same kind, different direction, among K.
    for (i, a) in k.iter().enumerate() {
        for b in &k[i + 1..] {
            if a.kind == b.kind {
                out.push(ChannelPair {
                    channel: "direction",
                    a: a.clone(),
                    b: b.clone(),
                });
            }
        }
    }
    for b in &k {
        let v: Vec<GlyphSpec> = PHASES
            .iter()
            .map(|&p| GlyphSpec {
                phase: p,
                ..b.clone()
            })
            .collect();
        pairs_of("phase", &v, &mut out);
    }
    for b in &k {
        let v: Vec<GlyphSpec> = [0.2, 0.9]
            .iter()
            .map(|&s| GlyphSpec {
                strength: s,
                ..b.clone()
            })
            .collect();
        pairs_of("strength", &v, &mut out);
    }
    for b in &k {
        let v: Vec<GlyphSpec> = [1u8, 2, 3]
            .iter()
            .map(|&n| GlyphSpec {
                confirmations: n,
                ..b.clone()
            })
            .collect();
        pairs_of("confirmations", &v, &mut out);
    }
    for b in &k {
        let v: Vec<GlyphSpec> = [None, Some(0.3), Some(0.8)]
            .iter()
            .map(|&c| GlyphSpec {
                confidence: c,
                ..b.clone()
            })
            .collect();
        pairs_of("confidence", &v, &mut out);
    }
    for b in &k {
        let v: Vec<GlyphSpec> = [1.0, 0.3]
            .iter()
            .map(|&f| GlyphSpec {
                freshness: f,
                ..b.clone()
            })
            .collect();
        pairs_of("freshness", &v, &mut out);
    }
    for b in &k {
        let v: Vec<GlyphSpec> = [false, true]
            .iter()
            .map(|&s| GlyphSpec {
                server: s,
                ..b.clone()
            })
            .collect();
        pairs_of("server", &v, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_sizes() {
        assert_eq!(core_set().len(), 12);
        let p = channel_pairs();
        let n = |c: &str| p.iter().filter(|x| x.channel == c).count();
        assert_eq!(n("direction"), 7);
        assert_eq!(n("phase"), 120);
        assert_eq!(n("strength"), 12);
        assert_eq!(n("confirmations"), 36);
        assert_eq!(n("confidence"), 36);
        assert_eq!(n("freshness"), 12);
        assert_eq!(n("server"), 12);
        for x in &p {
            assert_ne!(spec_id(&x.a), spec_id(&x.b));
        }
    }

    #[test]
    fn deterministic_ids() {
        let a: Vec<String> = core_set().iter().map(spec_id).collect();
        let b: Vec<String> = core_set().iter().map(spec_id).collect();
        assert_eq!(a, b);
        assert_eq!(a[4], "breakout.up|final|s0.6|c0.6|n1|f1|-");
    }
}
