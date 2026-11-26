//! The limits of the language (PR §8.4; `docs/PARAMETERS.md` "RL1"): every bound the
//! checker, the executor, the budget and the text language use, filled by the caller from
//! the parameter registry (`SessionView::param(key)`); a missing or invalid value takes
//! the default.
//!
//! The single source is the registry: each [`Limit`] names its row in
//! `qp_schema::params::program` and reads the key, default and range from it. No default
//! is restated here.

use std::collections::BTreeMap;

use qp_schema::params::{program as rows, ParamDef, ParamValue};
use qp_schema::program::ProgramParamValue;

/// One RL1 limit key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Limit {
    /// `program.max_nodes` (`E_TOO_MANY_NODES`).
    MaxNodes,
    /// `program.max_window` (`E_WINDOW_LIMIT`).
    MaxWindow,
    /// `program.max_outputs` (`E_TOO_LARGE`).
    MaxOutputs,
    /// `program.max_params` (`E_TOO_LARGE`).
    MaxParams,
    /// `program.max_ir_bytes` (`E_TOO_LARGE`).
    MaxIrBytes,
    /// `program.max_cost_per_bar` (`E_COST_LIMIT`).
    MaxCostPerBar,
    /// `program.signal.max_horizon_bars` (`E_BAD_RULE`).
    SignalMaxHorizonBars,
    /// `program.check.max_errors`.
    CheckMaxErrors,
    /// `program.live.doc.max_instances` (`E_BUDGET`).
    LiveDocMaxInstances,
    /// `program.live.doc.max_cost_per_bar` (`E_BUDGET`).
    LiveDocMaxCostPerBar,
    /// `program.live.doc.max_log_bytes` (`E_BUDGET`).
    LiveDocMaxLogBytes,
    /// `program.try.max_bars`.
    TryMaxBars,
    /// `program.try.max_work`.
    TryMaxWork,
    /// `program.try.max_items`.
    TryMaxItems,
    /// `program.work.record`.
    WorkRecord,
    /// `program.work.open_record`.
    WorkOpenRecord,
    /// `program.work.read`.
    WorkRead,
    /// `program.text.max_source_chars` (text language, `E_TOO_LARGE`).
    TextMaxSourceChars,
    /// `program.text.max_token_chars` (text language, `E_TOO_LARGE`).
    TextMaxTokenChars,
    /// `program.text.max_depth` (text language, `E_TOO_LARGE`).
    TextMaxDepth,
    /// `program.max_ops_per_command` (session, `E_TOO_LARGE`).
    MaxOpsPerCommand,
}

/// Number of limit keys.
const N: usize = 21;

impl Limit {
    /// Every key, in `docs/PARAMETERS.md` order.
    pub const ALL: [Limit; N] = [
        Limit::MaxNodes,
        Limit::MaxWindow,
        Limit::MaxOutputs,
        Limit::MaxParams,
        Limit::MaxIrBytes,
        Limit::MaxCostPerBar,
        Limit::SignalMaxHorizonBars,
        Limit::CheckMaxErrors,
        Limit::LiveDocMaxInstances,
        Limit::LiveDocMaxCostPerBar,
        Limit::LiveDocMaxLogBytes,
        Limit::TryMaxBars,
        Limit::TryMaxWork,
        Limit::TryMaxItems,
        Limit::WorkRecord,
        Limit::WorkOpenRecord,
        Limit::WorkRead,
        Limit::TextMaxSourceChars,
        Limit::TextMaxTokenChars,
        Limit::TextMaxDepth,
        Limit::MaxOpsPerCommand,
    ];

    /// The registry row (`qp_schema::params::program`).
    #[must_use]
    pub const fn def(self) -> &'static ParamDef {
        rows::PROGRAM_ROWS[self as usize]
    }

    /// The registry key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        self.def().key
    }

    /// The registry default (every RL1 row is a non-negative integer).
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub const fn default_value(self) -> u64 {
        match self.def().default {
            ParamValue::Int(v) if v >= 0 => v as u64,
            _ => 0,
        }
    }

    /// The allowed range `[min, max]` of the registry row.
    #[must_use]
    pub fn range(self) -> (f64, f64) {
        let d = self.def();
        (d.min.unwrap_or(0.0), d.max.unwrap_or(f64::INFINITY))
    }

    /// Index into [`Limits`].
    const fn index(self) -> usize {
        self as usize
    }
}

/// The values in force for every [`Limit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    values: [u64; N],
}

impl Default for Limits {
    fn default() -> Self {
        Limits::from_params(|_| None)
    }
}

impl Limits {
    /// Values from `get(key)` (the host's `SessionView::param`); a missing, non-integral
    /// or out-of-range value takes the default (PR §8.4).
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn from_params(get: impl Fn(&str) -> Option<f64>) -> Limits {
        let mut values = [0u64; N];
        for l in Limit::ALL {
            let (lo, hi) = l.range();
            values[l.index()] = match get(l.key()) {
                Some(v) if v.is_finite() && v.fract() == 0.0 && v >= lo && v <= hi => v as u64,
                _ => l.default_value(),
            };
        }
        Limits { values }
    }

    /// The value of `l`.
    #[must_use]
    pub const fn get(&self, l: Limit) -> u64 {
        self.values[l.index()]
    }

    /// The same limits with `l` set to `v` (tests, hosts with a fixed override).
    #[must_use]
    pub const fn with(mut self, l: Limit, v: u64) -> Limits {
        self.values[l.index()] = v;
        self
    }

    /// Key → value, as `check_program` answers list them (`limits`).
    #[must_use]
    pub fn to_map(&self) -> BTreeMap<String, ProgramParamValue> {
        Limit::ALL
            .into_iter()
            .map(|l| {
                let v = i64::try_from(self.get(l)).unwrap_or(i64::MAX);
                (l.key().to_owned(), ProgramParamValue::Int(v))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_name_their_registry_rows() {
        for l in Limit::ALL {
            let def = qp_schema::params::by_key(l.key()).expect("registered");
            assert_eq!(def, l.def());
            assert_eq!(def.default, ParamValue::Int(l.default_value() as i64));
        }
        assert_eq!(Limits::default().get(Limit::MaxCostPerBar), 666);
    }

    #[test]
    fn invalid_values_fall_back() {
        let l = Limits::from_params(|k| match k {
            "program.max_nodes" => Some(10.0),
            "program.max_window" => Some(200_000.0),
            "program.max_outputs" => Some(2.5),
            _ => None,
        });
        assert_eq!(l.get(Limit::MaxNodes), 10);
        assert_eq!(l.get(Limit::MaxWindow), 2000);
        assert_eq!(l.get(Limit::MaxOutputs), 4);
        assert_eq!(l.to_map().len(), N);
    }
}
