//! The [`Study`] trait and the batch runner [`run`] (out of `lib.rs`).

use std::collections::BTreeMap;

use crate::ComputeError;

/// A streaming, multi-input, multi-output study ("Hesap").
///
/// Inputs are given per bar as one `f64` per declared input, in [`Study::inputs`]
/// order. Outputs are written into `out`, one slot per [`Study::outputs`] name; `None`
/// means "no value yet" (warmup).
pub trait Study {
    /// Study name, e.g. `macd`.
    fn name(&self) -> &'static str;
    /// Parameters, e.g. `{"fast": 12, "slow": 26, "signal": 9}`.
    fn params(&self) -> BTreeMap<String, f64>;
    /// Input names, in the order `push` expects them (e.g. `["close"]`, or another
    /// study's output name when chained).
    fn inputs(&self) -> Vec<String>;
    /// Output names, in the order `push` writes them.
    fn outputs(&self) -> &'static [&'static str];
    /// Warmup length per output: the number of leading `None` values.
    fn warmup(&self) -> Vec<usize>;
    /// Adds a new bar and writes its outputs. O(1).
    fn push(&mut self, input: &[f64], out: &mut [Option<f64>]);
    /// Replaces the inputs of the last pushed bar and writes its revised outputs. O(1).
    /// Without a previous bar it behaves like [`Study::push`].
    fn update_last(&mut self, input: &[f64], out: &mut [Option<f64>]);
    /// Takes the last pushed bar back out, as if it had never been pushed. O(1). Only the
    /// last bar can be discarded: afterwards there is no revisable bar, and the next
    /// [`Study::push`] (or [`Study::update_last`]) adds a bar on top of the bars before
    /// it. Without a revisable bar it does nothing. Used by [`Chain`] when a revision
    /// turns the first study's output into `None`.
    fn discard_last(&mut self);
    /// Forgets all bars.
    fn reset(&mut self);

    /// Adds many bars at once without producing their outputs (e.g. attaching a study to
    /// a live chart's history). `columns[i]` is input `i`; all columns have one value per
    /// bar. Afterwards the study is in exactly the state that `push`ing the same bars would
    /// leave (the last bar is the revisable one). The default pushes bar by bar.
    ///
    /// # Panics
    /// Wrong number of columns or columns of different lengths.
    fn extend(&mut self, columns: &[&[f64]]) {
        assert_eq!(columns.len(), self.inputs().len(), "one column per input");
        let len = columns.first().map_or(0, |c| c.len());
        assert!(
            columns.iter().all(|c| c.len() == len),
            "columns differ in length"
        );
        let mut x = vec![0.0; columns.len()];
        let mut o = vec![None; self.outputs().len()];
        for i in 0..len {
            for (slot, col) in x.iter_mut().zip(columns) {
                *slot = col[i];
            }
            self.push(&x, &mut o);
        }
    }
}

/// Runs `study` (after a reset) over input columns in batch mode.
///
/// `columns[i]` is the i-th input of [`Study::inputs`]; all columns have the same length.
/// Returns one column per output, each exactly as long as the input (K13).
///
/// # Errors
/// Wrong number of columns or columns of different lengths.
pub fn run(
    study: &mut dyn Study,
    columns: &[&[f64]],
) -> Result<Vec<Vec<Option<f64>>>, ComputeError> {
    let n_in = study.inputs().len();
    if columns.len() != n_in {
        return Err(ComputeError::new(format!(
            "{} expects {n_in} input column(s), got {}",
            study.name(),
            columns.len()
        )));
    }
    let len = columns.first().map_or(0, |c| c.len());
    if columns.iter().any(|c| c.len() != len) {
        return Err(ComputeError::new("input columns differ in length"));
    }
    study.reset();
    let n_out = study.outputs().len();
    let mut outs: Vec<Vec<Option<f64>>> = (0..n_out).map(|_| Vec::with_capacity(len)).collect();
    let mut x = vec![0.0; n_in];
    let mut o = vec![None; n_out];
    for i in 0..len {
        for (slot, col) in x.iter_mut().zip(columns) {
            *slot = col[i];
        }
        study.push(&x, &mut o);
        for (col, v) in outs.iter_mut().zip(&o) {
            col.push(*v);
        }
    }
    Ok(outs)
}
