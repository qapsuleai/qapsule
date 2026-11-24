//! `expand_op`: one operation expanded to its changes. The dispatcher; each arm is its own
//! function in a topic file (`op/`; RF1 S4, bodies unchanged).

use super::*;

mod finding;
mod people;
mod series;
mod view;

/// The document, the op's path and the expansion context, handed together to the arms with
/// many fields (RF1 S4: at most seven parameters).
pub(super) struct Arm<'a, 'c> {
    pub(super) doc: &'a Document,
    pub(super) p: &'a str,
    pub(super) ctx: &'a mut Ctx<'c>,
}

/// Expands one op into changes. `doc` is the scratch document *before* this op.
#[allow(clippy::too_many_lines)] // the dispatcher: one arm per op, fields opened (RF1 S4)
pub fn expand_op(doc: &Document, op: &Op, p: &str, ctx: &mut Ctx<'_>) -> PResult<Vec<ChangeOp>> {
    match op {
        Op::PutSeries { series } => series::put_series(doc, series, p, ctx),
        Op::AddStudy {
            id,
            study,
            inputs,
            params,
            show,
        } => series::add_study(Arm { doc, p, ctx }, id, study, inputs, params, show),
        Op::SetParams { series, params } => series::set_params(doc, series, params, p),
        Op::PutView { view } => view::put_view(doc, view, p, ctx),
        Op::AddLayer { view, layer, index } => view::add_layer(doc, view, layer, index, p, ctx),
        Op::RemoveLayer { view, index } => view::remove_layer(doc, view, index, p),
        Op::SetViewRange {
            view,
            from_bar,
            to_bar,
        } => view::set_view_range(doc, view, from_bar, to_bar, p, ctx),
        Op::AddFact {
            id,
            kind,
            series,
            bar,
            text,
            evidence,
        } => finding::add_fact(Arm { doc, p, ctx }, id, kind, series, bar, text, evidence),
        Op::AddFinding {
            id,
            claim,
            facts,
            confidence,
            invalidation,
            capsule,
        } => finding::add_finding(
            Arm { doc, p, ctx },
            id,
            claim,
            facts,
            confidence,
            invalidation,
            capsule,
        ),
        Op::UpdateFinding {
            finding,
            claim,
            confidence,
            invalidation,
        } => finding::update_finding(doc, finding, claim, confidence, invalidation, p, ctx),
        Op::ApproveFinding { finding, note } | Op::RejectFinding { finding, note } => {
            finding::decide_finding(doc, op, finding, note, p, ctx)
        }
        Op::AddIntent { id, text, trigger } => people::add_intent(doc, id, text, trigger, p, ctx),
        Op::SetIntentActive { intent, active } => {
            people::set_intent_active(doc, intent, active, p, ctx)
        }
        Op::AddWatch { .. } | Op::RemoveWatch { .. } => crate::watch::expand_watch(doc, op, p, ctx),
        Op::AddComment {
            id,
            about,
            lang,
            text,
            cites,
            external,
        } => people::add_comment(Arm { doc, p, ctx }, id, about, lang, text, cites, external),
        Op::AddNarrative { id, about, steps } => {
            finding::add_narrative(doc, id, about, steps, p, ctx)
        }
        Op::ShowFinding { finding, view } => finding::show_finding(doc, finding, view, p, ctx),
        Op::Remove { id } => people::remove(doc, id, p, ctx),
        Op::AppendBars { series, bars } => series::append_bars(doc, series, bars, p),
        Op::UpdateLastBar { series, bar } => series::update_last_bar(doc, series, bar, p),
        Op::Undo { seq } => expand_undo(doc, *seq, p, ctx),
        Op::SetAudience {
            principal,
            audience,
        } => people::set_audience(principal, audience, p, ctx),
        Op::SetSensitivity {
            principal,
            sensitivity,
        } => people::set_sensitivity(doc, principal, sensitivity, p, ctx),
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
        Op::RejectProposal { proposal, note } => {
            people::reject_proposal(doc, proposal, note, p, ctx)
        }
        Op::AcceptProposal { .. } => Err(invalid(
            "accept_proposal must be the only op of its command",
            p,
        )),
    }
}
