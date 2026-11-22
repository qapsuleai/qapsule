//! `remove` and `undo` expansion and position rebasing.

use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn expand_remove(doc: &Document, id: &str, p: &str) -> PResult<Vec<ChangeOp>> {
    use qp_schema::protocol::Collection as C;
    let ip = ptr(p, "id");
    let Some(coll) = find_collection(doc, id) else {
        return Err(
            E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                .at(ip)
                .dym(id, all_ids(doc))
                .build(),
        );
    };
    let mut deps: BTreeSet<String> = BTreeSet::new();
    let names = |c: &Condition| condition_series(c).iter().any(|(_, s)| *s == id);
    match coll {
        C::Series => {
            for s in &doc.series {
                if let SeriesSource::Study { inputs, .. } = &s.source {
                    if inputs.iter().any(|i| i == id) {
                        deps.insert(s.id.clone());
                    }
                }
            }
            deps.extend(
                doc.facts
                    .iter()
                    .filter(|f| f.series == id)
                    .map(|f| f.id.clone()),
            );
            deps.extend(
                doc.findings
                    .iter()
                    .filter(|f| names(&f.invalidation))
                    .map(|f| f.id.clone()),
            );
            deps.extend(
                doc.intents
                    .iter()
                    .filter(|i| i.trigger.as_ref().is_some_and(names))
                    .map(|i| i.id.clone()),
            );
        }
        C::Facts => deps.extend(
            doc.findings
                .iter()
                .filter(|f| f.facts.iter().any(|x| x == id))
                .map(|f| f.id.clone()),
        ),
        C::Findings => deps.extend(
            doc.capsules
                .iter()
                .filter(|c| c.finding == id)
                .map(|c| c.id.clone()),
        ),
        C::Views => deps.extend(
            doc.capsules
                .iter()
                .filter(|c| c.view.as_deref() == Some(id))
                .map(|c| c.id.clone()),
        ),
        C::Proposals => {
            return Err(invalid(
                "proposals stay in the document as a trace and cannot be removed",
                ip,
            ))
        }
        C::Capsules | C::Intents | C::Narratives | C::Comments => {}
    }
    if matches!(coll, C::Series | C::Facts | C::Findings) {
        deps.extend(view_deps(doc, id));
    }
    if !deps.is_empty() {
        return Err(E::new(
            ErrorCode::InUse,
            format!(
                "`{id}` is used by {}",
                deps.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        )
        .at(ip)
        .expected(deps)
        .hint("Remove the dependent items first.")
        .build());
    }
    let mut out = Vec::new();
    if coll == C::Series {
        for v in &doc.views {
            for (i, l) in v.layers.iter().enumerate().rev() {
                if l.series.as_deref() == Some(id) {
                    out.push(ChangeOp::DeleteLayer {
                        layer: l.id.clone(),
                        view: v.id.clone(),
                        index: u32::try_from(i).unwrap_or(u32::MAX),
                    });
                }
            }
        }
    }
    out.push(ChangeOp::Delete { id: id.to_owned() });
    Ok(out)
}

/// Whether an entry carries decision effects: capsule approved/rejected, ledger
/// records, proposal state changes.
fn has_decision(e: &LogEntry) -> bool {
    let decided = |s: &CapsuleState| matches!(s, CapsuleState::Approved | CapsuleState::Rejected);
    e.changes.iter().chain(&e.inverse).any(|c| match c {
        ChangeOp::SetCapsuleState { state, .. } => decided(state),
        ChangeOp::PushLedger { .. } | ChangeOp::PopLedger {} => true,
        ChangeOp::PutProposal { proposal, .. } => proposal.state != ProposalState::Pending,
        _ => false,
    })
}

/// The bound series an engine entry anchors, if it is an anchor stamp (§18.3): author
/// `qp`, every change a `put_series` of a bound series with `anchor_t` whose inverse is
/// the same series without it.
fn anchor_stamps(e: &LogEntry) -> Option<Vec<&str>> {
    if e.author != crate::session::ENGINE_PRINCIPAL || e.changes.is_empty() {
        return None;
    }
    let mut ids = Vec::new();
    for c in &e.changes {
        let ChangeOp::PutSeries { series, .. } = c else {
            return None;
        };
        let SeriesSource::Bind {
            anchor_t: Some(_), ..
        } = &series.source
        else {
            return None;
        };
        let unstamped = e.inverse.iter().any(|i| {
            matches!(i, ChangeOp::PutSeries { series: o, .. } if o.id == series.id
                && matches!(o.source, SeriesSource::Bind { anchor_t: None, .. }))
        });
        if !unstamped {
            return None;
        }
        ids.push(series.id.as_str());
    }
    Some(ids)
}

#[allow(clippy::too_many_lines)]
pub(super) fn expand_undo(
    doc: &Document,
    seq: Option<Seq>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let undone = undone_set(ctx.log);
    let speaker = ctx.speaker;
    // Only entries the speaker authored are theirs; approving an entry does not make it
    // the approver's to undo (PROTOCOL.md §8.3).
    let mine = |e: &LogEntry| e.author == speaker;
    let target: &LogEntry = match seq {
        Some(s) => {
            let Some(e) = s
                .checked_sub(1)
                .and_then(|i| ctx.log.get(usize::try_from(i).ok()?))
            else {
                return Err(
                    E::new(ErrorCode::NotFound, format!("log entry {s} does not exist"))
                        .at(ptr(p, "seq"))
                        .expected([format!("1..={}", ctx.log.len())])
                        .build(),
                );
            };
            if !mine(e) && !ctx.is_owner {
                return Err(E::new(
                    ErrorCode::Forbidden,
                    format!("entry {s} was written by `{}`; only an owner may undo someone else's entry", e.author),
                )
                .at(ptr(p, "seq"))
                .hint("Undo your own entries, or ask an owner.")
                .build());
            }
            if undone.contains(&s) {
                return Err(E::new(
                    ErrorCode::UndoConflict,
                    format!("entry {s} is already undone"),
                )
                .at(ptr(p, "seq"))
                .hint("To redo, undo the undo entry instead.")
                .build());
            }
            e
        }
        None => ctx
            .log
            .iter()
            .rev()
            .find(|e| mine(e) && !undone.contains(&e.seq) && e.undoes.is_none())
            .ok_or_else(|| {
                E::new(
                    ErrorCode::UndoConflict,
                    format!("`{speaker}` has nothing to undo"),
                )
                .at(p)
                .build()
            })?,
    };
    if !mine(target) && !ctx.is_owner {
        return Err(E::new(
            ErrorCode::Forbidden,
            format!(
                "entry {} was written by `{}`; only an owner may undo someone else's entry",
                target.seq, target.author
            ),
        )
        .at(p)
        .hint("Undo your own entries, or ask an owner.")
        .build());
    }
    if has_decision(target) && !ctx.can_decide {
        return Err(E::new(
            ErrorCode::Forbidden,
            format!(
                "entry {} records a decision (capsule approval/rejection, ledger or proposal state); undoing it needs the decide permission",
                target.seq
            ),
        )
        .at(p)
        .hint("Ask a principal who may decide.")
        .build());
    }
    if let Some(ids) = anchor_stamps(target) {
        return Err(E::new(
            ErrorCode::UndoConflict,
            format!(
                "entry {} is the authority's anchor stamp of bound series {}: an anchor is reset only by a command that rewrites the binding (§18.3), never by undo",
                target.seq,
                ids.iter().map(|i| format!("`{i}`")).collect::<Vec<_>>().join(", ")
            ),
        )
        .at(p)
        .hint("Rewrite the binding with put_series (window, symbol or feed), or undo the entry that put it; the bar numbers of facts and views rest on the anchor.")
        .build());
    }
    // Bindings the target put or rewrote: their anchor stamp (the engine entry right
    // behind) goes with the undo, so it does not block it (§18.3).
    let put: BTreeMap<&str, qp_schema::data::DataBinding> = target
        .changes
        .iter()
        .filter_map(|c| match c {
            ChangeOp::PutSeries { series, .. } => match &series.source {
                SeriesSource::Bind { .. } => Some(series),
                _ => None,
            },
            _ => None,
        })
        .filter_map(|s| Some((s.id.as_str(), crate::plane::binding_of(s)?)))
        .collect();
    let touched = touched_keys(&target.changes);
    let mut conflicts = Vec::new();
    let mut by = String::new();
    for e in &ctx.log[usize::try_from(target.seq)
        .unwrap_or(usize::MAX)
        .min(ctx.log.len())..]
    {
        if undone.contains(&e.seq) || e.undoes.is_some() {
            continue;
        }
        let mut t = touched_keys(&e.changes);
        if !put.is_empty() && anchor_stamps(e).is_some() {
            for c in &e.changes {
                if let ChangeOp::PutSeries { series, .. } = c {
                    let stamps_put = put.get(series.id.as_str()).is_some_and(|b| {
                        crate::plane::binding_of(series)
                            .is_some_and(|s| crate::plane::same_but_anchor(&s, b))
                    });
                    if stamps_put {
                        t.remove(&series.id);
                    }
                }
            }
        }
        if let Some(id) = first_overlap(&t, &touched) {
            if by.is_empty() {
                by = format!("`{id}` changed at seq {} by {}", e.seq, e.author);
            }
            conflicts.push(e.seq);
        }
    }
    if !conflicts.is_empty() {
        return Err(E::new(
            ErrorCode::UndoConflict,
            format!("cannot undo entry {}: {by}", target.seq),
        )
        .at(p)
        .conflicts(conflicts)
        .hint("Undo the later entries first, or change the item with a new command.")
        .build());
    }
    ctx.undoes = Some(target.seq);
    Ok(rebase_positions(doc, &target.inverse))
}

/// Re-anchors the positions of layer and panel changes of an inverse on the current
/// document: later entries that touched *other* layers of the same view may have
/// shifted positions (conflicts are per layer, `docs/PROTOCOL.md` §5.3). A deletion
/// follows its layer id; an insertion keeps its old position, clamped to the list.
fn rebase_positions(doc: &Document, changes: &[ChangeOp]) -> Vec<ChangeOp> {
    let positional = changes.iter().any(|c| {
        matches!(
            c,
            ChangeOp::InsertLayer { .. }
                | ChangeOp::DeleteLayer { .. }
                | ChangeOp::PutLayer { .. }
                | ChangeOp::PutPanel { .. }
        )
    });
    if !positional {
        return changes.to_vec();
    }
    let u = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
    let mut d = doc.clone();
    let mut out = Vec::with_capacity(changes.len());
    for c in changes {
        let view = |id: &str| d.views.iter().find(|v| v.id == id);
        let c2 = match c {
            ChangeOp::DeleteLayer {
                view: vid,
                layer: Some(id),
                index,
            } => {
                let i = view(vid)
                    .and_then(|v| v.layers.iter().position(|l| l.id.as_deref() == Some(id)))
                    .map_or(*index, u);
                ChangeOp::DeleteLayer {
                    view: vid.clone(),
                    index: i,
                    layer: Some(id.clone()),
                }
            }
            ChangeOp::InsertLayer {
                view: vid,
                index,
                layer,
            } => {
                let n = view(vid).map_or(0, |v| v.layers.len());
                ChangeOp::InsertLayer {
                    view: vid.clone(),
                    index: (*index).min(u(n)),
                    layer: layer.clone(),
                }
            }
            ChangeOp::PutLayer {
                view: vid,
                index,
                layer,
            } => {
                let n = view(vid).map_or(0, |v| {
                    v.layers.len() - usize::from(v.layers.iter().any(|l| l.id == layer.id))
                });
                ChangeOp::PutLayer {
                    view: vid.clone(),
                    index: (*index).min(u(n)),
                    layer: layer.clone(),
                }
            }
            ChangeOp::PutPanel {
                view: vid,
                index,
                panel,
            } => {
                let n = view(vid).map_or(0, |v| {
                    v.panels.len() - usize::from(v.panels.iter().any(|x| x.id == panel.id))
                });
                ChangeOp::PutPanel {
                    view: vid.clone(),
                    index: (*index).min(u(n)),
                    panel: panel.clone(),
                }
            }
            other => other.clone(),
        };
        let _ = crate::apply::apply_change(&mut d, &c2);
        out.push(c2);
    }
    out
}
