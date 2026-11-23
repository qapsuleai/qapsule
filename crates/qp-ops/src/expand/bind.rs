//! Entitlement, binding and layer checks used while expanding.

use super::*;

/// A condition evaluated on bound data (a finding's invalidation, an intent's trigger)
/// tells its outcome to whoever wrote it: the writer needs the data entitlement of every
/// bound feed the condition reads, through study inputs (§18.3); otherwise
/// `no_entitlement` at the series path.
pub(crate) fn cond_entitled(
    doc: &Document,
    ctx: &Ctx<'_>,
    c: &Condition,
    path: &str,
) -> PResult<()> {
    condition_entitled(ctx.plane, doc, ctx.speaker, c, path)
}

/// [`cond_entitled`] for `principal` (also the `condition` subscription, §18.3).
pub fn condition_entitled(
    plane: &qp_plane::plane::DataPlane,
    doc: &Document,
    principal: &str,
    c: &Condition,
    path: &str,
) -> PResult<()> {
    for (field, s) in condition_series(c) {
        if let Some(b) = plane.unentitled(doc, principal, s) {
            let mut e = b.error(s, &ptr(path, field));
            e.hint = Some(
                "A condition on bound data would tell you its values through its outcome; the data licence of this feed does not cover you. Ask the host for an entitlement."
                    .to_owned(),
            );
            return Err(e);
        }
    }
    Ok(())
}

/// True if a finding's invalidation or an intent's trigger in `doc` rests on series
/// `id` (itself or through study inputs).
fn read_by_conditions(doc: &Document, id: &str) -> bool {
    let reads = |c: &Condition| crate::live::closure_of(doc, c).iter().any(|s| s == id);
    doc.findings.iter().any(|f| reads(&f.invalidation))
        || doc
            .intents
            .iter()
            .any(|i| i.trigger.as_ref().is_some_and(reads))
}

/// Rewriting a series that a condition rests on (a finding's invalidation, an intent's
/// trigger) re-points that condition at other data: the writer needs the data
/// entitlement of every bound feed the new version rests on (its binding, or through
/// its study inputs), else `no_entitlement` at the series' source (§18.3). Otherwise an
/// unentitled writer could aim a condition someone else wrote, or one they wrote on an
/// inline series, at a feed they may not read.
pub(super) fn rebind_entitled(doc: &Document, ctx: &Ctx<'_>, s: &Series, sp: &str) -> PResult<()> {
    let src = ptr(sp, "source");
    let blocked = match &s.source {
        SeriesSource::Bind { .. } => s
            .binding()
            .map(|b| b.feed_key())
            .filter(|k| !ctx.plane.entitled(ctx.speaker, k))
            .map(|k| {
                (
                    s.id.clone(),
                    qp_plane::plane::Blocked::unentitled(&s.id, ctx.speaker, &k),
                    src.clone(),
                )
            }),
        SeriesSource::Study { inputs, .. } => inputs.iter().enumerate().find_map(|(i, inp)| {
            ctx.plane
                .unentitled(doc, ctx.speaker, inp)
                .map(|b| (inp.clone(), b, ptr(&ptr(&src, "inputs"), i)))
        }),
        _ => None,
    };
    let Some((about, b, path)) = blocked else {
        return Ok(());
    };
    if !read_by_conditions(doc, &s.id) {
        return Ok(());
    }
    let mut e = b.error(&about, &path);
    e.hint = Some(
        "A condition rests on this series: re-pointing it at a feed would tell you that feed's values through the condition's outcome, and its data licence does not cover you. Ask the host for an entitlement."
            .to_owned(),
    );
    Err(e)
}

/// `append_bars` / `update_last_bar` are for inline series (§18.2).
pub(super) fn bound_no_bar_writes(s: &Series, path: &str) -> PResult<()> {
    if matches!(s.source, SeriesSource::Bind { .. }) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "series `{}` is bound: the data of a bound series comes from the data plane",
                s.id
            ),
        )
        .at(path)
        .hint("Bar writes are for inline series; the host delivers bound data as data events.")
        .build());
    }
    Ok(())
}

/// Most bound series a document may hold (`max_ops × 4`, §18.7).
pub(crate) const MAX_BOUND_SERIES: usize = 256;

/// Canonical interval spelling (§18.2): `Nm` only when N is not a multiple of 60, no
/// leading zeros. Returns the canonical form when `s` is a valid but non-canonical one.
pub(super) fn canonical_interval(s: &str) -> Option<String> {
    let split = s.find(|c: char| !c.is_ascii_digit())?;
    let (num, unit) = s.split_at(split);
    let n: u32 = num.parse().ok()?;
    let canon = if unit == "m" && n.is_multiple_of(60) {
        format!("{}h", n / 60)
    } else {
        format!("{n}{unit}")
    };
    (canon != s).then_some(canon)
}

pub(super) fn inline_bars(s: &Series) -> Option<&[qp_schema::Bar]> {
    if matches!(s.source, SeriesSource::Inline {}) {
        s.bars()
    } else {
        None
    }
}

/// Whether a layer refers to series `id` other than through its `series` field (a field
/// of another series in a channel, or a `series` origin).
fn layer_refs_series(l: &Layer, id: &str) -> bool {
    let pos = |p: &Option<Position>| matches!(p, Some(Position::Ref(r)) if r.series.as_deref() == Some(id));
    let e = &l.encoding;
    pos(&e.x)
        || pos(&e.y)
        || pos(&e.y2)
        || matches!(&e.color, Some(ColorSpec::When(w))
            if matches!(&w.when.than, Operand::Ref(r) if r.series.as_deref() == Some(id)))
        || l.origin
            .values()
            .any(|o| matches!(o, Origin::Series { series, .. } if series == id))
}

/// Views whose layers or emphasis depend on item `id` (beyond layers drawing series
/// `id`, which are removed with it).
pub(super) fn view_deps(doc: &Document, id: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for v in &doc.views {
        let layer_dep = v.layers.iter().any(|l| {
            l.fact.as_deref() == Some(id)
                || l.finding.as_deref() == Some(id)
                || layer_refs_series(l, id)
                || l.origin
                    .values()
                    .any(|o| matches!(o, Origin::Fact { fact, .. } if fact == id))
        });
        let drawn: BTreeSet<&str> = v
            .layers
            .iter()
            .filter(|l| l.series.as_deref() == Some(id))
            .filter_map(|l| l.id.as_deref())
            .collect();
        let emph_dep = v.emphasis.iter().any(|e| {
            e.target.item.as_deref() == Some(id)
                || e.target.series.as_deref() == Some(id)
                || e.target.layer.as_deref().is_some_and(|l| drawn.contains(l))
        });
        if layer_dep || emph_dep {
            out.insert(v.id.clone());
        }
    }
    out
}

/// Checks a target's `layer` against its view (or any view when no view is given).
pub(super) fn check_target_layer(doc: &Document, t: &Target, path: &str) -> PResult<()> {
    let Some(l) = &t.layer else { return Ok(()) };
    let views: Vec<&View> = match &t.view {
        Some(v) => doc.views.iter().filter(|x| &x.id == v).collect(),
        None => doc.views.iter().collect(),
    };
    if views.iter().any(|v| v.layer(l).is_some()) {
        return Ok(());
    }
    Err(
        E::new(ErrorCode::NotFound, format!("layer `{l}` does not exist"))
            .at(ptr(path, "layer"))
            .dym(
                l,
                views
                    .iter()
                    .flat_map(|v| v.layers.iter().filter_map(|x| x.id.as_deref())),
            )
            .build(),
    )
}
