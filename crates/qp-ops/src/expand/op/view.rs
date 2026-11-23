//! (RF1 S4) `expand_op` arms for views and layers: `put_view`, `add_layer`, `remove_layer`,
//! `set_view_range`.

use super::*;

/// `put_view`: a view in canonical form.
pub(super) fn put_view(
    doc: &Document,
    view: &View,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let vp = ptr(p, "view");
    check_id(&view.id, &ptr(&vp, "id"))?;
    match find_collection(doc, &view.id) {
        Some(qp_schema::protocol::Collection::Views) | None => {}
        Some(c) => {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("id `{}` already exists (in {})", view.id, coll_name(c)),
            )
            .at(ptr(&vp, "id"))
            .build())
        }
    }
    let canonical = canon_view(doc, view, &vp, ctx.reads())?;
    if !doc.views.iter().any(|v| v.id == view.id) {
        ctx.note_created(&view.id);
    }
    Ok(vec![ChangeOp::PutView {
        view: canonical,
        index: None,
    }])
}

/// `add_layer`: a layer inserted at an index.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn add_layer(
    doc: &Document,
    view: &String,
    layer: &Layer,
    index: &Option<u32>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let v = need_view(doc, view, &ptr(p, "view"))?;
    if let Some(i) = index {
        if *i as usize > v.layers.len() {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("index {i} is past the end ({} layers)", v.layers.len()),
            )
            .at(ptr(p, "index"))
            .expected([format!("0..={}", v.layers.len())])
            .build());
        }
    }
    let at = index.map_or(v.layers.len(), |i| i as usize);
    let l = new_layer(doc, v, layer, at, &ptr(p, "layer"), ctx.reads())?;
    Ok(vec![ChangeOp::InsertLayer {
        view: view.clone(),
        index: u32::try_from(at).unwrap_or(u32::MAX),
        layer: l,
    }])
}

/// `remove_layer`: a layer removed by index.
pub(super) fn remove_layer(
    doc: &Document,
    view: &String,
    index: &u32,
    p: &str,
) -> PResult<Vec<ChangeOp>> {
    let v = need_view(doc, view, &ptr(p, "view"))?;
    if *index as usize >= v.layers.len() {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("view `{view}` has {} layers", v.layers.len()),
        )
        .at(ptr(p, "index"))
        .expected([format!("0..={}", v.layers.len().saturating_sub(1))])
        .build());
    }
    let l = &v.layers[*index as usize];
    if let Some(id) = &l.id {
        layer_in_use(v, id, &ptr(p, "index"))?;
    }
    Ok(vec![ChangeOp::DeleteLayer {
        layer: l.id.clone(),
        view: view.clone(),
        index: *index,
    }])
}

/// `set_view_range`: the view's x domain from two bars.
pub(super) fn set_view_range(
    doc: &Document,
    view: &String,
    from_bar: &u64,
    to_bar: &u64,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let v = need_view(doc, view, &ptr(p, "view"))?;
    let bars = view_bar_series_any(doc, v)
        .and_then(|s| qp_plane::plane::bars_for(doc, ctx.data.as_ref(), &s.id))
        .map(|(_, b)| b)
        .ok_or_else(|| invalid(format!("view `{view}` draws no bar series"), ptr(p, "view")))?;
    let last = bars.len() as u64 - 1;
    if *to_bar > last {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("to_bar {to_bar} is past the last bar {last}"),
        )
        .at(ptr(p, "to_bar"))
        .expected([format!("0..={last}")])
        .build());
    }
    if from_bar > to_bar {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("from_bar {from_bar} is after to_bar {to_bar}"),
        )
        .at(ptr(p, "from_bar"))
        .expected([format!("0..={to_bar}")])
        .build());
    }
    #[allow(clippy::cast_precision_loss)]
    let domain = [
        bars[*from_bar as usize].t as f64,
        bars[*to_bar as usize].t as f64,
    ];
    Ok(vec![ChangeOp::SetXDomain {
        view: view.clone(),
        domain: Some(domain),
    }])
}
