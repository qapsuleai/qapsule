//! Grammar v0 in the authority (R2/T5): canonical form, the ten grammar ops with exact
//! inverses, validation errors, origins, conflict keys, summary and presenter.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use serde_json::{json, Value};
use support::{code, H};

fn owner() -> H {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h
}

fn doc_bytes(h: &H) -> Vec<u8> {
    serde_json::to_vec(h.s.document()).unwrap()
}

fn view(h: &H, id: &str) -> Value {
    let d = serde_json::to_value(h.s.document()).unwrap();
    d["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == id)
        .cloned()
        .unwrap()
}

fn layer_ids(h: &H) -> Vec<String> {
    view(h, "main")["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["id"].as_str().unwrap().to_owned())
        .collect()
}

fn by_id(h: &H, id: &str) -> Value {
    view(h, "main")["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == id)
        .cloned()
        .unwrap()
}

fn last_changes(h: &H) -> Value {
    serde_json::to_value(&h.s.log().last().unwrap().changes).unwrap()
}

/// Applies `ops`, then undo, then redo: undo must restore the bytes, redo must give the
/// bytes after the op again, and every state must replay byte for byte.
fn roundtrip(h: &mut H, ops: Value) -> Value {
    let before = doc_bytes(h);
    let n = h.s.log().len();
    let r = h.cmd("a", &format!("c{n}"), ops.clone());
    assert_eq!(r["status"], "applied", "{ops} -> {r}");
    let after = doc_bytes(h);
    assert_ne!(before, after, "{ops} changed nothing");
    h.assert_replay();
    let seq = r["seq"].as_u64().unwrap();
    let u = h.cmd("a", "u", json!([{"op":"undo"}]));
    assert_eq!(u["status"], "applied", "undo of {ops}: {u}");
    assert_eq!(doc_bytes(h), before, "undo of {ops} is not byte-identical");
    h.assert_replay();
    let useq = u["seq"].as_u64().unwrap();
    let rr = h.cmd("a", "r", json!([{"op":"undo","seq":useq}]));
    assert_eq!(rr["status"], "applied", "redo of {ops}: {rr}");
    assert_eq!(doc_bytes(h), after, "redo of {ops} differs");
    h.assert_replay();
    let _ = seq;
    r
}

#[test]
fn thyao_loads_in_canonical_form() {
    let h = owner();
    let v = view(&h, "main");
    assert!(v.get("y").is_none(), "y moved into panels");
    assert_eq!(v["x"], json!({"kind":"time","id":"x"}));
    assert_eq!(
        v["panels"],
        json!([
            {"id":0,"y":{"kind":"linear","id":"y0"}},
            {"id":1,"y":{"kind":"linear","domain":[0.0,100.0],"id":"y1"}},
            {"id":2,"y":{"kind":"linear","id":"y2"}}
        ])
    );
    assert_eq!(layer_ids(&h), ["l0", "l1", "l2", "l3", "l4"]);
    assert_eq!(
        v["layers"][3],
        json!({"id":"l3","mark":"rule","panel":1,"encoding":{"y":"70","color":"#9e9e9e"},
               "origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbought"}}})
    );
    // The initial document of the log is the canonical one.
    assert_eq!(
        serde_json::to_vec(h.s.initial_document()).unwrap(),
        doc_bytes(&h)
    );
}

#[test]
fn a_document_with_an_untraceable_constant_does_not_load() {
    let mut init = support::init();
    let mut doc: Value = serde_json::from_str(
        &std::fs::read_to_string(support::root().join("fixtures/protocol/docs/thyao.json"))
            .unwrap(),
    )
    .unwrap();
    doc["views"][0]["layers"][3]["encoding"]["y"] = json!("71");
    init.document =
        qp_schema::protocol::DocumentRef::Inline(Box::new(serde_json::from_value(doc).unwrap()));
    let Err(e) = qp_session::Session::new(init, &support::root()) else {
        panic!("loaded");
    };
    assert!(e.contains("no origin"), "{e}");
    assert!(e.contains("/views/0/layers/3/encoding/y"), "{e}");
}

#[test]
fn insert_layer_by_id_with_undo_and_redo() {
    let mut h = owner();
    let r = roundtrip(
        &mut h,
        json!([{"op":"insert_layer","view":"main","after":"l1",
                "layer":{"mark":"line","series":"bb20","panel":0,"encoding":{"y":"upper","color":"down"}}}]),
    );
    assert_eq!(r["created"], json!(["l5"]));
    assert_eq!(
        last_changes(&h)[0]["op"],
        "insert_layer",
        "the log's last entry is the redo"
    );
    assert_eq!(layer_ids(&h), ["l0", "l1", "l5", "l2", "l3", "l4"]);
    let r = roundtrip(
        &mut h,
        json!([{"op":"insert_layer","view":"main","before":"l0",
                "layer":{"id":"vol","mark":"bar","series":"px","panel":0,"encoding":{"y":"v"}}}]),
    );
    assert_eq!(r["created"], json!(["vol"]));
    assert_eq!(layer_ids(&h)[0], "vol");
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"insert_layer","view":"main","after":"l1","before":"l2","layer":{"mark":"line","series":"bb20"}}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/before");
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"insert_layer","view":"main","after":"l9","layer":{"mark":"line","series":"bb20"}}]),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/after");
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"insert_layer","view":"main","layer":{"id":"l1","mark":"line","series":"bb20"}}]),
    );
    assert_eq!(code(&r), "duplicate");
    assert_eq!(r["error"]["path"], "/ops/0/layer/id");
}

#[test]
fn delete_layer_by_id_with_undo_and_redo() {
    let mut h = owner();
    roundtrip(
        &mut h,
        json!([{"op":"delete_layer","view":"main","layer":"l3"}]),
    );
    assert_eq!(
        h.s.log()[0].changes.len(),
        1,
        "one delete_layer change with index and id"
    );
    assert_eq!(
        serde_json::to_value(&h.s.log()[0].changes[0]).unwrap(),
        json!({"op":"delete_layer","view":"main","index":3,"layer":"l3"})
    );
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"delete_layer","view":"main","layer":"l3"}]),
    );
    assert_eq!(code(&r), "not_found", "already deleted (redo)");
    assert_eq!(r["error"]["path"], "/ops/0/layer");
}

#[test]
fn move_layer_and_set_encoding() {
    let mut h = owner();
    roundtrip(
        &mut h,
        json!([{"op":"move_layer","view":"main","layer":"l1","after":"l3"}]),
    );
    roundtrip(
        &mut h,
        json!([{"op":"move_layer","view":"main","layer":"l4","panel":1}]),
    );
    assert_eq!(by_id(&h, "l4")["panel"], 1);
    roundtrip(
        &mut h,
        json!([{"op":"set_encoding","view":"main","layer":"l1","encoding":{"color":"accent","width":2.0},"label":"BB"}]),
    );
    let l1 = &by_id(&h, "l1");
    assert_eq!(l1["encoding"]["color"], "accent");
    assert_eq!(l1["encoding"]["width"], 2.0);
    assert_eq!(l1["label"], "BB");
    assert_eq!(last_changes(&h)[0]["op"], "put_layer");
    // Changing the rule's constant drops its old origin and binds the new value.
    roundtrip(
        &mut h,
        json!([{"op":"set_encoding","view":"main","layer":"l3","encoding":{"y":30}}]),
    );
    assert_eq!(
        by_id(&h, "l3")["origin"],
        json!({"/encoding/y":{"kind":"threshold","name":"rsi.oversold"}})
    );
    roundtrip(
        &mut h,
        json!([{"op":"set_encoding","view":"main","layer":"l2","unset":["color"]}]),
    );
    assert!(by_id(&h, "l2")["encoding"].get("color").is_none());
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"set_encoding","view":"main","layer":"l2","unset":["colour"]}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["did_you_mean"], json!(["color"]));
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"move_layer","view":"main","layer":"l2","after":"l2"}]),
    );
    assert_eq!(code(&r), "invalid");
}

#[test]
fn panels_by_id_with_undo_and_redo() {
    let mut h = owner();
    let r = roundtrip(
        &mut h,
        json!([{"op":"insert_panel","view":"main","after":0,"panel":{"name":"volume","y":{"kind":"linear"}},
                "layers":[{"mark":"bar","series":"px","encoding":{"y":"v"}},
                          {"mark":"line","series":"px","encoding":{"y":"v","color":"s2"}}]}]),
    );
    assert_eq!(r["created"], json!(["3", "l5", "l6"]));
    let v = view(&h, "main");
    assert_eq!(
        v["panels"][1],
        json!({"id":3,"name":"volume","y":{"kind":"linear","id":"y3"}})
    );
    assert_eq!(v["layers"][5]["panel"], 3);
    let ch = last_changes(&h);
    assert_eq!(ch[0]["op"], "put_panel");
    assert_eq!(ch[1]["op"], "insert_layer");
    roundtrip(
        &mut h,
        json!([{"op":"delete_panel","view":"main","panel":1}]),
    );
    let ch = serde_json::to_value(&h.s.log()[h.s.log().len() - 3].changes).unwrap();
    assert_eq!(
        ch,
        json!([{"op":"delete_layer","view":"main","index":3,"layer":"l3"},
               {"op":"delete_layer","view":"main","index":2,"layer":"l2"},
               {"op":"delete_panel","view":"main","panel":1}])
    );
    roundtrip(
        &mut h,
        json!([{"op":"move_panel","view":"main","panel":2,"before":0}]),
    );
    assert_eq!(view(&h, "main")["panels"][0]["id"], 2);
    roundtrip(
        &mut h,
        json!([{"op":"set_panel","view":"main","panel":3,"name":"vol","weight":1.5}]),
    );
    roundtrip(
        &mut h,
        json!([{"op":"set_scale","view":"main","scale":"y0","kind":"log"}]),
    );
    roundtrip(
        &mut h,
        json!([{"op":"set_scale","view":"main","scale":"y3","domain":[0.0,1.0e9]}]),
    );
    roundtrip(
        &mut h,
        json!([{"op":"set_scale","view":"main","scale":"x","domain":[1752710400000.0,1759881600000.0]}]),
    );
    assert_eq!(last_changes(&h)[0]["op"], "set_x_domain");
    roundtrip(
        &mut h,
        json!([{"op":"set_emphasis","view":"main","emphasis":[
            {"target":{"item":"fnd-overbought"},"kind":"highlight"},
            {"target":{"layer":"l5"},"kind":"focus"}]}]),
    );
    assert_eq!(last_changes(&h)[0]["op"], "set_emphasis");
}

#[test]
fn panel_and_scale_errors() {
    let mut h = owner();
    let cases = [
        (
            json!({"op":"insert_panel","view":"main","panel":{"id":1,"y":{"kind":"linear"}}}),
            "duplicate",
            "/ops/0/panel/id",
        ),
        (
            json!({"op":"insert_panel","view":"main","panel":{"y":{"kind":"time"}}}),
            "invalid",
            "/ops/0/panel/y/kind",
        ),
        (
            json!({"op":"insert_panel","view":"main","panel":{"weight":0.0,"y":{"kind":"linear"}}}),
            "invalid",
            "/ops/0/panel/weight",
        ),
        (
            json!({"op":"delete_panel","view":"main","panel":7}),
            "not_found",
            "/ops/0/panel",
        ),
        (
            json!({"op":"set_panel","view":"main","panel":0}),
            "invalid",
            "/ops/0",
        ),
        (
            json!({"op":"set_scale","view":"main","scale":"x","kind":"linear"}),
            "invalid",
            "/ops/0/kind",
        ),
        (
            json!({"op":"set_scale","view":"main","scale":"y0","kind":"time"}),
            "invalid",
            "/ops/0/kind",
        ),
        (
            json!({"op":"set_scale","view":"main","scale":"y1","kind":"log"}),
            "invalid",
            "/ops/0/domain",
        ),
        (
            json!({"op":"set_scale","view":"main","scale":"y9","auto_domain":true}),
            "not_found",
            "/ops/0/scale",
        ),
        (
            json!({"op":"set_scale","view":"main","scale":"y0","domain":[5.0,1.0]}),
            "invalid",
            "/ops/0/domain",
        ),
        (
            json!({"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l9"},"kind":"dim"}]}),
            "not_found",
            "/ops/0/emphasis/0/target/layer",
        ),
        (
            json!({"op":"set_emphasis","view":"main","emphasis":[{"target":{"panel":5},"kind":"dim"}]}),
            "not_found",
            "/ops/0/emphasis/0/target/panel",
        ),
    ];
    for (op, c, path) in cases {
        let r = h.cmd("a", "x", json!([op]));
        assert_eq!(code(&r), c, "{op} -> {r}");
        assert_eq!(r["error"]["path"], path, "{op} -> {r}");
    }
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"set_scale","view":"main","scale":"y5"}]),
    );
    assert_eq!(code(&r), "invalid", "nothing to change");
    // The last panel cannot go.
    let r = h.cmd(
        "a",
        "v",
        json!([{"op":"put_view","view":{"id":"one","x":{"kind":"time"},"layers":[{"mark":"candle","series":"px"}]}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"delete_panel","view":"one","panel":0}]),
    );
    assert_eq!(code(&r), "invalid");
}

#[test]
fn panel_and_layer_limits() {
    let mut h = owner();
    let ops: Vec<Value> = (0..13)
        .map(|_| json!({"op":"insert_panel","view":"main","panel":{"y":{"kind":"linear"}}}))
        .collect();
    let r = h.cmd("a", "p", json!(ops));
    assert_eq!(r["status"], "applied", "16 panels fit: {r}");
    let r = h.cmd(
        "a",
        "p2",
        json!([{"op":"insert_panel","view":"main","panel":{"y":{"kind":"linear"}}}]),
    );
    assert_eq!(code(&r), "limit");
    assert_eq!(r["error"]["expected"], json!(["16"]));
    let r = h.cmd(
        "a",
        "p3",
        json!([{"op":"add_study","study":"sma","inputs":["px"],"show":{"view":"main"}}]),
    );
    assert_eq!(
        code(&r),
        "limit",
        "a new panel from add_study is capped too"
    );
    let ops: Vec<Value> = (0..59)
        .map(|_| json!({"op":"insert_layer","view":"main","layer":{"mark":"line","series":"px","panel":0}}))
        .collect();
    let r = h.cmd("a", "l", json!(ops));
    assert_eq!(r["status"], "applied", "64 layers fit: {r}");
    let r = h.cmd(
        "a",
        "l2",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"line","series":"px","panel":0}}]),
    );
    assert_eq!(code(&r), "limit");
    assert_eq!(r["error"]["expected"], json!(["64"]));
}

#[test]
fn a_layer_cannot_create_panels_from_large_numbers() {
    let mut h = owner();
    for (op, path) in [
        (
            json!({"op":"add_layer","view":"main","layer":{"mark":"line","series":"px","panel":4_000_000_000_u32}}),
            "/ops/0/layer/panel",
        ),
        (
            json!({"op":"insert_layer","view":"main","layer":{"mark":"line","series":"px","panel":9}}),
            "/ops/0/layer/panel",
        ),
        (
            json!({"op":"put_view","view":{"id":"v9","x":{"kind":"time"},"y":[{"kind":"linear"}],
                "layers":[{"mark":"candle","series":"px","panel":4_000_000_000_u32}]}}),
            "/ops/0/view/layers/0/panel",
        ),
    ] {
        let r = h.cmd("a", "x", json!([op]));
        assert_eq!(code(&r), "not_found", "{op} -> {r}");
        assert_eq!(r["error"]["path"], path);
    }
    let r = h.cmd(
        "a",
        "x",
        json!([{"op":"add_study","study":"sma","inputs":["px"],"show":{"view":"main","panel":4_000_000_000_u32}}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/show/panel");
    assert_eq!(view(&h, "main")["panels"].as_array().unwrap().len(), 3);
}

#[test]
fn layer_validation_errors() {
    let mut h = owner();
    let ins = |layer: Value| json!([{"op":"insert_layer","view":"main","layer":layer}]);
    let cases = [
        (
            json!({"mark":"line","series":"rsi","panel":1}),
            "not_found",
            "/ops/0/layer/series",
        ),
        (
            json!({"mark":"line","series":"px","fact":"fact-rsi-cross70"}),
            "invalid",
            "/ops/0/layer/fact",
        ),
        (
            json!({"mark":"point","fact":"fact-x"}),
            "not_found",
            "/ops/0/layer/fact",
        ),
        (
            json!({"mark":"candle","series":"rsi14","panel":1}),
            "invalid",
            "/ops/0/layer/series",
        ),
        (
            json!({"mark":"band","series":"bb20","encoding":{"y":"upper"}}),
            "invalid",
            "/ops/0/layer/encoding",
        ),
        (
            json!({"mark":"rule","panel":1,"encoding":{"color":"muted"}}),
            "invalid",
            "/ops/0/layer/encoding/y",
        ),
        (
            json!({"mark":"text","fact":"fact-rsi-cross70","panel":1}),
            "invalid",
            "/ops/0/layer/encoding",
        ),
        (
            json!({"mark":"line","series":"bb20","encoding":{"y":"uper"}}),
            "invalid",
            "/ops/0/layer/encoding/y",
        ),
        (
            json!({"mark":"line","series":"bb20","encoding":{"color":"#12345"}}),
            "invalid",
            "/ops/0/layer/encoding/color",
        ),
        (
            json!({"mark":"line","series":"bb20","encoding":{"color":"dwn"}}),
            "invalid",
            "/ops/0/layer/encoding/color",
        ),
        (
            json!({"mark":"area","series":"px","encoding":{"opacity":1.5}}),
            "invalid",
            "/ops/0/layer/encoding/opacity",
        ),
        (
            json!({"mark":"line","series":"px","encoding":{"width":0.0}}),
            "invalid",
            "/ops/0/layer/encoding/width",
        ),
        (
            json!({"mark":"point","series":"px","encoding":{"size":-1.0}}),
            "invalid",
            "/ops/0/layer/encoding/size",
        ),
        (
            json!({"mark":"line","series":"px","encoding":{"y":{"series":"bb20","field":"mid"}}}),
            "invalid",
            "/ops/0/layer/encoding/y/field",
        ),
        (
            json!({"mark":"line","series":"px","encoding":{"y":{"series":"bb","field":"middle"}}}),
            "not_found",
            "/ops/0/layer/encoding/y/series",
        ),
        (
            json!({"mark":"line","series":"rsi14","panel":1,"encoding":{"color":{"field":"rsi","thresholds":[70,30],"colors":["up","muted","down"]}}}),
            "invalid",
            "/ops/0/layer/encoding/color/thresholds/1",
        ),
        (
            json!({"mark":"line","series":"rsi14","panel":1,"encoding":{"color":{"field":"rsi","thresholds":[30,70],"colors":["up","down"]}}}),
            "invalid",
            "/ops/0/layer/encoding/color/colors",
        ),
        (
            json!({"mark":"line","series":"bb20","origin":{"/encoding/z":{"kind":"threshold","name":"rsi.mid"}}}),
            "invalid",
            "/ops/0/layer/origin/~1encoding~1z",
        ),
        (
            json!({"mark":"rule","panel":1,"encoding":{"y":71},"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbought"}}}),
            "invalid",
            "/ops/0/layer/encoding/y",
        ),
        (
            json!({"mark":"rule","panel":1,"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbougth"}}}),
            "not_found",
            "/ops/0/layer/origin/~1encoding~1y/name",
        ),
        (
            json!({"mark":"rule","panel":1,"origin":{"/encoding/y":{"kind":"fact","fact":"fact-rsi-cross70","field":"lvl"}}}),
            "invalid",
            "/ops/0/layer/origin/~1encoding~1y/field",
        ),
        (
            json!({"mark":"rule","panel":0,"encoding":{"y":124.0},"origin":{"/encoding/y":{"kind":"series","series":"px","field":"c","at":"last"}}}),
            "invalid",
            "/ops/0/layer/encoding/y",
        ),
        (
            json!({"mark":"line","series":"px","encoding":{"y":"c"},"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.mid"}}}),
            "invalid",
            "/ops/0/layer/origin/~1encoding~1y",
        ),
    ];
    for (layer, c, path) in cases {
        let r = h.cmd("a", "x", ins(layer.clone()));
        assert_eq!(code(&r), c, "{layer} -> {r}");
        assert_eq!(r["error"]["path"], path, "{layer} -> {r}");
    }
    let r = h.cmd(
        "a",
        "x",
        ins(json!({"mark":"line","series":"bb20","encoding":{"y":"uper"}})),
    );
    assert_eq!(r["error"]["did_you_mean"], json!(["upper", "lower"]));
    let r = h.cmd(
        "a",
        "x",
        ins(json!({"mark":"line","series":"bb20","encoding":{"color":"dwn"}})),
    );
    assert_eq!(r["error"]["did_you_mean"][0], "down");
    let r = h.cmd(
        "a",
        "x",
        ins(json!({"mark":"line","series":"bb20","panel":5})),
    );
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["path"], "/ops/0/layer/panel");
    assert_eq!(r["error"]["expected"], json!(["0", "1", "2"]));
    assert!(h.s.log().is_empty(), "nothing applied");
}

#[test]
fn unknown_channel_is_a_schema_error_with_suggestion() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"line","series":"bb20","encoding":{"colour":"down"}}}]),
    );
    println!("invalid channel: {}", serde_json::to_string(&r).unwrap());
    assert_eq!(code(&r), "schema");
    assert_eq!(r["error"]["path"], "/ops/0/layer/encoding/colour");
    assert_eq!(r["error"]["did_you_mean"], json!(["color"]));
}

#[test]
fn no_origin_names_the_layer_and_offers_candidates() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "f",
        json!([{"op":"add_fact","id":"f-brk-2","kind":"breakout","series":"px","bar":185,"text":"Kırılım","evidence":{"level":121.3}}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":0,"encoding":{"y":123.45}}}]),
    );
    println!("no_origin: {}", serde_json::to_string(&r).unwrap());
    assert_eq!(code(&r), "no_origin");
    assert_eq!(r["error"]["path"], "/ops/0/layer/encoding/y");
    assert_eq!(
        r["error"]["message"],
        "layer `l5` of view `main`: constant 123.45 at /encoding/y has no origin"
    );
    assert_eq!(
        r["error"]["candidates"][0],
        json!({"id":"{\"kind\":\"fact\",\"fact\":\"f-brk-2\",\"field\":\"level\"}",
               "about":"breakout level 121.3 (bar 185)"})
    );
    assert!(r["error"]["hint"].as_str().unwrap().contains("add_fact"));
    // In the RSI panel the thresholds of the studies drawn there are offered, nearest first.
    let r = h.cmd(
        "a",
        "c5",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":1,"encoding":{"y":65}}}]),
    );
    assert_eq!(code(&r), "no_origin");
    let c = &r["error"]["candidates"];
    assert_eq!(
        c[0]["id"],
        "{\"kind\":\"fact\",\"fact\":\"fact-rsi-cross70\",\"field\":\"prev\"}"
    );
    assert!(c.as_array().unwrap().iter().any(|x| x["id"]
        == "{\"kind\":\"threshold\",\"name\":\"rsi.overbought\"}"
        && x["about"] == "rsi.overbought = 70"));
    // Constants of colour conditions need origins too.
    let r = h.cmd(
        "a",
        "c6",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"line","series":"px","panel":0,
            "encoding":{"color":{"when":{"field":"c","op":"ge","than":120},"then":"up","else":"down"}}}}]),
    );
    assert_eq!(code(&r), "no_origin");
    assert_eq!(r["error"]["path"], "/ops/0/layer/encoding/color/when/than");
    // Bound to the fact, it applies.
    let r = h.cmd(
        "a",
        "c7",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":0,"encoding":{"y":121.3},
            "origin":{"/encoding/y":{"kind":"fact","fact":"f-brk-2","field":"level"}}}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    // The fact cannot be removed while a layer traces a constant to it.
    let r = h.cmd("a", "c8", json!([{"op":"remove","id":"f-brk-2"}]));
    assert_eq!(code(&r), "in_use");
    assert_eq!(r["error"]["expected"], json!(["main"]));
    h.assert_replay();
}

#[test]
fn origins_bind_and_write_constants() {
    let mut h = owner();
    // qp/0 form: the string constant binds to the one matching named threshold.
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"add_layer","view":"main","layer":{"mark":"rule","panel":1,"encoding":{"y":"30"}}}]),
    );
    assert_eq!(r["status"], "applied");
    let l = &view(&h, "main")["layers"][5];
    assert_eq!(l["encoding"]["y"], "30");
    assert_eq!(
        l["origin"],
        json!({"/encoding/y":{"kind":"threshold","name":"rsi.oversold"}})
    );
    // An origin without a constant: the authority writes the value.
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":1,
            "origin":{"/encoding/y":{"kind":"fact","fact":"fact-rsi-cross70","field":"level"}}}}]),
    );
    assert_eq!(r["status"], "applied");
    assert_eq!(view(&h, "main")["layers"][6]["encoding"]["y"], 70.0);
    // A live series origin writes no constant.
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":0,
            "origin":{"/encoding/y":{"kind":"series","series":"px","field":"c","at":"last"}}}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    assert!(view(&h, "main")["layers"][7]["encoding"].get("y").is_none());
    // MACD zero line binds in the MACD panel; in the price panel 0 has no origin.
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"bar","series":"macd","panel":2,
            "encoding":{"y":"histogram","color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(
        view(&h, "main")["layers"][8]["origin"],
        json!({"/encoding/color/when/than":{"kind":"threshold","name":"macd.zero"}})
    );
    let r = h.cmd(
        "a",
        "c5",
        json!([{"op":"insert_layer","view":"main","layer":{"mark":"rule","panel":0,"encoding":{"y":0}}}]),
    );
    assert_eq!(code(&r), "no_origin");
    h.assert_replay();
}

#[test]
fn put_view_is_made_canonical() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"put_view","view":{"id":"v2","x":{"kind":"time"},
            "panels":[{"name":"price","y":{"kind":"log"}},{"id":5,"y":{"kind":"linear","domain":[0,100]}}],
            "layers":[{"mark":"candle","series":"px","panel":6},{"id":"r","mark":"line","series":"rsi14","panel":5},
                      {"mark":"rule","panel":5,"encoding":{"y":50}}]}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let v = view(&h, "v2");
    assert_eq!(v["panels"][0]["id"], 6, "largest id + 1");
    assert_eq!(v["panels"][0]["y"]["id"], "y6");
    assert_eq!(v["panels"][1]["y"]["id"], "y5");
    assert_eq!(v["layers"][0]["id"], "l0");
    assert_eq!(v["layers"][2]["id"], "l1");
    assert_eq!(
        v["layers"][2]["origin"]["/encoding/y"],
        json!({"kind":"threshold","name":"rsi.mid"})
    );
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"put_view","view":{"id":"v3","x":{"kind":"time"},"y":[{"kind":"linear"}],
            "panels":[{"y":{"kind":"linear"}}],"layers":[]}}]),
    );
    assert_eq!(code(&r), "invalid");
    assert_eq!(r["error"]["path"], "/ops/0/view/y");
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"put_view","view":{"id":"v3","x":{"kind":"time"},
            "layers":[{"id":"a","mark":"candle","series":"px"},{"id":"a","mark":"line","series":"px"}]}}]),
    );
    assert_eq!(code(&r), "duplicate");
    assert_eq!(r["error"]["path"], "/ops/0/view/layers/1/id");
    let panels: Vec<Value> = (0..17).map(|_| json!({"y":{"kind":"linear"}})).collect();
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"put_view","view":{"id":"v3","x":{"kind":"time"},"panels":panels,"layers":[]}}]),
    );
    assert_eq!(code(&r), "limit");
    h.assert_replay();
}

#[test]
fn conflicts_are_per_layer_and_per_panel_structure() {
    let mut h = owner();
    h.hello("e", "editor", "headless");
    // Editor writes to layer l1, then to the panel list.
    let r = h.cmd(
        "e",
        "e1",
        json!([{"op":"set_encoding","view":"main","layer":"l1","encoding":{"color":"s3"}}]),
    );
    assert_eq!(r["status"], "applied");
    // A write from base 0 to another layer of the same view applies.
    let r = h.one(
        "a",
        json!({"type":"command","id":"a1","base":0,"ops":[
            {"op":"set_encoding","view":"main","layer":"l2","encoding":{"width":2.0}}]}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.one(
        "a",
        json!({"type":"command","id":"a2","base":0,"ops":[
            {"op":"insert_layer","view":"main","layer":{"mark":"line","series":"px","panel":0}}]}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    // The same layer conflicts.
    let r = h.one(
        "a",
        json!({"type":"command","id":"a3","base":0,"ops":[
            {"op":"set_encoding","view":"main","layer":"l1","encoding":{"width":3.0}}]}),
    );
    assert_eq!(code(&r), "conflict");
    assert_eq!(r["error"]["conflicts"], json!([1]));
    assert!(r["error"]["message"]
        .as_str()
        .unwrap()
        .starts_with("`main/l1` changed after base 0"));
    // Panel structure.
    let r = h.cmd(
        "e",
        "e2",
        json!([{"op":"set_panel","view":"main","panel":2,"name":"macd"}]),
    );
    let pseq = r["seq"].as_u64().unwrap();
    let r = h.one(
        "a",
        json!({"type":"command","id":"a4","base":3,"ops":[
            {"op":"move_panel","view":"main","panel":1,"before":0}]}),
    );
    assert_eq!(code(&r), "conflict");
    assert_eq!(r["error"]["conflicts"], json!([pseq]));
    // A whole-view write conflicts with any layer write after the base.
    let r = h.one(
        "a",
        json!({"type":"command","id":"a5","base":0,"ops":[{"op":"set_view_range","view":"main","from_bar":10,"to_bar":20}]}),
    );
    assert_eq!(code(&r), "conflict");
    // Undo is per layer too: undoing the editor's l1 write is not blocked by a2/a1.
    let r = h.cmd("e", "e3", json!([{"op":"undo","seq":1}]));
    assert_eq!(r["status"], "applied", "{r}");
    h.assert_replay();
}

#[test]
fn undo_follows_layer_ids_after_other_layers_moved() {
    let mut h = owner();
    h.hello("e", "editor", "headless");
    let r = h.cmd(
        "a",
        "a1",
        json!([{"op":"insert_layer","view":"main","layer":{"id":"mine","mark":"line","series":"px","panel":0}}]),
    );
    assert_eq!(r["status"], "applied");
    // Someone else deletes a layer before it: positions shift.
    let r = h.cmd(
        "e",
        "e1",
        json!([{"op":"delete_layer","view":"main","layer":"l0"}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd("a", "a2", json!([{"op":"undo"}]));
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(layer_ids(&h), ["l1", "l2", "l3", "l4"]);
    assert_eq!(
        serde_json::to_value(&h.s.log()[2].changes).unwrap(),
        json!([{"op":"delete_layer","view":"main","index":4,"layer":"mine"}])
    );
    h.assert_replay();
}

#[test]
fn emphasis_blocks_deleting_its_target() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l1"},"kind":"focus"}]}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd(
        "a",
        "c2",
        json!([{"op":"delete_layer","view":"main","layer":"l1"}]),
    );
    assert_eq!(code(&r), "in_use");
    let r = h.cmd("a", "c3", json!([{"op":"remove","id":"bb20"}]));
    assert_eq!(code(&r), "in_use");
    assert_eq!(r["error"]["expected"], json!(["main"]));
    let r = h.cmd(
        "a",
        "c4",
        json!([{"op":"delete_panel","view":"main","panel":0}]),
    );
    assert_eq!(code(&r), "in_use");
}

#[test]
fn describe_lists_thresholds() {
    let mut h = owner();
    let r = h.query("a", json!({"kind":"describe"}));
    assert_eq!(
        r["data"]["description"]["thresholds"],
        json!({"macd.zero":0.0,"rsi.mid":50.0,"rsi.overbought":70.0,"rsi.oversold":30.0})
    );
}

#[test]
fn welcome_summary_of_thyao() {
    let mut h = H::new();
    let w = h.hello("a", "arkin", "desktop");
    let s = &w["summary"];
    println!("summary: {}", serde_json::to_string(s).unwrap());
    assert_eq!(
        s["series"][0],
        json!({"id":"px","kind":"bars","fields":["o","h","l","c","v"],"symbol":"THYAO",
               "interval":"1d","len":200,"label":"THYAO 1d"})
    );
    assert_eq!(s["series"][1]["label"], "RSI14");
    assert_eq!(s["series"][1]["inputs"], json!(["px"]));
    assert_eq!(s["series"][1]["params"], json!({"period":14.0}));
    assert_eq!(s["series"][2]["label"], "MACD(12,26,9)");
    assert_eq!(
        s["series"][2]["fields"],
        json!(["macd", "signal", "histogram"])
    );
    assert_eq!(s["series"][3]["label"], "BOLLINGER(20,2)");
    // Documented order (PROTOCOL §7), not the engine's `middle, upper, …`.
    assert_eq!(
        s["series"][3]["fields"],
        json!(["upper", "middle", "lower", "bandwidth"])
    );
    let p = &s["views"][0]["panels"];
    assert_eq!(p[0]["scale"], "y0 linear");
    assert_eq!(p[1]["scale"], "y1 linear [0, 100]");
    assert_eq!(
        p[0]["layers"],
        json!([{"id":"l0","mark":"candle","about":"px candles"},
               {"id":"l1","mark":"line","about":"bb20.upper/middle/lower line"}])
    );
    assert_eq!(
        p[1]["layers"],
        json!([{"id":"l2","mark":"line","about":"rsi14.rsi line"},
               {"id":"l3","mark":"rule","about":"rule 70 (rsi.overbought)"}])
    );
    assert_eq!(p[2]["layers"][0]["about"], "macd.macd/signal line");
    assert_eq!(s["facts"][0]["id"], "fact-rsi-cross70");
    assert_eq!(
        s["findings"][0],
        json!({"id":"fnd-overbought","claim":"THYAO günlükte RSI(14) aşırı alım bölgesine girdi; momentum güçlü.",
               "facts":["fact-rsi-cross70"],"bar":150,"capsule":"cap-overbought live"})
    );
    assert_eq!(s["pending_proposals"], 0);
    assert!(s.get("truncated").is_none());
    // Deterministic.
    let mut h2 = H::new();
    assert_eq!(h2.hello("a", "arkin", "desktop")["summary"], *s);
}

#[test]
fn presenter_view_is_canonical_with_origin_and_emphasis() {
    let mut h = owner();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"show_finding","finding":"fnd-overbought","view":"vp"}]),
    );
    assert_eq!(r["status"], "applied");
    let v = view(&h, "vp");
    assert_eq!(
        v["layers"][2],
        json!({"id":"level","mark":"rule","panel":1,"encoding":{"y":"70"},
               "origin":{"/encoding/y":{"kind":"fact","fact":"fact-rsi-cross70","field":"level"}}})
    );
    assert_eq!(
        v["emphasis"],
        json!([{"target":{"item":"fact-rsi-cross70"},"kind":"highlight"}])
    );
    assert_eq!(v["panels"].as_array().unwrap().len(), 2);
    let p = h.query("a", json!({"kind":"present","finding":"fnd-overbought"}));
    assert_eq!(p["data"]["presentation"]["view"]["id"], "vp");
    h.assert_replay();
}

#[test]
fn welcome_summary_only_for_readers() {
    let mut h = H::new();
    let w = h.hello("f", "feed", "headless");
    assert_eq!(w["type"], "welcome");
    assert!(w.get("summary").is_none(), "a feed may not read: {w}");
    let w = h.hello("v", "viewer", "headless");
    assert!(w.get("summary").is_some());
}

#[test]
fn an_emphasis_target_never_dangles() {
    let mut h = owner();
    h.hello("s", "scout", "headless");
    h.hello("e", "editor", "headless");
    // A pending proposal to delete l1, then an emphasis on l1: accepting must fail.
    let r = h.cmd(
        "s",
        "p1",
        json!([{"op":"delete_layer","view":"main","layer":"l1"}]),
    );
    assert_eq!(r["status"], "proposed");
    let pid = r["proposal"].as_str().unwrap().to_owned();
    let r = h.cmd(
        "a",
        "c1",
        json!([{"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l1"},"kind":"focus"}]}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd("a", "c2", json!([{"op":"accept_proposal","proposal":pid}]));
    assert_eq!(code(&r), "in_use", "{r}");
    // A concurrent delete from an earlier base conflicts with the emphasis write.
    let r = h.one(
        "e",
        json!({"type":"command","id":"e1","base":1,"ops":[
            {"op":"delete_layer","view":"main","layer":"l2"}]}),
    );
    assert_eq!(code(&r), "conflict", "{r}");
    // Undo cannot restore an emphasis whose target was deleted later.
    let r = h.cmd(
        "a",
        "c3",
        json!([{"op":"set_emphasis","view":"main","emphasis":[]}]),
    );
    assert_eq!(r["status"], "applied");
    let r = h.cmd(
        "e",
        "e2",
        json!([{"op":"delete_layer","view":"main","layer":"l1"}]),
    );
    assert_eq!(r["status"], "applied");
    let seq = h.s.log().len() as u64 - 1;
    let r = h.cmd("a", "c4", json!([{"op":"undo","seq":seq}]));
    assert_eq!(code(&r), "undo_conflict", "{r}");
    // The apply layer refuses dangling targets on its own as well.
    let mut d = h.s.document().clone();
    let bad: qp_schema::protocol::ChangeOp = serde_json::from_value(json!(
        {"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l1"},"kind":"dim"}]}))
    .unwrap();
    assert!(qp_ops::apply_change(&mut d, &bad).is_err());
    let set: qp_schema::protocol::ChangeOp = serde_json::from_value(json!(
        {"op":"set_emphasis","view":"main","emphasis":[{"target":{"layer":"l2"},"kind":"dim"}]}))
    .unwrap();
    qp_ops::apply_change(&mut d, &set).unwrap();
    let del: qp_schema::protocol::ChangeOp = serde_json::from_value(json!(
        {"op":"delete_layer","view":"main","index":1,"layer":"l2"}))
    .unwrap();
    assert!(qp_ops::apply_change(&mut d, &del).is_err());
    h.assert_replay();
}
