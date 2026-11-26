//! Qapsule compute language (`qp-ir/0`, `docs/PROGRAMS.md`): the checker, the error model,
//! the static cost, program and instance identities, the language's limits, and the
//! executor (streaming, live transient, higher timeframes, provenance, work budget) with
//! the `try_program` summary.

mod budget;
mod check;
mod checked;
mod diag;
mod exec;
mod identity;
mod limits;
mod message;
mod names;
mod reason;
mod sha256;

pub use budget::{check_live_budget, LiveUsage};
pub use check::{check, check_program, Binding, CheckCtx, CheckReport, Purpose};
pub use checked::{
    Anchor, BinaryKind, Checked, CheckedNode, CheckedOutput, Clock, CondKind, Literal, Op,
    OutputSpec, Rule, Slot, Tf, Ty, UnaryKind, WindowKind,
};
pub use exec::{
    program_digest, try_program, try_window, Align, ExecError, ExecOptions, Executor, Lineage,
    RecordCtx, SignalHit, Stopped, TryRequest, WorkBudget, DIGEST_SUITE,
};
pub use identity::{instance_id, program_id};
pub use limits::{Limit, Limits};
pub use message::Lang;
pub use names::{
    did_you_mean, edit_distance, is_lower_name, is_node_id, suggest, BAR_FIELDS, MAX_CANDIDATES,
    MAX_SUGGEST, OP_NAMES, OUTPUT_KINDS, TURKISH_NAMES,
};
pub use reason::Reason;
pub use sha256::{sha256, sha256_hex};
