//! Qapsule compute core (`Study` = "Hesap" in `docs/CONCEPTS.md`).
//!
//! Every study is a deterministic function from input columns to named output columns:
//!
//! - **f64 only**, no clock, network, randomness or LLM.
//! - **Same length as the input (K13):** one output value per input bar; warmup values
//!   are `None` (`null` in JSON). Nothing is trimmed.
//! - **Three run modes, bit-identical:** batch ([`run`]), streaming ([`Study::push`]) and
//!   "revise the last bar" ([`Study::update_last`], for a live bar whose price keeps
//!   changing). A new bar and a revision are both O(1).
//! - **Cross-target bit-identical:** only IEEE basic operations (`+ - * /`, `sqrt`) in a
//!   fixed order; no `mul_add`, `powf`, `exp`, `ln`. Rust never contracts `a * b + c` into
//!   an FMA on its own, so aarch64, x86_64 and wasm32 produce the same bits (tested by the
//!   cross-target digest, see `examples/digest.rs`).
//!
//! # How "revise the last bar" works
//!
//! Each study is written as a [`Core`]: a *committed* state that covers every bar before
//! the last one, plus two pure-ish operations. `peek(state, x)` computes the outputs of a
//! bar with input `x` on top of the committed state without changing it; `commit(x)`
//! folds `x` into the committed state. [`Streaming`] keeps the last bar's input pending:
//!
//! - `push(x)`: commit the pending input (if any), make `x` pending, return `peek(x)`.
//! - `update_last(x')`: replace the pending input, return `peek(x')`.
//!
//! The output of a bar therefore only depends on the committed state and the bar's
//! *final* input, whichever way it got there; batch, streaming and revised runs are
//! bit-identical by construction (and tested). Both operations are O(1); ring buffers
//! hold windows.

pub mod address;
mod chain;
mod core;
pub mod graph;
mod lanes;
pub mod late;
pub mod par;
#[doc(hidden)]
pub mod profiles;
pub mod studies;
pub mod sweep;

use std::collections::BTreeMap;
use std::fmt;

pub use address::{Address, AddressError};
pub use chain::Chain;
pub use core::{Core, Streaming};
pub use studies::{Atr, Bollinger, Ema, Highest, Lowest, Macd, Rsi, Sma};

use qp_schema::{Bar, StudyOutput};

/// A bar field a study can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// Open price.
    Open,
    /// High price.
    High,
    /// Low price.
    Low,
    /// Close price.
    Close,
    /// Volume.
    Volume,
}

impl Field {
    /// All fields, in OHLCV order.
    pub const ALL: [Self; 5] = [Self::Open, Self::High, Self::Low, Self::Close, Self::Volume];

    /// The field's value in `bar`.
    #[must_use]
    pub fn of(self, bar: &Bar) -> f64 {
        match self {
            Self::Open => bar.o,
            Self::High => bar.h,
            Self::Low => bar.l,
            Self::Close => bar.c,
            Self::Volume => bar.v,
        }
    }

    /// Lower-case name: `open`, `high`, `low`, `close`, `volume`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
        }
    }

    /// Parses a field name (see [`Field::name`]).
    ///
    /// # Errors
    /// Unknown name.
    pub fn parse(name: &str) -> Result<Self, ComputeError> {
        Self::ALL
            .into_iter()
            .find(|f| f.name() == name)
            .ok_or_else(|| ComputeError::new(format!("unknown field `{name}`")))
    }

    /// The column of this field over `bars`.
    #[must_use]
    pub fn column(self, bars: &[Bar]) -> Vec<f64> {
        bars.iter().map(|b| self.of(b)).collect()
    }
}

/// Any study construction or input error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeError(String);

impl ComputeError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ComputeError {}

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

/// Default parameters per study name.
#[must_use]
pub fn default_params(name: &str) -> Option<BTreeMap<String, f64>> {
    let p = |kv: &[(&str, f64)]| kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect();
    Some(match name {
        "sma" | "ema" => p(&[("period", 20.0)]),
        "rsi" => p(&[("period", 14.0)]),
        "macd" => p(&[("fast", 12.0), ("slow", 26.0), ("signal", 9.0)]),
        "bollinger" => p(&[("period", 20.0), ("k", 2.0)]),
        _ => return None,
    })
}

/// Names of the built-in studies.
pub const STUDY_NAMES: [&str; 5] = ["sma", "ema", "rsi", "macd", "bollinger"];

/// Largest accepted period (every period-like parameter: `period`, `fast`, `slow`,
/// `signal`), in bars.
///
/// A study allocates its window when it is built (`sma` and `bollinger` keep the last
/// `period - 1` inputs), before it sees any bar, so the bound has to be a fixed number
/// rather than the series length. 100 000 bars is the largest series the project plans
/// for (the 100k-bar render target); a longer period could only produce `None` on such a
/// series. The largest window is then 800 KB of `f64`, which a wasm32 module can always
/// allocate (R1 audit finding 6: up to 1e9 was accepted, an 8 GB allocation).
pub const MAX_PERIOD: usize = 100_000;

/// Checks `1 <= period <= MAX_PERIOD` for a study constructor.
pub(crate) fn check_period(study: &str, key: &str, period: usize) -> Result<(), ComputeError> {
    if period == 0 {
        return Err(ComputeError::new(format!("{study}: {key} must be >= 1")));
    }
    if period > MAX_PERIOD {
        return Err(ComputeError::new(format!(
            "{study}: {key} {period} exceeds the maximum period {MAX_PERIOD}"
        )));
    }
    Ok(())
}

/// A parameter value for an error message: plain below 1e6, scientific above (so
/// `1e18` reads as `1e18`, not as nineteen digits).
fn show(v: f64) -> String {
    if v.is_finite() && v.abs() >= 1.0e6 {
        format!("{v:e}")
    } else {
        format!("{v}")
    }
}

/// Reads an integer period parameter (`1 ..= MAX_PERIOD`).
pub(crate) fn period_param(
    params: &BTreeMap<String, f64>,
    key: &str,
) -> Result<usize, ComputeError> {
    let v = *params
        .get(key)
        .ok_or_else(|| ComputeError::new(format!("missing parameter `{key}`")))?;
    if !(v.is_finite() && v >= 1.0 && v.fract() == 0.0) {
        return Err(ComputeError::new(format!(
            "parameter `{key}` must be an integer between 1 and {MAX_PERIOD}, got {}",
            show(v)
        )));
    }
    #[allow(clippy::cast_precision_loss)]
    if v > MAX_PERIOD as f64 {
        return Err(ComputeError::new(format!(
            "parameter `{key}` is {}, which exceeds the maximum period {MAX_PERIOD}",
            show(v)
        )));
    }
    // Checked above: integral, in [1, MAX_PERIOD].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(v as usize)
}

/// Builds a built-in study by name. Missing parameters take their defaults
/// ([`default_params`]); unknown parameters are rejected. `field` is the bar field the
/// study reads.
///
/// # Errors
/// Unknown study, unknown or invalid parameter.
pub fn create(
    name: &str,
    params: &BTreeMap<String, f64>,
    field: Field,
) -> Result<Box<dyn Study>, ComputeError> {
    let mut full = default_params(name).ok_or_else(|| {
        ComputeError::new(format!(
            "unknown study `{name}`; expected one of: {}",
            STUDY_NAMES.join(", ")
        ))
    })?;
    for (k, v) in params {
        if !full.contains_key(k) {
            return Err(ComputeError::new(format!(
                "unknown parameter `{k}` for `{name}`; expected: {}",
                full.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        full.insert(k.clone(), *v);
    }
    Ok(match name {
        "sma" => Box::new(Streaming::new(
            Sma::new(period_param(&full, "period")?)?,
            field,
        )),
        "ema" => Box::new(Streaming::new(
            Ema::new(period_param(&full, "period")?)?,
            field,
        )),
        "rsi" => Box::new(Streaming::new(
            Rsi::new(period_param(&full, "period")?)?,
            field,
        )),
        "macd" => Box::new(Streaming::new(
            Macd::new(
                period_param(&full, "fast")?,
                period_param(&full, "slow")?,
                period_param(&full, "signal")?,
            )?,
            field,
        )),
        "bollinger" => Box::new(Streaming::new(
            Bollinger::new(period_param(&full, "period")?, full["k"])?,
            field,
        )),
        _ => unreachable!("default_params covers every study name"),
    })
}

/// Runs a built-in study over bars and returns the schema result type (K13: `t` and every
/// output have one entry per bar; warmup is `null`).
///
/// # Errors
/// See [`create`].
pub fn compute_bars(
    name: &str,
    params: &BTreeMap<String, f64>,
    field: Field,
    bars: &[Bar],
) -> Result<StudyOutput, ComputeError> {
    let mut study = create(name, params, field)?;
    let column = field.column(bars);
    let outs = run(study.as_mut(), &[&column])?;
    let names = study.outputs();
    let warmup = study.warmup();
    Ok(StudyOutput {
        study: name.to_owned(),
        params: study.params(),
        t: bars.iter().map(|b| b.t).collect(),
        outputs: names
            .iter()
            .zip(outs)
            .map(|(n, col)| ((*n).to_owned(), col))
            .collect(),
        warmup: names
            .iter()
            .zip(warmup)
            .map(|(n, w)| ((*n).to_owned(), u32::try_from(w).unwrap_or(u32::MAX)))
            .collect(),
    })
}
