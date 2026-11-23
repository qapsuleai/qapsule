//! One-line summaries of operations for log entries.

use super::*;

#[allow(clippy::too_many_lines)]
/// Short deterministic summary of one op.
pub fn summarize(op: &Op) -> String {
    fn params(p: &BTreeMap<String, f64>) -> String {
        p.iter()
            .map(|(k, v)| format!("{k}={}", num(*v)))
            .collect::<Vec<_>>()
            .join(",")
    }
    match op {
        Op::PutSeries { series } => format!("put_series {}", series.id),
        Op::AddStudy {
            id,
            study,
            params: p,
            ..
        } => {
            let base = format!("add_study {study}({})", params(p));
            match id {
                Some(i) => format!("{base} -> {i}"),
                None => base,
            }
        }
        Op::SetParams { series, params: p } => format!("set_params {series}({})", params(p)),
        Op::PutView { view } => format!("put_view {}", view.id),
        Op::AddLayer { view, .. } => format!("add_layer {view}"),
        Op::RemoveLayer { view, index } => format!("remove_layer {view}[{index}]"),
        Op::SetViewRange {
            view,
            from_bar,
            to_bar,
        } => format!("set_view_range {view} {from_bar}..={to_bar}"),
        Op::AddFact { series, bar, .. } => format!("add_fact {series}@{bar}"),
        Op::AddFinding { .. } => "add_finding".to_owned(),
        Op::UpdateFinding { finding, .. } => format!("update_finding {finding}"),
        Op::ApproveFinding { finding, .. } => format!("approve_finding {finding}"),
        Op::RejectFinding { finding, .. } => format!("reject_finding {finding}"),
        Op::AddIntent { .. } => "add_intent".to_owned(),
        Op::SetIntentActive { intent, active } => format!("set_intent_active {intent}={active}"),
        Op::AddNarrative { about, .. } => format!("add_narrative about {about}"),
        Op::AddWatch { .. } => "add_watch".to_owned(),
        Op::RemoveWatch { intent } => format!("remove_watch {intent}"),
        Op::AddComment { about, .. } => format!(
            "add_comment about {}",
            about
                .capsule
                .as_deref()
                .or(about.instrument.as_deref())
                .unwrap_or_default()
        ),
        Op::ShowFinding { finding, .. } => format!("show_finding {finding}"),
        Op::Remove { id } => format!("remove {id}"),
        Op::AppendBars { series, bars } => format!("append_bars {series} +{}", bars.len()),
        Op::UpdateLastBar { series, .. } => format!("update_last_bar {series}"),
        Op::Undo { seq } => match seq {
            Some(s) => format!("undo {s}"),
            None => "undo".to_owned(),
        },
        Op::AcceptProposal { proposal, .. } => format!("accept_proposal {proposal}"),
        Op::RejectProposal { proposal, .. } => format!("reject_proposal {proposal}"),
        Op::InsertLayer { view, layer, .. } => match &layer.id {
            Some(id) => format!("insert_layer {view} {id}"),
            None => format!("insert_layer {view}"),
        },
        Op::DeleteLayer { view, layer } => format!("delete_layer {view} {layer}"),
        Op::SplitLayer { view, layer } => format!("split_layer {view} {layer}"),
        Op::MoveLayer { view, layer, .. } => format!("move_layer {view} {layer}"),
        Op::SetEncoding { view, layer, .. } => format!("set_encoding {view} {layer}"),
        Op::InsertPanel { view, .. } => format!("insert_panel {view}"),
        Op::DeletePanel { view, panel } => format!("delete_panel {view} {panel}"),
        Op::MovePanel { view, panel, .. } => format!("move_panel {view} {panel}"),
        Op::SetPanel { view, panel, .. } => format!("set_panel {view} {panel}"),
        Op::SetScale { view, scale, .. } => format!("set_scale {view} {scale}"),
        Op::SetEmphasis { view, emphasis } => {
            format!("set_emphasis {view} ({})", emphasis.len())
        }
        Op::SetAudience { principal, .. } => match principal {
            Some(w) => format!("set_audience {w}"),
            None => "set_audience".to_owned(),
        },
        Op::SetSensitivity {
            principal,
            sensitivity,
        } => {
            let level = sensitivity.map_or("default", |s| match s {
                qp_schema::capsule::Sensitivity::Important => "important",
                qp_schema::capsule::Sensitivity::Balanced => "balanced",
                qp_schema::capsule::Sensitivity::All => "all",
            });
            match principal {
                Some(w) => format!("set_sensitivity {w} {level}"),
                None => format!("set_sensitivity {level}"),
            }
        }
    }
}

/// Summary of a whole request, with created ids.
pub fn summarize_ops(ops: &[Op], created: &[String]) -> String {
    let mut s = ops.iter().map(summarize).collect::<Vec<_>>().join("; ");
    if !created.is_empty() {
        s.push_str(" -> ");
        s.push_str(&created.join(", "));
    }
    s
}
