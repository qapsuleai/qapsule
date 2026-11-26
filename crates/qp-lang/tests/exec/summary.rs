//! The `try_program` summary (PS §3.2): window, counts, bounded items, runs and isolated
//! events (D1 condition 2), records, state keys, budget stop; and the document rule: with
//! the same start, try's event bars equal the event series of a cone-only run.

use qp_lang::{
    try_program, try_window, Align, ExecOptions, Executor, Limit, RecordCtx, TryRequest,
};
use qp_schema::program::{EventAt, StopReason};
use qp_schema::{Author, Bar};
use serde_json::json;

use crate::support::*;

fn ctx() -> RecordCtx {
    RecordCtx {
        instance: "p1".to_owned(),
        owner: "arkin".to_owned(),
        author: Author::Human {
            name: "arkin".to_owned(),
        },
        instrument: "SYN".to_owned(),
        period: "1h".to_owned(),
        series: "px".to_owned(),
        anchor_t: Some(T0),
        feeds: Vec::new(),
    }
}

fn req<'a>(c: &'a qp_lang::Checked, b: &'a [Bar]) -> TryRequest<'a> {
    TryRequest {
        checked: c,
        bars: b,
        from_t: None,
        to_t: None,
        align: Align::Utc,
        limits: wide(),
        ctx: ctx(),
    }
}

#[test]
fn summary_of_every_output_kind() {
    let c = checked(&kitchen_sink());
    let b = bars(800, 13);
    let s = try_program(&req(&c, &b)).unwrap();
    assert_eq!((s.from_t, s.to_t), (Some(b[0].t), Some(b[799].t)));
    assert!(s.stopped.is_none());
    assert_eq!(s.cost, c.cost);
    assert!(s.work.used >= c.cost.per_bar * 800);
    let (se, ev, sg, st) = (&s.series[0], &s.event[0], &s.signal[0], &s.state[0]);
    assert_eq!(se.name, "mix");
    assert_eq!(se.last.len(), 10);
    assert_eq!(se.last[9].t, b[799].t);
    assert!(se.defined > 700);
    assert!(ev.events > 0 && ev.runs > 0 && ev.runs <= ev.events);
    assert_eq!(ev.notifications, ev.runs);
    assert_eq!(ev.consecutive, ev.runs < ev.events);
    assert!(ev.isolated <= ev.runs);
    assert!(ev.first.len() <= 10 && !ev.first.is_empty());
    assert_eq!(ev.first[0].known_t, ev.first[0].t + HOUR);
    assert!(sg.count > 0);
    let o = sg.outcomes;
    assert_eq!(o.target + o.stop + o.expired + o.open, sg.count);
    assert_eq!(sg.first.len() as u64, sg.count.min(10));
    assert!(sg.first.iter().all(|r| r.id.starts_with("p1.entry@")));
    assert!(st.distinct > 1 && st.top.len() <= 10);
    assert!(st.top.windows(2).all(|w| w[0].count >= w[1].count));
    assert!(st.top[0].key.contains_key("mom"));
    // Two tries are byte-equal.
    assert_eq!(
        serde_json::to_string(&s).unwrap(),
        serde_json::to_string(&try_program(&req(&c, &b)).unwrap()).unwrap()
    );
}

#[test]
fn runs_and_isolated_events() {
    // Events on bars 2, 3, 4 (one run of three) and 7 (isolated), and the last bar 9
    // (isolated, still open at the window's end).
    let p = prog(
        json!([{"id": "i", "op": "bar_index"},
               {"id": "a", "op": "const", "value": 2}, {"id": "b", "op": "const", "value": 4},
               {"id": "s", "op": "const", "value": 7}, {"id": "n", "op": "const", "value": 9},
               {"id": "ge", "op": "ge", "a": "i", "b": "a"}, {"id": "le", "op": "le", "a": "i", "b": "b"},
               {"id": "in", "op": "and", "a": "ge", "b": "le"},
               {"id": "e7", "op": "eq", "a": "i", "b": "s"}, {"id": "e9", "op": "eq", "a": "i", "b": "n"},
               {"id": "x", "op": "or", "a": "in", "b": "e7"}, {"id": "y", "op": "or", "a": "x", "b": "e9"}]),
        json!([{"name": "e", "kind": "event", "node": "y"}]),
    );
    let c = checked(&p);
    let b = bars(10, 2);
    let s = try_program(&req(&c, &b)).unwrap();
    let e = &s.event[0];
    assert_eq!(
        (e.events, e.runs, e.isolated, e.consecutive),
        (5, 3, 2, true)
    );
    let at = |i: usize| EventAt {
        t: b[i].t,
        known_t: b[i].t + HOUR,
    };
    assert_eq!(e.first, vec![at(2), at(3), at(4), at(7), at(9)]);
}

#[test]
fn window_range_and_bar_limit() {
    let c = checked(&kitchen_sink());
    let b = bars(100, 3);
    let mut r = req(&c, &b);
    r.from_t = Some(b[10].t);
    r.to_t = Some(b[59].t);
    assert_eq!(try_window(&r).len(), 50);
    r.limits = wide().with(Limit::TryMaxBars, 20);
    let w = try_window(&r);
    assert_eq!((w.len(), w[0].t), (20, b[40].t), "the latest bars");
    let s = try_program(&r).unwrap();
    assert_eq!((s.from_t, s.to_t), (Some(b[40].t), Some(b[59].t)));
}

#[test]
fn budget_stops_the_try() {
    let c = checked(&kitchen_sink());
    let b = bars(300, 3);
    let mut r = req(&c, &b);
    r.limits = wide().with(Limit::TryMaxWork, 2_000);
    let s = try_program(&r).unwrap();
    let st = s.stopped.expect("stopped");
    assert_eq!(st.reason, StopReason::Budget);
    assert_eq!(st.limit, 2_000);
    assert!(st.work > 2_000);
    assert_eq!(s.to_t, Some(b[st.bar as usize - 1].t));
    assert!(s.work.used <= 2_000);
}

#[test]
fn try_events_equal_the_document_event_series() {
    // PR §8.3: same start, same bars → try's event bars = the document's event series
    // (which runs only its cone, with a live bar at the end).
    let c = checked(&kitchen_sink());
    let b = bars(500, 17);
    let s = try_program(&req(&c, &b)).unwrap();
    let opts = ExecOptions {
        outputs: Some(vec!["brk".to_owned()]),
        ..ExecOptions::default()
    };
    let mut doc = Executor::new(&c, opts).unwrap();
    for bar in &b {
        doc.push(bar.clone()).unwrap();
    }
    doc.push(Bar {
        t: b[499].t + HOUR,
        ..b[499].clone()
    })
    .unwrap();
    let ev: Vec<i64> = (0..500)
        .filter(|&i| doc.value_at(0, i) == Some(1.0))
        .map(|i| b[i].t)
        .collect();
    assert_eq!(ev.len() as u64, s.event[0].events);
    assert!(!ev.is_empty());
    let try_first: Vec<i64> = s.event[0].first.iter().map(|e| e.t).collect();
    assert_eq!(&ev[..try_first.len()], &try_first[..]);
    let try_last: Vec<i64> = s.event[0].last.iter().map(|e| e.t).collect();
    assert_eq!(&ev[ev.len() - try_last.len()..], &try_last[..]);
}
