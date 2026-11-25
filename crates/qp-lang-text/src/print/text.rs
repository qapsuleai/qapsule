//! Program text: parameter lines, node lines, output lines in canonical order.

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::program::{
    Num, OutcomeRule, Output, OutputKind, Program, ProgramDoc, ProgramParamDef, ProgramParamKind,
    ProgramReject,
};

use super::check::{check_program, check_reject, PrintError};
use super::expr::Printer;
use super::lit::{self, fmt_num, quote};
use crate::limits::TextLimits;
use crate::names::text_name;

/// The canonical text of a program.
///
/// # Errors
/// When a name the text cannot spell safely sits in the IR (see [`PrintError`]).
pub fn print(p: &Program) -> Result<String, PrintError> {
    print_with(p, &TextLimits::default())
}

/// [`print`] with explicit limits (`max_depth` bounds inlining).
///
/// # Errors
/// As [`print`].
pub fn print_with(p: &Program, limits: &TextLimits) -> Result<String, PrintError> {
    check_program(p)?;
    let pr = Printer::new(p, limits.max_depth);
    let mut lines: Vec<String> = p.params.iter().map(param_line).collect();
    let mut stmts: Vec<(usize, u8, String)> = Vec::new();
    for (i, n) in p.nodes.iter().enumerate() {
        if !pr.inlined(i) {
            stmts.push((i, 0, format!("{} = {}", pr.name(i), pr.op(&n.op))));
        }
    }
    let mut last = 0usize;
    for o in &p.outputs {
        let key = pr.output_pos(o).max(last);
        last = key;
        stmts.push((key, 1, output_line(&pr, o)));
    }
    stmts.sort_by_key(|(k, t, _)| (*k, *t));
    lines.extend(stmts.into_iter().map(|(_, _, s)| s));
    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out)
}

/// The canonical text of a program or a refusal.
///
/// # Errors
/// As [`print`].
pub fn print_doc(doc: &ProgramDoc) -> Result<String, PrintError> {
    match doc {
        ProgramDoc::Program(p) => print(p),
        ProgramDoc::Reject(r) => print_reject(&r.reject),
    }
}

/// The canonical text of a refusal: `reject <CODE> "<reason>"`.
///
/// # Errors
/// When the code is not a plain identifier.
pub fn print_reject(r: &ProgramReject) -> Result<String, PrintError> {
    check_reject(r)?;
    Ok(format!("reject {} {}\n", r.code, quote(&r.reason)))
}

fn param_line(d: &ProgramParamDef) -> String {
    let kind = match d.kind {
        ProgramParamKind::Int => "int",
        ProgramParamKind::Float => "float",
        ProgramParamKind::Bool => "bool",
    };
    let v = |x| lit::param_value(d.kind, x);
    let mut s = format!("param {} {kind} = {}", d.key, v(d.default));
    if let (Some(lo), Some(hi)) = (d.min, d.max) {
        s.push_str(&format!(" in {}..{}", v(lo), v(hi)));
    }
    if let Some(u) = &d.unit {
        s.push_str(&format!(" unit {}", quote(u)));
    }
    if let Some(desc) = &d.description {
        s.push(' ');
        s.push_str(&quote(desc));
    }
    s
}

pub(super) fn num(n: &Num) -> String {
    match n {
        Num::Lit(f) => fmt_num(*f),
        Num::Param(p) => p.param.clone(),
    }
}

fn rule(r: &OutcomeRule) -> String {
    let mut s = String::new();
    if let Some(t) = &r.target_pct {
        s.push_str(&format!(" target {}%", num(t)));
    }
    if let Some(t) = &r.stop_pct {
        s.push_str(&format!(" stop {}%", num(t)));
    }
    s.push_str(&format!(" max_bars {}", lit::int_arg(&r.max_bars)));
    s
}

fn output_line(pr: &Printer, o: &Output) -> String {
    match &o.kind {
        OutputKind::Series { node } => format!("series {} = {}", o.name, pr.top(node)),
        OutputKind::Event { node } => format!("event {} = {}", o.name, pr.top(node)),
        OutputKind::Signal {
            node,
            side,
            rule: r,
            source,
        } => {
            let side = match side {
                qp_schema::program::Side::Al => "al",
                qp_schema::program::Side::Sat => "sat",
            };
            format!(
                "signal {} = {} side {side}{} source {} {}",
                o.name,
                pr.top(node),
                rule(r),
                quote(&source.strategy),
                quote(&source.version)
            )
        }
        OutputKind::State { dims } => {
            let ds: Vec<String> = dims
                .iter()
                .map(|d| format!("{} = {}", d.name, pr.top(&d.node)))
                .collect();
            format!("state {} {{ {} }}", o.name, ds.join(", "))
        }
    }
}

/// Text names for the nodes that get a line: the id when the text can spell it and it
/// does not collide with a parameter key (one text namespace), else `v<k>`.
pub(super) fn names(p: &Program, inline: &[bool]) -> Vec<String> {
    let params: BTreeSet<&str> = p.params.iter().map(|d| d.key.as_str()).collect();
    let keep = |id: &str| !id.starts_with('_') && text_name(id) && !params.contains(id);
    let mut taken: BTreeSet<String> = params.iter().map(|s| (*s).to_owned()).collect();
    taken.extend(p.nodes.iter().map(|n| n.id.clone()));
    let mut next = 1u64;
    let mut out = Vec::with_capacity(p.nodes.len());
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for (i, n) in p.nodes.iter().enumerate() {
        if inline[i] || keep(&n.id) {
            out.push(n.id.clone());
            continue;
        }
        if let Some(prev) = seen.get(&n.id) {
            out.push(prev.clone());
            continue;
        }
        while taken.contains(&format!("v{next}")) {
            next += 1;
        }
        let v = format!("v{next}");
        taken.insert(v.clone());
        seen.insert(n.id.clone(), v.clone());
        out.push(v);
    }
    out
}
