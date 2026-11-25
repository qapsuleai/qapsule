//! The resolved program the executor runs (PR §7): slots, multi-output references,
//! types and clocks, base timeframe.
#![allow(clippy::panic)]

mod support;

use qp_lang::{BinaryKind, Clock, Op, OutputSpec, Slot, Ty};
use serde_json::json;
use support::*;

fn checked(v: &serde_json::Value, base: Option<&str>) -> qp_lang::Checked {
    let r = match base {
        Some(b) => chk_bound(v, b),
        None => chk(v),
    };
    assert!(r.ok(), "{:#?}", r.errors);
    r.checked.unwrap_or_else(|| panic!("checked"))
}

#[test]
fn multi_output_references_resolve_to_slots() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "m", "op": "macd", "x": "c", "fast": 12, "slow": 26, "signal": 9},
               {"id": "b", "op": "bollinger", "x": "c", "period": 20, "k": 2},
               {"id": "d", "op": "sub", "a": "m.signal", "b": "b"}]),
        json!([{"name": "macd", "kind": "series", "node": "m"},
               {"name": "up", "kind": "series", "node": "b.upper"},
               {"name": "d", "kind": "series", "node": "d"}]),
    );
    let c = checked(&p, None);
    assert_eq!(c.nodes[1].outputs, vec![Ty::Num; 3]);
    assert_eq!(c.nodes[2].outputs.len(), 4);
    let d = Op::Binary {
        kind: BinaryKind::Sub,
        a: Slot { node: 1, out: 1 },
        b: Slot { node: 2, out: 1 },
    };
    assert_eq!(c.nodes[3].op, d);
    assert_eq!(
        c.outputs[0].spec,
        OutputSpec::Series(Slot { node: 1, out: 0 })
    );
    assert_eq!(
        c.outputs[1].spec,
        OutputSpec::Series(Slot { node: 2, out: 0 })
    );
    assert_eq!(
        c.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        ["c", "m", "b", "d"]
    );
}

#[test]
fn clocks_free_base_and_higher() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "d", "op": "input", "field": "close", "tf": "1d"},
               {"id": "m", "op": "sma", "x": "d", "period": 10},
               {"id": "k", "op": "const", "value": 2},
               {"id": "mk", "op": "mul", "a": "m", "b": "k"},
               {"id": "a", "op": "align", "x": "mk"},
               {"id": "t", "op": "const", "value": true},
               {"id": "tt", "op": "and", "a": "t", "b": "t"},
               {"id": "s", "op": "streak", "c": "tt"},
               {"id": "g", "op": "gt", "a": "c", "b": "a"}]),
        json!([{"name": "a", "kind": "series", "node": "a"},
               {"name": "s", "kind": "series", "node": "s"},
               {"name": "g", "kind": "event", "node": "g"},
               {"name": "k", "kind": "series", "node": "k"}]),
    );
    let c = checked(&p, Some("1h"));
    let clocks: Vec<&str> = c.nodes.iter().map(|n| n.clock.name()).collect();
    assert_eq!(
        clocks,
        ["base", "1d", "1d", "const", "1d", "base", "const", "const", "base", "base"]
    );
    assert!(matches!(&c.nodes[1].clock, Clock::Tf(tf) if tf.name == "1d"));
    assert_eq!(c.base.as_ref().map(|t| t.name.as_str()), Some("1h"));
    assert_eq!(c.nodes[9].outputs, vec![Ty::Bool]);
    // Unbound: a higher timeframe is only spell-checked, the base is unknown.
    assert!(checked(&p, None).base.is_none());
}

#[test]
fn types_flow_through_lag_align_and_bucket() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "k", "op": "const", "value": 1},
               {"id": "g", "op": "gt", "a": "c", "b": "k"},
               {"id": "lg", "op": "lag", "x": "g", "n": 1},
               {"id": "b", "op": "bucket", "x": "c", "edges": [1, 2]},
               {"id": "db", "op": "defined", "x": "b"},
               {"id": "e", "op": "and", "a": "lg", "b": "db"}]),
        json!([{"name": "e", "kind": "event", "node": "e"},
               {"name": "st", "kind": "state", "dims": [{"name": "lvl", "node": "b"}]}]),
    );
    let c = checked(&p, None);
    assert_eq!(c.nodes[3].outputs, vec![Ty::Bool]);
    assert_eq!(c.nodes[4].outputs, vec![Ty::Cat]);
    assert_eq!(
        c.outputs[1].spec,
        OutputSpec::State(vec![("lvl".to_owned(), Slot { node: 4, out: 0 })])
    );
    // A cat anywhere but `defined` and a state is a type error.
    let mut bad = p.clone();
    bad["nodes"][6]["b"] = json!("b");
    only(&chk(&bad), "E_TYPE", "/nodes/6/b");
}

#[test]
fn the_check_is_deterministic() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "x", "op": "rsii", "x": "c", "period": 3},
               {"id": "y", "op": "sma", "x": "q", "period": 0}]),
        json!([{"name": "y", "kind": "series", "node": "y"}]),
    );
    let a = chk(&p);
    let b = chk(&p);
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_string(&a.to_answer()).unwrap_or_default(),
        serde_json::to_string(&b.to_answer()).unwrap_or_default()
    );
}
