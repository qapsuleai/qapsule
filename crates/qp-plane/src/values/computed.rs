//! [`Computed`]: the values of one series, aligned with its bars.

use std::collections::BTreeMap;

/// Values of one series, aligned with its bars.
#[derive(Debug, Clone, PartialEq)]
pub struct Computed {
    /// Bar times.
    pub t: Vec<i64>,
    /// Output name -> one value per bar.
    pub outputs: BTreeMap<String, Vec<Option<f64>>>,
    /// The output used in conditions.
    pub main: String,
    /// Position of `main` among the keys of `outputs` (they never change once computed:
    /// the live path reads the main output without a string lookup).
    pub(super) main_ix: Option<usize>,
    /// (RL1, PS §1.2) An event series' closed length: the evaluable bars (its live bar
    /// holds no value). `None` for every other series.
    pub(super) closed: Option<usize>,
}

impl Computed {
    pub fn new(t: Vec<i64>, outputs: BTreeMap<String, Vec<Option<f64>>>, main: String) -> Self {
        let main_ix = outputs.keys().position(|k| *k == main);
        Self {
            t,
            outputs,
            main,
            main_ix,
            closed: None,
        }
    }

    /// Evaluable bars: the closed length of an event series (PS §1.2), else the bar
    /// count. Conditions and evidence read the last value at `len() - 1`; the vectors
    /// themselves always hold one value per bar (`t.len()`, K13).
    #[inline]
    pub fn len(&self) -> usize {
        self.closed.unwrap_or(self.t.len())
    }

    /// (RF1 S2) Whether there are no bars (`Computed` is `pub` across the crate boundary;
    /// clippy's `len_without_is_empty`).
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn main_values(&self) -> &[Option<f64>] {
        // The main output first in key order (bar columns: `c`; one-output studies): no
        // key comparison; else a lookup by name (faster than walking the map in WASM).
        let v = match self.main_ix {
            Some(0) => self.outputs.values().next(),
            Some(_) => self.outputs.get(&self.main),
            None => None,
        };
        v.map_or(&[], Vec::as_slice)
    }
}
