//! (RF1 S4) `expand_op` arms for facts, findings and narratives: `add_fact`, `add_finding`,
//! `update_finding`, `approve_finding` / `reject_finding`, `add_narrative`, `show_finding`.

use super::*;

/// `add_fact`: a fact on a bar, with frozen evidence.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn add_fact(
    arm: Arm<'_, '_>,
    id: &Option<String>,
    kind: &String,
    series: &String,
    bar: &u64,
    text: &String,
    evidence: &BTreeMap<String, f64>,
) -> PResult<Vec<ChangeOp>> {
    let Arm { doc, p, ctx } = arm;
    need_series(doc, series, &ptr(p, "series"))?;
    // `add_fact` reads the fact's bar time: the speaker needs the data
    // entitlement of a bound series (§18.3).
    if let Some(r) = &ctx.data {
        if let Some(b) = r.plane.gate(doc, Some(ctx.speaker), series) {
            return Err(b.error(series, &ptr(p, "series")));
        }
    }
    let (bs, bars) = qp_plane::plane::bars_for(doc, ctx.data.as_ref(), series)
        .ok_or_else(|| invalid(format!("series `{series}` has no bars"), ptr(p, "series")))?;
    let Some(b) = bars.get(usize::try_from(*bar).unwrap_or(usize::MAX)) else {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("bar {bar} is past the last bar {}", bars.len() - 1),
        )
        .at(ptr(p, "bar"))
        .expected([format!("0..={}", bars.len() - 1)])
        .build());
    };
    if evidence.values().any(|v| !v.is_finite()) {
        return Err(invalid(
            "evidence values must be finite",
            ptr(p, "evidence"),
        ));
    }
    let t = b.t;
    check_evidence_values(doc, ctx, series, *bar, evidence, p)?;
    // Frozen evidence (§18.5): bars 0..=bar of the bar series, by digest.
    let evidence_ref = Some(qp_plane::plane::evidence_ref(
        bs,
        bars,
        usize::try_from(*bar).unwrap_or(0),
    ));
    let fid = ctx.new_id(doc, "fact", id.as_ref(), &ptr(p, "id"))?;
    let fact = Fact {
        id: fid,
        kind: kind.clone(),
        series: series.clone(),
        bar: *bar,
        t,
        text: text.clone(),
        evidence: evidence.clone(),
        evidence_ref,
        meaning: None,
        provenance: ctx.prov(vec![series.clone()]),
    };
    Ok(vec![ChangeOp::PutFact { fact, index: None }])
}

/// `add_finding`: a finding on facts, optionally with a capsule.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn add_finding(
    arm: Arm<'_, '_>,
    id: &Option<String>,
    claim: &String,
    facts: &Vec<String>,
    confidence: &f64,
    invalidation: &Condition,
    capsule: &Option<String>,
) -> PResult<Vec<ChangeOp>> {
    let Arm { doc, p, ctx } = arm;
    if facts.is_empty() {
        return Err(invalid(
            "a finding rests on at least one fact",
            ptr(p, "facts"),
        ));
    }
    for (j, f) in facts.iter().enumerate() {
        if !doc.facts.iter().any(|x| &x.id == f) {
            return Err(
                E::new(ErrorCode::NotFound, format!("fact `{f}` does not exist"))
                    .at(ptr(&ptr(p, "facts"), j))
                    .dym(f, doc.facts.iter().map(|x| x.id.as_str()))
                    .build(),
            );
        }
    }
    check_confidence(*confidence, &ptr(p, "confidence"))?;
    check_condition(doc, invalidation, &ptr(p, "invalidation"))?;
    cond_entitled(doc, ctx, invalidation, &ptr(p, "invalidation"))?;
    let fid = ctx.new_id(doc, "fnd", id.as_ref(), &ptr(p, "id"))?;
    let finding = Finding {
        id: fid.clone(),
        claim: claim.clone(),
        facts: facts.clone(),
        confidence: *confidence,
        invalidation: invalidation.clone(),
        provenance: ctx.prov(facts.clone()),
    };
    let mut out = vec![ChangeOp::PutFinding {
        finding,
        index: None,
    }];
    if let Some(headline) = capsule {
        let cid = ctx.new_id(doc, "cap", None, &ptr(p, "capsule"))?;
        out.push(ChangeOp::PutCapsule {
            index: None,
            capsule: Capsule {
                id: cid,
                finding: fid.clone(),
                state: CapsuleState::Live,
                revision: 0,
                headline: headline.clone(),
                glyphs: Vec::new(),
                view: None,
                story: None,
                depth: Depth::Capsule,
                expires_at: None,
                provenance: ctx.prov(vec![fid]),
            },
        });
    }
    Ok(out)
}

/// `update_finding`: claim, confidence or invalidation changed.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn update_finding(
    doc: &Document,
    finding: &String,
    claim: &Option<String>,
    confidence: &Option<f64>,
    invalidation: &Option<Condition>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let f = need_finding(doc, finding, &ptr(p, "finding"))?;
    if claim.is_none() && confidence.is_none() && invalidation.is_none() {
        return Err(E::new(ErrorCode::Invalid, "nothing to update")
            .at(p)
            .expected(["claim", "confidence", "invalidation"])
            .build());
    }
    let mut nf = f.clone();
    if let Some(c) = claim {
        nf.claim.clone_from(c);
    }
    if let Some(c) = confidence {
        check_confidence(*c, &ptr(p, "confidence"))?;
        nf.confidence = *c;
    }
    if let Some(c) = invalidation {
        check_condition(doc, c, &ptr(p, "invalidation"))?;
        cond_entitled(doc, ctx, c, &ptr(p, "invalidation"))?;
        nf.invalidation = c.clone();
    }
    Ok(vec![ChangeOp::PutFinding {
        finding: nf,
        index: None,
    }])
}

/// `approve_finding` / `reject_finding`: the capsules' state and a ledger row.
pub(super) fn decide_finding(
    doc: &Document,
    op: &Op,
    finding: &String,
    note: &Option<String>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    need_finding(doc, finding, &ptr(p, "finding"))?;
    let state = if matches!(op, Op::ApproveFinding { .. }) {
        CapsuleState::Approved
    } else {
        CapsuleState::Rejected
    };
    let mut out: Vec<ChangeOp> = doc
        .capsules
        .iter()
        .filter(|c| &c.finding == finding)
        .map(|c| ChangeOp::SetCapsuleState {
            capsule: c.id.clone(),
            state,
        })
        .collect();
    out.push(ChangeOp::PushLedger {
        entry: LedgerEntry {
            finding: finding.clone(),
            state: Some(state),
            outcome: Outcome::Pending,
            recorded_at: ctx.at,
            note: note.clone(),
            capsule: None,
            key: None,
            stage: None,
            bar: None,
        },
    });
    Ok(out)
}

/// `add_narrative`: a narrative of timed steps about an item.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn add_narrative(
    doc: &Document,
    id: &Option<String>,
    about: &String,
    steps: &Vec<qp_schema::protocol::NarrativeStep>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    need_item(doc, about, &ptr(p, "about"))?;
    if steps.is_empty() {
        return Err(invalid(
            "a narrative needs at least one step",
            ptr(p, "steps"),
        ));
    }
    let sp = ptr(p, "steps");
    for (j, s) in steps.iter().enumerate() {
        let jp = ptr(&sp, j);
        if j > 0 && s.at_ms <= steps[j - 1].at_ms {
            return Err(E::new(
                ErrorCode::Invalid,
                format!(
                    "steps must be in ascending at_ms order: step {j} at {} ms is not after step {} at {} ms",
                    s.at_ms,
                    j - 1,
                    steps[j - 1].at_ms
                ),
            )
            .at(ptr(&jp, "at_ms"))
            .expected([format!("> {}", steps[j - 1].at_ms)])
            .build());
        }
        check_target(doc, &s.target, &ptr(&jp, "target"))?;
    }
    let nid = ctx.new_id(doc, "narr", id.as_ref(), &ptr(p, "id"))?;
    Ok(vec![ChangeOp::PutNarrative {
        index: None,
        narrative: Narrative {
            id: nid,
            about: about.clone(),
            steps: steps.clone(),
            provenance: ctx.prov(vec![about.clone()]),
        },
    }])
}

/// `show_finding`: the presenter's view of a finding.
pub(super) fn show_finding(
    doc: &Document,
    finding: &String,
    view: &Option<String>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let f = need_finding(doc, finding, &ptr(p, "finding"))?;
    let vid = match view {
        Some(v) => {
            check_id(v, &ptr(p, "view"))?;
            match find_collection(doc, v) {
                None => {
                    ctx.note_created(v);
                }
                Some(qp_schema::protocol::Collection::Views) => {}
                Some(c) => {
                    return Err(E::new(
                        ErrorCode::Duplicate,
                        format!("id `{v}` already exists (in {})", coll_name(c)),
                    )
                    .at(ptr(p, "view"))
                    .build())
                }
            }
            v.clone()
        }
        None => ctx.new_id(doc, "view", None, &ptr(p, "view"))?,
    };
    // The view reads the bar times of the finding's facts' series: on a bound
    // series the speaker needs its data entitlement (§18.3), as for `add_fact`.
    if let Some(r) = &ctx.data {
        for fid in &f.facts {
            if let Some(x) = doc.facts.iter().find(|x| &x.id == fid) {
                if let Some(b) = r.plane.gate(doc, Some(ctx.speaker), &x.series) {
                    return Err(b.error(&x.series, &ptr(p, "finding")));
                }
            }
        }
    }
    let built = crate::present::presenter_view(doc, ctx.data, f, &vid)
        .map_err(|m| invalid(m, ptr(p, "finding")))?;
    let built = canon_view(doc, &built, &ptr(p, "finding"), ctx.reads())?;
    let mut out = vec![ChangeOp::PutView {
        view: built,
        index: None,
    }];
    if let Some(c) = doc.capsules.iter().find(|c| &c.finding == finding) {
        if c.view.as_deref() != Some(vid.as_str()) {
            let mut nc = c.clone();
            nc.view = Some(vid);
            out.push(ChangeOp::PutCapsule {
                capsule: nc,
                index: None,
            });
        }
    }
    Ok(out)
}
