//! `add_program` `show` (PS §1.4): one layer per output — `series` → `line`, `event` →
//! `bar` (0/1 columns) — in the panel of `with`, in panel `panel`, else in a new panel
//! below the others. The layers are made by the built-in grammar ops (`insert_panel`,
//! `insert_layer`), so their checks and ids are the ones every layer gets. Each layer's
//! legend label is its series id (`<instance>.<output>`).

use qp_ops::expand::{expand_op, need_view, Ctx};
use qp_plane::error::{invalid, ptr, PResult, E};
use qp_schema::grammar::{Layer, Mark, Panel, Scale, ScaleKind};
use qp_schema::program::OutputKind;
use qp_schema::protocol::{ChangeOp, ErrorCode, Op, Show};
use qp_schema::{Document, Series};

use crate::inst::{output_name, program_of};

/// The mark of program series `s`: `bar` for an event output, else `line`.
fn mark_of(s: &Series) -> Mark {
    let event = program_of(s).is_some_and(|p| {
        p.ir.outputs.iter().any(|o| {
            Some(o.name.as_str()) == output_name(s) && matches!(o.kind, OutputKind::Event { .. })
        })
    });
    if event {
        Mark::Bar
    } else {
        Mark::Line
    }
}

/// Where the layers go: an existing panel, or a new one (its id).
#[allow(clippy::result_large_err)]
fn target(doc: &Document, sh: &Show, p: &str) -> PResult<(u32, bool)> {
    let v = need_view(doc, &sh.view, &ptr(p, "view"))?;
    let next = v
        .panels
        .iter()
        .filter_map(|x| x.id)
        .max()
        .map_or(Some(0), |m| m.checked_add(1));
    if let Some(w) = &sh.with {
        return v
            .layers
            .iter()
            .find(|l| l.series.as_deref() == Some(w.as_str()))
            .map(|l| (l.panel, false))
            .ok_or_else(|| {
                E::new(
                    ErrorCode::Invalid,
                    format!("series `{w}` is not drawn in view `{}`", sh.view),
                )
                .at(ptr(p, "with"))
                .build()
            });
    }
    match sh.panel {
        Some(id) if v.panel(id).is_some() => Ok((id, false)),
        Some(id) if Some(id) != next => Err(E::new(
            ErrorCode::NotFound,
            format!("no panel {id} in view `{}`", sh.view),
        )
        .at(ptr(p, "panel"))
        .build()),
        _ => next
            .map(|n| (n, true))
            .ok_or_else(|| invalid("no panel id is left above the largest one", ptr(p, "panel"))),
    }
}

/// The layer changes showing the series `put` writes, made on `doc` with them applied.
#[allow(clippy::result_large_err)]
pub(crate) fn layers(
    doc: &Document,
    put: &[ChangeOp],
    sh: &Show,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let (panel, new) = target(doc, sh, p)?;
    let mut scratch = doc.clone();
    qp_ops::apply_all(&mut scratch, put).map_err(|m| invalid(m, p))?;
    let layers: Vec<Layer> = put
        .iter()
        .filter_map(|c| match c {
            ChangeOp::PutSeries { series, .. } => Some(series),
            _ => None,
        })
        .map(|s| {
            let mut l = Layer::new(mark_of(s), panel);
            l.series = Some(s.id.clone());
            l.label = Some(s.id.clone());
            l
        })
        .collect();
    // A new panel by `insert_panel` (its checks: the panel limit); its id is not a
    // created item of `add_program` (`ack.created`: series ids, then layer ids).
    let mut out = Vec::new();
    if new {
        let pn = Panel {
            id: Some(panel),
            name: None,
            weight: None,
            y: Scale {
                id: Some(format!("y{panel}")),
                ..Scale::new(ScaleKind::Linear)
            },
        };
        let op = Op::InsertPanel {
            view: sh.view.clone(),
            panel: pn,
            layers: Vec::new(),
            after: None,
            before: None,
        };
        let created = ctx.created.len();
        let cs = expand_op(&scratch, &op, p, ctx)?;
        ctx.created.truncate(created);
        qp_ops::apply_all(&mut scratch, &cs).map_err(|m| invalid(m, p))?;
        out.extend(cs);
    }
    let ops: Vec<Op> = layers
        .into_iter()
        .map(|layer| Op::InsertLayer {
            view: sh.view.clone(),
            layer,
            after: None,
            before: None,
        })
        .collect();
    for op in &ops {
        let cs = expand_op(&scratch, op, p, ctx)?;
        qp_ops::apply_all(&mut scratch, &cs).map_err(|m| invalid(m, p))?;
        out.extend(cs);
    }
    Ok(out)
}
