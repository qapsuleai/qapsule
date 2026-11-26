//! The program ops (PS §3.1) and their policy (PS §2.2):
//!
//! | op | allowed | proposable | agent |
//! |---|---|---|---|
//! | `add_program` | `write` | yes | only as a proposal, a new instance only |
//! | `set_program_params` | `write` | yes | only as a proposal |
//! | `remove_program` | `write` | yes | never |
//!
//! The public world and a reader's projection take program series by the rule of every
//! study series (the default `to_public`: the same op).

use qp_lang::{Lang, Limits};
use qp_ops::expand::Ctx;
use qp_plane::error::invalid;
use qp_schema::protocol::{ChangeOp, Op, ProtocolError};
use qp_schema::Document;
use qp_subsystem::{AgentOp, OpHandler};

use crate::expand::{add_program, remove_program, set_program_params, AddArgs, Rules};
use crate::inst::instance_names;

/// A program op: its wire tag and its variant.
pub(crate) struct ProgramOp {
    tag: &'static str,
    of: fn(&Op) -> bool,
}

/// `add_program`.
pub(crate) static ADD_PROGRAM: ProgramOp = ProgramOp {
    tag: "add_program",
    of: |op| matches!(op, Op::AddProgram { .. }),
};

/// `set_program_params`.
pub(crate) static SET_PROGRAM_PARAMS: ProgramOp = ProgramOp {
    tag: "set_program_params",
    of: |op| matches!(op, Op::SetProgramParams { .. }),
};

/// `remove_program`.
pub(crate) static REMOVE_PROGRAM: ProgramOp = ProgramOp {
    tag: "remove_program",
    of: |op| matches!(op, Op::RemoveProgram { .. }),
};

/// The command path's rules: the registry defaults (`qp_ops::Ctx` has no `param`; the
/// host's values are checked in the `Check` phase, PS §2.3).
fn default_rules() -> Rules {
    Rules {
        limits: Limits::default(),
        lang: Lang::En,
    }
}

/// Expands program op `op` against `doc` under `rules`.
#[allow(clippy::result_large_err)]
pub(crate) fn expand_with(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
    rules: Rules,
) -> Result<Vec<ChangeOp>, ProtocolError> {
    match op {
        Op::AddProgram {
            id,
            program,
            inputs,
            params,
            show,
        } => {
            let a = AddArgs {
                id: id.as_ref(),
                program,
                inputs,
                params,
                show: show.as_ref(),
            };
            add_program(doc, a, p, ctx, rules)
        }
        Op::SetProgramParams { program, params } => {
            set_program_params(doc, program, params, p, ctx, rules)
        }
        Op::RemoveProgram { program } => remove_program(doc, program, p, ctx),
        _ => Err(invalid("not a program op", p)),
    }
}

impl OpHandler for ProgramOp {
    fn op(&self) -> &'static str {
        self.tag
    }
    fn owns(&self, op: &Op) -> bool {
        (self.of)(op)
    }
    fn expand(
        &self,
        doc: &Document,
        op: &Op,
        at: &str,
        ctx: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError> {
        expand_with(doc, op, at, ctx, default_rules())
    }
    /// (PS §2.2) An agent only proposes adding (a new instance) and changing values; it
    /// never removes. The person decides (`accept_proposal`, human origin only).
    fn agent(&self, doc: &Document, op: &Op) -> AgentOp {
        match op {
            Op::AddProgram { id, .. } => {
                let taken = id.as_deref().is_some_and(|id| {
                    instance_names(doc).contains(&id) || doc.series.iter().any(|s| s.id == id)
                });
                if taken {
                    AgentOp::Deny
                } else {
                    AgentOp::Proposal
                }
            }
            Op::SetProgramParams { .. } => AgentOp::Proposal,
            _ => AgentOp::Deny,
        }
    }
}
