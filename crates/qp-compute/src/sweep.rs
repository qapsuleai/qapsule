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

/// A single-input study that can produce one bar's outputs as plain `f64` (NaN = none).
trait Kernel: Core {
    /// Variants stepped together (tuned per study by measurement, see BENCH.md).
    const LANES: usize;
    /// Peek then commit: the outputs of bar `x` (first `OUTPUTS.len()` slots used).
    fn step(&mut self, x: f64) -> [f64; 4];
}

const NAN: f64 = f64::NAN;

impl Kernel for Sma {
    const LANES: usize = SMA_LANES;
    #[inline]
    fn step(&mut self, x: f64) -> [f64; 4] {
        let v = self.peek1(x).unwrap_or(NAN);
        self.commit1(x);
        [v, 0.0, 0.0, 0.0]
    }
}

impl Kernel for Ema {
    const LANES: usize = EMA_LANES;
    #[inline]
    fn step(&mut self, x: f64) -> [f64; 4] {
        let v = self.peek1(x).unwrap_or(NAN);
        self.commit1(x);
        [v, 0.0, 0.0, 0.0]
    }
}

impl Kernel for Rsi {
    const LANES: usize = RSI_LANES;
    #[inline]
    fn step(&mut self, x: f64) -> [f64; 4] {
        let v = self.peek1(x).unwrap_or(NAN);
        self.commit1(x);
        [v, 0.0, 0.0, 0.0]
    }
}

impl Kernel for Macd {
    const LANES: usize = MACD_LANES;
    #[inline]
    fn step(&mut self, x: f64) -> [f64; 4] {
        let v = self.peek1(x);
        self.commit1(x);
        [
            v.macd.unwrap_or(NAN),
            v.signal.unwrap_or(NAN),
            v.histogram.unwrap_or(NAN),
            0.0,
        ]
    }
}

impl Kernel for Bollinger {
    const LANES: usize = BOLLINGER_LANES;
    #[inline]
    fn step(&mut self, x: f64) -> [f64; 4] {
        match self.step1(x) {
            Some(v) => [v.middle, v.upper, v.lower, v.bandwidth],
            None => [NAN; 4],
        }
    }
}

// Variants stepped together per study. Each study's per-bar update is a short chain of
// dependent operations; stepping several independent variants in the same loop lets the
// CPU overlap those chains. Each lane runs exactly the arithmetic of a single run, so
// results stay bit-identical. Values chosen by measurement (rounds/R1/BENCH.md).
const SMA_LANES: usize = 1;
const EMA_LANES: usize = 2;
const RSI_LANES: usize = 4;
const MACD_LANES: usize = 2;
const BOLLINGER_LANES: usize = 1;

/// Largest lane count any study uses (buffer pool size).
const MAX_LANES: usize = 8;

fn prepare<K: Kernel>(k: &K, n: usize, buf: &mut SweepColumns) {
    buf.names = K::OUTPUTS;
    buf.warmup = k.warmup();
    buf.columns.resize_with(K::OUTPUTS.len(), Vec::new);
    for col in &mut buf.columns {
        col.clear();
        col.resize(n, 0.0);
    }
}

/// Steps `L` kernels over `x` together; lane `l` writes its `N` outputs into
/// `bufs[l].columns`.
fn step_lanes<K: Kernel, const L: usize, const N: usize>(
    ks: &mut [K],
    x: &[f64],
    bufs: &mut [SweepColumns],
) {
    let ks: &mut [K; L] = ks.try_into().expect("L kernels");
    let n = x.len();
    // Flat list of output slices, lane-major: cols[l * N + j].
    let mut cols: Vec<&mut [f64]> = bufs[..L]
        .iter_mut()
        .flat_map(|b| b.columns.iter_mut().map(|c| &mut c[..n]))
        .collect();
    assert_eq!(cols.len(), L * N);
    for (i, &xi) in x.iter().enumerate() {
        let mut outs = [[0.0; 4]; L];
        for l in 0..L {
            outs[l] = ks[l].step(xi);
        }
        for l in 0..L {
            for j in 0..N {
                cols[l * N + j][i] = outs[l][j];
            }
        }
    }
}

fn step_lanes_n<K: Kernel, const L: usize>(ks: &mut [K], x: &[f64], bufs: &mut [SweepColumns]) {
    match K::OUTPUTS.len() {
        1 => step_lanes::<K, L, 1>(ks, x, bufs),
        3 => step_lanes::<K, L, 3>(ks, x, bufs),
        4 => step_lanes::<K, L, 4>(ks, x, bufs),
        n => unreachable!("no kernel with {n} outputs"),
    }
}

/// Steps `ks` (1, 2, 4 or 8 kernels; any other count runs one by one) over `x`.
fn run_lanes<K: Kernel>(ks: &mut [K], x: &[f64], bufs: &mut [SweepColumns]) {
    for (k, b) in ks.iter().zip(bufs.iter_mut()) {
        prepare(k, x.len(), b);
    }
    match ks.len() {
        8 => step_lanes_n::<K, 8>(ks, x, bufs),
        4 => step_lanes_n::<K, 4>(ks, x, bufs),
        2 => step_lanes_n::<K, 2>(ks, x, bufs),
        _ => {
            for (k, b) in ks.iter_mut().zip(bufs.iter_mut()) {
                step_lanes_n::<K, 1>(std::slice::from_mut(k), x, std::slice::from_mut(b));
            }
        }
    }
}

fn sweep_kernel<K: Kernel>(
    build: impl Fn(&BTreeMap<String, f64>) -> Result<K, ComputeError>,
    full: &[BTreeMap<String, f64>],
    x: &[f64],
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let lanes = lanes.unwrap_or(K::LANES).clamp(1, MAX_LANES);
    for (ci, chunk) in full.chunks(lanes).enumerate() {
        let mut ks: Vec<K> = chunk.iter().map(&build).collect::<Result<_, _>>()?;
        run_lanes(&mut ks, x, &mut bufs[..chunk.len()]);
        for (l, b) in bufs[..chunk.len()].iter().enumerate() {
            visit(ci * lanes + l, b);
        }
    }
    Ok(())
}

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
    match name {
        "sma" => sweep_kernel(
            |p| Sma::new(period_param(p, "period")?),
            &full,
            x,
            lanes,
            &mut bufs,
            v,
        ),
        "ema" => sweep_kernel(
            |p| Ema::new(period_param(p, "period")?),
            &full,
            x,
            lanes,
            &mut bufs,
            v,
        ),
        "rsi" => sweep_kernel(
            |p| Rsi::new(period_param(p, "period")?),
            &full,
            x,
            lanes,
            &mut bufs,
            v,
        ),
        "macd" => sweep_kernel(
            |p| {
                Macd::new(
                    period_param(p, "fast")?,
                    period_param(p, "slow")?,
                    period_param(p, "signal")?,
                )
            },
            &full,
            x,
            lanes,
            &mut bufs,
            v,
        ),
        "bollinger" => sweep_kernel(
            |p| Bollinger::new(period_param(p, "period")?, p["k"]),
            &full,
            x,
            lanes,
            &mut bufs,
            v,
        ),
        _ => unreachable!("validated by create"),
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
