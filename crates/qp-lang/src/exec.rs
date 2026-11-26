//! The executor (PR §3.2, §4–§7, §8.3, §9, §10; PS §1): runs a checked program over a bar
//! series. Closed bars are pushed one at a time — batch is the same pushes, byte-equal —
//! and the live (forming) bar is a transient evaluation that never emits. Higher
//! timeframes are built from closed base bars on the instrument's session (or UTC).
//! Every published value traces to its node and base bar range `[lo, i]`. With a work
//! budget the run stops, coded, before the bar that would overrun it.

mod cursor;
mod digest;
mod engine;
mod kernel;
mod node;
mod plan;
mod record;
mod summary;
mod tf;

pub use cursor::Lineage;
pub use digest::{program_digest, DIGEST_SUITE};
pub use engine::Executor;
pub use record::SignalHit;
pub use summary::{try_program, try_window, RecordCtx, TryRequest};
pub use tf::Align;

use crate::limits::{Limit, Limits};

/// Work units of a run (PR §8.3; `program.try.max_work`, `program.work.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkBudget {
    /// Work at most; the run stops before the bar that would exceed it.
    pub max_work: u64,
    /// Per emitted record (event bar, signal record, state key).
    pub record: u64,
    /// Per open signal record per bar.
    pub open_record: u64,
    /// Per output cursor read.
    pub read: u64,
}

impl WorkBudget {
    /// The budget of a try or replay run under `limits`.
    #[must_use]
    pub fn from_limits(limits: &Limits) -> WorkBudget {
        WorkBudget {
            max_work: limits.get(Limit::TryMaxWork),
            record: limits.get(Limit::WorkRecord),
            open_record: limits.get(Limit::WorkOpenRecord),
            read: limits.get(Limit::WorkRead),
        }
    }
}

/// How to run.
#[derive(Debug, Clone)]
pub struct ExecOptions {
    /// Bar boundaries of higher timeframes (the session picks it from the instrument).
    pub align: Align,
    /// Run only these outputs' cones (a document series); all outputs when `None`.
    pub outputs: Option<Vec<String>>,
    /// Work budget (try and replay); none for the live document path, whose work is
    /// bounded statically by `live_per_bar`.
    pub budget: Option<WorkBudget>,
}

impl Default for ExecOptions {
    fn default() -> Self {
        ExecOptions {
            align: Align::Utc,
            outputs: None,
            budget: None,
        }
    }
}

/// Where and why a run stopped (not an error: everything before `bar` stands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stopped {
    /// The bar that would have overrun the budget (not recorded).
    pub bar: u64,
    /// Its time.
    pub t: i64,
    /// The work it would have reached.
    pub work: u64,
    /// The limit.
    pub limit: u64,
}

/// Why an executor cannot be built or fed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    /// A higher timeframe without a bound base interval (check the program bound).
    Unbound(String),
    /// No output of that name.
    UnknownOutput(String),
    /// The compute core refused a kernel: node id, reason.
    Kernel(String, String),
    /// A bar not after the last one.
    NotAscending {
        /// The bar's time.
        t: i64,
        /// The last bar's time.
        last: i64,
    },
    /// `update_last` with another time than the live bar's.
    NotLive {
        /// The bar's time.
        t: i64,
        /// The live bar's time.
        live: i64,
    },
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::Unbound(tf) => write!(f, "timeframe `{tf}` needs a bound base interval"),
            ExecError::UnknownOutput(o) => write!(f, "no output `{o}`"),
            ExecError::Kernel(id, e) => write!(f, "node `{id}`: {e}"),
            ExecError::NotAscending { t, last } => {
                write!(f, "bar t={t} is not after the last bar t={last}")
            }
            ExecError::NotLive { t, live } => {
                write!(f, "bar t={t} is not the live bar t={live}")
            }
        }
    }
}

impl std::error::Error for ExecError {}
