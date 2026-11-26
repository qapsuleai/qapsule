//! `check_program`, `try_program`, `explain_program` (PS §3.2).

use qp_schema::protocol::SessionInit;
use serde_json::{json, Value};

use crate::rig::{code, H, SIGNAL_TEXT, SMA_TEXT};

/// The answer fields that do not depend on how the program was given.
fn essence(d: &Value) -> Value {
    let mut warnings = d["warnings"].clone();
    for w in warnings.as_array_mut().into_iter().flatten() {
        w.as_object_mut().map(|o| o.remove("span"));
    }
    json!({"ok": d["ok"], "cost": d["cost"], "outputs": d["outputs"], "params": d["params"],
           "program_id": d["program_id"], "limits": d["limits"], "warnings": warnings})
}

#[test]
fn text_and_ir_give_the_same_check() {
    let mut h = H::new();
    let t = h.query(
        "a",
        json!({"kind":"check_program","source":SMA_TEXT,"inputs":["px"]}),
    );
    assert_eq!(t["data"]["kind"], "program_check", "{t}");
    let ir = t["data"]["ir"].clone();
    assert!(ir.is_object(), "text gives its IR: {t}");
    let i = h.query(
        "a",
        json!({"kind":"check_program","program":ir,"inputs":["px"]}),
    );
    assert_eq!(essence(&t["data"]), essence(&i["data"]));
    assert!(i["data"]["ir"].is_null(), "no IR echoed for an IR input");
    assert!(
        t["data"]["cost"]["bars"].as_u64().unwrap() > 0,
        "bound: bars counted"
    );
    assert_eq!(
        t["data"]["limits"]["program.live.doc.max_log_bytes"],
        4_194_304
    );
    let outs = &t["data"]["outputs"];
    assert_eq!(outs[0]["name"], "avg");
    assert_eq!(outs[1]["kind"], "event");
    assert_eq!(outs[1]["documentable"], true);
    // A signal output checks (only add_program refuses it), not documentable.
    let s = h.query("a", json!({"kind":"check_program","source":SIGNAL_TEXT}));
    assert_eq!(s["data"]["ok"], true, "{s}");
    assert_eq!(s["data"]["outputs"][0]["documentable"], false);
}

#[test]
fn program_errors_are_in_the_answer_with_spans() {
    let mut h = H::new();
    let r = h.query(
        "a",
        json!({"kind":"check_program","source":"x = smaa(close, 3)\nseries y = x\n"}),
    );
    let d = &r["data"];
    assert_eq!(d["ok"], false, "{r}");
    let e = &d["errors"][0];
    assert_eq!(e["reason"], "E_UNKNOWN_OP", "{r}");
    assert_eq!(e["span"]["line"], 1, "{r}");
    assert!(
        e["did_you_mean"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "sma"),
        "{r}"
    );
    // A checker error (window above the limit) is mapped back to the text.
    let r = h.query(
        "a",
        json!({"kind":"check_program","source":"series y = sma(close, 5000)\n","lang":"tr"}),
    );
    let e = &r["data"]["errors"][0];
    assert_eq!(e["reason"], "E_WINDOW_LIMIT", "{r}");
    assert_eq!(e["span"]["line"], 1, "{r}");
    // Exactly one of program / source.
    let r = h.query("a", json!({"kind":"check_program"}));
    assert_eq!(code(&r), "invalid", "{r}");
    let r = h.query(
        "a",
        json!({"kind":"check_program","source":"series y = close\n","inputs":["rsi14"]}),
    );
    assert_eq!(r["error"]["reason"], "E_NOT_BAR_SERIES", "{r}");
}

#[test]
fn try_runs_over_the_closed_bars() {
    let mut h = H::new();
    let r = h.query(
        "a",
        json!({"kind":"try_program","source":SMA_TEXT,"inputs":["px"]}),
    );
    let d = &r["data"];
    assert_eq!(d["kind"], "program_try", "{r}");
    assert_eq!(d["series"][0]["name"], "avg", "{r}");
    assert!(d["series"][0]["defined"].as_u64().unwrap() > 0, "{r}");
    assert_eq!(d["event"][0]["name"], "up", "{r}");
    assert!(d["work"]["used"].as_u64().unwrap() > 0, "{r}");
    assert_eq!(d["work"]["limit"], 50_000_000);
    // Signals: simulated records.
    let r = h.query(
        "a",
        json!({"kind":"try_program","source":SIGNAL_TEXT,"inputs":["px"]}),
    );
    assert_eq!(r["data"]["signal"][0]["name"], "dip", "{r}");
    // An instance: its program, values and bar series.
    h.add("p1", SMA_TEXT);
    let i = h.query("a", json!({"kind":"try_program","instance":"p1"}));
    assert_eq!(i["data"]["series"], d["series"], "{i}");
    let i2 = h.query(
        "a",
        json!({"kind":"try_program","instance":"p1","params":{"len":9}}),
    );
    assert_ne!(i2["data"]["series"], d["series"]);
    // A window.
    let w = h.query(
        "a",
        json!({"kind":"try_program","instance":"p1","range":{"last":20}}),
    );
    let to = w["data"]["to_t"].as_i64().unwrap();
    assert!(w["data"]["from_t"].as_i64().unwrap() < to, "{w}");
    // No bar series: invalid.
    let r = h.query("a", json!({"kind":"try_program","source":SMA_TEXT}));
    assert_eq!(code(&r), "invalid", "{r}");
    let r = h.query("a", json!({"kind":"try_program","instance":"nope"}));
    assert_eq!(code(&r), "not_found", "{r}");
}

#[test]
fn explain_gives_canonical_text_cost_and_provenance() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let r = h.query("a", json!({"kind":"explain_program","instance":"p1"}));
    let d = &r["data"];
    assert_eq!(d["kind"], "program_explain", "{r}");
    let text = d["text"].as_str().unwrap();
    assert!(
        text.contains("sma(close, len)") || text.contains("sma(close,len)"),
        "{text}"
    );
    assert_eq!(
        d["instance_id"],
        h.series("p1.avg")["source"]["program"]["instance_id"]
    );
    assert_eq!(d["params"], json!({"len": 5}));
    let cost = d["cost"].as_array().unwrap();
    assert!(
        cost.iter()
            .any(|c| c["op"] == "sma" && c["clock"] == "base"),
        "{r}"
    );
    // The text explained checks to the same program.
    let c = h.query("a", json!({"kind":"check_program","source":text}));
    assert_eq!(c["data"]["program_id"], d["program_id"], "{c}");
    // A value's provenance.
    let try_ = h.query("a", json!({"kind":"try_program","instance":"p1"}));
    let t = try_["data"]["series"][0]["last"][0]["t"].as_i64().unwrap();
    let r = h.query(
        "a",
        json!({"kind":"explain_program","instance":"p1","at":{"output":"avg","t":t}}),
    );
    let p = &r["data"]["provenance"];
    assert_eq!(p["output"], "avg", "{r}");
    assert_eq!(p["t"], t);
    assert_eq!(p["data"]["series"], "px");
    let r = h.query(
        "a",
        json!({"kind":"explain_program","instance":"p1","at":{"output":"avgg","t":t}}),
    );
    assert_eq!(code(&r), "not_found", "{r}");
}

#[test]
fn try_respects_data_entitlement() {
    let init: SessionInit = serde_json::from_value(json!({
        "document": "fixtures/protocol/docs/thyao-bound.json",
        "clock": 1_760_000_000_000_i64,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"viewer","author":{"kind":"human","name":"viewer"},"role":"viewer"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ],
        "data": {
            "feeds": [{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":120,"seed":5}}],
            "entitlements": {"host":["*"], "arkin":["THYAO"]}
        }
    }))
    .unwrap();
    let mut h = H::with(init, &[("a", "arkin"), ("v", "viewer")]);
    let q = json!({"kind":"try_program","source":SMA_TEXT,"inputs":["px"]});
    let r = h.query("a", q.clone());
    assert_eq!(r["data"]["kind"], "program_try", "{r}");
    let r = h.query("v", q);
    assert_eq!(code(&r), "no_entitlement", "{r}");
    let r = h.query("v", json!({"kind":"explain_program","source":SMA_TEXT,"inputs":["px"],"at":{"output":"avg","t":0}}));
    assert_eq!(code(&r), "no_entitlement", "{r}");
    // Checking needs no data: the viewer gets the check without a bar count.
    let r = h.query(
        "v",
        json!({"kind":"check_program","source":SMA_TEXT,"inputs":["px"]}),
    );
    assert_eq!(r["data"]["ok"], true, "{r}");
    assert!(r["data"]["cost"]["bars"].is_null(), "{r}");
}
