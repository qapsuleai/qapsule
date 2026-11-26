//! RL1 compute-language parameters (`docs/PARAMETERS.md` "RL1"; `docs/PROGRAMS.md` §8):
//! the limits of the checker, the executor, the budget and the text language. They have
//! no constant in an owner crate: `qp_lang::Limits` reads these rows directly.

use super::{ChangedBy, ParamDef, ParamKind, ParamScope, ParamValue};

const SYSTEM: &[ParamScope] = &[ParamScope::System];
const SYSTEM_PLAN: &[ParamScope] = &[ParamScope::System, ParamScope::Plan];
const SINCE: &str = "RL1";

/// An integer RL1 row; `plan` rows are sellable entitlements changed by an admin.
const fn row(
    key: &'static str,
    unit: &'static str,
    (min, max): (f64, Option<f64>),
    default: i64,
    plan: bool,
    description: &'static str,
) -> ParamDef {
    ParamDef {
        key,
        kind: ParamKind::Int,
        unit,
        min: Some(min),
        max,
        default: ParamValue::Int(default),
        scopes: if plan { SYSTEM_PLAN } else { SYSTEM },
        changed_by: if plan {
            ChangedBy::Admin
        } else {
            ChangedBy::ReleaseOnly
        },
        entitlement: plan,
        description,
        since: SINCE,
    }
}

/// `program.max_nodes`.
pub const PROGRAM_MAX_NODES: ParamDef = row(
    "program.max_nodes",
    "count",
    (1.0, None),
    200,
    false,
    "Nodes per program at most (`E_TOO_MANY_NODES`).",
);
/// `program.max_window`.
pub const PROGRAM_MAX_WINDOW: ParamDef = row(
    "program.max_window",
    "bars",
    (1.0, Some(100_000.0)),
    2000,
    false,
    "Largest `period`, `n` or `bars` in a program (`E_WINDOW_LIMIT`); at most `study.max_period`.",
);
/// `program.max_outputs`.
pub const PROGRAM_MAX_OUTPUTS: ParamDef = row(
    "program.max_outputs",
    "count",
    (1.0, None),
    4,
    false,
    "Outputs per program at most (`E_TOO_LARGE`).",
);
/// `program.max_params`.
pub const PROGRAM_MAX_PARAMS: ParamDef = row(
    "program.max_params",
    "count",
    (0.0, None),
    16,
    false,
    "Parameters per program at most (`E_TOO_LARGE`).",
);
/// `program.max_ir_bytes`.
pub const PROGRAM_MAX_IR_BYTES: ParamDef = row(
    "program.max_ir_bytes",
    "bytes",
    (256.0, None),
    8192,
    false,
    "Canonical IR size at most; bounds log growth (`E_TOO_LARGE`).",
);
/// `program.max_cost_per_bar`.
pub const PROGRAM_MAX_COST_PER_BAR: ParamDef = row(
    "program.max_cost_per_bar",
    "unit/bar",
    (1.0, None),
    666,
    false,
    "Static cost per bar of one program at most (`E_COST_LIMIT`); ≈ RX4's 2 000 000 units for 3000 bars.",
);
/// `program.signal.max_horizon_bars`.
pub const PROGRAM_SIGNAL_MAX_HORIZON_BARS: ParamDef = row(
    "program.signal.max_horizon_bars",
    "bars",
    (1.0, None),
    500,
    false,
    "Largest `max_bars` of a signal outcome rule (`E_BAD_RULE`).",
);
/// `program.check.max_errors`.
pub const PROGRAM_CHECK_MAX_ERRORS: ParamDef = row(
    "program.check.max_errors",
    "count",
    (1.0, None),
    50,
    false,
    "Errors returned by one check at most; the rest is `truncated`.",
);
/// `program.live.doc.max_instances`.
pub const PROGRAM_LIVE_DOC_MAX_INSTANCES: ParamDef = row(
    "program.live.doc.max_instances",
    "count",
    (0.0, None),
    32,
    true,
    "Per document, per owner: program instances at most (`E_BUDGET`).",
);
/// `program.live.doc.max_cost_per_bar`.
pub const PROGRAM_LIVE_DOC_MAX_COST_PER_BAR: ParamDef = row(
    "program.live.doc.max_cost_per_bar",
    "unit/bar",
    (0.0, None),
    2000,
    true,
    "Per document, per owner: sum of the live cost per bar of the owner's instances at most (`E_BUDGET`).",
);
/// `program.live.doc.max_log_bytes`.
pub const PROGRAM_LIVE_DOC_MAX_LOG_BYTES: ParamDef = row(
    "program.live.doc.max_log_bytes",
    "bytes",
    (0.0, None),
    4_194_304,
    true,
    "Per document, per owner: bytes of program changes in the log at most; bounds log growth by repeated edits (`E_BUDGET`).",
);
/// `program.try.max_bars`.
pub const PROGRAM_TRY_MAX_BARS: ParamDef = row(
    "program.try.max_bars",
    "bars",
    (1.0, None),
    20_000,
    true,
    "Closed bars one try or replay run reads at most (the latest ones).",
);
/// `program.try.max_work`.
pub const PROGRAM_TRY_MAX_WORK: ParamDef = row(
    "program.try.max_work",
    "unit",
    (1.0, None),
    50_000_000,
    true,
    "Work of one try or replay run at most; the run stops with `stopped: budget`.",
);
/// `program.try.max_items`.
pub const PROGRAM_TRY_MAX_ITEMS: ParamDef = row(
    "program.try.max_items",
    "count",
    (1.0, Some(100.0)),
    10,
    false,
    "Values, bars or records listed per output in a try summary.",
);
/// `program.work.record`.
pub const PROGRAM_WORK_RECORD: ParamDef = row(
    "program.work.record",
    "unit",
    (0.0, None),
    16,
    false,
    "Work charged per emitted record (event bar, signal record, state key).",
);
/// `program.work.open_record`.
pub const PROGRAM_WORK_OPEN_RECORD: ParamDef = row(
    "program.work.open_record",
    "unit",
    (0.0, None),
    1,
    false,
    "Work charged per open signal record per bar.",
);
/// `program.work.read`.
pub const PROGRAM_WORK_READ: ParamDef = row(
    "program.work.read",
    "unit",
    (0.0, None),
    1,
    false,
    "Work charged per output cursor read.",
);
/// `program.text.max_source_chars`.
pub const PROGRAM_TEXT_MAX_SOURCE_CHARS: ParamDef = row(
    "program.text.max_source_chars",
    "chars",
    (1.0, None),
    65_536,
    false,
    "Characters of a program text at most (`E_TOO_LARGE`); eight times `program.max_ir_bytes` for comments and spelling.",
);
/// `program.text.max_token_chars`.
pub const PROGRAM_TEXT_MAX_TOKEN_CHARS: ParamDef = row(
    "program.text.max_token_chars",
    "chars",
    (1.0, None),
    1024,
    false,
    "Characters of one token (name, number, string) of a program text at most (`E_TOO_LARGE`).",
);
/// `program.text.max_depth`.
pub const PROGRAM_TEXT_MAX_DEPTH: ParamDef = row(
    "program.text.max_depth",
    "count",
    (1.0, None),
    64,
    false,
    "Expression nesting of a program text at most, parentheses included (`E_TOO_LARGE`).",
);
/// `program.max_ops_per_command`.
pub const PROGRAM_MAX_OPS_PER_COMMAND: ParamDef = row(
    "program.max_ops_per_command",
    "count",
    (1.0, Some(64.0)),
    8,
    false,
    "Program operations (`add_program`, `set_program_params`, `remove_program`) in one command at most (`E_TOO_LARGE`).",
);

/// The RL1 rows, in `docs/PARAMETERS.md` order.
pub const PROGRAM_ROWS: [&ParamDef; 21] = [
    &PROGRAM_MAX_NODES,
    &PROGRAM_MAX_WINDOW,
    &PROGRAM_MAX_OUTPUTS,
    &PROGRAM_MAX_PARAMS,
    &PROGRAM_MAX_IR_BYTES,
    &PROGRAM_MAX_COST_PER_BAR,
    &PROGRAM_SIGNAL_MAX_HORIZON_BARS,
    &PROGRAM_CHECK_MAX_ERRORS,
    &PROGRAM_LIVE_DOC_MAX_INSTANCES,
    &PROGRAM_LIVE_DOC_MAX_COST_PER_BAR,
    &PROGRAM_LIVE_DOC_MAX_LOG_BYTES,
    &PROGRAM_TRY_MAX_BARS,
    &PROGRAM_TRY_MAX_WORK,
    &PROGRAM_TRY_MAX_ITEMS,
    &PROGRAM_WORK_RECORD,
    &PROGRAM_WORK_OPEN_RECORD,
    &PROGRAM_WORK_READ,
    &PROGRAM_TEXT_MAX_SOURCE_CHARS,
    &PROGRAM_TEXT_MAX_TOKEN_CHARS,
    &PROGRAM_TEXT_MAX_DEPTH,
    &PROGRAM_MAX_OPS_PER_COMMAND,
];
