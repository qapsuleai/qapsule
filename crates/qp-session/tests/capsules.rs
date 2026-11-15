//! The capsule engine in the authority (R3, `docs/PROTOCOL.md` §20): replay and
//! determinism (GOAL 4), the entitlement sweep (GOAL 9, VERIFY 10), candidates leave no
//! trace (GOAL 2, VERIFY 9), K26 in both languages, sensitivity is never changed by the
//! system, and a person's preferences reach nobody else.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::Bar;
use qp_session::{replay, Session};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bars() -> Vec<Bar> {
    qp_source::synthetic_market(1, 160, 1)[0].bars.clone()
}

/// The init of transcript `name` (38–43 share the construction).
fn init_of(name: &str) -> Value {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join(format!("fixtures/protocol/conformance/{name}.json")))
            .unwrap(),
    )
    .unwrap();
    t["init"].clone()
}

/// The 38/39 init with the feed's first `k` bars.
fn init_k(k: usize) -> SessionInit {
    let mut v = init_of("38-capsule-lifecycle");
    v["data"]["feeds"][0]["bars"] = json!(bars()[..k]);
    serde_json::from_value(v).unwrap()
}

fn ev(seq: usize, b: &Bar) -> DataEvent {
    serde_json::from_value(
        json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},
        "seq": seq, "change": {"kind":"bar","bar": b}}),
    )
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn hello(s: &mut Session, conn: &str, who: &str, lang: Option<&str>) {
    let mut surface = json!({"kind":"headless"});
    if let Some(l) = lang {
        surface["lang"] = json!(l);
    }
    let w = send(
        s,
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":who,"surface":surface}),
    );
    assert_eq!(serde_json::to_value(&w[0].msg).unwrap()["type"], "welcome");
}

fn q(s: &mut Session, conn: &str, query: Value) -> Value {
    let r = send(s, conn, json!({"type":"query","id":"q","query":query}));
    serde_json::to_value(&r[0].msg).unwrap()
}

/// Feeds bars `from..to` (the feed holds `k` initial bars); returns every line.
fn feed(s: &mut Session, k: usize, from: usize, to: usize) -> Vec<OutboundLine> {
    let b = bars();
    let mut out = Vec::new();
    for (i, bar) in b.iter().enumerate().take(to).skip(from) {
        out.extend(s.data_event(&ev(i - k + 1, bar)).unwrap());
    }
    out
}

fn texts(lines: &[OutboundLine], conn: &str) -> Vec<String> {
    lines
        .iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_string(&l.msg).unwrap())
        .collect()
}

/// GOAL 4: the same inputs give the same log byte for byte (twice, and on the
/// from-scratch reference path); replaying the log gives the same document, so the same
/// capsule history.
#[test]
fn replaying_the_log_reproduces_the_capsule_history() {
    let run = |incremental: bool| {
        let mut s = Session::new(init_k(60), &root()).unwrap();
        s.set_incremental(incremental);
        hello(&mut s, "a", "ana", None);
        send(
            &mut s,
            "a",
            json!({"type":"subscribe","id":"k","topic":{"kind":"capsules","all":true}}),
        );
        let lines = feed(&mut s, 60, 60, 160);
        (
            serde_json::to_string(s.log()).unwrap(),
            texts(&lines, "a"),
            s.document().clone(),
            s.initial_document().clone(),
        )
    };
    let (log1, out1, doc1, init1) = run(true);
    let (log2, out2, _, _) = run(true);
    let (log3, out3, _, _) = run(false);
    assert_eq!(log1, log2);
    assert_eq!(out1, out2);
    assert_eq!(log1, log3, "incremental = from scratch");
    assert_eq!(out1, out3);
    let log: Vec<qp_schema::protocol::LogEntry> = serde_json::from_str(&log1).unwrap();
    let again = replay(&init1, &log).unwrap();
    assert_eq!(
        serde_json::to_vec(&again).unwrap(),
        serde_json::to_vec(&doc1).unwrap()
    );
    let stories = doc1.capsules.iter().filter(|c| c.story.is_some()).count();
    assert!(stories > 20, "{stories} capsules");
    // Every closed story but the retracted ones has its ledger row; every row is a close.
    for e in &doc1.ledger.entries {
        let c = doc1
            .capsules
            .iter()
            .find(|c| Some(&c.id) == e.capsule.as_ref())
            .unwrap();
        assert!(c.story.as_ref().unwrap().stage.is_closed());
    }
}

/// Every non-integer evidence value of the facts in `ids`, as JSON writes it.
fn values_of(doc: &qp_schema::Document, ids: &BTreeSet<String>) -> BTreeSet<String> {
    doc.facts
        .iter()
        .filter(|f| ids.contains(&f.id))
        .flat_map(|f| f.evidence.values())
        .filter(|v| v.fract() != 0.0)
        .map(|v| serde_json::to_string(v).unwrap())
        .filter(|s| s.len() >= 4)
        .collect()
}

/// GOAL 9 / VERIFY 10 and the M6 integration rule: over every query, topic and present
/// path, a reader without the flow entitlement receives no licensed fact, story id,
/// value or digest, and a reader without any data entitlement receives no evidence value,
/// no data digest and no digit in a capsule sentence. Both get the ledger.
#[test]
#[allow(clippy::too_many_lines)]
fn entitlement_sweep_no_licensed_value_reaches_an_unentitled_reader() {
    let mut s = Session::new(init_k(60), &root()).unwrap();
    hello(&mut s, "a", "ana", None);
    hello(&mut s, "b", "bora", None);
    hello(&mut s, "c", "can", Some("en"));
    let topics = [
        json!({"kind":"changes"}),
        json!({"kind":"capsules","all":true}),
        json!({"kind":"capsules"}),
        json!({"kind":"claims"}),
        json!({"kind":"intents"}),
        json!({"kind":"proposals"}),
        json!({"kind":"view","view":"none"}),
    ];
    let mut out: Vec<OutboundLine> = Vec::new();
    for c in ["b", "c"] {
        for (i, t) in topics.iter().enumerate() {
            out.extend(send(
                &mut s,
                c,
                json!({"type":"subscribe","id":format!("s{i}"),"topic":t}),
            ));
        }
    }
    out.extend(feed(&mut s, 60, 60, 160));
    // Ana (entitled) shows a licensed story's chart (S-B §3: a stored view).
    let doc = s.document().clone();
    let licensed: BTreeSet<String> = doc
        .facts
        .iter()
        .filter(|f| {
            f.meaning
                .as_ref()
                .is_some_and(|m| m.license.class == qp_schema::capsule::LicenseClass::Licensed)
        })
        .map(|f| f.id.clone())
        .collect();
    assert!(!licensed.is_empty(), "the run has licensed facts");
    let lic_caps: Vec<&qp_schema::Capsule> = doc
        .capsules
        .iter()
        .filter(|c| {
            c.story
                .as_ref()
                .is_some_and(|st| st.history[0].facts.iter().any(|f| licensed.contains(f)))
        })
        .collect();
    assert!(!lic_caps.is_empty());
    let f0 = lic_caps[0].finding.clone();
    let r = send(
        &mut s,
        "a",
        json!({"type":"command","id":"sf","ops":[{"op":"show_finding","finding":f0,"view":"lic"}]}),
    );
    out.extend(r);
    let doc = s.document().clone();
    let all_ids: Vec<String> = doc
        .facts
        .iter()
        .map(|f| f.id.clone())
        .chain(doc.findings.iter().map(|f| f.id.clone()))
        .chain(doc.capsules.iter().map(|c| c.id.clone()))
        .chain(doc.views.iter().map(|v| v.id.clone()))
        .collect();
    for c in ["b", "c"] {
        for qv in [
            json!({"kind":"describe"}),
            json!({"kind":"document"}),
            json!({"kind":"log"}),
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","audience":true}),
            json!({"kind":"ledger"}),
            json!({"kind":"ledger","by_kind":true}),
            json!({"kind":"binding"}),
            json!({"kind":"screen"}),
        ] {
            out.extend(send(&mut s, c, json!({"type":"query","id":"q","query":qv})));
        }
        for coll in [
            "series", "views", "facts", "findings", "capsules", "intents",
        ] {
            out.extend(send(
                &mut s,
                c,
                json!({"type":"query","id":"l","query":{"kind":"list","collection":coll,"full":true}}),
            ));
        }
        for id in &all_ids {
            for qv in [
                json!({"kind":"get","id":id}),
                json!({"kind":"capsule","capsule":id}),
                json!({"kind":"present","finding":id}),
                json!({"kind":"present","finding":id,"surface":"phone"}),
                json!({"kind":"verify_evidence","item":id}),
            ] {
                out.extend(send(&mut s, c, json!({"type":"query","id":"g","query":qv})));
            }
        }
        for sel in [json!({"latest":true}), json!({"kind":"flow"})] {
            out.extend(send(
                &mut s,
                c,
                json!({"type":"query","id":"sel","query":{"kind":"present","finding":sel}}),
            ));
        }
        out.extend(send(
            &mut s,
            c,
            json!({"type":"command","id":"x","ops":[{"op":"approve_finding","finding":f0}]}),
        ));
    }
    // Bora: no licensed fact or licensed-born story, by id, value or digest.
    let lic_values = values_of(&doc, &licensed);
    assert!(lic_values.len() > 5, "{lic_values:?}");
    // (The licensed facts' `evidence_ref` digests are of the bist bars Bora may read.)
    let mut forbidden_b: Vec<String> = licensed.iter().cloned().collect();
    forbidden_b.extend(lic_caps.iter().map(|c| c.id.clone()));
    forbidden_b.extend(lic_caps.iter().map(|c| c.finding.clone()));
    let b_out = texts(&out, "b");
    assert!(b_out.len() > 100);
    for line in &b_out {
        for id in &forbidden_b {
            assert!(
                !line.contains(&format!("\"{id}\"")),
                "bora got {id}: {line:.300}"
            );
        }
        for v in &lic_values {
            assert!(
                !line.contains(v.as_str()),
                "bora got licensed value {v}: {line:.300}"
            );
        }
    }
    // Bora gets the server comments (trust: server) and the ledger.
    assert!(b_out.iter().any(|l| l.contains("\"server_comment\"")));
    assert!(b_out.iter().any(|l| l.contains("\"kind\":\"ledger\"")));
    // Can: also no evidence value and no data digest of any R3 fact, no digit in a
    // capsule sentence.
    let r3: BTreeSet<String> = doc
        .facts
        .iter()
        .filter(|f| f.meaning.is_some())
        .map(|f| f.id.clone())
        .collect();
    let values = values_of(&doc, &r3);
    let digests: BTreeSet<String> = doc
        .facts
        .iter()
        .filter_map(|f| f.evidence_ref.as_ref().map(|r| r.digest.clone()))
        .collect();
    let c_out = texts(&out, "c");
    for line in &c_out {
        for id in &forbidden_b {
            assert!(!line.contains(&format!("\"{id}\"")), "can got {id}");
        }
        for v in &values {
            assert!(!line.contains(v.as_str()), "can got value {v}: {line:.300}");
        }
        for d in &digests {
            assert!(!line.contains(d.as_str()), "can got digest {d}");
        }
        let v: Value = serde_json::from_str(line).unwrap();
        let mut items: Vec<&Value> = Vec::new();
        if let Some(a) = v.pointer("/data/capsules").and_then(Value::as_array) {
            items.extend(a);
        }
        if let Some(i) = v.pointer("/data/detail/item") {
            items.push(i);
        }
        if let Some(i) = v.pointer("/event/capsule") {
            items.push(i);
        }
        for it in items {
            for k in ["text", "tr", "en"] {
                let t = it["sentence"][k].as_str().unwrap_or_default();
                assert!(!t.chars().any(|c| c.is_ascii_digit()), "digit for can: {t}");
            }
        }
        if let Some(ev) = v.pointer("/data/detail/evidence").and_then(Value::as_array) {
            for e in ev {
                assert_eq!(e["open"], false);
                assert!(e["requires"].is_string() && e["rests_on"].is_string());
            }
        }
    }
    assert!(c_out
        .iter()
        .any(|l| l.contains("\"requires\":\"bist/THYAO\"")));
    // Ana, entitled, sees the licensed story with its numbers.
    let a = q(
        &mut s,
        "a",
        json!({"kind":"capsule","capsule":lic_caps[0].id}),
    );
    assert_eq!(a["data"]["detail"]["evidence"][0]["open"], true);
    assert!(a["data"]["detail"]["item"]["sentence"]["text"]
        .as_str()
        .unwrap()
        .chars()
        .any(|c| c.is_ascii_digit()));
}

/// GOAL 2 / VERIFY 9: a breakout that forms inside the live bar is a candidate on the
/// reader's own data only; when it disappears before the close, the authority's log and
/// ledger hold no trace of it.
#[test]
fn a_candidate_that_disappears_leaves_no_trace() {
    let b = bars();
    let k = 143;
    let mut s = Session::new(init_k(k), &root()).unwrap();
    let log0 = s.log().len();
    s.data_event(&ev(1, &b[143])).unwrap();
    let mut live: Vec<Bar> = b[..=143].to_vec();
    let c = qp_session::capsule::candidates("THYAO", "px", &live);
    assert!(
        c.iter().any(|x| x.capsule == "cap.THYAO.breakout.up.143"),
        "{:?}",
        c.iter().map(|x| &x.capsule).collect::<Vec<_>>()
    );
    assert!(c
        .iter()
        .all(|x| x.story.stage == qp_schema::capsule::Stage::Candidate));
    let level = b[123..143].iter().map(|x| x.h).fold(f64::MIN, f64::max);
    let low = (level - 0.05).min(b[143].c);
    let patch: DataEvent = serde_json::from_value(
        json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},
        "seq": 2, "change": {"kind":"patch","t": b[143].t, "fields": {"c": low}}}),
    )
    .unwrap();
    assert!(s.data_event(&patch).unwrap().is_empty());
    live[143].c = low;
    let c = qp_session::capsule::candidates("THYAO", "px", &live);
    assert!(c.iter().all(|x| x.capsule != "cap.THYAO.breakout.up.143"));
    s.data_event(&ev(3, &b[144])).unwrap();
    assert_eq!(s.log().len(), log0 + 2, "one engine entry per close");
    let log = serde_json::to_string(s.log()).unwrap();
    assert!(!log.contains("breakout.up.143"));
    assert!(!log.contains("\"candidate\""));
    assert!(s
        .document()
        .ledger
        .entries
        .iter()
        .all(|e| !e.finding.contains("breakout.up.143")));
}

/// K26 in both languages: an ended state is told in the past with "artık … değil" /
/// "no longer".
#[test]
fn k26_ended_state_past_tense_tr_and_en() {
    let mut s = Session::new(init_k(92), &root()).unwrap();
    hello(&mut s, "t", "ana", None);
    hello(&mut s, "e", "ana", Some("en"));
    feed(&mut s, 92, 92, 96);
    let tr = q(
        &mut s,
        "t",
        json!({"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}),
    );
    let en = q(
        &mut s,
        "e",
        json!({"kind":"capsule","capsule":"cap.THYAO.continuation.down.84"}),
    );
    let t = tr["data"]["detail"]["item"]["sentence"]["text"]
        .as_str()
        .unwrap();
    let e = en["data"]["detail"]["item"]["sentence"]["text"]
        .as_str()
        .unwrap();
    assert!(
        t.contains("önce") && t.contains("artık") && t.contains("değil"),
        "{t}"
    );
    assert!(e.contains("ago") && e.contains("no longer"), "{e}");
    assert_eq!(
        tr["data"]["detail"]["item"]["sentence"]["en"],
        en["data"]["detail"]["item"]["sentence"]["en"]
    );
}

/// K31: the system never changes a person's preferences, however many bars and stories.
#[test]
fn the_engine_never_changes_the_audience() {
    let mut s = Session::new(init_k(60), &root()).unwrap();
    hello(&mut s, "a", "ana", None);
    let r = send(
        &mut s,
        "a",
        json!({"type":"command","id":"s","ops":[{"op":"set_audience","audience":{"group":"future","watchlist":["THYAO"]}}]}),
    );
    assert_eq!(
        serde_json::to_value(&r[0].msg).unwrap()["status"],
        "applied"
    );
    let before = s.document().audiences.clone();
    let n = s.log().len();
    feed(&mut s, 60, 60, 160);
    assert!(s.log().len() > n + 90);
    assert_eq!(s.document().audiences, before);
    for e in &s.log()[n..] {
        assert!(e
            .changes
            .iter()
            .all(|c| !matches!(c, qp_schema::protocol::ChangeOp::PutAudience { .. })));
    }
}

/// A person's preferences (positions, watch list, level) reach nobody else: not through
/// `document`, `change`, `log`, `list`/`get` or any topic. The host receives them all.
#[test]
fn a_persons_audience_reaches_nobody_else() {
    let mut s = Session::new(init_k(60), &root()).unwrap();
    hello(&mut s, "a", "ana", None);
    hello(&mut s, "b", "bora", None);
    hello(&mut s, "o", "arkin", None);
    hello(&mut s, "h", "host", None);
    let mut out = Vec::new();
    for c in ["b", "o"] {
        for (i, t) in [
            json!({"kind":"changes"}),
            json!({"kind":"capsules","all":true}),
            json!({"kind":"proposals"}),
        ]
        .iter()
        .enumerate()
        {
            out.extend(send(
                &mut s,
                c,
                json!({"type":"subscribe","id":format!("s{i}"),"topic":t}),
            ));
        }
    }
    out.extend(send(
        &mut s,
        "a",
        json!({"type":"command","id":"s1","ops":[{"op":"set_audience","audience":{"group":"future","watchlist":["SECRETWATCH"],"positions":{"SECRETPOS":123456.5}}}]}),
    ));
    out.extend(send(
        &mut s,
        "a",
        json!({"type":"command","id":"s2","ops":[{"op":"set_sensitivity","sensitivity":"balanced"}]}),
    ));
    out.extend(send(
        &mut s,
        "o",
        json!({"type":"command","id":"s3","ops":[{"op":"set_audience","principal":"ana","audience":{"group":"future","positions":{"SECRETPOS":654321.5}}}]}),
    ));
    // The proposal path is closed: preferences are never carried in a proposal.
    let r = send(
        &mut s,
        "a",
        json!({"type":"command","id":"s4","mode":"propose","ops":[{"op":"set_audience","audience":{"group":"future","positions":{"SECRETPOS":111.5}}}]}),
    );
    let r = serde_json::to_value(&r[0].msg).unwrap();
    assert_eq!(r["error"]["code"], "forbidden", "{r}");
    out.extend(send(
        &mut s,
        "b",
        json!({"type":"command","id":"s5","mode":"propose","ops":[{"op":"set_sensitivity","principal":"ana","sensitivity":"all"}]}),
    ));
    assert!(s.document().proposals.is_empty());
    out.extend(feed(&mut s, 60, 60, 70));
    for c in ["b", "o"] {
        for qv in [
            json!({"kind":"document"}),
            json!({"kind":"log"}),
            json!({"kind":"describe"}),
            json!({"kind":"capsules","audience":true}),
        ] {
            out.extend(send(&mut s, c, json!({"type":"query","id":"q","query":qv})));
        }
        for coll in [
            "series", "views", "facts", "findings", "capsules", "intents",
        ] {
            out.extend(send(
                &mut s,
                c,
                json!({"type":"query","id":"l","query":{"kind":"list","collection":coll}}),
            ));
        }
        out.extend(send(
            &mut s,
            c,
            json!({"type":"query","id":"g","query":{"kind":"get","id":"ana"}}),
        ));
    }
    for c in ["b", "o"] {
        for line in texts(&out, c) {
            for secret in [
                "SECRETWATCH",
                "SECRETPOS",
                "123456.5",
                "654321.5",
                "111.5",
                "balanced",
            ] {
                assert!(!line.contains(secret), "{c} got {secret}: {line:.300}");
            }
        }
    }
    // Ana sees her own; the host sees everyone's.
    let own = q(&mut s, "a", json!({"kind":"document"}));
    assert_eq!(
        own["data"]["document"]["audiences"]["ana"]["positions"]["SECRETPOS"],
        654_321.5
    );
    let host = q(&mut s, "h", json!({"kind":"document"}));
    assert!(host["data"]["document"]["audiences"]["ana"].is_object());
}
