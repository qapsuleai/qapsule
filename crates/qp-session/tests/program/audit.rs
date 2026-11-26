//! Audit repros: the guards hold per op against the document the earlier ops of the
//! command left; one bar series for the queries; a warning when the live cost alone is
//! over the per-owner live limit.

use serde_json::json;

use crate::rig::{reason, H, SMA_TEXT};

#[test]
fn guards_see_the_series_made_earlier_in_the_same_command() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    // An event series made by op 0 compared in op 1.
    let cmp = json!({"kind":"compare","left":"rsi14","op":"gt","right":"q1.up"});
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","id":"q1","program":ir,"inputs":["px"]},
               {"op":"add_intent","text":"x","trigger":cmp}]),
    );
    assert_eq!(reason(&r), "E_EVENT_OPERAND", "{r}");
    assert_eq!(r["error"]["path"], "/ops/1/trigger/right", "{r}");
    // An output made by op 0 removed alone in op 1.
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","id":"q2","program":ir,"inputs":["px"]},
               {"op":"remove","id":"q2.avg"}]),
    );
    assert_eq!(reason(&r), "E_PROGRAM_SERIES", "{r}");
    assert_eq!(r["error"]["path"], "/ops/1/id", "{r}");
    assert!(h.s.log().is_empty(), "nothing applied");
    // The same pair as a proposal: refused when proposed (and so never accepted).
    let r = h.cmd_mode(
        "a",
        "propose",
        json!([{"op":"add_program","id":"q3","program":ir,"inputs":["px"]},
               {"op":"remove","id":"q3.up"}]),
    );
    assert_eq!(reason(&r), "E_PROGRAM_SERIES", "{r}");
}

#[test]
fn every_op_is_guarded_even_when_an_earlier_op_fails_the_dry_run() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let ir = h.ir(SMA_TEXT);
    let bad = json!({"kind":"threshold","series":"nope","op":"gt","value":1});
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","id":"q4","program":ir,"inputs":["px"]},
               {"op":"add_intent","text":"x","trigger":bad},
               {"op":"remove","id":"p1.avg"}]),
    );
    assert_eq!(reason(&r), "E_PROGRAM_SERIES", "{r}");
    assert_eq!(r["error"]["path"], "/ops/2/id", "{r}");
}

#[test]
fn queries_take_exactly_one_bar_series() {
    let mut h = H::new();
    for kind in ["check_program", "try_program", "explain_program"] {
        let r = h.query(
            "a",
            json!({"kind":kind,"source":SMA_TEXT,"inputs":["px","px"]}),
        );
        assert_eq!(reason(&r), "E_NOT_BAR_SERIES", "{kind}: {r}");
        assert_eq!(r["error"]["path"], "/query/inputs", "{kind}: {r}");
    }
}

#[test]
fn check_warns_when_the_live_cost_alone_is_over_the_owner_limit() {
    let mut h = H::new();
    let text = "s = stdev(close, 600)\nseries a = s\nseries b = s\nseries c = s\nseries d = s\n";
    let r = h.query("a", json!({"kind":"check_program","source":text}));
    let d = &r["data"];
    assert_eq!(d["ok"], true, "{r}");
    assert!(d["cost"]["live_per_bar"].as_u64().unwrap() > 2000, "{r}");
    let w = d["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["reason"] == "W_LIVE_BUDGET")
        .unwrap_or_else(|| panic!("no warning: {r}"));
    assert_eq!(w["expected"], json!(["2000"]));
    // And add_program refuses it, as the warning says.
    let ir = d["ir"].clone();
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(reason(&r), "E_BUDGET", "{r}");
    // A cheap program: no such warning.
    let r = h.query("a", json!({"kind":"check_program","source":SMA_TEXT}));
    let ws = r["data"]["warnings"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(ws.iter().all(|w| w["reason"] != "W_LIVE_BUDGET"), "{r}");
}
