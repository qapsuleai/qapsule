//! `add_program`, `set_program_params`, `remove_program` through the command path.

use serde_json::json;

use crate::rig::{code, reason, H, SIGNAL_TEXT, SMA_TEXT};

#[test]
fn add_writes_one_series_per_output_with_the_program_inline() {
    let mut h = H::new();
    let r = h.add("p1", SMA_TEXT);
    assert_eq!(r["status"], "applied", "{r}");
    let created: Vec<&str> = r["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(&created[..2], ["p1.avg", "p1.up"], "series ids first: {r}");
    assert_eq!(created.len(), 4, "then one layer per output: {r}");
    let s = h.series("p1.avg");
    let src = &s["source"];
    assert_eq!(src["kind"], "study");
    assert_eq!(src["study"], "program");
    assert_eq!(src["inputs"], json!(["px"]));
    assert_eq!(src["output"], "avg");
    let p = &src["program"];
    assert_eq!(p["instance"], "p1");
    assert_eq!(p["scope"], "document");
    assert_eq!(p["owner"], "arkin");
    assert_eq!(p["since_t"], 1_760_000_000_000_i64);
    assert_eq!(p["params"], json!({"len": 5}));
    assert_eq!(p["ir"]["version"], "qp-ir/0");
    assert!(p["program_id"].as_str().unwrap().starts_with("sha256:"));
    assert_eq!(s["instrument"], h.series("px")["instrument"]);
    // Layers: series → line, event → bar, in a new panel, labelled with the series id.
    let ls = h.layers();
    let avg = ls.iter().find(|l| l["series"] == "p1.avg").unwrap();
    let up = ls.iter().find(|l| l["series"] == "p1.up").unwrap();
    assert_eq!(avg["mark"], "line");
    assert_eq!(up["mark"], "bar");
    assert_eq!(avg["label"], "p1.avg");
    assert_eq!(avg["panel"], up["panel"]);
    h.assert_replay();
}

#[test]
fn an_assigned_instance_id_and_two_instances_of_one_program() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let seq = r["seq"].as_u64().unwrap();
    assert_eq!(r["created"][0], format!("prog-{seq}-0.avg"));
    let r2 = h.cmd(
        "a",
        json!([{"op":"add_program","program":ir,"inputs":["px"],"params":{"len":9}}]),
    );
    assert_eq!(r2["status"], "applied", "{r2}");
    let (a, b) = (
        h.series(r["created"][0].as_str().unwrap()),
        h.series(r2["created"][0].as_str().unwrap()),
    );
    assert_eq!(
        a["source"]["program"]["program_id"],
        b["source"]["program"]["program_id"]
    );
    assert_ne!(
        a["source"]["program"]["instance_id"],
        b["source"]["program"]["instance_id"]
    );
    // The same instance id again: duplicate.
    let id = r["created"][0]
        .as_str()
        .unwrap()
        .trim_end_matches(".avg")
        .to_owned();
    let r3 = h.cmd(
        "a",
        json!([{"op":"add_program","id":id,"program":ir,"inputs":["px"]}]),
    );
    assert_eq!(code(&r3), "duplicate", "{r3}");
    h.assert_replay();
}

#[test]
fn set_params_keeps_ids_and_layers_with_a_new_instance() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let before = h.series("p1.avg");
    let layers = h.layers();
    h.clock(1_760_000_100_000);
    let r = h.cmd(
        "a",
        json!([{"op":"set_program_params","program":"p1","params":{"len":7}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let after = h.series("p1.avg");
    let (pb, pa) = (&before["source"]["program"], &after["source"]["program"]);
    assert_eq!(pa["params"], json!({"len": 7}));
    assert_eq!(pa["ir"], pb["ir"]);
    assert_eq!(pa["program_id"], pb["program_id"]);
    assert_ne!(pa["instance_id"], pb["instance_id"]);
    assert_eq!(pa["since_t"], 1_760_000_100_000_i64);
    assert_eq!(h.series("p1.up")["source"]["program"], *pa);
    assert_eq!(h.layers(), layers);
    // Out of range and unknown keys: refused, nothing changes.
    for (params, sub) in [
        (json!({"len": 99}), "E_PARAM_RANGE"),
        (json!({"lne": 3}), "E_UNKNOWN_PARAM"),
    ] {
        let r = h.cmd(
            "a",
            json!([{"op":"set_program_params","program":"p1","params":params}]),
        );
        assert_eq!(reason(&r), sub, "{r}");
        assert_eq!(r["error"]["path"], "/ops/0/params", "{r}");
    }
    let r = h.cmd(
        "a",
        json!([{"op":"set_program_params","program":"p2","params":{}}]),
    );
    assert_eq!(code(&r), "not_found", "{r}");
    assert_eq!(r["error"]["did_you_mean"], json!(["p1"]), "{r}");
    h.assert_replay();
}

#[test]
fn remove_takes_series_and_layers_unless_something_rests_on_them() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let n_layers = h.layers().len();
    let r = h.cmd(
        "a",
        json!([{"op":"add_intent","id":"i1","text":"breakout",
                "trigger":{"kind":"threshold","series":"p1.up","op":"ge","value":1}}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.cmd("a", json!([{"op":"remove_program","program":"p1"}]));
    assert_eq!(code(&r), "in_use", "{r}");
    assert_eq!(r["error"]["expected"], json!(["i1"]), "{r}");
    assert_eq!(
        h.cmd("a", json!([{"op":"remove","id":"i1"}]))["status"],
        "applied"
    );
    let r = h.cmd("a", json!([{"op":"remove_program","program":"p1"}]));
    assert_eq!(r["status"], "applied", "{r}");
    assert!(h.series("p1.avg").is_null() && h.series("p1.up").is_null());
    assert_eq!(h.layers().len(), n_layers - 2);
    // Undo brings the instance back, byte-equal by replay.
    let seq = r["seq"].as_u64().unwrap();
    assert_eq!(
        h.cmd("a", json!([{"op":"undo","seq":seq}]))["status"],
        "applied"
    );
    assert!(!h.series("p1.avg").is_null());
    h.assert_replay();
}

#[test]
fn signal_and_state_outputs_are_try_only() {
    let mut h = H::new();
    let ir = h.ir(SIGNAL_TEXT);
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","id":"s1","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(reason(&r), "E_TRY_ONLY", "{r}");
    assert_eq!(code(&r), "unsupported", "{r}");
    assert!(
        r["error"]["path"]
            .as_str()
            .unwrap()
            .starts_with("/ops/0/program/outputs/0"),
        "{r}"
    );
    assert!(h.series("s1.dip").is_null());
}

#[test]
fn every_program_error_at_once_and_inputs() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    let mut bad = ir.clone();
    bad["nodes"][1]["op"] = json!("smaa");
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","program":bad,"inputs":["rsi14"],"params":{"len":99}}]),
    );
    assert_eq!(reason(&r), "E_NOT_BAR_SERIES", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/inputs/0", "{r}");
    let subs: Vec<&str> = r["error"]["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["reason"].as_str().unwrap())
        .collect();
    assert!(
        subs.contains(&"E_UNKNOWN_OP") && subs.contains(&"E_PARAM_RANGE"),
        "{subs:?}"
    );
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","program":ir,"inputs":["px","px"]}]),
    );
    assert_eq!(reason(&r), "E_NOT_BAR_SERIES", "{r}");
    assert_eq!(h.s.log().len(), 0);
}

#[test]
fn program_series_are_written_only_by_program_ops() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let mut s = h.series("p1.avg");
    s["id"] = json!("forged");
    for (ops, path) in [
        (json!([{"op":"put_series","series":s}]), "/ops/0/series"),
        (
            json!([{"op":"set_params","series":"p1.avg","params":{"period":3}}]),
            "/ops/0/series",
        ),
        (json!([{"op":"remove","id":"p1.up"}]), "/ops/0/id"),
    ] {
        let r = h.cmd("a", ops);
        assert_eq!(reason(&r), "E_PROGRAM_SERIES", "{r}");
        assert_eq!(r["error"]["path"], path, "{r}");
    }
    let mut plain = h.series("rsi14");
    plain["id"] = json!("x");
    plain["source"]["study"] = json!("program");
    let r = h.cmd("a", json!([{"op":"put_series","series":plain}]));
    assert_eq!(reason(&r), "E_PROGRAM_SERIES", "{r}");
}

#[test]
fn an_event_series_enters_conditions_only_as_threshold() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let cmp = json!({"kind":"compare","left":"rsi14","op":"gt","right":"p1.up"});
    let r = h.cmd("a", json!([{"op":"add_intent","text":"x","trigger":cmp}]));
    assert_eq!(reason(&r), "E_EVENT_OPERAND", "{r}");
    assert_eq!(code(&r), "unsupported", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/trigger/right", "{r}");
    // A series output may be compared.
    let ok = json!({"kind":"compare","left":"p1.avg","op":"gt","right":"rsi14"});
    let r = h.cmd("a", json!([{"op":"add_intent","text":"x","trigger":ok}]));
    assert_eq!(r["status"], "applied", "{r}");
}
