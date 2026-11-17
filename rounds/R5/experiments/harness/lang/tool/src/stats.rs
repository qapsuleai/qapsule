//! Statistics: seeded RNG (SplitMix64), Wilson interval, Bradley–Terry (MM) with a
//! bootstrap interval. Deterministic for a given seed.

/// SplitMix64.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// New generator from a seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform index below `n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        let n64 = u64::try_from(n).unwrap_or(u64::MAX).max(1);
        usize::try_from(self.next_u64() % n64).unwrap_or(0)
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

/// Stable 64-bit hash of a string (FNV-1a), for blind codes.
#[must_use]
pub fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Blind code of a language name under a seed: six hex digits, never the name.
#[must_use]
pub fn blind_code(name: &str, seed: u64) -> String {
    let mut r = Rng::new(fnv(name) ^ seed.wrapping_mul(0x9E37_79B9));
    format!("{:06x}", r.next_u64() & 0x00FF_FFFF)
}

/// Wilson 95 % interval of `k` successes in `n` trials: `(low, high)`; `(0, 1)` for `n = 0`.
#[must_use]
pub fn wilson(k: f64, n: f64) -> (f64, f64) {
    if n <= 0.0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let p = k / n;
    let d = 1.0 + z * z / n;
    let c = (p + z * z / (2.0 * n)) / d;
    let h = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / d;
    ((c - h).max(0.0), (c + h).min(1.0))
}

/// One pairwise result: `a` vs `b`, score of `a` (1 win, 0,5 tie, 0 loss).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Duel {
    /// Item index.
    pub a: usize,
    /// Item index.
    pub b: usize,
    /// Score of `a`.
    pub sa: f64,
}

/// Bradley–Terry strengths (log scale, mean 0) by the MM algorithm, with a weak prior of
/// one tie between every pair (keeps strengths finite).
#[must_use]
pub fn bradley_terry(n_items: usize, duels: &[Duel]) -> Vec<f64> {
    let n = n_items;
    let mut w = vec![vec![0.0f64; n]; n];
    for d in duels {
        w[d.a][d.b] += d.sa;
        w[d.b][d.a] += 1.0 - d.sa;
    }
    for i in 0..n {
        for j in 0..n {
            if i != j {
                w[i][j] += 0.5;
            }
        }
    }
    let mut p = vec![1.0f64; n];
    for _ in 0..500 {
        let mut np = p.clone();
        for i in 0..n {
            let wins: f64 = (0..n).map(|j| w[i][j]).sum();
            let den: f64 = (0..n)
                .filter(|&j| j != i)
                .map(|j| (w[i][j] + w[j][i]) / (p[i] + p[j]))
                .sum();
            if den > 0.0 {
                np[i] = wins / den;
            }
        }
        let g = (np.iter().map(|x| x.ln()).sum::<f64>() / n.max(1) as f64).exp();
        for x in &mut np {
            *x /= g;
        }
        p = np;
    }
    p.iter().map(|x| x.ln()).collect()
}

/// Bootstrap 95 % intervals of the BT log-strengths: resample the duel *units* (each
/// unit a group of duels, e.g. one situation × repetition) with replacement `b` times.
#[must_use]
pub fn bt_bootstrap(n_items: usize, units: &[Vec<Duel>], b: usize, seed: u64) -> Vec<(f64, f64)> {
    let mut rng = Rng::new(seed);
    let mut samples: Vec<Vec<f64>> = vec![Vec::with_capacity(b); n_items];
    for _ in 0..b {
        let mut ds = Vec::new();
        for _ in 0..units.len() {
            ds.extend_from_slice(&units[rng.below(units.len())]);
        }
        let s = bradley_terry(n_items, &ds);
        for (i, v) in s.into_iter().enumerate() {
            samples[i].push(v);
        }
    }
    samples
        .into_iter()
        .map(|mut v| {
            v.sort_by(f64::total_cmp);
            if v.is_empty() {
                return (0.0, 0.0);
            }
            let q = |p: f64| v[((p * (v.len() - 1) as f64).round() as usize).min(v.len() - 1)];
            (q(0.025), q(0.975))
        })
        .collect()
}

/// Pearson correlation (0 when undefined).
#[must_use]
pub fn corr(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }
    let mx = x[..n].iter().sum::<f64>() / n as f64;
    let my = y[..n].iter().sum::<f64>() / n as f64;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        sxy += (x[i] - mx) * (y[i] - my);
        sxx += (x[i] - mx).powi(2);
        syy += (y[i] - my).powi(2);
    }
    if sxx <= 0.0 || syy <= 0.0 {
        0.0
    } else {
        sxy / (sxx * syy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_known() {
        let (l, h) = wilson(7.0, 10.0);
        assert!(
            (l - 0.3968).abs() < 1e-3 && (h - 0.8922).abs() < 1e-3,
            "{l} {h}"
        );
    }

    #[test]
    fn bt_orders_by_wins() {
        let mut d = Vec::new();
        for _ in 0..10 {
            d.push(Duel {
                a: 0,
                b: 1,
                sa: 1.0,
            });
            d.push(Duel {
                a: 1,
                b: 2,
                sa: 1.0,
            });
            d.push(Duel {
                a: 0,
                b: 2,
                sa: 1.0,
            });
        }
        let s = bradley_terry(3, &d);
        assert!(s[0] > s[1] && s[1] > s[2]);
        let units: Vec<Vec<Duel>> = d.chunks(3).map(<[Duel]>::to_vec).collect();
        let a = bt_bootstrap(3, &units, 50, 7);
        assert_eq!(a, bt_bootstrap(3, &units, 50, 7));
        assert!(a[0].0 > a[2].1);
    }

    #[test]
    fn codes_are_stable_and_blind() {
        assert_eq!(blind_code("r4", 1), blind_code("r4", 1));
        assert_ne!(blind_code("r4", 1), blind_code("bozuk", 1));
        assert!(!blind_code("r4", 1).contains("r4"));
    }
}
