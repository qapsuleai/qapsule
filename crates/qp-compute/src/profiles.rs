//! Benchmark load profiles (RP1). Each profile fixes a workload (input data, parameter
//! sets, requested outputs) and a digest of every output value, so an implementation
//! change can be timed *and* checked for bit-identical results.
//!
//! - **A** — one series, many parameter sets: 10,000 bars × 1,000 variants per study
//!   (the R1 benchmark, [`crate::sweep::bench_grid`]).
//! - **B** — many series, one study: 500 symbols × 5 timeframes (10,000 / 5,000 / 2,000 /
//!   1,000 / 500 bars; 2,500 series, 9.25 M bars), each study with default parameters.
//! - **C** — chained studies: 1,000 variants of a chain per family on 10,000 bars
//!   (`ema>rsi`, `rsi>bollinger`, `sma>ema>rsi`); every stage reads output 0 of the
//!   previous one, with [`crate::Chain`] semantics.
//! - **D** — live update: a bank of 500 studies (100 of each built-in, parameters from the
//!   benchmark grids) on one series of 10,000 bars; then ticks revise the last bar
//!   (`update_last`) and new bars arrive (`push`) on all 500 at once.
//!
//! Not a public API: used by the benchmark driver, the tests and the round reports.

use std::collections::BTreeMap;

use crate::sweep::{bench_closes, bench_grid, sweep_each, SweepColumns};
use crate::{create, default_params, Chain, Field, Study};

/// Profile A series length.
pub const A_BARS: usize = 10_000;
/// Profile A series seed (same as R1 `BENCH.md`).
pub const A_SEED: u64 = 20_261_114;

/// Profile A input series.
#[must_use]
pub fn a_series() -> Vec<f64> {
    bench_closes(A_BARS, A_SEED)
}

/// Profile A, study `name`: the R1 sweep over [`bench_grid`].
///
/// # Panics
/// Unknown study.
pub fn a_run(name: &str, x: &[f64], visit: impl FnMut(usize, &SweepColumns)) {
    sweep_each(name, &bench_grid(name), x, visit).expect("benchmark grid is valid");
}

/// Profile B: number of symbols.
pub const B_SYMBOLS: usize = 500;
/// Profile B: series length per timeframe (e.g. 1m, 5m, 1h, 1d, 1w windows).
pub const B_LENGTHS: [usize; 5] = [10_000, 5_000, 2_000, 1_000, 500];

/// Profile B input: `B_SYMBOLS * B_LENGTHS.len()` series, symbol-major.
#[must_use]
pub fn b_series() -> Vec<Vec<f64>> {
    let mut v = Vec::with_capacity(B_SYMBOLS * B_LENGTHS.len());
    for s in 0..B_SYMBOLS {
        for (t, &n) in B_LENGTHS.iter().enumerate() {
            v.push(bench_closes(n, 1_000 + (s * B_LENGTHS.len() + t) as u64));
        }
    }
    v
}

/// Profile B, study `name` with default parameters over every series;
/// `visit(series_index, outputs)`.
///
/// # Panics
/// Unknown study.
pub fn b_run(name: &str, series: &[Vec<f64>], mut visit: impl FnMut(usize, &SweepColumns)) {
    let p = [default_params(name).expect("known study")];
    for (si, x) in series.iter().enumerate() {
        sweep_each(name, &p, x, |_, c| visit(si, c)).expect("defaults are valid");
    }
}

/// Profile C chain families.
pub const C_FAMILIES: [&str; 3] = ["ema>rsi", "rsi>bollinger", "sma>ema>rsi"];

/// One chain variant: stages in order (study name, parameters).
pub type ChainSpec = Vec<(&'static str, BTreeMap<String, f64>)>;

fn period(p: usize) -> BTreeMap<String, f64> {
    BTreeMap::from([("period".to_owned(), p as f64)])
}

/// Profile C grid of a family (1,000 variants each).
///
/// - `ema>rsi`: ema period ∈ {2,3,5,8,13,21,34,55,89,144} × rsi period 2..=101
/// - `rsi>bollinger`: rsi period ∈ {2,3,5,7,9,14,21,28,35,50} × bollinger period
///   5,10,..,50 × k 1.0,1.25,..,3.25
/// - `sma>ema>rsi`: sma period ∈ {2,3,5,8,13,21,34,55,89,144} × ema period
///   {2,3,4,5,6,8,10,12,15,20} × rsi period {2,3,5,7,9,14,21,28,35,50}
///
/// # Panics
/// Unknown family.
#[must_use]
pub fn c_grid(family: &str) -> Vec<ChainSpec> {
    const FIB: [usize; 10] = [2, 3, 5, 8, 13, 21, 34, 55, 89, 144];
    const RSI: [usize; 10] = [2, 3, 5, 7, 9, 14, 21, 28, 35, 50];
    const EMA: [usize; 10] = [2, 3, 4, 5, 6, 8, 10, 12, 15, 20];
    let mut v = Vec::with_capacity(1000);
    match family {
        "ema>rsi" => {
            for &e in &FIB {
                for r in 2..=101 {
                    v.push(vec![("ema", period(e)), ("rsi", period(r))]);
                }
            }
        }
        "rsi>bollinger" => {
            for &r in &RSI {
                for b in 1..=10 {
                    for k in 0..10 {
                        v.push(vec![
                            ("rsi", period(r)),
                            (
                                "bollinger",
                                BTreeMap::from([
                                    ("period".to_owned(), (5 * b) as f64),
                                    ("k".to_owned(), 1.0 + 0.25 * f64::from(k)),
                                ]),
                            ),
                        ]);
                    }
                }
            }
        }
        "sma>ema>rsi" => {
            for &s in &FIB {
                for &e in &EMA {
                    for &r in &RSI {
                        v.push(vec![
                            ("sma", period(s)),
                            ("ema", period(e)),
                            ("rsi", period(r)),
                        ]);
                    }
                }
            }
        }
        _ => panic!("unknown chain family {family}"),
    }
    v
}

/// Builds a chain variant as one [`Study`] (nested [`Chain`]s reading output 0).
///
/// # Panics
/// Invalid stage.
#[must_use]
pub fn c_build(spec: &ChainSpec) -> Box<dyn Study> {
    let mut it = spec.iter();
    let (n0, p0) = it.next().expect("at least one stage");
    let mut s = create(n0, p0, Field::Close).expect("valid stage");
    for (n, p) in it {
        let next = create(n, p, Field::Close).expect("valid stage");
        s = Box::new(Chain::new(s, 0, next));
    }
    s
}

/// Profile C, one family, with the computation graph ([`crate::graph::sweep_chains`]):
/// every distinct prefix once, the last stage as a lane sweep.
///
/// # Panics
/// Unknown family.
pub fn c_run(family: &str, x: &[f64], visit: impl FnMut(usize, &SweepColumns)) {
    let chains: Vec<Vec<crate::graph::Stage>> = c_grid(family)
        .iter()
        .map(|spec| {
            spec.iter()
                .map(|(n, p)| crate::graph::Stage::new(n, p.clone()))
                .collect()
        })
        .collect();
    crate::graph::sweep_chains(&chains, x, visit).expect("valid chains");
}

/// Profile C as in RP1/T1: one nested [`Chain`] per variant, bar by bar.
pub fn c_run_baseline(family: &str, x: &[f64], mut visit: impl FnMut(usize, &SweepColumns)) {
    let grid = c_grid(family);
    let mut buf = SweepColumns {
        names: &[],
        columns: Vec::new(),
        warmup: Vec::new(),
    };
    let n = x.len();
    for (vi, spec) in grid.iter().enumerate() {
        let mut s = c_build(spec);
        buf.names = s.outputs();
        buf.warmup = s.warmup();
        buf.columns.resize_with(buf.names.len(), Vec::new);
        for c in &mut buf.columns {
            c.clear();
            c.resize(n, 0.0);
        }
        let mut o = vec![None; buf.names.len()];
        for (i, &xi) in x.iter().enumerate() {
            s.push(&[xi], &mut o);
            for (col, v) in buf.columns.iter_mut().zip(&o) {
                col[i] = v.unwrap_or(f64::NAN);
            }
        }
        visit(vi, &buf);
    }
}

/// Profile D: bank size per study.
pub const D_PER_STUDY: usize = 100;
/// Profile D: history length before the timed ticks.
pub const D_BARS: usize = 10_000;
/// Profile D: number of timed ticks (revisions of the last bar).
pub const D_TICKS: usize = 2_000;
/// Profile D: number of timed new bars.
pub const D_NEW_BARS: usize = 1_000;

/// Profile D bank: 100 parameter sets per study, every 10th entry of the benchmark grid.
#[must_use]
pub fn d_specs() -> Vec<(&'static str, BTreeMap<String, f64>)> {
    let mut v = Vec::with_capacity(D_PER_STUDY * 5);
    for name in crate::STUDY_NAMES {
        for p in bench_grid(name).into_iter().step_by(10).take(D_PER_STUDY) {
            v.push((name, p));
        }
    }
    v
}

/// Profile D price stream: `D_BARS` history bars, then `D_TICKS` tick prices for the last
/// bar, then `D_NEW_BARS` new bars. Deterministic.
#[must_use]
pub fn d_prices() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let all = bench_closes(D_BARS + D_NEW_BARS, 4_242);
    let last = all[D_BARS - 1];
    // Ticks wander around the last close by up to ±1% in cents.
    let mut s: u64 = 99;
    let ticks = (0..D_TICKS)
        .map(|_| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let u = f64::from(u32::try_from(s >> 32).unwrap_or(0)) / 2_147_483_648.0 - 1.0;
            ((last * (1.0 + 0.01 * u)) * 100.0).round() / 100.0
        })
        .collect();
    (all[..D_BARS].to_vec(), ticks, all[D_BARS..].to_vec())
}

/// A bank of live studies driven together (profile D).
pub struct Bank {
    studies: Vec<Box<dyn Study>>,
    outs: Vec<Vec<Option<f64>>>,
}

impl Bank {
    /// Builds the bank from specs.
    ///
    /// # Panics
    /// Invalid spec.
    #[must_use]
    pub fn new(specs: &[(&'static str, BTreeMap<String, f64>)]) -> Self {
        let studies: Vec<Box<dyn Study>> = specs
            .iter()
            .map(|(n, p)| create(n, p, Field::Close).expect("valid spec"))
            .collect();
        let outs = studies
            .iter()
            .map(|s| vec![None; s.outputs().len()])
            .collect();
        Self { studies, outs }
    }

    /// A new bar with price `x` on every study.
    pub fn push(&mut self, x: f64) {
        for (s, o) in self.studies.iter_mut().zip(&mut self.outs) {
            s.push(&[x], o);
        }
    }

    /// Revises the last bar to price `x` on every study.
    pub fn update_last(&mut self, x: f64) {
        for (s, o) in self.studies.iter_mut().zip(&mut self.outs) {
            s.update_last(&[x], o);
        }
    }

    /// Folds the current outputs of every study into an FNV-1a digest (`None` = all ones).
    #[must_use]
    pub fn digest(&self, acc: u64) -> u64 {
        let mut h = acc;
        for o in &self.outs {
            for v in o {
                h = fnv_u64(h, v.map_or(u64::MAX, f64::to_bits));
            }
        }
        h
    }

    /// A cheap value depending on the outputs (keeps the work observable in timing runs).
    #[must_use]
    pub fn sink(&self) -> f64 {
        self.outs
            .iter()
            .filter_map(|o| o[0])
            .fold(0.0, |a, v| a + v)
    }
}

/// FNV-1a step over the 8 little-endian bytes of `v`.
#[must_use]
pub fn fnv_u64(acc: u64, v: u64) -> u64 {
    let mut h = acc;
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Profile D digest: warm the bank with the history, then every tick and every new bar,
/// folding all outputs after each operation.
#[must_use]
pub fn d_digest() -> u64 {
    let (hist, ticks, new_bars) = d_prices();
    let mut bank = Bank::new(&d_specs());
    let mut h = crate::sweep::FNV_OFFSET;
    for &x in &hist {
        bank.push(x);
    }
    h = bank.digest(h);
    for &t in &ticks {
        bank.update_last(t);
        h = bank.digest(h);
    }
    for &x in &new_bars {
        bank.push(x);
        h = bank.digest(h);
    }
    h
}
