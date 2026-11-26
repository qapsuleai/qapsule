//! Program output series in the values cache (RL1, `docs/PROGRAMS-SESSION.md` §1.2–§1.3):
//! values equal the executor's, event series semantics (K13 full length, `null` live bar,
//! closed evaluable length, `since_t`), the incremental path equals a rebuild bit for bit,
//! and a study without `program` is unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use qp_lang::{check, Align, Binding, CheckCtx, ExecOptions, Executor};
use qp_plane::values::{eval_condition, values, Cache, Delta};
use qp_schema::{Bar, CompareOp, Condition};
use serde_json::{json, Value};
use support::{
    bits, doc, program_series, set_bars, walk, ABOVE, BREAKOUT, DAILY, HOUR, SPREAD, T0,
};

/// The executor's own run: closed bars, the last one live; values via `doc_value_at`.
fn reference(
    ir: &str,
    params: &Value,
    output: &str,
    bars: &[Bar],
    since_t: i64,
) -> Vec<Option<f64>> {
    let v: Value = serde_json::from_str(ir).unwrap();
    let ctx = CheckCtx {
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        params: Some(params),
        ..CheckCtx::default()
    };
    let c = check(&v, &ctx).checked.expect("checks");
    let opts = ExecOptions {
        align: Align::Utc,
        outputs: Some(vec![output.to_owned()]),
        budget: None,
    };
    let (closed, last) = bars.split_at(bars.len() - 1);
    let mut x = Executor::run_closed(&c, closed, opts).unwrap();
    x.push(last[0].clone()).unwrap();
    let o = x.output(output).unwrap();
    (0..bars.len())
        .map(|i| x.doc_value_at(o, i, Some(since_t)))
        .collect()
}

fn main_of(d: &qp_schema::Document, cache: &mut Cache, id: &str) -> (Vec<Option<f64>>, usize) {
    let c = values(d, cache, id).unwrap();
    (c.main_values().to_vec(), c.len())
}

#[test]
fn program_series_values_are_the_executor_output() {
    let bars = walk(300, 7, T0, HOUR);
    for (ir, out, params) in [
        (SPREAD, "spread", json!({"fast": 5, "slow": 13})),
        (SPREAD, "trigger", json!({"fast": 12, "slow": 26})),
        (BREAKOUT, "kirilim", json!({"len": 10})),
        (DAILY, "above_daily", json!({})),
        (DAILY, "daily_ma", json!({})),
    ] {
        let d = doc(&bars, vec![program_series("p1", ir, out, &params, 0)]);
        let mut cache = Cache::default();
        let (got, _) = main_of(&d, &mut cache, &format!("p1.{out}"));
        let want = reference(ir, &params, out, &bars, 0);
        assert_eq!(bits(&got), bits(&want), "{out}");
        assert!(got.iter().any(Option::is_some), "{out}: some values");
    }
}

#[test]
fn event_series_is_full_length_null_live_and_closed_evaluable() {
    let bars = walk(300, 11, T0, HOUR);
    let d = doc(
        &bars,
        vec![program_series(
            "p1",
            BREAKOUT,
            "kirilim",
            &json!({"len": 5}),
            0,
        )],
    );
    let mut cache = Cache::default();
    let c = values(&d, &mut cache, "p1.kirilim").unwrap();
    let v = c.main_values();
    assert_eq!(c.t.len(), bars.len(), "K13: one value per bar");
    assert_eq!(v.len(), bars.len());
    assert_eq!(c.len(), bars.len() - 1, "evaluable: the closed bars");
    assert_eq!(v[bars.len() - 1], None, "live bar: null");
    assert!(v[..bars.len() - 1]
        .iter()
        .all(|x| matches!(x, Some(f) if *f == 0.0 || *f == 1.0)));
    assert!(
        v.iter().filter(|x| **x == Some(1.0)).count() > 3,
        "events occur"
    );
    // A series output keeps the bar count and a transient live value.
    let d = doc(
        &bars,
        vec![program_series("p2", SPREAD, "spread", &json!({}), 0)],
    );
    let c = values(&d, &mut cache, "p2.spread").unwrap();
    assert_eq!(c.len(), bars.len());
    assert!(c.main_values()[bars.len() - 1].is_some());
}

#[test]
fn since_t_masks_events_known_at_or_before_it() {
    let bars = walk(300, 11, T0, HOUR);
    let params = json!({"len": 5});
    let all = {
        let d = doc(
            &bars,
            vec![program_series("p1", BREAKOUT, "kirilim", &params, 0)],
        );
        main_of(&d, &mut Cache::default(), "p1.kirilim").0
    };
    // Bar i is known at t_i + 1h; since_t = known_t of bar 150 masks bars 0..=150.
    let since = bars[150].t + HOUR;
    let d = doc(
        &bars,
        vec![program_series("p1", BREAKOUT, "kirilim", &params, since)],
    );
    let (masked, _) = main_of(&d, &mut Cache::default(), "p1.kirilim");
    assert!(
        all[..=150].contains(&Some(1.0)),
        "events before since_t exist"
    );
    assert!(masked[..=150].iter().all(|x| *x == Some(0.0)));
    assert_eq!(masked[151..], all[151..]);
    // since_t after every bar: no event in the document at all.
    let d = doc(
        &bars,
        vec![program_series("p1", BREAKOUT, "kirilim", &params, i64::MAX)],
    );
    let (none, _) = main_of(&d, &mut Cache::default(), "p1.kirilim");
    assert!(!none.contains(&Some(1.0)));
}

#[test]
fn threshold_ge_1_holds_when_the_event_bar_closes() {
    // Closes 100..104, then a live bar crossing 105, then the next bar.
    let mk = |t: i64, c: f64| Bar {
        t,
        o: c,
        h: c,
        l: c,
        c,
        v: 1.0,
    };
    let mut bars: Vec<Bar> = (0..5)
        .map(|i| mk(T0 + i * HOUR, 100.0 + i as f64))
        .collect();
    let mut d = doc(
        &bars,
        vec![program_series("p1", ABOVE, "up", &json!({}), 0)],
    );
    let cond = Condition::Threshold {
        series: "p1.up".to_owned(),
        op: CompareOp::Ge,
        value: 1.0,
    };
    let mut cache = Cache::default();
    let first = eval_condition(&d, &mut cache, &cond).unwrap();
    assert_eq!(
        first,
        (false, 3, bars[3].t),
        "evaluated at the last closed bar"
    );
    // The live bar crosses: no value on the live bar, the condition does not fire yet.
    bars[4].c = 106.0;
    set_bars(&mut d, &bars);
    cache.begin_tick();
    cache.apply_delta(&d, "px", Delta::Revised { index: 4 });
    assert_eq!(
        eval_condition(&d, &mut cache, &cond).unwrap(),
        (false, 3, bars[3].t)
    );
    // The next bar closes it: the condition holds at the event bar (t4 < the last bar's t,
    // so `claims::closed` makes the evidence final).
    bars.push(mk(T0 + 5 * HOUR, 106.5));
    set_bars(&mut d, &bars);
    cache.begin_tick();
    cache.apply_delta(&d, "px", Delta::Appended { from: 5 });
    assert_eq!(
        eval_condition(&d, &mut cache, &cond).unwrap(),
        (true, 4, bars[4].t)
    );
    // Consecutive event bars: every one is in the series.
    bars.push(mk(T0 + 6 * HOUR, 99.0));
    set_bars(&mut d, &bars);
    cache.apply_delta(&d, "px", Delta::Appended { from: 6 });
    let (v, n) = main_of(&d, &mut cache, "p1.up");
    assert_eq!(n, 6);
    assert_eq!(
        v,
        [0.0, 0.0, 0.0, 0.0, 1.0, 1.0]
            .map(Some)
            .into_iter()
            .chain([None])
            .collect::<Vec<_>>()
    );
}

/// One random live step on `bars`: a revision of the live bar, a new bar, a correction
/// of an earlier bar or several new bars at once.
fn step(bars: &mut Vec<Bar>, k: u64) -> Delta {
    let n = bars.len();
    let next = |b: &Bar, f: f64| Bar {
        t: b.t + HOUR,
        o: b.c,
        h: b.c.max(b.c * f) * 1.002,
        l: b.c.min(b.c * f) * 0.998,
        c: b.c * f,
        v: 10.0,
    };
    let f = 1.0 + (((k * 7919) % 41) as f64 - 20.0) / 1000.0;
    match k % 7 {
        0..=2 => {
            let b = &mut bars[n - 1];
            b.c *= f;
            b.h = b.h.max(b.c);
            b.l = b.l.min(b.c);
            Delta::Revised { index: n - 1 }
        }
        3 | 4 => {
            let b = next(&bars[n - 1], f);
            bars.push(b);
            Delta::Appended { from: n }
        }
        5 => {
            let b = next(&bars[n - 1], f);
            let c = next(&b, 1.0 / f);
            bars.push(b);
            bars.push(c);
            Delta::Appended { from: n }
        }
        _ => {
            bars[n - 10].c *= f;
            Delta::Revised { index: n - 10 }
        }
    }
}

#[test]
fn incremental_path_equals_rebuild_bit_for_bit() {
    let mut bars = walk(200, 3, T0, HOUR);
    let extra = vec![
        program_series("a", BREAKOUT, "kirilim", &json!({"len": 4}), bars[120].t),
        program_series("b", SPREAD, "spread", &json!({"fast": 3, "slow": 8}), 0),
        program_series("c", DAILY, "above_daily", &json!({}), 0),
        program_series("c", DAILY, "daily_ma", &json!({}), 0),
        json!({"id": "s", "source": {"kind": "study", "study": "ema", "inputs": ["px"],
            "params": {"period": 5}}}),
    ];
    let ids = ["a.kirilim", "b.spread", "c.above_daily", "c.daily_ma", "s"];
    let mut d = doc(&bars, extra);
    let mut cache = Cache::default();
    for id in ids {
        values(&d, &mut cache, id).unwrap();
    }
    for k in 0..400u64 {
        let delta = step(&mut bars, k);
        set_bars(&mut d, &bars);
        cache.begin_tick();
        let gen = cache.gen();
        cache.apply_delta(&d, "px", delta);
        if k % 7 != 6 {
            // A live revision or new bars advance the state: no slot is dropped.
            assert_eq!(cache.gen(), gen, "step {k}: incremental, not rebuilt");
        }
        for id in ids {
            let inc = cache.get(id).cloned().unwrap();
            let fresh = values(&d, &mut Cache::default(), id).unwrap();
            assert_eq!(inc.t, fresh.t, "{id} step {k}");
            assert_eq!(inc.len(), fresh.len(), "{id} step {k}");
            assert_eq!(
                bits(inc.main_values()),
                bits(fresh.main_values()),
                "{id} step {k}"
            );
        }
    }
}

#[test]
fn study_without_program_is_unchanged() {
    let bars = walk(100, 5, T0, HOUR);
    let d = doc(
        &bars,
        vec![
            json!({"id": "s", "source": {"kind": "study", "study": "sma", "inputs": ["px"],
            "params": {"period": 10}}}),
        ],
    );
    let s = serde_json::to_value(&d.series[1]).unwrap();
    assert!(s["source"].get("program").is_none(), "no `program` written");
    let c = values(&d, &mut Cache::default(), "s").unwrap();
    let params = [("period".to_owned(), 10.0)].into_iter().collect();
    let mut st = qp_compute::create("sma", &params, qp_compute::Field::Close).unwrap();
    let mut out = vec![None];
    let want: Vec<Option<f64>> = bars
        .iter()
        .map(|b| {
            st.push(&[b.c], &mut out);
            out[0]
        })
        .collect();
    assert_eq!(c.len(), bars.len());
    assert_eq!(bits(c.main_values()), bits(&want));
    // A `program` study name without a program stays an unknown study.
    let d = doc(
        &bars,
        vec![json!({"id": "x", "source": {"kind": "study", "study": "program", "inputs": ["px"]}})],
    );
    assert!(values(&d, &mut Cache::default(), "x").is_err());
}
