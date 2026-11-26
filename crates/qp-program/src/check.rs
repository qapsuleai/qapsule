//! The `Check` phase (PS §2.1, §2.3): the program-series guards over every op, then the
//! program ops of the command — or of the proposal it accepts, with the decider as owner —
//! expanded on a copy under the host's limits (`SessionView::param`, PR §8.4): the
//! program's limits and the owner's live budget. Nothing is written here (ARCHITECTURE
//! "Yazarken dikkat": no slot state; the dry run's document is dropped).

use std::cell::Cell;

use qp_lang::{Lang, Limits};
use qp_ops::expand::{expand_ops_with, Ctx};
use qp_plane::plane::Reader;
use qp_schema::protocol::{Op, ProtocolError, Role};
use qp_schema::Author;
use qp_subsystem::{SessionCtx, SessionView};

use crate::expand::Rules;
use crate::guard::guards;
use crate::ops::expand_with;

/// Whether `op` is a program op.
fn is_program_op(op: &Op) -> bool {
    matches!(
        op,
        Op::AddProgram { .. } | Op::SetProgramParams { .. } | Op::RemoveProgram { .. }
    )
}

/// Who writes the ops being checked: the speaker and, on an acceptance, the decider.
struct Writer {
    speaker: String,
    author: Author,
    approved_by: Option<Author>,
    is_owner: bool,
    /// The seq in assigned ids (an accepted proposal's own).
    id_seq: Option<u64>,
}

/// The principal id of `a` among the session's principals (else its name).
fn principal_of(view: &dyn SessionView, a: &Author) -> String {
    let name = match a {
        Author::Human { name } | Author::Agent { name, .. } | Author::Engine { name, .. } => name,
    };
    match view.principal(name) {
        Some(p) if &p.author == a => p.id.clone(),
        _ => name.clone(),
    }
}

/// The program ops to check and who writes them; `None`: nothing of ours.
fn program_ops(view: &dyn SessionView, conn: &str, ops: &[Op]) -> Option<(Vec<Op>, Writer)> {
    let c = view.conn(conn)?;
    let me = view.principal(c.principal)?;
    if let [Op::AcceptProposal { proposal, .. }] = ops {
        let pr = view
            .document()
            .proposals
            .iter()
            .find(|p| &p.id == proposal)?;
        if !pr.ops.iter().any(is_program_op) {
            return None;
        }
        let proposer = principal_of(view, &pr.provenance.author);
        let is_owner = view
            .principal(&proposer)
            .is_some_and(|p| p.role == Role::Owner);
        let w = Writer {
            speaker: proposer,
            author: pr.provenance.author.clone(),
            approved_by: Some(me.author.clone()),
            is_owner,
            id_seq: proposal.strip_prefix("prop-").and_then(|s| s.parse().ok()),
        };
        return Some((pr.ops.clone(), w));
    }
    if !ops.iter().any(is_program_op) {
        return None;
    }
    let w = Writer {
        speaker: me.id.clone(),
        author: me.author.clone(),
        approved_by: None,
        is_owner: me.role == Role::Owner,
        id_seq: None,
    };
    Some((ops.to_vec(), w))
}

/// The `Check` phase of connection `conn`'s command `ops`.
///
/// # Errors
/// A guard, a program limit or the live budget refuses it.
#[allow(clippy::result_large_err)]
pub(crate) fn check(conn: &str, ops: &[Op], ctx: &mut SessionCtx<'_>) -> Result<(), ProtocolError> {
    let view = ctx.view;
    guards(view.document(), ops)?;
    let Some((ops, w)) = program_ops(view, conn, ops) else {
        return Ok(());
    };
    let lang = view
        .conn(conn)
        .and_then(|c| c.hello.lang)
        .map_or(Lang::En, |l| match l {
            qp_schema::protocol::Language::Tr => Lang::Tr,
            qp_schema::protocol::Language::En => Lang::En,
        });
    let rules = Rules {
        limits: Limits::from_params(|k| view.param(k)),
        lang,
    };
    let mut qctx = Ctx {
        author: w.author,
        approved_by: w.approved_by,
        at: view.clock(),
        seq: view.seq() + 1,
        id_seq: w.id_seq.unwrap_or(view.seq() + 1),
        n: 0,
        created: Vec::new(),
        speaker: &w.speaker,
        is_owner: w.is_owner,
        can_decide: false,
        log: view.log(),
        undoes: None,
        data: Some(Reader {
            rd: view.reading(),
            plane: view.plane(),
        }),
        plane: view.plane(),
    };
    // Only a program op's refusal is ours: any other error is the command path's to give.
    let ours = Cell::new(false);
    let reg = ctx.registry;
    let res = expand_ops_with(view.document(), &ops, "/ops", &mut qctx, |d, op, p, c| {
        ours.set(is_program_op(op));
        if is_program_op(op) {
            expand_with(d, op, p, c, rules)
        } else {
            reg.expand_op(d, op, p, c)
        }
    });
    match res {
        Err(e) if ours.get() => Err(e),
        _ => Ok(()),
    }
}
