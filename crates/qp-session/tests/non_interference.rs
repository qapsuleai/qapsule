//! R3/K1 (audit Y1, O1): non-interference of the licensed flow.
//!
//! Same public bars, different licensed flows (seeds of the synthetic flow): everything a
//! reader without the flow entitlement receives — every topic (`capsules`, `capsules
//! {all}`, `capsules {instrument}`, `changes`, `claims`) and the final queries the
//! auditor's probe asked (`rounds/R3/audit/src/bin/session_probe.rs`) — is byte-identical
//! once the server comments (mühür messages: `trust: server`, `server_comment`) are taken
//! out. Those are the only channel the licensed data may influence for them (M6
//! integration rule, Ö37). Positive control: the fully entitled reader's lines do differ.

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Transcript 39's construction (ana `*`, bora `bist/THYAO`, can nothing; synthetic
/// licensed flow on `px`) with the feed's first `k` bars and flow seed `seed` (0: no
/// licensed flow at all).
fn init(bars: &[Bar], k: usize, seed: u64) -> SessionInit {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/39-capsule-entitlement.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut v = t["init"].clone();
    v["data"]["feeds"][0]["bars"] = json!(bars[..k]);
    if seed == 0 {
        v["capsules"]["flow"] = json!([]);
    } else {
        v["capsules"]["flow"][0]["seed"] = json!(seed);
    }
    serde_json::from_value(v).unwrap()
}

fn ev(seq: usize, b: &Bar) -> DataEvent {
    serde_json::from_value(
        json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},
        "seq": seq, "change": {"kind":"bar","bar": b}}),
    )
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn hello(s: &mut Session, conn: &str, who: &str, lang: Option<&str>) -> Vec<OutboundLine> {
    let mut surface = json!({"kind":"headless"});
    if let Some(l) = lang {
        surface["lang"] = json!(l);
    }
    send(
        s,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":surface}),
    )
}

/// One run (the probe's `full_run`): hellos, audiences and subscriptions of the
/// unentitled readers, the bars `k..n` one by one, then 16 queries per connection.
fn full_run(bars: &[Bar], n: usize, seed: u64) -> Vec<OutboundLine> {
    let k = 60;
    let mut s = Session::new(init(bars, k, seed), &root()).unwrap();
    let mut out = Vec::new();
    out.extend(hello(&mut s, "t", "ana", None));
    out.extend(hello(&mut s, "e", "ana", Some("en")));
    out.extend(hello(&mut s, "b", "bora", None));
    out.extend(hello(&mut s, "c", "can", Some("en")));
    for c in ["b", "c"] {
        out.extend(send(
            &mut s,
            c,
            &json!({"type":"command","id":"aud","ops":[{"op":"set_audience",
            "audience":{"group":"future","sensitivity":"important","watchlist":["THYAO"]}}]}),
        ));
        for (i, t) in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","all":true}),
            json!({"kind":"capsules","instrument":"THYAO"}),
            json!({"kind":"changes"}),
            json!({"kind":"claims"}),
        ]
        .iter()
        .enumerate()
        {
            out.extend(send(
                &mut s,
                c,
                &json!({"type":"subscribe","id":format!("s{i}"),"topic":t}),
            ));
        }
    }
    for (i, b) in bars.iter().enumerate().take(n).skip(k) {
        out.extend(s.data_event(&ev(i - k + 1, b)).unwrap());
    }
    let last = (n - 1) as u64;
    for c in ["b", "c", "t", "e"] {
        for q in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsules","audience":true}),
            json!({"kind":"capsules","meaning":"flow"}),
            json!({"kind":"capsules","open":true}),
            json!({"kind":"capsules","since_bar": last - 30}),
            json!({"kind":"ledger"}),
            json!({"kind":"ledger","by_kind":true}),
            json!({"kind":"describe"}),
            json!({"kind":"log"}),
            json!({"kind":"document"}),
            json!({"kind":"list","collection":"facts","full":true}),
            json!({"kind":"list","collection":"capsules","full":true}),
            json!({"kind":"list","collection":"findings","full":true}),
            json!({"kind":"present","finding":{"latest":true}}),
            json!({"kind":"present","finding":{"kind":"flow"}}),
            json!({"kind":"verify_evidence","item":{"latest":true}}),
        ] {
            out.extend(send(&mut s, c, &json!({"type":"query","id":"q","query":q})));
        }
    }
    out
}

/// Removes the server comments (the mühür messages) from a message.
fn strip_comments(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("comments");
            if m.get("kind").and_then(Value::as_str) == Some("server_comment") {
                m.clear();
                m.insert("kind".into(), json!("server_comment(stripped)"));
            }
            for x in m.values_mut() {
                strip_comments(x);
            }
        }
        Value::Array(a) => {
            a.retain(|x| x.get("trust").and_then(Value::as_str) != Some("server"));
            for x in a {
                strip_comments(x);
            }
        }
        _ => {}
    }
}

/// Connection `conn`'s lines, mühür messages removed (a line that is one is dropped).
fn stripped(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l.conn == conn)
        .filter_map(|l| {
            let mut v = serde_json::to_value(&l.msg).unwrap();
            strip_comments(&mut v);
            let t = serde_json::to_string(&v).unwrap();
            (!t.contains("server_comment(stripped)")).then_some(t)
        })
        .collect()
}

fn lines_of(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_string(&l.msg).unwrap())
        .collect()
}

/// The first differing line of two streams (for the failure message).
fn first_diff(a: &[String], b: &[String]) -> String {
    if a.len() != b.len() {
        return format!("{} vs {} lines", a.len(), b.len());
    }
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x != y {
            let at = x
                .bytes()
                .zip(y.bytes())
                .position(|(p, q)| p != q)
                .unwrap_or(0);
            let from = at.saturating_sub(120);
            return format!(
                "line {i}: …{}… vs …{}…",
                &x[from..(at + 120).min(x.len())],
                &y[from..(at + 120).min(y.len())]
            );
        }
    }
    String::new()
}

#[test]
fn licensed_flow_does_not_interfere_with_unentitled_readers() {
    let n = 200;
    let bars = qp_source::synthetic_market(1, n, 1)[0].bars.clone();
    let base = full_run(&bars, n, 1);
    let mut ana_differs = false;
    let mut comments_differ = false;
    for seed in [2_u64, 7, 42] {
        let other = full_run(&bars, n, seed);
        for c in ["b", "c"] {
            let x = stripped(&base, c);
            let y = stripped(&other, c);
            assert!(x.len() > 50, "{c}: {} lines", x.len());
            assert!(
                x == y,
                "seed 1 vs {seed}: {c} differs: {}",
                first_diff(&x, &y)
            );
            comments_differ |= lines_of(&base, c) != lines_of(&other, c);
        }
        ana_differs |= lines_of(&base, "t") != lines_of(&other, "t");
    }
    // Without any licensed flow (no `init.capsules.flow`): the same, too — an unentitled
    // reader gets what it would get if the licensed data did not exist, plus the mühür.
    let none = full_run(&bars, n, 0);
    for c in ["b", "c"] {
        let x = stripped(&base, c);
        let y = stripped(&none, c);
        assert!(x == y, "flow vs none: {c} differs: {}", first_diff(&x, &y));
    }
    // Positive controls: the licensed flow does change what the entitled reader gets,
    // and the mühür messages (the one channel left) do vary with it.
    assert!(ana_differs, "the entitled reader's lines never changed");
    assert!(comments_differ, "the server comments never changed");
}

/// Transcript 43's run (a bad print, a dispute by Bora, settled data that corrects it and
/// retracts the story) with Can and Arkın (owner, entitled) added: Arkın approves the
/// disputed finding, shows it and writes his audience; then the queries. Bora and Can
/// get the same lines (mühür messages aside) for every licensed flow and without one.
fn retract_run(seed: u64) -> Vec<OutboundLine> {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/43-capsule-retract.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut init = t["init"].clone();
    if seed == 0 {
        init["capsules"]["flow"] = json!([]);
    } else {
        init["capsules"]["flow"][0]["seed"] = json!(seed);
    }
    let mut s = Session::new(serde_json::from_value(init).unwrap(), &root()).unwrap();
    let mut out = Vec::new();
    out.extend(hello(&mut s, "c", "can", None));
    out.extend(hello(&mut s, "o", "arkin", None));
    for (i, step) in t["steps"].as_array().unwrap().iter().enumerate() {
        if let Some(d) = step.get("data") {
            let ev: DataEvent = serde_json::from_value(d.clone()).unwrap();
            out.extend(s.data_event(&ev).unwrap());
        } else if let Some(m) = step.get("send") {
            let conn = m["conn"].as_str().unwrap();
            out.extend(send(&mut s, conn, &m["msg"]));
            if i == 1 {
                // After Bora's hello: Bora and Can subscribe.
                for c in ["r", "c"] {
                    for (j, topic) in [
                        json!({"kind":"capsules","all":true}),
                        json!({"kind":"changes"}),
                        json!({"kind":"claims"}),
                    ]
                    .iter()
                    .enumerate()
                    {
                        out.extend(send(
                            &mut s,
                            c,
                            &json!({"type":"subscribe","id":format!("s{j}"),"topic":topic}),
                        ));
                    }
                }
            }
        }
        if step
            .get("note")
            .and_then(Value::as_str)
            .is_some_and(|n| n.starts_with("Bar 152 is final"))
        {
            for op in [
                json!({"op":"approve_finding","finding":"fnd.THYAO.breakout.up.152","note":"ok"}),
                json!({"op":"show_finding","finding":"fnd.THYAO.breakout.up.152","view":"v152"}),
                json!({"op":"set_audience","audience":{"group":"future","watchlist":["THYAO"]}}),
            ] {
                out.extend(send(
                    &mut s,
                    "o",
                    &json!({"type":"command","id":"o","ops":[op]}),
                ));
            }
        }
    }
    for c in ["r", "c"] {
        for q in [
            json!({"kind":"capsules"}),
            json!({"kind":"capsule","capsule":"cap.THYAO.breakout.up.152"}),
            json!({"kind":"ledger","by_kind":true}),
            json!({"kind":"log"}),
            json!({"kind":"document"}),
            json!({"kind":"list","collection":"findings","full":true}),
            json!({"kind":"present","finding":{"latest":true}}),
        ] {
            out.extend(send(&mut s, c, &json!({"type":"query","id":"q","query":q})));
        }
    }
    out
}

#[test]
fn a_retraction_and_an_entitled_persons_commands_do_not_interfere_either() {
    let base = retract_run(1);
    // The scenario does what it says for the unentitled reader too.
    let r = stripped(&base, "r");
    assert!(
        r.iter().any(|l| l.contains("\"verdict\":\"upheld\"")),
        "{r:?}"
    );
    assert!(r.iter().any(|l| l.contains("dispute_upheld")));
    assert!(r.iter().any(|l| l.contains("\"v152\"")));
    let mut ana_differs = false;
    for seed in [2_u64, 7, 42, 0] {
        let other = retract_run(seed);
        for c in ["r", "c"] {
            let x = stripped(&base, c);
            let y = stripped(&other, c);
            assert!(
                x == y,
                "seed 1 vs {seed}: {c} differs: {}",
                first_diff(&x, &y)
            );
        }
        ana_differs |= lines_of(&base, "a") != lines_of(&other, "a");
    }
    assert!(ana_differs, "the entitled reader's lines never changed");
}
