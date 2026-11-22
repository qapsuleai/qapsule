//! Shared sub-computations inside a sweep (RP1/E1).
//!
//! Variants that share a parameter prefix share part of their computation:
//!
//! - `macd`: variants with the same `(fast, slow)` share the macd line (both EMAs and
//!   their difference); only the signal EMA and the histogram run per variant.
//! - `bollinger`: variants with the same `period` share `middle` and the standard
//!   deviation; only `band = k * std` and the three columns derived from it run per
//!   variant, as a dependency-free column pass.
//!
//! The shared part runs exactly the operations a single run performs, in the same order
//! (it *is* the single-run code, split where the parameter enters), so every output stays
//! bit-identical to [`crate::run`].
//!
//! Grouping is by **runs of consecutive** parameter sets with an equal shared key, so any
//! list in any order works and `visit` still sees the variants in input order. Stretches
//! of variants that share nothing with their neighbours go through the regular lane
//! kernels. A list sorted by the shared key gets the full benefit; an unsorted one runs
//! at the speed of the plain kernels.

use std::collections::BTreeMap;

use super::{soa_plain, Soa, SweepColumns, MAX_LANES, NAN};
use crate::studies::{rsi_from_avg_pass, rsi_split, Bollinger, Ema, Macd, Rsi};
use crate::{period_param, ComputeError, Core};

type Params = BTreeMap<String, f64>;

/// Shortest run of equal keys that takes the shared path.
const MIN_RUN: usize = 2;

/// Signal EMAs stepped together over one shared macd line.
const MACD_SIGNAL_LANES: usize = 2;

/// RSIs stepped together over the shared split (measured: 8 beats 4 on this path).
const RSI_SPLIT_LANES: usize = 8;

/// Fewer RSI variants than this run the lane kernels (no shared split to amortize;
/// RP1 integration measurement).
pub(super) const RSI_MIN_VARIANTS: usize = 4;

/// A stretch of input positions `start..end`.
#[derive(Debug, PartialEq)]
enum Seg {
    /// Variants that share nothing with their neighbours.
    Plain(usize, usize),
    /// A run of variants with equal shared keys.
    Shared(usize, usize),
}

/// Splits `0..keys.len()` into maximal runs of equal keys (runs of at least `MIN_RUN`
/// become `Shared`) and the plain stretches between them, in order.
fn segments<K: PartialEq>(keys: &[K]) -> Vec<Seg> {
    let mut out = Vec::new();
    let (mut i, mut plain_start) = (0, 0);
    while i < keys.len() {
        let mut j = i + 1;
        while j < keys.len() && keys[j] == keys[i] {
            j += 1;
        }
        if j - i >= MIN_RUN {
            if plain_start < i {
                out.push(Seg::Plain(plain_start, i));
            }
            out.push(Seg::Shared(i, j));
            plain_start = j;
        }
        i = j;
    }
    if plain_start < keys.len() {
        out.push(Seg::Plain(plain_start, keys.len()));
    }
    out
}

/// Gives `b` `count` columns of length `n` (contents unspecified).
fn shape(b: &mut SweepColumns, names: &'static [&'static str], n: usize) {
    b.names = names;
    b.columns.resize_with(names.len(), Vec::new);
    for c in &mut b.columns {
        c.resize(n, 0.0);
    }
}

fn macd_build(p: &Params) -> Result<Macd, ComputeError> {
    Macd::new(
        period_param(p, "fast")?,
        period_param(p, "slow")?,
        period_param(p, "signal")?,
    )
}

/// MACD sweep with the macd line shared by consecutive variants of equal `(fast, slow)`.
pub(super) fn macd(
    full: &[Params],
    x: &[f64],
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    macd_cached(full, x, lanes, bufs, EMA_CACHE_BYTES, visit)
}

/// [`macd`] with an explicit EMA cache budget (tests use 0 to cover the uncached path).
fn macd_cached(
    full: &[Params],
    x: &[f64],
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    cache_bytes: usize,
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let keys: Vec<(u64, u64)> = full
        .iter()
        .map(|p| (p["fast"].to_bits(), p["slow"].to_bits()))
        .collect();
    let segs = segments(&keys);
    let emas = ema_cache(&segs, full, x, cache_bytes)?;
    let mut line = Vec::new();
    for seg in segs {
        match seg {
            Seg::Plain(a, b) => soa_plain(Soa::Macd, &full[a..b], x, lanes, bufs, a, visit)?,
            Seg::Shared(a, b) => {
                let fast = period_param(&full[a], "fast")?;
                let slow = period_param(&full[a], "slow")?;
                line.resize(x.len(), 0.0);
                fill_line(fast, slow, x, &emas, &mut line)?;
                macd_run(&full[a..b], &line, slow, lanes, bufs, a, visit)?;
            }
        }
    }
    Ok(())
}

/// The macd line `ema(fast) - ema(slow)` of `x` into `line` (`NaN` before bar
/// `slow - 1`), from the cached EMA columns when present.
fn fill_line(
    fast: usize,
    slow: usize,
    x: &[f64],
    emas: &BTreeMap<usize, Vec<f64>>,
    line: &mut [f64],
) -> Result<(), ComputeError> {
    let w = (slow - 1).min(x.len());
    if let (Some(f), Some(s)) = (emas.get(&fast), emas.get(&slow)) {
        line[..w].fill(NAN);
        for ((m, &f), &s) in line[w..].iter_mut().zip(&f[w..]).zip(&s[w..]) {
            *m = f - s;
        }
    } else {
        macd_line(Ema::new(fast)?, Ema::new(slow)?, x, line);
    }
    Ok(())
}

/// Upper bound on the memory of cached EMA columns (bytes).
const EMA_CACHE_BYTES: usize = 16 << 20;

/// EMA lanes when filling the cache.
const EMA_CACHE_LANES: usize = 4;

/// The EMA column of every fast and slow period used by a shared run, each period
/// computed once (several macd lines share their fast or slow EMA). Empty when they
/// would not fit in [`EMA_CACHE_BYTES`]; the runs then compute their EMAs themselves.
fn ema_cache(
    segs: &[Seg],
    full: &[Params],
    x: &[f64],
    cache_bytes: usize,
) -> Result<BTreeMap<usize, Vec<f64>>, ComputeError> {
    let mut periods = std::collections::BTreeSet::new();
    for seg in segs {
        if let Seg::Shared(a, _) = *seg {
            periods.insert(period_param(&full[a], "fast")?);
            periods.insert(period_param(&full[a], "slow")?);
        }
    }
    let mut cache = BTreeMap::new();
    if periods.len().saturating_mul(x.len()).saturating_mul(8) > cache_bytes {
        return Ok(cache);
    }
    let periods: Vec<usize> = periods.into_iter().collect();
    for chunk in periods.chunks(EMA_CACHE_LANES) {
        let mut emas: Vec<Ema> = chunk
            .iter()
            .map(|&p| Ema::new(p))
            .collect::<Result<_, _>>()?;
        let mut cols: Vec<Vec<f64>> = chunk.iter().map(|_| vec![0.0; x.len()]).collect();
        match chunk.len() {
            4 => ema_lanes::<4>(&mut emas, x, &mut cols),
            2 => ema_lanes::<2>(&mut emas, x, &mut cols),
            _ => {
                for (e, c) in emas.iter_mut().zip(cols.iter_mut()) {
                    ema_lanes::<1>(std::slice::from_mut(e), x, std::slice::from_mut(c));
                }
            }
        }
        cache.extend(chunk.iter().copied().zip(cols));
    }
    Ok(cache)
}

#[allow(clippy::expect_used)]
/// Steps `L` EMAs over `x` into `cols` (`NaN` in warmup): `peek1`/`commit1` until every
/// lane is past its seed bar, then the steady expression with the states in registers.
fn ema_lanes<const L: usize>(emas: &mut [Ema], x: &[f64], cols: &mut [Vec<f64>]) {
    let emas: &mut [Ema; L] = emas.try_into().expect("L EMAs");
    let n = x.len();
    let cols: &mut [Vec<f64>; L] = cols.try_into().expect("L columns");
    let seeded = emas.iter().map(Ema::period).max().unwrap_or(0).min(n);
    for i in 0..seeded {
        for l in 0..L {
            cols[l][i] = emas[l].peek1(x[i]).unwrap_or(NAN);
            emas[l].commit1(x[i]);
        }
    }
    if seeded == n {
        return;
    }
    let mut a = [0.0; L];
    let mut prev = [0.0; L];
    for l in 0..L {
        (a[l], prev[l]) = emas[l].steady().expect("past the seed bar");
    }
    for i in seeded..n {
        let xi = x[i];
        for l in 0..L {
            prev[l] = Ema::steady_next(a[l], prev[l], xi);
            cols[l][i] = prev[l];
        }
    }
}

/// One run of variants with equal `(fast, slow)` (input positions `base..`) over their
/// shared macd `line`.
fn macd_run(
    run: &[Params],
    line: &[f64],
    slow: usize,
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    base: usize,
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let ms: Vec<Macd> = run.iter().map(macd_build).collect::<Result<_, _>>()?;
    let n = line.len();
    // First bar with a macd value.
    let w = (slow - 1).min(n);
    let lanes = lanes.unwrap_or(MACD_SIGNAL_LANES).clamp(1, MAX_LANES);
    for b in &mut bufs[..lanes.min(run.len())] {
        shape(b, Macd::OUTPUTS, n);
        b.columns[0].copy_from_slice(line);
        b.columns[1][..w].fill(NAN);
        b.columns[2][..w].fill(NAN);
    }
    for (ci, chunk) in ms.chunks(lanes).enumerate() {
        let mut sigs: Vec<Ema> = run[ci * lanes..ci * lanes + chunk.len()]
            .iter()
            .map(|p| Ema::new(period_param(p, "signal")?))
            .collect::<Result<_, _>>()?;
        for (b, m) in bufs.iter_mut().zip(chunk) {
            b.warmup = m.warmup();
        }
        let bufs = &mut bufs[..chunk.len()];
        match chunk.len() {
            8 => signal_lanes::<8>(&mut sigs, line, w, bufs),
            4 => signal_lanes::<4>(&mut sigs, line, w, bufs),
            2 => signal_lanes::<2>(&mut sigs, line, w, bufs),
            _ => {
                for (s, b) in sigs.iter_mut().zip(bufs.iter_mut()) {
                    signal_lanes::<1>(std::slice::from_mut(s), line, w, std::slice::from_mut(b));
                }
            }
        }
        for (l, b) in bufs.iter().enumerate() {
            visit(base + ci * lanes + l, b);
        }
    }
    Ok(())
}

#[allow(clippy::expect_used)]
/// The macd line of `x` into `line`: the operations of `Macd::peek1`/`commit1` on the
/// fast and slow EMAs; once both are past their seed bar, the steady expressions run with
/// the states in registers.
fn macd_line(mut ef: Ema, mut es: Ema, x: &[f64], line: &mut [f64]) {
    let n = x.len();
    let seeded = es.period().max(ef.period()).min(n);
    for (m, &xi) in line[..seeded].iter_mut().zip(x) {
        *m = match (ef.peek1(xi), es.peek1(xi)) {
            (Some(f), Some(s)) => f - s,
            _ => NAN,
        };
        ef.commit1(xi);
        es.commit1(xi);
    }
    if seeded == n {
        return;
    }
    let (af, mut f) = ef.steady().expect("past the seed bar");
    let (as_, mut s) = es.steady().expect("past the seed bar");
    for (m, &xi) in line[seeded..n].iter_mut().zip(&x[seeded..]) {
        f = Ema::steady_next(af, f, xi);
        s = Ema::steady_next(as_, s, xi);
        *m = f - s;
    }
}

#[allow(clippy::expect_used)]
/// Steps `L` signal EMAs over `m[w..]` (the macd values), writing signal and histogram
/// into columns 1 and 2 of `bufs[l]` exactly as `Macd::peek1`/`commit1` do. Until every
/// lane is past its seed bar the regular `peek1`/`commit1` run; after that the steady
/// expression runs with the lane states in registers.
fn signal_lanes<const L: usize>(sigs: &mut [Ema], m: &[f64], w: usize, bufs: &mut [SweepColumns]) {
    let sigs: &mut [Ema; L] = sigs.try_into().expect("L signal EMAs");
    let n = m.len();
    let mut sc: Vec<&mut [f64]> = Vec::with_capacity(L);
    let mut hc: Vec<&mut [f64]> = Vec::with_capacity(L);
    for b in &mut bufs[..L] {
        let [_, s, h] = &mut b.columns[..] else {
            unreachable!("three macd outputs")
        };
        sc.push(&mut s[..n]);
        hc.push(&mut h[..n]);
    }
    let sc: [&mut [f64]; L] = sc.try_into().expect("L lanes");
    let hc: [&mut [f64]; L] = hc.try_into().expect("L lanes");
    // Bars up to and including the latest seed bar.
    let seeded = sigs
        .iter()
        .map(Ema::period)
        .max()
        .map_or(w, |p| w + p)
        .min(n);
    for i in w..seeded {
        let mi = m[i];
        for l in 0..L {
            let v = sigs[l].peek1(mi);
            sigs[l].commit1(mi);
            (sc[l][i], hc[l][i]) = match v {
                Some(s) => (s, mi - s),
                None => (NAN, NAN),
            };
        }
    }
    if seeded == n {
        return;
    }
    let mut a = [0.0; L];
    let mut prev = [0.0; L];
    for l in 0..L {
        (a[l], prev[l]) = sigs[l].steady().expect("past the seed bar");
    }
    for i in seeded..n {
        let mi = m[i];
        for l in 0..L {
            let s = Ema::steady_next(a[l], prev[l], mi);
            prev[l] = s;
            sc[l][i] = s;
            hc[l][i] = mi - s;
        }
    }
}

fn bollinger_build(p: &Params) -> Result<Bollinger, ComputeError> {
    Bollinger::new(period_param(p, "period")?, p["k"])
}

/// Bollinger sweep with `middle` and `std` shared by consecutive variants of equal
/// `period`.
pub(super) fn bollinger(
    full: &[Params],
    x: &[f64],
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let keys: Vec<u64> = full.iter().map(|p| p["period"].to_bits()).collect();
    let mut std = Vec::new();
    for seg in segments(&keys) {
        match seg {
            Seg::Plain(a, b) => {
                soa_plain(Soa::Bollinger, &full[a..b], x, lanes, bufs, a, visit)?;
            }
            Seg::Shared(a, b) => bollinger_run(&full[a..b], x, &mut bufs[0], &mut std, a, visit)?,
        }
    }
    Ok(())
}

/// One run of variants with equal `period` (input positions `base..`). Kept out of line:
/// inlined into a large caller it compiled 3–4× slower on wasm32 (RP1 integration).
#[inline(never)]
fn bollinger_run(
    run: &[Params],
    x: &[f64],
    b: &mut SweepColumns,
    std: &mut Vec<f64>,
    base: usize,
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let bs: Vec<Bollinger> = run.iter().map(bollinger_build).collect::<Result<_, _>>()?;
    let n = x.len();
    shape(b, Bollinger::OUTPUTS, n);
    std.resize(n, 0.0);
    // The k-independent state, stepped once: middle (column 0) and std.
    let mut st = bs[0].clone();
    for ((m, s), &xi) in b.columns[0].iter_mut().zip(std.iter_mut()).zip(x) {
        (*m, *s) = st.step_std(xi).unwrap_or((NAN, NAN));
    }
    let w = bs[0].warmup()[0].min(n);
    for c in &mut b.columns[1..] {
        c[..w].fill(NAN);
    }
    for (j, (bv, p)) in bs.iter().zip(run).enumerate() {
        let k = p["k"];
        b.warmup = bv.warmup();
        let [mid, up, lo, bw] = &mut b.columns[..] else {
            unreachable!("four bollinger outputs")
        };
        for ((((&m, &s), u), l), q) in mid[w..]
            .iter()
            .zip(&std[w..])
            .zip(&mut up[w..])
            .zip(&mut lo[w..])
            .zip(&mut bw[w..])
        {
            let v = Bollinger::bands(k, m, s);
            *u = v.upper;
            *l = v.lower;
            *q = v.bandwidth;
        }
        visit(base + j, b);
    }
    Ok(())
}

/// RSI sweep with the per-bar change split `(gain, loss)` computed once for all periods.
pub(super) fn rsi(
    full: &[Params],
    x: &[f64],
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let n = x.len();
    let mut gain = vec![0.0; n];
    let mut loss = vec![0.0; n];
    for i in 1..n {
        (gain[i], loss[i]) = rsi_split(x[i] - x[i - 1]);
    }
    let lanes = lanes.unwrap_or(RSI_SPLIT_LANES).clamp(1, MAX_LANES);
    for (ci, chunk) in full.chunks(lanes).enumerate() {
        let mut rs: Vec<Rsi> = chunk
            .iter()
            .map(|p| Rsi::new(period_param(p, "period")?))
            .collect::<Result<_, _>>()?;
        for (r, b) in rs.iter().zip(bufs.iter_mut()) {
            shape(b, Rsi::OUTPUTS, n);
            b.warmup = r.warmup();
        }
        let bufs = &mut bufs[..chunk.len()];
        match chunk.len() {
            8 => rsi_lanes::<8>(&mut rs, &gain, &loss, bufs),
            4 => rsi_lanes::<4>(&mut rs, &gain, &loss, bufs),
            2 => rsi_lanes::<2>(&mut rs, &gain, &loss, bufs),
            _ => {
                for (r, b) in rs.iter_mut().zip(bufs.iter_mut()) {
                    rsi_lanes::<1>(
                        std::slice::from_mut(r),
                        &gain,
                        &loss,
                        std::slice::from_mut(b),
                    );
                }
            }
        }
        for (l, b) in bufs.iter().enumerate() {
            visit(ci * lanes + l, b);
        }
    }
    Ok(())
}

#[allow(clippy::expect_used)]
/// Steps `L` RSIs over the precomputed split into column 0 of `bufs[l]`.
fn rsi_lanes<const L: usize>(
    rs: &mut [Rsi],
    gain: &[f64],
    loss: &[f64],
    bufs: &mut [SweepColumns],
) {
    let rs: &mut [Rsi; L] = rs.try_into().expect("L RSIs");
    let n = gain.len();
    let cols: Vec<&mut [f64]> = bufs[..L]
        .iter_mut()
        .map(|b| &mut b.columns[0][..n])
        .collect();
    let cols: [&mut [f64]; L] = cols.try_into().expect("L lanes");
    // Through the latest seed bar (bar `period`), then the steady expressions with the
    // lane states in registers.
    let seeded = rs.iter().map(Rsi::period).max().map_or(0, |p| p + 1).min(n);
    for i in 0..seeded {
        for k in 0..L {
            cols[k][i] = rs[k].step_split(gain[i], loss[i]);
        }
    }
    if seeded == n {
        return;
    }
    let st: [[f64; 4]; L] = std::array::from_fn(|k| rs[k].steady().expect("past the seed bar"));
    let (mut ag, mut al) = (st.map(|s| s[2]), st.map(|s| s[3]));
    // Blocks of bars: the dependent smoothing chains run lane-interleaved into small
    // local buffers, then `rsi_from_avg_pass` (two of the four divisions per bar) runs as a
    // dependency-free pass per lane, off the chains, and each lane's output is written
    // as one sequential stream (2 * L interleaved write streams thrash at L = 8).
    const B: usize = 128;
    let mut tg = [[0.0; B]; L];
    let mut tl = [[0.0; B]; L];
    let mut i0 = seeded;
    while i0 < n {
        let len = B.min(n - i0);
        for (j, (&g, &l)) in gain[i0..i0 + len]
            .iter()
            .zip(&loss[i0..i0 + len])
            .enumerate()
        {
            for k in 0..L {
                let [pm1, pf, _, _] = st[k];
                ag[k] = Rsi::smooth(ag[k], pm1, pf, g);
                al[k] = Rsi::smooth(al[k], pm1, pf, l);
                tg[k][j] = ag[k];
                tl[k][j] = al[k];
            }
        }
        for k in 0..L {
            for ((o, &g), &l) in cols[k][i0..i0 + len]
                .iter_mut()
                .zip(&tg[k][..len])
                .zip(&tl[k][..len])
            {
                *o = rsi_from_avg_pass(g, l);
            }
        }
        i0 += len;
    }
}

#[cfg(test)]
mod tests {
    use super::{macd_cached, segments, Params, Seg, SweepColumns, MAX_LANES};
    use crate::sweep::{bench_closes, bench_grid};
    use crate::{create, run, Field};

    #[test]
    fn macd_without_ema_cache_equals_single_runs() {
        let x = bench_closes(300, 5);
        let full: Vec<Params> = bench_grid("macd")[..60]
            .iter()
            .map(|p| create("macd", p, Field::Close).unwrap().params())
            .collect();
        let mut bufs: Vec<SweepColumns> = (0..MAX_LANES)
            .map(|_| SweepColumns {
                names: &[],
                columns: Vec::new(),
                warmup: Vec::new(),
            })
            .collect();
        let mut seen = 0;
        macd_cached(&full, &x, None, &mut bufs, 0, &mut |vi, c| {
            let mut s = create("macd", &full[vi], Field::Close).unwrap();
            let single = run(s.as_mut(), &[&x]).unwrap();
            for (j, col) in single.iter().enumerate() {
                let a: Vec<Option<u64>> = c
                    .to_options(j)
                    .iter()
                    .map(|v| v.map(f64::to_bits))
                    .collect();
                let b: Vec<Option<u64>> = col.iter().map(|v| v.map(f64::to_bits)).collect();
                assert_eq!(a, b, "variant {vi} output {j}");
            }
            assert_eq!(vi, seen);
            seen += 1;
        })
        .unwrap();
        assert_eq!(seen, full.len());
    }

    #[test]
    fn segments_split_runs_and_plain_stretches() {
        assert_eq!(segments::<u8>(&[]), vec![]);
        assert_eq!(segments(&[1]), vec![Seg::Plain(0, 1)]);
        assert_eq!(
            segments(&[1, 2, 2, 3, 4, 4, 4, 5]),
            vec![
                Seg::Plain(0, 1),
                Seg::Shared(1, 3),
                Seg::Plain(3, 4),
                Seg::Shared(4, 7),
                Seg::Plain(7, 8),
            ]
        );
        assert_eq!(segments(&[7, 7]), vec![Seg::Shared(0, 2)]);
    }
}
