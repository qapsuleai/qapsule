//! Expansion of the grammar operations (R2, `docs/PROTOCOL.md` §5.1): layers, panels,
//! scales and emphasis addressed by id. Each op expands into position-carrying changes
//! computed against the current document (`docs/PROTOCOL.md` §5.3), so concurrent
//! requests by id never act on the wrong item.

use std::collections::BTreeSet;

use qp_schema::grammar::{Layer, ScaleKind, View};
use qp_schema::protocol::{ChangeOp, ErrorCode, Op, Target};
use qp_schema::Document;

use crate::canon::{
    check_emphasis, check_layer, check_new_layer_id, check_panel, check_value_scale, check_weight,
    layer_ids, limit_layers, limit_panels, need_layer, need_panel, next_layer_id, Site, CHANNELS,
};
use crate::error::{invalid, num, ptr, PResult, E};
use crate::expand::{need_view, Ctx};

fn u32_of(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

fn not_both<T>(after: Option<&T>, before: Option<&T>, p: &str) -> PResult<()> {
    if after.is_some() && before.is_some() {
        return Err(
            E::new(ErrorCode::Invalid, "give `after` or `before`, not both")
                .at(ptr(p, "before"))
                .build(),
        );
    }
    Ok(())
}

/// Index in `ids` for "after `after`" / "before `before`" / the end. `moving` is the
/// id of the item being moved (it cannot be its own anchor).
fn place<T: PartialEq + std::fmt::Display + Copy>(
    ids: &[T],
    after: Option<T>,
    before: Option<T>,
    moving: Option<T>,
    p: &str,
) -> PResult<usize> {
    let anchor = |a: T, key: &str| -> PResult<usize> {
        if moving == Some(a) {
            return Err(invalid(
                format!("`{a}` cannot be placed relative to itself"),
                ptr(p, key),
            ));
        }
        ids.iter()
            .position(|x| *x == a)
            .ok_or_else(|| invalid(format!("`{a}` is not in the list"), ptr(p, key)))
    };
    match (after, before) {
        (Some(a), _) => Ok(anchor(a, "after")? + 1),
        (_, Some(b)) => anchor(b, "before"),
        _ => Ok(ids.len()),
    }
}

fn in_use_by_emphasis(
    view: &View,
    hit: impl Fn(&Target) -> bool,
    what: &str,
    p: &str,
) -> PResult<()> {
    if view.emphasis.iter().any(|e| hit(&e.target)) {
        return Err(E::new(
            ErrorCode::InUse,
            format!("{what} is targeted by an emphasis of view `{}`", view.id),
        )
        .at(p)
        .expected([view.id.clone()])
        .hint("Change the view's emphasis with set_emphasis first.")
        .build());
    }
    Ok(())
}

/// Refuses deleting layer `id` while an emphasis of its view targets it.
pub(crate) fn layer_in_use(view: &View, id: &str, p: &str) -> PResult<()> {
    in_use_by_emphasis(
        view,
        |t| t.layer.as_deref() == Some(id),
        &format!("layer `{id}`"),
        p,
    )
}

/// Assigns ids to new layers (given ids checked against the view and each other).
fn assign_ids(view: &View, layers: &mut [Layer], base: &str) -> PResult<()> {
    let mut taken = layer_ids(view);
    for (k, l) in layers.iter().enumerate() {
        if let Some(id) = &l.id {
            let ip = ptr(&ptr(base, k), "id");
            check_new_layer_id(view, id, &ip)?;
            if !taken.insert(id.clone()) {
                return Err(E::new(
                    ErrorCode::Duplicate,
                    format!("layer id `{id}` is given twice"),
                )
                .at(ip)
                .build());
            }
        }
    }
    for l in layers.iter_mut() {
        if l.id.is_none() {
            let id = next_layer_id(&taken);
            taken.insert(id.clone());
            l.id = Some(id);
        }
    }
    Ok(())
}

/// A new layer at `index` of `view`, canonical and checked; returns the layer.
pub(crate) fn new_layer(
    doc: &Document,
    view: &View,
    layer: &Layer,
    index: usize,
    base: &str,
    reads: Option<crate::canon::Reads<'_>>,
) -> PResult<Layer> {
    limit_layers(view, 1, base)?;
    let mut l = layer.clone();
    match &l.id {
        Some(id) => check_new_layer_id(view, id, &ptr(base, "id"))?,
        None => l.id = Some(next_layer_id(&layer_ids(view))),
    }
    let mut nv = view.clone();
    nv.layers.insert(index, l.clone());
    check_layer(
        doc,
        &mut l,
        &Site {
            view: &nv,
            base,
            reads,
        },
    )?;
    Ok(l)
}

fn layer_id(l: &Layer) -> String {
    l.id.clone().unwrap_or_default()
}

/// Expands one grammar op.
#[allow(clippy::too_many_lines)]
pub(crate) fn expand_grammar(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    match op {
        Op::InsertLayer {
            view,
            layer,
            after,
            before,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            not_both(after.as_ref(), before.as_ref(), p)?;
            if let Some(a) = after {
                need_layer(v, a, &ptr(p, "after"))?;
            }
            if let Some(b) = before {
                need_layer(v, b, &ptr(p, "before"))?;
            }
            let ids: Vec<&str> = v.layers.iter().filter_map(|l| l.id.as_deref()).collect();
            let index = place(&ids, after.as_deref(), before.as_deref(), None, p)?;
            let l = new_layer(doc, v, layer, index, &ptr(p, "layer"), ctx.reads())?;
            ctx.created.push(layer_id(&l));
            Ok(vec![ChangeOp::InsertLayer {
                view: view.clone(),
                index: u32_of(index),
                layer: l,
            }])
        }
        Op::DeleteLayer { view, layer } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let i = need_layer(v, layer, &ptr(p, "layer"))?;
            layer_in_use(v, layer, &ptr(p, "layer"))?;
            Ok(vec![ChangeOp::DeleteLayer {
                view: view.clone(),
                index: u32_of(i),
                layer: Some(layer.clone()),
            }])
        }
        Op::SplitLayer { view, layer } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let lp = ptr(p, "layer");
            let i = need_layer(v, layer, &lp)?;
            let old = &v.layers[i];
            let outputs = match crate::canon::layer_data(doc, old) {
                crate::canon::Data::Series(s)
                    if old.mark == qp_schema::Mark::Line && old.encoding.y.is_none() =>
                {
                    crate::summary::drawing_outputs(s)
                }
                _ => Vec::new(),
            };
            if outputs.len() < 2 {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!(
                        "layer `{layer}` draws one output; only a `line` layer without `y` on a multi-output study (bollinger, macd) can be split"
                    ),
                )
                .at(lp)
                .hint("Style the layer directly with set_encoding.")
                .build());
            }
            layer_in_use(v, layer, &lp)?;
            limit_layers(v, outputs.len() - 1, p)?;
            // New ids: the smallest unused `l<k>`, the split layer's id still counted
            // as taken (it is never reused by its own parts).
            let mut taken = layer_ids(v);
            let mut nv = v.clone();
            nv.layers.remove(i);
            let mut out = vec![ChangeOp::DeleteLayer {
                view: view.clone(),
                index: u32_of(i),
                layer: Some(layer.clone()),
            }];
            for (k, o) in outputs.iter().enumerate() {
                let id = next_layer_id(&taken);
                taken.insert(id.clone());
                let mut l = old.clone();
                l.id = Some(id.clone());
                l.encoding.y = Some(qp_schema::grammar::Position::Name(o.clone()));
                let index = i + k;
                let l = new_layer(doc, &nv, &l, index, &lp, ctx.reads())?;
                nv.layers.insert(index, l.clone());
                ctx.created.push(id);
                out.push(ChangeOp::InsertLayer {
                    view: view.clone(),
                    index: u32_of(index),
                    layer: l,
                });
            }
            Ok(out)
        }
        Op::MoveLayer {
            view,
            layer,
            panel,
            after,
            before,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let i = need_layer(v, layer, &ptr(p, "layer"))?;
            if panel.is_none() && after.is_none() && before.is_none() {
                return Err(E::new(ErrorCode::Invalid, "nothing to move")
                    .at(p)
                    .expected(["panel", "after", "before"])
                    .build());
            }
            not_both(after.as_ref(), before.as_ref(), p)?;
            if let Some(a) = after {
                need_layer(v, a, &ptr(p, "after"))?;
            }
            if let Some(b) = before {
                need_layer(v, b, &ptr(p, "before"))?;
            }
            let mut l = v.layers[i].clone();
            if let Some(pid) = panel {
                need_panel(v, *pid, &ptr(p, "panel"))?;
                l.panel = *pid;
            }
            let mut rest = v.layers.clone();
            rest.remove(i);
            let index = if after.is_some() || before.is_some() {
                let ids: Vec<&str> = rest.iter().filter_map(|x| x.id.as_deref()).collect();
                place(
                    &ids,
                    after.as_deref(),
                    before.as_deref(),
                    Some(layer.as_str()),
                    p,
                )?
            } else {
                i
            };
            let mut nv = v.clone();
            nv.layers = rest;
            nv.layers.insert(index, l.clone());
            check_layer(
                doc,
                &mut l,
                &Site {
                    view: &nv,
                    base: p,
                    reads: ctx.reads(),
                },
            )?;
            Ok(vec![ChangeOp::PutLayer {
                view: view.clone(),
                index: u32_of(index),
                layer: l,
            }])
        }
        Op::SetEncoding {
            view,
            layer,
            encoding,
            unset,
            origin,
            label,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let i = need_layer(v, layer, &ptr(p, "layer"))?;
            if encoding.is_empty() && unset.is_empty() && origin.is_empty() && label.is_none() {
                return Err(E::new(ErrorCode::Invalid, "nothing to change")
                    .at(p)
                    .expected(["encoding", "unset", "origin", "label"])
                    .build());
            }
            let e = encoding;
            let set: Vec<&str> = [
                ("x", e.x.is_some()),
                ("y", e.y.is_some()),
                ("y2", e.y2.is_some()),
                ("color", e.color.is_some()),
                ("opacity", e.opacity.is_some()),
                ("width", e.width.is_some()),
                ("size", e.size.is_some()),
                ("text", e.text.is_some()),
            ]
            .into_iter()
            .filter(|(_, s)| *s)
            .map(|(n, _)| n)
            .collect();
            let up = ptr(p, "unset");
            for (k, name) in unset.iter().enumerate() {
                if !CHANNELS.contains(&name.as_str()) {
                    return Err(
                        E::new(ErrorCode::Invalid, format!("unknown channel `{name}`"))
                            .at(ptr(&up, k))
                            .dym(name, CHANNELS)
                            .expected(CHANNELS.map(str::to_owned))
                            .build(),
                    );
                }
                if set.contains(&name.as_str()) {
                    return Err(invalid(
                        format!("channel `{name}` is both set and unset"),
                        ptr(&up, k),
                    ));
                }
            }
            let mut l = v.layers[i].clone();
            let changed: BTreeSet<&str> = set
                .iter()
                .copied()
                .chain(unset.iter().map(String::as_str))
                .collect();
            l.origin.retain(|k, _| {
                !changed.iter().any(|c| {
                    let base = format!("/encoding/{c}");
                    *k == base || k.starts_with(&format!("{base}/"))
                })
            });
            if e.x.is_some() {
                l.encoding.x.clone_from(&e.x);
            }
            if e.y.is_some() {
                l.encoding.y.clone_from(&e.y);
            }
            if e.y2.is_some() {
                l.encoding.y2.clone_from(&e.y2);
            }
            if e.color.is_some() {
                l.encoding.color.clone_from(&e.color);
            }
            if e.opacity.is_some() {
                l.encoding.opacity = e.opacity;
            }
            if e.width.is_some() {
                l.encoding.width = e.width;
            }
            if e.size.is_some() {
                l.encoding.size.clone_from(&e.size);
            }
            if e.text.is_some() {
                l.encoding.text.clone_from(&e.text);
            }
            for name in unset {
                match name.as_str() {
                    "x" => l.encoding.x = None,
                    "y" => l.encoding.y = None,
                    "y2" => l.encoding.y2 = None,
                    "color" => l.encoding.color = None,
                    "opacity" => l.encoding.opacity = None,
                    "width" => l.encoding.width = None,
                    "size" => l.encoding.size = None,
                    _ => l.encoding.text = None,
                }
            }
            for (k, o) in origin {
                l.origin.insert(k.clone(), o.clone());
            }
            if let Some(t) = label {
                l.label = if t.is_empty() { None } else { Some(t.clone()) };
            }
            let mut nv = v.clone();
            nv.layers[i] = l.clone();
            check_layer(
                doc,
                &mut l,
                &Site {
                    view: &nv,
                    base: p,
                    reads: ctx.reads(),
                },
            )?;
            Ok(vec![ChangeOp::PutLayer {
                view: view.clone(),
                index: u32_of(i),
                layer: l,
            }])
        }
        Op::InsertPanel {
            view,
            panel,
            layers,
            after,
            before,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            not_both(after.as_ref(), before.as_ref(), p)?;
            let pp = ptr(p, "panel");
            limit_panels(v, 1, &pp)?;
            limit_layers(v, layers.len(), &ptr(p, "layers"))?;
            let mut pn = panel.clone();
            let id = match pn.id {
                Some(id) => {
                    if v.panel(id).is_some() {
                        return Err(E::new(
                            ErrorCode::Duplicate,
                            format!("panel {id} already exists in view `{view}`"),
                        )
                        .at(ptr(&pp, "id"))
                        .hint("Choose another id or omit it to have one assigned.")
                        .build());
                    }
                    id
                }
                None => match v.panels.iter().filter_map(|x| x.id).max() {
                    None => 0,
                    Some(m) => m.checked_add(1).ok_or_else(|| {
                        invalid("no panel id is left above the largest one", ptr(&pp, "id"))
                    })?,
                },
            };
            pn.id = Some(id);
            check_panel(&mut pn, &pp)?;
            if let Some(a) = after {
                need_panel(v, *a, &ptr(p, "after"))?;
            }
            if let Some(b) = before {
                need_panel(v, *b, &ptr(p, "before"))?;
            }
            let pids: Vec<u32> = v.panels.iter().filter_map(|x| x.id).collect();
            let index = place(&pids, *after, *before, None, p)?;
            let mut nv = v.clone();
            nv.panels.insert(index, pn.clone());
            let lbase = ptr(p, "layers");
            let mut new: Vec<Layer> = layers
                .iter()
                .map(|l| {
                    let mut l = l.clone();
                    l.panel = id;
                    l
                })
                .collect();
            assign_ids(v, &mut new, &lbase)?;
            let first = nv.layers.len();
            nv.layers.extend(new.iter().cloned());
            for (k, l) in new.iter_mut().enumerate() {
                check_layer(
                    doc,
                    l,
                    &Site {
                        view: &nv,
                        base: &ptr(&lbase, k),
                        reads: ctx.reads(),
                    },
                )?;
            }
            ctx.created.push(id.to_string());
            let mut out = vec![ChangeOp::PutPanel {
                view: view.clone(),
                index: u32_of(index),
                panel: pn,
            }];
            for (k, l) in new.into_iter().enumerate() {
                ctx.created.push(layer_id(&l));
                out.push(ChangeOp::InsertLayer {
                    view: view.clone(),
                    index: u32_of(first + k),
                    layer: l,
                });
            }
            Ok(out)
        }
        Op::DeletePanel { view, panel } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let pp = ptr(p, "panel");
            need_panel(v, *panel, &pp)?;
            if v.panels.len() == 1 {
                return Err(
                    E::new(ErrorCode::Invalid, "a view keeps at least one panel")
                        .at(pp)
                        .hint("Delete the view with remove, or insert another panel first.")
                        .build(),
                );
            }
            let ids: BTreeSet<&str> = v
                .layers
                .iter()
                .filter(|l| l.panel == *panel)
                .filter_map(|l| l.id.as_deref())
                .collect();
            in_use_by_emphasis(
                v,
                |t| t.panel == Some(*panel) || t.layer.as_deref().is_some_and(|l| ids.contains(l)),
                &format!("panel {panel} (or one of its layers)"),
                &pp,
            )?;
            let mut out: Vec<ChangeOp> = v
                .layers
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, l)| l.panel == *panel)
                .map(|(i, l)| ChangeOp::DeleteLayer {
                    view: view.clone(),
                    index: u32_of(i),
                    layer: l.id.clone(),
                })
                .collect();
            out.push(ChangeOp::DeletePanel {
                view: view.clone(),
                panel: *panel,
            });
            Ok(out)
        }
        Op::MovePanel {
            view,
            panel,
            after,
            before,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let i = need_panel(v, *panel, &ptr(p, "panel"))?;
            not_both(after.as_ref(), before.as_ref(), p)?;
            if let Some(a) = after {
                need_panel(v, *a, &ptr(p, "after"))?;
            }
            if let Some(b) = before {
                need_panel(v, *b, &ptr(p, "before"))?;
            }
            let mut rest = v.panels.clone();
            let pn = rest.remove(i);
            let pids: Vec<u32> = rest.iter().filter_map(|x| x.id).collect();
            let index = place(&pids, *after, *before, Some(*panel), p)?;
            Ok(vec![ChangeOp::PutPanel {
                view: view.clone(),
                index: u32_of(index),
                panel: pn,
            }])
        }
        Op::SetPanel {
            view,
            panel,
            name,
            weight,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let i = need_panel(v, *panel, &ptr(p, "panel"))?;
            if name.is_none() && weight.is_none() {
                return Err(E::new(ErrorCode::Invalid, "nothing to change")
                    .at(p)
                    .expected(["name", "weight"])
                    .build());
            }
            let mut pn = v.panels[i].clone();
            if let Some(n) = name {
                pn.name = if n.is_empty() { None } else { Some(n.clone()) };
            }
            if let Some(w) = weight {
                check_weight(*w, &ptr(p, "weight"))?;
                pn.weight = Some(*w);
            }
            Ok(vec![ChangeOp::PutPanel {
                view: view.clone(),
                index: u32_of(i),
                panel: pn,
            }])
        }
        Op::SetScale {
            view,
            scale,
            kind,
            domain,
            auto_domain,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            if domain.is_some() && *auto_domain {
                return Err(invalid(
                    "give `domain` or `auto_domain`, not both",
                    ptr(p, "auto_domain"),
                ));
            }
            if kind.is_none() && domain.is_none() && !*auto_domain {
                return Err(E::new(ErrorCode::Invalid, "nothing to change")
                    .at(p)
                    .expected(["kind", "domain", "auto_domain"])
                    .build());
            }
            let scale_ids: Vec<String> = std::iter::once("x".to_owned())
                .chain(
                    v.panels
                        .iter()
                        .filter_map(|x| x.id)
                        .map(|i| format!("y{i}")),
                )
                .collect();
            if scale == "x" {
                if let Some(k) = kind {
                    if *k != ScaleKind::Time {
                        return Err(E::new(ErrorCode::Invalid, "the x scale is always `time`")
                            .at(ptr(p, "kind"))
                            .expected(["time"])
                            .build());
                    }
                }
                if let Some(d) = domain {
                    let ok = d[0].is_finite() && d[1].is_finite() && d[0] <= d[1];
                    if !ok {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            format!(
                                "domain [{}, {}] must be finite with min <= max",
                                num(d[0]),
                                num(d[1])
                            ),
                        )
                        .at(ptr(p, "domain"))
                        .expected(["[t0, t1] epoch ms with t0 <= t1"])
                        .build());
                    }
                    return Ok(vec![ChangeOp::SetXDomain {
                        view: view.clone(),
                        domain: Some(*d),
                    }]);
                }
                if *auto_domain {
                    return Ok(vec![ChangeOp::SetXDomain {
                        view: view.clone(),
                        domain: None,
                    }]);
                }
                return Err(E::new(ErrorCode::Invalid, "the x scale is already `time`")
                    .at(ptr(p, "kind"))
                    .hint("Give a `domain` (epoch ms) or `auto_domain`.")
                    .build());
            }
            let pid = scale
                .strip_prefix('y')
                .filter(|s| !s.starts_with('+'))
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|i| v.panel(*i).is_some() && scale == &format!("y{i}"));
            let Some(pid) = pid else {
                return Err(E::new(
                    ErrorCode::NotFound,
                    format!("scale `{scale}` does not exist in view `{view}`"),
                )
                .at(ptr(p, "scale"))
                .dym(scale, scale_ids.iter().map(String::as_str))
                .expected(scale_ids.clone())
                .hint("Scale ids are `x` and `y<panel id>`.")
                .build());
            };
            let i = need_panel(v, pid, &ptr(p, "scale"))?;
            let mut pn = v.panels[i].clone();
            if let Some(k) = kind {
                pn.y.kind = *k;
            }
            if let Some(d) = domain {
                pn.y.domain = Some(*d);
            }
            if *auto_domain {
                pn.y.domain = None;
            }
            check_value_scale(&mut pn.y, pid, p)?;
            Ok(vec![ChangeOp::PutPanel {
                view: view.clone(),
                index: u32_of(i),
                panel: pn,
            }])
        }
        Op::SetEmphasis { view, emphasis } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            check_emphasis(doc, v, emphasis, &ptr(p, "emphasis"))?;
            Ok(vec![ChangeOp::SetEmphasis {
                view: view.clone(),
                emphasis: emphasis.clone(),
            }])
        }
        _ => Err(invalid("not a grammar operation", p)),
    }
}
