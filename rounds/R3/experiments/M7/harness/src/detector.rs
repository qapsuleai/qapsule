//! The reference detector shared by every approach and by the oracle.
//!
//! - `rsi_cross`: RSI(period) crosses above `upper` (`prev <= upper && cur > upper`,
//!   direction `up`) or below `lower` (`prev >= lower && cur < lower`, direction `down`)
//!   at a bar. On a closed bar the claim is **final**; on the live (last) bar it is
//!   **provisional**.
//! - `rsi_invalidation` (S18, optional): RSI crosses below `level` at bar `u` and there
//!   is a cross above `upper` at a bar in `[u - lookback, u - 1]` — the reference
//!   invalidation condition of an "overbought" finding resting on that cross.
//! - `rsi_pair_overbought` (S16, optional): RSI of binding 0 and of binding 1 both above
//!   `upper` at the same bar time and not both at the bar before (a two-binding claim).
//!
//! Claims are identified by meaning ([`ClaimKey`]), never by an id assigned by a side.
//! RSI is computed from the first bar held (the anchor) — see `series.rs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::series::SeriesState;
use crate::spec::DetectorSpec;

/// Direction of a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Upward.
    Up,
    /// Downward.
    Down,
}

/// A claim identified by meaning: which detector with which parameters said what about
/// which bar of which binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClaimKey {
    /// Detector name (`rsi_cross`, `rsi_invalidation`, `rsi_pair_overbought`).
    pub detector: String,
    /// Canonical parameter string, e.g. `period=14,level=70`.
    pub params: String,
    /// Binding id (`px0`), or `px0+px1` for a pair claim.
    pub binding: String,
    /// Bar open time (epoch ms) — the identity of the bar across sides.
    pub bar_t: i64,
    /// Direction.
    pub direction: Direction,
}

impl ClaimKey {
    /// Compact text form used in the display log.
    #[must_use]
    pub fn text(&self) -> String {
        format!(
            "{}[{}]@{}:{}:{}",
            self.detector,
            self.params,
            self.binding,
            self.bar_t,
            match self.direction {
                Direction::Up => "up",
                Direction::Down => "down",
            }
        )
    }

    /// Binding ids this claim rests on.
    #[must_use]
    pub fn bindings(&self) -> Vec<&str> {
        self.binding.split('+').collect()
    }
}

/// Maturity of a claim on one side's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    /// On the live (not yet closed) bar.
    Provisional,
    /// On closed bars only.
    Final,
}

/// Bars a claim's evidence lies on (the user must see them to see the evidence).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvidenceRange {
    /// Binding id.
    pub binding: String,
    /// First bar time (inclusive).
    pub from_t: i64,
    /// Last bar time (inclusive): the claim's bar.
    pub to_t: i64,
}

/// A claim as one side's data supports it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// Identity.
    pub key: ClaimKey,
    /// Provisional or final on this side's data.
    pub maturity: Maturity,
    /// Evidence values (`prev`, `value`, `level`; pair: `a`, `b`, `level`).
    pub values: BTreeMap<String, f64>,
    /// Evidence bars.
    pub evidence: Vec<EvidenceRange>,
}

/// A change of a side's claim set.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimDelta {
    /// Identity.
    pub key: ClaimKey,
    /// Before (None: did not hold).
    pub before: Option<Claim>,
    /// After (None: no longer holds).
    pub after: Option<Claim>,
}

/// Detector configuration with the derived parameter strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectorCfg {
    /// The spec.
    pub spec: DetectorSpec,
}

/// Detector names.
pub const CROSS: &str = "rsi_cross";
/// Invalidation detector.
pub const INVALIDATION: &str = "rsi_invalidation";
/// Pair detector.
pub const PAIR: &str = "rsi_pair_overbought";

fn fmt_num(x: f64) -> String {
    format!("{x}")
}

impl DetectorCfg {
    /// From a spec.
    #[must_use]
    pub fn new(spec: DetectorSpec) -> Self {
        Self { spec }
    }

    fn cross_params(&self, level: f64) -> String {
        format!("period={},level={}", self.spec.period, fmt_num(level))
    }

    fn inval_params(&self) -> String {
        let inv = self.spec.invalidation.clone().unwrap_or_default();
        format!(
            "period={},level={},upper={},lookback={}",
            self.spec.period,
            fmt_num(inv.level),
            fmt_num(self.spec.upper),
            inv.lookback
        )
    }

    fn pair_params(&self) -> String {
        format!(
            "period={},level={}",
            self.spec.period,
            fmt_num(self.spec.upper)
        )
    }

    fn evidence(&self, id: &str, s: &SeriesState, i: usize) -> EvidenceRange {
        let from = i.saturating_sub(self.spec.period as usize);
        EvidenceRange {
            binding: id.to_owned(),
            from_t: s.bars()[from].t,
            to_t: s.bars()[i].t,
        }
    }

    fn maturity(s: &SeriesState, i: usize) -> Maturity {
        if i + 1 < s.bars().len() {
            Maturity::Final
        } else {
            Maturity::Provisional
        }
    }

    /// Single-binding claims at bar index `i` of series `s` (binding `id`).
    pub fn claims_at(&self, id: &str, s: &SeriesState, i: usize, out: &mut Vec<Claim>) {
        let rsi = s.rsi();
        if i == 0 || i >= rsi.len() {
            return;
        }
        let (Some(prev), Some(cur)) = (rsi[i - 1], rsi[i]) else {
            return;
        };
        let t = s.bars()[i].t;
        let mk = |det: &str, params: String, dir: Direction, level: f64| Claim {
            key: ClaimKey {
                detector: det.to_owned(),
                params,
                binding: id.to_owned(),
                bar_t: t,
                direction: dir,
            },
            maturity: Self::maturity(s, i),
            values: [
                ("prev".to_owned(), prev),
                ("value".to_owned(), cur),
                ("level".to_owned(), level),
            ]
            .into_iter()
            .collect(),
            evidence: vec![self.evidence(id, s, i)],
        };
        let up = self.spec.upper;
        let lo = self.spec.lower;
        if prev <= up && cur > up {
            out.push(mk(CROSS, self.cross_params(up), Direction::Up, up));
        }
        if prev >= lo && cur < lo {
            out.push(mk(CROSS, self.cross_params(lo), Direction::Down, lo));
        }
        if let Some(inv) = &self.spec.invalidation {
            if prev >= inv.level && cur < inv.level {
                let lb = inv.lookback as usize;
                let start = i.saturating_sub(lb).max(1);
                let crossed = (start..i).any(|k| match (rsi[k - 1], rsi[k]) {
                    (Some(p), Some(c)) => p <= up && c > up,
                    _ => false,
                });
                if crossed {
                    let mut c = mk(
                        INVALIDATION,
                        self.inval_params(),
                        Direction::Down,
                        inv.level,
                    );
                    c.evidence = vec![EvidenceRange {
                        binding: id.to_owned(),
                        from_t: s.bars()[i.saturating_sub(lb + self.spec.period as usize)].t,
                        to_t: t,
                    }];
                    out.push(c);
                }
            }
        }
    }

    /// Pair claim at time `t` (bindings `ids[0]`, `ids[1]`).
    pub fn pair_at(&self, ids: [&str; 2], s: [&SeriesState; 2], t: i64, out: &mut Vec<Claim>) {
        let (Some(i), Some(j)) = (s[0].index_of(t), s[1].index_of(t)) else {
            return;
        };
        if i == 0 || j == 0 {
            return;
        }
        let up = self.spec.upper;
        let (ra, rb, pa, pb) = (
            s[0].rsi()[i],
            s[1].rsi()[j],
            s[0].rsi()[i - 1],
            s[1].rsi()[j - 1],
        );
        let (Some(ra), Some(rb), Some(pa), Some(pb)) = (ra, rb, pa, pb) else {
            return;
        };
        if ra > up && rb > up && !(pa > up && pb > up) {
            let maturity = if Self::maturity(s[0], i) == Maturity::Final
                && Self::maturity(s[1], j) == Maturity::Final
            {
                Maturity::Final
            } else {
                Maturity::Provisional
            };
            out.push(Claim {
                key: ClaimKey {
                    detector: PAIR.to_owned(),
                    params: self.pair_params(),
                    binding: format!("{}+{}", ids[0], ids[1]),
                    bar_t: t,
                    direction: Direction::Up,
                },
                maturity,
                values: [
                    ("a".to_owned(), ra),
                    ("b".to_owned(), rb),
                    ("level".to_owned(), up),
                ]
                .into_iter()
                .collect(),
                evidence: vec![
                    self.evidence(ids[0], s[0], i),
                    self.evidence(ids[1], s[1], j),
                ],
            });
        }
    }

    /// Recomputes the claims of binding `b` (and pair claims involving it) for bars
    /// with time `>= from_t`, updating `claims` and appending the differences to
    /// `deltas`. Returns the number of bars evaluated (work units).
    pub fn recompute(
        &self,
        ids: &[String],
        series: &[&SeriesState],
        b: usize,
        from_t: i64,
        claims: &mut BTreeMap<ClaimKey, Claim>,
        deltas: &mut Vec<ClaimDelta>,
    ) -> u64 {
        let id = ids[b].as_str();
        let pair_id = if self.spec.pair && ids.len() >= 2 && b < 2 {
            Some(format!("{}+{}", ids[0], ids[1]))
        } else {
            None
        };
        // Old claims in range.
        let old: Vec<ClaimKey> = claims
            .keys()
            .filter(|k| {
                k.bar_t >= from_t && (k.binding == id || pair_id.as_deref() == Some(&k.binding))
            })
            .cloned()
            .collect();
        let s = series[b];
        let start = s.bars().partition_point(|x| x.t < from_t);
        let mut fresh = Vec::new();
        for i in start..s.bars().len() {
            self.claims_at(id, s, i, &mut fresh);
            if pair_id.is_some() {
                self.pair_at(
                    [&ids[0], &ids[1]],
                    [series[0], series[1]],
                    s.bars()[i].t,
                    &mut fresh,
                );
            }
        }
        let work = (s.bars().len() - start) as u64;
        let mut fresh_map: BTreeMap<ClaimKey, Claim> =
            fresh.into_iter().map(|c| (c.key.clone(), c)).collect();
        for k in old {
            let before = claims.remove(&k);
            match fresh_map.remove(&k) {
                Some(after) => {
                    if before.as_ref() != Some(&after) {
                        deltas.push(ClaimDelta {
                            key: k.clone(),
                            before,
                            after: Some(after.clone()),
                        });
                    }
                    claims.insert(k, after);
                }
                None => deltas.push(ClaimDelta {
                    key: k,
                    before,
                    after: None,
                }),
            }
        }
        for (k, c) in fresh_map {
            deltas.push(ClaimDelta {
                key: k.clone(),
                before: None,
                after: Some(c.clone()),
            });
            claims.insert(k, c);
        }
        work
    }

    /// All claims on the given series, computed from scratch (batch reference).
    #[must_use]
    pub fn claims_batch(
        &self,
        ids: &[String],
        series: &[&SeriesState],
    ) -> BTreeMap<ClaimKey, Claim> {
        let mut out = Vec::new();
        for (b, s) in series.iter().enumerate() {
            for i in 0..s.bars().len() {
                self.claims_at(&ids[b], s, i, &mut out);
                if self.spec.pair && b == 0 && series.len() >= 2 {
                    self.pair_at(
                        [&ids[0], &ids[1]],
                        [series[0], series[1]],
                        s.bars()[i].t,
                        &mut out,
                    );
                }
            }
        }
        out.into_iter().map(|c| (c.key.clone(), c)).collect()
    }
}
