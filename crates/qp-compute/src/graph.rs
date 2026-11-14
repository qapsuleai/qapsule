//! Computation graph for chained studies and a content-addressed result cache (RP1/E4).
//!
//! A chain feeds one output of a study into the next study ([`crate::Chain`] semantics:
//! the next stage gets nothing while the previous one warms up, so its first input is the
//! previous stage's first value). Sweeping many chains repeats work: chains that share a
//! prefix compute the same intermediate column. [`sweep_chains`] computes every distinct
//! prefix once and runs each group's last stage as one parameter sweep (lane kernels) over
//! the prefix's defined suffix. Each stage runs the same operations in the same order as
//! the nested `Chain`, so outputs are bit-identical to it.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::sweep::{sweep_each, SweepColumns};
use crate::{create, ComputeError, Field};

/// One stage of a chain: a study, its parameters, and which of its outputs feeds the next
/// stage (validated on every stage, also the last, where it is not read).
#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    /// Study name (`sma`, `rsi`, ...).
    pub study: String,
    /// Parameters (missing ones take their defaults).
    pub params: BTreeMap<String, f64>,
    /// Output index read by the next stage (must exist).
    pub output: usize,
}

impl Stage {
    /// A stage feeding its first output onward.
    #[must_use]
    pub fn new(study: &str, params: BTreeMap<String, f64>) -> Self {
        Self {
            study: study.to_owned(),
            params,
            output: 0,
        }
    }
}

/// Canonical key of a validated stage (name, effective parameter bits, output).
fn stage_key(s: &Stage) -> Result<String, ComputeError> {
    let study = create(&s.study, &s.params, Field::Close)?;
    if s.output >= study.outputs().len() {
        return Err(ComputeError::new(format!(
            "{}: no output {} (has {})",
            s.study,
            s.output,
            study.outputs().len()
        )));
    }
    let mut k = s.study.clone();
    for (name, v) in study.params() {
        k.push_str(&format!("|{name}={:016x}", v.to_bits()));
    }
    k.push_str(&format!("|>{}", s.output));
    Ok(k)
}

/// The defined part of a column: `values[i]` is bar `start + i`.
struct Suffix {
    start: usize,
    values: Vec<f64>,
}

/// Runs one stage alone over `input` and returns the output `which` after its warmup.
fn run_stage(s: &Stage, input: &Suffix) -> Result<Suffix, ComputeError> {
    let mut out = None;
    sweep_each(
        &s.study,
        std::slice::from_ref(&s.params),
        &input.values,
        |_, c| {
            let w = c.warmup[s.output];
            let col = &c.columns[s.output];
            out = Some(Suffix {
                start: input.start + w,
                values: col[w.min(col.len())..].to_vec(),
            });
        },
    )?;
    Ok(out.expect("one variant"))
}

/// Sweeps chains over `x` and calls `visit(chain_index, outputs)` in chain order. The
/// outputs are those of the last stage, as long as `x`, `NaN` before the chain's first
/// value (`warmup` = the chain's warmup, as [`crate::Chain::warmup`]).
///
/// Every distinct stage prefix is computed once per call; consecutive chains that share
/// all but the last stage run that last stage as one sweep. Memory: one column per
/// distinct prefix.
///
/// # Errors
/// An empty chain, an unknown study, invalid parameters or output index (checked before
/// any work).
pub fn sweep_chains(
    chains: &[Vec<Stage>],
    x: &[f64],
    mut visit: impl FnMut(usize, &SweepColumns),
) -> Result<(), ComputeError> {
    // Validate everything first and build the prefix keys.
    let mut keys: Vec<Vec<String>> = Vec::with_capacity(chains.len());
    for c in chains {
        if c.is_empty() {
            return Err(ComputeError::new("empty chain"));
        }
        keys.push(c.iter().map(stage_key).collect::<Result<_, _>>()?);
    }
    let n = x.len();
    let root = Suffix {
        start: 0,
        values: x.to_vec(),
    };
    // Distinct prefixes (all stages but the last), keyed by their joined stage keys.
    let mut prefixes: BTreeMap<String, Suffix> = BTreeMap::new();
    let mut out = SweepColumns {
        names: &[],
        columns: Vec::new(),
        warmup: Vec::new(),
    };
    let mut c0 = 0;
    while c0 < chains.len() {
        let depth = chains[c0].len();
        let pre = &keys[c0][..depth - 1];
        let mut c1 = c0 + 1;
        while c1 < chains.len() && chains[c1].len() == depth && keys[c1][..depth - 1] == *pre {
            c1 += 1;
        }
        // Make sure every prefix level of this group exists.
        let mut path = String::new();
        for d in 0..depth - 1 {
            let parent = path.clone();
            path.push_str(&keys[c0][d]);
            path.push('\n');
            if !prefixes.contains_key(&path) {
                let input = if d == 0 { &root } else { &prefixes[&parent] };
                let s = run_stage(&chains[c0][d], input)?;
                prefixes.insert(path.clone(), s);
            }
        }
        let input = if depth == 1 { &root } else { &prefixes[&path] };
        let last: Vec<BTreeMap<String, f64>> = chains[c0..c1]
            .iter()
            .map(|c| c[depth - 1].params.clone())
            .collect();
        let study = &chains[c0][depth - 1].study;
        let start = input.start;
        sweep_each(study, &last, &input.values, |j, c| {
            out.names = c.names;
            out.warmup.clear();
            out.warmup.extend(c.warmup.iter().map(|w| start + w));
            out.columns.resize_with(c.columns.len(), Vec::new);
            for (dst, src) in out.columns.iter_mut().zip(&c.columns) {
                dst.clear();
                dst.resize(start.min(n), f64::NAN);
                dst.extend_from_slice(src);
                dst.resize(n, f64::NAN);
            }
            visit(c0 + j, &out);
        })?;
        c0 = c1;
    }
    Ok(())
}

/// 128-bit content hash of an input column (two independent 64-bit word hashes over the
/// value bits and the length). Deterministic on every target.
#[must_use]
pub fn content_hash(x: &[f64]) -> (u64, u64) {
    let mut a: u64 = 0x9e37_79b9_7f4a_7c15 ^ x.len() as u64;
    let mut b: u64 = 0xc2b2_ae3d_27d4_eb4f ^ (x.len() as u64).rotate_left(32);
    for v in x {
        let w = v.to_bits();
        a = (a ^ w).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(29);
        b = (b ^ w.rotate_left(17))
            .wrapping_mul(0xff51_afd7_ed55_8ccd)
            .rotate_left(31);
    }
    (a, b)
}

/// Content-addressed cache of study outputs: key = (input content hash, study, effective
/// parameter bits). A hit returns the shared columns computed earlier for the same key;
/// the value is a pure function of the key, so caching never changes a result.
#[derive(Default)]
pub struct ResultCache {
    map: BTreeMap<((u64, u64), String), Arc<SweepColumns>>,
}

impl ResultCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of cached results.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Outputs of `study(params)` over `x`, computed once per distinct key. `input` is
    /// `content_hash(x)` (pass it to avoid rehashing the same input).
    ///
    /// # Errors
    /// Unknown study or invalid parameters.
    pub fn get_or_compute(
        &mut self,
        input: (u64, u64),
        study: &str,
        params: &BTreeMap<String, f64>,
        x: &[f64],
    ) -> Result<Arc<SweepColumns>, ComputeError> {
        let key = (input, stage_key(&Stage::new(study, params.clone()))?);
        if let Some(v) = self.map.get(&key) {
            return Ok(Arc::clone(v));
        }
        let mut got = None;
        sweep_each(study, std::slice::from_ref(params), x, |_, c| {
            got = Some(Arc::new(c.clone()));
        })?;
        let v = got.expect("one variant");
        self.map.insert(key, Arc::clone(&v));
        Ok(v)
    }
}
