//! The limits of the language (PR §8.4; `docs/PARAMETERS.md` "RL1"): every bound the
//! checker, the executor and the budget use, filled by the caller from the parameter
//! registry (`SessionView::param(key)`); a missing or invalid value takes the default.
//!
//! Defaults come from `qp_schema::params::REGISTRY` when it has the key. The RL1 rows are
//! not in the registry yet (`docs/PARAMETERS.md` says they enter it directly; the registry
//! is `qp-schema`'s, outside this crate), so [`Limit::row`] holds the documented
//! default and range — the one place in code — and a test fails as soon as a registry row
//! disagrees with it. When the rows land, the table can shrink to the keys.

use std::collections::BTreeMap;

use qp_schema::params::{by_key, ParamValue};
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
}

/// Number of limit keys.
const N: usize = 17;

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
    ];

    /// `(key, documented default, min, max)` of the registry row (`docs/PARAMETERS.md`).
    #[must_use]
    pub const fn row(self) -> (&'static str, u64, u64, Option<u64>) {
        match self {
            Limit::MaxNodes => ("program.max_nodes", 200, 1, None),
            Limit::MaxWindow => ("program.max_window", 2000, 1, Some(100_000)),
            Limit::MaxOutputs => ("program.max_outputs", 4, 1, None),
            Limit::MaxParams => ("program.max_params", 16, 0, None),
            Limit::MaxIrBytes => ("program.max_ir_bytes", 8192, 256, None),
            Limit::MaxCostPerBar => ("program.max_cost_per_bar", 666, 1, None),
            Limit::SignalMaxHorizonBars => ("program.signal.max_horizon_bars", 500, 1, None),
            Limit::CheckMaxErrors => ("program.check.max_errors", 50, 1, None),
            Limit::LiveDocMaxInstances => ("program.live.doc.max_instances", 32, 0, None),
            Limit::LiveDocMaxCostPerBar => ("program.live.doc.max_cost_per_bar", 2000, 0, None),
            Limit::LiveDocMaxLogBytes => ("program.live.doc.max_log_bytes", 4_194_304, 0, None),
            Limit::TryMaxBars => ("program.try.max_bars", 20_000, 1, None),
            Limit::TryMaxWork => ("program.try.max_work", 50_000_000, 1, None),
            Limit::TryMaxItems => ("program.try.max_items", 10, 1, Some(100)),
            Limit::WorkRecord => ("program.work.record", 16, 0, None),
            Limit::WorkOpenRecord => ("program.work.open_record", 1, 0, None),
            Limit::WorkRead => ("program.work.read", 1, 0, None),
        }
    }

    /// The registry key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        self.row().0
    }

    /// The default: the registry row's when the registry has the key, else the
    /// documented one.
    #[must_use]
    pub fn default_value(self) -> u64 {
        match by_key(self.key()).map(|d| d.default) {
            Some(ParamValue::Int(v)) => u64::try_from(v).unwrap_or(self.row().1),
            _ => self.row().1,
        }
    }

    /// The allowed range `[min, max]` (registry row's when present).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn range(self) -> (f64, f64) {
        let (_, _, min, max) = self.row();
        let doc = (min as f64, max.map_or(f64::INFINITY, |m| m as f64));
        by_key(self.key()).map_or(doc, |d| {
            (d.min.unwrap_or(doc.0), d.max.unwrap_or(f64::INFINITY))
        })
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
    fn documented_defaults_agree_with_registry_rows() {
        for l in Limit::ALL {
            if let Some(def) = by_key(l.key()) {
                assert_eq!(
                    def.default,
                    ParamValue::Int(l.row().1 as i64),
                    "{}",
                    l.key()
                );
            }
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
        assert_eq!(l.to_map().len(), 17);
    }
}
