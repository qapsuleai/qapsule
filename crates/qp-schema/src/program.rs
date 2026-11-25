//! Programs of the compute language (`qp-ir/0`, `docs/PROGRAMS.md`): the typed dataflow
//! intermediate representation every indicator, condition, signal and state definition is
//! written in, and its place in a document (`docs/PROGRAMS-SESSION.md` §1).
//!
//! Types only. Checking, running, budgets and the text language live in `qp-lang`; the
//! session subsystem in `qp-program`. The IR is strict (`deny_unknown_fields`); collecting
//! every error of a malformed program at once (PR §11) is the checker's job, on the raw
//! JSON, which is why protocol messages carry an incoming program as JSON.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::TimestampMs;

mod canonical;
mod head;
mod node;
mod output;
mod param;
mod record;
mod reply;

pub use canonical::instance_id_input;
pub use node::*;
pub use output::*;
pub use param::*;
pub use record::*;
pub use reply::*;

/// The IR version string of this module (`Program::version`).
pub const PROGRAM_VERSION: &str = "qp-ir/0";

/// Prefix of `program_id` and `instance_id` (followed by 64 lowercase hex digits).
pub const PROGRAM_ID_PREFIX: &str = "sha256:";

/// `SeriesSource::Study::study` of a program output series (PS §1.1).
pub const PROGRAM_STUDY: &str = "program";

/// A program (PR §2): parameters, nodes in topological order, named outputs.
///
/// Canonical JSON is `serde_json::to_string` of [`Program::normalized`]
/// ([`Program::canonical_json`]); `program_id` hashes [`Program::identity_json`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Program {
    /// Exactly `"qp-ir/0"` ([`PROGRAM_VERSION`]); anything else is `E_VERSION`.
    pub version: String,
    /// Program parameters (PR §2.1); not written when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ProgramParamDef>,
    /// Nodes in topological order: a node refers only to nodes before it.
    pub nodes: Vec<Node>,
    /// Named outputs (at least one).
    pub outputs: Vec<Output>,
}

/// A coded refusal instead of a program (PR §12.1): `{"reject": {"code", "reason"}}`.
/// Not a program; never enters a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramReject {
    /// Sub-code from PR §11.2, e.g. `E_LOOKAHEAD`.
    pub code: String,
    /// Short reason.
    pub reason: String,
}

/// The wrapper object of a refusal: `{"reject": {...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RejectDoc {
    /// The refusal.
    pub reject: ProgramReject,
}

/// What an IR file holds: a program or a refusal (PR §12.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProgramDoc {
    /// A refusal.
    Reject(RejectDoc),
    /// A program.
    Program(Program),
}

/// Who sees a program instance (PS §1.1). RL1: only `document`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgramScope {
    /// Every reader of the document.
    Document,
}

/// The program behind a program output series (`SeriesSource::Study::program`, PS §1.1):
/// the instance, its identities, owner, start and the canonical IR inline. Written by the
/// expansion of program operations only, never by a client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramRef {
    /// Instance name in the document (`add_program` `id`); series ids are
    /// `<instance>.<output>`.
    pub instance: String,
    /// `sha256:` + SHA-256 of the program's identity JSON (PR §2).
    pub program_id: String,
    /// `sha256:` + SHA-256 of [`instance_id_input`] (PR §2.1).
    pub instance_id: String,
    /// Who sees it.
    pub scope: ProgramScope,
    /// The person whose budget it counts against (PS §2.3).
    pub owner: String,
    /// Start of the instance (the expansion's clock); event series are 0 at bars known
    /// at or before it (PS §1.2).
    pub since_t: TimestampMs,
    /// The bound values, complete (defaults filled), by key.
    pub params: BTreeMap<String, ProgramParamValue>,
    /// The canonical IR.
    pub ir: Program,
}
