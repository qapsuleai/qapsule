//! The work budget (PR §8.3, VERIFY 14): work = Σ per_bar + records × record + Σ open
//! records × open_record + reads × read; the run stops, coded, before the bar that would
//! overrun it, and nothing is born after.

use qp_lang::{ExecOptions, Executor, WorkBudget};
use serde_json::json;

use crate::support::*;

fn budget(max_work: u64) -> WorkBudget {
    WorkBudget {
        max_work,
        record: 16,
        open_record: 1,
        read: 1,
    }
}

fn with_budget(b: WorkBudget) -> ExecOptions {
    ExecOptions {
        budget: Some(b),
        ..ExecOptions::default()
    }
}

#[test]
fn bars_times_weight() {
    // sma (w 2) + one output: 3 per bar.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "s", "op": "sma", "x": "c", "period": 5}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let c = checked(&p);
    assert_eq!(c.cost.per_bar, 3);
    let x = Executor::run_closed(&c, &bars(50, 1), with_budget(budget(30))).unwrap();
    assert_eq!(x.closed_len(), 10);
    let st = x.stopped().expect("stopped");
    assert_eq!(
        (st.bar, st.t, st.work, st.limit),
        (10, T0 + 10 * HOUR, 33, 30)
    );
    assert_eq!(x.work(), 30);
    // Exactly enough: no stop.
    let x = Executor::run_closed(&c, &bars(10, 1), with_budget(budget(30))).unwrap();
    assert!(x.stopped().is_none());
}

#[test]
fn records_and_open_records_are_charged() {
    // An event every bar: 2 per bar + 16 per event.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "z", "op": "const", "value": 0},
               {"id": "up", "op": "gt", "a": "c", "b": "z"}]),
        json!([{"name": "e", "kind": "event", "node": "up"}]),
    );
    let x = Executor::run_closed(&checked(&p), &bars(50, 1), with_budget(budget(100))).unwrap();
    assert_eq!(
        (x.closed_len(), x.stopped().map(|s| s.work)),
        (5, Some(108))
    );
    // A signal every bar that never resolves: bar j pays 2 + 16 + j open records.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "z", "op": "const", "value": 0},
               {"id": "up", "op": "gt", "a": "c", "b": "z"}]),
        json!([{"name": "g", "kind": "signal", "node": "up", "side": "al",
                "rule": {"max_bars": 400}, "source": {"strategy": "s", "version": "1"}}]),
    );
    let x = Executor::run_closed(&checked(&p), &bars(50, 1), with_budget(budget(100))).unwrap();
    // 18, 19, 20, 21 → 78; the fifth bar would make 100 + ... : 78 + 22 = 100 fits, 6th 123.
    assert_eq!(x.closed_len(), 5);
    assert_eq!(x.stopped().map(|s| s.work), Some(123));
}

#[test]
fn reads_are_charged_and_nothing_is_born_after_a_stop() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "z", "op": "const", "value": 0},
               {"id": "up", "op": "gt", "a": "c", "b": "z"}]),
        json!([{"name": "e", "kind": "event", "node": "up"}]),
    );
    let c = checked(&p);
    let b = bars(20, 1);
    let mut x = Executor::new(&c, with_budget(budget(60))).unwrap();
    x.push_closed(b[0].clone()).unwrap(); // 18
    for _ in 0..30 {
        let _ = x.value_at(0, 0); // 30 reads
    }
    assert_eq!(x.work(), 48);
    x.push_closed(b[1].clone()).unwrap(); // 48 + 18 = 66 > 60: stop
    assert_eq!(x.closed_len(), 1);
    assert_eq!(x.stopped().map(|s| s.bar), Some(1));
    x.push_closed(b[2].clone()).unwrap();
    x.push(b[3].clone()).unwrap();
    x.close_live();
    assert_eq!(x.closed_len(), 1, "a stopped run stays stopped");
    assert!(x.events_at(1).is_empty());
}
