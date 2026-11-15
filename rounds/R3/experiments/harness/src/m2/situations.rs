//! The M2 situation set (SPEC "Durum kümesi"): 72 seeded situations + the K26 situation,
//! each with its hidden truth.
//!
//! Factors: time kind (event / state / expectation) × kind (six) gives 18 cells; each cell
//! gets 4 situations with bars-ago 0, 1, 5 and 30 (balanced). Across the 72: each stage
//! (born, confirmed, weakened, realized, invalidated, expired) 12 times, `redacted` 18
//! times, ledger small (n = 8) / large (n = 140) 36 times each, all assigned by seeded
//! shuffles. `still_holds` follows from the time kind and stage (rules below, with a
//! seeded coin where free). Every number a realizer can see comes from the `Story` / `Fact`
//! values built here (engine-consistent shapes: ids `fx.<instrument>.<detector>.<code>.<bar>`,
//! families sorted, history `born` then the stage transition, ledger at birth).
//!
//! `still_holds` rules: event → `None`; state → `Some(false)` when invalidated/expired,
//! `Some(true)` when bars-ago is 0, else a coin; expectation → `Some(false)` when
//! invalidated/expired, `None` when realized, else `Some(true)` (2/3) or `None` (1/3).
//!
//! Truth: **what** = kind + direction; **when** = `past` with bars-ago (event, ended
//! state), `now` (state that still holds), `conditional` (expectation); **still valid** =
//! `no` when invalidated/expired, else from `still_holds` (`yes` / `no` / `unknown`).

use std::collections::BTreeMap;

use qp_capsule::SentenceInput;
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, LedgerStat, License, LicenseClass,
    MeaningCondition, MeaningKind, Stage, Story, TimeKind, Transition, Trust,
};
use qp_schema::{Author, CompareOp, Fact, Provenance};
use serde::{Deserialize, Serialize};

use crate::io::round;
use crate::rng::Rng;

/// Seed of the situation set (fixed: the set is part of the harness).
pub const SET_SEED: u64 = 3;
/// Bars-ago levels.
pub const BARS_AGO: [u64; 4] = [0, 1, 5, 30];
/// Stages used.
pub const STAGES: [Stage; 6] = [
    Stage::Born,
    Stage::Confirmed,
    Stage::Weakened,
    Stage::Realized,
    Stage::Invalidated,
    Stage::Expired,
];
const TIMES: [TimeKind; 3] = [TimeKind::Event, TimeKind::State, TimeKind::Expectation];
const T0: i64 = 1_735_689_600_000; // 2025-01-01T00:00Z
const DAY: i64 = 86_400_000;
const INSTRUMENTS: [&str; 12] = [
    "THYAO", "GARAN", "ASELS", "AKBNK", "EREGL", "KCHOL", "SISE", "TUPRS", "BIMAS", "FROTO",
    "SAHOL", "PGSUS",
];

/// Hidden truth of a situation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Truth {
    /// Kind name.
    pub kind: String,
    /// `up` / `down` / `neutral`.
    pub direction: String,
    /// `past` / `now` / `conditional`.
    pub when: String,
    /// Bars ago when `when = past`.
    pub bars_ago: Option<u64>,
    /// `yes` / `no` / `unknown`.
    pub still_valid: String,
}

/// The factor levels of a situation (for breakdowns).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Factors {
    /// `event` / `state` / `expectation`.
    pub time: String,
    /// Kind name.
    pub kind: String,
    /// Stage name.
    pub stage: String,
    /// Bars since the lead fact.
    pub bars_ago: u64,
    /// `yes` / `no` / `na`.
    pub still_holds: String,
    /// Licensed, unentitled reader.
    pub redacted: bool,
    /// `small` / `large`.
    pub ledger: String,
}

/// One situation.
#[derive(Debug, Clone, PartialEq)]
pub struct Situation {
    /// `s01` … `s72`, `k26`.
    pub id: String,
    /// The story.
    pub story: Story,
    /// Lead fact.
    pub lead: Fact,
    /// Latest fact.
    pub latest: Fact,
    /// Bar the sentence is for.
    pub now_bar: u64,
    /// Its time.
    pub now_t: i64,
    /// See `SentenceInput::still_holds`.
    pub still_holds: Option<bool>,
    /// See `SentenceInput::current`.
    pub current: Option<f64>,
    /// See `SentenceInput::redacted`.
    pub redacted: bool,
    /// Hidden truth.
    pub truth: Truth,
    /// Factor levels.
    pub factors: Factors,
}

impl Situation {
    /// The realizer input.
    #[must_use]
    pub fn input(&self) -> SentenceInput<'_> {
        SentenceInput {
            story: &self.story,
            lead: &self.lead,
            latest: &self.latest,
            now_bar: self.now_bar,
            now_t: self.now_t,
            still_holds: self.still_holds,
            current: self.current,
            redacted: self.redacted,
        }
    }

    /// Bars since the lead fact.
    #[must_use]
    pub fn bars_ago(&self) -> u64 {
        self.now_bar.saturating_sub(self.lead.bar)
    }
}

/// Stage name (`born`, …).
#[must_use]
pub fn stage_name(s: Stage) -> &'static str {
    match s {
        Stage::Candidate => "candidate",
        Stage::Born => "born",
        Stage::Strengthened => "strengthened",
        Stage::Weakened => "weakened",
        Stage::Confirmed => "confirmed",
        Stage::Realized => "realized",
        Stage::Invalidated => "invalidated",
        Stage::Expired => "expired",
        Stage::Retracted => "retracted",
    }
}

/// Time-kind name.
#[must_use]
pub fn time_name(t: TimeKind) -> &'static str {
    match t {
        TimeKind::Event => "event",
        TimeKind::State => "state",
        TimeKind::Expectation => "expectation",
    }
}

/// Direction name.
#[must_use]
pub fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

/// A draft: the factor levels and values of one situation before the story is built.
#[derive(Debug, Clone)]
pub struct Draft {
    /// Id.
    pub id: String,
    /// Instrument.
    pub instrument: String,
    /// Kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Time kind.
    pub time: TimeKind,
    /// Stage.
    pub stage: Stage,
    /// Bar now.
    pub now_bar: u64,
    /// Bars since the lead fact.
    pub bars_ago: u64,
    /// Still holds.
    pub still_holds: Option<bool>,
    /// Redacted.
    pub redacted: bool,
    /// Ledger at birth.
    pub ledger: LedgerStat,
    /// Lead evidence.
    pub evidence: BTreeMap<String, f64>,
    /// Current subject value (state, not redacted).
    pub current: Option<f64>,
    /// Strength.
    pub strength: f64,
    /// Tier.
    pub tier: u8,
    /// Horizon in bars.
    pub horizon: u32,
}

/// Detector id, condition subject and condition key of a kind.
#[must_use]
pub fn template(kind: MeaningKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        MeaningKind::Continuation => ("continuation.ma_stack", "close", "slow"),
        MeaningKind::Reversal => ("reversal.rsi_divergence", "close", "price"),
        MeaningKind::Breakout => ("breakout.range", "close", "level"),
        MeaningKind::Squeeze => ("squeeze.bandwidth", "bandwidth", "threshold"),
        MeaningKind::Extreme => ("extreme.rsi", "rsi", "level"),
        MeaningKind::Flow => ("flow.institutional", "net", "avg5"),
    }
}

fn family(kind: MeaningKind) -> EvidenceFamily {
    if kind == MeaningKind::Flow {
        EvidenceFamily::Flow
    } else {
        EvidenceFamily::Price
    }
}

/// +1 when "holds" means the subject is above the key, −1 when below.
fn sense(kind: MeaningKind, d: Direction) -> f64 {
    if kind == MeaningKind::Squeeze || d == Direction::Down {
        -1.0
    } else {
        1.0
    }
}

fn evidence(kind: MeaningKind, d: Direction, r: &mut Rng) -> BTreeMap<String, f64> {
    let p = round(r.range_f64(20.0, 300.0), 2);
    let kv = |v: &[(&str, f64)]| -> BTreeMap<String, f64> {
        v.iter().map(|(k, x)| ((*k).to_owned(), *x)).collect()
    };
    let s = sense(kind, d);
    match kind {
        MeaningKind::Continuation => kv(&[
            ("close", p),
            ("fast", round(p * (1.0 - s * 0.01), 2)),
            ("slow", round(p * (1.0 - s * 0.04), 2)),
        ]),
        MeaningKind::Reversal => kv(&[
            ("price", p),
            ("rsi", round(r.range_f64(35.0, 65.0), 1)),
            ("prev_rsi", round(r.range_f64(25.0, 75.0), 1)),
        ]),
        MeaningKind::Breakout => kv(&[
            ("level", p),
            ("close", round(p * (1.0 + s * r.range_f64(0.005, 0.03)), 2)),
            ("atr", round(p * 0.021, 2)),
        ]),
        MeaningKind::Squeeze => kv(&[
            ("bandwidth", round(r.range_f64(0.02, 0.05), 3)),
            ("threshold", 0.06),
        ]),
        MeaningKind::Extreme => {
            let level = if d == Direction::Down { 30.0 } else { 70.0 };
            kv(&[
                ("level", level),
                ("prev", round(level - s * r.range_f64(0.5, 4.0), 1)),
                ("value", round(level + s * r.range_f64(0.5, 4.0), 1)),
            ])
        }
        MeaningKind::Flow => {
            let avg = (r.range_f64(200.0, 900.0).round()) * 1000.0;
            kv(&[
                ("avg5", avg),
                (
                    "net",
                    s * (avg * r.range_f64(2.1, 4.0) / 1000.0).round() * 1000.0,
                ),
            ])
        }
    }
}

fn fact(
    instrument: &str,
    detector: &str,
    code: &str,
    bar: u64,
    m: FactMeaning,
    evidence: BTreeMap<String, f64>,
) -> Fact {
    let series = format!("{instrument}.1d");
    let t = T0 + i64::try_from(bar).unwrap_or(0) * DAY;
    Fact {
        id: format!("fx.{instrument}.{detector}.{code}.{bar}"),
        kind: format!("{detector}.{code}"),
        series: series.clone(),
        bar,
        t,
        text: format!("{detector} {code} at bar {bar}"),
        evidence,
        evidence_ref: None,
        meaning: Some(m),
        provenance: Provenance {
            author: Author::Engine {
                name: "qp-facts".to_owned(),
                version: "r3-harness".to_owned(),
            },
            at: t,
            based_on: vec![series],
            approved_by: None,
        },
    }
}

/// Builds the situation of a draft.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build(d: &Draft) -> Situation {
    let (detector, subject, key) = template(d.kind);
    let lead_bar = d.now_bar - d.bars_ago;
    let license = if d.redacted {
        License {
            class: LicenseClass::Licensed,
            entitlement: Some(format!("flow/{}", d.instrument)),
        }
    } else {
        License {
            class: LicenseClass::Public,
            entitlement: None,
        }
    };
    let s = sense(d.kind, d.direction);
    let op = if s > 0.0 {
        CompareOp::Gt
    } else {
        CompareOp::Lt
    };
    let condition = match d.time {
        TimeKind::Event => None,
        TimeKind::State => Some(MeaningCondition {
            subject: subject.to_owned(),
            op,
            key: key.to_owned(),
            within_bars: None,
        }),
        TimeKind::Expectation => Some(MeaningCondition {
            subject: subject.to_owned(),
            op,
            key: key.to_owned(),
            within_bars: Some(3),
        }),
    };
    let meaning = |dir: Direction, edge: Edge, fam: EvidenceFamily, det: &str, time| FactMeaning {
        kind: d.kind,
        direction: dir,
        family: fam,
        time,
        edge,
        detector: det.to_owned(),
        detector_version: 1,
        instrument: d.instrument.clone(),
        strength: d.strength,
        license: license.clone(),
        geometry: Vec::new(),
        condition: condition.clone(),
        server: d.redacted,
    };
    let lead_edge = if d.time == TimeKind::State {
        Edge::Enter
    } else {
        Edge::Point
    };
    let lead = fact(
        &d.instrument,
        detector,
        dir_name(d.direction),
        lead_bar,
        meaning(d.direction, lead_edge, family(d.kind), detector, d.time),
        d.evidence.clone(),
    );
    let ended_state = d.time == TimeKind::State && d.still_holds == Some(false);
    let (latest, reason) = match d.stage {
        Stage::Confirmed => {
            let mut ev = BTreeMap::new();
            let avg = 1_000_000.0 + 1000.0 * (lead_bar % 500) as f64;
            ev.insert("avg20".to_owned(), avg);
            ev.insert("volume".to_owned(), avg * 2.5);
            let m = meaning(
                d.direction,
                Edge::Point,
                EvidenceFamily::Volume,
                "flow.volume_surge",
                TimeKind::Event,
            );
            (
                fact(
                    &d.instrument,
                    "flow.volume_surge",
                    "surge",
                    d.now_bar,
                    m,
                    ev,
                ),
                "new_family",
            )
        }
        Stage::Weakened if ended_state => {
            let mut ev = d.evidence.clone();
            if let Some(c) = d.current {
                ev.insert("value".to_owned(), c);
            }
            let m = meaning(d.direction, Edge::Exit, family(d.kind), detector, d.time);
            (
                fact(&d.instrument, detector, "exit", d.now_bar, m, ev),
                "state_ended",
            )
        }
        Stage::Weakened => {
            let opp = match d.direction {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
                Direction::Neutral => Direction::Neutral,
            };
            let m = meaning(opp, Edge::Point, family(d.kind), detector, TimeKind::Event);
            (
                fact(
                    &d.instrument,
                    detector,
                    &format!("opp_{}", dir_name(opp)),
                    d.now_bar,
                    m,
                    d.evidence.clone(),
                ),
                "opposite_fact",
            )
        }
        Stage::Realized => (lead.clone(), "outcome_hit"),
        Stage::Invalidated => (lead.clone(), "outcome_miss"),
        Stage::Expired => (lead.clone(), "horizon"),
        _ => (lead.clone(), "new_story"),
    };
    let mut families = vec![family(d.kind)];
    if d.stage == Stage::Confirmed {
        families.push(EvidenceFamily::Volume);
    }
    families.sort();
    families.dedup();
    let t_of = |bar: u64| T0 + i64::try_from(bar).unwrap_or(0) * DAY;
    let mut history = vec![Transition {
        stage: Stage::Born,
        bar: lead_bar,
        t: t_of(lead_bar),
        facts: vec![lead.id.clone()],
        reason: "new_story".to_owned(),
    }];
    if d.stage != Stage::Born {
        history.push(Transition {
            stage: d.stage,
            bar: d.now_bar,
            t: t_of(d.now_bar),
            facts: if latest.id == lead.id {
                Vec::new()
            } else {
                vec![latest.id.clone()]
            },
            reason: reason.to_owned(),
        });
    }
    let story = Story {
        instrument: d.instrument.clone(),
        theme: d.kind.name().to_owned(),
        kind: d.kind,
        direction: d.direction,
        time: d.time,
        stage: d.stage,
        families,
        strength: d.strength,
        importance: 1.0,
        tier: d.tier,
        series: format!("{}.1d", d.instrument),
        born_bar: lead_bar,
        last_bar: if d.stage == Stage::Born {
            lead_bar
        } else {
            d.now_bar
        },
        horizon_bars: d.horizon,
        ledger_key: format!("{}/{detector}@1", d.kind.name()),
        ledger_at_birth: d.ledger,
        trust: if d.redacted {
            Trust::Server
        } else {
            Trust::Replica
        },
        history,
    };
    let closed_no = matches!(d.stage, Stage::Invalidated | Stage::Expired);
    let (when, bars_ago) = match d.time {
        TimeKind::Event => ("past", Some(d.bars_ago)),
        TimeKind::State if d.still_holds == Some(true) && !closed_no => ("now", None),
        TimeKind::State => ("past", Some(d.bars_ago)),
        TimeKind::Expectation => ("conditional", None),
    };
    let still_valid = if closed_no {
        "no"
    } else {
        match d.still_holds {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        }
    };
    Situation {
        id: d.id.clone(),
        lead,
        latest,
        now_bar: d.now_bar,
        now_t: t_of(d.now_bar),
        still_holds: d.still_holds,
        current: d.current,
        redacted: d.redacted,
        truth: Truth {
            kind: d.kind.name().to_owned(),
            direction: dir_name(d.direction).to_owned(),
            when: when.to_owned(),
            bars_ago,
            still_valid: still_valid.to_owned(),
        },
        factors: Factors {
            time: time_name(d.time).to_owned(),
            kind: d.kind.name().to_owned(),
            stage: stage_name(d.stage).to_owned(),
            bars_ago: d.bars_ago,
            still_holds: match d.still_holds {
                Some(true) => "yes",
                Some(false) => "no",
                None => "na",
            }
            .to_owned(),
            redacted: d.redacted,
            ledger: if d.ledger.n >= 100 { "large" } else { "small" }.to_owned(),
        },
        story,
    }
}

/// The K26 situation: RSI crossed above 70 three bars ago; now 55, the state no longer
/// holds (`still_holds = false`, `current = 55`).
#[must_use]
pub fn k26() -> Situation {
    let evidence: BTreeMap<String, f64> = [("level", 70.0), ("prev", 68.9), ("value", 71.4)]
        .iter()
        .map(|(k, v)| ((*k).to_owned(), *v))
        .collect();
    build(&Draft {
        id: "k26".to_owned(),
        instrument: "THYAO".to_owned(),
        kind: MeaningKind::Extreme,
        direction: Direction::Up,
        time: TimeKind::State,
        stage: Stage::Weakened,
        now_bar: 480,
        bars_ago: 3,
        still_holds: Some(false),
        redacted: false,
        ledger: LedgerStat {
            n: 140,
            realized: 87,
        },
        evidence,
        current: Some(55.0),
        strength: 0.64,
        tier: 1,
        horizon: 20,
    })
}

/// The 72 seeded situations, then K26 (73 in all), in id order.
#[must_use]
pub fn all() -> Vec<Situation> {
    let mut r = Rng::derive(SET_SEED, "m2.situations");
    let mut stages: Vec<Stage> = (0..72).map(|i| STAGES[i % 6]).collect();
    r.shuffle(&mut stages);
    let mut redacted: Vec<bool> = (0..72).map(|i| i % 4 == 0).collect();
    r.shuffle(&mut redacted);
    let mut large: Vec<bool> = (0..72).map(|i| i % 2 == 0).collect();
    r.shuffle(&mut large);
    let mut out = Vec::with_capacity(73);
    for i in 0..72 {
        let time = TIMES[i / 24];
        let kind = MeaningKind::ALL[(i / 4) % 6];
        let bars_ago = BARS_AGO[i % 4];
        let stage = stages[i];
        let direction = if kind == MeaningKind::Squeeze {
            Direction::Neutral
        } else if r.below(2) == 0 {
            Direction::Up
        } else {
            Direction::Down
        };
        let closed_no = matches!(stage, Stage::Invalidated | Stage::Expired);
        let coin = r.below(3);
        let still_holds = match time {
            TimeKind::Event => None,
            TimeKind::State if closed_no => Some(false),
            TimeKind::State if bars_ago == 0 => Some(true),
            TimeKind::State => Some(coin != 0),
            TimeKind::Expectation if closed_no => Some(false),
            TimeKind::Expectation if stage == Stage::Realized => None,
            TimeKind::Expectation => (coin != 0).then_some(true),
        };
        let ev = evidence(kind, direction, &mut r);
        let (_, _, key) = template(kind);
        let s = sense(kind, direction);
        let current = (time == TimeKind::State && !redacted[i]).then(|| {
            let kv = ev.get(key).copied().unwrap_or(0.0);
            let f = if still_holds == Some(true) {
                1.0 + s * 0.02
            } else {
                1.0 - s * 0.03
            };
            let dec = if kind == MeaningKind::Squeeze { 4 } else { 2 };
            round(kv * f, dec)
        });
        let ledger = if large[i] {
            LedgerStat {
                n: 140,
                realized: 40 + u32::try_from(r.below(61)).unwrap_or(0),
            }
        } else {
            LedgerStat {
                n: 8,
                realized: 2 + u32::try_from(r.below(5)).unwrap_or(0),
            }
        };
        let now_bar = 300 + r.below(400) as u64;
        let horizon = if stage == Stage::Expired {
            u32::try_from(bars_ago.max(1)).unwrap_or(1)
        } else {
            u32::try_from(bars_ago).unwrap_or(0) + 10 + u32::try_from(r.below(20)).unwrap_or(0)
        };
        out.push(build(&Draft {
            id: format!("s{:02}", i + 1),
            instrument: INSTRUMENTS[r.below(INSTRUMENTS.len())].to_owned(),
            kind,
            direction,
            time,
            stage,
            now_bar,
            bars_ago,
            still_holds,
            redacted: redacted[i],
            ledger,
            evidence: ev,
            current,
            strength: round(r.range_f64(0.3, 0.95), 2),
            tier: 1 + u8::try_from(r.below(3)).unwrap_or(0),
            horizon,
        }));
    }
    out.push(k26());
    out
}

/// A serializable view of one situation's realizer input (no truth), for alternatives that
/// generate sentences offline (`harness m2 inputs`).
#[derive(Debug, Clone, Serialize)]
pub struct InputView<'a> {
    /// Situation id.
    pub id: &'a str,
    /// Story.
    pub story: &'a Story,
    /// Lead fact.
    pub lead: &'a Fact,
    /// Latest fact.
    pub latest: &'a Fact,
    /// Now bar.
    pub now_bar: u64,
    /// Now time.
    pub now_t: i64,
    /// Still holds.
    pub still_holds: Option<bool>,
    /// Current value.
    pub current: Option<f64>,
    /// Redacted.
    pub redacted: bool,
}

impl<'a> From<&'a Situation> for InputView<'a> {
    fn from(s: &'a Situation) -> Self {
        Self {
            id: &s.id,
            story: &s.story,
            lead: &s.lead,
            latest: &s.latest,
            now_bar: s.now_bar,
            now_t: s.now_t,
            still_holds: s.still_holds,
            current: s.current,
            redacted: s.redacted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_shape() {
        let v = all();
        assert_eq!(v.len(), 73);
        assert_eq!(v, all());
        let set = &v[..72];
        for st in STAGES {
            assert_eq!(set.iter().filter(|s| s.story.stage == st).count(), 12);
        }
        assert_eq!(set.iter().filter(|s| s.redacted).count(), 18);
        for b in BARS_AGO {
            assert_eq!(set.iter().filter(|s| s.bars_ago() == b).count(), 18);
        }
        let mut ids: Vec<&str> = v.iter().map(|s| s.id.as_str()).collect();
        ids.dedup();
        assert_eq!(ids.len(), 73);
        for s in &v {
            assert!(s.story.families.windows(2).all(|w| w[0] < w[1]));
            assert_eq!(s.story.history[0].stage, Stage::Born);
            if s.redacted {
                assert!(s.current.is_none());
                assert_eq!(s.story.trust, Trust::Server);
            }
            assert!(s.lead.id.matches('.').count() >= 4);
        }
    }

    #[test]
    fn k26_truth() {
        let k = k26();
        assert_eq!(k.bars_ago(), 3);
        assert_eq!(k.still_holds, Some(false));
        assert_eq!(k.current, Some(55.0));
        assert_eq!(k.truth.when, "past");
        assert_eq!(k.truth.bars_ago, Some(3));
        assert_eq!(k.truth.still_valid, "no");
        assert_eq!(k.truth.kind, "extreme");
    }
}
