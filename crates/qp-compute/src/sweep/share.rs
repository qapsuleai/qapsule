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

use super::{sweep_kernel, SweepColumns, MAX_LANES, NAN};
use crate::studies::{Bollinger, Ema, Macd};
use crate::{period_param, ComputeError, Core};

type Params = BTreeMap<String, f64>;

/// Shortest run of equal keys that takes the shared path.
const MIN_RUN: usize = 2;

/// Signal EMAs stepped together over one shared macd line.
const MACD_SIGNAL_LANES: usize = 2;

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
    let keys: Vec<(u64, u64)> = full
        .iter()
        .map(|p| (p["fast"].to_bits(), p["slow"].to_bits()))
        .collect();
    let mut line = Vec::new();
    for seg in segments(&keys) {
        match seg {
            Seg::Plain(a, b) => sweep_kernel(macd_build, &full[a..b], x, lanes, bufs, a, visit)?,
            Seg::Shared(a, b) => macd_run(&full[a..b], x, lanes, bufs, &mut line, a, visit)?,
        }
    }
    Ok(())
}

/// One run of variants with equal `(fast, slow)` (input positions `base..`).
fn macd_run(
    run: &[Params],
    x: &[f64],
    lanes: Option<usize>,
    bufs: &mut [SweepColumns],
    line: &mut Vec<f64>,
    base: usize,
    visit: &mut impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    let ms: Vec<Macd> = run.iter().map(macd_build).collect::<Result<_, _>>()?;
    let fast = period_param(&run[0], "fast")?;
    let slow = period_param(&run[0], "slow")?;
    let n = x.len();
    // The macd line: the operations of `Macd::peek1`/`commit1` on the fast/slow EMAs.
    line.resize(n, 0.0);
    macd_line(Ema::new(fast)?, Ema::new(slow)?, x, line);
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
                sweep_kernel(bollinger_build, &full[a..b], x, lanes, bufs, a, visit)?;
            }
            Seg::Shared(a, b) => bollinger_run(&full[a..b], x, &mut bufs[0], &mut std, a, visit)?,
        }
    }
    Ok(())
}

/// One run of variants with equal `period` (input positions `base..`).
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

#[cfg(test)]
mod tests {
    use super::{segments, Seg};

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
