//! Shared pieces: deterministic RNG, statistics, threads, paths, calendar helpers.
//!
//! Everything here is deterministic (seeded integer LCG; no clock). Percentiles are
//! nearest-rank on the sorted sample (as in R3's harness).

use std::path::{Path, PathBuf};

use qp_calendar::{ExchangeCalendar, LocalTime, Zone};

/// Bootstrap seed (SPEC §3).
pub const SEED: u64 = 20_261_121;
/// Bootstrap repetitions (SPEC §3).
pub const REPS: usize = 1000;

/// 64-bit LCG (Knuth MMIX constants), high 32 bits out; state = `mix(seed)`.
#[derive(Debug, Clone)]
pub struct Lcg(pub u64);

impl Lcg {
    /// A generator whose state is `mix(seed)`.
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
    /// Integer in `[0, n)` (`n > 0`), by multiply-shift (no modulo bias worth noting).
    pub fn below(&mut self, n: usize) -> usize {
        let n64 = u64::try_from(n.max(1)).unwrap_or(1);
        usize::try_from((u64::from(self.next_u32()) * n64) >> 32).unwrap_or(0)
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

/// Nearest-rank percentile of a sorted, non-empty sample (`p` in `[0, 1]`).
#[must_use]
pub fn pct_sorted(v: &[f64], p: f64) -> f64 {
    let n = v.len();
    #[allow(clippy::cast_precision_loss)]
    let rank = (p * n as f64).ceil();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = (rank as usize).clamp(1, n) - 1;
    v[idx]
}

/// Nearest-rank percentile of an unsorted sample; `None` if empty.
#[must_use]
pub fn percentile(xs: &[f64], p: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    Some(pct_sorted(&v, p))
}

/// `a / b`, `None` when `b == 0`.
#[must_use]
pub fn ratio(a: f64, b: f64) -> Option<f64> {
    (b > 0.0).then(|| a / b)
}

/// Run `f` over `items` on up to `available_parallelism` threads; results in input order
/// (deterministic whatever the scheduling).
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

/// The XIST exchange calendar (built in, `qp_calendar::builtin`).
///
/// # Panics
/// Never in practice (the built-in calendar is valid).
#[must_use]
pub fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&qp_calendar::builtin("XIST").expect("XIST built in")).expect("XIST")
}

/// UTC milliseconds of local midnight (Europe/Istanbul) of `date` (`YYYY-MM-DD`).
///
/// # Panics
/// On a malformed date (callers pass constants).
#[must_use]
pub fn istanbul_midnight_ms(date: &str) -> i64 {
    let day = qp_calendar::parse_date(date).expect("date");
    let zone = Zone::get("Europe/Istanbul").expect("zone");
    match zone.local_to_utc(day * 86_400) {
        LocalTime::Unique(t) | LocalTime::Ambiguous(t, _) => t * 1000,
        LocalTime::Gap => (day * 86_400 - 3 * 3600) * 1000,
    }
}

/// `YYYY-MM-DD` of a local day number.
#[must_use]
pub fn day_str(day: i64) -> String {
    let (y, m, d) = qp_calendar::civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Write `text` to `path`, creating parent directories.
///
/// # Errors
/// I/O.
pub fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(path, text)
}

/// Pretty JSON (deterministic: struct field order, `BTreeMap` keys).
///
/// # Panics
/// Never for the harness's types.
#[must_use]
pub fn to_json<T: serde::Serialize>(v: &T) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("json");
    s.push('\n');
    s
}

/// The first ancestor of the current directory that has `fixtures/local` (the main
/// checkout from a worktree too), joined with `fixtures/local/rp4`.
#[must_use]
pub fn default_local_root() -> PathBuf {
    let mut d = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if d.join("fixtures/local").is_dir() {
            return d.join("fixtures/local/rp4");
        }
        if !d.pop() {
            return PathBuf::from("fixtures/local/rp4");
        }
    }
}

/// Format an optional number with `digits` decimals (`—` when absent).
#[must_use]
pub fn f(x: Option<f64>, digits: usize) -> String {
    x.map_or_else(|| "—".to_owned(), |v| format!("{v:.digits$}"))
}

/// (RP4 `cases`) Istanbul is UTC+3 all year since 2016 (`docs/DATA_ACCESS.md`).
const IST_OFFSET_MS: i64 = 3 * 3_600_000;

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Epoch ms of 12:00 Istanbul on `YYYY-MM-DD`.
#[must_use]
pub fn local_noon_ms(date: &str) -> Option<i64> {
    let mut it = date.split('-');
    let (y, m, d): (i64, i64, i64) = (
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    );
    let yy = if m <= 2 { y - 1 } else { y };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Some(days * 86_400_000 + 12 * 3_600_000 - IST_OFFSET_MS)
}

/// Local (Istanbul) `YYYY-MM-DD` of epoch ms.
#[must_use]
pub fn local_date(t: i64) -> String {
    let (y, m, d) = civil_from_days((t + IST_OFFSET_MS).div_euclid(86_400_000));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Local (Istanbul) `HH:MM` of epoch ms.
#[must_use]
pub fn local_hhmm(t: i64) -> String {
    let s = (t + IST_OFFSET_MS).rem_euclid(86_400_000) / 60_000;
    format!("{:02}:{:02}", s / 60, s % 60)
}
