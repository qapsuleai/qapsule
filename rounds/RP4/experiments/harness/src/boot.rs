//! Symbol-clustered bootstrap (SPEC §3): symbols resampled with replacement, 1,000
//! repetitions, seed 20261121, deterministic. The same weight vectors are used for every
//! statistic and for both rules, so alternative − reference differences are paired.

use serde::{Deserialize, Serialize};

use crate::common::{pct_sorted, Lcg, REPS, SEED};

/// Multiplicity of each symbol in each repetition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boot {
    /// `weights[r][i]`: how many times symbol `i` is drawn in repetition `r`.
    pub weights: Vec<Vec<u32>>,
    /// All ones (the point estimate).
    pub ones: Vec<u32>,
}

impl Boot {
    /// `REPS` repetitions over `n` symbols, seed [`SEED`].
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self::with(n, REPS, SEED)
    }

    /// Custom size and seed (tests).
    #[must_use]
    pub fn with(n: usize, reps: usize, seed: u64) -> Self {
        let mut rng = Lcg::new(seed);
        let weights = (0..reps)
            .map(|_| {
                let mut w = vec![0u32; n];
                for _ in 0..n {
                    w[rng.below(n)] += 1;
                }
                w
            })
            .collect();
        Self {
            weights,
            ones: vec![1; n],
        }
    }
}

/// A point estimate with its 95 % percentile interval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Est {
    /// Point estimate (all symbols once).
    pub pt: Option<f64>,
    /// 2.5 % percentile of the bootstrap distribution.
    pub lo: Option<f64>,
    /// 97.5 % percentile.
    pub hi: Option<f64>,
}

impl Est {
    /// From a point and the bootstrap values (repetitions where the statistic is
    /// undefined are left out).
    #[must_use]
    pub fn from(pt: Option<f64>, reps: &[Option<f64>]) -> Self {
        let mut v: Vec<f64> = reps.iter().filter_map(|x| *x).collect();
        if v.is_empty() {
            return Self {
                pt,
                lo: None,
                hi: None,
            };
        }
        v.sort_by(f64::total_cmp);
        Self {
            pt,
            lo: Some(pct_sorted(&v, 0.025)),
            hi: Some(pct_sorted(&v, 0.975)),
        }
    }
}

/// Estimates of `stat` for A, B and A − B, with the same weights.
pub fn paired<FA, FB>(boot: &Boot, stat_a: FA, stat_b: FB) -> (Est, Est, Est)
where
    FA: Fn(&[u32]) -> Option<f64>,
    FB: Fn(&[u32]) -> Option<f64>,
{
    let pa = stat_a(&boot.ones);
    let pb = stat_b(&boot.ones);
    let mut ra = Vec::with_capacity(boot.weights.len());
    let mut rb = Vec::with_capacity(boot.weights.len());
    let mut rd = Vec::with_capacity(boot.weights.len());
    for w in &boot.weights {
        let a = stat_a(w);
        let b = stat_b(w);
        ra.push(a);
        rb.push(b);
        rd.push(a.zip(b).map(|(a, b)| a - b));
    }
    (
        Est::from(pa, &ra),
        Est::from(pb, &rb),
        Est::from(pa.zip(pb).map(|(a, b)| a - b), &rd),
    )
}

/// Weighted ratio of sums: `Σ w·num / Σ w·den`.
#[must_use]
pub fn ratio_w(w: &[u32], num: &[f64], den: &[f64]) -> Option<f64> {
    let mut a = 0.0;
    let mut b = 0.0;
    for i in 0..w.len() {
        let x = f64::from(w[i]);
        a += x * num[i];
        b += x * den[i];
    }
    (b > 0.0).then(|| a / b)
}
