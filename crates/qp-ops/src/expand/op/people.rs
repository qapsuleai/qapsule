//! (RF1 S4) `expand_op` arms for intents, comments, removal, audiences and proposals:
//! `add_intent`, `set_intent_active`, `add_comment`, `remove`, `set_audience`,
//! `set_sensitivity`, `reject_proposal`.

use super::*;

/// `add_intent`: an intent, optionally with a trigger.
pub(super) fn add_intent(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let Op::AddIntent { id, text, trigger } = op else {
        unreachable!("only add_intent")
    };
    if let Some(t) = trigger {
        check_condition(doc, t, &ptr(p, "trigger"))?;
        cond_entitled(doc, ctx, t, &ptr(p, "trigger"))?;
    }
    let iid = ctx.new_id(doc, "intent", id.as_ref(), &ptr(p, "id"))?;
    Ok(vec![ChangeOp::PutIntent {
        index: None,
        intent: Intent {
            id: iid,
            text: text.clone(),
            trigger: trigger.clone(),
            active: true,
            provenance: ctx.prov(trigger.as_ref().map_or_else(Vec::new, |t| {
                condition_series(t)
                    .into_iter()
                    .map(|(_, s)| s.to_owned())
                    .collect()
            })),
            watch: None,
            state: None,
            matches: 0,
        },
    }])
}

/// `set_intent_active`: an intent (not a watch) turned on or off.
pub(super) fn set_intent_active(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let Op::SetIntentActive { intent, active } = op else {
        unreachable!("only set_intent_active")
    };
    // (R5B, §25.5) Another person's watch reads as a missing intent; suggestions
    // come from the intents the speaker may see.
    let visible = |x: &&Intent| crate::watch::intent_visible(x, ctx);
    let i = doc
        .intents
        .iter()
        .filter(visible)
        .find(|x| &x.id == intent)
        .ok_or_else(|| {
            E::new(
                ErrorCode::NotFound,
                format!("intent `{intent}` does not exist"),
            )
            .at(ptr(p, "intent"))
            .dym(
                intent,
                doc.intents.iter().filter(visible).map(|x| x.id.as_str()),
            )
            .build()
        })?;
    if i.watch.is_some() {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("`{intent}` is a watch: it ends with `remove_watch`"),
        )
        .at(ptr(p, "intent"))
        .hint("Send remove_watch with this intent id to end a watch.")
        .build());
    }
    let mut ni = i.clone();
    ni.active = *active;
    Ok(vec![ChangeOp::PutIntent {
        intent: ni,
        index: None,
    }])
}

/// `add_comment`: a comment about a capsule or an instrument.
pub(super) fn add_comment(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let Op::AddComment {
        id,
        about,
        lang,
        text,
        cites,
        external,
    } = op
    else {
        unreachable!("only add_comment")
    };
    let ap = ptr(p, "about");
    let about = match (&about.capsule, &about.instrument) {
        (Some(c), None) => {
            let cap = doc
                .capsules
                .iter()
                .find(|x| &x.id == c || &x.finding == c)
                .ok_or_else(|| {
                    E::new(ErrorCode::NotFound, format!("no capsule `{c}`"))
                        .at(ptr(&ap, "capsule"))
                        .dym(c, doc.capsules.iter().map(|x| x.id.as_str()))
                        .build()
                })?;
            qp_schema::protocol::CommentAbout {
                capsule: Some(cap.id.clone()),
                instrument: None,
            }
        }
        (None, Some(i)) => {
            let known = doc.series.iter().any(|s| s.symbol() == Some(i.as_str()));
            if !known {
                return Err(E::new(
                    ErrorCode::NotFound,
                    format!("no series of instrument `{i}`"),
                )
                .at(ptr(&ap, "instrument"))
                .dym(i, doc.series.iter().filter_map(|s| s.symbol()))
                .build());
            }
            about.clone()
        }
        _ => {
            return Err(E::new(
                ErrorCode::Invalid,
                "`about` names exactly one of `capsule` and `instrument`",
            )
            .at(ap)
            .expected(["capsule", "instrument"])
            .build())
        }
    };
    // Cites are the validator's (`unknown_id`, the same for a hidden id).
    let cid = ctx.new_id(doc, "cmt", id.as_ref(), &ptr(p, "id"))?;
    Ok(vec![ChangeOp::PutComment {
        index: None,
        comment: qp_schema::protocol::Comment {
            id: cid,
            about,
            lang: *lang,
            text: text.clone(),
            cites: cites.clone(),
            external: external.clone(),
            strictness: ctx.plane.comment_strictness,
            provenance: ctx.prov(cites.clone()),
            on_behalf: ctx.plane.delegates.get(ctx.speaker).cloned(),
        },
    }])
}

/// `remove`: an item removed (a watch ends with `remove_watch`).
pub(super) fn remove(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let Op::Remove { id } = op else {
        unreachable!("only remove")
    };
    // (R5B, §25.5) A watch: another person's reads as missing; one's own ends
    // with `remove_watch`.
    if let Some(i) = doc
        .intents
        .iter()
        .find(|i| &i.id == id && i.watch.is_some())
    {
        if !crate::watch::intent_visible(i, ctx) {
            return Err(
                E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                    .at(ptr(p, "id"))
                    .dym(id, crate::watch::visible_ids(doc, ctx))
                    .build(),
            );
        }
        return Err(E::new(
            ErrorCode::Invalid,
            format!("`{id}` is a watch: it ends with `remove_watch`"),
        )
        .at(ptr(p, "id"))
        .hint("Send remove_watch with this intent id to end a watch.")
        .build());
    }
    if find_collection(doc, id).is_none() {
        return Err(
            E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                .at(ptr(p, "id"))
                .dym(id, crate::watch::visible_ids(doc, ctx))
                .build(),
        );
    }
    expand_remove(doc, id, p)
}

/// `set_audience`: a principal's audience.
pub(super) fn set_audience(op: &Op, p: &str, ctx: &mut Ctx<'_>) -> PResult<Vec<ChangeOp>> {
    let Op::SetAudience {
        principal,
        audience,
    } = op
    else {
        unreachable!("only set_audience")
    };
    let who = audience_target(principal.as_deref(), p, ctx)?;
    for (sym, q) in &audience.positions {
        if !q.is_finite() {
            return Err(invalid(
                format!("position of `{sym}` must be a finite number"),
                ptr(&ptr(&ptr(p, "audience"), "positions"), sym),
            ));
        }
    }
    if let Some(i) = audience.watchlist.iter().position(String::is_empty) {
        return Err(invalid(
            "a watch-list symbol must not be empty",
            ptr(&ptr(&ptr(p, "audience"), "watchlist"), i),
        ));
    }
    Ok(vec![ChangeOp::PutAudience {
        principal: who,
        audience: Some(audience.clone()),
    }])
}

/// `set_sensitivity`: a principal's sensitivity.
pub(super) fn set_sensitivity(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let Op::SetSensitivity {
        principal,
        sensitivity,
    } = op
    else {
        unreachable!("only set_sensitivity")
    };
    let who = audience_target(principal.as_deref(), p, ctx)?;
    let mut a = doc
        .audiences
        .get(&who)
        .cloned()
        .unwrap_or(qp_schema::capsule::Audience {
            group: qp_schema::capsule::UserGroup::Habituated,
            sensitivity: None,
            watchlist: Vec::new(),
            positions: BTreeMap::new(),
        });
    a.sensitivity = *sensitivity;
    Ok(vec![ChangeOp::PutAudience {
        principal: who,
        audience: Some(a),
    }])
}

/// `reject_proposal`: a pending proposal rejected.
pub(super) fn reject_proposal(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let Op::RejectProposal { proposal, note } = op else {
        unreachable!("only reject_proposal")
    };
    let pr = need_proposal(doc, proposal, &ptr(p, "proposal"))?;
    let mut np = pr.clone();
    np.state = ProposalState::Rejected;
    np.decided_by = Some(ctx.author.clone());
    np.note.clone_from(note);
    np.decided_seq = Some(ctx.seq);
    Ok(vec![ChangeOp::PutProposal {
        proposal: np,
        index: None,
    }])
}
