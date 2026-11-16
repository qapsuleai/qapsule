//! Shared test helpers: hand-made facts, deterministic bars, tiny fake detectors and
//! test rules. Fake detectors stand in for `qp-facts` detectors (written elsewhere).

#![allow(dead_code)]

use std::collections::BTreeMap;

use qp_capsule::engine::EngineConfig;
use qp_capsule::{OpenStory, OutcomeRule, RankCtx, Ranker, Verdict};
use qp_facts::{Detection, Detector, FlowPoint};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, License, LicenseClass, MeaningKind, TimeKind,
};
use qp_schema::{Author, Bar, Fact, Provenance};

pub const SERIES: &str = "1d";

pub fn t_of(i: u64) -> i64 {
    1_000 + i64::try_from(i).unwrap() * 60_000
}

pub fn bar(i: u64, c: f64) -> Bar {
    Bar {
        t: t_of(i),
        o: c,
        h: c,
        l: c,
        c,
        v: 1.0,
    }
}

/// Bars with the given closes.
pub fn bars(closes: &[f64]) -> Vec<Bar> {
    closes
        .iter()
        .enumerate()
        .map(|(i, c)| bar(i as u64, *c))
        .collect()
}

/// A hand-made final fact.
#[allow(clippy::too_many_arguments)]
pub fn fact(
    inst: &str,
    det: &str,
    code: &str,
    kind: MeaningKind,
    dir: Direction,
    fam: EvidenceFamily,
    time: TimeKind,
    edge: Edge,
    bar: u64,
) -> Fact {
    Fact {
        id: format!("fx.{inst}.{det}.{code}.{bar}"),
        kind: format!("{det}.{code}"),
        series: SERIES.to_owned(),
        bar,
        t: t_of(bar),
        text: String::new(),
        evidence: BTreeMap::new(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind,
            direction: dir,
            family: fam,
            time,
            edge,
            detector: det.to_owned(),
            detector_version: 1,
            instrument: inst.to_owned(),
            strength: 0.5,
            license: License {
                class: LicenseClass::Public,
                entitlement: None,
            },
            geometry: Vec::new(),
            condition: None,
            server: false,
        }),
        provenance: Provenance {
            author: Author::Engine {
                name: "test".to_owned(),
                version: "0".to_owned(),
            },
            at: t_of(bar),
            based_on: Vec::new(),
            approved_by: None,
        },
    }
}

/// Point event of kind breakout on the price family, direction up.
pub fn up(inst: &str, det: &str, bar: u64) -> Fact {
    fact(
        inst,
        det,
        "up",
        MeaningKind::Breakout,
        Direction::Up,
        EvidenceFamily::Price,
        TimeKind::Event,
        Edge::Point,
        bar,
    )
}

// ---------------------------------------------------------------------------
// Test rules
// ---------------------------------------------------------------------------

/// Outcome rule for tests: 3% move from the birth close in the story's direction is a
/// hit, 3% against is a miss. Panics if the engine passes anything after `now`.
pub struct MoveRule(pub u32);

impl OutcomeRule for MoveRule {
    fn name(&self) -> &'static str {
        "test.move"
    }
    fn horizon(&self, _k: MeaningKind, _t: TimeKind) -> u32 {
        self.0
    }
    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        assert_eq!(
            bars.len() as u64,
            now + 1,
            "outcome rule saw look-ahead data"
        );
        let entry = bars[usize::try_from(s.story.born_bar).unwrap()].c;
        let cur = bars[usize::try_from(now).unwrap()].c;
        let (hit, miss) = match s.story.direction {
            Direction::Up => (cur >= entry * 1.03, cur <= entry * 0.97),
            Direction::Down => (cur <= entry * 0.97, cur >= entry * 1.03),
            Direction::Neutral => (false, false),
        };
        if hit {
            Some((Verdict::Realized, "outcome_hit"))
        } else if miss {
            Some((Verdict::Invalidated, "outcome_miss"))
        } else {
            None
        }
    }
}

/// Ranker for tests that reads every context field, so a look-ahead in the context
/// would show in the events.
pub struct CtxRank;

impl Ranker for CtxRank {
    fn name(&self) -> &'static str {
        "test.ctx"
    }
    fn rank(&self, s: &OpenStory, c: &RankCtx) -> (f64, u8) {
        let imp = f64::from(c.ledger.n) * 1000.0
            + f64::from(c.ledger.realized) * 100.0
            + f64::from(c.ledger_all.n) * 10.0
            + f64::from(c.same_kind_on_instrument)
            + f64::from(c.same_kind_all) * 0.1
            + f64::from(c.stories_all) * 0.01
            + f64::from(c.instruments) * 0.001
            + (c.bars_seen as f64) * 1e-6
            + s.story.families.len() as f64 * 1e-7;
        (imp, 1 + u8::try_from(c.stories_all % 3).unwrap())
    }
}

pub fn test_config(horizon: u32) -> EngineConfig {
    EngineConfig {
        theme: Box::new(qp_capsule::scaffold::KindTheme),
        outcome: Box::new(MoveRule(horizon)),
        ranker: Box::new(CtxRank),
    }
}

// ---------------------------------------------------------------------------
// Fake detectors (pending-bar model)
// ---------------------------------------------------------------------------

type DetectFn = fn(&[Bar], &Bar) -> Vec<Detection>;

pub struct FakeDet {
    id: &'static str,
    family: EvidenceFamily,
    hist: Vec<Bar>,
    pending: Option<Bar>,
    f: DetectFn,
}

impl Detector for FakeDet {
    fn id(&self) -> &'static str {
        self.id
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        self.family
    }
    fn license(&self) -> License {
        License {
            class: LicenseClass::Public,
            entitlement: None,
        }
    }
    fn warmup(&self) -> usize {
        5
    }
    fn push(&mut self, bar: &Bar, _flow: Option<&FlowPoint>) -> Vec<Detection> {
        if let Some(p) = self.pending.take() {
            self.hist.push(p);
        }
        self.pending = Some(bar.clone());
        (self.f)(&self.hist, bar)
    }
    fn update_last(&mut self, bar: &Bar, _flow: Option<&FlowPoint>) -> Vec<Detection> {
        self.pending = Some(bar.clone());
        (self.f)(&self.hist, bar)
    }
}

fn det(
    code: &'static str,
    kind: MeaningKind,
    dir: Direction,
    time: TimeKind,
    edge: Edge,
) -> Detection {
    Detection {
        code,
        kind,
        direction: dir,
        time,
        edge,
        strength: 0.6,
        evidence: BTreeMap::new(),
        geometry: Vec::new(),
        condition: None,
    }
}

/// Close above the highest of the last 5 closes (up) or below the lowest (down).
fn breakout5(hist: &[Bar], b: &Bar) -> Vec<Detection> {
    if hist.len() < 5 {
        return Vec::new();
    }
    let w = &hist[hist.len() - 5..];
    let hi = w.iter().map(|x| x.c).fold(f64::MIN, f64::max);
    let lo = w.iter().map(|x| x.c).fold(f64::MAX, f64::min);
    let k = MeaningKind::Breakout;
    if b.c > hi {
        vec![det("up", k, Direction::Up, TimeKind::Event, Edge::Point)]
    } else if b.c < lo {
        vec![det(
            "down",
            k,
            Direction::Down,
            TimeKind::Event,
            Edge::Point,
        )]
    } else {
        Vec::new()
    }
}

/// Volume above 1.5x the previous bar's: breakout in the bar's direction (volume family).
fn volspike(hist: &[Bar], b: &Bar) -> Vec<Detection> {
    let Some(p) = hist.last() else {
        return Vec::new();
    };
    if b.v > p.v * 1.5 {
        let d = if b.c >= p.c {
            Direction::Up
        } else {
            Direction::Down
        };
        vec![det(
            "spike",
            MeaningKind::Breakout,
            d,
            TimeKind::Event,
            Edge::Point,
        )]
    } else {
        Vec::new()
    }
}

/// State "close above 103": enter / exit edges.
fn above103(hist: &[Bar], b: &Bar) -> Vec<Detection> {
    let prev = hist.last().is_some_and(|p| p.c > 103.0);
    let cur = b.c > 103.0;
    let k = MeaningKind::Extreme;
    match (prev, cur) {
        (false, true) => vec![det("above", k, Direction::Up, TimeKind::State, Edge::Enter)],
        (true, false) => vec![det("above", k, Direction::Up, TimeKind::State, Edge::Exit)],
        _ => Vec::new(),
    }
}

pub fn make_detectors() -> Vec<Box<dyn Detector>> {
    let mk = |id, family, f| -> Box<dyn Detector> {
        Box::new(FakeDet {
            id,
            family,
            hist: Vec::new(),
            pending: None,
            f,
        })
    };
    vec![
        mk(
            "fake.breakout5",
            EvidenceFamily::Price,
            breakout5 as DetectFn,
        ),
        mk(
            "fake.volspike",
            EvidenceFamily::Volume,
            volspike as DetectFn,
        ),
        mk("fake.above103", EvidenceFamily::Price, above103 as DetectFn),
    ]
}

// ---------------------------------------------------------------------------
// Deterministic bars (fixed-seed LCG; no randomness at run time)
// ---------------------------------------------------------------------------

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next_f(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// A random walk around 100 with `n` bars.
pub fn walk(seed: u64, n: u64) -> Vec<Bar> {
    let mut r = Lcg(seed);
    let mut c = 100.0;
    (0..n)
        .map(|i| {
            c *= 1.0 + (r.next_f() - 0.5) * 0.04;
            let v = 1000.0 * (0.5 + r.next_f() * 1.5);
            Bar {
                t: t_of(i),
                o: c,
                h: c,
                l: c,
                c,
                v,
            }
        })
        .collect()
}
