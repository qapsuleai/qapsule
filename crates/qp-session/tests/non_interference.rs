//! R3/K1 (audit Y1, O1): non-interference of the licensed flow.
//!
//! Same public bars, different licensed flows (seeds of the synthetic flow): everything a
//! reader without the flow entitlement receives — every topic (`capsules`, `capsules
//! {all}`, `capsules {instrument}`, `changes`, `claims`) and the final queries the
//! auditor's probe asked (`rounds/R3/audit/src/bin/session_probe.rs`) — is byte-identical
//! once the server comments (mühür messages: `trust: server`, `server_comment`) are taken
//! out. Those are the only channel the licensed data may influence for them (M6
//! integration rule, Ö37). Positive control: the fully entitled reader's lines do differ.

use std::collections::BTreeSet;
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
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x != y {
            let at = x
                .bytes()
                .zip(y.bytes())
                .position(|(p, q)| p != q)
                .unwrap_or(0);
            let cut = |t: &str| {
                let mut a = at.saturating_sub(120);
                while !t.is_char_boundary(a) {
                    a -= 1;
                }
                let mut b = (at + 120).min(t.len());
                while !t.is_char_boundary(b) {
                    b += 1;
                }
                t[a..b].to_owned()
            };
            return format!("line {i}: …{}… vs …{}…", cut(x), cut(y));
        }
    }
    format!("{} vs {} lines", a.len(), b.len())
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

/// Security review of K1 (four channels): a public-view reader who may write (Bora, made
/// an owner here: his entitlements are unchanged) acts on every engine id the authority's
/// world has in any of the runs — direct `approve_finding`, a proposal of it, a `dispute`,
/// a `capsule` query — and an entitled owner (Arkın) adds a finding of his own with a
/// data invalidation that holds (an invalidation entry with evidence) and writes a data
/// invalidation onto an engine finding both worlds have (never evaluated: an engine
/// capsule ends by the outcome rule only).
fn oracle_run(
    bars: &[Bar],
    n: usize,
    seed: u64,
    ids: &[(String, String)],
    shared: &str,
) -> (Vec<OutboundLine>, Session) {
    let k = 60;
    let mut v = serde_json::to_value(init(bars, k, seed)).unwrap();
    for p in v["principals"].as_array_mut().unwrap() {
        if p["id"] == "bora" {
            p["role"] = json!("owner");
        }
    }
    let mut s = Session::new(serde_json::from_value(v).unwrap(), &root()).unwrap();
    let mut out = Vec::new();
    out.extend(hello(&mut s, "t", "ana", None));
    out.extend(hello(&mut s, "b", "bora", None));
    out.extend(hello(&mut s, "c", "can", Some("en")));
    out.extend(hello(&mut s, "o", "arkin", None));
    for c in ["b", "c"] {
        for (i, t) in [
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
                &json!({"type":"subscribe","id":format!("s{i}"),"topic":t}),
            ));
        }
    }
    let half = (k + n) / 2;
    for (i, b) in bars.iter().enumerate().take(half).skip(k) {
        out.extend(s.data_event(&ev(i - k + 1, b)).unwrap());
    }
    // Arkın: his own finding, invalid as soon as it is written (close > half of now).
    let cond = json!({"kind":"threshold","series":"px","op":"gt","value": bars[half].c * 0.5});
    for op in [
        json!({"op":"add_fact","id":"fx-arkin","kind":"note","series":"px","bar":half - 2,"text":"not"}),
        json!({"op":"add_finding","id":"fnd-arkin","claim":"mine","facts":["fx-arkin"],"confidence":0.5,
               "invalidation":cond,"capsule":"Arkın'ın notu"}),
        json!({"op":"update_finding","finding":shared,"invalidation":cond}),
    ] {
        out.extend(send(
            &mut s,
            "o",
            &json!({"type":"command","id":"o","ops":[op]}),
        ));
    }
    // (Audit K1, attempt 3) Arkın, entitled, sends the same list in every configuration:
    // approve or reject every finding id any run's authority had, and show one. Whether
    // each applies in the authority's world depends on the licensed data; nothing of it
    // may reach a public-view reader.
    for (i, (_, fnd)) in ids.iter().enumerate() {
        let op = if i % 3 == 0 {
            json!({"op":"reject_finding","finding":fnd})
        } else {
            json!({"op":"approve_finding","finding":fnd})
        };
        out.extend(send(
            &mut s,
            "o",
            &json!({"type":"command","id":format!("o{i}"),"ops":[op]}),
        ));
    }
    if let Some((_, fnd)) = ids.first() {
        out.extend(send(
            &mut s,
            "o",
            &json!({"type":"command","id":"os","ops":[{"op":"show_finding","finding":fnd,"view":"ov"}]}),
        ));
    }
    for (i, b) in bars.iter().enumerate().take(n - 1).skip(half) {
        out.extend(s.data_event(&ev(i - k + 1, b)).unwrap());
    }
    let t = bars[n - 5].t;
    // (Audit K1, attempt 4) The authority's engine items before the public-view writer.
    let engine_items = |s: &Session| -> Vec<String> {
        s.document()
            .capsules
            .iter()
            .filter(|c| c.story.is_some())
            .map(|c| serde_json::to_string(c).unwrap())
            .chain(
                s.document()
                    .findings
                    .iter()
                    .filter(|f| matches!(f.provenance.author, qp_schema::Author::Engine { .. }))
                    .map(|f| serde_json::to_string(f).unwrap()),
            )
            .collect()
    };
    let before = engine_items(&s);
    for (cap, fnd) in ids {
        for m in [
            json!({"type":"command","id":"x","ops":[{"op":"approve_finding","finding":fnd}]}),
            json!({"type":"command","id":"p","mode":"propose","ops":[{"op":"reject_finding","finding":fnd}]}),
            json!({"type":"dispute","id":"d","claim":{"kind":"capsule","capsule":cap,"finding":fnd},"t":t}),
            json!({"type":"dispute","id":"e","claim":{"kind":"capsule","capsule":cap,"finding":fnd},"t":t,"digest":"zz"}),
            json!({"type":"query","id":"q","query":{"kind":"capsule","capsule":cap}}),
        ] {
            let r = send(&mut s, "b", &m);
            // (R3/K1b) Bora accepts every proposal he made (a public-view decider).
            let props: Vec<String> = r
                .iter()
                .filter_map(|l| {
                    let v = serde_json::to_value(&l.msg).unwrap();
                    v["proposal"].as_str().map(str::to_owned)
                })
                .collect();
            out.extend(r);
            for pid in props {
                out.extend(send(
                    &mut s,
                    "b",
                    &json!({"type":"command","id":"a","ops":[{"op":"accept_proposal","proposal":pid}]}),
                ));
            }
        }
    }
    // (R3/K1b) Bora writes his own fact, finding and capsule under ids the authority's
    // world may give to its own (licensed) items, with an invalidation that holds.
    for (i, (_, fnd)) in ids.iter().take(4).enumerate() {
        let cond = json!({"kind":"threshold","series":"px","op":"gt","value": bars[n - 3].c * 0.5});
        for op in [
            json!({"op":"add_fact","id":format!("fx-bora-{i}"),"kind":"note","series":"px","bar":n - 4,"text":"b"}),
            json!({"op":"add_finding","id":fnd,"claim":"Bora'nın","facts":[format!("fx-bora-{i}")],
                   "confidence":0.4,"invalidation":cond,"capsule":"Bora'nın notu"}),
        ] {
            out.extend(send(
                &mut s,
                "b",
                &json!({"type":"command","id":"w","ops":[op]}),
            ));
        }
    }
    // Bora (a public-view owner) approved, proposed, accepted, disputed and wrote under
    // engine ids: the authority's engine items are untouched.
    assert!(
        before == engine_items(&s),
        "a public-view writer changed an authority engine item (seed {seed})"
    );
    out.extend(s.data_event(&ev(n - k, &bars[n - 1])).unwrap());
    for c in ["b", "c"] {
        for q in [
            json!({"kind":"log"}),
            json!({"kind":"document"}),
            json!({"kind":"capsules"}),
        ] {
            out.extend(send(&mut s, c, &json!({"type":"query","id":"q","query":q})));
        }
    }
    (out, s)
}

/// Engine (capsule, finding) ids of a seed's authority document after `n` bars.
fn engine_ids(bars: &[Bar], n: usize, seed: u64) -> BTreeSet<(String, String)> {
    let k = 60;
    let mut s = Session::new(init(bars, k, seed), &root()).unwrap();
    for (i, b) in bars.iter().enumerate().take(n).skip(k) {
        s.data_event(&ev(i - k + 1, b)).unwrap();
    }
    s.document()
        .capsules
        .iter()
        .filter(|c| c.story.is_some())
        .map(|c| (c.id.clone(), c.finding.clone()))
        .collect()
}

#[test]
fn no_oracle_through_commands_proposals_disputes_or_invalidations() {
    let n = 200;
    let bars = qp_source::synthetic_market(1, n, 1)[0].bars.clone();
    let seeds = [1_u64, 2, 7, 42];
    let per: Vec<BTreeSet<(String, String)>> =
        seeds.iter().map(|s| engine_ids(&bars, n, *s)).collect();
    let public = engine_ids(&bars, n, 0);
    // Every id some authority world has and the public world has not (licensed-born, or
    // only there because a licensed fact changed a story), and a few shared ones.
    let mut ids: Vec<(String, String)> = per
        .iter()
        .flatten()
        .filter(|x| !public.contains(x))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(ids.len() > 5, "{ids:?}");
    // Seed 1's authority-only ids first: Bora writes his own items under the first ones.
    ids.sort_by_key(|x| !per[0].contains(x));
    ids.extend(public.iter().cloned());
    // An engine finding every world has by the middle of the run.
    let half = 130;
    let mut common = engine_ids(&bars, half, 0);
    for s in seeds {
        let x = engine_ids(&bars, half, s);
        common.retain(|c| x.contains(c));
    }
    let shared = common
        .iter()
        .next()
        .expect("a shared engine finding")
        .1
        .clone();
    let (base, auth) = oracle_run(&bars, n, 1, &ids, &shared);
    let b = stripped(&base, "b");
    assert!(
        b.iter()
            .any(|l| l.contains("invalidation of fnd-arkin holds")),
        "no invalidation: {:.3000}",
        lines_of(&base, "o")[1..].join("\n")
    );
    assert!(!b
        .iter()
        .any(|l| l.contains(&format!("invalidation of {shared}"))));
    // (R3/K1b) Bora accepted proposals as a public-view decider.
    assert!(b
        .iter()
        .any(|l| l.contains("\"id\":\"a\"") && l.contains("\"status\":\"applied\"")));
    // (R3/K1b) Bora's own findings under ids the authority's world gives to its own
    // items: the authority's items are untouched, Bora sees his own (and its capsule's
    // invalidation, evaluated in the public world).
    let mut collided = 0;
    for (cap, fnd) in ids.iter().take(4) {
        let pair = (cap.clone(), fnd.clone());
        if let Some(f) = auth
            .document()
            .findings
            .iter()
            .find(|f| &f.id == fnd && per[0].contains(&pair))
        {
            collided += 1;
            assert!(
                matches!(f.provenance.author, qp_schema::Author::Engine { .. }),
                "{fnd} overwritten in the authority: {f:?}"
            );
            assert_ne!(f.claim, "Bora'nın");
        }
        assert!(
            b.iter()
                .any(|l| l.contains(&format!("invalidation of {fnd} holds"))),
            "{fnd}: Bora's capsule not invalidated in his world"
        );
    }
    assert!(collided > 0, "no collision exercised");
    let doc_b = b
        .iter()
        .rev()
        .find(|l| l.contains("\"kind\":\"document\""))
        .unwrap();
    assert!(doc_b.contains("Bora'nın"));
    // (Without a flow there is no public world: Bora, a writer there, reads the authority,
    // whose claims on document capsules a public-view reader does not get; so only the
    // licensed flow varies here.)
    for seed in [2_u64, 7, 42] {
        let other = oracle_run(&bars, n, seed, &ids, &shared).0;
        for c in ["b", "c"] {
            let x = stripped(&base, c);
            let y = stripped(&other, c);
            assert!(
                x == y,
                "seed 1 vs {seed}: {c} differs: {}",
                first_diff(&x, &y)
            );
        }
    }
}
