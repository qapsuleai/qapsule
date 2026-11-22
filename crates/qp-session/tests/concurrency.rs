//! The 18 concurrent-write cases of RD1/T6c (`rounds/RD1/experiments/T6c-changes.md`
//! §3.5) with the id-based grammar ops (R2 acceptance criterion 2).
//!
//! Base: `thyao.json` + the first three steps of the T6c workload (`sma20`, `ema50` in
//! panel 0, `rsi7` with `rsi14`): eight layers `l0`…`l7`. Two clients see the same base
//! (seq 3) and write without knowing of each other; the authority applies them in order
//! X then Y, and Y then X. Each run is done without `base` and with `base: 3`.
//!
//! Oracle: the intent of each request interpreted by identity, applied in server order
//! with the protocol's rules (item-level last writer wins, §8.1; requests that are
//! invalid on the new state are refused). Classes: **correct** (the final document
//! satisfies the oracle), **refused** (the second write was refused and the document
//! equals the first write alone: safe, needs a retry), **WRONG** (anything else).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use qp_schema::protocol::SessionInit;
use serde_json::{json, Value};
use support::H;

fn init() -> SessionInit {
    let mut v: Value = serde_json::from_str(
        &std::fs::read_to_string(support::root().join("fixtures/protocol/init/thyao.json"))
            .unwrap(),
    )
    .unwrap();
    v["principals"] = json!([
        {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
        {"id":"scout","author":{"kind":"agent","name":"scout"},"role":"editor"},
        {"id":"owner2","author":{"kind":"human","name":"owner2"},"role":"owner"},
        {"id":"feed","author":{"kind":"engine","name":"feed","version":"0"},"role":"feed"}
    ]);
    serde_json::from_value(v).unwrap()
}

/// A session at the T6c base (seq 3) with connections `arkin`, `scout`, `owner2`,
/// `feed` (and `feed2`, the same feed principal on a second connection).
fn base() -> H {
    let mut h = H::with(init());
    for p in ["arkin", "scout", "owner2", "feed"] {
        h.hello(p, p, "headless");
    }
    h.hello("feed2", "feed", "headless");
    for (who, op) in [
        (
            "arkin",
            json!({"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20.0},"show":{"view":"main","panel":0}}),
        ),
        (
            "arkin",
            json!({"op":"add_study","id":"ema50","study":"ema","inputs":["px"],"params":{"period":50.0},"show":{"view":"main","panel":0}}),
        ),
        (
            "scout",
            json!({"op":"add_study","id":"rsi7","study":"rsi","inputs":["px"],"params":{"period":7.0},"show":{"view":"main","with":"rsi14"}}),
        ),
    ] {
        let r = h.cmd(who, "b", json!([op]));
        assert_eq!(r["status"], "applied", "{r}");
    }
    h
}

fn doc(h: &H) -> Value {
    serde_json::to_value(h.s.document()).unwrap()
}

fn layer_ids(d: &Value) -> Vec<String> {
    d["views"][0]["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["id"].as_str().unwrap().to_owned())
        .collect()
}

fn series<'a>(d: &'a Value, id: &str) -> Option<&'a Value> {
    d["series"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
}

fn draws(d: &Value, id: &str) -> bool {
    d["views"][0]["layers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["series"] == id)
}

const BASE_LAYERS: [&str; 8] = ["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7"];

fn base_minus(gone: &[&str]) -> Vec<String> {
    BASE_LAYERS
        .iter()
        .filter(|x| !gone.contains(x))
        .map(|x| (*x).to_owned())
        .collect()
}

type Oracle = fn(&Value, bool) -> bool;

struct Scn {
    name: &'static str,
    x: (&'static str, Value),
    y: (&'static str, Value),
    /// The final document satisfies the intent (`x_first`: X was applied first).
    oracle: Oracle,
}

#[allow(clippy::too_many_lines)]
fn scenarios(next_bar: &Value) -> Vec<Scn> {
    vec![
        Scn {
            name: "S1 two delete_layer by id (bb20=l1, rule70=l3)",
            x: (
                "arkin",
                json!({"op":"delete_layer","view":"main","layer":"l1"}),
            ),
            y: (
                "scout",
                json!({"op":"delete_layer","view":"main","layer":"l3"}),
            ),
            oracle: |d, _| layer_ids(d) == base_minus(&["l1", "l3"]),
        },
        Scn {
            name: "S2 both delete the same layer (bb20=l1)",
            x: (
                "arkin",
                json!({"op":"delete_layer","view":"main","layer":"l1"}),
            ),
            y: (
                "scout",
                json!({"op":"delete_layer","view":"main","layer":"l1"}),
            ),
            oracle: |d, _| layer_ids(d) == base_minus(&["l1"]),
        },
        Scn {
            name: "S3 insert_layer before l0 vs delete_layer rsi14=l2",
            x: (
                "arkin",
                json!({"op":"insert_layer","view":"main","before":"l0",
                "layer":{"id":"area","mark":"area","series":"px","panel":0}}),
            ),
            y: (
                "scout",
                json!({"op":"delete_layer","view":"main","layer":"l2"}),
            ),
            oracle: |d, _| {
                let mut want = vec!["area".to_owned()];
                want.extend(base_minus(&["l2"]));
                layer_ids(d) == want
            },
        },
        Scn {
            name: "S4 remove series macd vs set_params bb20",
            x: ("arkin", json!({"op":"remove","id":"macd"})),
            y: (
                "scout",
                json!({"op":"set_params","series":"bb20","params":{"period":30.0}}),
            ),
            oracle: |d, _| {
                series(d, "macd").is_none()
                    && !draws(d, "macd")
                    && series(d, "bb20").unwrap()["source"]["params"]["period"] == 30.0
                    && series(d, "sma20").unwrap()["source"]["params"]["period"] == 20.0
                    && layer_ids(d) == base_minus(&["l4"])
            },
        },
        Scn {
            name: "S5 set_params ema50 vs remove series sma20",
            x: (
                "arkin",
                json!({"op":"set_params","series":"ema50","params":{"period":34.0}}),
            ),
            y: ("scout", json!({"op":"remove","id":"sma20"})),
            oracle: |d, _| {
                series(d, "ema50").unwrap()["source"]["params"]["period"] == 34.0
                    && series(d, "sma20").is_none()
                    && !draws(d, "sma20")
                    && layer_ids(d) == base_minus(&["l5"])
            },
        },
        Scn {
            name: "S6 two feeds append the same bar",
            x: (
                "feed",
                json!({"op":"append_bars","series":"px","bars":[next_bar]}),
            ),
            y: (
                "feed2",
                json!({"op":"append_bars","series":"px","bars":[next_bar]}),
            ),
            oracle: |d, _| {
                series(d, "px").unwrap()["data"]["bars"]
                    .as_array()
                    .unwrap()
                    .len()
                    == 201
            },
        },
        Scn {
            name: "S7 reject_finding vs update_finding confidence",
            x: (
                "arkin",
                json!({"op":"reject_finding","finding":"fnd-overbought","note":"no"}),
            ),
            y: (
                "scout",
                json!({"op":"update_finding","finding":"fnd-overbought","confidence":0.4}),
            ),
            oracle: |d, _| {
                d["capsules"][0]["state"] == "rejected"
                    && d["findings"][0]["confidence"] == 0.4
                    && d["ledger"]["entries"].as_array().unwrap().len() == 1
            },
        },
        Scn {
            name: "S8 reject_finding vs approve_finding",
            x: (
                "arkin",
                json!({"op":"reject_finding","finding":"fnd-overbought","note":"no"}),
            ),
            y: (
                "owner2",
                json!({"op":"approve_finding","finding":"fnd-overbought","note":"yes"}),
            ),
            // Item-level last writer wins (§8.1); the ledger records both decisions.
            oracle: |d, x_first| {
                let (first, second) = if x_first {
                    ("rejected", "approved")
                } else {
                    ("approved", "rejected")
                };
                let e = d["ledger"]["entries"].as_array().unwrap();
                d["capsules"][0]["state"] == second
                    && e.len() == 2
                    && e[0]["state"] == first
                    && e[1]["state"] == second
            },
        },
        Scn {
            name: "S9 update_finding vs remove rsi7 (disjoint control)",
            x: (
                "scout",
                json!({"op":"update_finding","finding":"fnd-overbought",
                "invalidation":{"kind":"threshold","series":"px","op":"lt","value":100.0}}),
            ),
            y: ("arkin", json!({"op":"remove","id":"rsi7"})),
            oracle: |d, _| {
                d["findings"][0]["invalidation"]["series"] == "px"
                    && series(d, "rsi7").is_none()
                    && layer_ids(d) == base_minus(&["l7"])
            },
        },
    ]
}

fn send(h: &mut H, who: &str, id: &str, op: &Value, base: Option<u64>) -> bool {
    let mut msg = json!({"type":"command","id":id,"ops":[op]});
    if let Some(b) = base {
        msg["base"] = json!(b);
    }
    let r = h.one(who, msg);
    r["status"] == "applied"
}

#[test]
fn eighteen_concurrent_writes_by_id() {
    let b = base();
    let d0 = doc(&b);
    assert_eq!(layer_ids(&d0), BASE_LAYERS);
    let last = d0["series"][0]["data"]["bars"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    let t = last["t"].as_i64().unwrap() + 86_400_000;
    let c = last["c"].as_f64().unwrap();
    let next_bar = json!({"t":t,"o":c,"h":c + 1.0,"l":c - 1.0,"c":c + 0.5,"v":1000.0});
    let mut table =
        String::from("| scenario | order | B-id (no base) | B-id + base |\n|---|---|---|---|\n");
    let mut counts = [[0u32; 3]; 2];
    for sc in scenarios(&next_bar) {
        for x_first in [true, false] {
            let (f, s) = if x_first {
                (&sc.x, &sc.y)
            } else {
                (&sc.y, &sc.x)
            };
            let mut row = format!("| {} | {} |", sc.name, if x_first { "X,Y" } else { "Y,X" });
            for (m, base_seq) in [None, Some(3u64)].into_iter().enumerate() {
                let mut h = base();
                assert!(
                    send(&mut h, f.0, "f", &f.1, base_seq),
                    "first write applies"
                );
                let first_only = serde_json::to_vec(h.s.document()).unwrap();
                let second_ok = send(&mut h, s.0, "s", &s.1, base_seq);
                h.assert_replay();
                let d = doc(&h);
                let class = if (sc.oracle)(&d, x_first) {
                    0
                } else if !second_ok && serde_json::to_vec(h.s.document()).unwrap() == first_only {
                    1
                } else {
                    2
                };
                counts[m][class] += 1;
                row.push_str(match class {
                    0 => " correct |",
                    1 => " refused |",
                    _ => " **WRONG** |",
                });
            }
            table.push_str(&row);
            table.push('\n');
        }
    }
    for (m, name) in ["B-id (no base)", "B-id + base"].iter().enumerate() {
        table.push_str(&format!(
            "\n{name}: correct {}, refused {}, WRONG {}",
            counts[m][0], counts[m][1], counts[m][2]
        ));
    }
    println!("{table}");
    assert_eq!(counts[0][2], 0, "wrong results without base\n{table}");
    assert_eq!(counts[1][2], 0, "wrong results with base\n{table}");
    assert_eq!(counts[0][0] + counts[0][1] + counts[0][2], 18);
}
