//! R5/T2 M19 feed harness (`rounds/R5/experiments/harness/feed/SPEC.md`).
//!
//! The chosen engine (`EngineConfig::r3()`, seven default detectors) runs once over a
//! dataset ([`prepare`]); the product code is not changed. Then, for every trading day
//! `d` and every configuration ([`Config`]: hide rule × ordering × K42 exemption), the
//! feed of a synthetic audience is decided **at the close of `d` from what was known
//! then**: story snapshots with a transition on or before `d`, ledger rows recorded
//! strictly before `cutoff[d]` (the last bar time of day `d` + 1; the product's
//! `LedgerRecord::counts` rule), state conditions on bars up to `d`. Nothing after `d`
//! is read by a decision; the outcomes after `d` are read only by the *evaluation*
//! (hit rate of what was kept / hidden).

use std::collections::{BTreeMap, BTreeSet};

use qp_capsule::engine::EngineConfig;
use qp_capsule::pipeline::simulate_engine;
use qp_capsule::sentence::{closed_override, condition_now_memo, validity_now, StudyMemo};
use qp_capsule::{admits, InstrumentInput};
use qp_facts::{default_detectors, FlowPoint};
use qp_schema::capsule::{Audience, MeaningKind, Sensitivity, Stage, Story, UserGroup, Validity};
use qp_schema::{Fact, Outcome};
use serde::Serialize;

/// Detector warm-up: days counted from the first day of bar 50 (as the R4 sensitivity
/// simulation).
pub const WARMUP_BARS: usize = 50;
/// One phone screen of faces (persona tasks' "top K").
pub const TOP_K: usize = 5;

// --------------------------------------------------------------------------------------
// Data
// --------------------------------------------------------------------------------------

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

/// `qp_source::synthetic_market(n, days, seed)` with the synthetic licensed flow (the
/// construction of `qp-capsule/examples/support`).
#[must_use]
pub fn synthetic(seed: u64, n: usize, days: usize) -> Vec<InstrumentInput> {
    qp_source::synthetic_market(n, days, seed)
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
                series: format!("{}.1day", inst.symbol),
                symbol: inst.symbol,
                bars: inst.bars,
                flow: Some(flow),
            }
        })
        .collect()
}

/// The local real daily bars (`$QAPSULE_LOCAL_BARS`, else the first `fixtures/local/bars`
/// walking up from this crate — works from a git worktree under the main tree). `None`
/// when absent. Never committed; only aggregate statistics leave the process.
#[must_use]
pub fn real() -> Option<Vec<InstrumentInput>> {
    let dir = std::env::var_os(qp_source::LOCAL_BARS_ENV)
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| {
            let mut d = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
            loop {
                let c = d.join("fixtures/local/bars");
                if c.is_dir() {
                    return Some(c);
                }
                if !d.pop() {
                    return None;
                }
            }
        })?;
    let all = qp_source::load_local_bars(&dir, "1day");
    if all.is_empty() {
        return None;
    }
    Some(
        all.into_iter()
            .map(|(sym, bars)| InstrumentInput {
                series: format!("{sym}.1day"),
                symbol: sym,
                bars,
                flow: None,
            })
            .collect(),
    )
}

/// The synthetic audience (as `qp-capsule/examples/sensitivity.rs`): ≥ 30 symbols →
/// watch the first 10, positions on the 1st, 4th (short) and 12th (outside the watch
/// list), intent on the 7th; fewer → watch all, positions on the first three, intent on
/// the fourth. Level `balanced` (habituated group, chosen explicitly).
#[derive(Debug, Clone, Serialize)]
pub struct AudienceSpec {
    /// Watch list.
    pub watchlist: Vec<String>,
    /// Positions.
    pub positions: BTreeMap<String, f64>,
    /// Active intents.
    pub intents_on: Vec<String>,
}

impl AudienceSpec {
    /// For a symbol list.
    #[must_use]
    pub fn for_symbols(symbols: &[String]) -> Self {
        let mut s = symbols.to_vec();
        s.sort();
        if s.len() >= 30 {
            Self {
                watchlist: s[..10].to_vec(),
                positions: BTreeMap::from([
                    (s[0].clone(), 100.0),
                    (s[3].clone(), -50.0),
                    (s[11].clone(), 200.0),
                ]),
                intents_on: vec![s[6].clone()],
            }
        } else {
            Self {
                watchlist: s.clone(),
                positions: s
                    .iter()
                    .take(3)
                    .enumerate()
                    .map(|(i, x)| (x.clone(), if i == 1 { -50.0 } else { 100.0 }))
                    .collect(),
                intents_on: s.iter().skip(3).take(1).cloned().collect(),
            }
        }
    }

    /// Watch list ∪ positions ∪ intents.
    #[must_use]
    pub fn universe(&self) -> BTreeSet<String> {
        self.watchlist
            .iter()
            .chain(self.positions.keys())
            .chain(&self.intents_on)
            .cloned()
            .collect()
    }

    /// Instruments the person holds or intends (K42 "elindeki").
    #[must_use]
    pub fn holdings(&self) -> BTreeSet<String> {
        self.positions
            .keys()
            .chain(&self.intents_on)
            .cloned()
            .collect()
    }

    /// The audience value.
    #[must_use]
    pub fn audience(&self) -> Audience {
        Audience {
            group: UserGroup::Habituated,
            sensitivity: Some(Sensitivity::Balanced),
            watchlist: self.watchlist.clone(),
            positions: self.positions.clone(),
        }
    }
}

// --------------------------------------------------------------------------------------
// Prepared engine run
// --------------------------------------------------------------------------------------

/// One story's state at the end of one day.
#[derive(Debug, Clone)]
pub struct DayState {
    /// Snapshot after the day's last transition (or the latest before).
    pub story: Story,
    /// Stages of the transitions on this day (empty: none).
    pub moved: Vec<Stage>,
    /// Validity at the day's close (K42/A28).
    pub validity: Validity,
    /// Time of the last transition so far.
    pub last_t: i64,
}

/// One capsule over the run.
#[derive(Debug, Clone)]
pub struct Cap {
    /// Capsule id.
    pub id: String,
    /// Instrument.
    pub instrument: String,
    /// Kind.
    pub kind: MeaningKind,
    /// Ledger key (directional, K42/A23).
    pub key: String,
    /// Birth day.
    pub born_day: usize,
    /// Close day and how: `Some(true)` played out, `Some(false)` did not, `None`
    /// retracted (no ledger row).
    pub close: Option<(usize, Option<bool>)>,
    /// State per day from the birth day to the close day (or the end): index =
    /// `day − born_day`.
    pub days: Vec<DayState>,
}

impl Cap {
    /// State at the end of day `d` (`None` before birth or after the close day).
    #[must_use]
    pub fn at(&self, d: usize) -> Option<&DayState> {
        if d < self.born_day {
            return None;
        }
        self.days.get(d - self.born_day)
    }
}

/// A ledger row as the decisions see it.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// Close time.
    pub recorded_at: i64,
    /// Played out.
    pub realized: bool,
    /// Retracted after closing at this time.
    pub retracted_at: Option<i64>,
}

/// A dataset after one engine run.
pub struct Prepared {
    /// Name.
    pub name: String,
    /// Trading days.
    pub n_days: usize,
    /// First counted day.
    pub first_day: usize,
    /// Decision time per day: last bar time of the day + 1.
    pub cutoff: Vec<i64>,
    /// Capsules.
    pub caps: Vec<Cap>,
    /// Ledger rows by key, in close order.
    pub ledger: BTreeMap<String, Vec<Row>>,
    /// The audience.
    pub spec: AudienceSpec,
    /// Ledger rows in the engine (all keys), for the karne test.
    pub ledger_rows_total: usize,
    /// Capsule id of every ledger row (karne test).
    pub ledger_capsules: BTreeSet<String>,
}

fn local_day(t: i64) -> i64 {
    (t + 3 * 3_600_000).div_euclid(86_400_000)
}

/// Runs the engine once and builds the per-day story states (see the module).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn prepare(name: &str, inputs: &[InstrumentInput]) -> Prepared {
    let (engine, _facts) = simulate_engine(inputs, &default_detectors, EngineConfig::r3());
    // Day ranks over all instruments (Istanbul local dates).
    let mut days: Vec<i64> = inputs
        .iter()
        .flat_map(|x| x.bars.iter().map(|b| local_day(b.t)))
        .collect();
    days.sort_unstable();
    days.dedup();
    let pos: BTreeMap<i64, usize> = days.iter().enumerate().map(|(i, d)| (*d, i)).collect();
    let n_days = days.len();
    let mut cutoff = vec![i64::MIN; n_days];
    let mut first = usize::MAX;
    let mut day_of: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for x in inputs {
        let v: Vec<usize> = x.bars.iter().map(|b| pos[&local_day(b.t)]).collect();
        for (b, &d) in x.bars.iter().zip(&v) {
            cutoff[d] = cutoff[d].max(b.t + 1);
        }
        first = first.min(v.get(WARMUP_BARS).copied().unwrap_or(usize::MAX));
        day_of.insert(x.series.as_str(), v);
    }
    let first_day = if first == usize::MAX { n_days } else { first };
    let bars_of: BTreeMap<&str, &InstrumentInput> =
        inputs.iter().map(|x| (x.series.as_str(), x)).collect();
    let facts_of: BTreeMap<&str, &[Fact]> = engine
        .open_stories()
        .iter()
        .chain(engine.closed_stories())
        .map(|s| (s.id.as_str(), s.facts.as_slice()))
        .collect();
    // Events per capsule.
    let mut evs: BTreeMap<&str, Vec<&qp_capsule::StoryEvent>> = BTreeMap::new();
    for e in engine.events() {
        evs.entry(e.capsule.as_str()).or_default().push(e);
    }
    let mut caps = Vec::new();
    // One study memo per (series, bar): shared by every story of that series at that bar.
    let mut memos: BTreeMap<(&str, usize), StudyMemo> = BTreeMap::new();
    for (cid, es) in &evs {
        let st0 = &es[0].story;
        let Some(inp) = bars_of.get(st0.series.as_str()) else {
            continue;
        };
        let dv = &day_of[st0.series.as_str()];
        let Some(facts) = facts_of.get(cid) else {
            continue;
        };
        let lead = &facts[0];
        let born_day = dv[usize::try_from(es[0].transition.bar).unwrap_or(0)];
        let last_ev = es.last().map_or(st0, |e| &e.story);
        let close = if last_ev.stage.is_closed() {
            let d = dv[usize::try_from(es.last().map_or(0, |e| e.transition.bar)).unwrap_or(0)];
            let how = match last_ev.stage {
                Stage::Realized => Some(true),
                Stage::Retracted => None,
                _ => Some(false),
            };
            Some((d, how))
        } else {
            None
        };
        let end_day = close.map_or(n_days - 1, |(d, _)| d);
        // Bar index of each day for this series (last bar of the day).
        let mut last_bar_of_day: BTreeMap<usize, usize> = BTreeMap::new();
        for (i, &d) in dv.iter().enumerate() {
            last_bar_of_day.insert(d, i);
        }
        let mut states = Vec::new();
        let mut k = 0usize;
        let mut cur: Option<&qp_capsule::StoryEvent> = None;
        for d in born_day..=end_day {
            let mut moved = Vec::new();
            while k < es.len() && dv[usize::try_from(es[k].transition.bar).unwrap_or(0)] <= d {
                if dv[usize::try_from(es[k].transition.bar).unwrap_or(0)] == d {
                    moved.push(es[k].transition.stage);
                }
                cur = Some(es[k]);
                k += 1;
            }
            let Some(e) = cur else { continue };
            // Validity at the day's close: the series' last bar on or before day d.
            let bar_i = last_bar_of_day
                .range(..=d)
                .next_back()
                .map_or(0, |(_, &i)| i);
            let fs: Vec<&Fact> = facts
                .iter()
                .filter(|f| usize::try_from(f.bar).unwrap_or(0) <= bar_i)
                .collect();
            let validity = if e.story.stage.is_closed() {
                Validity::Closed
            } else {
                let memo = memos.entry((st0.series.as_str(), bar_i)).or_default();
                let (sh, _) = condition_now_memo(lead, &fs, &inp.bars[..=bar_i], memo);
                validity_now(&e.story, closed_override(&e.story, sh))
            };
            states.push(DayState {
                story: e.story.clone(),
                moved,
                validity,
                last_t: e.transition.t,
            });
        }
        caps.push(Cap {
            id: (*cid).to_owned(),
            instrument: st0.instrument.clone(),
            kind: st0.kind,
            key: st0.ledger_key.clone(),
            born_day,
            close,
            days: states,
        });
    }
    let mut ledger: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    let mut ledger_capsules = BTreeSet::new();
    let recs = engine.ledger_records();
    for r in recs {
        if let Some(k) = r.entry.key.as_ref() {
            ledger.entry(k.clone()).or_default().push(Row {
                recorded_at: r.entry.recorded_at,
                realized: r.entry.outcome == Outcome::Confirmed,
                retracted_at: r.retracted_at,
            });
        }
        if let Some(c) = r.entry.capsule.as_ref() {
            ledger_capsules.insert(c.clone());
        }
    }
    for v in ledger.values_mut() {
        v.sort_by_key(|r| r.recorded_at);
    }
    let symbols: Vec<String> = inputs.iter().map(|x| x.symbol.clone()).collect();
    Prepared {
        name: name.to_owned(),
        n_days,
        first_day,
        cutoff,
        caps,
        ledger,
        spec: AudienceSpec::for_symbols(&symbols),
        ledger_rows_total: recs.len(),
        ledger_capsules,
    }
}

impl Prepared {
    /// Ledger `(n, played out)` of `key` as of `before_t` (rows closed strictly before it
    /// and not retracted before it: `LedgerRecord::counts`). Hiding never changes it.
    #[must_use]
    pub fn ledger_at(&self, key: &str, before_t: i64) -> (u32, u32) {
        let mut n = 0u32;
        let mut r = 0u32;
        if let Some(rows) = self.ledger.get(key) {
            for row in rows {
                if row.recorded_at >= before_t {
                    break;
                }
                if row.retracted_at.is_none_or(|x| x >= before_t) {
                    n += 1;
                    if row.realized {
                        r += 1;
                    }
                }
            }
        }
        (n, r)
    }
}

// --------------------------------------------------------------------------------------
// Rules
// --------------------------------------------------------------------------------------

/// A hide rule (SPEC §2). Every rule leaves `n < min_n` alone ("az örnekli": shown and
/// said); it only judges `n ≥ min_n`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "rule")]
pub enum Rule {
    /// Never hide (today's product; the reference).
    None,
    /// Hide when the Wilson 95 % interval of the hit rate contains 0,5.
    Wilson {
        /// Minimum sample.
        min_n: u32,
    },
    /// Hide when `|p − 0,5| < delta`.
    Delta {
        /// Minimum sample.
        min_n: u32,
        /// Half-width.
        delta: f64,
    },
    /// Hide when the Beta(1 + r, 1 + n − r) posterior puts ≥ 0,5 of its mass in
    /// `[0,5 − delta, 0,5 + delta]`.
    Bayes {
        /// Minimum sample.
        min_n: u32,
        /// Half-width.
        delta: f64,
    },
    /// Hide when `p < tau` (research F16: reliability below ≈ 0,7 harms trust).
    Rate {
        /// Minimum sample.
        min_n: u32,
        /// Threshold.
        tau: f64,
    },
}

/// What the feed does with one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Verdict {
    /// Shown, ledger enough (`n ≥ min_n`).
    Shown,
    /// Shown and said to have few cases.
    Thin,
    /// Hidden (yazı tura).
    Hidden,
    /// Shown only because of the K42 exemption.
    Exempt,
}

/// Wilson 95 % interval.
#[must_use]
pub fn wilson(k: f64, n: f64) -> (f64, f64) {
    if n <= 0.0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let p = k / n;
    let d = 1.0 + z * z / n;
    let c = (p + z * z / (2.0 * n)) / d;
    let h = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / d;
    ((c - h).max(0.0), (c + h).min(1.0))
}

fn ln_gamma(x: f64) -> f64 {
    // Lanczos (g = 7, n = 9).
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Beta(a, b) mass on `[lo, hi]` (Simpson, 400 panels).
#[must_use]
pub fn beta_mass(a: f64, b: f64, lo: f64, hi: f64) -> f64 {
    let lb = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
    let pdf = |x: f64| {
        if x <= 0.0 || x >= 1.0 {
            0.0
        } else {
            ((a - 1.0) * x.ln() + (b - 1.0) * (1.0 - x).ln() - lb).exp()
        }
    };
    let n = 400;
    let h = (hi - lo) / n as f64;
    let mut s = pdf(lo) + pdf(hi);
    for i in 1..n {
        s += pdf(lo + i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    s * h / 3.0
}

impl Rule {
    /// Minimum sample (0 for `None`).
    #[must_use]
    pub fn min_n(&self) -> u32 {
        match *self {
            Rule::None => 0,
            Rule::Wilson { min_n }
            | Rule::Delta { min_n, .. }
            | Rule::Bayes { min_n, .. }
            | Rule::Rate { min_n, .. } => min_n,
        }
    }

    /// Number of free parameters (tie-break: fewer first).
    #[must_use]
    pub fn params(&self) -> u32 {
        match self {
            Rule::None => 0,
            Rule::Wilson { .. } => 1,
            _ => 2,
        }
    }

    /// Short label.
    #[must_use]
    pub fn label(&self) -> String {
        match *self {
            Rule::None => "none".into(),
            Rule::Wilson { min_n } => format!("wilson N={min_n}"),
            Rule::Delta { min_n, delta } => format!("delta δ={delta} N={min_n}"),
            Rule::Bayes { min_n, delta } => format!("bayes δ={delta} N={min_n}"),
            Rule::Rate { min_n, tau } => format!("rate τ={tau} N={min_n}"),
        }
    }

    /// The verdict for a ledger `(n, r)` (no exemption).
    #[must_use]
    pub fn judge(&self, n: u32, r: u32) -> Verdict {
        if matches!(self, Rule::None) {
            return if n < 20 {
                Verdict::Thin
            } else {
                Verdict::Shown
            };
        }
        if n < self.min_n() {
            return Verdict::Thin;
        }
        let (nf, rf) = (f64::from(n), f64::from(r));
        let p = rf / nf;
        let hide = match *self {
            Rule::None => false,
            Rule::Wilson { .. } => {
                let (l, h) = wilson(rf, nf);
                l <= 0.5 && 0.5 <= h
            }
            Rule::Delta { delta, .. } => (p - 0.5).abs() < delta,
            Rule::Bayes { delta, .. } => {
                beta_mass(1.0 + rf, 1.0 + nf - rf, 0.5 - delta, 0.5 + delta) >= 0.5
            }
            Rule::Rate { tau, .. } => p < tau,
        };
        if hide {
            Verdict::Hidden
        } else {
            Verdict::Shown
        }
    }
}

/// The rule grid (SPEC §2.1).
#[must_use]
pub fn rule_grid() -> Vec<Rule> {
    let mut v = vec![Rule::None];
    for min_n in [5, 10, 20, 30] {
        v.push(Rule::Wilson { min_n });
        for delta in [0.05, 0.10] {
            v.push(Rule::Delta { min_n, delta });
        }
        v.push(Rule::Bayes { min_n, delta: 0.10 });
        for tau in [0.55, 0.60, 0.70] {
            v.push(Rule::Rate { min_n, tau });
        }
    }
    v
}

/// Orderings (SPEC §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Order {
    /// Live (valid, open) first, then no longer valid, then closed today; newest last
    /// transition first.
    LiveNew,
    /// Ledger trust first (Wilson lower bound of the hit rate; few cases last), then
    /// newest.
    Trust,
    /// One row per instrument (research F15), rows by their newest transition; inside a
    /// row live-new.
    Grouped,
}

/// All orderings.
pub const ORDERS: [Order; 3] = [Order::LiveNew, Order::Trust, Order::Grouped];

/// One configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Config {
    /// Hide rule.
    pub rule: Rule,
    /// Ordering.
    pub order: Order,
    /// K42: a negative transition today on a held instrument is never hidden.
    pub exempt: bool,
}

/// One feed item on one day.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    /// Capsule index.
    pub cap: usize,
    /// Verdict.
    pub verdict: Verdict,
    /// Position among visible items (rows for `Grouped`), 0-based; `None` when hidden.
    pub pos: Option<usize>,
    /// Ledger at decision.
    pub ledger: (u32, u32),
}

/// The feed of day `d` under `cfg` (decided from what was known at `cutoff[d]`).
#[must_use]
pub fn feed_on(p: &Prepared, d: usize, cfg: &Config) -> Vec<Item> {
    let aud = p.spec.audience();
    let uni = p.spec.universe();
    let hold = p.spec.holdings();
    let mut items: Vec<(usize, &DayState, Verdict, (u32, u32))> = Vec::new();
    for (ci, c) in p.caps.iter().enumerate() {
        if !uni.contains(&c.instrument) {
            continue;
        }
        let Some(st) = c.at(d) else { continue };
        let closed_today = st.story.stage.is_closed() && !st.moved.is_empty();
        if st.story.stage.is_closed() && !closed_today {
            continue;
        }
        if !admits(&aud, &st.story, &p.spec.intents_on) {
            continue;
        }
        let l = p.ledger_at(&c.key, p.cutoff[d]);
        let mut v = cfg.rule.judge(l.0, l.1);
        if v == Verdict::Hidden
            && cfg.exempt
            && hold.contains(&c.instrument)
            && st.moved.iter().any(|s| s.is_negative())
        {
            v = Verdict::Exempt;
        }
        items.push((ci, st, v, l));
    }
    let live_rank = |s: &DayState| match s.validity {
        Validity::Valid => 0,
        Validity::NoLonger => 1,
        Validity::Closed => 2,
    };
    let trust = |l: (u32, u32)| {
        if l.0 < 5 {
            -1.0
        } else {
            wilson(f64::from(l.1), f64::from(l.0)).0
        }
    };
    let mut vis: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].2 != Verdict::Hidden)
        .collect();
    let cid = |i: usize| p.caps[items[i].0].id.clone();
    match cfg.order {
        Order::LiveNew | Order::Grouped => vis.sort_by(|&a, &b| {
            live_rank(items[a].1)
                .cmp(&live_rank(items[b].1))
                .then(items[b].1.last_t.cmp(&items[a].1.last_t))
                .then(cid(a).cmp(&cid(b)))
        }),
        Order::Trust => vis.sort_by(|&a, &b| {
            trust(items[b].3)
                .total_cmp(&trust(items[a].3))
                .then(items[b].1.last_t.cmp(&items[a].1.last_t))
                .then(cid(a).cmp(&cid(b)))
        }),
    }
    let mut pos: BTreeMap<usize, usize> = BTreeMap::new();
    if cfg.order == Order::Grouped {
        // Rows by instrument, ordered by their newest transition (then name).
        let mut rows: BTreeMap<&str, i64> = BTreeMap::new();
        for &i in &vis {
            let e = rows
                .entry(p.caps[items[i].0].instrument.as_str())
                .or_insert(i64::MIN);
            *e = (*e).max(items[i].1.last_t);
        }
        let mut order: Vec<(&str, i64)> = rows.into_iter().collect();
        order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let row_of: BTreeMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(r, (s, _))| (*s, r))
            .collect();
        for &i in &vis {
            pos.insert(i, row_of[p.caps[items[i].0].instrument.as_str()]);
        }
    } else {
        for (r, &i) in vis.iter().enumerate() {
            pos.insert(i, r);
        }
    }
    items
        .iter()
        .enumerate()
        .map(|(i, (ci, _, v, l))| Item {
            cap: *ci,
            verdict: *v,
            pos: pos.get(&i).copied(),
            ledger: *l,
        })
        .collect()
}

// --------------------------------------------------------------------------------------
// Metrics
// --------------------------------------------------------------------------------------

/// Measures of one configuration on one dataset (SPEC §3).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Metrics {
    /// Counted days.
    pub days: usize,
    /// Visible items per day, mean.
    pub kept_per_day: f64,
    /// Visible rows per day (instruments for `Grouped`, items otherwise), mean.
    pub rows_per_day: f64,
    /// Hidden items per day, mean.
    pub hidden_per_day: f64,
    /// Share of days with no visible item.
    pub empty_share: f64,
    /// Share of visible items said to have few cases.
    pub thin_share: f64,
    /// Births decided "shown" (n ≥ N) and closed: played out / closed.
    pub kept_hit: (u32, u32),
    /// Births hidden and closed: played out / closed.
    pub hidden_hit: (u32, u32),
    /// Births shown as few cases and closed.
    pub thin_hit: (u32, u32),
    /// Brier skill of the shown-and-decided births: 1 − BS(p̂) / BS(0,5).
    pub kept_skill: f64,
    /// Brier skill of the hidden births.
    pub hidden_skill: f64,
    /// T1 "since I last looked" (yesterday): changed items visible / changed.
    pub t1_visible: (u32, u32),
    /// T1: changed items in the top K / min(K, changed), summed over days.
    pub t1_top: (u32, u32),
    /// T2 "anything negative on what I hold": negative transitions today on holdings,
    /// visible / all.
    pub t2_visible: (u32, u32),
    /// T2: in the top K / all.
    pub t2_top: (u32, u32),
    /// `(p̂, outcome)` of every shown-and-decided birth (bootstrap of the skill).
    #[serde(skip)]
    pub kept_pairs: Vec<(f64, f64)>,
}

/// Brier skill `1 − BS(p̂) / BS(0,5)` of `(p̂, y)` pairs (0 when empty).
#[must_use]
pub fn skill(pairs: &[(f64, f64)]) -> f64 {
    let (a, b) = pairs.iter().fold((0.0, 0.0), |(a, b), (p, y)| {
        (a + (p - y).powi(2), b + (0.5 - y).powi(2))
    });
    if b > 0.0 {
        1.0 - a / b
    } else {
        0.0
    }
}

/// Bootstrap 95 % interval of [`skill`] (resampling births; SplitMix64 seed).
#[must_use]
pub fn skill_ci(pairs: &[(f64, f64)], b: usize, seed: u64) -> (f64, f64) {
    if pairs.is_empty() {
        return (0.0, 0.0);
    }
    let mut z = seed;
    let mut next = || {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(z)
    };
    let n = pairs.len();
    let mut v: Vec<f64> = (0..b)
        .map(|_| {
            let s: Vec<(f64, f64)> = (0..n)
                .map(|_| pairs[usize::try_from(next() % n as u64).unwrap_or(0)])
                .collect();
            skill(&s)
        })
        .collect();
    v.sort_by(f64::total_cmp);
    let q = |p: f64| v[((p * (b - 1) as f64).round() as usize).min(b - 1)];
    (q(0.025), q(0.975))
}

fn rate(x: (u32, u32)) -> f64 {
    if x.1 == 0 {
        0.0
    } else {
        f64::from(x.0) / f64::from(x.1)
    }
}

impl Metrics {
    /// Kept hit rate.
    #[must_use]
    pub fn kept_rate(&self) -> f64 {
        rate(self.kept_hit)
    }
    /// Hidden hit rate.
    #[must_use]
    pub fn hidden_rate(&self) -> f64 {
        rate(self.hidden_hit)
    }
    /// T1 top-K recall.
    #[must_use]
    pub fn t1(&self) -> f64 {
        rate(self.t1_top)
    }
    /// T2 top-K recall.
    #[must_use]
    pub fn t2(&self) -> f64 {
        rate(self.t2_top)
    }
}

/// Runs one configuration over every counted day.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn measure(p: &Prepared, cfg: &Config) -> Metrics {
    let hold = p.spec.holdings();
    let mut m = Metrics::default();
    let (mut kept, mut rows, mut hidden, mut empty, mut thin, mut vis_total) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut bs_k = (0.0, 0.0, 0u32);
    let mut bs_h = (0.0, 0.0, 0u32);
    for d in p.first_day..p.n_days {
        m.days += 1;
        let f = feed_on(p, d, cfg);
        let vis: Vec<&Item> = f.iter().filter(|i| i.verdict != Verdict::Hidden).collect();
        kept += vis.len();
        vis_total += vis.len();
        thin += vis.iter().filter(|i| i.verdict == Verdict::Thin).count();
        hidden += f.len() - vis.len();
        let nrows: BTreeSet<usize> = vis.iter().filter_map(|i| i.pos).collect();
        rows += nrows.len();
        if vis.is_empty() {
            empty += 1;
        }
        let changed: Vec<&Item> = f
            .iter()
            .filter(|i| p.caps[i.cap].at(d).is_some_and(|s| !s.moved.is_empty()))
            .collect();
        let nch = u32::try_from(changed.len()).unwrap_or(0);
        m.t1_visible.1 += nch;
        m.t1_visible.0 += u32::try_from(
            changed
                .iter()
                .filter(|i| i.verdict != Verdict::Hidden)
                .count(),
        )
        .unwrap_or(0);
        m.t1_top.1 += nch.min(u32::try_from(TOP_K).unwrap_or(5));
        m.t1_top.0 += u32::try_from(
            changed
                .iter()
                .filter(|i| i.pos.is_some_and(|x| x < TOP_K))
                .count(),
        )
        .unwrap_or(0)
        .min(u32::try_from(TOP_K).unwrap_or(5));
        for i in &f {
            let c = &p.caps[i.cap];
            let Some(st) = c.at(d) else { continue };
            if hold.contains(&c.instrument) && st.moved.iter().any(|s| s.is_negative()) {
                m.t2_visible.1 += 1;
                m.t2_top.1 += 1;
                if i.verdict != Verdict::Hidden {
                    m.t2_visible.0 += 1;
                }
                if i.pos.is_some_and(|x| x < TOP_K) {
                    m.t2_top.0 += 1;
                }
            }
            // Birth decision → eventual outcome (evaluation only).
            if c.born_day == d && st.moved.contains(&Stage::Born) {
                if let Some((_, Some(hit))) = c.close {
                    let h = u32::from(hit);
                    let (n, r) = i.ledger;
                    let ph = if n > 0 {
                        f64::from(r) / f64::from(n)
                    } else {
                        0.5
                    };
                    let y = f64::from(h);
                    match i.verdict {
                        Verdict::Shown | Verdict::Exempt => {
                            m.kept_hit.0 += h;
                            m.kept_hit.1 += 1;
                            bs_k.0 += (ph - y).powi(2);
                            bs_k.1 += (0.5 - y).powi(2);
                            bs_k.2 += 1;
                            m.kept_pairs.push((ph, y));
                        }
                        Verdict::Hidden => {
                            m.hidden_hit.0 += h;
                            m.hidden_hit.1 += 1;
                            bs_h.0 += (ph - y).powi(2);
                            bs_h.1 += (0.5 - y).powi(2);
                            bs_h.2 += 1;
                        }
                        Verdict::Thin => {
                            m.thin_hit.0 += h;
                            m.thin_hit.1 += 1;
                        }
                    }
                }
            }
        }
    }
    let dn = m.days.max(1) as f64;
    m.kept_per_day = kept as f64 / dn;
    m.rows_per_day = rows as f64 / dn;
    m.hidden_per_day = hidden as f64 / dn;
    m.empty_share = empty as f64 / dn;
    m.thin_share = if vis_total == 0 {
        0.0
    } else {
        thin as f64 / vis_total as f64
    };
    m.kept_skill = if bs_k.1 > 0.0 {
        1.0 - bs_k.0 / bs_k.1
    } else {
        0.0
    };
    m.hidden_skill = if bs_h.1 > 0.0 {
        1.0 - bs_h.0 / bs_h.1
    } else {
        0.0
    };
    m
}

/// Per-day decisions (capsule id, verdict, position) for days `..=upto` — the
/// no-look-ahead test compares these between a full and a truncated history.
#[must_use]
pub fn decisions(
    p: &Prepared,
    cfg: &Config,
    upto: usize,
) -> Vec<Vec<(String, Verdict, Option<usize>)>> {
    (0..=upto.min(p.n_days.saturating_sub(1)))
        .map(|d| {
            feed_on(p, d, cfg)
                .into_iter()
                .map(|i| (p.caps[i.cap].id.clone(), i.verdict, i.pos))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn truncate(inputs: &[InstrumentInput], keep: usize) -> Vec<InstrumentInput> {
        inputs
            .iter()
            .map(|x| InstrumentInput {
                symbol: x.symbol.clone(),
                series: x.series.clone(),
                bars: x.bars[..keep.min(x.bars.len())].to_vec(),
                flow: x.flow.as_ref().map(|f| f[..keep.min(f.len())].to_vec()),
            })
            .collect()
    }

    fn cfgs() -> Vec<Config> {
        vec![
            Config {
                rule: Rule::Wilson { min_n: 5 },
                order: Order::LiveNew,
                exempt: false,
            },
            Config {
                rule: Rule::Rate { min_n: 5, tau: 0.6 },
                order: Order::Trust,
                exempt: true,
            },
            Config {
                rule: Rule::Bayes {
                    min_n: 5,
                    delta: 0.1,
                },
                order: Order::Grouped,
                exempt: false,
            },
            Config {
                rule: Rule::Delta {
                    min_n: 5,
                    delta: 0.1,
                },
                order: Order::LiveNew,
                exempt: false,
            },
        ]
    }

    /// VERIFY 12: moving time forward does not change past decisions. The history is cut
    /// at many days `D`; the decisions of the boundary days `D − 1` and `D` (where a leak
    /// from the future would show) must equal the full history's.
    #[test]
    fn no_look_ahead() {
        let full = synthetic(5, 30, 320);
        let pf = prepare("full", &full);
        let mut hidden_seen = 0;
        for keep in (80..320).step_by(6) {
            let pt = prepare("cut", &truncate(&full, keep));
            let upto = pt.n_days - 1;
            for c in cfgs() {
                for d in [upto - 1, upto] {
                    let a = feed_on(&pf, d, &c);
                    let b = feed_on(&pt, d, &c);
                    let ida = |v: &[Item], p: &Prepared| {
                        v.iter()
                            .map(|i| (p.caps[i.cap].id.clone(), i.verdict, i.pos, i.ledger))
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        ida(&a, &pf),
                        ida(&b, &pt),
                        "day {d} (cut {keep}) differs under {c:?}"
                    );
                    hidden_seen += a.iter().filter(|i| i.verdict == Verdict::Hidden).count();
                }
            }
        }
        assert!(hidden_seen > 0, "the test must exercise hiding");
    }

    /// K53 (2): hidden capsules keep feeding the ledger; the decisions' ledger is the
    /// product's (`SimResult::ledger_stat`).
    #[test]
    fn hidden_still_count_in_ledger() {
        let inputs = synthetic(6, 30, 400);
        let p = prepare("t", &inputs);
        let cfg = Config {
            rule: Rule::Wilson { min_n: 5 },
            order: Order::LiveNew,
            exempt: false,
        };
        let mut hidden_closed = BTreeSet::new();
        for d in p.first_day..p.n_days {
            for i in feed_on(&p, d, &cfg) {
                if i.verdict == Verdict::Hidden && matches!(p.caps[i.cap].close, Some((_, Some(_))))
                {
                    hidden_closed.insert(p.caps[i.cap].id.clone());
                }
            }
        }
        assert!(!hidden_closed.is_empty(), "some hidden capsule must close");
        for c in &hidden_closed {
            assert!(
                p.ledger_capsules.contains(c),
                "hidden capsule {c} has no ledger row"
            );
        }
        // Same numbers as the product's ledger statistic.
        let sim = qp_capsule::simulate(&inputs, &default_detectors, EngineConfig::r3());
        for c in p.caps.iter().take(200) {
            for d in [p.first_day, p.n_days / 2, p.n_days - 1] {
                let st = sim.ledger_stat(&c.key, p.cutoff[d]);
                assert_eq!(p.ledger_at(&c.key, p.cutoff[d]), (st.n, st.realized));
            }
        }
        let total: usize = p.ledger.values().map(Vec::len).sum();
        assert_eq!(total, p.ledger_rows_total);
    }

    #[test]
    fn rules_judge() {
        assert_eq!(Rule::Wilson { min_n: 10 }.judge(9, 5), Verdict::Thin);
        assert_eq!(Rule::Wilson { min_n: 10 }.judge(40, 21), Verdict::Hidden);
        assert_eq!(Rule::Wilson { min_n: 10 }.judge(40, 32), Verdict::Shown);
        assert_eq!(
            Rule::Delta {
                min_n: 10,
                delta: 0.1
            }
            .judge(20, 11),
            Verdict::Hidden
        );
        assert_eq!(
            Rule::Rate {
                min_n: 10,
                tau: 0.7
            }
            .judge(20, 13),
            Verdict::Hidden
        );
        assert_eq!(
            Rule::Rate {
                min_n: 10,
                tau: 0.6
            }
            .judge(20, 13),
            Verdict::Shown
        );
        assert_eq!(
            Rule::Bayes {
                min_n: 5,
                delta: 0.1
            }
            .judge(200, 101),
            Verdict::Hidden
        );
        assert_eq!(
            Rule::Bayes {
                min_n: 5,
                delta: 0.1
            }
            .judge(10, 9),
            Verdict::Shown
        );
        assert!((beta_mass(1.0, 1.0, 0.4, 0.6) - 0.2).abs() < 1e-9);
        assert!((beta_mass(3.0, 5.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
    }
}
