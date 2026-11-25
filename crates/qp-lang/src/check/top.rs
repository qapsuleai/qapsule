//! The root of a check: a refusal or a program; the sections in order; size limits;
//! identities; and, with no error, the resolved program.

use std::collections::BTreeMap;

use qp_schema::program::{Program, ProgramCost, ProgramReject, PROGRAM_VERSION};
use serde_json::Value;

use super::clock::parse_tf;
use super::cost::static_cost;
use super::fields::Fields;
use super::outputs::{outputs, Outputs};
use super::params::{bind, definitions};
use super::warn::warn;
use super::{nodes, Base, CheckCtx, CheckReport, Cx, Typed};
use crate::checked::{Checked, CheckedNode, CheckedOutput, Op, Slot};
use crate::diag::Diags;
use crate::identity::{instance_id, program_id};
use crate::limits::Limit;
use crate::message::{Msg, Space};
use crate::reason::Reason;

/// Runs a whole check.
pub(super) fn run(raw: &Value, ctx: &CheckCtx<'_>) -> CheckReport {
    let base = match &ctx.binding {
        None => Base::Unbound,
        Some(b) => Base::Bound(b.interval.as_deref().and_then(|s| parse_tf(s).ok())),
    };
    let mut cx = Cx {
        d: Diags::new(ctx.lang),
        limits: ctx.limits,
        base,
        params: BTreeMap::new(),
        ids: BTreeMap::new(),
        nodes: Vec::new(),
        ctx,
    };
    let mut report = CheckReport {
        errors: Vec::new(),
        truncated: false,
        warnings: Vec::new(),
        cost: None,
        outputs: Vec::new(),
        params: Vec::new(),
        program_id: None,
        instance_id: None,
        reject: None,
        checked: None,
        limits: ctx.limits,
    };
    if let Some(f) = Fields::new(raw, "", "program", &mut cx.d) {
        if f.has("reject") {
            reject(&mut cx, f, &mut report);
        } else {
            program(&mut cx, f, raw, &mut report);
        }
    }
    let max = usize::try_from(cx.limit(Limit::CheckMaxErrors)).unwrap_or(usize::MAX);
    report.truncated = cx.d.finish(max);
    report.errors = std::mem::take(&mut cx.d.errors);
    report.warnings = std::mem::take(&mut cx.d.warnings);
    if !report.errors.is_empty() {
        report.checked = None;
        report.instance_id = None;
    }
    report
}

/// A refusal (PR §12.1): `{"reject": {"code", "reason"}}`, the code a PR §11.2 sub-code.
fn reject(cx: &mut Cx<'_>, mut f: Fields<'_>, report: &mut CheckReport) {
    let v = f.opt("reject");
    f.finish(&mut cx.d, &[]);
    let Some(mut r) = v.and_then(|v| Fields::new(v, "/reject", "reject", &mut cx.d)) else {
        return;
    };
    let code = r.req_str("code", &mut cx.d);
    if let Some(c) = code {
        if Reason::parse(c).is_none_or(Reason::is_warning) {
            let codes: Vec<&str> = Reason::ALL
                .iter()
                .filter(|r| !r.is_warning())
                .map(|r| r.as_str())
                .collect();
            let dym = crate::names::did_you_mean(c, codes.iter().copied());
            cx.d.at(
                Reason::BadParam,
                &r.ptr("code"),
                &Msg::RejectCode(c.to_owned()),
            )
            .expected(codes)
            .dym(dym);
        }
    }
    let reason = r.req_str("reason", &mut cx.d);
    r.finish(&mut cx.d, &[]);
    if let (Some(code), Some(reason), 0) = (code, reason, cx.d.count()) {
        report.reject = Some(ProgramReject {
            code: code.to_owned(),
            reason: reason.to_owned(),
        });
    }
}

fn program(cx: &mut Cx<'_>, mut f: Fields<'_>, raw: &Value, report: &mut CheckReport) {
    match f.req_str("version", &mut cx.d) {
        Some(v) if v != PROGRAM_VERSION => {
            cx.d.at(Reason::Version, "/version", &Msg::Version(v.to_owned()))
                .expected([PROGRAM_VERSION]);
        }
        _ => {}
    }
    let praw = f.opt("params");
    report.params = definitions(cx, praw);
    if let Some(Value::Array(p)) = praw {
        count_limit(
            cx,
            "/params",
            p.len(),
            Limit::MaxParams,
            Reason::TooLarge,
            Space::Param,
        );
    }
    bind(cx);
    let nraw = f.req("nodes", &mut cx.d);
    let ids = node_section(cx, nraw);
    let outs = outputs(cx, f.opt("outputs"));
    f.finish(&mut cx.d, &[]);
    report.outputs.clone_from(&outs.infos);
    resolve(cx, raw, &ids, &outs, report);
}

fn count_limit(cx: &mut Cx<'_>, at: &str, n: usize, l: Limit, r: Reason, s: Space) {
    let max = cx.limit(l);
    if n as u64 > max {
        cx.d.at(r, at, &Msg::TooMany(s, n as u64, max))
            .expected([max]);
    }
}

/// Checks every node; returns the ids by index (empty string where unreadable).
fn node_section(cx: &mut Cx<'_>, raw: Option<&Value>) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let Value::Array(items) = raw else {
        cx.d.at(Reason::Type, "/nodes", &Msg::NotArray("nodes".to_owned()));
        return Vec::new();
    };
    count_limit(
        cx,
        "/nodes",
        items.len(),
        Limit::MaxNodes,
        Reason::TooManyNodes,
        Space::Node,
    );
    let ids: Vec<String> = items
        .iter()
        .map(|n| n.get("id").and_then(Value::as_str).unwrap_or("").to_owned())
        .collect();
    for (i, id) in ids.iter().enumerate() {
        if !id.is_empty() {
            cx.ids.entry(id.clone()).or_insert(i);
        }
    }
    for (i, v) in items.iter().enumerate() {
        let entry = nodes::node(cx, v, i);
        cx.nodes.push(entry);
    }
    ids
}

/// Cost, warnings, size, identities and the resolved program.
fn resolve(cx: &mut Cx<'_>, raw: &Value, ids: &[String], outs: &Outputs, report: &mut CheckReport) {
    let typed: Option<Vec<Typed>> = cx.nodes.iter().map(|n| n.typed.clone()).collect();
    let resolved: Option<Vec<&CheckedOutput>> = outs.specs.iter().map(Option::as_ref).collect();
    if let (Some(typed), Some(resolved)) = (typed.as_ref(), resolved.as_ref()) {
        report.cost = Some(cost_and_warnings(cx, typed, resolved, ids));
    }
    let program = serde_json::from_value::<Program>(raw.clone()).ok();
    let size = match &program {
        Some(p) => p.canonical_json().map_or(0, |s| s.len()),
        None => serde_json::to_string(raw).map_or(0, |s| s.len()),
    };
    count_limit(
        cx,
        "",
        size,
        Limit::MaxIrBytes,
        Reason::TooLarge,
        Space::Bytes,
    );
    let Some(program) = program.filter(|p| p.version == PROGRAM_VERSION) else {
        return;
    };
    report.program_id = program_id(&program).ok();
    if cx.d.count() > 0 {
        return;
    }
    if let (Some(pid), Some(typed), Some(cost)) =
        (report.program_id.clone(), typed, report.cost.clone())
    {
        report.checked = build(cx, program, pid, typed, ids, outs, cost);
        report.instance_id = report.checked.as_ref().map(|c| c.instance_id.clone());
    }
}

/// Static cost (with `E_COST_LIMIT`) and warnings of a program whose nodes all typed.
fn cost_and_warnings(
    cx: &mut Cx<'_>,
    typed: &[Typed],
    resolved: &[&CheckedOutput],
    ids: &[String],
) -> ProgramCost {
    let weights: Vec<u64> = typed.iter().map(|t| t.w).collect();
    let inputs: Vec<Vec<Slot>> = typed.iter().map(|t| t.op.inputs()).collect();
    let roots: Vec<Vec<Slot>> = resolved.iter().map(|o| o.spec.inputs()).collect();
    let bars = cx.ctx.binding.as_ref().and_then(|b| b.bars);
    let cost = static_cost(&weights, &inputs, &roots, bars, &cx.limits);
    if cost.per_bar > cost.limit {
        cx.d.at(
            Reason::CostLimit,
            "",
            &Msg::CostLimit(cost.per_bar, cost.limit),
        )
        .expected([cost.limit])
        .hint(&Msg::HintCost);
    }
    let ops: Vec<&Op> = typed.iter().map(|t| &t.op).collect();
    let indexed: Vec<(usize, &CheckedOutput)> = resolved.iter().copied().enumerate().collect();
    warn(cx, &ops, ids, &indexed);
    cost
}

/// The resolved program of an error-free check.
fn build(
    cx: &Cx<'_>,
    program: Program,
    pid: String,
    typed: Vec<Typed>,
    ids: &[String],
    outs: &Outputs,
    cost: ProgramCost,
) -> Option<Checked> {
    let params: BTreeMap<_, _> = cx
        .params
        .iter()
        .filter_map(|(k, p)| p.value.map(|v| (k.clone(), v)))
        .collect();
    let iid = instance_id(&pid, &params).ok()?;
    let nodes = typed
        .into_iter()
        .zip(ids)
        .map(|(t, id)| CheckedNode {
            id: id.clone(),
            op: t.op,
            outputs: t.outputs,
            clock: t.clock,
            w: t.w,
        })
        .collect();
    let base = match &cx.base {
        Base::Bound(b) => b.clone(),
        Base::Unbound => None,
    };
    Some(Checked {
        program,
        program_id: pid,
        instance_id: iid,
        params,
        nodes,
        outputs: outs.specs.iter().flatten().cloned().collect(),
        cost,
        base,
    })
}
