//! Program sweeps (K17): every variation of `qp_lang::sweep` is bit-equal to running the
//! executor with that parameter set, on the shared path (the five PR §14 programs, a mixed
//! program with several swept kernels and undefined inputs) and on the per-variation
//! fallback.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]

use qp_compute::sweep::bench_closes;
use qp_lang::{
    sweep, sweep_param, Binding, CheckCtx, ExecOptions, Executor, Limits, SweepError, SweepPath,
    Variation, WorkBudget,
};
use qp_schema::Bar;
use serde_json::{json, Value};

fn bars(n: usize) -> Vec<Bar> {
    bench_closes(n, 11)
        .into_iter()
        .enumerate()
        .map(|(i, c)| Bar {
            t: 1_704_067_200_000 + i as i64 * 3_600_000,
            o: c * 0.999,
            h: c * 1.01,
            l: c * 0.98,
            c,
            v: 1000.0 + i as f64,
        })
        .collect()
}

fn ctx(params: Option<&Value>) -> CheckCtx<'_> {
    CheckCtx {
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        params,
        // The mixed program has more outputs than the default limit.
        limits: Limits::from_params(|k| (k == "program.max_outputs").then_some(8.0)),
        ..CheckCtx::default()
    }
}

fn int(key: &str, default: i64) -> Value {
    json!({"key": key, "kind": "int", "min": 1, "max": 2000, "default": default})
}

fn prog(params: Value, nodes: Value, outputs: Value) -> Value {
    json!({"version": "qp-ir/0", "params": params, "nodes": nodes, "outputs": outputs})
}

fn series(name: &str, node: &str) -> Value {
    json!({"name": name, "kind": "series", "node": node})
}

/// The PR §14 program of `study`.
fn program(study: &str) -> Value {
    let close = json!({"id": "_1", "op": "input", "field": "close"});
    match study {
        "sma" | "ema" | "rsi" => prog(
            json!([int("period", 14)]),
            json!([close, {"id": "_2", "op": study, "x": "_1", "period": {"param": "period"}}]),
            json!([series(study, "_2")]),
        ),
        "macd" => prog(
            json!([int("fast", 12), int("slow", 26), int("signal", 9)]),
            json!([close, {"id": "m", "op": "macd", "x": "_1", "fast": {"param": "fast"},
                           "slow": {"param": "slow"}, "signal": {"param": "signal"}}]),
            json!([
                series("macd", "m.macd"),
                series("signal", "m.signal"),
                series("histogram", "m.histogram")
            ]),
        ),
        _ => prog(
            json!([int("period", 20),
                   {"key": "k", "kind": "float", "min": 0.0001, "max": 100.0, "default": 2.0}]),
            json!([close, {"id": "b", "op": "bollinger", "x": "_1", "period": {"param": "period"},
                           "k": {"param": "k"}}]),
            json!([
                series("upper", "b.upper"),
                series("middle", "b.middle"),
                series("lower", "b.lower"),
                series("bandwidth", "b.bandwidth")
            ]),
        ),
    }
}

fn bits(v: Option<f64>) -> Option<u64> {
    v.map(f64::to_bits)
}

/// Sweeps and compares every variation with its own executor run; returns the path.
fn assert_equal(
    ir: &Value,
    base: Option<&Value>,
    sets: &[Value],
    b: &[Bar],
    opts: &ExecOptions,
) -> SweepPath {
    let mut seen = vec![false; sets.len()];
    let path = sweep(ir, &ctx(base), sets, b, opts, |v: &Variation<'_>| {
        let x = Executor::run_closed(v.checked(), b, opts.clone()).unwrap();
        assert!(!seen[v.index()], "variation {} twice", v.index());
        seen[v.index()] = true;
        assert_eq!(v.len(), b.len());
        for (j, name) in v.names().iter().enumerate() {
            let o = x.output(name).unwrap();
            for i in 0..b.len() {
                assert_eq!(
                    bits(v.value(j, i)),
                    bits(x.value_at(o, i)),
                    "set {} output {name} bar {i}",
                    v.index()
                );
            }
        }
        // The set really was bound.
        let want = &sets[v.index()];
        for (k, val) in want.as_object().unwrap() {
            let got = serde_json::to_value(v.checked().params[k]).unwrap();
            assert_eq!(got.as_f64(), val.as_f64(), "param {k}");
        }
    })
    .unwrap();
    assert!(seen.iter().all(|s| *s), "every variation visited");
    path
}

fn one(key: &str, values: impl IntoIterator<Item = Value>) -> Vec<Value> {
    values.into_iter().map(|v| json!({ key: v })).collect()
}

#[test]
fn five_programs_one_param_each_bit_equal_shared() {
    let b = bars(600);
    let opts = ExecOptions::default();
    for study in ["sma", "ema", "rsi"] {
        let sets = one(
            "period",
            (1..=40).chain([99, 250, 599, 600, 700]).map(|p| json!(p)),
        );
        let path = assert_equal(&program(study), None, &sets, &b, &opts);
        assert_eq!(
            path,
            SweepPath::Shared {
                swept: 1,
                direct: 0
            },
            "{study}"
        );
    }
    let m = program("macd");
    for (key, vals) in [("fast", 2..=25), ("slow", 13..=60), ("signal", 1..=30)] {
        let sets = one(key, vals.map(|p| json!(p)));
        let path = assert_equal(&m, None, &sets, &b, &opts);
        assert_eq!(
            path,
            SweepPath::Shared {
                swept: 1,
                direct: 0
            },
            "macd {key}"
        );
    }
    let bb = program("bollinger");
    let sets = one("period", (1..=50).map(|p| json!(p)));
    assert_eq!(
        assert_equal(&bb, None, &sets, &b, &opts),
        SweepPath::Shared {
            swept: 1,
            direct: 0
        }
    );
    let sets = one("k", (1..=24).map(|k| json!(f64::from(k) * 0.25)));
    assert_eq!(
        assert_equal(&bb, None, &sets, &b, &opts),
        SweepPath::Shared {
            swept: 1,
            direct: 0
        }
    );
}

#[test]
fn several_params_at_once_and_a_base_binding() {
    let b = bars(500);
    let opts = ExecOptions::default();
    // The K17 benchmark's macd grid shape (fast × slow × signal) and bollinger (period × k).
    let mut sets = Vec::new();
    for fast in 2..=5 {
        for d in 0..4 {
            for signal in 2..=5 {
                sets.push(json!({"fast": fast, "slow": fast + 1 + 2 * d, "signal": signal}));
            }
        }
    }
    let path = assert_equal(&program("macd"), None, &sets, &b, &opts);
    assert_eq!(
        path,
        SweepPath::Shared {
            swept: 1,
            direct: 0
        }
    );
    let sets: Vec<Value> = (2..=12)
        .flat_map(|p| (0..4).map(move |k| json!({"period": p, "k": 1.0 + 0.25 * f64::from(k)})))
        .collect();
    assert_equal(&program("bollinger"), None, &sets, &b, &opts);
    // `sweep_param` over `period` with `k` bound by the base parameters.
    let base = json!({"k": 1.5});
    let vals: Vec<Value> = (2..=30).map(|p| json!(p)).collect();
    let sets = one("period", vals.clone());
    assert_equal(&program("bollinger"), Some(&base), &sets, &b, &opts);
    let mut n = 0;
    sweep_param(
        &program("bollinger"),
        &ctx(Some(&base)),
        "period",
        &vals,
        &b,
        &opts,
        |v| {
            assert_eq!(
                serde_json::to_value(v.checked().params["k"]).unwrap(),
                json!(1.5)
            );
            n += 1;
        },
    )
    .unwrap();
    assert_eq!(n, vals.len());
}

/// Two swept kernels over different inputs (one of them with undefined values: a warming
/// `sma` and non-finite closes), a directly stepped window kernel, an unswept kernel output
/// and a plain input output.
fn mixed() -> Value {
    prog(
        json!([int("p", 10), int("q", 5)]),
        json!([
            {"id": "c", "op": "input", "field": "close"},
            {"id": "h", "op": "input", "field": "high"},
            {"id": "s5", "op": "sma", "x": "c", "period": {"param": "q"}},
            {"id": "e", "op": "ema", "x": "s5", "period": {"param": "p"}},
            {"id": "r", "op": "rsi", "x": "h", "period": {"param": "p"}},
            {"id": "hi", "op": "highest", "x": "h", "period": {"param": "p"}},
            {"id": "sd", "op": "stdev", "x": "s5", "period": {"param": "p"}},
            {"id": "bb", "op": "bollinger", "x": "c", "period": 20, "k": 2.0},
            {"id": "d", "op": "sub", "a": "c", "b": "e"}
        ]),
        json!([
            series("ema_of_sma", "e"),
            series("rsi_high", "r"),
            series("highest", "hi"),
            series("stdev", "sd"),
            series("upper", "bb.upper"),
            series("close", "c"),
            series("sma_q", "s5")
        ]),
    )
}

#[test]
fn mixed_program_several_swept_kernels_and_undefined_inputs() {
    let mut b = bars(400);
    for i in [3, 50, 51, 52, 200, 399] {
        b[i].c = f64::NAN;
    }
    b[120].h = f64::INFINITY;
    let mut ir = mixed();
    // Drop the stateless `sub` node (it would force the fallback; tested below).
    ir["nodes"].as_array_mut().unwrap().pop();
    let opts = ExecOptions::default();
    let sets = one("p", (1..=35).map(|p| json!(p)));
    let path = assert_equal(&ir, None, &sets, &b, &opts);
    assert_eq!(
        path,
        SweepPath::Shared {
            swept: 2,
            direct: 2
        }
    );
    // Sweeping `q`: the warming `sma` itself is swept, and `ema`/`stdev` over it fall back.
    let sets = one("q", (1..=12).map(|p| json!(p)));
    let path = assert_equal(&ir, None, &sets, &b, &opts);
    assert_eq!(path, SweepPath::PerVariation("a kernel over a swept input"));
    // Only some outputs.
    let only = ExecOptions {
        outputs: Some(vec!["rsi_high".to_owned(), "close".to_owned()]),
        ..ExecOptions::default()
    };
    let sets = one("p", (2..=20).map(|p| json!(p)));
    assert_eq!(
        assert_equal(&ir, None, &sets, &b, &only),
        SweepPath::Shared {
            swept: 1,
            direct: 0
        }
    );
    // A parameter no selected output depends on: everything shared.
    let sets = one("q", (2..=6).map(|p| json!(p)));
    assert_eq!(
        assert_equal(&ir, None, &sets, &b, &only),
        SweepPath::Shared {
            swept: 0,
            direct: 0
        }
    );
}

#[test]
fn fallback_per_variation_is_bit_equal() {
    let b = bars(300);
    let mut ir = mixed();
    ir["outputs"]
        .as_array_mut()
        .unwrap()
        .push(series("dist", "d"));
    let sets = one("p", (1..=15).map(|p| json!(p)));
    let path = assert_equal(&ir, None, &sets, &b, &ExecOptions::default());
    assert_eq!(
        path,
        SweepPath::PerVariation("a swept node that is not a kernel")
    );
    let budget = ExecOptions {
        budget: Some(WorkBudget {
            max_work: 2_000,
            record: 1,
            open_record: 1,
            read: 0,
        }),
        ..ExecOptions::default()
    };
    let sets = one("period", (1..=15).map(|p| json!(p)));
    let path = assert_equal(&program("sma"), None, &sets, &b, &budget);
    assert_eq!(path, SweepPath::PerVariation("a work budget"));
}

#[test]
fn errors_come_before_any_variation() {
    let b = bars(50);
    let opts = ExecOptions::default();
    let sets = one("period", [json!(5), json!(0), json!(9)]);
    let r = sweep(&program("sma"), &ctx(None), &sets, &b, &opts, |_| {
        panic!("visited")
    });
    match r {
        Err(SweepError::Check { index, errors }) => {
            assert_eq!(index, 1);
            assert!(!errors.is_empty());
        }
        other => panic!("{other:?}"),
    }
    let only = ExecOptions {
        outputs: Some(vec!["nope".to_owned()]),
        ..ExecOptions::default()
    };
    let sets = one("period", [json!(5)]);
    let r = sweep(&program("sma"), &ctx(None), &sets, &b, &only, |_| {
        panic!("visited")
    });
    assert!(matches!(r, Err(SweepError::Exec(_))), "{r:?}");
    let ev = prog(
        json!([int("period", 5)]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "s", "op": "sma", "x": "c", "period": {"param": "period"}},
               {"id": "g", "op": "gt", "a": "c", "b": "s"}]),
        json!([series("sma", "s"), {"name": "up", "kind": "event", "node": "g"}]),
    );
    let only = ExecOptions {
        outputs: Some(vec!["up".to_owned()]),
        ..ExecOptions::default()
    };
    let r = sweep(&ev, &ctx(None), &sets, &b, &only, |_| panic!("visited"));
    assert_eq!(r, Err(SweepError::NotSeries("up".to_owned())));
    // Event outputs are skipped by default; the series one is swept.
    let path = assert_equal(
        &ev,
        None,
        &one("period", (2..=9).map(|p| json!(p))),
        &b,
        &opts,
    );
    assert_eq!(
        path,
        SweepPath::Shared {
            swept: 1,
            direct: 0
        }
    );
    // No sets: nothing to do.
    let r = sweep(&program("sma"), &ctx(None), &[], &b, &opts, |_| {
        panic!("visited")
    });
    assert_eq!(r, Ok(SweepPath::PerVariation("no parameter sets")));
}
