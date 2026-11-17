//! Situations: the engine data of one capsule at one "now" for one reader — everything a
//! language may use (the product's `SentenceInput` plus the headline pieces' inputs).
//!
//! Situations come from the chosen engine (`EngineConfig::r3()`, the seven default
//! detectors) over the synthetic market (daily world `syn-d`: 30 instruments × 500 days,
//! seed 1; hourly world `syn-h`: 10 instruments × 160 days, seed 2; both with the
//! synthetic licensed flow), never from real data. For every story event the "now" is
//! the transition's bar; for an open story also the first later bar (before its next
//! event) where its state condition no longer holds (validity `no_longer`). A licensed
//! fact gives an entitled and a redacted (K33) situation. Stories are never retracted
//! in a replay (§20.7), so one **constructed** retracted situation per detector and
//! direction is added (a closed situation with the stage set to `retracted`; marked
//! `origin = constructed`).
//!
//! One situation is kept per cell `(world, detector, direction, stage, validity,
//! redacted, ledger bucket)`: the first in event order. Deterministic.

use std::collections::{BTreeMap, BTreeSet};

use qp_capsule::engine::EngineConfig;
use qp_capsule::pipeline::simulate_engine;
use qp_capsule::realize::elapsed::Period;
use qp_capsule::sentence::{closed_override, condition_now, sentence_lead, validity_now, Reader};
use qp_capsule::{InstrumentInput, SentenceInput};
use qp_facts::{default_detectors, FlowPoint};
use qp_schema::capsule::{Direction, Stage, Story, Transition, Validity};
use qp_schema::{Bar, Fact};
use serde::{Deserialize, Serialize};

/// Ledger sample size below which the product says "az örnek" (`LEDGER_MIN_N`).
pub const THIN_N: u32 = qp_capsule::realize::LEDGER_MIN_N;

/// One capsule at one "now" for one reader.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Situation {
    /// `s001`… in generation order.
    pub id: String,
    /// World name (`syn-d`, `syn-h`).
    pub world: String,
    /// `engine` or `constructed` (retracted).
    pub origin: String,
    /// Capsule id.
    pub capsule: String,
    /// Detector of the lead fact.
    pub detector: String,
    /// Code of the lead fact (`up`, `overbought`, …).
    pub code: String,
    /// Bar period string (`1day`, `60m`).
    pub period: String,
    /// Calendar name (`synthetic`).
    pub calendar: String,
    /// The story as of now.
    pub story: Story,
    /// The lead fact as the realizer reads it (`sentence_lead` applied).
    pub lead: Fact,
    /// The lead fact as it is (conditions intact; the answer key reads it).
    pub lead_raw: Fact,
    /// Latest fact that joined at or before now (may be the lead).
    pub latest: Fact,
    /// Facts joined at or before now.
    pub facts: Vec<Fact>,
    /// Now (bar index).
    pub now_bar: u64,
    /// Now (bar open time, ms).
    pub now_t: i64,
    /// Trading-day rank of every history transition's bar and of now, by bar index
    /// (`(bar, day rank)`); the key's day offsets come from here.
    pub day_rank: Vec<(u64, i64)>,
    /// Whether the state condition holds now.
    pub still_holds: Option<bool>,
    /// Current value of the condition subject (`None` when redacted).
    pub current: Option<f64>,
    /// No digits for this reader (K33).
    pub redacted: bool,
    /// Validity now (K42/A28).
    pub validity: Validity,
}

impl Situation {
    /// The product's sentence input.
    #[must_use]
    pub fn input(&self) -> SentenceInput<'_> {
        SentenceInput {
            story: &self.story,
            lead: &self.lead,
            latest: if self.latest.id == self.lead.id {
                &self.lead
            } else {
                &self.latest
            },
            now_bar: self.now_bar,
            now_t: self.now_t,
            still_holds: self.still_holds,
            current: self.current,
            redacted: self.redacted,
        }
    }

    /// Bar period.
    #[must_use]
    pub fn period(&self) -> Period {
        Period::parse(&self.period).unwrap_or(Period::Daily)
    }

    /// The exchange calendar of the elapsed phrases.
    ///
    /// # Panics
    /// Never for the built-in calendars.
    #[must_use]
    pub fn exchange_calendar(&self) -> qp_calendar::ExchangeCalendar {
        let cal = match self.calendar.as_str() {
            "synthetic" => qp_source::synthetic_calendar(),
            other => qp_calendar::builtin(other).expect("built-in calendar"),
        };
        qp_calendar::ExchangeCalendar::new(&cal).expect("valid calendar")
    }

    /// Trading-day rank of bar `bar` (if recorded).
    #[must_use]
    pub fn rank_of(&self, bar: u64) -> Option<i64> {
        self.day_rank
            .iter()
            .find(|(b, _)| *b == bar)
            .map(|(_, r)| *r)
    }

    /// Ledger bucket name: `empty`, `thin`, `counts`.
    #[must_use]
    pub fn ledger_bucket(&self) -> &'static str {
        let n = self.story.ledger_at_birth.n;
        if n == 0 {
            "empty"
        } else if n < THIN_N {
            "thin"
        } else {
            "counts"
        }
    }
}

/// Stage name.
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

/// Validity name.
#[must_use]
pub fn validity_name(v: Validity) -> &'static str {
    match v {
        Validity::Valid => "valid",
        Validity::NoLonger => "no_longer",
        Validity::Closed => "closed",
    }
}

/// Direction name.
#[must_use]
pub fn direction_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn flow_seed(seed: u64, i: usize) -> u64 {
    mix(seed
        .wrapping_mul(1_000_003)
        .wrapping_add(u64::try_from(i).unwrap_or(0).wrapping_mul(7919))
        .wrapping_add(0xF1))
}

fn with_flow(
    seed: u64,
    insts: Vec<qp_source::SyntheticInstrument>,
    suffix: &str,
) -> Vec<InstrumentInput> {
    insts
        .into_iter()
        .enumerate()
        .map(|(i, inst)| {
            let flow = qp_source::synthetic_flow(&inst.bars, flow_seed(seed, i))
                .into_iter()
                .map(|r| FlowPoint {
                    t: r.t,
                    values: r.values,
                })
                .collect();
            InstrumentInput {
                series: format!("{}.{suffix}", inst.symbol),
                symbol: inst.symbol,
                bars: inst.bars,
                flow: Some(flow),
            }
        })
        .collect()
}

/// A world: engine inputs plus their period and calendar.
pub struct World {
    /// Name.
    pub name: &'static str,
    /// Period string.
    pub period: &'static str,
    /// Inputs.
    pub inputs: Vec<InstrumentInput>,
}

/// The two synthetic worlds (see the module).
#[must_use]
pub fn worlds() -> Vec<World> {
    vec![
        World {
            name: "syn-d",
            period: "1day",
            inputs: with_flow(1, qp_source::synthetic_market(30, 500, 1), "1day"),
        },
        World {
            name: "syn-h",
            period: "60m",
            inputs: with_flow(2, qp_source::synthetic_market_hourly(10, 160, 2), "60m"),
        },
    ]
}

/// Istanbul local day number of a bar time.
#[must_use]
pub fn local_day(t: i64) -> i64 {
    (t + 3 * 3_600_000).div_euclid(86_400_000)
}

/// Day rank of every bar of a series (distinct local days in order).
fn ranks(bars: &[Bar]) -> Vec<i64> {
    let mut out = Vec::with_capacity(bars.len());
    let mut last = None;
    let mut r = -1i64;
    for b in bars {
        let d = local_day(b.t);
        if last != Some(d) {
            r += 1;
            last = Some(d);
        }
        out.push(r);
    }
    out
}

struct Cand<'a> {
    capsule: &'a str,
    now: u64,
}

/// Builds a situation (`None` when the capsule was not born at `now`).
#[allow(clippy::too_many_arguments)]
fn build(
    world: &World,
    story_facts: &[Fact],
    events: &[qp_capsule::StoryEvent],
    capsule: &str,
    bars: &[Bar],
    rank: &[i64],
    now: u64,
    redacted_reader: bool,
) -> Option<Situation> {
    let now_i = usize::try_from(now).ok()?;
    let bars_now = bars.get(..=now_i)?;
    let story = events
        .iter()
        .rfind(|e| e.capsule == capsule && e.transition.bar <= now)
        .map(|e| e.story.clone())?;
    let facts: Vec<Fact> = story_facts
        .iter()
        .filter(|f| f.bar <= now)
        .cloned()
        .collect();
    let lead = facts.first()?.clone();
    let latest = facts.last()?.clone();
    let (sh, cur) = condition_now(&lead, &facts, bars_now);
    let still_holds = closed_override(&story, sh);
    let validity = validity_now(&story, still_holds);
    let m = lead.meaning.as_ref()?;
    let reader = if redacted_reader {
        Reader::unentitled()
    } else {
        Reader::entitled()
    };
    let redacted = !reader.may_see(&m.license, &m.instrument);
    if redacted_reader && !redacted {
        return None;
    }
    let lead_view = sentence_lead(&lead).unwrap_or_else(|| lead.clone());
    let latest_view = if latest.id == lead.id {
        lead_view.clone()
    } else {
        latest
    };
    let code = lead.kind.rsplit('.').next().unwrap_or("").to_owned();
    let mut day_rank: Vec<(u64, i64)> = story
        .history
        .iter()
        .map(|h| h.bar)
        .chain([now, lead.bar, latest_view.bar])
        .collect::<BTreeSet<u64>>()
        .into_iter()
        .filter_map(|b| rank.get(usize::try_from(b).ok()?).map(|r| (b, *r)))
        .collect();
    day_rank.sort_unstable();
    Some(Situation {
        id: String::new(),
        world: world.name.to_owned(),
        origin: "engine".to_owned(),
        capsule: capsule.to_owned(),
        detector: m.detector.clone(),
        code,
        period: world.period.to_owned(),
        calendar: "synthetic".to_owned(),
        story,
        lead: lead_view,
        lead_raw: lead,
        latest: latest_view,
        facts,
        now_bar: now,
        now_t: bars[now_i].t,
        day_rank,
        still_holds,
        current: if redacted { None } else { cur },
        redacted,
        validity,
    })
}

/// Cell key of a situation (see the module).
#[must_use]
pub fn cell(s: &Situation) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        s.world,
        s.detector,
        direction_name(s.story.direction),
        stage_name(s.story.stage),
        validity_name(s.validity),
        if s.redacted { "redacted" } else { "entitled" },
        s.ledger_bucket()
    )
}

/// Every situation of every world (see the module), ids assigned in order.
#[must_use]
pub fn generate() -> Vec<Situation> {
    let mut out: Vec<Situation> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut retract_seen: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut constructed: Vec<Situation> = Vec::new();
    for world in worlds() {
        let (engine, _) = simulate_engine(&world.inputs, &default_detectors, EngineConfig::r3());
        let by_series: BTreeMap<&str, (&[Bar], Vec<i64>)> = world
            .inputs
            .iter()
            .map(|i| (i.series.as_str(), (i.bars.as_slice(), ranks(&i.bars))))
            .collect();
        let facts_of: BTreeMap<&str, &[Fact]> = engine
            .open_stories()
            .iter()
            .chain(engine.closed_stories())
            .map(|s| (s.id.as_str(), s.facts.as_slice()))
            .collect();
        let events = engine.events();
        // Candidate "now"s: every transition bar; for an open story also the first bar
        // before its next event where its state condition no longer holds.
        let mut cands: Vec<Cand<'_>> = Vec::new();
        for (k, e) in events.iter().enumerate() {
            cands.push(Cand {
                capsule: &e.capsule,
                now: e.transition.bar,
            });
            if e.story.stage.is_closed() {
                continue;
            }
            let Some((bars, _)) = by_series.get(e.story.series.as_str()) else {
                continue;
            };
            let next = events[k + 1..]
                .iter()
                .find(|x| x.capsule == e.capsule)
                .map_or(bars.len() as u64, |x| x.transition.bar);
            let Some(facts) = facts_of.get(e.capsule.as_str()) else {
                continue;
            };
            let lead = &facts[0];
            for b in (e.transition.bar + 1)..next.min(e.transition.bar + 60) {
                let bi = usize::try_from(b).unwrap_or(usize::MAX);
                let Some(slice) = bars.get(..=bi) else { break };
                let fs: Vec<Fact> = facts.iter().filter(|f| f.bar <= b).cloned().collect();
                if condition_now(lead, &fs, slice).0 == Some(false) {
                    cands.push(Cand {
                        capsule: &e.capsule,
                        now: b,
                    });
                    break;
                }
            }
        }
        for c in cands {
            let Some(facts) = facts_of.get(c.capsule) else {
                continue;
            };
            let series = facts[0].series.as_str();
            let Some((bars, rank)) = by_series.get(series) else {
                continue;
            };
            for red in [false, true] {
                let Some(s) = build(&world, facts, events, c.capsule, bars, rank, c.now, red)
                else {
                    continue;
                };
                // A constructed retraction from the first closed situation per detector
                // and direction (daily world).
                if world.name == "syn-d" && s.story.stage.is_closed() && !s.redacted {
                    let k = (
                        s.detector.clone(),
                        direction_name(s.story.direction).to_owned(),
                        s.world.clone(),
                    );
                    if retract_seen.insert(k) {
                        let mut r = s.clone();
                        r.origin = "constructed".to_owned();
                        r.story.stage = Stage::Retracted;
                        if let Some(last) = r.story.history.last_mut() {
                            *last = Transition {
                                stage: Stage::Retracted,
                                bar: last.bar,
                                t: last.t,
                                facts: Vec::new(),
                                reason: "data_corrected".to_owned(),
                            };
                        }
                        constructed.push(r);
                    }
                }
                if seen.insert(cell(&s)) {
                    out.push(s);
                }
            }
        }
    }
    for r in constructed {
        if seen.insert(cell(&r)) {
            out.push(r);
        }
    }
    for (i, s) in out.iter_mut().enumerate() {
        s.id = format!("s{:03}", i + 1);
    }
    out
}
