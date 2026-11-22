//! Reference by meaning with structured selectors (R2/T8a, `docs/PROTOCOL.md` §17).
//!
//! `r_tasks` writes the twenty grammar tasks of `rounds/R2/tasks/TASKS.md` without a
//! single id of the document (only ids the agent itself chooses for new items) and
//! checks each task's success criterion on the resulting document.

mod support;

use serde_json::{json, Value};
use support::{code, H};

/// The R2 task setup on the `thyao` document: an SMA(50) series not in the view, two
/// breakout facts on the price series (bars 120 and 185, `evidence.level`) and a finding
/// with a live capsule on each. The setup knows ids; the tasks below do not.
fn setup() -> H {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    let r = h.cmd(
        "a",
        "setup",
        json!([
            {"op":"add_study","id":"s-50","study":"sma","inputs":["px"],"params":{"period":50}},
            {"op":"add_fact","id":"f-old","kind":"breakout","series":"px","bar":120,
             "text":"Old breakout.","evidence":{"level":101.5}},
            {"op":"add_fact","id":"f-new","kind":"breakout","series":"px","bar":185,
             "text":"New breakout.","evidence":{"level":121.3}},
            {"op":"add_finding","id":"n-old","claim":"Old breakout held.","facts":["f-old"],
             "confidence":0.5,"invalidation":{"kind":"threshold","series":"px","op":"lt","value":90},
             "capsule":"Old breakout"},
            {"op":"add_finding","id":"n-new","claim":"New breakout above 121.","facts":["f-new"],
             "confidence":0.7,"invalidation":{"kind":"threshold","series":"px","op":"lt","value":110},
             "capsule":"Breakout above 121"}
        ]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    h
}

fn doc(h: &H) -> Value {
    serde_json::to_value(h.s.document()).unwrap()
}

fn main_view(h: &H) -> Value {
    doc(h)["views"][0].clone()
}

fn layers(h: &H) -> Vec<Value> {
    main_view(h)["layers"].as_array().unwrap().clone()
}

fn panel_order(h: &H) -> Vec<u64> {
    main_view(h)["panels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_u64().unwrap())
        .collect()
}

fn applied(r: &Value) {
    assert_eq!(r["type"], "ack", "{r}");
    assert_eq!(r["status"], "applied", "{r}");
}

#[test]
fn r_tasks() {
    // R01: RSI(7) in a new panel right below the price panel.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r01",
        json!([
            {"op":"add_study","id":"rsi7","study":"rsi","inputs":[{"kind":"bars"}],"params":{"period":7}},
            {"op":"insert_panel","view":{},"panel":{"name":"rsi7","y":{"kind":"linear"}},
             "after":{"mark":"candle"},"layers":[{"mark":"line","series":"rsi7"}]}
        ]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/0/inputs/0"], "px");
    assert_eq!(r["resolved"]["/ops/1/after"], "0");
    let new_panel = r["created"][1].as_str().unwrap().parse::<u64>().unwrap();
    assert_eq!(panel_order(&h)[1], new_panel);
    h.assert_replay();

    // R02: the upper band red (the single Bollinger line is split into three).
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r02",
        json!([
            {"op":"set_encoding","view":{},"layer":{"shows":{"study":"bollinger"}},"encoding":{"y":"middle"}},
            {"op":"insert_layer","view":{},"layer":{"mark":"line","series":{"study":"bollinger"},
             "panel":{"mark":"candle"},"encoding":{"y":"upper","color":"down"}}},
            {"op":"insert_layer","view":{},"layer":{"mark":"line","series":{"study":"bollinger"},
             "panel":{"mark":"candle"},"encoding":{"y":"lower"}}}
        ]),
    );
    applied(&r);
    let ls = layers(&h);
    let upper = ls.iter().find(|l| l["encoding"]["y"] == "upper").unwrap();
    assert_eq!(upper["encoding"]["color"], "down");
    assert!(ls.iter().any(|l| l["encoding"]["y"] == "middle"));
    assert!(ls.iter().any(|l| l["encoding"]["y"] == "lower"));
    h.assert_replay();

    // R03: highlight the latest breakout.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r03",
        json!([{"op":"set_emphasis","view":{},"emphasis":[
            {"target":{"item":{"finding":{"kind":"breakout","latest":true}}},"kind":"highlight"}]}]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/0/emphasis/0/target/item"], "n-new");
    assert_eq!(main_view(&h)["emphasis"][0]["target"]["item"], "n-new");

    // R04: what the watch says about the latest breakout.
    let r = h.query(
        "a",
        json!({"kind":"present","finding":{"kind":"breakout","latest":true},"surface":"watch"}),
    );
    assert_eq!(r["data"]["presentation"]["text"], "Breakout above 121");
    assert_eq!(r["resolved"]["/query/finding"], "n-new");

    // R05: the price panel on a log scale.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r05",
        json!([{"op":"set_scale","view":{},"scale":{"panel":{"mark":"candle"}},"kind":"log"}]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/0/scale"], "y0");
    assert_eq!(main_view(&h)["panels"][0]["y"]["kind"], "log");
    assert_eq!(main_view(&h)["panels"][1]["y"]["kind"], "linear");

    // R06: a 30 line in the RSI panel (origin: rsi.oversold).
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r06",
        json!([{"op":"insert_layer","view":{},"layer":{"mark":"rule",
            "panel":{"shows":{"study":"rsi"}},"encoding":{"y":30}}}]),
    );
    applied(&r);
    let ls = layers(&h);
    let rule30 = ls.last().unwrap();
    assert_eq!(rule30["panel"], 1);
    assert_eq!(rule30["origin"]["/encoding/y"]["name"], "rsi.oversold");

    // R07: remove the MACD panel.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r07",
        json!([{"op":"delete_panel","view":{},"panel":{"shows":{"study":"macd"}}}]),
    );
    applied(&r);
    assert_eq!(panel_order(&h), vec![0, 1]);

    // R08: volume as bars right below the price panel.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r08",
        json!([{"op":"insert_panel","view":{},"panel":{"name":"volume","y":{"kind":"linear"}},
            "after":{"mark":"candle"},
            "layers":[{"mark":"bar","series":{"kind":"bars"},"encoding":{"y":"v"}}]}]),
    );
    applied(&r);
    let vol = layers(&h).into_iter().find(|l| l["mark"] == "bar").unwrap();
    assert_eq!(vol["series"], "px");
    assert_eq!(panel_order(&h)[1], vol["panel"].as_u64().unwrap());

    // R09: Bollinger as a filled band.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r09",
        json!([{"op":"insert_layer","view":{},"layer":{"mark":"band","series":{"study":"bollinger"},
            "panel":{"mark":"candle"},"encoding":{"y":"upper","y2":"lower","opacity":0.2}}}]),
    );
    applied(&r);

    // R10: MACD histogram as bars, green when >= 0, red otherwise.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r10",
        json!([{"op":"insert_layer","view":{},"layer":{"mark":"bar","series":{"study":"macd"},
            "panel":{"shows":{"study":"macd"}},"encoding":{"y":"histogram",
            "color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}}}]),
    );
    applied(&r);

    // R11: the close green above SMA(50), red below.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r11",
        json!([{"op":"insert_layer","view":{},"layer":{"mark":"line","series":{"kind":"bars"},
            "panel":{"mark":"candle"},"encoding":{"y":"c","color":{"when":{"field":"c","op":"ge",
            "than":{"series":{"study":"sma","params":{"period":50}},"field":"sma"}},
            "then":"up","else":"down"}}}}]),
    );
    applied(&r);
    assert_eq!(
        r["resolved"]["/ops/0/layer/encoding/color/when/than/series"],
        "s-50"
    );
    let l = layers(&h).last().unwrap().clone();
    assert_eq!(l["encoding"]["color"]["when"]["than"]["series"], "s-50");
    h.assert_replay();

    // R12: persistent range = last 60 bars.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r12",
        json!([{"op":"set_view_range","view":{},"from_bar":140,"to_bar":199}]),
    );
    applied(&r);

    // R13: what is on screen.
    let r = h.query("a", json!({"kind":"screen"}));
    assert_eq!(
        r["data"]["screens"][0]["panels"].as_array().unwrap().len(),
        3
    );
    assert_eq!(
        r["data"]["screens"][0]["panels"][1]["select"],
        json!({"shows":{"study":"rsi"}})
    );

    // R14: point query in the (only) view.
    let r = h.query(
        "a",
        json!({"kind":"hit","view":{},"x":900,"y":200,"width":1200,"height":600}),
    );
    assert_eq!(r["data"]["kind"], "hit", "{r}");
    assert_eq!(r["data"]["hit"]["view"], "main");
    assert_eq!(r["resolved"]["/query/view"], "main");

    // R15: thicker RSI line.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r15",
        json!([{"op":"set_encoding","view":{},"layer":{"shows":{"study":"rsi"}},"encoding":{"width":2}}]),
    );
    applied(&r);
    let l = layers(&h)
        .into_iter()
        .find(|l| l["series"] == "rsi14")
        .unwrap();
    assert_eq!(l["encoding"]["width"], 2.0);

    // R16: the old breakout's fact as a point in the price panel.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r16",
        json!([{"op":"insert_layer","view":{},"layer":{"mark":"point",
            "fact":{"kind":"breakout","first":true},"panel":{"mark":"candle"}}}]),
    );
    applied(&r);
    assert_eq!(layers(&h).last().unwrap()["fact"], "f-old");

    // R17: a horizontal line at 123.45: a fact first, then a rule bound to it.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r17",
        json!([
            {"op":"add_fact","id":"lvl-12345","kind":"level","series":{"kind":"bars"},"bar":199,
             "text":"User level 123.45.","evidence":{"level":123.45}},
            {"op":"insert_layer","view":{},"layer":{"mark":"rule","panel":{"mark":"candle"},
             "encoding":{"y":123.45},
             "origin":{"/encoding/y":{"kind":"fact","fact":"lvl-12345","field":"level"}}}}
        ]),
    );
    applied(&r);

    // R18: MACD panel above RSI.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r18",
        json!([{"op":"move_panel","view":{},"panel":{"shows":{"study":"macd"}},
            "before":{"shows":{"study":"rsi"}}}]),
    );
    applied(&r);
    assert_eq!(panel_order(&h), vec![0, 2, 1]);

    // R19: how the phone shows the latest breakout.
    let r = h.query(
        "a",
        json!({"kind":"present","finding":{"kind":"breakout","latest":true},"surface":"phone"}),
    );
    assert_eq!(r["data"]["presentation"]["depth"], "capsule");

    // R20: band width as a line in its own panel.
    let mut h = setup();
    let r = h.cmd(
        "a",
        "r20",
        json!([{"op":"insert_panel","view":{},"panel":{"y":{"kind":"linear"}},
            "layers":[{"mark":"line","series":{"study":"bollinger"},"encoding":{"y":"bandwidth"}}]}]),
    );
    applied(&r);
    h.assert_replay();
}

#[test]
fn ambiguous_lists_ranked_candidates_and_first_latest_pick() {
    let mut h = setup();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"approve_finding","finding":{"kind":"breakout"}}]),
    );
    assert_eq!(code(&r), "ambiguous");
    assert_eq!(r["error"]["path"], "/ops/0/finding");
    let ids: Vec<&str> = r["error"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    // Findings are ordered by their latest fact bar: old (120) before new (185).
    assert_eq!(ids, ["n-old", "n-new"]);
    assert!(h.s.log().len() == 1, "nothing applied");
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"approve_finding","finding":{"kind":"breakout","first":true}}]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/0/finding"], "n-old");
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"set_encoding","view":{},"layer":{"mark":"line"},"encoding":{"width":2}}]),
    );
    assert_eq!(code(&r), "ambiguous");
    let c = &r["error"]["candidates"];
    assert_eq!(c[0]["id"], "l1");
    assert_eq!(c[1]["id"], "l2");
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"approve_finding","finding":{"kind":"breakout","first":true,"latest":true}}]),
    );
    assert_eq!(code(&r), "invalid");
}

#[test]
fn not_found_and_schema_errors_explain_themselves() {
    let mut h = setup();
    // A typo in a value: did_you_mean from the document's values.
    let r = h.query(
        "a",
        json!({"kind":"study","series":{"study":"bolinger"},"reduce":"last"}),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/query/series");
    assert_eq!(r["error"]["did_you_mean"], json!(["bollinger"]));
    // Nothing matches all keys: the closest items, most keys satisfied first.
    let r = h.query(
        "a",
        json!({"kind":"study","series":{"study":"rsi","params":{"period":9}},"reduce":"last"}),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["candidates"][0]["id"], "rsi14");
    // A nested selector that matches nothing is reported where it is.
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"delete_layer","view":{},"layer":{"shows":{"study":"ema"}}}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/layer/shows");
    // Unknown selector key: schema, with did_you_mean, at the key.
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"delete_layer","view":{},"layer":{"shows":{"stdy":"rsi"}}}]),
    );
    assert_eq!(code(&r), "schema");
    assert_eq!(r["error"]["path"], "/ops/0/layer/shows/stdy");
    assert_eq!(r["error"]["did_you_mean"], json!(["study"]));
    let r = h.query("a", json!({"kind":"get","id":{"findings":{}}}));
    assert_eq!(code(&r), "schema");
    assert_eq!(r["error"]["did_you_mean"], json!(["finding"]));
    // A view that does not exist, named by a later layer selector's op.
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"delete_layer","view":"nope","layer":{"mark":"rule"}}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/view");
}

#[test]
fn the_log_and_proposals_hold_ids_only() {
    let mut h = setup();
    h.hello("s", "scout", "headless");
    let r = h.cmd(
        "s",
        "p1",
        json!([{"op":"set_encoding","view":{},"layer":{"shows":{"study":"rsi"}},"encoding":{"width":3}}]),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    assert_eq!(r["resolved"]["/ops/0/layer"], "l2");
    let d = doc(&h);
    let op = &d["proposals"][0]["ops"][0];
    assert_eq!(op["view"], "main");
    assert_eq!(op["layer"], "l2");
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"accept_proposal","proposal":{"state":"pending","latest":true}}]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/0/proposal"], "prop-2");
    for e in h.s.log() {
        let text = serde_json::to_string(&e.request).unwrap();
        assert!(
            !text.contains("\"study\":\"rsi\"}"),
            "selector in the log: {text}"
        );
    }
    h.assert_replay();
}

#[test]
fn a_selector_may_name_what_an_earlier_op_created() {
    let mut h = setup();
    let r = h.cmd(
        "a",
        "c1",
        json!([
            {"op":"add_study","study":"ema","inputs":[{"kind":"bars"}],"params":{"period":10},
             "show":{"view":{},"panel":{"mark":"candle"}}},
            {"op":"set_encoding","view":{},"layer":{"shows":{"study":"ema"}},"encoding":{"width":2}}
        ]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/1/layer"], "l5");
    h.assert_replay();
}

#[test]
fn subscribe_and_attend_resolve_too() {
    let mut h = setup();
    let r = h.one(
        "a",
        json!({"type":"subscribe","id":"s1","topic":{"kind":"finding","finding":{"kind":"breakout","latest":true}}}),
    );
    assert_eq!(r["status"], "subscribed");
    assert_eq!(r["resolved"]["/topic/finding"], "n-new");
    let r = h.one(
        "a",
        json!({"type":"attend","id":"a1","directive":{"to":"arkin","steps":[
            {"target":{"view":{},"layer":{"shows":{"study":"rsi"}},"series":{"study":"rsi"}},"say":"Here."}]}}),
    );
    assert_eq!(r["status"], "delivered", "{r}");
    assert_eq!(r["resolved"]["/directive/steps/0/target/layer"], "l2");
}

#[test]
fn a_feed_cannot_use_selectors() {
    let mut h = setup();
    h.hello("f", "feed", "headless");
    let r = h.cmd(
        "f",
        "c1",
        json!([{"op":"append_bars","series":{"kind":"bars"},"bars":[]}]),
    );
    assert_eq!(code(&r), "forbidden");
    assert_eq!(r["error"]["path"], "/ops/0/series");
}

#[test]
fn welcome_hints_resolve_to_their_items() {
    let mut h = setup();
    let w = h.hello("b", "arkin", "desktop");
    let sel = &w["summary"]["select"];
    assert_eq!(sel["series"]["s-50"], json!({"study":"sma"}));
    assert_eq!(sel["panels"]["main/0"], json!({"mark":"candle"}));
    assert_eq!(
        sel["layers"]["main/l3"],
        json!({"mark":"rule","value":70.0})
    );
    assert_eq!(
        sel["findings"]["n-new"],
        json!({"kind":"breakout","latest":true})
    );
    assert_eq!(
        sel["facts"]["f-old"],
        json!({"kind":"breakout","first":true})
    );
    // Every hint resolves to its own item.
    for (id, s) in sel["series"].as_object().unwrap() {
        let r = h.query("b", json!({"kind":"get","id":{"series":s}}));
        assert_eq!(r["data"]["item"]["id"], *id, "{s}");
    }
    for (id, s) in sel["findings"].as_object().unwrap() {
        let r = h.query("b", json!({"kind":"get","id":{"finding":s}}));
        assert_eq!(r["data"]["item"]["id"], *id, "{s}");
    }
}

#[test]
fn split_then_style_the_upper_band_in_one_command() {
    let mut h = setup();
    let before = serde_json::to_vec(h.s.document()).unwrap();
    let r = h.cmd(
        "a",
        "c1",
        json!([
            {"op":"split_layer","view":{},"layer":{"shows":{"study":"bollinger"}}},
            {"op":"set_encoding","view":{},"layer":{"output":"upper"},"encoding":{"color":"down"}}
        ]),
    );
    applied(&r);
    assert_eq!(r["created"], json!(["l5", "l6", "l7"]));
    assert_eq!(r["resolved"]["/ops/1/layer"], "l5");
    let ls = layers(&h);
    let ids: Vec<&str> = ls.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["l0", "l5", "l6", "l7", "l2", "l3", "l4"]);
    assert_eq!(ls[1]["encoding"], json!({"y":"upper","color":"down"}));
    assert_eq!(ls[2]["encoding"], json!({"y":"middle","color":"#90a4ae"}));
    assert_eq!(ls[3]["encoding"], json!({"y":"lower","color":"#90a4ae"}));
    assert!(ls[1..4]
        .iter()
        .all(|l| l["panel"] == 0 && l["series"] == "bb20"));
    h.assert_replay();
    // Exact inverse: undo restores the bytes.
    let r = h.cmd("a", "u", json!([{"op":"undo"}]));
    applied(&r);
    assert_eq!(serde_json::to_vec(h.s.document()).unwrap(), before);
    h.assert_replay();
}

#[test]
fn output_matches_only_a_layer_drawing_it_alone() {
    let mut h = setup();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"set_encoding","view":{},"layer":{"output":"upper"},"encoding":{"color":"down"}}]),
    );
    assert_eq!(code(&r), "invalid", "{r}");
    assert_eq!(r["error"]["candidates"][0]["id"], "l1");
    let hint = r["error"]["hint"].as_str().unwrap();
    assert!(
        hint.contains("\"op\":\"split_layer\",\"view\":\"main\",\"layer\":\"l1\""),
        "{hint}"
    );
    assert_eq!(h.s.log().len(), 1, "nothing applied");
    // A single-output layer still matches by its default output.
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"set_encoding","view":{},"layer":{"output":"rsi"},"encoding":{"width":2}}]),
    );
    applied(&r);
    assert_eq!(r["resolved"]["/ops/0/layer"], "l2");
    // Only multi-output line layers split.
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"split_layer","view":{},"layer":{"shows":{"study":"rsi"}}}]),
    );
    assert_eq!(code(&r), "invalid");
}

#[test]
fn screen_lists_panels_in_stack_order() {
    let mut h = setup();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"move_panel","view":{},"panel":{"shows":{"study":"macd"}},
            "before":{"shows":{"study":"rsi"}}}]),
    );
    applied(&r);
    let r = h.query("a", json!({"kind":"screen"}));
    let order: Vec<u64> = r["data"]["screens"][0]["panels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["panel"].as_u64().unwrap())
        .collect();
    assert_eq!(order, vec![0, 2, 1]);
}
