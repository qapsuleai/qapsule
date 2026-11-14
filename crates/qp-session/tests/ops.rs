//! Every operation: expansion into changes, ids, provenance, and replay.

mod common;

use common::{code, H};
use serde_json::{json, Value};

fn owner() -> H {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h
}

fn last_changes(h: &H) -> Value {
    serde_json::to_value(&h.s.log().last().unwrap().changes).unwrap()
}

#[test]
fn put_series_inline_and_study_and_unsupported() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"put_series","series":{"id":"spy","source":{"kind":"inline"},
            "data":{"kind":"bars","bars":[{"t":1,"o":1.0,"h":2.0,"l":0.5,"c":1.5,"v":10.0}]}}}]),
    );
    assert_eq!(r["status"], "applied");
    assert_eq!(r["created"], json!(["spy"]));
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"put_series","series":{"id":"e9","source":{"kind":"study","study":"ema","inputs":["px"],"params":{"period":9}}}}]),
    );
    assert_eq!(r["status"], "applied");
    assert_eq!(
        h.s.document().series.last().unwrap().source,
        serde_json::from_value(
            json!({"kind":"study","study":"ema","inputs":["px"],"params":{"period":9.0}})
        )
        .unwrap()
    );
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"put_series","series":{"id":"f","source":{"kind":"file","path":"/etc/passwd"}}}]),
    );
    assert_eq!(code(&r), "unsupported");
    assert_eq!(r["error"]["path"], "/ops/0/series/source/kind");
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"put_series","series":{"id":"main","source":{"kind":"inline"},"data":{"kind":"bars","bars":[{"t":1,"o":1.0,"h":2.0,"l":0.5,"c":1.5,"v":1.0}]}}}]),
    );
    assert_eq!(code(&r), "duplicate", "ids are unique across collections");
    h.assert_replay();
}

#[test]
fn add_study_new_panel_adds_a_panel() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_study","study":"ema","inputs":["px"],"show":{"view":"main"}}]),
    );
    assert_eq!(r["created"], json!(["series-1-0"]));
    let ch = last_changes(&h);
    assert_eq!(ch[1]["op"], "put_panel");
    assert_eq!(
        ch[1]["panel"],
        json!({"id":3,"y":{"kind":"linear","id":"y3"}})
    );
    assert_eq!(ch[2]["op"], "insert_layer");
    let v = &h.s.document().views[0];
    assert!(v.y.is_empty());
    assert_eq!(v.panels.len(), 4);
    assert_eq!(v.layers.last().unwrap().panel, 3);
    assert_eq!(v.layers.last().unwrap().id.as_deref(), Some("l5"));
    let params = &h.s.document().series.last().unwrap().source;
    assert_eq!(
        serde_json::to_value(params).unwrap()["params"],
        json!({"period":20.0}),
        "defaults are stored"
    );
    h.assert_replay();
}

#[test]
fn set_params_merges_and_validates() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"set_params","series":"macd","params":{"signal":5}}]),
    );
    assert_eq!(r["status"], "applied");
    assert_eq!(
        last_changes(&h)[0]["series"]["source"]["params"],
        json!({"fast":12.0,"slow":26.0,"signal":5.0})
    );
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"set_params","series":"macd","params":{"fats":5}}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/params/fats");
    assert_eq!(r["error"]["did_you_mean"], json!(["fast"]));
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"set_params","series":"px","params":{"period":5}}]),
    );
    assert_eq!(code(&r), "invalid");
}

#[test]
fn views_and_layers() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"put_view","view":{"id":"v2","x":{"kind":"time"},"y":[{"kind":"linear"}],
            "layers":[{"mark":"candle","series":"px","panel":0}]}},
            {"op":"add_layer","view":"v2","layer":{"mark":"line","series":"bb20"},"index":0},
            {"op":"remove_layer","view":"v2","index":1}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(r["created"], json!(["v2"]));
    let v = h.s.document().views.iter().find(|v| v.id == "v2").unwrap();
    assert_eq!(v.layers.len(), 1);
    assert_eq!(v.layers[0].series.as_deref(), Some("bb20"));
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"remove_layer","view":"v2","index":7}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/index");
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"put_view","view":{"id":"v3","x":{"kind":"time"},"y":[],"layers":[{"mark":"candle","series":"pz"}]}}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/view/layers/0/series");
    assert_eq!(r["error"]["did_you_mean"], json!(["px"]));
    h.assert_replay();
}

#[test]
fn facts_findings_capsules_ledger() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([
            {"op":"add_fact","kind":"cross_below","series":"rsi14","bar":160,"text":"x","evidence":{"level":70}},
            {"op":"add_finding","claim":"c","facts":["fact-1-0"],"confidence":0.4,
             "invalidation":{"kind":"text","text":"never"},"capsule":"H"}
        ]),
    );
    assert_eq!(r["created"], json!(["fact-1-0", "fnd-1-1", "cap-1-2"]));
    let d = h.s.document();
    let fact = d.facts.last().unwrap();
    assert_eq!(fact.t, d.series[0].bars().unwrap()[160].t);
    let cap = d.capsules.last().unwrap();
    assert_eq!(cap.provenance.at, 1_760_000_000_000);
    assert_eq!(
        serde_json::to_value(&cap.provenance.author).unwrap(),
        json!({"kind":"human","name":"arkin"})
    );
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"update_finding","finding":"fnd-1-1","confidence":0.9,"claim":"new"}]),
    );
    assert_eq!(r["status"], "applied");
    assert_eq!(h.s.document().findings.last().unwrap().confidence, 0.9);
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"approve_finding","finding":"fnd-1-1","note":"ok"}]),
    );
    assert_eq!(r["status"], "applied");
    let d = h.s.document();
    assert_eq!(
        serde_json::to_value(d.capsules.last().unwrap().state).unwrap(),
        "approved"
    );
    assert_eq!(d.ledger.entries.len(), 1);
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"update_finding","finding":"fnd-1-1","confidence":1.5}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/confidence");
    let r = h.cmd(
        "a",
        "c5",
        json!([{"op":"add_finding","claim":"c","facts":["fact-rsi-cross7"],"confidence":0.4,"invalidation":{"kind":"text","text":"x"}}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/facts/0");
    assert_eq!(r["error"]["did_you_mean"], json!(["fact-rsi-cross70"]));
    h.assert_replay();
}

#[test]
fn intents_and_narratives() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_intent","id":"i1","text":"t"},{"op":"set_intent_active","intent":"i1","active":false}]),
    );
    assert_eq!(r["status"], "applied");
    assert!(!h.s.document().intents[0].active);
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"add_narrative","about":"fnd-overbought","steps":[
            {"at_ms":0,"target":{"series":"px"},"say":"a"},
            {"at_ms":10,"target":{"view":"nope"},"say":"b"}]}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/steps/1/target/view");
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"add_narrative","about":"fnd-overbought","steps":[{"at_ms":0,"target":{},"say":"a"}]}]),
    );
    assert_eq!(r["created"], json!(["narr-2-0"]));
    h.assert_replay();
}

#[test]
fn show_finding_assigns_view_id_and_attaches_capsule() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"show_finding","finding":"fnd-overbought"}]),
    );
    assert_eq!(r["created"], json!(["view-1-0"]));
    let d = h.s.document();
    assert_eq!(d.capsules[0].view.as_deref(), Some("view-1-0"));
    let v = d.views.iter().find(|v| v.id == "view-1-0").unwrap();
    assert_eq!(v.layers.len(), 3);
    assert_eq!(v.panels.len(), 2);
    // Deterministic: the same document gives the same view.
    let mut h2 = owner();
    h2.cmd(
        "a",
        "c1",
        json!([{"op":"show_finding","finding":"fnd-overbought"}]),
    );
    assert_eq!(
        serde_json::to_vec(h.s.document()).unwrap(),
        serde_json::to_vec(h2.s.document()).unwrap()
    );
    h.assert_replay();
}

#[test]
fn remove_cascade_and_in_use() {
    let mut h = owner();
    let r = h.cmd("a", "c1", json!([{"op":"remove","id":"rsi14"}]));
    assert_eq!(code(&r), "in_use");
    assert_eq!(
        r["error"]["expected"],
        json!(["fact-rsi-cross70", "fnd-overbought"])
    );
    let r = h.cmd("a", "c2", json!([{"op":"remove","id":"macd"}]));
    assert_eq!(r["status"], "applied");
    assert_eq!(
        last_changes(&h),
        json!([{"op":"delete_layer","view":"main","index":4,"layer":"l4"},{"op":"delete","id":"macd"}])
    );
    let r = h.cmd("a", "c3", json!([{"op":"remove","id":"cap-overbought"}]));
    assert_eq!(r["status"], "applied");
    h.assert_replay();
}

#[test]
fn feed_appends_and_updates_bars() {
    let mut h = owner();
    h.hello("f", "feed", "headless");
    let last_t = h.s.document().series[0].bars().unwrap().last().unwrap().t;
    let bar = |t: i64, c: f64| json!({"t":t,"o":124.0,"h":126.0,"l":123.0,"c":c,"v":1.0});
    let r = h.cmd(
        "f",
        "b1",
        json!([{"op":"append_bars","series":"px","bars":[bar(last_t + 86_400_000, 125.0)]}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd(
        "f",
        "b2",
        json!([{"op":"update_last_bar","series":"px","bar":bar(last_t + 86_400_000, 125.5)}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd(
        "f",
        "b3",
        json!([{"op":"update_last_bar","series":"px","bar":bar(last_t, 125.5)}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/bar/t");
    let r = h.cmd(
        "f",
        "b4",
        json!([{"op":"append_bars","series":"px","bars":[bar(last_t, 125.0)]}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/bars/0/t");
    let r = h.cmd(
        "f",
        "b5",
        json!([{"op":"append_bars","series":"px","bars":[{"t":last_t + 2 * 86_400_000,"o":1.0,"h":0.5,"l":1.0,"c":1.0,"v":1.0}]}]),
    );
    assert_eq!(code(&r), "invalid", "inconsistent OHLC");
    let r = h.cmd(
        "f",
        "b6",
        json!([{"op":"append_bars","series":"rsi14","bars":[bar(last_t + 9 * 86_400_000, 1.0)]}]),
    );
    assert_eq!(code(&r), "invalid", "study series take no bars");
    let n = h.s.document().series[0].bars().unwrap().len();
    assert_eq!(n, 201);
    h.assert_replay();
}

#[test]
fn set_view_range_sets_domain() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"set_view_range","view":"main","from_bar":10,"to_bar":5}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/from_bar");
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"set_view_range","view":"main","from_bar":0,"to_bar":199}]),
    );
    assert_eq!(r["status"], "applied");
    assert!(h.s.document().views[0].x.domain.is_some());
}

#[test]
fn all_or_nothing_and_ids_counted_per_entry() {
    let mut h = owner();
    let before = serde_json::to_vec(h.s.document()).unwrap();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_intent","text":"a"},{"op":"remove","id":"nope"}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(before, serde_json::to_vec(h.s.document()).unwrap());
    assert!(h.s.log().is_empty());
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"add_intent","text":"a"},{"op":"add_intent","id":"mine","text":"b"},{"op":"add_intent","text":"c"}]),
    );
    assert_eq!(r["created"], json!(["intent-1-0", "mine", "intent-1-2"]));
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"add_intent","id":"bad id","text":"a"}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/id");
}

#[test]
fn log_entries_carry_request_inverse_and_summary() {
    let mut h = owner();
    h.cmd(
        "a",
        "c1",
        json!([{"op":"add_study","study":"rsi","inputs":["px"],"params":{"period":7},"show":{"view":"main","with":"rsi14"}}]),
    );
    let e = serde_json::to_value(&h.s.log()[0]).unwrap();
    assert_eq!(e["cause"], "c1");
    assert_eq!(e["request"][0]["op"], "add_study");
    assert_eq!(
        e["inverse"],
        json!([{"op":"delete_layer","view":"main","index":5,"layer":"l5"},{"op":"delete","id":"series-1-0"}])
    );
    assert_eq!(e["summary"], "add_study rsi(period=7) -> series-1-0");
}
