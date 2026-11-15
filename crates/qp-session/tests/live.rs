//! The incremental live path (RP2/T5, `docs/PROTOCOL.md` §18.8).
//!
//! - Property test (fixed seeds): random sequences of new bars, live revisions, past-bar
//!   corrections, undos, structure changes and queries, on inline and bound series, with
//!   5–50 study series, condition subscriptions, intents and live capsules. The
//!   incremental session and the from-scratch reference (`set_incremental(false)`: every
//!   bar change clears the values cache, every watcher and capsule is evaluated, bar
//!   commands go through the copying expansion) get the same inputs; every outbound
//!   message, the document bytes, the log bytes and every study value (bits, also
//!   against `qp_compute::compute_bars`) must be identical.
//! - Wake only the affected: two views on two feeds; a tick on one feed sends nothing to
//!   subscribers of the other view or to conditions on the other feed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::{Bar, SeriesSource};
use qp_session::{replay, Session};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// SplitMix64 (test data only).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

const DAY: i64 = 86_400_000;
const T0: i64 = 1_700_000_000_000;

fn walk(rng: &mut Rng, n: usize, start: f64) -> Vec<Bar> {
    let mut out = Vec::with_capacity(n);
    let mut c = start;
    for i in 0..n {
        let o = c;
        c = r2((o * (1.0 + (rng.unit() - 0.5) * 0.05)).max(1.0));
        out.push(Bar {
            t: T0 + i as i64 * DAY,
            o,
            h: r2(o.max(c) * (1.0 + rng.unit() * 0.01)),
            l: r2(o.min(c) * (1.0 - rng.unit() * 0.01)),
            c,
            v: r2(rng.unit() * 1e6),
        });
    }
    out
}

fn study_json(rng: &mut Rng) -> (&'static str, Value) {
    match rng.below(5) {
        0 => ("sma", json!({"period": (2 + rng.below(30)) as f64})),
        1 => ("ema", json!({"period": (2 + rng.below(30)) as f64})),
        2 => ("rsi", json!({"period": (2 + rng.below(20)) as f64})),
        3 => {
            let fast = 2 + rng.below(8);
            (
                "macd",
                json!({"fast": fast as f64, "slow": (fast + 2 + rng.below(12)) as f64,
                       "signal": (2 + rng.below(8)) as f64}),
            )
        }
        _ => (
            "bollinger",
            json!({"period": (2 + rng.below(25)) as f64, "k": 2.0}),
        ),
    }
}

/// One scenario: document (px inline or bound to THYAO, qx inline on another
/// instrument), studies, watchers.
struct Scenario {
    init: SessionInit,
    /// (id, study, input)
    studies: Vec<(String, String, String)>,
    bound: bool,
}

fn scenario(seed: u64, bound: bool) -> Scenario {
    let mut rng = Rng(seed);
    let inst = |s: &str| json!({"symbol": s});
    let px_src = if bound {
        json!({"id":"px","instrument":inst("THYAO"),"interval":"1d",
               "source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":240}}})
    } else {
        json!({"id":"px","instrument":inst("THYAO"),"interval":"1d","source":{"kind":"inline"},
               "data":{"kind":"bars","bars":walk(&mut rng, 240, 100.0)}})
    };
    let qx = json!({"id":"qx","instrument":inst("ASELS"),"interval":"1d","source":{"kind":"inline"},
                    "data":{"kind":"bars","bars":walk(&mut rng, 120, 50.0)}});
    let mut series = vec![px_src, qx];
    let n = 5 + rng.below(46);
    let mut studies = Vec::new();
    for i in 0..n {
        let input = if rng.below(4) == 0 { "qx" } else { "px" };
        let (study, params) = study_json(&mut rng);
        let id = format!("s{i}");
        series.push(json!({"id":id,"instrument":inst(if input == "px" {"THYAO"} else {"ASELS"}),
            "interval":"1d","source":{"kind":"study","study":study,"inputs":[input],"params":params}}));
        studies.push((id, study.to_owned(), input.to_owned()));
    }
    // Views: one on px and some of its studies, one on qx.
    let mut l1 = vec![json!({"mark":"line","series":"px","panel":0,"encoding":{}})];
    for (id, _, input) in studies.iter().take(6) {
        if input == "px" {
            l1.push(json!({"mark":"line","series":id,"panel":0,"encoding":{}}));
        }
    }
    let doc = json!({
        "schema_version":"0","id":"doc-live","title":"live","display_tz":"Europe/Istanbul",
        "series": series,
        "views": [
            {"id":"v1","x":{"kind":"time"},"y":[{"kind":"linear"}],"layers":l1},
            {"id":"v2","x":{"kind":"time"},"y":[{"kind":"linear"}],
             "layers":[{"mark":"line","series":"qx","panel":0,"encoding":{}}]}
        ],
        "facts":[],"findings":[],"capsules":[],"intents":[],"ledger":{"entries":[]}
    });
    let mut init = json!({
        "document": doc,
        "clock": T0,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"feed","author":{"kind":"engine","name":"feed","version":"0"},"role":"feed"},
            {"id":"viewer","author":{"kind":"human","name":"viewer"},"role":"viewer"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ]
    });
    if bound {
        init["data"] = json!({
            "feeds": [{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":260,"seed":seed % 97}}],
            "entitlements": {"host":["*"], "arkin":["THYAO"], "viewer":["THYAO"]}
        });
    }
    Scenario {
        init: serde_json::from_value(init).unwrap(),
        studies,
        bound,
    }
}

fn lines_json(lines: &[OutboundLine]) -> Vec<String> {
    lines
        .iter()
        .map(|l| serde_json::to_string(l).unwrap())
        .collect()
}

/// Two sessions fed the same inputs; every step's outputs compared.
struct Pair {
    inc: Session,
    full: Session,
    steps: usize,
    outputs: usize,
    /// Outbound messages by kind (`type`, or the event's `kind` for `notify`).
    kinds: BTreeMap<String, usize>,
}

impl Pair {
    fn new(init: &SessionInit) -> Self {
        let inc = Session::new(init.clone(), &root()).unwrap();
        let mut full = Session::new(init.clone(), &root()).unwrap();
        full.set_incremental(false);
        Self {
            inc,
            full,
            steps: 0,
            outputs: 0,
            kinds: BTreeMap::new(),
        }
    }

    fn count(&mut self, lines: &[String]) {
        self.outputs += lines.len();
        for l in lines {
            let v: Value = serde_json::from_str(l).unwrap();
            let m = &v["msg"];
            let k = if m["type"] == "notify" {
                m["event"]["kind"].as_str().unwrap_or("?").to_owned()
            } else {
                m["type"].as_str().unwrap_or("?").to_owned()
            };
            *self.kinds.entry(k).or_default() += 1;
        }
    }

    fn send(&mut self, conn: &str, msg: &Value) -> Vec<String> {
        let text = msg.to_string();
        let a = lines_json(&self.inc.handle_raw(conn, &text));
        let b = lines_json(&self.full.handle_raw(conn, &text));
        assert_eq!(a, b, "step {} ({conn}: {text})", self.steps);
        self.steps += 1;
        self.count(&a);
        a
    }

    fn data(&mut self, ev: &DataEvent) -> Vec<String> {
        let a = self.inc.data_event(ev).map(|l| lines_json(&l));
        let b = self.full.data_event(ev).map(|l| lines_json(&l));
        assert_eq!(a, b, "step {} (data {ev:?})", self.steps);
        self.steps += 1;
        let a = a.unwrap_or_default();
        self.count(&a);
        a
    }

    fn refresh(&mut self) {
        let a = lines_json(&self.inc.refresh_bindings());
        let b = lines_json(&self.full.refresh_bindings());
        assert_eq!(a, b, "step {} (refresh)", self.steps);
        self.steps += 1;
        self.count(&a);
    }
}

fn hello(p: &mut Pair, conn: &str, principal: &str) {
    let r = p.send(
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":"headless"}}),
    );
    assert!(r[0].contains("\"welcome\""), "{r:?}");
}

fn sub(p: &mut Pair, conn: &str, id: &str, topic: Value) {
    let r = p.send(conn, &json!({"type":"subscribe","id":id,"topic":topic}));
    assert!(r[0].contains("subscribed"), "{r:?}");
}

fn cmd(p: &mut Pair, conn: &str, id: &str, ops: Value) -> Vec<String> {
    p.send(conn, &json!({"type":"command","id":id,"ops":ops}))
}

/// A random condition on a study, px or qx (likely to cross now and then).
fn condition(rng: &mut Rng, sc: &Scenario) -> Value {
    let (id, study, _) = &sc.studies[rng.below(sc.studies.len())];
    let op = ["cross_above", "cross_below", "gt", "lt"][rng.below(4)];
    match study.as_str() {
        "rsi" => {
            json!({"kind":"threshold","series":id,"op":op,"value":(40 + rng.below(21)) as f64})
        }
        "macd" => json!({"kind":"threshold","series":id,"op":op,"value":0.0}),
        _ => {
            let input = &sc.studies.iter().find(|s| &s.0 == id).unwrap().2;
            json!({"kind":"compare","left":input,"op":op,"right":id})
        }
    }
}

/// Bars of `series` (inline from the document, bound from the reading document) through
/// the grammar provider of the incremental session.
fn bars_of(s: &mut Session, series: &str) -> Vec<Bar> {
    use qp_grammar::DataProvider;
    s.provider()
        .bars(series)
        .map(|b| b.to_vec())
        .unwrap_or_default()
}

fn revise(rng: &mut Rng, b: &Bar) -> Bar {
    let mut n = b.clone();
    n.c = r2((b.c * (1.0 + (rng.unit() - 0.5) * 0.02)).max(1.0));
    n.h = n.h.max(n.c);
    n.l = n.l.min(n.c);
    n.v = r2(n.v + rng.unit() * 1000.0);
    n
}

fn next_bar(rng: &mut Rng, last: &Bar) -> Bar {
    let o = last.c;
    let c = r2((o * (1.0 + (rng.unit() - 0.5) * 0.04)).max(1.0));
    Bar {
        t: last.t + DAY,
        o,
        h: o.max(c),
        l: o.min(c),
        c,
        v: r2(rng.unit() * 1000.0),
    }
}

fn run(seed: u64, bound: bool, steps: usize, kinds: &mut BTreeMap<String, usize>) -> usize {
    let sc = scenario(seed, bound);
    let mut rng = Rng(seed ^ 0xA5A5);
    let mut p = Pair::new(&sc.init);
    hello(&mut p, "a", "arkin");
    hello(&mut p, "f", "feed");
    hello(&mut p, "w", "viewer");
    hello(&mut p, "h", "host");
    sub(&mut p, "a", "ch", json!({"kind":"changes"}));
    sub(&mut p, "w", "ch-qx", json!({"kind":"changes","ids":["qx"]}));
    sub(&mut p, "w", "v1", json!({"kind":"view","view":"v1"}));
    sub(&mut p, "w", "v2", json!({"kind":"view","view":"v2"}));
    for k in 0..12 {
        let c = condition(&mut rng, &sc);
        let conn = if k % 2 == 0 { "a" } else { "w" };
        sub(
            &mut p,
            conn,
            &format!("c{k}"),
            json!({"kind":"condition","condition":c}),
        );
    }
    // Live capsules and intents on random studies.
    cmd(
        &mut p,
        "a",
        "facts",
        json!([{"op":"add_fact","id":"f1","kind":"note","series":"px","bar":10,"text":"x"},
               {"op":"add_fact","id":"f2","kind":"note","series":"qx","bar":10,"text":"y"}]),
    );
    for k in 0..6 {
        let c = condition(&mut rng, &sc);
        let fact = if k % 2 == 0 { "f1" } else { "f2" };
        let r = cmd(
            &mut p,
            "a",
            &format!("fnd{k}"),
            json!([{"op":"add_finding","id":format!("fnd{k}"),"claim":"c","facts":[fact],
                    "confidence":0.5,"invalidation":c,"capsule":"h"}]),
        );
        assert!(r[0].contains("applied"), "{r:?}");
        sub(
            &mut p,
            "w",
            &format!("fs{k}"),
            json!({"kind":"finding","finding":format!("fnd{k}")}),
        );
        let t = condition(&mut rng, &sc);
        cmd(
            &mut p,
            "a",
            &format!("int{k}"),
            json!([{"op":"add_intent","text":"t","trigger":t}]),
        );
    }
    sub(&mut p, "a", "in", json!({"kind":"intents"}));
    // RP2/T8: the claim lifecycle (evidence, finalisation, withdrawal, cleanliness) is part
    // of the compared output.
    sub(&mut p, "a", "cl", json!({"kind":"claims"}));
    sub(&mut p, "w", "cl", json!({"kind":"claims"}));
    let feed = json!({"symbol":"THYAO","interval":"1d"});
    let mut dseq = 0u64;
    for i in 0..steps {
        let r = rng.below(100);
        let id = format!("t{i}");
        // Which bar series ticks: px (inline command or bound event) or qx (inline).
        let target = if r < 80 { "px" } else { "qx" };
        let bars = bars_of(&mut p.inc, target);
        let Some(last) = bars.last().cloned() else {
            // px stale after a gap: re-resolve.
            p.refresh();
            continue;
        };
        if target == "px" && sc.bound {
            let k = rng.below(100);
            let change = if k < 55 {
                let b = revise(&mut rng, &last);
                json!({"kind":"patch","t":b.t,"fields":{"c":b.c,"h":b.h,"l":b.l,"v":b.v}})
            } else if k < 60 {
                json!({"kind":"patch","t":last.t,"fields":{"v":r2(last.v + 1.0)}})
            } else if k < 80 {
                json!({"kind":"bar","bar":next_bar(&mut rng, &last)})
            } else if k < 92 && bars.len() > 2 {
                // Correction of an earlier bar.
                let j = bars.len() - 2 - rng.below(bars.len().min(40) - 1);
                let b = &bars[j];
                let c = r2(b.l + (b.h - b.l) * rng.unit());
                json!({"kind":"patch","t":b.t,"fields":{"c":c}})
            } else if k < 97 {
                // Duplicate: an old seq, ignored.
                let ev: DataEvent = serde_json::from_value(json!({"feed":feed,"seq":dseq.max(1),
                    "change":{"kind":"patch","t":last.t,"fields":{"c":last.c}}}))
                .unwrap();
                p.data(&ev);
                continue;
            } else if i + 30 > steps && i % 3 == 0 {
                // A gap (stale), then the host re-resolves.
                let ev: DataEvent = serde_json::from_value(json!({"feed":feed,"seq":dseq + 2,
                    "change":{"kind":"patch","t":last.t,"fields":{"c":last.c}}}))
                .unwrap();
                p.data(&ev);
                p.refresh();
                continue;
            } else {
                // A study query between ticks.
                let (s, _, _) = &sc.studies[rng.below(sc.studies.len())];
                p.send(
                    "a",
                    &json!({"type":"query","id":id,"query":{"kind":"study","series":s,"range":{"last":3}}}),
                );
                continue;
            };
            dseq += 1;
            let ev: DataEvent =
                serde_json::from_value(json!({"feed":feed,"seq":dseq,"change":change})).unwrap();
            p.data(&ev);
            continue;
        }
        let k = rng.below(100);
        let conn = if rng.below(4) == 0 { "a" } else { "f" };
        if k < 55 {
            let b = revise(&mut rng, &last);
            cmd(
                &mut p,
                conn,
                &id,
                json!([{"op":"update_last_bar","series":target,"bar":b}]),
            );
        } else if k < 75 {
            let b = next_bar(&mut rng, &last);
            cmd(
                &mut p,
                conn,
                &id,
                json!([{"op":"append_bars","series":target,"bars":[b]}]),
            );
        } else if k < 80 {
            let b1 = next_bar(&mut rng, &last);
            let b2 = next_bar(&mut rng, &b1);
            cmd(
                &mut p,
                conn,
                &id,
                json!([{"op":"append_bars","series":target,"bars":[b1,b2]}]),
            );
        } else if k < 85 {
            // Two ops: new bar, then its revision (one entry, two bar changes).
            let b1 = next_bar(&mut rng, &last);
            let b2 = revise(&mut rng, &b1);
            cmd(
                &mut p,
                conn,
                &id,
                json!([{"op":"append_bars","series":target,"bars":[b1]},
                       {"op":"update_last_bar","series":target,"bar":b2}]),
            );
        } else if k < 88 {
            // A failing second op rolls the first back.
            let b1 = next_bar(&mut rng, &last);
            cmd(
                &mut p,
                conn,
                &id,
                json!([{"op":"append_bars","series":target,"bars":[b1.clone()]},
                       {"op":"append_bars","series":target,"bars":[b1]}]),
            );
        } else if k < 93 {
            cmd(&mut p, "a", &id, json!([{"op":"undo"}]));
        } else if k < 96 {
            // Structure change: a study's parameters.
            let (s, study, _) = &sc.studies[rng.below(sc.studies.len())];
            let params = match study.as_str() {
                "macd" => json!({"fast":3.0,"slow":9.0,"signal":4.0}),
                "bollinger" => json!({"period":(3 + rng.below(9)) as f64,"k":2.0}),
                _ => json!({"period":(2 + rng.below(9)) as f64}),
            };
            cmd(
                &mut p,
                "a",
                &id,
                json!([{"op":"set_params","series":s,"params":params}]),
            );
        } else {
            let (s, _, _) = &sc.studies[rng.below(sc.studies.len())];
            p.send(
                "a",
                &json!({"type":"query","id":id,"query":{"kind":"study","series":s,"range":{"last":3}}}),
            );
        }
    }
    check_end(&mut p, &sc);
    eprintln!(
        "{} seed {seed:#x}: {} studies, {} steps, {} outputs",
        if bound { "bound" } else { "inline" },
        sc.studies.len(),
        p.steps,
        p.outputs
    );
    for (k, n) in &p.kinds {
        *kinds.entry(k.clone()).or_default() += n;
    }
    sc.studies.len()
}

fn check_end(p: &mut Pair, sc: &Scenario) {
    let doc_a = serde_json::to_string(p.inc.document()).unwrap();
    let doc_b = serde_json::to_string(p.full.document()).unwrap();
    assert_eq!(doc_a, doc_b, "document bytes");
    let log_a = serde_json::to_string(p.inc.log()).unwrap();
    let log_b = serde_json::to_string(p.full.log()).unwrap();
    assert_eq!(log_a, log_b, "log bytes");
    let replayed = replay(p.inc.initial_document(), p.inc.log()).unwrap();
    assert_eq!(serde_json::to_string(&replayed).unwrap(), doc_a, "replay");
    // Study values: incremental cache vs reference vs batch, bit for bit.
    use qp_grammar::DataProvider;
    let doc = p.inc.document().clone();
    for s in &doc.series {
        let SeriesSource::Study {
            study,
            params,
            inputs,
            ..
        } = &s.source
        else {
            continue;
        };
        let bars = bars_of(&mut p.inc, &inputs[0]);
        let batch = qp_compute::compute_bars(study, params, qp_compute::Field::Close, &bars);
        let pa = p.inc.provider();
        let fields: Vec<String> = pa.plot_fields(&s.id);
        let mut va: BTreeMap<String, Vec<Option<u64>>> = BTreeMap::new();
        for f in batch
            .as_ref()
            .map(|b| b.outputs.keys().cloned().collect::<Vec<_>>())
            .unwrap_or(fields)
        {
            if let Some(v) = pa.values(&s.id, &f) {
                va.insert(f, v.iter().map(|x| x.map(f64::to_bits)).collect());
            }
        }
        let pb = p.full.provider();
        let mut vb: BTreeMap<String, Vec<Option<u64>>> = BTreeMap::new();
        for f in va.keys() {
            if let Some(v) = pb.values(&s.id, f) {
                vb.insert(f.clone(), v.iter().map(|x| x.map(f64::to_bits)).collect());
            }
        }
        assert_eq!(va, vb, "values of {}", s.id);
        if let Ok(b) = batch {
            let bits: BTreeMap<String, Vec<Option<u64>>> = b
                .outputs
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().map(|x| x.map(f64::to_bits)).collect()))
                .collect();
            assert_eq!(va, bits, "values of {} vs compute_bars", s.id);
        }
    }
    let _ = sc.bound;
}

fn property(bound: bool, seeds: std::ops::Range<u64>) {
    let mut kinds = BTreeMap::new();
    let mut sizes = Vec::new();
    for seed in seeds {
        sizes.push(run(0x5EED_0000 + seed, bound, 600, &mut kinds));
    }
    eprintln!("study series per scenario: {sizes:?}; outputs by kind: {kinds:?}");
    // The sequences reach every kind of notification the live path produces.
    for k in [
        "ack",
        "error",
        "change",
        "condition_met",
        "intent_triggered",
        "finding_state",
        "view_changed",
        "result",
    ] {
        assert!(
            kinds.get(k).copied().unwrap_or(0) > 0,
            "no `{k}` in {kinds:?}"
        );
    }
    assert!(sizes.iter().any(|&n| n >= 30) && sizes.iter().any(|&n| n <= 15));
}

#[test]
fn incremental_equals_from_scratch_inline() {
    property(false, 1..7);
}

#[test]
fn incremental_equals_from_scratch_bound() {
    property(true, 11..17);
}

/// Two bound series on two feeds (THYAO, ASELS), each with a study and its own view, an
/// inline series with a third view; condition subscriptions and a live capsule on each.
fn two_feeds() -> Session {
    let inst = |s: &str| json!({"symbol": s});
    let bind = |id: &str, sym: &str| {
        json!({"id":id,"instrument":inst(sym),"interval":"1d",
               "source":{"kind":"bind","symbol":sym,"interval":"1d","window":{"last":100}}})
    };
    let study = |id: &str, input: &str, sym: &str, st: &str, p: Value| {
        json!({"id":id,"instrument":inst(sym),"interval":"1d",
               "source":{"kind":"study","study":st,"inputs":[input],"params":p}})
    };
    let mut rng = Rng(9);
    let view = |id: &str, series: &[&str]| {
        json!({"id":id,"x":{"kind":"time"},"y":[{"kind":"linear"}],
               "layers": series.iter().map(|s| json!({"mark":"line","series":s,"panel":0,"encoding":{}})).collect::<Vec<_>>()})
    };
    let doc = json!({
        "schema_version":"0","id":"doc-two","title":"two feeds","display_tz":"Europe/Istanbul",
        "series":[
            bind("px","THYAO"), study("rsi","px","THYAO","rsi",json!({"period":14.0})),
            bind("asl","ASELS"), study("asma","asl","ASELS","sma",json!({"period":5.0})),
            {"id":"idx","instrument":inst("XU100"),"interval":"1d","source":{"kind":"inline"},
             "data":{"kind":"bars","bars":walk(&mut rng, 50, 10.0)}}
        ],
        "views":[view("va",&["px","rsi"]), view("vb",&["asl","asma"]), view("vi",&["idx"])],
        "facts":[],"findings":[],"capsules":[],"intents":[],"ledger":{"entries":[]}
    });
    let init: SessionInit = serde_json::from_value(json!({
        "document": doc, "clock": T0,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"viewer","author":{"kind":"human","name":"viewer"},"role":"viewer"},
            {"id":"feed","author":{"kind":"engine","name":"feed","version":"0"},"role":"feed"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ],
        "data": {"feeds":[
            {"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":120,"seed":3}},
            {"feed":{"symbol":"ASELS","interval":"1d"},"synthetic":{"n":120,"seed":4}}],
            "entitlements":{"host":["*"],"arkin":["*"],"viewer":["*"]}}
    }))
    .unwrap();
    Session::new(init, &root()).unwrap()
}

fn msgs(lines: &[OutboundLine]) -> Vec<Value> {
    lines
        .iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

#[test]
fn a_tick_wakes_only_affected_views_and_watchers() {
    let mut s = two_feeds();
    let send = |s: &mut Session, conn: &str, m: Value| msgs(&s.handle_raw(conn, &m.to_string()));
    for (c, p) in [("a", "arkin"), ("b", "viewer"), ("f", "feed")] {
        send(
            &mut s,
            c,
            json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
        );
    }
    // Conn a watches feed 1 (THYAO), conn b feed 2 (ASELS) and the inline view.
    for (c, id, topic) in [
        ("a", "va", json!({"kind":"view","view":"va"})),
        (
            "a",
            "ca",
            json!({"kind":"condition","condition":{"kind":"threshold","series":"rsi","op":"gt","value":0.0}}),
        ),
        ("b", "vb", json!({"kind":"view","view":"vb"})),
        ("b", "vi", json!({"kind":"view","view":"vi"})),
        (
            "b",
            "cb",
            json!({"kind":"condition","condition":{"kind":"threshold","series":"asma","op":"cross_above","value":-1.0}}),
        ),
        (
            "b",
            "cb2",
            json!({"kind":"condition","condition":{"kind":"compare","left":"asl","op":"cross_below","right":"asma"}}),
        ),
        (
            "b",
            "chb",
            json!({"kind":"changes","ids":["asl","asma","vb"]}),
        ),
    ] {
        let r = send(&mut s, c, json!({"type":"subscribe","id":id,"topic":topic}));
        assert_eq!(r[0]["msg"]["status"], "subscribed", "{r:?}");
    }
    let last = |s: &mut Session, id: &str| bars_of(s, id).last().cloned().unwrap();
    let feed1 = json!({"symbol":"THYAO","interval":"1d"});
    let mut rng = Rng(77);
    let mut seen = 0;
    for seq in 1..=200u64 {
        let b = last(&mut s, "px");
        let change = match seq % 10 {
            0 => json!({"kind":"bar","bar":next_bar(&mut rng, &b)}),
            5 => {
                let bars = bars_of(&mut s, "px");
                let e = &bars[bars.len() - 5];
                json!({"kind":"patch","t":e.t,"fields":{"c":r2((e.l + e.h) / 2.0)}})
            }
            _ => {
                let r = revise(&mut rng, &b);
                json!({"kind":"patch","t":r.t,"fields":{"c":r.c,"h":r.h,"l":r.l}})
            }
        };
        let ev: DataEvent =
            serde_json::from_value(json!({"feed":feed1,"seq":seq,"change":change})).unwrap();
        let out = msgs(&s.data_event(&ev).unwrap());
        // Nothing for conn b (view vb, view vi, conditions on ASELS, changes on ASELS).
        assert!(out.iter().all(|m| m["conn"] == "a"), "{out:?}");
        // View va is told every time, from the right bar.
        let vc: Vec<&Value> = out
            .iter()
            .filter(|m| m["msg"]["event"]["kind"] == "view_changed")
            .collect();
        // One event per changed bar series (px; rsi rests on it).
        assert_eq!(vc.len(), 1, "{out:?}");
        let n = bars_of(&mut s, "px").len() as u64;
        let e = &vc[0]["msg"]["event"];
        assert_eq!(
            (e["view"].as_str(), e["series"].as_str()),
            (Some("va"), Some("px"))
        );
        assert_eq!(e["to_bar"], n - 1);
        let from = if seq % 10 == 5 { n - 5 } else { n - 1 };
        assert_eq!(e["from_bar"], from, "{out:?}");
        seen += 1;
    }
    assert_eq!(seen, 200);
    // An inline bar command on idx wakes only vi (conn b) besides the ack.
    let b = last(&mut s, "idx");
    let out = send(
        &mut s,
        "f",
        json!({"type":"command","id":"i1","ops":[{"op":"append_bars","series":"idx","bars":[next_bar(&mut rng, &b)]}]}),
    );
    assert_eq!(out.len(), 2, "{out:?}");
    assert_eq!(out[0]["msg"]["type"], "ack");
    assert_eq!(out[1]["conn"], "b");
    assert_eq!(out[1]["msg"]["sub"], "vi");
    assert_eq!(out[1]["msg"]["event"]["from_bar"], 50);
}
