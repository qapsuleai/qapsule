//! RP1/E4: `sweep_chains` = nested `Chain` single runs, bit for bit, for any output index,
//! depth and series length; the result cache returns the same bits as a fresh run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;

use qp_compute::graph::{content_hash, sweep_chains, ResultCache, Stage};
use qp_compute::sweep::{bench_closes, sweep_each};
use qp_compute::{create, Chain, Field, Study};

fn p(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

fn st(study: &str, kv: &[(&str, f64)], output: usize) -> Stage {
    Stage {
        study: study.to_owned(),
        params: p(kv),
        output,
    }
}

/// Nested `Chain` reference, outputs as `Option`s.
fn reference(chain: &[Stage], x: &[f64]) -> Vec<Vec<Option<u64>>> {
    let mut s: Box<dyn Study> = create(&chain[0].study, &chain[0].params, Field::Close).unwrap();
    let mut which = chain[0].output;
    for st in &chain[1..] {
        let next = create(&st.study, &st.params, Field::Close).unwrap();
        s = Box::new(Chain::new(s, which, next));
        which = st.output;
    }
    let mut outs = vec![Vec::new(); s.outputs().len()];
    let mut o = vec![None; s.outputs().len()];
    for &xi in x {
        s.push(&[xi], &mut o);
        for (col, v) in outs.iter_mut().zip(&o) {
            col.push(v.map(f64::to_bits));
        }
    }
    outs
}

fn chains() -> Vec<Vec<Stage>> {
    let mut v = Vec::new();
    for s in [3.0, 9.0] {
        v.push(vec![
            st("macd", &[("fast", 5.0), ("slow", 13.0), ("signal", s)], 1),
            st("ema", &[("period", 4.0)], 0),
        ]);
        v.push(vec![
            st("macd", &[("fast", 5.0), ("slow", 13.0), ("signal", s)], 2),
            st("rsi", &[("period", 6.0)], 0),
        ]);
    }
    for k in [1.0, 2.5] {
        v.push(vec![
            st("bollinger", &[("period", 10.0), ("k", k)], 1),
            st("rsi", &[("period", 5.0)], 0),
        ]);
        v.push(vec![
            st("bollinger", &[("period", 10.0), ("k", k)], 3),
            st("sma", &[("period", 7.0)], 0),
        ]);
    }
    for r in [2.0, 3.0, 14.0] {
        v.push(vec![
            st("sma", &[("period", 3.0)], 0),
            st("ema", &[("period", 5.0)], 0),
            st("rsi", &[("period", r)], 0),
            st("bollinger", &[("period", 6.0), ("k", 2.0)], 0),
        ]);
    }
    v.push(vec![st("rsi", &[("period", 30.0)], 0)]); // depth 1
    v.push(vec![
        st("sma", &[("period", 3.0)], 0),
        st("ema", &[("period", 5.0)], 0),
    ]); // repeat prefix later
    v
}

#[test]
fn chain_sweep_equals_nested_chain() {
    for n in [0, 1, 12, 40, 700] {
        let x = bench_closes(n, 31 + n as u64);
        let cs = chains();
        let mut seen = 0;
        sweep_chains(&cs, &x, |ci, c| {
            assert_eq!(ci, seen, "chains visited in order");
            seen += 1;
            let r = reference(&cs[ci], &x);
            for (j, col) in c.columns.iter().enumerate() {
                assert_eq!(col.len(), n);
                let got: Vec<Option<u64>> = col
                    .iter()
                    .map(|v| (!v.is_nan()).then(|| v.to_bits()))
                    .collect();
                assert_eq!(got, r[j], "chain {ci} output {j} n={n}");
            }
        })
        .unwrap();
        assert_eq!(seen, cs.len());
    }
    let x = bench_closes(50, 1);
    assert!(sweep_chains(&[vec![]], &x, |_, _| {}).is_err());
    assert!(sweep_chains(&[vec![st("rsi", &[], 1)]], &x, |_, _| panic!()).is_err());
    assert!(sweep_chains(
        &[vec![st("rsi", &[], 1), st("ema", &[], 0)]],
        &x,
        |_, _| panic!()
    )
    .is_err());
}

#[test]
fn result_cache_returns_the_same_bits() {
    let x = bench_closes(500, 5);
    let h = content_hash(&x);
    assert_ne!(h, content_hash(&x[..499]));
    let mut cache = ResultCache::new();
    let params = p(&[("period", 9.0)]);
    let a = cache.get_or_compute(h, "rsi", &params, &x).unwrap();
    let b = cache
        .get_or_compute(h, "rsi", &BTreeMap::from([("period".to_owned(), 9.0)]), &x)
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(&a, &b), "second call is a hit");
    assert_eq!(cache.len(), 1);
    let mut fresh = None;
    sweep_each("rsi", &[params], &x, |_, c| fresh = Some(c.clone())).unwrap();
    let fresh = fresh.unwrap();
    let bits = |c: &qp_compute::sweep::SweepColumns| -> Vec<u64> {
        c.columns[0].iter().map(|v| v.to_bits()).collect()
    };
    assert_eq!(bits(&a), bits(&fresh));
    // Defaults are part of the key: `{}` and `{period: 14}` are the same study.
    let d1 = cache
        .get_or_compute(h, "rsi", &BTreeMap::new(), &x)
        .unwrap();
    let d2 = cache
        .get_or_compute(h, "rsi", &p(&[("period", 14.0)]), &x)
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(&d1, &d2));
}
