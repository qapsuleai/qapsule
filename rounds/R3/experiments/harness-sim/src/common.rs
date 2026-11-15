//! Shared pieces: deterministic RNG, statistics, datasets, paths, data perturbation.
//!
//! Everything here is deterministic (seeded integer LCG; no clock). Statistics use the
//! nearest-rank percentile on the sorted sample.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qp_capsule::InstrumentInput;
use qp_facts::FlowPoint;
use qp_schema::capsule::{Direction, EvidenceFamily, MeaningKind, Stage};
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// RNG
// ---------------------------------------------------------------------------

/// 64-bit LCG (Knuth MMIX constants), high 32 bits out. Same constants as `qp-source`.
#[derive(Debug, Clone)]
pub struct Lcg(pub u64);

impl Lcg {
    /// A generator whose state is `mix(seed)` (nearby seeds diverge).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(mix(seed))
    }
    /// Next 32 random bits.
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u32::try_from(self.0 >> 32).unwrap_or(u32::MAX)
    }
    /// Uniform in `[0, 1)`.
    pub fn u01(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }
    /// Uniform in `[-1, 1)`.
    pub fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 2_147_483_648.0 - 1.0
    }
    /// Approximately standard normal (sum of 6 uniforms on `[-1,1)`, scaled).
    pub fn normal(&mut self) -> f64 {
        let mut s = 0.0;
        for _ in 0..6 {
            s += self.unit();
        }
        s / std::f64::consts::SQRT_2
    }
    /// Integer in `[0, n)` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        let n64 = u64::try_from(n.max(1)).unwrap_or(1);
        usize::try_from(u64::from(self.next_u32()) % n64).unwrap_or(0)
    }
}

/// SplitMix64 finaliser.
#[must_use]
pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a 64 of a string.
#[must_use]
pub fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

pub(crate) fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Nearest-rank percentile (`p` in `[0,1]`) of an unsorted sample; `None` if empty.
#[must_use]
pub fn percentile(xs: &[f64], p: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    Some(pct_sorted(&v, p))
}

/// Nearest-rank percentile of a sorted, non-empty sample.
#[must_use]
pub fn pct_sorted(v: &[f64], p: f64) -> f64 {
    let n = v.len();
    #[allow(clippy::cast_precision_loss)]
    let rank = (p * n as f64).ceil();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = (rank as usize).clamp(1, n) - 1;
    v[idx]
}

/// Mean; `None` if empty.
#[must_use]
pub fn mean(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut s = 0.0;
    for &x in xs {
        s += x;
    }
    #[allow(clippy::cast_precision_loss)]
    Some(s / xs.len() as f64)
}

/// Population standard deviation; `None` if empty.
#[must_use]
pub fn std_dev(xs: &[f64]) -> Option<f64> {
    let m = mean(xs)?;
    let mut s = 0.0;
    for &x in xs {
        s += (x - m) * (x - m);
    }
    #[allow(clippy::cast_precision_loss)]
    Some((s / xs.len() as f64).sqrt())
}

/// `a / b` as `f64`, `None` when `b == 0`.
#[must_use]
pub fn ratio(a: u64, b: u64) -> Option<f64> {
    if b == 0 {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(a as f64 / b as f64)
    }
}

/// Distribution summary of a count series (daily counts).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Dist {
    /// Sample size.
    pub n: u64,
    /// Mean.
    pub mean: Option<f64>,
    /// Median (nearest rank).
    pub median: Option<f64>,
    /// 90th percentile (nearest rank).
    pub p90: Option<f64>,
    /// Maximum.
    pub max: Option<f64>,
    /// Share of zeros.
    pub zero_share: Option<f64>,
    /// Coefficient of variation (population std / mean).
    pub cv: Option<f64>,
}

impl Dist {
    /// Summary of `xs`.
    #[must_use]
    pub fn of(xs: &[f64]) -> Self {
        let n = xs.len() as u64;
        let m = mean(xs);
        let zeros = xs.iter().filter(|&&x| x == 0.0).count() as u64;
        Self {
            n,
            mean: m,
            median: percentile(xs, 0.5),
            p90: percentile(xs, 0.9),
            max: xs.iter().copied().reduce(f64::max),
            zero_share: ratio(zeros, n),
            cv: match (m, std_dev(xs)) {
                (Some(m), Some(s)) if m > 0.0 => Some(s / m),
                _ => None,
            },
        }
    }
}

/// Format an optional number for a table (`—` when absent).
#[must_use]
pub fn f(x: Option<f64>, digits: usize) -> String {
    match x {
        Some(v) if v.is_finite() => format!("{v:.digits$}"),
        Some(v) => format!("{v}"),
        None => "—".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Codes
// ---------------------------------------------------------------------------

/// Stage code used in raw records.
#[must_use]
pub fn stage_code(s: Stage) -> u8 {
    match s {
        Stage::Candidate => 0,
        Stage::Born => 1,
        Stage::Strengthened => 2,
        Stage::Weakened => 3,
        Stage::Confirmed => 4,
        Stage::Realized => 5,
        Stage::Invalidated => 6,
        Stage::Expired => 7,
        Stage::Retracted => 8,
    }
}

/// Stage codes 5–7 (ledger closes).
pub const REALIZED: u8 = 5;
/// Invalidated.
pub const INVALIDATED: u8 = 6;
/// Expired.
pub const EXPIRED: u8 = 7;

/// Kind code (index in `MeaningKind::ALL` order).
#[must_use]
pub fn kind_code(k: MeaningKind) -> u8 {
    match k {
        MeaningKind::Continuation => 0,
        MeaningKind::Reversal => 1,
        MeaningKind::Breakout => 2,
        MeaningKind::Squeeze => 3,
        MeaningKind::Extreme => 4,
        MeaningKind::Flow => 5,
    }
}

/// Direction code: up 0, down 1, neutral 2.
#[must_use]
pub fn dir_code(d: Direction) -> u8 {
    match d {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Neutral => 2,
    }
}

/// Direction name.
#[must_use]
pub fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

/// Family bit (price 1, volume 2, flow 4, derivative 8, news 16).
#[must_use]
pub fn family_bit(f: EvidenceFamily) -> u8 {
    match f {
        EvidenceFamily::Price => 1,
        EvidenceFamily::Volume => 2,
        EvidenceFamily::Flow => 4,
        EvidenceFamily::Derivative => 8,
        EvidenceFamily::News => 16,
    }
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// `rounds/R3/experiments/`.
#[must_use]
pub fn experiments_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from(".."), Path::to_path_buf)
}

/// Raw directory of an experiment and alternative for synthetic data:
/// `rounds/R3/experiments/<exp>/raw/<alt>/`.
#[must_use]
pub fn raw_dir(exp: &str, alt: &str) -> PathBuf {
    experiments_dir().join(exp).join("raw").join(alt)
}

/// The local real-bars directory: `$QAPSULE_LOCAL_BARS`, else the first
/// `fixtures/local/bars` found walking up from this crate (works from a git worktree
/// under `.claude/worktrees/`). `None` when absent.
#[must_use]
pub fn real_bars_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(qp_source::LOCAL_BARS_ENV) {
        let p = PathBuf::from(p);
        return p.is_dir().then_some(p);
    }
    let mut d = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    loop {
        let c = d.join("fixtures/local/bars");
        if c.is_dir() {
            return Some(c);
        }
        if !d.pop() {
            return None;
        }
    }
}

/// Where per-capsule raw logs of real data go (never in the repo):
/// `<fixtures/local>/r3-raw/<exp>/<alt>/`.
#[must_use]
pub fn real_raw_dir(exp: &str, alt: &str) -> Option<PathBuf> {
    let bars = real_bars_dir()?;
    let local = bars.parent()?.to_path_buf();
    Some(local.join("r3-raw").join(exp).join(alt))
}

/// Write `text` to `path`, creating parents.
///
/// # Errors
/// I/O errors.
pub fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(path, text)
}

/// Raw file name of a dataset: `<dataset>.json` when the dataset name already starts
/// with the data kind (`synthetic-s1`, `real-1day`), else `<data>-<dataset>.json`
/// (`synthetic-episodes-s1`).
#[must_use]
pub fn raw_file_name(data: DataKind, dataset: &str) -> String {
    if dataset.starts_with(&format!("{}-", data.name())) {
        format!("{dataset}.json")
    } else {
        format!("{}-{dataset}.json", data.name())
    }
}

/// Serialize to compact JSON + newline.
///
/// # Panics
/// Never for the harness types (no maps with non-string keys).
#[must_use]
pub fn to_json<T: Serialize>(v: &T) -> String {
    let mut s = serde_json::to_string(v).expect("serializable");
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// Datasets
// ---------------------------------------------------------------------------

/// Which data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataKind {
    /// Regime synthetic market (30 × 500, seeds).
    Synthetic,
    /// Local real bars (daily and hourly).
    Real,
}

impl DataKind {
    /// Parse `synthetic` / `real`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "synthetic" => Some(Self::Synthetic),
            "real" => Some(Self::Real),
            _ => None,
        }
    }
    /// Name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::Real => "real",
        }
    }
}

/// Synthetic market size.
pub const SYN_INSTRUMENTS: usize = 30;
/// Synthetic market length (trading days).
pub const SYN_DAYS: usize = 500;
/// Default seeds.
pub const SEEDS: [u64; 3] = [1, 2, 3];
/// Bars at the start excluded from daily-count statistics (detector warm-up; the
/// longest common warm-up, `continuation.ma_stack`, is 49 bars).
pub const WARMUP_BARS: usize = 50;

/// Regime names by code.
pub const REGIMES: [&str; 4] = ["rising", "falling", "sideways", "volatile"];

/// One dataset: engine inputs plus the trading-day index of every bar.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// `synthetic-s1`, `real-1day`, …
    pub name: String,
    /// Engine inputs (one per instrument).
    pub inputs: Vec<InstrumentInput>,
    /// Trading-day index of every bar, per instrument.
    pub day_of_bar: Vec<Vec<u32>>,
    /// Regime code (index in [`REGIMES`]) per instrument per trading day (synthetic).
    pub regime: Option<Vec<Vec<u8>>>,
    /// Number of trading days.
    pub n_days: u32,
    /// First trading day counted in daily statistics.
    pub first_day: u32,
}

/// Flow seed of instrument `i` in the synthetic market of `seed`.
#[must_use]
pub fn flow_seed(seed: u64, i: usize) -> u64 {
    mix(seed
        .wrapping_mul(1_000_003)
        .wrapping_add(u64::try_from(i).unwrap_or(0).wrapping_mul(7919))
        .wrapping_add(0xF1))
}

/// Flow records → detector flow points.
#[must_use]
pub fn to_flow_points(recs: Vec<qp_source::FlowRecord>) -> Vec<FlowPoint> {
    recs.into_iter()
        .map(|r| FlowPoint {
            t: r.t,
            values: r.values,
        })
        .collect()
}

/// The regime synthetic market of `seed` (30 × 500) with a synthetic licensed flow per
/// instrument.
#[must_use]
pub fn synthetic(seed: u64) -> Dataset {
    synthetic_sized(seed, SYN_INSTRUMENTS, SYN_DAYS)
}

/// Like [`synthetic`] with another size (tests).
#[must_use]
pub fn synthetic_sized(seed: u64, n_instruments: usize, n_days: usize) -> Dataset {
    let m = qp_source::synthetic_market(n_instruments, n_days, seed);
    let mut inputs = Vec::new();
    let mut day_of_bar = Vec::new();
    let mut regime = Vec::new();
    for (i, inst) in m.into_iter().enumerate() {
        let flow = to_flow_points(qp_source::synthetic_flow(&inst.bars, flow_seed(seed, i)));
        let n = inst.bars.len();
        day_of_bar.push((0..n).map(|d| u32::try_from(d).unwrap_or(0)).collect());
        regime.push(
            (0..n)
                .map(|d| match qp_source::regime_at(&inst.schedule, d) {
                    Some(qp_source::Regime::Rising) => 0,
                    Some(qp_source::Regime::Falling) => 1,
                    Some(qp_source::Regime::Sideways) => 2,
                    Some(qp_source::Regime::Volatile) | None => 3,
                })
                .collect(),
        );
        inputs.push(InstrumentInput {
            series: format!("{}.1day", inst.symbol),
            symbol: inst.symbol,
            bars: inst.bars,
            flow: Some(flow),
        });
    }
    Dataset {
        name: format!("synthetic-s{seed}"),
        inputs,
        day_of_bar,
        regime: Some(regime),
        n_days: u32::try_from(n_days).unwrap_or(0),
        first_day: u32::try_from(WARMUP_BARS.min(n_days)).unwrap_or(0),
    }
}

/// Istanbul local date number (days since epoch) of a bar time (UTC+3, no DST since 2016).
#[must_use]
pub fn local_day(t: i64) -> i64 {
    (t + 3 * 3_600_000).div_euclid(86_400_000)
}

/// Real local bars of `period` (`1day`, `1hour`), adjusted; `None` when absent.
#[must_use]
pub fn real(period: &str) -> Option<Dataset> {
    let dir = real_bars_dir()?;
    let all = qp_source::load_local_bars(&dir, period);
    if all.is_empty() {
        return None;
    }
    let days: BTreeSet<i64> = all
        .iter()
        .flat_map(|(_, b)| b.iter().map(|x| local_day(x.t)))
        .collect();
    let index: BTreeMap<i64, u32> = days
        .iter()
        .enumerate()
        .map(|(i, d)| (*d, u32::try_from(i).unwrap_or(0)))
        .collect();
    let mut inputs = Vec::new();
    let mut day_of_bar = Vec::new();
    let mut first_day = u32::MAX;
    for (sym, bars) in all {
        let dob: Vec<u32> = bars.iter().map(|b| index[&local_day(b.t)]).collect();
        let w = dob.get(WARMUP_BARS).copied().unwrap_or(u32::MAX);
        first_day = first_day.min(w);
        day_of_bar.push(dob);
        inputs.push(InstrumentInput {
            series: format!("{sym}.{period}"),
            symbol: sym,
            bars,
            flow: None,
        });
    }
    Some(Dataset {
        name: format!("real-{period}"),
        inputs,
        day_of_bar,
        regime: None,
        n_days: u32::try_from(days.len()).unwrap_or(0),
        first_day: if first_day == u32::MAX { 0 } else { first_day },
    })
}

/// The datasets of a kind. Real: `1day` and `1hour` when present (empty if none).
#[must_use]
pub fn datasets(kind: DataKind, seeds: &[u64]) -> Vec<Dataset> {
    match kind {
        DataKind::Synthetic => seeds.iter().map(|&s| synthetic(s)).collect(),
        DataKind::Real => ["1day", "1hour"].iter().filter_map(|p| real(p)).collect(),
    }
}

// ---------------------------------------------------------------------------
// Perturbing the future (look-ahead tests)
// ---------------------------------------------------------------------------

/// Replace every bar with `t >= cut_t` (and its flow point) by a different seeded random
/// walk continuing from the last kept close. Bars before `cut_t` are untouched.
#[must_use]
pub fn perturb_future(inputs: &[InstrumentInput], cut_t: i64, seed: u64) -> Vec<InstrumentInput> {
    inputs
        .iter()
        .enumerate()
        .map(|(k, inp)| {
            let mut rng = Lcg::new(seed ^ (u64::try_from(k).unwrap_or(0) << 20) ^ 0xA5A5);
            let mut out = inp.clone();
            let mut prev_c = None;
            let mut prev_v = 1_000_000.0;
            for (i, b) in inp.bars.iter().enumerate() {
                if b.t < cut_t {
                    prev_c = Some(b.c);
                    prev_v = b.v;
                    continue;
                }
                let pc = prev_c.unwrap_or(b.o);
                let o = round2(pc * (1.0 + 0.004 * rng.unit()).max(0.01));
                let c = round2((o * (1.0 + 0.03 * rng.unit())).max(0.01));
                let h = round2(o.max(c) * (1.0 + 0.01 * rng.u01()));
                let l = round2((o.min(c) * (1.0 - 0.01 * rng.u01())).max(0.01));
                let v = round2(prev_v * (0.4 + 1.6 * rng.u01()));
                out.bars[i] = Bar {
                    t: b.t,
                    o,
                    h,
                    l,
                    c,
                    v,
                };
                if let Some(fl) = out.flow.as_mut() {
                    if let Some(p) = fl.get_mut(i) {
                        let net = round2(v * 0.1 * rng.unit());
                        let buy = round2(v * 0.05 + net.max(0.0));
                        p.values = BTreeMap::from([
                            ("net".to_owned(), net),
                            ("buy".to_owned(), buy),
                            ("sell".to_owned(), round2(buy - net)),
                            ("brokers_net_top5".to_owned(), round2(net * 1.8)),
                        ]);
                    }
                }
                prev_c = Some(c);
            }
            out
        })
        .collect()
}

/// Shuffle (seeded permutation) the bars with `t > cut_t` of every instrument, keeping
/// the time stamps in place (records move, `t` stays); flow points move with their bar.
#[must_use]
pub fn shuffle_after(inputs: &[InstrumentInput], cut_t: i64, seed: u64) -> Vec<InstrumentInput> {
    inputs
        .iter()
        .enumerate()
        .map(|(k, inp)| {
            let mut rng = Lcg::new(seed ^ (u64::try_from(k).unwrap_or(0) << 24) ^ 0x5A5A);
            let mut out = inp.clone();
            let idx: Vec<usize> = (0..inp.bars.len())
                .filter(|&i| inp.bars[i].t > cut_t)
                .collect();
            let mut perm = idx.clone();
            for j in (1..perm.len()).rev() {
                let r = rng.below(j + 1);
                perm.swap(j, r);
            }
            for (dst, src) in idx.iter().zip(perm.iter()) {
                let t = inp.bars[*dst].t;
                out.bars[*dst] = Bar {
                    t,
                    ..inp.bars[*src].clone()
                };
                if let (Some(fo), Some(fi)) = (out.flow.as_mut(), inp.flow.as_ref()) {
                    fo[*dst] = FlowPoint {
                        t,
                        values: fi[*src].values.clone(),
                    };
                }
            }
            out
        })
        .collect()
}

/// Bar times at the 25 %, 50 % and 75 % points of the first instrument (look-ahead cuts).
#[must_use]
pub fn cut_times(inputs: &[InstrumentInput]) -> Vec<i64> {
    let Some(b) = inputs.first().map(|i| &i.bars) else {
        return Vec::new();
    };
    if b.len() < 8 {
        return Vec::new();
    }
    [1usize, 2, 3]
        .iter()
        .map(|q| b[b.len() * q / 4].t)
        .collect()
}

/// Run `f` over `items` on up to `available_parallelism` std threads; results in input
/// order (deterministic whatever the scheduling).
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = items.len();
    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZeroUsize::get)
        .min(n.max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: Vec<std::sync::Mutex<Option<R>>> =
        (0..n).map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|sc| {
        for _ in 0..workers {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= n {
                    break;
                }
                let r = f(&items[i]);
                *slots[i].lock().expect("slot lock") = Some(r);
            });
        }
    });
    slots
        .into_iter()
        .map(|m| m.into_inner().expect("slot").expect("filled"))
        .collect()
}
