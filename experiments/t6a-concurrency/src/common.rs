//! Shared harness: deterministic RNG, the semantic read model (`Snap`), the semantic
//! operation subset (`SemOp`), workload generation, local validation and the anomaly
//! checker. Every model is driven by exactly the same functions here.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

// ---------------------------------------------------------------------------
// Principals
// ---------------------------------------------------------------------------

/// The three writers of the scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Who {
    Arkin,
    Scout,
    Editor,
}

pub const ALL: [Who; 3] = [Who::Arkin, Who::Scout, Who::Editor];

impl Who {
    pub fn name(self) -> &'static str {
        match self {
            Who::Arkin => "arkin",
            Who::Scout => "scout",
            Who::Editor => "editor",
        }
    }
    pub fn idx(self) -> usize {
        self as usize
    }
    pub fn from_idx(i: usize) -> Who {
        ALL[i]
    }
    pub fn from_name(s: &str) -> Option<Who> {
        ALL.iter().copied().find(|w| w.name() == s)
    }
    /// Only the human may decide (approve/reject) and undo in this scenario.
    pub fn may_decide(self) -> bool {
        self == Who::Arkin
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64): no wall clock, no OS randomness.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Pick from a slice with a raw draw.
pub fn pick<T: Clone>(xs: &[T], r: u64) -> Option<T> {
    if xs.is_empty() {
        None
    } else {
        Some(xs[(r % xs.len() as u64) as usize].clone())
    }
}

// ---------------------------------------------------------------------------
// Semantic snapshot: the model-independent read model of a replica.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct SSeries {
    pub kind: String, // "study" | "inline"
    pub study: String,
    pub inputs: Vec<String>,
    pub params: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SLayer {
    pub mark: String,
    pub series: Option<String>,
    pub panel: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SFinding {
    pub claim: String,
    pub confidence: f64,
    pub inval_series: String,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SCapsule {
    pub finding: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SLedger {
    pub finding: String,
    pub state: String,
    /// The claim the decider saw when deciding.
    pub decided_claim: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct Snap {
    pub series: BTreeMap<String, SSeries>,
    pub layers: Vec<SLayer>,
    pub facts: BTreeMap<String, String>,
    pub findings: BTreeMap<String, SFinding>,
    pub capsules: BTreeMap<String, SCapsule>,
    pub intents: BTreeMap<String, Option<String>>,
    pub ledger: Vec<SLedger>,
}

impl Snap {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("snap json")
    }
    pub fn id_exists(&self, id: &str) -> bool {
        self.series.contains_key(id)
            || self.facts.contains_key(id)
            || self.findings.contains_key(id)
            || self.capsules.contains_key(id)
            || self.intents.contains_key(id)
            || id == "main"
    }
    pub fn capsule_of(&self, finding: &str) -> Option<(&String, &SCapsule)> {
        self.capsules.iter().find(|(_, c)| c.finding == finding)
    }
    pub fn study_ids(&self) -> Vec<String> {
        self.series
            .iter()
            .filter(|(_, s)| s.kind == "study")
            .map(|(k, _)| k.clone())
            .collect()
    }
    pub fn max_panel(&self) -> u32 {
        self.layers.iter().map(|l| l.panel).max().unwrap_or(0)
    }
    pub fn live_findings(&self) -> Vec<String> {
        self.findings
            .keys()
            .filter(|f| matches!(self.capsule_of(f), Some((_, c)) if c.state == "live"))
            .cloned()
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Semantic operations (the protocol subset under test)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SemOp {
    AddStudy {
        id: String,
        study: String,
        params: BTreeMap<String, f64>,
        show_panel: Option<u32>,
    },
    SetParams {
        series: String,
        params: BTreeMap<String, f64>,
    },
    /// `index` is the position in the writer's view; `anchor` is the series of the layer
    /// currently at `index`, i.e. the new layer goes just before it (`None` = append).
    /// Used to re-resolve the position after a conflict.
    AddLayer {
        series: String,
        panel: u32,
        index: u32,
        anchor: Option<String>,
    },
    /// `index` in the writer's view; `series` identifies the layer for re-resolution.
    RemoveLayer {
        index: u32,
        series: Option<String>,
        mark: String,
    },
    RemoveSeries {
        id: String,
    },
    AddFinding {
        id: String,
        claim: String,
        confidence: f64,
        inval_series: String,
    },
    UpdateFinding {
        finding: String,
        claim: Option<String>,
        confidence: Option<f64>,
    },
    Decide {
        finding: String,
        approve: bool,
        note: String,
    },
    AddIntent {
        id: String,
        text: String,
        trigger_series: String,
    },
    RemoveFinding {
        id: String,
    },
    Undo,
}

impl SemOp {
    pub fn kind(&self) -> &'static str {
        match self {
            SemOp::AddStudy { .. } => "add_study",
            SemOp::SetParams { .. } => "set_params",
            SemOp::AddLayer { .. } => "add_layer",
            SemOp::RemoveLayer { .. } => "remove_layer",
            SemOp::RemoveSeries { .. } => "remove_series",
            SemOp::AddFinding { .. } => "add_finding",
            SemOp::UpdateFinding { .. } => "update_finding",
            SemOp::Decide { .. } => "decide",
            SemOp::AddIntent { .. } => "add_intent",
            SemOp::RemoveFinding { .. } => "remove_finding",
            SemOp::Undo => "undo",
        }
    }

    /// Register-like fields this op writes, as stable keys (for lost-update accounting).
    pub fn written_fields(&self) -> Vec<String> {
        match self {
            SemOp::SetParams { series, params } => params
                .keys()
                .map(|k| format!("series/{series}/params/{k}"))
                .collect(),
            SemOp::UpdateFinding {
                finding,
                claim,
                confidence,
            } => {
                let mut v = Vec::new();
                if claim.is_some() {
                    v.push(format!("finding/{finding}/claim"));
                }
                if confidence.is_some() {
                    v.push(format!("finding/{finding}/confidence"));
                }
                v
            }
            SemOp::Decide { finding, .. } => vec![format!("capsule-of/{finding}/state")],
            _ => Vec::new(),
        }
    }

    /// Ids this op creates (for id-collision accounting).
    pub fn created_id(&self) -> Option<&str> {
        match self {
            SemOp::AddStudy { id, .. }
            | SemOp::AddFinding { id, .. }
            | SemOp::AddIntent { id, .. } => Some(id),
            _ => None,
        }
    }
}

pub fn study_param_names(study: &str) -> &'static [&'static str] {
    match study {
        "sma" | "ema" | "rsi" => &["period"],
        "macd" => &["fast", "slow", "signal"],
        "bollinger" => &["period", "k"],
        _ => &[],
    }
}

/// Error codes shared by all models (subset of protocol §13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Code {
    NotFound,
    Duplicate,
    Invalid,
    InUse,
    Forbidden,
    Conflict,
    UndoConflict,
    Unsupported,
}

/// Local validation against a snapshot (what a careful client or the authority checks).
pub fn validate(s: &Snap, who: Who, op: &SemOp) -> Result<(), Code> {
    match op {
        SemOp::AddStudy {
            id,
            study,
            show_panel,
            ..
        } => {
            if s.id_exists(id) {
                return Err(Code::Duplicate);
            }
            if study_param_names(study).is_empty() {
                return Err(Code::Invalid);
            }
            if let Some(p) = show_panel {
                if *p > s.max_panel() + 1 {
                    return Err(Code::Invalid);
                }
            }
            Ok(())
        }
        SemOp::SetParams { series, params } => {
            let Some(ser) = s.series.get(series) else {
                return Err(Code::NotFound);
            };
            if ser.kind != "study" {
                return Err(Code::Invalid);
            }
            let names = study_param_names(&ser.study);
            if params.keys().any(|k| !names.contains(&k.as_str())) {
                return Err(Code::Invalid);
            }
            Ok(())
        }
        SemOp::AddLayer { series, index, .. } => {
            if !s.series.contains_key(series) {
                return Err(Code::NotFound);
            }
            if *index as usize > s.layers.len() {
                return Err(Code::Invalid);
            }
            Ok(())
        }
        SemOp::RemoveLayer { index, .. } => {
            if *index as usize >= s.layers.len() {
                return Err(Code::Invalid);
            }
            Ok(())
        }
        SemOp::RemoveSeries { id } => {
            let Some(ser) = s.series.get(id) else {
                return Err(Code::NotFound);
            };
            if ser.kind != "study" {
                return Err(Code::Invalid);
            }
            if !dependents(s, id).is_empty() {
                return Err(Code::InUse);
            }
            Ok(())
        }
        SemOp::AddFinding {
            id, inval_series, ..
        } => {
            if s.id_exists(id) || s.id_exists(&format!("cap-{id}")) {
                return Err(Code::Duplicate);
            }
            if !s.series.contains_key(inval_series) {
                return Err(Code::NotFound);
            }
            if !s.facts.contains_key(FACT) {
                return Err(Code::NotFound);
            }
            Ok(())
        }
        SemOp::UpdateFinding { finding, .. } => {
            if !s.findings.contains_key(finding) {
                return Err(Code::NotFound);
            }
            match s.capsule_of(finding) {
                Some((_, c)) if c.state != "live" => Err(Code::Invalid),
                _ => Ok(()),
            }
        }
        SemOp::Decide { finding, .. } => {
            if !who.may_decide() {
                return Err(Code::Forbidden);
            }
            if !s.findings.contains_key(finding) {
                return Err(Code::NotFound);
            }
            match s.capsule_of(finding) {
                Some((_, c)) if c.state == "live" => Ok(()),
                Some(_) => Err(Code::Invalid),
                None => Err(Code::NotFound),
            }
        }
        SemOp::AddIntent {
            id, trigger_series, ..
        } => {
            if s.id_exists(id) {
                return Err(Code::Duplicate);
            }
            if !s.series.contains_key(trigger_series) {
                return Err(Code::NotFound);
            }
            Ok(())
        }
        SemOp::RemoveFinding { id } => {
            if !s.findings.contains_key(id) {
                return Err(Code::NotFound);
            }
            if s.ledger.iter().any(|l| &l.finding == id) {
                return Err(Code::InUse);
            }
            Ok(())
        }
        SemOp::Undo => {
            if who.may_decide() {
                Ok(())
            } else {
                Err(Code::Forbidden)
            }
        }
    }
}

/// Items that depend on a series (study inputs, facts, conditions). Layers are not
/// dependents: removing a series removes the layers that draw it.
pub fn dependents(s: &Snap, id: &str) -> Vec<String> {
    let mut d = Vec::new();
    for (k, ser) in &s.series {
        if ser.inputs.iter().any(|i| i == id) {
            d.push(k.clone());
        }
    }
    for (k, ser) in &s.facts {
        if ser == id {
            d.push(k.clone());
        }
    }
    for (k, f) in &s.findings {
        if f.inval_series == id {
            d.push(k.clone());
        }
    }
    for (k, t) in &s.intents {
        if t.as_deref() == Some(id) {
            d.push(k.clone());
        }
    }
    d
}

pub const FACT: &str = "fact-rsi-cross70";

// ---------------------------------------------------------------------------
// Workload: a list of intents; each is concretized against the author's own view.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Intent {
    pub round: usize,
    pub who: Who,
    pub n: usize, // per-author sequence number
    pub r: [u64; 5],
}

#[derive(Debug, Clone, Serialize)]
pub struct Workload {
    pub seed: u64,
    pub rounds: usize,
    pub offline_from: usize,
    pub offline_to: usize, // exclusive
    pub intents: Vec<Intent>,
}

/// `ops` intents in rounds of three (one per author). The editor is offline for
/// `window` of its own intents (= `window` rounds), starting at one third of the run.
pub fn workload(seed: u64, ops: usize, window: usize) -> Workload {
    let mut rng = Rng::new(seed);
    let rounds = ops.div_ceil(3);
    let offline_from = rounds / 3;
    let offline_to = (offline_from + window).min(rounds);
    let mut intents = Vec::with_capacity(ops);
    let mut n = [0usize; 3];
    'outer: for round in 0..rounds {
        for who in ALL {
            if intents.len() >= ops {
                break 'outer;
            }
            let r = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
            intents.push(Intent {
                round,
                who,
                n: n[who.idx()],
                r,
            });
            n[who.idx()] += 1;
        }
    }
    Workload {
        seed,
        rounds,
        offline_from,
        offline_to,
        intents,
    }
}

const STUDY_POOL: [(&str, &str); 8] = [
    ("sma20", "sma"),
    ("sma50", "sma"),
    ("ema9", "ema"),
    ("ema21", "ema"),
    ("rsi7", "rsi"),
    ("bb10", "bollinger"),
    ("sma100", "sma"),
    ("macd2", "macd"),
];

fn weights(who: Who) -> &'static [(&'static str, u64)] {
    match who {
        Who::Arkin => &[
            ("set_params", 20),
            ("remove_layer", 10),
            ("add_layer", 10),
            ("decide", 12),
            ("remove_series", 6),
            ("add_intent", 6),
            ("add_study", 8),
            ("undo", 8),
        ],
        Who::Scout => &[
            ("add_study", 18),
            ("add_finding", 18),
            ("update_finding", 18),
            ("add_layer", 14),
            ("set_params", 12),
            ("add_intent", 6),
            ("remove_finding", 4),
            ("remove_series", 4),
        ],
        Who::Editor => &[
            ("set_params", 22),
            ("add_layer", 12),
            ("remove_layer", 10),
            ("update_finding", 14),
            ("add_study", 14),
            ("remove_series", 8),
            ("add_finding", 8),
            ("add_intent", 6),
        ],
    }
}

fn params_for(study: &str, r: u64) -> BTreeMap<String, f64> {
    let mut p = BTreeMap::new();
    match study {
        "macd" => {
            // Pick one or two of the three, with values from small grids.
            let which = r % 4;
            if which != 1 {
                p.insert("fast".into(), (6 + (r >> 8) % 10) as f64);
            }
            if which != 0 {
                p.insert("slow".into(), (20 + (r >> 16) % 15) as f64);
            }
            if which == 3 {
                p.insert("signal".into(), (5 + (r >> 24) % 7) as f64);
            }
        }
        "bollinger" => {
            if r % 2 == 0 {
                p.insert("period".into(), (10 + (r >> 8) % 30) as f64);
            } else {
                p.insert("k".into(), 1.5 + ((r >> 8) % 4) as f64 * 0.5);
            }
        }
        _ => {
            p.insert("period".into(), (5 + (r >> 8) % 60) as f64);
        }
    }
    p
}

/// Turn an intent into a concrete operation against the author's view. `None` if the
/// chosen kind has no target in this view (the author idles).
pub fn concretize(s: &Snap, it: &Intent) -> Option<SemOp> {
    let w = weights(it.who);
    let total: u64 = w.iter().map(|(_, x)| x).sum();
    let mut roll = it.r[0] % total;
    let mut kind = w[0].0;
    for (k, x) in w {
        if roll < *x {
            kind = k;
            break;
        }
        roll -= x;
    }
    let [_, a, b, c, d] = it.r;
    let studies = s.study_ids();
    match kind {
        "set_params" => {
            // Hot spot: MACD gets 30% of parameter edits.
            let series = if a % 10 < 3 && s.series.contains_key("macd") {
                "macd".to_string()
            } else {
                pick(&studies, b)?
            };
            let study = s.series.get(&series)?.study.clone();
            Some(SemOp::SetParams {
                series,
                params: params_for(&study, c),
            })
        }
        "add_layer" => {
            let series = pick(&studies, a)?;
            let index = (b % (s.layers.len() as u64 + 1)) as u32;
            // Anchor: the layer currently at `index` (the new one goes before it).
            let anchor = s.layers.get(index as usize).and_then(|l| l.series.clone());
            let panel = (c % (u64::from(s.max_panel()) + 1)) as u32;
            Some(SemOp::AddLayer {
                series,
                panel,
                index,
                anchor,
            })
        }
        "remove_layer" => {
            if s.layers.len() <= 1 {
                return None;
            }
            // Never remove the candle layer at 0 in this workload.
            let index = 1 + (a % (s.layers.len() as u64 - 1)) as u32;
            let l = &s.layers[index as usize];
            Some(SemOp::RemoveLayer {
                index,
                series: l.series.clone(),
                mark: l.mark.clone(),
            })
        }
        "decide" => {
            let live = s.live_findings();
            let finding = if a % 10 < 4 && live.iter().any(|f| f == "fnd-overbought") {
                "fnd-overbought".to_string()
            } else {
                pick(&live, b)?
            };
            Some(SemOp::Decide {
                finding,
                approve: c % 3 == 0,
                note: format!("karar {}-{}", it.who.name(), it.n),
            })
        }
        "remove_series" => {
            let cands: Vec<String> = studies
                .into_iter()
                .filter(|id| dependents(s, id).is_empty())
                .collect();
            let id = pick(&cands, a)?;
            Some(SemOp::RemoveSeries { id })
        }
        "add_intent" => {
            let trigger_series = pick(&studies, a)?;
            Some(SemOp::AddIntent {
                id: format!("intent-{}-{}", it.who.name(), it.n),
                text: format!("{trigger_series} eşiği geçerse haber ver"),
                trigger_series,
            })
        }
        "add_study" => {
            let (id, study) = if a % 2 == 0 {
                let (id, st) = STUDY_POOL[(b % STUDY_POOL.len() as u64) as usize];
                (id.to_string(), st.to_string())
            } else {
                let st = ["sma", "ema", "rsi", "macd", "bollinger"][(b % 5) as usize];
                (format!("s-{}-{}", it.who.name(), it.n), st.to_string())
            };
            let show_panel = if c % 4 == 0 {
                None
            } else {
                Some((d % (u64::from(s.max_panel()) + 2)) as u32)
            };
            Some(SemOp::AddStudy {
                params: params_for(&study, d),
                id,
                study,
                show_panel,
            })
        }
        "add_finding" => {
            let inval_series = pick(&studies, a)?;
            // Small id pool so concurrent creators can collide.
            let id = if b % 3 == 0 {
                format!("fnd-pool-{}", c % 6)
            } else {
                format!("fnd-{}-{}", it.who.name(), it.n)
            };
            Some(SemOp::AddFinding {
                id,
                claim: format!("{} iddiası {}", it.who.name(), it.n),
                confidence: 0.4 + (d % 50) as f64 / 100.0,
                inval_series,
            })
        }
        "update_finding" => {
            let live = s.live_findings();
            let finding = if a % 10 < 4 && live.iter().any(|f| f == "fnd-overbought") {
                "fnd-overbought".to_string()
            } else {
                pick(&live, b)?
            };
            let (claim, confidence) = match c % 3 {
                0 => (Some(format!("{} revize {}", it.who.name(), it.n)), None),
                1 => (None, Some(0.3 + (d % 60) as f64 / 100.0)),
                _ => (
                    Some(format!("{} revize {}", it.who.name(), it.n)),
                    Some(0.3 + (d % 60) as f64 / 100.0),
                ),
            };
            Some(SemOp::UpdateFinding {
                finding,
                claim,
                confidence,
            })
        }
        "remove_finding" => {
            let cands: Vec<String> = s
                .findings
                .keys()
                .filter(|f| !s.ledger.iter().any(|l| &l.finding == *f))
                .filter(|f| f.as_str() != "fnd-overbought")
                .cloned()
                .collect();
            Some(SemOp::RemoveFinding {
                id: pick(&cands, a)?,
            })
        }
        "undo" => Some(SemOp::Undo),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Anomaly checker (final state)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Anomalies {
    /// Layer draws a series that does not exist.
    pub dangling_layer: usize,
    /// Study input, fact, finding invalidation or intent trigger names a missing series.
    pub dangling_ref: usize,
    /// Capsule or ledger entry whose finding does not exist.
    pub orphan: usize,
    /// Capsule state disagrees with the last ledger decision, or two decisions for one
    /// finding (e.g. both approved and rejected).
    pub decision_mismatch: usize,
    /// The finding's claim changed after (or concurrently with) the decision.
    pub stale_decision: usize,
    /// A study series whose parameters are not valid for its study (mixed creates).
    pub invalid_item: usize,
}

impl Anomalies {
    pub fn total(&self) -> usize {
        self.dangling_layer
            + self.dangling_ref
            + self.orphan
            + self.decision_mismatch
            + self.stale_decision
            + self.invalid_item
    }
}

pub fn anomalies(s: &Snap) -> Anomalies {
    let mut a = Anomalies::default();
    for l in &s.layers {
        if let Some(ser) = &l.series {
            if !s.series.contains_key(ser) {
                a.dangling_layer += 1;
            }
        }
    }
    for ser in s.series.values() {
        for i in &ser.inputs {
            if !s.series.contains_key(i) {
                a.dangling_ref += 1;
            }
        }
        if ser.kind == "study" {
            let names = study_param_names(&ser.study);
            if ser.params.keys().any(|k| !names.contains(&k.as_str())) {
                a.invalid_item += 1;
            }
        }
    }
    for f in s.facts.values() {
        if !s.series.contains_key(f) {
            a.dangling_ref += 1;
        }
    }
    for f in s.findings.values() {
        if !s.series.contains_key(&f.inval_series) {
            a.dangling_ref += 1;
        }
        for fact in &f.facts {
            if !s.facts.contains_key(fact) {
                a.dangling_ref += 1;
            }
        }
    }
    for t in s.intents.values().flatten() {
        if !s.series.contains_key(t) {
            a.dangling_ref += 1;
        }
    }
    for c in s.capsules.values() {
        if !s.findings.contains_key(&c.finding) {
            a.orphan += 1;
        }
    }
    let mut decided: BTreeMap<&str, Vec<&SLedger>> = BTreeMap::new();
    for l in &s.ledger {
        if !s.findings.contains_key(&l.finding) {
            a.orphan += 1;
        }
        decided.entry(&l.finding).or_default().push(l);
    }
    for (f, ls) in &decided {
        let states: BTreeSet<&str> = ls.iter().map(|l| l.state.as_str()).collect();
        if ls.len() > 1 {
            a.decision_mismatch += ls.len() - 1;
        }
        let last = ls.last().expect("non-empty");
        if let Some((_, c)) = s.capsule_of(f) {
            if c.state != last.state || states.len() > 1 {
                a.decision_mismatch += 1;
            }
        }
        if let Some(fd) = s.findings.get(*f) {
            if fd.claim != last.decided_claim {
                a.stale_decision += 1;
            }
        }
    }
    a
}

// ---------------------------------------------------------------------------
// Concurrency accounting with vector clocks (model-independent).
// ---------------------------------------------------------------------------

pub type Vc = [u32; 3];

pub fn vc_le(a: &Vc, b: &Vc) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y)
}

pub fn vc_max(a: &Vc, b: &Vc) -> Vc {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]
}

/// One locally accepted write, with the writer's clock at write time.
#[derive(Debug, Clone)]
pub struct WriteRec {
    pub who: Who,
    pub vc: Vc,
    pub op: SemOp,
}

/// Counts, over locally accepted writes, pairs that were concurrent (neither saw the
/// other) on the same register field, and creates of the same id by different writers.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConcurrencyStats {
    pub concurrent_same_field_pairs: usize,
    pub concurrent_same_id_creates: usize,
    pub concurrent_decide_vs_update: usize,
}

pub fn concurrency_stats(writes: &[WriteRec]) -> ConcurrencyStats {
    let mut st = ConcurrencyStats::default();
    let mut by_field: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut by_id: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut decides: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut updates: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, w) in writes.iter().enumerate() {
        for f in w.op.written_fields() {
            by_field.entry(f).or_default().push(i);
        }
        if let Some(id) = w.op.created_id() {
            by_id.entry(id.to_string()).or_default().push(i);
        }
        match &w.op {
            SemOp::Decide { finding, .. } => decides.entry(finding.clone()).or_default().push(i),
            SemOp::UpdateFinding { finding, .. } => {
                updates.entry(finding.clone()).or_default().push(i)
            }
            _ => {}
        }
    }
    let conc = |i: usize, j: usize| {
        let (a, b) = (&writes[i], &writes[j]);
        a.who != b.who && !vc_le(&a.vc, &b.vc) && !vc_le(&b.vc, &a.vc)
    };
    for ix in by_field.values() {
        for (p, &i) in ix.iter().enumerate() {
            for &j in &ix[p + 1..] {
                if conc(i, j) {
                    st.concurrent_same_field_pairs += 1;
                }
            }
        }
    }
    for ix in by_id.values() {
        for (p, &i) in ix.iter().enumerate() {
            for &j in &ix[p + 1..] {
                if conc(i, j) {
                    st.concurrent_same_id_creates += 1;
                }
            }
        }
    }
    for (f, ds) in &decides {
        if let Some(us) = updates.get(f) {
            for &i in ds {
                for &j in us {
                    if conc(i, j) {
                        st.concurrent_decide_vs_update += 1;
                    }
                }
            }
        }
    }
    st
}

/// Load the start document (fixture) once.
pub fn start_document() -> qp_schema::Document {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/protocol/docs/thyao.json"
    );
    let s = std::fs::read_to_string(path).expect("read fixture");
    serde_json::from_str(&s).expect("parse fixture")
}

/// Hex SHA-256 for determinism checks.
pub fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}
