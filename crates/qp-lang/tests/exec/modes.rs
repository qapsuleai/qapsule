//! Determinism and run modes (PR §7, §10): two runs byte-equal; batch = streaming = live
//! bar revised then closed; the live bar's transient value never emits.

use qp_lang::{ExecOptions, Executor};
use qp_schema::Bar;
use serde_json::json;

use crate::support::*;

/// `bar` nudged (a live tick before the final values).
fn nudged(b: &Bar, r: &mut Rng) -> Bar {
    let f = 1.0 + (r.unit() - 0.5) * 0.02;
    Bar {
        c: b.c * f,
        h: b.h.max(b.c * f),
        l: b.l.min(b.c * f),
        ..b.clone()
    }
}

/// Streams `bars` as live bars with a few revisions each, then closes the last.
fn streamed(c: &qp_lang::Checked, bars: &[Bar], seed: u64) -> Executor {
    let mut r = Rng(seed);
    let mut x = Executor::new(c, ExecOptions::default()).unwrap();
    for b in bars {
        x.push(nudged(b, &mut r)).unwrap();
        for _ in 0..r.below(3) {
            x.update_last(nudged(b, &mut r)).unwrap();
        }
        x.update_last(b.clone()).unwrap();
    }
    x.close_live();
    x
}

#[test]
fn two_runs_are_byte_equal() {
    let c = checked(&kitchen_sink());
    let b = bars(600, 7);
    let a = run(&c, &b).canonical();
    assert_eq!(a, run(&c, &b).canonical());
    assert!(a.lines().count() > 600 * 3, "everything is published");
}

#[test]
fn batch_equals_streaming_and_revised_live_bars() {
    let c = checked(&kitchen_sink());
    let b = bars(500, 11);
    let batch = run(&c, &b);
    let mut one_by_one = Executor::new(&c, ExecOptions::default()).unwrap();
    for x in &b {
        one_by_one.push_closed(x.clone()).unwrap();
    }
    assert_eq!(batch.canonical(), one_by_one.canonical());
    assert_eq!(batch.canonical(), streamed(&c, &b, 3).canonical());
    // Something was born on closed bars.
    let e = batch.output("brk").unwrap();
    assert!((0..batch.closed_len()).any(|i| batch.value_at(e, i) == Some(1.0)));
}

#[test]
fn live_bar_never_emits() {
    let c = checked(&kitchen_sink());
    let b = bars(400, 5);
    let mut x = Executor::new(&c, ExecOptions::default()).unwrap();
    let (e, s, st, m) = (
        x.output("brk").unwrap(),
        x.output("entry").unwrap(),
        x.output("reg").unwrap(),
        x.output("mix").unwrap(),
    );
    let mut seen_transient = false;
    for (i, bar) in b.iter().enumerate() {
        x.push(bar.clone()).unwrap();
        assert_eq!(x.closed_len(), i);
        assert_eq!(x.value_at(e, i), None, "event on the live bar");
        assert!(x.events_at(i).is_empty());
        assert!(x.signals_at(s, i).is_none(), "signal on the live bar");
        assert_eq!(x.value_at(s, i), None);
        assert_eq!(x.state_at(st, i), None);
        seen_transient |= x.value_at(m, i).is_some();
    }
    assert!(
        seen_transient,
        "series carry a transient value on the live bar"
    );
}

#[test]
fn transient_value_equals_closed_value_without_higher_timeframes() {
    // PR §5.3: the transient value of a base-clock subtree is what the bar gives once
    // closed with the same data (higher timeframes do not see the live bar).
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "r", "op": "rsi", "x": "c", "period": 14},
               {"id": "e", "op": "ema", "x": "r", "period": 5},
               {"id": "d", "op": "sub", "a": "r", "b": "e"}]),
        json!([{"name": "d", "kind": "series", "node": "d"}]),
    );
    let c = checked(&p);
    let b = bars(300, 9);
    let mut x = Executor::new(&c, ExecOptions::default()).unwrap();
    let full = run(&c, &b);
    for (i, bar) in b.iter().enumerate() {
        x.push(bar.clone()).unwrap();
        assert_eq!(
            x.value_at(0, i).map(f64::to_bits),
            full.value_at(0, i).map(f64::to_bits),
            "bar {i}"
        );
    }
}

#[test]
fn bars_must_ascend() {
    let c = checked(&kitchen_sink());
    let b = bars(3, 1);
    let mut x = Executor::new(&c, ExecOptions::default()).unwrap();
    x.push(b[1].clone()).unwrap();
    assert!(x.push(b[0].clone()).is_err());
    assert!(x.update_last(b[2].clone()).is_err());
    assert!(x.push_closed(b[1].clone()).is_err());
}

#[test]
fn document_event_series_is_zero_before_since_t() {
    let c = checked(&kitchen_sink());
    let b = bars(300, 5);
    let x = run(&c, &b);
    let e = x.output("brk").unwrap();
    let ev: Vec<usize> = (0..300)
        .filter(|&i| x.value_at(e, i) == Some(1.0))
        .collect();
    assert!(ev.len() > 2);
    // `since_t` = the close of the second event bar: the first two are hidden.
    let since = x.known_at(ev[1]);
    assert_eq!(x.doc_value_at(e, ev[0], since), Some(0.0));
    assert_eq!(x.doc_value_at(e, ev[1], since), Some(0.0));
    assert_eq!(x.doc_value_at(e, ev[2], since), Some(1.0));
    assert_eq!(x.doc_value_at(e, ev[0], None), Some(1.0));
    let m = x.output("mix").unwrap();
    assert_eq!(x.doc_value_at(m, 10, since), x.value_at(m, 10));
}
