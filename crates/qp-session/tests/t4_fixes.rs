//! RP2/T4 audit fixes (`rounds/RP2/audits/T4.md`, `rounds/RP2/experiments/T4-fixes.md`):
//! Y1 (conditions as a value oracle), Y2 (undo of the anchor stamp), O1 (gap notice for
//! the host), O2 (undo of one's own `put_series` behind its anchor stamp), O3 (the
//! engine says the evidence numbers), D2 (`value` origin of a bound fact), D4 (selector
//! scratch over the document).

mod common;

use common::{code, root, H};
use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::HostNotice;
use serde_json::{json, Value};

/// The `init` of a conformance transcript.
fn transcript_init(name: &str) -> SessionInit {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join(format!("fixtures/protocol/conformance/{name}.json")))
            .unwrap(),
    )
    .unwrap();
    serde_json::from_value(t["init"].clone()).unwrap()
}

fn ev(symbol: &str, seq: u64, change: Value) -> DataEvent {
    serde_json::from_value(
        json!({"feed":{"symbol":symbol,"interval":"1d"},"seq":seq,"change":change}),
    )
    .unwrap()
}

fn lines(h: &mut H, e: &DataEvent) -> Vec<Value> {
    h.s.data_event(e)
        .unwrap()
        .iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

/// (t, close, high) of the last bar of `px`, read by an entitled principal.
fn last_bar(h: &mut H, conn: &str) -> (i64, f64, f64) {
    let r = h.query(conn, json!({"kind":"at","series":"px"}));
    let o = &r["data"]["at"]["ohlcv"];
    (
        o["t"].as_i64().unwrap(),
        o["c"].as_f64().unwrap(),
        o["h"].as_f64().unwrap(),
    )
}

fn capsule_state(h: &mut H, conn: &str, id: &str) -> String {
    let r = h.query(conn, json!({"kind":"get","id":id}));
    r["data"]["item"]["state"]
        .as_str()
        .or_else(|| r["data"]["state"].as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("no capsule state in {r}"))
}

/// `31-no-entitlement`: arkin entitled (THYAO, ASELS), editor and viewer not.
fn no_entitlement() -> H {
    let mut h = H::with(transcript_init("31-no-entitlement"));
    h.hello("a", "arkin", "headless");
    h.hello("e", "editor", "headless");
    h.hello("v", "viewer", "headless");
    h
}

// ---------------------------------------------------------------------------
// Y1
// ---------------------------------------------------------------------------

/// The audit's attack: an unentitled editor binary-searches the last close of `px` with
/// findings whose invalidation thresholds `px`, reading each capsule's state. Every
/// probe is refused (`no_entitlement` at the condition's series), so the 20 probes
/// learn nothing: the outcomes do not depend on the data.
#[test]
fn y1_binary_search_learns_nothing() {
    let mut h = no_entitlement();
    let r = h.cmd(
        "e",
        "c0",
        json!([{"op":"add_fact","id":"fx","kind":"note","series":"idx","bar":0,"text":"x","evidence":{}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let lo = 0.0_f64;
    let mut hi = 1000.0_f64;
    let mut outcomes = Vec::new();
    for i in 0..20 {
        let x = f64::midpoint(lo, hi);
        let r = h.cmd(
            "e",
            &format!("p{i}"),
            json!([{"op":"add_finding","id":format!("probe{i}"),"claim":"p","facts":["fx"],
                "confidence":0.5,"invalidation":{"kind":"threshold","series":"px","op":"gt","value":x},
                "capsule":"p"}]),
        );
        assert_eq!(code(&r), "no_entitlement", "{r}");
        assert_eq!(r["error"]["path"], "/ops/0/invalidation/series");
        outcomes.push(
            r.to_string()
                .replace(&format!("{x}"), "X")
                .replace(&format!("p{i}"), "P"),
        );
        // What an attacker would do with the outcome: nothing to branch on.
        hi = x;
    }
    outcomes.dedup();
    assert_eq!(outcomes.len(), 1, "every probe answers the same");
    // Through a study on the bound series, and as the right side of a comparison.
    for (cond, path) in [
        (
            json!({"kind":"threshold","series":"rsi14","op":"gt","value":50.0}),
            "/ops/0/invalidation/series",
        ),
        (
            json!({"kind":"compare","left":"idx","op":"gt","right":"px"}),
            "/ops/0/invalidation/right",
        ),
    ] {
        let r = h.cmd(
            "e",
            "cx",
            json!([{"op":"add_finding","claim":"p","facts":["fx"],"confidence":0.5,
                "invalidation":cond,"capsule":"p"}]),
        );
        assert_eq!(code(&r), "no_entitlement", "{r}");
        assert_eq!(r["error"]["path"], path);
    }
    // update_finding: the editor's own finding on inline data, re-pointed at px.
    let r = h.cmd(
        "e",
        "c1",
        json!([{"op":"add_finding","id":"mine","claim":"p","facts":["fx"],"confidence":0.5,
            "invalidation":{"kind":"threshold","series":"idx","op":"gt","value":1e9}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.cmd(
        "e",
        "c2",
        json!([{"op":"update_finding","finding":"mine",
            "invalidation":{"kind":"threshold","series":"px","op":"gt","value":1.0}}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/invalidation/series");
    // A proposal is checked as its proposer (dry run now).
    let r = h.one(
        "e",
        json!({"type":"command","id":"c3","mode":"propose","ops":[{"op":"add_finding","claim":"p",
            "facts":["fx"],"confidence":0.5,
            "invalidation":{"kind":"threshold","series":"px","op":"gt","value":1.0}}]}),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    // The entitled owner may.
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"add_finding","claim":"p","facts":["fx"],"confidence":0.5,
            "invalidation":{"kind":"threshold","series":"px","op":"gt","value":1.0}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    h.assert_replay();
}

/// `subscribe` with a `condition` topic and `add_intent` need the entitlement too.
#[test]
fn y1_condition_subscription_and_intent_refused() {
    let mut h = no_entitlement();
    let r = h.one(
        "v",
        json!({"type":"subscribe","id":"s1","topic":{"kind":"condition",
            "condition":{"kind":"threshold","series":"px","op":"gt","value":1.0}}}),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    assert_eq!(r["error"]["path"], "/topic/condition/series");
    let r = h.cmd(
        "e",
        "c1",
        json!([{"op":"add_intent","text":"t","trigger":{"kind":"threshold","series":"rsi14","op":"gt","value":70.0}}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/trigger/series");
    // Inline data stays open to everyone.
    let r = h.one(
        "v",
        json!({"type":"subscribe","id":"s2","topic":{"kind":"condition",
            "condition":{"kind":"threshold","series":"idx","op":"gt","value":1.0}}}),
    );
    assert_eq!(r["type"], "ack", "{r}");
    let r = h.one(
        "a",
        json!({"type":"subscribe","id":"s3","topic":{"kind":"condition",
            "condition":{"kind":"threshold","series":"px","op":"gt","value":1.0}}}),
    );
    assert_eq!(r["type"], "ack", "{r}");
}

/// An inline series of `n` bars closing at `c`, from the bars of `px`'s time axis.
fn inline_series(id: &str, n: usize, c: f64) -> Value {
    let bars: Vec<Value> = (0..n)
        .map(|i| json!({"t": 1_700_000_000_000_i64 + i as i64 * 86_400_000, "o": c, "h": c, "l": c, "c": c, "v": 1.0}))
        .collect();
    json!({"id": id, "source": {"kind":"inline"}, "data": {"kind":"bars","bars": bars}})
}

fn bind_thyao(id: &str) -> Value {
    json!({"id": id, "source": {"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":300}}})
}

/// The rebinding bypass: an unentitled editor writes a condition on its own inline
/// series (allowed), then rebinds that series to the feed it may not read. Since the RP2
/// T5 fixes the rebind itself is refused (a condition rests on the series); when the
/// owner rebinds it, the editor's condition is still not evaluated (its author is not
/// entitled), so its capsule's state tells nothing, whatever the data does; the owner's
/// identical capsule is invalidated.
#[test]
fn y1_rebinding_does_not_open_an_oracle() {
    let mut h = no_entitlement();
    let (t, c, _) = last_bar(&mut h, "a");
    let probe = c + 500.0;
    for (conn, id) in [("e", "own"), ("a", "ownA")] {
        let r = h.cmd(
            conn,
            "c0",
            json!([{"op":"put_series","series":inline_series(id, 5, c + 1000.0)},
                   {"op":"add_fact","id":format!("f-{id}"),"kind":"note","series":id,"bar":4,"text":"x","evidence":{}},
                   {"op":"add_finding","id":format!("fnd-{id}"),"claim":"p","facts":[format!("f-{id}")],"confidence":0.5,
                    "invalidation":{"kind":"threshold","series":id,"op":"lt","value":probe},"capsule":"p"}]),
        );
        assert_eq!(r["status"], "applied", "{r}");
    }
    let cap_e = "cap-2-3";
    let cap_a = "cap-3-3";
    assert_eq!(capsule_state(&mut h, "a", cap_e), "live");
    assert_eq!(capsule_state(&mut h, "a", cap_a), "live");
    // RP2 T5 fixes (§18.3): the editor may not re-point a series a condition rests on at
    // a feed it may not read.
    let r = h.cmd(
        "e",
        "c1",
        json!([{"op":"put_series","series":bind_thyao("own")}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/series/source", "{r}");
    // Rebind both to THYAO (the owner does it for the editor's series): px closes at
    // c < probe, so the condition holds on the data.
    for id in ["own", "ownA"] {
        let r = h.cmd(
            "a",
            "c1",
            json!([{"op":"put_series","series":bind_thyao(id)}]),
        );
        assert_eq!(r["status"], "applied", "{r}");
    }
    assert_eq!(capsule_state(&mut h, "a", cap_a), "invalidated");
    assert_eq!(capsule_state(&mut h, "e", cap_e), "live");
    // A data event does not wake it either.
    let _ = lines(
        &mut h,
        &ev(
            "THYAO",
            1,
            json!({"kind":"patch","t":t,"fields":{"c":c - 1.0}}),
        ),
    );
    assert_eq!(capsule_state(&mut h, "e", cap_e), "live");
    h.assert_replay();
}

/// `condition_met` and `intent_triggered` go only to subscribers entitled to the feeds
/// the condition reads (per subscriber, not the authority's view). The viewer's
/// condition on an inline series stays silent once the owner rebinds that series.
#[test]
fn y1_notifications_are_gated_per_subscriber() {
    // Incremental path (§18.8) and the from-scratch reference alike.
    notifications_gated(true);
    notifications_gated(false);
}

fn notifications_gated(incremental: bool) {
    let mut h = no_entitlement();
    h.s.set_incremental(incremental);
    let (t, c, hi) = last_bar(&mut h, "a");
    let r = h.cmd(
        "a",
        "c0",
        json!([{"op":"put_series","series":inline_series("shared", 5, 1.0)},
               {"op":"add_intent","id":"buy","text":"t","trigger":{"kind":"threshold","series":"px","op":"gt","value":c + 1.0}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let cond = json!({"kind":"threshold","series":"shared","op":"gt","value":c + 1.0});
    for conn in ["a", "v"] {
        let r = h.one(
            conn,
            json!({"type":"subscribe","id":"sc","topic":{"kind":"condition","condition":cond}}),
        );
        assert_eq!(r["type"], "ack", "{r}");
        let r = h.one(
            conn,
            json!({"type":"subscribe","id":"si","topic":{"kind":"intents"}}),
        );
        assert_eq!(r["type"], "ack", "{r}");
    }
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"put_series","series":bind_thyao("shared")}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let out = lines(
        &mut h,
        &ev(
            "THYAO",
            1,
            json!({"kind":"patch","t":t,"fields":{"c":c + 2.0,"h":hi.max(c + 2.0)}}),
        ),
    );
    let kinds = |conn: &str| -> Vec<String> {
        out.iter()
            .filter(|l| l["conn"] == conn)
            .filter_map(|l| l["msg"]["event"]["kind"].as_str().map(str::to_owned))
            .collect()
    };
    assert!(kinds("a").contains(&"condition_met".to_owned()), "{out:?}");
    assert!(
        kinds("a").contains(&"intent_triggered".to_owned()),
        "{out:?}"
    );
    assert!(kinds("v").is_empty(), "viewer hears nothing: {out:?}");
}

// ---------------------------------------------------------------------------
// Y2, O2
// ---------------------------------------------------------------------------

fn evidence() -> H {
    let mut h = H::with(transcript_init("32-verify-evidence"));
    h.hello("a", "arkin", "headless");
    h
}

fn anchor_of(h: &H, id: &str) -> Option<i64> {
    let s = h.s.document().series.iter().find(|s| s.id == id)?;
    match &s.source {
        qp_schema::SeriesSource::Bind { anchor_t, .. } => *anchor_t,
        _ => None,
    }
}

/// The owner may not undo the authority's anchor stamp (entry 1): bar numbers would
/// shift (audit Y2).
#[test]
fn y2_anchor_stamp_cannot_be_undone() {
    let mut h = evidence();
    let anchor = anchor_of(&h, "px").unwrap();
    assert_eq!(h.s.log()[0].author, "qp");
    let r = h.cmd("a", "u", json!([{"op":"undo","seq":1}]));
    assert_eq!(code(&r), "undo_conflict", "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("anchor"),
        "{r}"
    );
    assert_eq!(anchor_of(&h, "px"), Some(anchor));
    // After data events too (the audit's scenario).
    let (t, _, _) = last_bar(&mut h, "a");
    for i in 1..=5_u64 {
        let c = 100.0 + i as f64;
        let _ = lines(
            &mut h,
            &ev(
                "THYAO",
                i,
                json!({"kind":"bar","bar":{"t":t + i as i64 * 86_400_000,"o":c,"h":c,"l":c,"c":c,"v":1.0}}),
            ),
        );
    }
    let r = h.cmd("a", "u", json!([{"op":"undo","seq":1}]));
    assert_eq!(code(&r), "undo_conflict", "{r}");
    assert_eq!(anchor_of(&h, "px"), Some(anchor));
    h.assert_replay();
}

/// The owner who put a bound series undoes their own `put_series` although the anchor
/// stamp came after it; the stamp goes with it. A rewrite undone brings the earlier
/// binding back with its earlier anchor (bar numbers as before).
#[test]
fn o2_own_put_series_undoes_through_its_anchor() {
    let mut h = evidence();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"put_series","series":{"id":"px3","source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":50}}}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let put_seq = r["seq"].as_u64().unwrap();
    assert_eq!(h.s.log().len() as u64, put_seq + 1, "anchor stamp follows");
    assert!(anchor_of(&h, "px3").is_some());
    // The stamp itself stays refused.
    let r = h.cmd("a", "u0", json!([{"op":"undo","seq":put_seq + 1}]));
    assert_eq!(code(&r), "undo_conflict", "{r}");
    let r = h.cmd("a", "u1", json!([{"op":"undo"}]));
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(r["undoes"], put_seq);
    assert!(h.s.document().series.iter().all(|s| s.id != "px3"));
    h.assert_replay();

    // A rewrite of px (a new window) and its undo.
    let anchor = anchor_of(&h, "px").unwrap();
    let t250 = h.query("a", json!({"kind":"at","series":"px","bar":250}))["data"]["at"]["ohlcv"]
        ["t"]
        .as_i64()
        .unwrap();
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"put_series","series":{"id":"px","instrument":{"symbol":"THYAO","exchange":"BIST"},"interval":"1d",
            "source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":100}}}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    assert_ne!(
        anchor_of(&h, "px"),
        Some(anchor),
        "re-anchored on the new window"
    );
    let r = h.cmd("a", "u2", json!([{"op":"undo"}]));
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(anchor_of(&h, "px"), Some(anchor));
    let t = h.query("a", json!({"kind":"at","series":"px","bar":250}))["data"]["at"]["ohlcv"]["t"]
        .as_i64()
        .unwrap();
    assert_eq!(t, t250, "bar numbers as before the rewrite");
    h.assert_replay();
}

// ---------------------------------------------------------------------------
// O1
// ---------------------------------------------------------------------------

/// A `seq` gap is a notice for the host (not an error): the binding is stale.
#[test]
fn o1_gap_is_reported_to_the_host() {
    let mut h = evidence();
    let (t, _, _) = last_bar(&mut h, "a");
    let o =
        h.s.host_data_event(&ev(
            "THYAO",
            7,
            json!({"kind":"patch","t":t,"fields":{"v":1.0}}),
        ))
        .unwrap();
    assert_eq!(
        o.notices,
        vec![HostNotice::Gap {
            series: "px".to_owned(),
            feed: serde_json::from_value(json!({"symbol":"THYAO","interval":"1d"})).unwrap(),
            expected: 1,
            got: 7
        }]
    );
    assert_eq!(
        o.notices[0].frame(),
        json!({"host":{"gap":{"series":"px","feed":{"symbol":"THYAO","interval":"1d"},"expected":1,"got":7}}})
    );
    assert_eq!(
        h.s.binding_state("px"),
        Some(qp_schema::data::BindingState::Stale)
    );
    // While stale, later events are not noticed again; the host re-resolves.
    let o =
        h.s.host_data_event(&ev(
            "THYAO",
            8,
            json!({"kind":"patch","t":t,"fields":{"v":1.0}}),
        ))
        .unwrap();
    assert!(o.notices.is_empty());
    let _ = h.s.refresh_bindings();
    assert_eq!(
        h.s.binding_state("px"),
        Some(qp_schema::data::BindingState::Ready)
    );
    // The host frame path returns the same notice.
    let frame = json!({"host":{"data":{"feed":{"symbol":"THYAO","interval":"1d"},"seq":20,
        "change":{"kind":"patch","t":t,"fields":{"v":1.0}}}}});
    let o = h.s.handle_host_frame(&frame.to_string()).unwrap();
    assert!(matches!(
        o.notices.as_slice(),
        [HostNotice::Gap { got: 20, .. }]
    ));
}

// ---------------------------------------------------------------------------
// O3
// ---------------------------------------------------------------------------

/// `add_fact` on a study series: the client's `value` / `prev` must be the engine's
/// numbers, bit for bit (`invalid`, `expected` the engine's value).
#[test]
fn o3_evidence_values_are_the_engines() {
    let mut h = evidence();
    let r = h.query(
        "a",
        json!({"kind":"study","series":"rsi14","range":{"from_bar":249,"to_bar":250}}),
    );
    let col = &r["data"]["values"]["outputs"]["rsi"];
    let vals: Vec<f64> = col
        .as_array()
        .unwrap_or_else(|| panic!("{r}"))
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let (prev, value) = (vals[0], vals[1]);
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_fact","kind":"value","series":"rsi14","bar":250,"text":"x","evidence":{"value":50.0}}]),
    );
    assert_eq!(code(&r), "invalid", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/evidence/value");
    assert_eq!(r["error"]["expected"], json!([format!("{value}")]));
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"add_fact","kind":"value","series":"rsi14","bar":250,"text":"x","evidence":{"value":value,"prev":prev + 1e-9}}]),
    );
    assert_eq!(code(&r), "invalid", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/evidence/prev");
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"add_fact","kind":"value","series":"rsi14","bar":0,"text":"x","evidence":{"value":1.0}}]),
    );
    assert_eq!(code(&r), "invalid", "warm-up has no value: {r}");
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"add_fact","id":"ok","kind":"value","series":"rsi14","bar":250,"text":"x","evidence":{"value":value,"prev":prev,"level":40.0}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    // Other keys and bar series are not checked.
    let r = h.cmd(
        "a",
        "c5",
        json!([{"op":"add_fact","kind":"level","series":"px","bar":250,"text":"x","evidence":{"value":1.0}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
}

/// The same on an inline document (no data plane).
#[test]
fn o3_inline_study_checked_too() {
    let mut h = H::new();
    h.hello("a", "arkin", "headless");
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_fact","kind":"value","series":"rsi14","bar":100,"text":"x","evidence":{"value":12.5}}]),
    );
    assert_eq!(code(&r), "invalid", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/evidence/value");
}

// ---------------------------------------------------------------------------
// D2, D4
// ---------------------------------------------------------------------------

/// A `value` origin on a fact over a bound study works for the entitled writer (the
/// constant is the engine's value) and is `no_entitlement` for an unentitled one.
#[test]
fn d2_value_origin_of_a_bound_fact() {
    let mut h = H::with(transcript_init("31-no-entitlement"));
    h.hello("a", "arkin", "headless");
    h.hello("e", "editor", "headless");
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_fact","id":"f1","kind":"note","series":"rsi14","bar":100,"text":"x","evidence":{}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let expect = h.query(
        "a",
        json!({"kind":"study","series":"rsi14","range":{"from_bar":100,"to_bar":100}}),
    )["data"]["values"]["outputs"]["rsi"][0]
        .as_f64()
        .unwrap();
    let view = h.s.document().views[0].id.clone();
    let panel = h.s.document().views[0]
        .layers
        .iter()
        .find(|l| l.series.as_deref() == Some("rsi14"))
        .map(|l| l.panel)
        .unwrap();
    let layer = |id: &str| {
        json!({"id": id, "mark":"rule","panel":panel,"encoding":{},
            "origin":{"/encoding/y":{"kind":"fact","fact":"f1","field":"value"}}})
    };
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"insert_layer","view":view,"layer":layer("lv")}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let l = h.s.document().views[0]
        .layers
        .iter()
        .find(|l| l.id.as_deref() == Some("lv"))
        .unwrap()
        .clone();
    let y = serde_json::to_value(&l.encoding).unwrap()["y"].clone();
    assert_eq!(y.as_f64().map(f64::to_bits), Some(expect.to_bits()), "{y}");
    let r = h.cmd(
        "e",
        "c3",
        json!([{"op":"insert_layer","view":view,"layer":layer("lv2")}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    h.assert_replay();
}

/// A selector command on a bound document resolves over the document (no bound bars
/// copied) and gives the same result as the command with ids.
#[test]
fn d4_selector_command_on_a_bound_document() {
    let run = |sel: bool| {
        let mut h = evidence();
        let facts = if sel {
            json!([{"kind":"note","on":"rsi14","bar":200}])
        } else {
            json!(["f-new"])
        };
        let r = h.cmd(
            "a",
            "c1",
            json!([{"op":"add_fact","id":"f-new","kind":"note","series":"rsi14","bar":200,"text":"x","evidence":{}},
                   {"op":"add_finding","id":"g","claim":"c","facts":facts,"confidence":0.5,
                    "invalidation":{"kind":"threshold","series":"rsi14","op":"gt","value":99.0}}]),
        );
        assert_eq!(r["status"], "applied", "{r}");
        h.assert_replay();
        serde_json::to_string(h.s.document()).unwrap()
    };
    assert_eq!(run(true), run(false));
}
