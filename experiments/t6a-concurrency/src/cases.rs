//! Named, scripted conflict cases run on every model.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::am_model::AmModel;
use crate::common::{SemOp, Snap, Who};
use crate::log_model::LogModel;
use crate::loro_model::LoroModel;
use crate::sim::{run_case, CaseResult, Step};

type ABase = LogModel<true, true>;
type ANoBase = LogModel<false, false>;

fn p(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}

fn off() -> Step {
    Step::EndRound {
        editor_online: false,
    }
}

fn on() -> Step {
    Step::EndRound {
        editor_online: true,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseOut {
    pub case: String,
    pub what: String,
    pub results: Vec<CaseResult>,
    /// Model -> key facts extracted from the final state.
    pub facts: BTreeMap<String, BTreeMap<String, String>>,
}

type Probe = fn(&Snap) -> BTreeMap<String, String>;

struct Case {
    name: &'static str,
    what: &'static str,
    steps: Vec<Step>,
    probe: Probe,
    /// Provenance paths: (label, log (id, field), crdt path).
    prov: Vec<(&'static str, (&'static str, &'static str), Vec<&'static str>)>,
}

fn macd(s: &Snap) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    if let Some(x) = s.series.get("macd") {
        m.insert("macd.params".into(), format!("{:?}", x.params));
    }
    m
}

fn layers(s: &Snap) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let l: Vec<String> = s
        .layers
        .iter()
        .map(|l| format!("{}:{}", l.mark, l.series.as_deref().unwrap_or("-")))
        .collect();
    m.insert("layers".into(), l.join(", "));
    m
}

fn finding(s: &Snap) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    if let Some(f) = s.findings.get("fnd-overbought") {
        m.insert("claim".into(), f.claim.clone());
    }
    if let Some((_, c)) = s.capsule_of("fnd-overbought") {
        m.insert("capsule".into(), c.state.clone());
    }
    m.insert(
        "ledger".into(),
        format!(
            "{:?}",
            s.ledger
                .iter()
                .map(|l| (l.state.clone(), l.decided_claim.clone()))
                .collect::<Vec<_>>()
        ),
    );
    m
}

fn sma20(s: &Snap) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert(
        "sma20".into(),
        format!("{:?}", s.series.get("sma20").map(|x| x.params.clone())),
    );
    m.insert(
        "layers drawing sma20".into(),
        s.layers
            .iter()
            .filter(|l| l.series.as_deref() == Some("sma20"))
            .count()
            .to_string(),
    );
    m
}

fn rsi7(s: &Snap) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("rsi7 exists".into(), s.series.contains_key("rsi7").to_string());
    m.insert(
        "layers drawing rsi7".into(),
        s.layers
            .iter()
            .filter(|l| l.series.as_deref() == Some("rsi7"))
            .count()
            .to_string(),
    );
    m.insert(
        "intents on rsi7".into(),
        s.intents
            .values()
            .filter(|t| t.as_deref() == Some("rsi7"))
            .count()
            .to_string(),
    );
    m
}

fn macd_rsi(s: &Snap) -> BTreeMap<String, String> {
    let mut m = macd(s);
    if let Some(x) = s.series.get("rsi14") {
        m.insert("rsi14.params".into(), format!("{:?}", x.params));
    }
    m
}

fn cases() -> Vec<Case> {
    use SemOp::*;
    use Who::*;
    let sp = |series: &str, kv: &[(&str, f64)]| SetParams {
        series: series.into(),
        params: p(kv),
    };
    vec![
        Case {
            name: "C1",
            what: "editor offline sets MACD fast=8; arkin sets MACD fast=10, slow=30; editor returns",
            steps: vec![
                off(),
                Step::Write(Editor, sp("macd", &[("fast", 8.0)])),
                Step::Write(Arkin, sp("macd", &[("fast", 10.0), ("slow", 30.0)])),
                off(),
                Step::Reconnect,
            ],
            probe: macd,
            prov: vec![
                ("macd.fast", ("macd", "fast"), vec!["series", "macd", "params", "fast"]),
                ("macd.slow", ("macd", "slow"), vec!["series", "macd", "params", "slow"]),
            ],
        },
        Case {
            name: "C2",
            what: "same round: arkin removes layer 1 (bb20); scout inserts a macd layer at index 2 (before rsi14)",
            steps: vec![
                Step::Write(
                    Arkin,
                    RemoveLayer {
                        index: 1,
                        series: Some("bb20".into()),
                        mark: "line".into(),
                    },
                ),
                Step::Write(
                    Scout,
                    AddLayer {
                        series: "macd".into(),
                        panel: 1,
                        index: 2,
                        anchor: Some("rsi14".into()),
                    },
                ),
                on(),
            ],
            probe: layers,
            prov: vec![],
        },
        Case {
            name: "C3",
            what: "same round: arkin rejects fnd-overbought; scout updates its claim",
            steps: vec![
                Step::Write(
                    Arkin,
                    Decide {
                        finding: "fnd-overbought".into(),
                        approve: false,
                        note: "zayıf".into(),
                    },
                ),
                Step::Write(
                    Scout,
                    UpdateFinding {
                        finding: "fnd-overbought".into(),
                        claim: Some("Momentum çok güçlü, hedef 120.".into()),
                        confidence: Some(0.9),
                    },
                ),
                on(),
            ],
            probe: finding,
            prov: vec![
                ("finding.claim", ("fnd-overbought", "claim"), vec!["findings", "fnd-overbought", "claim"]),
                ("capsule.state", ("fnd-overbought", "state"), vec!["capsules", "cap-overbought", "state"]),
            ],
        },
        Case {
            name: "C3b",
            what: "same round, reverse order: scout updates the claim first, then arkin rejects",
            steps: vec![
                Step::Write(
                    Scout,
                    UpdateFinding {
                        finding: "fnd-overbought".into(),
                        claim: Some("Momentum çok güçlü, hedef 120.".into()),
                        confidence: Some(0.9),
                    },
                ),
                Step::Write(
                    Arkin,
                    Decide {
                        finding: "fnd-overbought".into(),
                        approve: false,
                        note: "zayıf".into(),
                    },
                ),
                on(),
            ],
            probe: finding,
            prov: vec![],
        },
        Case {
            name: "C4",
            what: "editor offline adds sma20 (period 50, shown); arkin adds sma20 (period 20, shown); editor returns",
            steps: vec![
                off(),
                Step::Write(
                    Editor,
                    AddStudy {
                        id: "sma20".into(),
                        study: "sma".into(),
                        params: p(&[("period", 50.0)]),
                        show_panel: Some(0),
                    },
                ),
                Step::Write(
                    Arkin,
                    AddStudy {
                        id: "sma20".into(),
                        study: "sma".into(),
                        params: p(&[("period", 20.0)]),
                        show_panel: Some(0),
                    },
                ),
                off(),
                Step::Reconnect,
            ],
            probe: sma20,
            prov: vec![("sma20.period", ("sma20", "period"), vec!["series", "sma20", "params", "period"])],
        },
        Case {
            name: "C5",
            what: "rsi7 exists; editor offline adds a layer and an intent on rsi7; arkin removes rsi7; editor returns",
            steps: vec![
                Step::Write(
                    Arkin,
                    AddStudy {
                        id: "rsi7".into(),
                        study: "rsi".into(),
                        params: p(&[("period", 7.0)]),
                        show_panel: None,
                    },
                ),
                on(),
                off(),
                Step::Write(
                    Editor,
                    AddLayer {
                        series: "rsi7".into(),
                        panel: 1,
                        index: 5,
                        anchor: None,
                    },
                ),
                Step::Write(
                    Editor,
                    AddIntent {
                        id: "intent-rsi7".into(),
                        text: "rsi7 70'i geçerse haber ver".into(),
                        trigger_series: "rsi7".into(),
                    },
                ),
                Step::Write(Arkin, RemoveSeries { id: "rsi7".into() }),
                off(),
                Step::Reconnect,
            ],
            probe: rsi7,
            prov: vec![],
        },
        Case {
            name: "C6",
            what: "arkin sets MACD fast=10; scout (having seen it) sets MACD fast=14; arkin undoes",
            steps: vec![
                Step::Write(Arkin, sp("macd", &[("fast", 10.0)])),
                on(),
                Step::Write(Scout, sp("macd", &[("fast", 14.0)])),
                on(),
                Step::Write(Arkin, Undo),
                on(),
            ],
            probe: macd,
            prov: vec![("macd.fast", ("macd", "fast"), vec!["series", "macd", "params", "fast"])],
        },
        Case {
            name: "C6b",
            what: "arkin sets MACD fast=10; scout sets MACD slow=30 (same item, other field); arkin undoes",
            steps: vec![
                Step::Write(Arkin, sp("macd", &[("fast", 10.0)])),
                on(),
                Step::Write(Scout, sp("macd", &[("slow", 30.0)])),
                on(),
                Step::Write(Arkin, Undo),
                on(),
            ],
            probe: macd,
            prov: vec![],
        },
        Case {
            name: "C6c",
            what: "arkin sets MACD fast=10; scout sets RSI14 period=7 (other item); arkin undoes",
            steps: vec![
                Step::Write(Arkin, sp("macd", &[("fast", 10.0)])),
                on(),
                Step::Write(Scout, sp("rsi14", &[("period", 7.0)])),
                on(),
                Step::Write(Arkin, Undo),
                on(),
            ],
            probe: macd_rsi,
            prov: vec![],
        },
        Case {
            name: "C6d",
            what: "arkin sets MACD fast=10; editor OFFLINE sets MACD fast=14 (never saw arkin's); arkin undoes; editor returns",
            steps: vec![
                off(),
                Step::Write(Editor, sp("macd", &[("fast", 14.0)])),
                Step::Write(Arkin, sp("macd", &[("fast", 10.0)])),
                off(),
                Step::Write(Arkin, Undo),
                off(),
                Step::Reconnect,
            ],
            probe: macd,
            prov: vec![("macd.fast", ("macd", "fast"), vec!["series", "macd", "params", "fast"])],
        },
    ]
}

pub fn run_all(doc: &qp_schema::Document) -> Vec<CaseOut> {
    let mut out = Vec::new();
    for c in cases() {
        let mut results = Vec::new();
        let mut facts = BTreeMap::new();
        let mut add = |r: CaseResult, prov: BTreeMap<String, String>| {
            let mut f = (c.probe)(&r.final_snap);
            for (k, v) in prov {
                f.insert(format!("writer({k})"), v);
            }
            facts.insert(r.model.clone(), f);
            results.push(r);
        };
        let (r, m) = run_case::<ABase>(&c.steps, doc);
        add(
            r,
            c.prov
                .iter()
                .map(|(l, (id, f), _)| ((*l).to_string(), m.writer_of(id, f)))
                .collect(),
        );
        let (r, m) = run_case::<ANoBase>(&c.steps, doc);
        add(
            r,
            c.prov
                .iter()
                .map(|(l, (id, f), _)| ((*l).to_string(), m.writer_of(id, f)))
                .collect(),
        );
        let (r, m) = run_case::<AmModel>(&c.steps, doc);
        add(
            r,
            c.prov
                .iter()
                .map(|(l, _, path)| ((*l).to_string(), m.writer_of(path)))
                .collect(),
        );
        let (r, m) = run_case::<LoroModel>(&c.steps, doc);
        add(
            r,
            c.prov
                .iter()
                .map(|(l, _, path)| ((*l).to_string(), m.writer_of(path)))
                .collect(),
        );
        out.push(CaseOut {
            case: c.name.into(),
            what: c.what.into(),
            results,
            facts,
        });
    }
    out
}

pub fn print(all: &[CaseOut]) {
    for c in all {
        println!("== {} — {}", c.case, c.what);
        for r in &c.results {
            println!(
                "  [{}] conv={} anomalies={} undo={:?}",
                r.model,
                r.converged,
                r.anomalies.total(),
                (r.undo.applied, r.undo.refused_conflict, r.undo.clobbered_others)
            );
            for (who, kind, e) in &r.experiences {
                println!("     {who:<6} {kind:<14} -> {e}");
            }
            for d in &r.reconnect.details {
                println!("     reconnect: {d}");
            }
            for (k, v) in &c.facts[&r.model] {
                println!("     {k} = {v}");
            }
        }
    }
}
