//! `expand_op`: one operation expanded to its changes. The dispatcher; each arm is its own
//! function in a topic file (`op/`; RF1 S4, bodies unchanged).

use super::*;

mod finding;
mod people;
mod series;
mod view;

/// Expands one op into changes. `doc` is the scratch document *before* this op.
pub fn expand_op(doc: &Document, op: &Op, p: &str, ctx: &mut Ctx<'_>) -> PResult<Vec<ChangeOp>> {
    match op {
        Op::PutSeries { .. } => series::put_series(doc, op, p, ctx),
        Op::AddStudy { .. } => series::add_study(doc, op, p, ctx),
        Op::SetParams { .. } => series::set_params(doc, op, p),
        Op::PutView { .. } => view::put_view(doc, op, p, ctx),
        Op::AddLayer { .. } => view::add_layer(doc, op, p, ctx),
        Op::RemoveLayer { .. } => view::remove_layer(doc, op, p),
        Op::SetViewRange { .. } => view::set_view_range(doc, op, p, ctx),
        Op::AddFact { .. } => finding::add_fact(doc, op, p, ctx),
        Op::AddFinding { .. } => finding::add_finding(doc, op, p, ctx),
        Op::UpdateFinding { .. } => finding::update_finding(doc, op, p, ctx),
        Op::ApproveFinding { .. } | Op::RejectFinding { .. } => {
            finding::decide_finding(doc, op, p, ctx)
        }
        Op::AddIntent { .. } => people::add_intent(doc, op, p, ctx),
        Op::SetIntentActive { .. } => people::set_intent_active(doc, op, p, ctx),
        Op::AddWatch { .. } | Op::RemoveWatch { .. } => crate::watch::expand_watch(doc, op, p, ctx),
        Op::AddComment { .. } => people::add_comment(doc, op, p, ctx),
        Op::AddNarrative { .. } => finding::add_narrative(doc, op, p, ctx),
        Op::ShowFinding { .. } => finding::show_finding(doc, op, p, ctx),
        Op::Remove { .. } => people::remove(doc, op, p, ctx),
        Op::AppendBars { .. } => series::append_bars(doc, op, p),
        Op::UpdateLastBar { .. } => series::update_last_bar(doc, op, p),
        Op::Undo { seq } => expand_undo(doc, *seq, p, ctx),
        Op::SetAudience { .. } => people::set_audience(op, p, ctx),
        Op::SetSensitivity { .. } => people::set_sensitivity(doc, op, p, ctx),
        Op::InsertLayer { .. }
        | Op::DeleteLayer { .. }
        | Op::SplitLayer { .. }
        | Op::MoveLayer { .. }
        | Op::SetEncoding { .. }
        | Op::InsertPanel { .. }
        | Op::DeletePanel { .. }
        | Op::MovePanel { .. }
        | Op::SetPanel { .. }
        | Op::SetScale { .. }
        | Op::SetEmphasis { .. } => expand_grammar(doc, op, p, ctx),
        Op::RejectProposal { .. } => people::reject_proposal(doc, op, p, ctx),
        Op::AcceptProposal { .. } => Err(invalid(
            "accept_proposal must be the only op of its command",
            p,
        )),
    }
}
