//! Static cost (PR §8.1; GOAL acceptance 11): known before running, from node kinds,
//! bound values and bar count; over the limit is rejected without running.
#![allow(clippy::panic)]

mod support;

use qp_lang::{check, Binding, CheckCtx, Limits};
use serde_json::json;
use support::*;

fn cost(r: &qp_lang::CheckReport) -> (u64, u64) {
    r.cost
        .as_ref()
        .map_or((0, 0), |c| (c.per_bar, c.live_per_bar))
}

#[test]
fn weights_and_outputs() {
    // input 0 + sma 2 + one output.
    assert_eq!(cost(&chk(&sma(json!(20)))), (3, 3));
    // PR §14 macd: input 0 + macd 3 + three outputs; live: each output's cone (3) + 1.
    let macd = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "m", "op": "macd", "x": "c", "fast": 12, "slow": 26, "signal": 9}]),
        json!([{"name": "macd", "kind": "series", "node": "m.macd"},
               {"name": "signal", "kind": "series", "node": "m.signal"},
               {"name": "histogram", "kind": "series", "node": "m.histogram"}]),
    );
    assert_eq!(cost(&chk(&macd)), (6, 12));
}

#[test]
fn live_cost_counts_each_outputs_cone() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "a", "op": "sma", "x": "c", "period": 10},
               {"id": "b", "op": "ema", "x": "c", "period": 10},
               {"id": "d", "op": "sub", "a": "a", "b": "b"}]),
        json!([{"name": "a", "kind": "series", "node": "a"},
               {"name": "d", "kind": "series", "node": "d"}]),
    );
    // per bar: 2 + 2 + 1 + 2 outputs = 7; live: (2 + 1) + (5 + 1) = 9.
    assert_eq!(cost(&chk(&p)), (7, 9));
}

#[test]
fn stdev_weighs_its_bound_period() {
    let p = prog_p(
        json!([int_param("len", 2, 600, 20)]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "s", "op": "stdev", "x": "c", "period": {"param": "len"}}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    assert_eq!(cost(&chk(&p)), (21, 21));
    assert_eq!(cost(&chk_params(&p, &json!({"len": 500}))), (501, 501));
}

#[test]
fn higher_timeframe_nodes_are_not_discounted() {
    let p = prog(
        json!([{"id": "d", "op": "input", "field": "close", "tf": "1d"},
               {"id": "m", "op": "sma", "x": "d", "period": 10},
               {"id": "a", "op": "align", "x": "m"}]),
        json!([{"name": "a", "kind": "series", "node": "a"}]),
    );
    assert_eq!(cost(&chk_bound(&p, "1h")), (3, 3));
}

#[test]
fn cost_known_before_running_with_bar_count() {
    let ctx = CheckCtx {
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: Some(3000),
        }),
        ..CheckCtx::default()
    };
    let r = check(&sma(json!(20)), &ctx);
    let c = r.cost.unwrap_or_else(|| panic!("cost"));
    assert_eq!((c.bars, c.total, c.limit), (Some(3000), Some(9000), 666));
    let c = chk(&sma(json!(20))).cost.unwrap_or_else(|| panic!("cost"));
    assert_eq!((c.bars, c.total), (None, None));
}

#[test]
fn over_the_limit_is_rejected_and_the_limit_is_a_parameter() {
    let stdev = |period: i64| {
        prog(
            json!([{"id": "c", "op": "input", "field": "close"},
                   {"id": "s", "op": "stdev", "x": "c", "period": period}]),
            json!([{"name": "s", "kind": "series", "node": "s"}]),
        )
    };
    // 665 + 1 output = 666: at the limit is fine.
    assert!(chk(&stdev(665)).ok());
    let r = chk(&stdev(666));
    only(&r, "E_COST_LIMIT", "");
    assert!(r.checked.is_none() && r.instance_id.is_none());
    assert_eq!(r.cost.map(|c| c.per_bar), Some(667));
    // The host raises the limit through the registry key.
    let l = Limits::from_params(|k| (k == "program.max_cost_per_bar").then_some(1000.0));
    assert!(chk_limits(&stdev(666), l).ok());
}

#[test]
fn checked_nodes_carry_their_weights() {
    let r = chk(&sma(json!(20)));
    let w: Vec<u64> = r
        .checked
        .map(|c| c.nodes.iter().map(|n| n.w).collect())
        .unwrap_or_default();
    assert_eq!(w, vec![0, 2]);
}
