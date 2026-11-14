//! Parameter sweep: run one study with many parameter sets over the same input in one
//! call ("the agent thinks by computing", NORTH_STAR principle 4).
//!
//! Every variant's output is **bit-identical** to running that parameter set alone
//! ([`crate::run`] / [`crate::Study::push`]): the sweep kernels call the very same
//! `peek1`/`commit1` functions, monomorphized per study, without the `dyn Study` and
//! `Option` overhead. Only the memory layout differs: a variant's outputs are written as
//! `f64` columns with `NaN` in the warmup slots (their count is [`SweepColumns::warmup`]),
//! into a buffer that is reused for the next variant. Use [`sweep_each`] to reduce each
//! variant on the fly (constant memory) or [`sweep`] to materialize everything.

use std::collections::BTreeMap;

use crate::lanes;
use crate::studies::{Bollinger, Ema, Macd, Rsi, Sma};
use crate::{create, period_param, ComputeError, Core, Field};

/// One variant's outputs: `columns[j][i]` is output `j` at bar `i`; `NaN` during warmup.
#[derive(Debug, Clone)]
pub struct SweepColumns {
    /// Output names, e.g. `["macd", "signal", "histogram"]`.
    pub names: &'static [&'static str],
    /// One column per output, each as long as the input.
    pub columns: Vec<Vec<f64>>,
    /// Leading warmup length per output (those slots hold `NaN`).
    pub warmup: Vec<usize>,
}

impl SweepColumns {
    /// Output `j` at bar `i` as the public `Option` form (`None` during warmup).
    #[must_use]
    pub fn get(&self, j: usize, i: usize) -> Option<f64> {
        if i < self.warmup[j] {
            None
        } else {
            Some(self.columns[j][i])
        }
    }

    /// Column `j` as `Option`s (K13 form).
    #[must_use]
    pub fn to_options(&self, j: usize) -> Vec<Option<f64>> {
        (0..self.columns[j].len()).map(|i| self.get(j, i)).collect()
    }
}

/// Largest lane count (buffer pool size).
const MAX_LANES: usize = 8;

/// Full parameter map for `name` (defaults filled in, unknown keys rejected) — the same
/// validation as [`create`].
fn full_params(
    name: &str,
    params: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, f64>, ComputeError> {
    // `create` performs all validation; reuse it, then read back the effective params.
    Ok(create(name, params, Field::Close)?.params())
}

/// Runs `name` once per parameter set over `x` and hands each variant's outputs to
/// `visit(variant_index, &columns)`. The buffer is reused, so memory stays at one
/// variant's outputs. All parameter sets are validated before any work starts.
///
/// # Errors
/// Unknown study or an invalid parameter set (nothing is computed in that case).
pub fn sweep_each(
    name: &str,
    param_sets: &[BTreeMap<String, f64>],
    x: &[f64],
    visit: impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    sweep_each_lanes(name, param_sets, x, None, visit)
}

/// [`sweep_each`] with an explicit lane count (1, 2, 4 or 8; `None` = the study's
/// default). Results are identical for every lane count; only speed differs. Exposed for
/// the benchmark's lane tuning.
///
/// # Errors
/// See [`sweep_each`].
#[doc(hidden)]
pub fn sweep_each_lanes(
    name: &str,
    param_sets: &[BTreeMap<String, f64>],
    x: &[f64],
    lanes: Option<usize>,
    mut visit: impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    if !crate::STUDY_NAMES.contains(&name) {
        // Validate the name even when there are no parameter sets.
        create(name, &BTreeMap::new(), Field::Close)?;
    }
    let full: Vec<BTreeMap<String, f64>> = param_sets
        .iter()
        .map(|p| full_params(name, p))
        .collect::<Result<_, _>>()?;
    let mut bufs: Vec<SweepColumns> = (0..MAX_LANES)
        .map(|_| SweepColumns {
            names: &[],
            columns: Vec::new(),
            warmup: Vec::new(),
        })
        .collect();
    let v = &mut visit;
    if let Some(kind) = Soa::of(name) {
        let jobs: Vec<Job> = full.iter().map(Job::of).collect::<Result<_, _>>()?;
        let lanes = lanes.map_or(kind.default_lanes(), |l| l.clamp(1, MAX_LANES));
        let xs: Vec<&[f64]> = vec![x; jobs.len()];
        soa_run(kind, &jobs, &xs, lanes, &mut bufs, &mut |i, c| v(i, c));
        return Ok(());
    }
    unreachable!("every built-in study has a lane kernel")
}

/// Built-in studies with a structure-of-arrays lane kernel ([`crate::lanes`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Soa {
    Sma,
    Ema,
    Rsi,
    Macd,
    Bollinger,
}

/// One run's parameters: periods (`period`, or `fast, slow, signal`) and `k`.
#[derive(Debug, Clone, Copy, Default)]
struct Job {
    p: [usize; 3],
    k: f64,
}

impl Job {
    fn of(params: &BTreeMap<String, f64>) -> Result<Self, ComputeError> {
        let get = |key: &str| params.get(key).map(|_| period_param(params, key));
        let mut j = Self::default();
        if let Some(p) = get("period") {
            j.p[0] = p?;
        } else {
            j.p = [
                period_param(params, "fast")?,
                period_param(params, "slow")?,
                period_param(params, "signal")?,
            ];
        }
        j.k = params.get("k").copied().unwrap_or(0.0);
        Ok(j)
    }
}

impl Soa {
    fn of(name: &str) -> Option<Self> {
        match name {
            "sma" => Some(Self::Sma),
            "ema" => Some(Self::Ema),
            "rsi" => Some(Self::Rsi),
            "macd" => Some(Self::Macd),
            "bollinger" => Some(Self::Bollinger),
            _ => None,
        }
    }

    /// Lanes per kernel call, chosen by measurement (RP1 E2).
    fn default_lanes(self) -> usize {
        match self {
            Self::Sma => SOA_SMA_LANES,
            Self::Ema => SOA_EMA_LANES,
            Self::Rsi => SOA_RSI_LANES,
            Self::Macd => SOA_MACD_LANES,
            Self::Bollinger => SOA_BOLLINGER_LANES,
        }
    }

    fn outputs(self) -> &'static [&'static str] {
        match self {
            Self::Sma => Sma::OUTPUTS,
            Self::Ema => Ema::OUTPUTS,
            Self::Rsi => Rsi::OUTPUTS,
            Self::Macd => Macd::OUTPUTS,
            Self::Bollinger => Bollinger::OUTPUTS,
        }
    }

    fn warmup(self, j: &Job, w: &mut Vec<usize>) {
        w.clear();
        match self {
            Self::Sma | Self::Ema => w.push(j.p[0] - 1),
            Self::Rsi => w.push(j.p[0]),
            Self::Macd => {
                let m = j.p[1] - 1;
                let s = m + j.p[2] - 1;
                w.extend([m, s, s]);
            }
            Self::Bollinger => w.extend([j.p[0] - 1; 4]),
        }
    }

    fn run<const L: usize>(self, xs: &[&[f64]; L], jobs: &[Job; L], outs: &mut [&mut [f64]]) {
        let p0: [usize; L] = std::array::from_fn(|l| jobs[l].p[0]);
        match self {
            Self::Sma => lanes::sma::<L>(xs, &p0, outs),
            Self::Ema => lanes::ema::<L>(xs, &p0, outs),
            Self::Rsi => lanes::rsi::<L>(xs, &p0, outs),
            Self::Macd => {
                let p = std::array::from_fn(|l| (jobs[l].p[0], jobs[l].p[1], jobs[l].p[2]));
                lanes::macd::<L>(xs, &p, outs);
            }
            Self::Bollinger => {
                let p = std::array::from_fn(|l| (jobs[l].p[0], jobs[l].k));
                lanes::bollinger::<L>(xs, &p, outs);
            }
        }
    }
}

// Lanes per study and target, chosen by measurement (rounds/RP1/experiments/E2-simd.md).
// Too many lanes spill vector registers (16 on x86_64 SSE2, 32 on aarch64); wasm32 in V8
// gains little from lanes for the wide-state studies.
#[cfg(target_arch = "x86_64")]
const SOA_LANES: [usize; 5] = [2, 4, 4, 2, 2];
#[cfg(target_arch = "wasm32")]
const SOA_LANES: [usize; 5] = [1, 2, 2, 1, 1];
#[cfg(not(any(target_arch = "x86_64", target_arch = "wasm32")))]
const SOA_LANES: [usize; 5] = [4, 4, 4, 2, 2];
const SOA_SMA_LANES: usize = SOA_LANES[0];
const SOA_EMA_LANES: usize = SOA_LANES[1];
const SOA_RSI_LANES: usize = SOA_LANES[2];
const SOA_MACD_LANES: usize = SOA_LANES[3];
const SOA_BOLLINGER_LANES: usize = SOA_LANES[4];

/// Runs `kind` for job `j` = (`xs[j]`, `jobs[j]`), up to `lanes` jobs per kernel call,
/// and calls `visit(j, columns)` in job order.
fn soa_run(
    kind: Soa,
    jobs: &[Job],
    xs: &[&[f64]],
    lanes: usize,
    bufs: &mut [SweepColumns],
    visit: &mut dyn FnMut(usize, &SweepColumns),
) {
    match lanes {
        8.. => soa_chunks::<8>(kind, jobs, xs, bufs, visit),
        4..=7 => soa_chunks::<4>(kind, jobs, xs, bufs, visit),
        2 | 3 => soa_chunks::<2>(kind, jobs, xs, bufs, visit),
        _ => soa_chunks::<1>(kind, jobs, xs, bufs, visit),
    }
}

fn soa_prepare(kind: Soa, job: &Job, n: usize, buf: &mut SweepColumns) {
    buf.names = kind.outputs();
    kind.warmup(job, &mut buf.warmup);
    buf.columns.resize_with(buf.names.len(), Vec::new);
    for col in &mut buf.columns {
        col.clear();
        col.resize(n, 0.0);
    }
}

fn soa_chunks<const L: usize>(
    kind: Soa,
    jobs: &[Job],
    xs: &[&[f64]],
    bufs: &mut [SweepColumns],
    visit: &mut dyn FnMut(usize, &SweepColumns),
) {
    // Full chunks of L; the tail (< L jobs) runs with fewer lanes instead of padding.
    let full = jobs.len() / L * L;
    let tail = jobs.len() - full;
    if L > 1 {
        let mut j0 = 0;
        while j0 < full {
            soa_chunk::<L>(kind, &jobs[j0..j0 + L], &xs[j0..j0 + L], bufs, j0, visit);
            j0 += L;
        }
        if tail > 0 {
            let (jt, xt) = (&jobs[full..], &xs[full..]);
            let mut shifted = |j: usize, c: &SweepColumns| visit(full + j, c);
            match tail {
                4.. => soa_chunks::<4>(kind, jt, xt, bufs, &mut shifted),
                2 | 3 => soa_chunks::<2>(kind, jt, xt, bufs, &mut shifted),
                _ => soa_chunks::<1>(kind, jt, xt, bufs, &mut shifted),
            }
        }
    } else {
        for j in 0..jobs.len() {
            soa_chunk::<1>(kind, &jobs[j..=j], &xs[j..=j], bufs, j, visit);
        }
    }
}

/// Exactly `L` jobs in one kernel call.
fn soa_chunk<const L: usize>(
    kind: Soa,
    jobs: &[Job],
    xs: &[&[f64]],
    bufs: &mut [SweepColumns],
    j0: usize,
    visit: &mut dyn FnMut(usize, &SweepColumns),
) {
    let js: [Job; L] = std::array::from_fn(|l| jobs[l]);
    let lane_xs: [&[f64]; L] = std::array::from_fn(|l| xs[l]);
    let n = lane_xs[0].len();
    for (l, b) in bufs[..L].iter_mut().enumerate() {
        soa_prepare(kind, &js[l], n, b);
    }
    {
        let mut outs: Vec<&mut [f64]> = bufs[..L]
            .iter_mut()
            .flat_map(|b| b.columns.iter_mut().map(|c| &mut c[..]))
            .collect();
        kind.run::<L>(&lane_xs, &js, &mut outs);
    }
    for (l, b) in bufs[..L].iter().enumerate() {
        visit(j0 + l, b);
    }
}

/// Runs one study with one parameter set over many series (e.g. every symbol and
/// timeframe of a market) and calls `visit(series_index, &columns)` in series order.
/// Outputs are bit-identical to running each series alone.
///
/// Series of equal length are stepped together in lanes (sma, ema, rsi); to keep the
/// visiting order, up to [`SERIES_WINDOW`] series are computed before they are visited
/// (memory: that many series' outputs).
///
/// # Errors
/// Unknown study or invalid parameters (nothing is computed in that case).
pub fn sweep_series_each(
    name: &str,
    params: &BTreeMap<String, f64>,
    series: &[&[f64]],
    mut visit: impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let full = full_params(name, params)?;
    let kind = Soa::of(name).expect("validated by full_params");
    let job = Job::of(&full)?;
    let lanes = kind.default_lanes();
    let mut held: Vec<SweepColumns> = Vec::new();
    let mut scratch: Vec<SweepColumns> = (0..MAX_LANES).map(|_| empty_columns()).collect();
    for (w, win) in series.chunks(SERIES_WINDOW).enumerate() {
        let base = w * SERIES_WINDOW;
        // Group the window's series by length (stable), run each group in lanes.
        let mut order: Vec<usize> = (0..win.len()).collect();
        order.sort_by_key(|&i| (std::cmp::Reverse(win[i].len()), i));
        held.resize_with(win.len(), empty_columns);
        let mut g0 = 0;
        while g0 < order.len() {
            let len = win[order[g0]].len();
            let mut g1 = g0;
            while g1 < order.len() && win[order[g1]].len() == len {
                g1 += 1;
            }
            let idx = &order[g0..g1];
            let xs: Vec<&[f64]> = idx.iter().map(|&i| win[i]).collect();
            let jobs = vec![job; idx.len()];
            soa_run(kind, &jobs, &xs, lanes, &mut scratch, &mut |j, c| {
                held[idx[j]].clone_from(c);
            });
            g0 = g1;
        }
        for (i, c) in held.iter().enumerate() {
            visit(base + i, c);
        }
    }
    Ok(())
}

/// Series computed before being visited by [`sweep_series_each`].
pub const SERIES_WINDOW: usize = 64;

fn empty_columns() -> SweepColumns {
    SweepColumns {
        names: &[],
        columns: Vec::new(),
        warmup: Vec::new(),
    }
}

/// Like [`sweep_each`] but keeps every variant's outputs (memory: variants × outputs ×
/// bars × 8 bytes).
///
/// # Errors
/// See [`sweep_each`].
pub fn sweep(
    name: &str,
    param_sets: &[BTreeMap<String, f64>],
    x: &[f64],
) -> Result<Vec<SweepColumns>, ComputeError> {
    let mut all = Vec::with_capacity(param_sets.len());
    sweep_each(name, param_sets, x, |_, c| all.push(c.clone()))?;
    Ok(all)
}

/// The parameter grid used by the R1 benchmark (`examples/sweep_bench.rs`, the wasm
/// bench and `rounds/R1/BENCH.md`): 1,000 variants per study.
///
/// - `sma`, `ema`, `rsi`: period 2..=1001
/// - `macd`: fast 2..=11 × slow fast+1..=fast+10 (step 2: +1,+3,..,+19) × signal 2..=11
/// - `bollinger`: period 2..=101 × k 1.0, 1.25, .., 3.25
#[must_use]
pub fn bench_grid(name: &str) -> Vec<BTreeMap<String, f64>> {
    let one = |k: &str, v: f64| BTreeMap::from([(k.to_owned(), v)]);
    match name {
        "sma" | "ema" | "rsi" => (2..=1001).map(|p| one("period", f64::from(p))).collect(),
        "macd" => {
            let mut v = Vec::with_capacity(1000);
            for fast in 2..=11 {
                for d in 0..10 {
                    for signal in 2..=11 {
                        v.push(BTreeMap::from([
                            ("fast".to_owned(), f64::from(fast)),
                            ("slow".to_owned(), f64::from(fast + 1 + 2 * d)),
                            ("signal".to_owned(), f64::from(signal)),
                        ]));
                    }
                }
            }
            v
        }
        "bollinger" => {
            let mut v = Vec::with_capacity(1000);
            for p in 2..=101 {
                for k in 0..10 {
                    v.push(BTreeMap::from([
                        ("period".to_owned(), f64::from(p)),
                        ("k".to_owned(), 1.0 + 0.25 * f64::from(k)),
                    ]));
                }
            }
            v
        }
        _ => Vec::new(),
    }
}

/// Deterministic synthetic close series for benchmarks and digests: a random walk from
/// 100 with two-decimal prices, integer LCG only (identical on every target).
#[must_use]
pub fn bench_closes(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    let mut x = 100.0_f64;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            // High 32 bits -> [-1, 1).
            let u = f64::from(u32::try_from(s >> 32).unwrap_or(0)) / 2_147_483_648.0 - 1.0;
            x = ((x * (1.0 + 0.02 * u)) * 100.0).round() / 100.0;
            if x < 1.0 {
                x = 1.0;
            }
            x
        })
        .collect()
}

/// FNV-1a over the bits of every output value of every variant (NaN warmup slots
/// included, they are the canonical NaN). Used by the cross-target digest.
#[must_use]
pub fn fnv1a_columns(acc: u64, cols: &SweepColumns) -> u64 {
    let mut h = acc;
    for col in &cols.columns {
        for v in col {
            for b in v.to_bits().to_le_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    h
}

/// FNV-1a offset basis.
pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The cross-target compute digest (`verify.sh` compares it across aarch64, x86_64 and
/// wasm32): FNV-1a over
/// 1. every output of every variant of the benchmark grid of all five studies on
///    `bench_closes(2_000, 7)` (sweep path), and
/// 2. every output of all five studies with default parameters on the same series via
///    the `Study` trait, every bar first pushed with a wrong price and then revised
///    (streaming + revise path).
#[must_use]
pub fn cross_target_digest() -> u64 {
    let x = bench_closes(2_000, 7);
    let mut h = FNV_OFFSET;
    for name in crate::STUDY_NAMES {
        sweep_each(name, &bench_grid(name), &x, |_, c| h = fnv1a_columns(h, c))
            .expect("benchmark grid is valid");
        let mut s = create(name, &BTreeMap::new(), Field::Close).expect("defaults are valid");
        let mut o = vec![None; s.outputs().len()];
        for &xi in &x {
            s.push(&[xi * 1.01], &mut o);
            s.update_last(&[xi], &mut o);
            for v in &o {
                for b in v.map_or(u64::MAX, f64::to_bits).to_le_bytes() {
                    h ^= u64::from(b);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                }
            }
        }
    }
    h
}
