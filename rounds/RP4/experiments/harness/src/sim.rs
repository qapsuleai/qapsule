//! One symbol through the product path: detectors → facts → `qp_capsule` engine (theme
//! `KindDir`, ranker `Confirm`, outcome = the rule under test), then the per-story
//! measures of SPEC §3 that only need this symbol's bars (`ECR`, `REV`, `SENS`, day
//! position).
//!
//! Symbols run independently (in parallel, results in symbol order). This is the product
//! path for everything M25/M26 read: the theme rule sees only the symbol's own stories and
//! the outcome rule only the story and its bars. The one cross-symbol quantity, the
//! ledger at birth (the key `kind/direction/detector@v` is shared by all symbols), is
//! recomputed afterwards from the merged closes of every symbol ([`Ledger`]), exactly as
//! the engine counts it (closes strictly before the birth time). A test checks this
//! against one engine over all symbols (`qp_capsule::pipeline::simulate`).

use std::collections::BTreeMap;

use qp_calendar::ExchangeCalendar;
use qp_capsule::engine::{Engine, EngineConfig, StoryEvent};
use qp_capsule::pipeline::ENGINE_VERSION;
use qp_capsule::rank::Confirm;
use qp_capsule::theme::KindDir;
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_facts::{to_fact, Detection};
use qp_schema::capsule::{Direction, MeaningKind, Stage};
use qp_schema::{Bar, Fact};

use crate::data::Window;
use crate::registry::{r3_levels, DetectorFactory, Levels, RuleEntry};
use crate::tick::tick;

/// Story still open at the end of the data.
pub const OPEN: u8 = 0;
/// Realized.
pub const REAL: u8 = 1;
/// Invalidated.
pub const INV: u8 = 2;
/// Expired.
pub const EXP: u8 = 3;

/// Day position bit: one of the day's first two bars (opening).
pub const POS_OPEN: u8 = 1;
/// Day position bit: the day's last bar (closing session merged in).
pub const POS_CLOSE: u8 = 2;

/// One story, reduced to what the measures need.
#[derive(Debug, Clone, PartialEq)]
pub struct StoryRec {
    /// Ledger key (`kind/direction/detector@version`).
    pub key: String,
    /// Detector of the fact that gave birth.
    pub detector: String,
    /// Kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Birth bar / time.
    pub born_bar: u64,
    /// Birth time.
    pub born_t: i64,
    /// [`OPEN`], [`REAL`], [`INV`], [`EXP`].
    pub stage: u8,
    /// Close bar (= `born_bar` while open).
    pub close_bar: u64,
    /// Close time (= `born_t` while open).
    pub close_t: i64,
    /// Horizon in bars.
    pub horizon: u32,
    /// [`POS_OPEN`] / [`POS_CLOSE`] bits of the birth bar (intraday).
    pub day_pos: u8,
    /// `ECR_own` (invalidated with levels): a later close in the window reached the own target.
    pub ecr_own: Option<bool>,
    /// `ECR_ref`: same with the `r3` target.
    pub ecr_ref: Option<bool>,
    /// `REV` (realized, directional).
    pub rev: Option<bool>,
    /// `SENS` (closed): stage changed under at least one perturbation.
    pub sens: Option<bool>,
    /// The unperturbed re-run reproduced the engine's stage (self-check).
    pub sens_replay_ok: Option<bool>,
}

impl StoryRec {
    /// Closed (realized / invalidated / expired)?
    #[must_use]
    pub fn closed(&self) -> bool {
        self.stage != OPEN
    }
}

/// Flip up ↔ down (decoy run, R3/M5).
fn flip(d: &mut Detection) {
    d.direction = match d.direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Neutral => Direction::Neutral,
    };
}

/// Facts of every bar (streaming, every bar final), as `qp_capsule::pipeline` makes them.
#[must_use]
pub fn facts_for(
    symbol: &str,
    series: &str,
    bars: &[Bar],
    make: DetectorFactory,
    flipped: bool,
) -> Vec<Vec<Fact>> {
    let mut dets = make();
    let mut out = Vec::with_capacity(bars.len());
    for (i, bar) in bars.iter().enumerate() {
        let mut facts = Vec::new();
        for det in &mut dets {
            for mut d in det.push(bar, None) {
                if flipped {
                    flip(&mut d);
                }
                facts.push(to_fact(
                    det.as_ref(),
                    &d,
                    symbol,
                    series,
                    i as u64,
                    bar.t,
                    ENGINE_VERSION,
                ));
            }
        }
        out.push(facts);
    }
    out
}

/// The engine of the product with `rule` as the outcome rule.
#[must_use]
pub fn engine_with(rule: Box<dyn OutcomeRule>) -> Engine {
    Engine::new(EngineConfig {
        theme: Box::new(KindDir::default()),
        outcome: rule,
        ranker: Box::new(Confirm::default()),
    })
}

/// Run the engine over one symbol.
#[must_use]
pub fn run_engine(
    symbol: &str,
    series: &str,
    bars: &[Bar],
    facts: &[Vec<Fact>],
    rule: Box<dyn OutcomeRule>,
) -> Engine {
    let mut e = engine_with(rule);
    for i in 0..bars.len() {
        e.on_bar(symbol, series, i as u64, &bars[..=i], &facts[i]);
    }
    e
}

/// Day-position bits of every bar (intraday): first two bars of a local day, last bar.
#[must_use]
pub fn day_positions(cal: &ExchangeCalendar, bars: &[Bar]) -> Vec<u8> {
    let days: Vec<i64> = bars.iter().map(|b| cal.local_day(b.t)).collect();
    let mut out = vec![0u8; bars.len()];
    let mut k = 0usize;
    for i in 0..bars.len() {
        if i > 0 && days[i] != days[i - 1] {
            k = 0;
        }
        if k < 2 {
            out[i] |= POS_OPEN;
        }
        if i + 1 == bars.len() || days[i + 1] != days[i] {
            out[i] |= POS_CLOSE;
        }
        k += 1;
    }
    out
}

fn stage_code(s: Stage) -> u8 {
    match s {
        Stage::Realized => REAL,
        Stage::Invalidated => INV,
        Stage::Expired => EXP,
        _ => OPEN,
    }
}

fn sign(d: Direction) -> f64 {
    if d == Direction::Down {
        -1.0
    } else {
        1.0
    }
}

/// Whether close `c` reaches the target of `lv` for a claim of direction `dir`.
#[must_use]
pub fn reaches_target(lv: &Levels, dir: Direction, c: f64) -> bool {
    if dir == Direction::Neutral {
        (c - lv.p0).abs() >= lv.target
    } else {
        sign(dir) * (c - lv.p0) >= lv.target
    }
}

/// The story as the engine showed it to the outcome rule at each bar: snapshots after
/// each non-closing event, valid for `now > bar`.
struct Snapshots {
    snaps: Vec<(u64, OpenStory)>,
}

impl Snapshots {
    fn new(s: &OpenStory, events: &[&StoryEvent]) -> Self {
        let mut snaps: Vec<(u64, OpenStory)> = Vec::new();
        for ev in events {
            if ev.story.stage.is_closed() {
                continue;
            }
            let b = ev.transition.bar;
            let os = OpenStory {
                id: s.id.clone(),
                story: ev.story.clone(),
                facts: s.facts.iter().filter(|f| f.bar <= b).cloned().collect(),
            };
            match snaps.last_mut() {
                Some(last) if last.0 == b => *last = (b, os),
                _ => snaps.push((b, os)),
            }
        }
        Self { snaps }
    }

    fn at(&self, now: u64) -> &OpenStory {
        let k = self.snaps.partition_point(|(b, _)| *b < now);
        &self.snaps[k.max(1) - 1].1
    }
}

/// Re-run the rule for one story from its birth (engine order: judge, then horizon).
/// Returns the stage ([`OPEN`] if the data ends first).
fn rerun(rule: &dyn OutcomeRule, snaps: &Snapshots, bars: &[Bar], born: usize, h: usize) -> u8 {
    let end = born + h.max(1);
    for now in born + 1..=end {
        if now >= bars.len() {
            return OPEN;
        }
        let s = snaps.at(now as u64);
        if let Some((v, _)) = rule.judge(s, &bars[..=now], now as u64) {
            return match v {
                Verdict::Realized => REAL,
                Verdict::Invalidated => INV,
            };
        }
        if now >= born + h {
            return EXP;
        }
    }
    OPEN
}

/// Move bar `j`'s close by `m` price steps (high / low widened to keep the close inside).
fn bump(bars: &mut [Bar], j: usize, delta: f64) {
    let b = &mut bars[j];
    b.c += delta;
    b.h = b.h.max(b.c);
    b.l = b.l.min(b.c);
}

/// Run `entry` over one symbol and reduce every story. Per-story measures (`ECR`,
/// `REV`, `SENS`) are computed only when `measures` is set and only for closed stories
/// born inside `window`.
#[must_use]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn run_symbol(
    symbol: &str,
    series: &str,
    bars: &[Bar],
    facts: &[Vec<Fact>],
    entry: &RuleEntry,
    window: Window,
    day_pos: &[u8],
    measures: bool,
) -> Vec<StoryRec> {
    let engine = run_engine(symbol, series, bars, facts, (entry.make)());
    let mut by_cap: BTreeMap<&str, Vec<&StoryEvent>> = BTreeMap::new();
    for ev in engine.events() {
        by_cap.entry(ev.capsule.as_str()).or_default().push(ev);
    }
    // A separate instance for the re-runs (the engine's instance stays untouched).
    let sens_rule = (entry.make)();
    let mut pb: Vec<Bar> = bars.to_vec();
    let mut out = Vec::new();
    let stories = engine
        .closed_stories()
        .iter()
        .chain(engine.open_stories().iter());
    for s in stories {
        let st = &s.story;
        let stage = stage_code(st.stage);
        let born = usize::try_from(st.born_bar).unwrap_or(0);
        let born_t = bars[born].t;
        let (close_bar, close_t) = if stage == OPEN {
            (st.born_bar, born_t)
        } else {
            (st.last_bar, st.history.last().map_or(born_t, |h| h.t))
        };
        let detector = s
            .facts
            .first()
            .and_then(|f| f.meaning.as_ref())
            .map(|m| m.detector.clone())
            .unwrap_or_default();
        let mut rec = StoryRec {
            key: st.ledger_key.clone(),
            detector,
            kind: st.kind,
            direction: st.direction,
            born_bar: st.born_bar,
            born_t,
            stage,
            close_bar,
            close_t,
            horizon: st.horizon_bars,
            day_pos: day_pos.get(born).copied().unwrap_or(0),
            ecr_own: None,
            ecr_ref: None,
            rev: None,
            sens: None,
            sens_replay_ok: None,
        };
        if measures && stage != OPEN && window.contains(born_t) {
            let h = st.horizon_bars as usize;
            let d = usize::try_from(close_bar).unwrap_or(born);
            let win_end = (born + h).min(bars.len() - 1);
            let own = (entry.levels)(s, &bars[..=born]);
            let refl = r3_levels(s, &bars[..=born]);
            if stage == INV {
                let ecr = |lv: Option<Levels>| {
                    lv.map(|lv| {
                        (d + 1..=win_end).any(|j| reaches_target(&lv, st.direction, bars[j].c))
                    })
                };
                rec.ecr_own = ecr(own);
                rec.ecr_ref = ecr(refl);
            }
            let p0 = bars[born].c;
            if stage == REAL && st.direction != Direction::Neutral {
                let half = h.div_ceil(2);
                let end = (d + half).min(bars.len() - 1);
                rec.rev = Some((d + 1..=end).any(|j| sign(st.direction) * (bars[j].c - p0) < 0.0));
            }
            // SENS.
            let evs = by_cap.get(s.id.as_str()).cloned().unwrap_or_default();
            let snaps = Snapshots::new(s, &evs);
            if !snaps.snaps.is_empty() {
                let base = rerun(sens_rule.as_ref(), &snaps, &pb, born, h);
                rec.sens_replay_ok = Some(base == stage);
                let mut targets = vec![d];
                if stage == EXP {
                    // The bar of the window whose close came nearest to a barrier.
                    let lv = own;
                    let last = (born + h).min(bars.len() - 1);
                    if let Some(lv) = lv {
                        let mut best: Option<(f64, usize)> = None;
                        for (j, bar) in bars.iter().enumerate().take(last + 1).skip(born + 1) {
                            let c = bar.c;
                            let dist = if st.direction == Direction::Neutral {
                                (lv.target - (c - lv.p0).abs()).abs()
                            } else {
                                let x = sign(st.direction) * (c - lv.p0);
                                let dt = (lv.target - x).abs();
                                lv.stop.map_or(dt, |sp| dt.min((x + sp).abs()))
                            };
                            if best.is_none_or(|(bd, _)| dist < bd) {
                                best = Some((dist, j));
                            }
                        }
                        if let Some((_, j)) = best {
                            if j != d {
                                targets.push(j);
                            }
                        }
                    }
                }
                let step = tick(p0);
                let mut changed = false;
                'outer: for &j in &targets {
                    for m in [1.0, -1.0, 2.0, -2.0] {
                        let saved = pb[j].clone();
                        bump(&mut pb, j, m * step);
                        let r = rerun(sens_rule.as_ref(), &snaps, &pb, born, h);
                        pb[j] = saved;
                        if r != base {
                            changed = true;
                            break 'outer;
                        }
                    }
                }
                rec.sens = Some(changed);
            }
        }
        out.push(rec);
    }
    out
}

/// The merged ledger of every symbol: per key, close times ascending with realized flags.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    by_key: BTreeMap<String, (Vec<i64>, Vec<u32>)>,
}

impl Ledger {
    /// Build from every closed story of every symbol.
    #[must_use]
    pub fn build<'a>(stories: impl Iterator<Item = &'a StoryRec>) -> Self {
        let mut rows: BTreeMap<String, Vec<(i64, bool)>> = BTreeMap::new();
        for s in stories.filter(|s| s.closed()) {
            rows.entry(s.key.clone())
                .or_default()
                .push((s.close_t, s.stage == REAL));
        }
        let by_key = rows
            .into_iter()
            .map(|(k, mut v)| {
                v.sort_by_key(|x| (x.0, x.1));
                let t: Vec<i64> = v.iter().map(|x| x.0).collect();
                let mut cum = Vec::with_capacity(v.len() + 1);
                cum.push(0u32);
                for (_, r) in &v {
                    let last = *cum.last().expect("non-empty");
                    cum.push(last + u32::from(*r));
                }
                (k, (t, cum))
            })
            .collect();
        Self { by_key }
    }

    /// `(n, realized)` of `key` over closes strictly before `before_t` (the engine's
    /// `ledger_stat`).
    #[must_use]
    pub fn stat(&self, key: &str, before_t: i64) -> (u32, u32) {
        self.by_key.get(key).map_or((0, 0), |(t, cum)| {
            let k = t.partition_point(|x| *x < before_t);
            (u32::try_from(k).unwrap_or(u32::MAX), cum[k])
        })
    }

    /// Keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.by_key.keys()
    }
}
