//! Authority ↔ replica check (RP2/T8, `docs/PROTOCOL.md` §19.6; VERIFY 12, GOAL
//! criterion 8): a reference authority ([`Session`]) and a [`Replica`] fed the same
//! deterministic tick stream, the replica's own data optionally delayed by `lag` events.
//!
//! [`run`] drives both and records a *tape*: every input the replica received, in order
//! (the authority's messages on its connection, its own data events), and after every
//! step the authority's and the replica's document and view digests and the replica's
//! display. Another build of the replica (WASM in Node, `crates/qp-wasm/tests/`) replays
//! the tape and must reproduce every digest; with `lag = 0` the replica's digests equal
//! the authority's after every step.
//!
//! Tooling, not engine logic: `qp replica-tape` writes the tape; the native tests use
//! [`run`] directly.

use std::path::Path;

use qp_schema::data::{DataEvent, FeedKey};
use qp_schema::protocol::{ClaimRef, ClaimState, Event, Maturity, ServerMsg, SessionInit};
use qp_schema::Condition;
use serde::Serialize;
use serde_json::{json, Value};

use crate::replica::{document_digest, view_digest, Replica, ReplicaOptions};
use crate::session::Session;

/// Bars resolved before the stream.
pub const START: usize = 200;
/// Bars after the stream.
pub const BARS: usize = 236;
/// Seed of the synthetic feed.
pub const SEED: u64 = 11;
/// Events per bar.
pub const TICKS: u32 = 5;
/// View digests are taken at this size.
pub const VIEW: (&str, u32, u32) = ("v", 800, 400);

/// One input of the replica.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    /// An outbound line of the authority on the replica's connection.
    Msg(Value),
    /// One of its own data events.
    Data(DataEvent),
}

/// What is compared after a step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Digests {
    /// Authority document digest.
    pub authority_doc: String,
    /// Authority view digest.
    pub authority_view: String,
    /// Replica document digest.
    pub doc: String,
    /// Replica view digest (over its own data).
    pub view: String,
    /// Replica display digest.
    pub display: String,
}

/// One step of the tape.
#[derive(Debug, Clone, Serialize)]
pub struct TapeStep {
    /// Inputs in order.
    pub inputs: Vec<Input>,
    /// Digests after the step.
    pub expect: Digests,
}

/// The tape.
#[derive(Debug, Clone, Serialize)]
pub struct Tape {
    /// The replica's principal.
    pub principal: String,
    /// The replica's provider at start (`InitData`: feeds and entitlements).
    pub provider: Value,
    /// Its condition subscriptions (id, condition).
    pub watches: Vec<(String, Condition)>,
    /// Replica data delay (events).
    pub lag: usize,
    /// Steps.
    pub steps: Vec<TapeStep>,
}

/// A violation of the display rule found by the independent oracle of [`run`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    /// Step index.
    pub step: usize,
    /// What.
    pub what: String,
}

/// The outcome of [`run`].
#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    /// The tape.
    pub tape: Tape,
    /// Steps whose replica document or view digest differs from the authority's.
    pub differing_steps: usize,
    /// Display-rule violations (wrong or early claims).
    pub violations: Vec<Violation>,
    /// Claims the authority asserted as final at the end.
    pub final_claims: usize,
    /// Of those, shown as final by the replica at the end.
    pub shown_final: usize,
    /// Display items over all steps by state: (shown final, shown provisional, pending,
    /// disputed).
    pub shown_counts: (usize, usize, usize, usize),
}

fn feed() -> FeedKey {
    FeedKey {
        symbol: "THYAO".to_owned(),
        interval: "1d".to_owned(),
        feed: None,
    }
}

/// The scenario's authority init: a bound THYAO series, RSI(14), SMA(10), a view of
/// candles + RSI, a live capsule invalidated by RSI crossing below 45 and an intent on
/// RSI crossing above 62; the authority's host and the replica's principal `dev` are
/// entitled.
#[must_use]
pub fn init() -> Value {
    let prov = json!({"author":{"kind":"agent","name":"scout"},"at":1_760_000_000_000_i64,"based_on":["f1"]});
    json!({
        "document": {
            "schema_version": "0", "id": "doc-replica", "title": "Yansı", "display_tz": "Europe/Istanbul",
            "series": [
                {"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                 "source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":START}}},
                {"id":"rsi14","instrument":{"symbol":"THYAO"},"interval":"1d",
                 "source":{"kind":"study","study":"rsi","inputs":["px"],"params":{"period":14.0}}},
                {"id":"sma10","instrument":{"symbol":"THYAO"},"interval":"1d",
                 "source":{"kind":"study","study":"sma","inputs":["px"],"params":{"period":10.0}}}
            ],
            "views": [{"id":"v","title":"THYAO","x":{"kind":"time"},"y":[{"kind":"linear"},{"kind":"linear"}],
                       "layers":[{"mark":"candle","series":"px","panel":0,"encoding":{}},
                                 {"mark":"line","series":"sma10","panel":0,"encoding":{}},
                                 {"mark":"line","series":"rsi14","panel":1,"encoding":{}}]}],
            "facts": [{"id":"f1","kind":"note","series":"px","bar":10,"t":0,"text":"x","evidence":{},
                       "provenance":{"author":{"kind":"engine","name":"qp-facts","version":"0"},"at":1_760_000_000_000_i64,"based_on":["px"]}}],
            "findings": [{"id":"fnd","claim":"RSI 45'in üstünde kalır.","facts":["f1"],"confidence":0.5,
                          "invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":45.0},"provenance":prov}],
            "capsules": [{"id":"cap","finding":"fnd","state":"live","revision":0,"headline":"RSI 45 üstü.",
                          "glyphs":[],"depth":"capsule","provenance":prov}],
            "intents": [{"id":"int","text":"RSI 62'yi yukarı keserse haber ver.","active":true,
                         "trigger":{"kind":"threshold","series":"rsi14","op":"cross_above","value":62.0},
                         "provenance":prov}],
            "ledger": []
        },
        "clock": 1_760_000_000_000_i64, "epoch": 0, "proposal_ttl_ms": null,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"dev","author":{"kind":"human","name":"dev"},"role":"viewer"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ],
        "data": {"feeds":[{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":START,"seed":SEED}}],
                 "entitlements":{"host":["*"],"arkin":["*"],"dev":["THYAO"]}}
    })
}

/// The replica's provider: the same feed (its own source), `dev` entitled.
#[must_use]
pub fn provider_init() -> Value {
    json!({"feeds":[{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":START,"seed":SEED}}],
           "entitlements":{"dev":["THYAO"]}})
}

/// The replica's condition subscriptions.
#[must_use]
pub fn watches() -> Vec<(String, Condition)> {
    let c = |op: &str, v: f64| -> Condition {
        serde_json::from_value(json!({"kind":"threshold","series":"rsi14","op":op,"value":v}))
            .expect("condition")
    };
    vec![
        ("up55".to_owned(), c("cross_above", 55.0)),
        ("dn45".to_owned(), c("cross_below", 45.0)),
    ]
}

/// The tick stream: every bar in `TICKS` events, every 6th bar first delivered wrong and
/// corrected two bars later.
#[must_use]
pub fn events() -> Vec<DataEvent> {
    let mut cfg = qp_source::SyntheticEventsConfig::new(BARS, SEED, TICKS);
    cfg.start = START;
    cfg.correct_every = 6;
    cfg.correct_after = 2;
    qp_source::SyntheticEvents::new(feed(), cfg).collect()
}

/// Builds a memory provider from an `InitData` JSON value.
///
/// # Errors
/// Invalid provider description.
pub fn memory_provider(v: &Value) -> Result<qp_source::MemoryProvider, String> {
    let d: qp_schema::protocol::InitData =
        serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    qp_source::MemoryProvider::from_init(&d, Path::new(".")).map_err(|e| e.to_string())
}

struct Driver {
    a: Session,
    r: Replica,
    inputs: Vec<Input>,
    /// Latest authority word per claim, as sent to the replica's connection.
    said: std::collections::BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)>,
}

impl Driver {
    fn deliver(&mut self, lines: Vec<qp_schema::protocol::OutboundLine>) {
        let mut queue: std::collections::VecDeque<_> = lines.into();
        while let Some(l) = queue.pop_front() {
            if l.conn != "r" {
                continue;
            }
            if let ServerMsg::Notify {
                event:
                    Event::Claim {
                        claim,
                        state,
                        evidence,
                        ..
                    },
                ..
            } = &l.msg
            {
                self.said
                    .insert((claim.clone(), evidence.t), (*state, evidence.maturity));
            }
            let v = serde_json::to_value(&l).expect("line");
            self.inputs.push(Input::Msg(v));
            self.r.on_message(&l.msg);
            for m in self.r.take_outbox() {
                queue.extend(self.a.handle_raw("r", &m.to_string()));
            }
        }
    }

    fn data(&mut self, ev: &DataEvent) {
        self.inputs.push(Input::Data(ev.clone()));
        // The replica's own feed follows the §18.4 rules; this stream has no gaps.
        let _ = self.r.data_event(ev);
    }
}

/// The predicate on two values (`values::cmp` semantics, written out independently).
fn holds(op: &str, prev: Option<f64>, v: Option<f64>, k: f64) -> bool {
    match (op, prev, v) {
        ("cross_above", Some(p), Some(v)) => p < k && v >= k,
        ("cross_below", Some(p), Some(v)) => p > k && v <= k,
        _ => false,
    }
}

/// Runs the scenario with the replica's data `lag` events behind the authority's.
///
/// # Errors
/// The scenario cannot be set up.
pub fn run(lag: usize) -> Result<Outcome, String> {
    run_with(lag, &provider_init())
}

/// [`run`] with another replica provider (e.g. one that does not entitle `dev`).
///
/// # Errors
/// The scenario cannot be set up.
#[allow(clippy::too_many_lines)]
pub fn run_with(lag: usize, provider: &Value) -> Result<Outcome, String> {
    let init: SessionInit = serde_json::from_value(init()).map_err(|e| e.to_string())?;
    let a = Session::new(init, Path::new("."))?;
    let r = Replica::with_memory("dev", memory_provider(provider)?);
    let mut d = Driver {
        a,
        r,
        inputs: Vec::new(),
        said: std::collections::BTreeMap::new(),
    };
    d.r.set_options(ReplicaOptions::default());
    let hello =
        json!({"type":"hello","protocol":"qp/0","principal":"dev","surface":{"kind":"phone"}});
    let lines = d.a.handle_raw("r", &hello.to_string());
    for (id, c) in watches() {
        d.r.watch(&id, c);
    }
    d.deliver(lines);
    let evs = events();
    let mut steps = Vec::new();
    let mut violations = Vec::new();
    let mut differing = 0;
    let mut counts = (0, 0, 0, 0);
    let wl = watches();
    let total = evs.len() + lag;
    for i in 0..total {
        if let Some(ev) = evs.get(i) {
            let lines = d.a.data_event(ev).map_err(|e| e.to_string())?;
            d.deliver(lines);
        }
        if i >= lag {
            d.data(&evs[i - lag]);
        }
        let mut dg = Digests {
            authority_doc: document_digest(d.a.document()),
            authority_view: view_digest(&mut d.a, VIEW.0, VIEW.1, VIEW.2).map_err(|e| e.message)?,
            doc: d.r.document_digest().unwrap_or_default(),
            view: d
                .r
                .view_digest(VIEW.0, VIEW.1, VIEW.2)
                .unwrap_or_else(|e| format!("error: {e}")),
            display: d.r.display_digest(),
        };
        if dg.doc != dg.authority_doc || dg.view != dg.authority_view {
            differing += 1;
        }
        // Independent oracle over the replica's own visible bars.
        let own =
            d.r.bars("px")
                .map(<[qp_schema::Bar]>::to_vec)
                .unwrap_or_default();
        let rsi = qp_compute::compute_bars(
            "rsi",
            &std::collections::BTreeMap::from([("period".to_owned(), 14.0)]),
            qp_compute::Field::Close,
            &own,
        )
        .map_err(|e| e.to_string())?;
        let col = rsi.outputs.get("rsi").cloned().unwrap_or_default();
        let doc = d.r.document().cloned();
        for it in d.r.display() {
            match (it.state, it.label) {
                (crate::replica::Shown::Shown, Some(Maturity::Final)) => counts.0 += 1,
                (crate::replica::Shown::Shown, _) => counts.1 += 1,
                (crate::replica::Shown::Pending, _) => counts.2 += 1,
                (crate::replica::Shown::Disputed, _) => counts.3 += 1,
            }
            if it.state != crate::replica::Shown::Shown {
                continue;
            }
            let bad = |w: String| Violation { step: i, what: w };
            let key = (it.claim.clone(), it.t);
            let said = d.said.get(&key).copied();
            if !matches!(said, Some((ClaimState::Asserted, _))) {
                violations.push(bad(format!("{key:?} shown but not asserted ({said:?})")));
            }
            let cond = match &it.claim {
                ClaimRef::Condition { sub } => {
                    wl.iter().find(|(s, _)| s == sub).map(|x| x.1.clone())
                }
                ClaimRef::Intent { .. } => doc
                    .as_ref()
                    .and_then(|x| x.intents.first().and_then(|i| i.trigger.clone())),
                ClaimRef::Capsule { .. } => doc
                    .as_ref()
                    .and_then(|x| x.findings.first().map(|f| f.invalidation.clone())),
            };
            let Some(Condition::Threshold { op, value, .. }) = cond else {
                violations.push(bad(format!("{key:?}: no condition")));
                continue;
            };
            let op = serde_json::to_value(op)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let Some(j) = own.iter().position(|b| b.t == it.t) else {
                violations.push(bad(format!("{key:?} shown, own data lacks t")));
                continue;
            };
            let (p, v) = (j.checked_sub(1).and_then(|k| col[k]), col[j]);
            if !holds(&op, p, v, value) {
                violations.push(bad(format!("{key:?} shown, predicate false on own data")));
            }
            let shown = it.values.first().map(|x| (x.prev, x.value));
            if shown != Some((p, v)) {
                violations.push(bad(format!("{key:?} shown values are not its own")));
            }
            match it.label {
                Some(Maturity::Final) => {
                    if j + 1 >= own.len() {
                        violations.push(bad(format!("{key:?} final before its own bar closed")));
                    }
                    if !matches!(said, Some((_, Maturity::Final))) {
                        violations.push(bad(format!("{key:?} final before the authority's final")));
                    }
                }
                _ => violations.push(bad(format!("{key:?} provisional with live display off"))),
            }
        }
        dg.display = d.r.display_digest();
        steps.push(TapeStep {
            inputs: std::mem::take(&mut d.inputs),
            expect: dg,
        });
    }
    let final_keys: Vec<_> = d
        .said
        .iter()
        .filter(|(_, (s, m))| *s == ClaimState::Asserted && *m == Maturity::Final)
        .map(|(k, _)| k.clone())
        .collect();
    let shown_final = final_keys
        .iter()
        .filter(|k| {
            d.r.display().iter().any(|it| {
                it.claim == k.0
                    && it.t == k.1
                    && it.state == crate::replica::Shown::Shown
                    && it.label == Some(Maturity::Final)
            })
        })
        .count();
    Ok(Outcome {
        tape: Tape {
            principal: "dev".to_owned(),
            provider: provider.clone(),
            watches: watches(),
            lag,
            steps,
        },
        differing_steps: differing,
        violations,
        final_claims: final_keys.len(),
        shown_final,
        shown_counts: counts,
    })
}
