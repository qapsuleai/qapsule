//! The study registry: names, default parameters, period checks, [`create`] and
//! [`compute_bars`] (out of `lib.rs`).

use std::collections::BTreeMap;

use qp_schema::{Bar, StudyOutput};

use crate::{run, Bollinger, ComputeError, Ema, Field, Macd, Rsi, Sma, Streaming, Study};

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
        #[allow(clippy::unreachable)] // invariant: default_params rejected unknown names
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
