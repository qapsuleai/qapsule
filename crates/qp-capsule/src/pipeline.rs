//! Headless simulation: detectors → facts → engine, over closed history.
//!
//! Each instrument gets its own detector set (`make_detectors()`), run in streaming
//! mode (`Detector::push`). In a simulation every pushed bar is final (closed history),
//! so the detections of a push are the facts of that bar. Bars of all instruments are
//! fed to the engine in global time order: by bar `t`, then symbol, then series.

use qp_facts::{to_fact, Detector, FlowPoint};
use qp_schema::capsule::LedgerStat;
use qp_schema::{Bar, Fact, LedgerEntry};

use crate::engine::{Engine, EngineConfig, LedgerRecord, StoryEvent};

/// One instrument's input.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentInput {
    /// Instrument symbol.
    pub symbol: String,
    /// Bar series id.
    pub series: String,
    /// Bars, in time order.
    pub bars: Vec<Bar>,
    /// Licensed flow feed aligned by index with `bars` (server facts), if any.
    pub flow: Option<Vec<FlowPoint>>,
}

/// Result of [`simulate`].
#[derive(Debug, Clone, PartialEq)]
pub struct SimResult {
    /// Every story event, in order.
    pub events: Vec<StoryEvent>,
    /// Ledger rows as of the end (retracted rows left out).
    pub ledger: Vec<LedgerEntry>,
    /// Every ledger row with its point-in-time bookkeeping.
    pub records: Vec<LedgerRecord>,
    /// Every fact, in the order fed to the engine.
    pub facts: Vec<Fact>,
}

impl SimResult {
    /// Ledger statistic of `key` over closes strictly before `before_t`.
    #[must_use]
    pub fn ledger_stat(&self, key: &str, before_t: i64) -> LedgerStat {
        let mut st = LedgerStat::default();
        for r in &self.records {
            if r.entry.key.as_deref() == Some(key) && r.counts(before_t) {
                st.n += 1;
                if r.entry.outcome == qp_schema::Outcome::Confirmed {
                    st.realized += 1;
                }
            }
        }
        st
    }
}

/// Engine version stamped into fact provenance.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the detectors and the engine over closed history.
pub fn simulate(
    inputs: &[InstrumentInput],
    make_detectors: &dyn Fn() -> Vec<Box<dyn Detector>>,
    config: EngineConfig,
) -> SimResult {
    let (engine, facts) = simulate_engine(inputs, make_detectors, config);
    SimResult {
        events: engine.events().to_vec(),
        ledger: engine.ledger(),
        records: engine.ledger_records().to_vec(),
        facts,
    }
}

/// Like [`simulate`], but returns the engine itself (for further queries) and the facts.
pub fn simulate_engine(
    inputs: &[InstrumentInput],
    make_detectors: &dyn Fn() -> Vec<Box<dyn Detector>>,
    config: EngineConfig,
) -> (Engine, Vec<Fact>) {
    let mut engine = Engine::new(config);
    let mut dets: Vec<Vec<Box<dyn Detector>>> = inputs.iter().map(|_| make_detectors()).collect();

    // Global order: (t, symbol, series, input index, bar index).
    let mut order: Vec<(i64, &str, &str, usize, usize)> = Vec::new();
    for (k, inp) in inputs.iter().enumerate() {
        for (i, b) in inp.bars.iter().enumerate() {
            order.push((b.t, inp.symbol.as_str(), inp.series.as_str(), k, i));
        }
    }
    order.sort_unstable();

    let mut all = Vec::new();
    for (t, symbol, series, k, i) in order {
        let inp = &inputs[k];
        let bar = &inp.bars[i];
        let flow = inp.flow.as_ref().and_then(|f| f.get(i));
        let idx = i as u64;
        let mut facts = Vec::new();
        for det in &mut dets[k] {
            for d in det.push(bar, flow) {
                facts.push(to_fact(
                    det.as_ref(),
                    &d,
                    symbol,
                    series,
                    idx,
                    t,
                    ENGINE_VERSION,
                ));
            }
        }
        engine.on_bar(symbol, series, idx, &inp.bars[..=i], &facts);
        all.extend(facts);
    }
    (engine, all)
}
