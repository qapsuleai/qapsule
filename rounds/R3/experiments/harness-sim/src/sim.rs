//! Running the engine with the harness's wrappers.
//!
//! Wrappers only observe or restrict; they never change a strategy's answer:
//! - [`LoggedTheme`] records every theme decision (M4 lost information);
//! - [`GuardedOutcome`] passes `judge` exactly `bars[..=now]` and counts any call that
//!   received more (M5 look-ahead guard (i));
//! - [`FlipDetector`] flips the direction of every detection (M5 decoy).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use qp_capsule::engine::Engine;
use qp_capsule::{
    pipeline::simulate_engine, EngineConfig, InstrumentInput, OpenStory, OutcomeRule, Ranker,
    ThemeDecision, ThemeRule, Verdict,
};
use qp_facts::{Detection, Detector, FlowPoint};
use qp_schema::capsule::{Direction, EvidenceFamily, License, MeaningKind, TimeKind};
use qp_schema::{Bar, Fact};

/// Decision codes: new 0, join 1, ignore 2.
pub type DecisionLog = Rc<RefCell<BTreeMap<String, u8>>>;

/// Theme rule wrapper that records the decision for every fact id.
pub struct LoggedTheme {
    /// Wrapped rule.
    pub inner: Box<dyn ThemeRule>,
    /// Log (fact id → code).
    pub log: DecisionLog,
}

impl ThemeRule for LoggedTheme {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let d = self.inner.decide(fact, open);
        let code = match &d {
            ThemeDecision::New { .. } => 0,
            ThemeDecision::Join(_) => 1,
            ThemeDecision::Ignore => 2,
        };
        self.log.borrow_mut().insert(fact.id.clone(), code);
        d
    }
}

/// Outcome wrapper: slices to `bars[..=now]` itself and counts calls that got more.
pub struct GuardedOutcome {
    /// Wrapped rule.
    pub inner: Box<dyn OutcomeRule>,
    /// Number of `judge` calls whose slice was longer than `now + 1` (or shorter).
    pub violations: Rc<Cell<u64>>,
}

impl OutcomeRule for GuardedOutcome {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn horizon(&self, kind: MeaningKind, time: TimeKind) -> u32 {
        self.inner.horizon(kind, time)
    }
    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let n = usize::try_from(now).ok()?;
        if bars.len() != n + 1 {
            self.violations.set(self.violations.get() + 1);
        }
        if bars.len() <= n {
            return None;
        }
        self.inner.judge(s, &bars[..=n], now)
    }
}

/// Detector wrapper flipping `Up` ↔ `Down` (decoy run).
pub struct FlipDetector(pub Box<dyn Detector>);

fn flip(mut v: Vec<Detection>) -> Vec<Detection> {
    for d in &mut v {
        d.direction = match d.direction {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Neutral => Direction::Neutral,
        };
    }
    v
}

impl Detector for FlipDetector {
    fn id(&self) -> &'static str {
        self.0.id()
    }
    fn version(&self) -> u32 {
        self.0.version()
    }
    fn family(&self) -> EvidenceFamily {
        self.0.family()
    }
    fn license(&self) -> License {
        self.0.license()
    }
    fn warmup(&self) -> usize {
        self.0.warmup()
    }
    fn push(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        flip(self.0.push(bar, flow))
    }
    fn update_last(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        flip(self.0.update_last(bar, flow))
    }
}

/// The default detectors.
#[must_use]
pub fn detectors() -> Vec<Box<dyn Detector>> {
    qp_facts::default_detectors()
}

/// The default detectors, directions flipped.
#[must_use]
pub fn flipped_detectors() -> Vec<Box<dyn Detector>> {
    qp_facts::default_detectors()
        .into_iter()
        .map(|d| Box::new(FlipDetector(d)) as Box<dyn Detector>)
        .collect()
}

/// Run detectors + engine over `inputs`.
pub fn run(
    inputs: &[InstrumentInput],
    theme: Box<dyn ThemeRule>,
    outcome: Box<dyn OutcomeRule>,
    ranker: Box<dyn Ranker>,
) -> (Engine, Vec<Fact>) {
    simulate_engine(
        inputs,
        &detectors,
        EngineConfig {
            theme,
            outcome,
            ranker,
        },
    )
}

/// Every story (closed in close order, then open in birth order) of an engine.
#[must_use]
pub fn all_stories(e: &Engine) -> Vec<&OpenStory> {
    e.closed_stories()
        .iter()
        .chain(e.open_stories().iter())
        .collect()
}
