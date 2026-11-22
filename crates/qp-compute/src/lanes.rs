//! Structure-of-arrays lane kernels (RP1/E2).
//!
//! `L` independent runs ("lanes") are stepped together. Lane `l` reads its own input
//! slice `xs[l]` and has its own period, so the same kernel serves a parameter sweep
//! (every lane reads the same series) and many series with one parameter set.
//!
//! Each lane's state is a field of `[f64; L]` arrays. The warmup bars of every lane run
//! through the study's own scalar `peek1`/`commit1`; once every lane is in its steady
//! state, a single branch-free loop applies **the same IEEE operations in the same order**
//! as the scalar steady state, lane by lane. The compiler packs the lane loop into vector
//! instructions (NEON, SSE2, AVX, wasm `simd128`); IEEE `+ - * /` round each lane exactly
//! like the scalar instruction, and the code has no `mul_add`, so every output is
//! bit-identical to a single run (tested against `run`, and by the cross-target digest).
//! Branches of the scalar code become selects, which copy bits.

use crate::studies::{Ema, Rsi, Sma};

const NAN: f64 = f64::NAN;

fn trimmed<const L: usize>(xs: &[&[f64]; L], outs: &mut [&mut [f64]]) -> usize {
    let n = xs[0].len();
    assert!(
        xs.iter().all(|x| x.len() == n),
        "lanes must have equal length"
    );
    assert!(outs.len() >= L && outs[..L].iter().all(|o| o.len() >= n));
    n
}

#[allow(clippy::expect_used)]
/// RSI over `L` lanes; writes `outs[l][..n]` (`NaN` in warmup).
pub(crate) fn rsi<const L: usize>(xs: &[&[f64]; L], periods: &[usize; L], outs: &mut [&mut [f64]]) {
    let n = trimmed(xs, outs);
    let maxp = periods.iter().copied().max().unwrap_or(0);
    // Lane with period p is steady from bar p + 1 on (bar p initializes the averages).
    let start = (maxp + 1).min(n);
    let mut ag = [0.0; L];
    let mut al = [0.0; L];
    let mut prev = [0.0; L];
    let mut pf = [0.0; L];
    let mut pm1 = [0.0; L];
    for l in 0..L {
        let mut r = Rsi::new(periods[l]).expect("validated period");
        let (x, o) = (xs[l], &mut *outs[l]);
        for i in 0..start {
            o[i] = r.peek1(x[i]).unwrap_or(NAN);
            r.commit1(x[i]);
        }
        ag[l] = r.avg_gain;
        al[l] = r.avg_loss;
        prev[l] = r.prev_x;
        pf[l] = r.pf;
        pm1[l] = r.pm1;
    }
    let xs: [&[f64]; L] = std::array::from_fn(|l| &xs[l][..n]);
    for i in start..n {
        let mut v = [0.0; L];
        for l in 0..L {
            let x = xs[l][i];
            // `split` and the steady branch of `peek1`/`commit1`, same order.
            let change = x - prev[l];
            let up = change > 0.0;
            let g = if up { change } else { 0.0 };
            let lo = if up { 0.0 } else { change.abs() };
            let a = (ag[l] * pm1[l] + g) / pf[l];
            let b = (al[l] * pm1[l] + lo) / pf[l];
            ag[l] = a;
            al[l] = b;
            prev[l] = x;
            // `rsi_from_avg` with selects: 50 (no move), 100 (no loss), else the formula.
            let r = 100.0 - 100.0 / (1.0 + a / b);
            let z = if a == 0.0 { 50.0 } else { 100.0 };
            v[l] = if b == 0.0 { z } else { r };
        }
        for l in 0..L {
            outs[l][i] = v[l];
        }
    }
}

#[allow(clippy::expect_used)]
/// EMA over `L` lanes.
pub(crate) fn ema<const L: usize>(xs: &[&[f64]; L], periods: &[usize; L], outs: &mut [&mut [f64]]) {
    let n = trimmed(xs, outs);
    let maxp = periods.iter().copied().max().unwrap_or(0);
    // Steady (recursive) from bar p on; bar p - 1 is the seed mean.
    let start = maxp.min(n);
    let mut prev = [0.0; L];
    let mut a = [0.0; L];
    for l in 0..L {
        let mut e = Ema::new(periods[l]).expect("validated period");
        let (x, o) = (xs[l], &mut *outs[l]);
        for i in 0..start {
            o[i] = e.peek1(x[i]).unwrap_or(NAN);
            e.commit1(x[i]);
        }
        prev[l] = e.prev;
        a[l] = e.a;
    }
    let xs: [&[f64]; L] = std::array::from_fn(|l| &xs[l][..n]);
    for i in start..n {
        let mut v = [0.0; L];
        for l in 0..L {
            v[l] = xs[l][i] * a[l] + prev[l] * (1.0 - a[l]);
            prev[l] = v[l];
        }
        for l in 0..L {
            outs[l][i] = v[l];
        }
    }
}

#[allow(clippy::expect_used)]
/// SMA over `L` lanes.
pub(crate) fn sma<const L: usize>(xs: &[&[f64]; L], periods: &[usize; L], outs: &mut [&mut [f64]]) {
    let n = trimmed(xs, outs);
    let maxp = periods.iter().copied().max().unwrap_or(1);
    // The `period - 1` window is full from bar p - 1 on.
    let start = maxp.saturating_sub(1).min(n);
    let mut sum = [0.0; L];
    let mut pf = [0.0; L];
    let mut back = [0usize; L];
    for l in 0..L {
        let mut s = Sma::new(periods[l]).expect("validated period");
        let (x, o) = (xs[l], &mut *outs[l]);
        for i in 0..start {
            o[i] = s.peek1(x[i]).unwrap_or(NAN);
            s.commit1(x[i]);
        }
        sum[l] = s.window_sum();
        pf[l] = s.pf;
        back[l] = periods[l] - 1;
    }
    let xs: [&[f64]; L] = std::array::from_fn(|l| &xs[l][..n]);
    for i in start..n {
        let mut v = [0.0; L];
        for l in 0..L {
            // `step_steady`: value of (sum + x), then evict the input `p - 1` bars back.
            let s = sum[l] + xs[l][i];
            v[l] = s / pf[l];
            sum[l] = s - xs[l][i - back[l]];
        }
        for l in 0..L {
            outs[l][i] = v[l];
        }
    }
}

#[allow(clippy::expect_used)]
/// MACD over `L` lanes, `(fast, slow, signal)` per lane; outputs `outs[l * 3 + j]`
/// (macd, signal, histogram).
pub(crate) fn macd<const L: usize>(
    xs: &[&[f64]; L],
    params: &[(usize, usize, usize); L],
    outs: &mut [&mut [f64]],
) {
    let n = xs[0].len();
    assert!(
        xs.iter().all(|x| x.len() == n),
        "lanes must have equal length"
    );
    assert!(outs.len() >= 3 * L && outs[..3 * L].iter().all(|o| o.len() >= n));
    // Fast, slow and signal EMAs are all recursive from bar slow + signal - 1 on.
    let start = params
        .iter()
        .map(|&(_, s, g)| s + g - 1)
        .max()
        .unwrap_or(0)
        .min(n);
    let (mut f, mut s, mut g) = ([0.0; L], [0.0; L], [0.0; L]);
    let (mut af, mut as_, mut ag) = ([0.0; L], [0.0; L], [0.0; L]);
    for l in 0..L {
        let (pf, ps, pg) = params[l];
        let mut m = crate::studies::Macd::new(pf, ps, pg).expect("validated periods");
        for i in 0..start {
            let x = xs[l][i];
            let v = m.peek1(x);
            outs[l * 3][i] = v.macd.unwrap_or(NAN);
            outs[l * 3 + 1][i] = v.signal.unwrap_or(NAN);
            outs[l * 3 + 2][i] = v.histogram.unwrap_or(NAN);
            m.commit1(x);
        }
        (f[l], s[l], g[l]) = (m.fast.prev, m.slow.prev, m.signal.prev);
        (af[l], as_[l], ag[l]) = (m.fast.a, m.slow.a, m.signal.a);
    }
    let xs: [&[f64]; L] = std::array::from_fn(|l| &xs[l][..n]);
    for i in start..n {
        let (mut vm, mut vs, mut vh) = ([0.0; L], [0.0; L], [0.0; L]);
        for l in 0..L {
            let x = xs[l][i];
            // `Ema::peek1` steady branch for fast and slow, then the signal EMA of macd.
            f[l] = x * af[l] + f[l] * (1.0 - af[l]);
            s[l] = x * as_[l] + s[l] * (1.0 - as_[l]);
            let m = f[l] - s[l];
            g[l] = m * ag[l] + g[l] * (1.0 - ag[l]);
            vm[l] = m;
            vs[l] = g[l];
            vh[l] = m - g[l];
        }
        for l in 0..L {
            outs[l * 3][i] = vm[l];
            outs[l * 3 + 1][i] = vs[l];
            outs[l * 3 + 2][i] = vh[l];
        }
    }
}

#[allow(clippy::expect_used)]
/// Bollinger over `L` lanes, `(period, k)` per lane; outputs `outs[l * 4 + j]` (middle,
/// upper, lower, bandwidth). The flat-window branch of [`crate::Bollinger`] becomes a
/// per-lane select between its two formulas (both computed, one kept).
pub(crate) fn bollinger<const L: usize>(
    xs: &[&[f64]; L],
    params: &[(usize, f64); L],
    outs: &mut [&mut [f64]],
) {
    let n = xs[0].len();
    assert!(
        xs.iter().all(|x| x.len() == n),
        "lanes must have equal length"
    );
    assert!(outs.len() >= 4 * L && outs[..4 * L].iter().all(|o| o.len() >= n));
    let maxp = params.iter().map(|p| p.0).max().unwrap_or(1);
    // Window of `period - 1` inputs full from bar p - 1 on; at least one committed bar.
    let start = maxp.saturating_sub(1).max(1).min(n);
    let mut sum = [0.0; L];
    let mut m2 = [0.0; L];
    let mut last = [0.0; L];
    let mut run = [0.0; L];
    let mut w = [0.0; L];
    let mut pf = [0.0; L];
    let mut inv_p = [0.0; L];
    let mut inv_pm1 = [0.0; L];
    let mut k = [0.0; L];
    let mut back = [0usize; L];
    for l in 0..L {
        let (p, kk) = params[l];
        let mut b = crate::Bollinger::new(p, kk).expect("validated parameters");
        for i in 0..start {
            let v = b.step1(xs[l][i]);
            let o = v.map_or([NAN; 4], |v| [v.middle, v.upper, v.lower, v.bandwidth]);
            for (j, o) in o.into_iter().enumerate() {
                outs[l * 4 + j][i] = o;
            }
        }
        sum[l] = b.sma.window_sum();
        m2[l] = b.m2;
        last[l] = b.last;
        run[l] = b.run as f64;
        w[l] = (p - 1) as f64;
        pf[l] = b.pf;
        inv_p[l] = b.inv_p;
        inv_pm1[l] = b.inv_pm1;
        k[l] = b.k;
        back[l] = p - 1;
    }
    let xs: [&[f64]; L] = std::array::from_fn(|l| &xs[l][..n]);
    for i in start..n {
        let mut v = [[0.0; L]; 4];
        for l in 0..L {
            let x = xs[l][i];
            let s = sum[l];
            let mean = (s + x) * inv_p[l];
            // `add`: flat committed window -> exact M2 from `last`, else Welford.
            let flat = run[l] >= w[l];
            let m2_flat = (x - last[l]) * (x - mean);
            let m2_roll = m2[l] + (x - s * inv_pm1[l]) * (x - mean);
            let m2a = if flat { m2_flat } else { m2_roll };
            // `track_run`.
            let same = run[l] > 0.0 && x == last[l];
            run[l] = if same { run[l] + 1.0 } else { 1.0 };
            last[l] = if same { last[l] } else { x };
            // `Sma::step_steady`.
            let s2 = s + x;
            let middle = s2 / pf[l];
            let old = xs[l][i - back[l]];
            sum[l] = s2 - old;
            // `value`.
            let var = m2a * inv_p[l];
            let sd = var.sqrt();
            let std = if var > 0.0 { sd } else { 0.0 };
            let band = k[l] * std;
            let bw = (2.0 * band) / middle;
            v[0][l] = middle;
            v[1][l] = middle + band;
            v[2][l] = middle - band;
            v[3][l] = if middle == 0.0 { 0.0 } else { bw };
            // Remove `old` from the window.
            let mean_after = sum[l] * inv_pm1[l];
            m2[l] = m2a - (old - mean) * (old - mean_after);
        }
        for l in 0..L {
            for j in 0..4 {
                outs[l * 4 + j][i] = v[j][l];
            }
        }
    }
}
