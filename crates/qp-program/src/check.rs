//! The `Check` phase (PS §2.1, §2.3): the program-series guards over every op (and over
//! the ops of a proposal being accepted), then the command — its program ops, the ops of
//! the proposal it accepts (the decider as owner) or an `undo` that may restore program
//! series — expanded on a copy under the host's limits (`SessionView::param`, PR §8.4):
//! the program's limits and every restored or written owner's live budget, with the
//! log's program bytes. On an acceptance the owner the expansion will write (the decider's
//! author name read as a principal, `crate::expand::owner_of`) must be the deciding
//! connection's person, else it is refused (missing hook: the decider's principal in
//! `qp_ops::Ctx`). Nothing is written here (ARCHITECTURE "Yazarken dikkat").

use std::cell::Cell;

use qp_lang::{Lang, Limits};
use qp_ops::expand::{expand_ops_with, Ctx};
use qp_plane::plane::Reader;
use qp_schema::protocol::{Op, ProtocolError, Role};
use qp_schema::Author;
use qp_subsystem::{SessionCtx, SessionView};

use crate::budget::{logged, over_budget};
use crate::expand::author_name;
use crate::expand::Rules;
use crate::guard::guards;
use crate::ops::expand_with;
use crate::refuse::{refuse, under};

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
        if !pr
            .ops
            .iter()
            .any(|o| is_program_op(o) || matches!(o, Op::Undo { .. }))
        {
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
    if !ops
        .iter()
        .any(|o| is_program_op(o) || matches!(o, Op::Undo { .. }))
    {
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

/// The ops of the proposal `ops` accepts, if it is a lone `accept_proposal`.
fn accepted<'d>(view: &'d dyn SessionView, ops: &[Op]) -> Option<&'d [Op]> {
    let [Op::AcceptProposal { proposal, .. }] = ops else {
        return None;
    };
    let doc = view.document();
    doc.proposals
        .iter()
        .find(|p| &p.id == proposal)
        .map(|p| p.ops.as_slice())
}

/// The message language of connection `conn`.
fn lang_of(view: &dyn SessionView, conn: &str) -> Lang {
    match view.conn(conn).and_then(|c| c.hello.lang) {
        Some(qp_schema::protocol::Language::Tr) => Lang::Tr,
        _ => Lang::En,
    }
}

/// On an acceptance: the person the expansion will charge (the decider's author name
/// read as a principal) is the deciding connection's person.
///
/// # Errors
/// `unsupported` when they differ (the expansion cannot know the decider's principal).
#[allow(clippy::result_large_err)]
fn decider_identity(view: &dyn SessionView, conn: &str, w: &Writer) -> Result<(), ProtocolError> {
    let (Some(a), Some(c)) = (&w.approved_by, view.conn(conn)) else {
        return Ok(());
    };
    let person = |p: &str| qp_ops::watch::person_of(view.plane(), p).to_owned();
    let (exact, read) = (person(c.principal), person(author_name(a)));
    if exact == read {
        return Ok(());
    }
    Err(qp_plane::error::E::new(
        qp_schema::protocol::ErrorCode::Unsupported,
        format!(
            "this proposal writes programs owned by its decider, and the decider's principal `{exact}` cannot be named from its author `{read}` on the command path"
        ),
    )
    .at("/ops/0")
    .reason("owner_unresolved")
    .hint("Missing hook: the decider's principal id in the expansion context. Until then, a person whose principal id is their author name decides program proposals.")
    .build())
}

/// The `Check` phase of connection `conn`'s command `ops`.
///
/// # Errors
/// A guard, a program limit or the live budget refuses it.
#[allow(clippy::result_large_err)]
pub(crate) fn check(conn: &str, ops: &[Op], ctx: &mut SessionCtx<'_>) -> Result<(), ProtocolError> {
    let view = ctx.view;
    guards(view.document(), ops)?;
    if let Some(inner) = accepted(view, ops) {
        guards(view.document(), inner).map_err(|mut e| {
            e.message = format!("the proposal cannot be applied now: {}", e.message);
            e
        })?;
    }
    let Some((ops, w)) = program_ops(view, conn, ops) else {
        return Ok(());
    };
    decider_identity(view, conn, &w)?;
    let rules = Rules {
        limits: Limits::from_params(|k| view.param(k)),
        lang: lang_of(view, conn),
    };
    let can_decide = view.conn(conn).is_some_and(|c| c.permissions.decide);
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
        can_decide,
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
    let ex = match res {
        Ok(ex) => ex,
        Err(e) if ours.get() => return Err(e),
        Err(_) => return Ok(()),
    };
    // The whole command against every owner it writes or restores, with the log's bytes.
    let prior = logged(view.log());
    let errs = over_budget(&ex.doc, &prior, &ex.changes, &rules.limits, rules.lang);
    if errs.is_empty() {
        Ok(())
    } else {
        Err(refuse(under(&errs, "/ops/0", "/ops/0")))
    }
}
