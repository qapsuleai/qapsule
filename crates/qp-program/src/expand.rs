//! The expansion of the program ops (PS §3.1) into existing `ChangeOp`s: `add_program` →
//! one `put_series` per output (and with `show` its layers), `set_program_params` → a
//! `put_series` per output (same ids, new `instance_id` and `since_t`), `remove_program` →
//! the `remove` rule per output. Every program error at once; nothing applies on error.
//!
//! The expansion checks against the limits it is given: the registry defaults on the
//! command path (`qp_ops::Ctx` has no `param`, PS §2.3), the host's values in the
//! `Check` phase.

use std::collections::BTreeMap;

use qp_lang::{
    check, check_program, Binding, CheckCtx, CheckReport, Lang, Limits, Purpose, Reason,
};
use qp_ops::expand::{expand_op, valid_id, Ctx};
use qp_ops::find_collection;
use qp_plane::error::{ptr, PResult, E};
use qp_plane::plane::{bars_for, is_bar_series};
use qp_plane::values::find_series;
use qp_schema::program::{ProgramParamValue, ProgramRef, ProgramScope, PROGRAM_STUDY};
use qp_schema::protocol::{ChangeOp, ErrorCode, Op, Show};
use qp_schema::{Author, Document, Series, SeriesSource};
use serde_json::Value;

use crate::budget::over_budget;
use crate::inst::{instance_names, output_name, outputs_of, program_of};
use crate::refuse::{refuse, session, under, Located};

/// What an expansion checks against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rules {
    /// Limits in force.
    pub(crate) limits: Limits,
    /// Message language.
    pub(crate) lang: Lang,
}

/// The owner of what a command writes (PS §2.3): the person who accepted the proposal,
/// else the speaker's person (an agent's person; a person is their own).
pub(crate) fn owner_of(ctx: &Ctx<'_>) -> String {
    match &ctx.approved_by {
        Some(Author::Human { name } | Author::Agent { name, .. } | Author::Engine { name, .. }) => {
            name.clone()
        }
        None => qp_ops::watch::person_of(ctx.plane, ctx.speaker).to_owned(),
    }
}

/// The bound bar series of `inputs` (exactly one bar series), or its errors.
fn bar_input<'d>(
    doc: &'d Document,
    inputs: &[String],
    p: &str,
) -> Result<&'d Series, Box<Located>> {
    let ip = ptr(p, "inputs");
    let [one] = inputs else {
        let msg = format!(
            "a program takes exactly one bar series, got {}",
            inputs.len()
        );
        return Err(Box::new(session(Reason::NotBarSeries, &ip, msg)));
    };
    match find_series(doc, one) {
        Some(s) if is_bar_series(s) => Ok(s),
        Some(_) => Err(Box::new(session(
            Reason::NotBarSeries,
            &ptr(&ip, 0),
            format!("`{one}` is not a bar series"),
        ))),
        None => {
            let mut l = session(
                Reason::NotBarSeries,
                &ptr(&ip, 0),
                format!("no series `{one}`"),
            );
            l.err.code = Some(ErrorCode::NotFound);
            l.err.did_you_mean = qp_lang::did_you_mean(
                one,
                doc.series
                    .iter()
                    .filter(|s| is_bar_series(s))
                    .map(|s| s.id.as_str()),
            );
            Err(Box::new(l))
        }
    }
}

/// The check binding of bar series `s` as `doc` and the command's data describe it.
fn binding(doc: &Document, s: &Series, ctx: &Ctx<'_>) -> Binding {
    Binding {
        interval: s.interval.clone(),
        bars: bars_for(doc, ctx.data.as_ref(), &s.id).map(|(_, b)| b.len() as u64),
    }
}

/// The report's errors (and a refusal given as a program) located under `ir_path`.
fn report_errors(r: &CheckReport, ir_path: &str, none_path: &str) -> Vec<Located> {
    let mut out = under(&r.errors, ir_path, none_path);
    if r.ok() && r.reject.is_some() {
        let msg = "a refusal (`reject`) is not a program; nothing to add".to_owned();
        out.push(session(Reason::Parse, ir_path, msg));
        if let Some(l) = out.last_mut() {
            l.err.code = Some(ErrorCode::Invalid);
            l.err.reason = "invalid".to_owned();
        }
    }
    out
}

/// The fields of an `add_program` op.
pub(crate) struct AddArgs<'o> {
    pub(crate) id: Option<&'o String>,
    pub(crate) program: &'o Value,
    pub(crate) inputs: &'o [String],
    pub(crate) params: &'o BTreeMap<String, ProgramParamValue>,
    pub(crate) show: Option<&'o Show>,
}

/// `add_program` (PS §3.1).
#[allow(clippy::result_large_err)]
pub(crate) fn add_program(
    doc: &Document,
    a: AddArgs<'_>,
    p: &str,
    ctx: &mut Ctx<'_>,
    rules: Rules,
) -> PResult<Vec<ChangeOp>> {
    let AddArgs {
        id,
        program,
        inputs,
        params,
        show,
    } = a;
    let input = bar_input(doc, inputs, p);
    let params_v = serde_json::to_value(params).ok();
    let cctx = CheckCtx {
        limits: rules.limits,
        binding: input.as_ref().ok().map(|s| binding(doc, s, ctx)),
        params: params_v.as_ref(),
        lang: rules.lang,
        purpose: Purpose::Document,
    };
    let report = check(program, &cctx);
    let mut errors: Vec<Located> = input
        .as_ref()
        .err()
        .map(|b| (**b).clone())
        .into_iter()
        .collect();
    errors.extend(report_errors(
        &report,
        &ptr(p, "program"),
        &ptr(p, "params"),
    ));
    let (Ok(input), Some(checked), true) = (input, report.checked.as_ref(), errors.is_empty())
    else {
        return Err(refuse(errors));
    };
    let instance = instance_id(doc, id, p, ctx)?;
    let pref = ProgramRef {
        instance: instance.clone(),
        program_id: checked.program_id.clone(),
        instance_id: checked.instance_id.clone(),
        scope: ProgramScope::Document,
        owner: owner_of(ctx),
        since_t: ctx.at,
        params: checked.params.clone(),
        ir: checked.program.normalized(),
    };
    let names: Vec<&str> = checked.outputs.iter().map(|o| o.name.as_str()).collect();
    let mut out = put_outputs(doc, &names, input, &pref, p)?;
    budget_check(doc, &out, ctx, rules, p)?;
    for c in &out {
        if let ChangeOp::PutSeries { series, .. } = c {
            ctx.n += 1;
            ctx.created.push(series.id.clone());
        }
    }
    if let Some(sh) = show {
        let layers = crate::show::layers(doc, &out, sh, &ptr(p, "show"), ctx)?;
        out.extend(layers);
    }
    Ok(out)
}

/// One `put_series` per output name of instance `pref` over bar series `input`.
#[allow(clippy::result_large_err)]
fn put_outputs(
    doc: &Document,
    names: &[&str],
    input: &Series,
    pref: &ProgramRef,
    p: &str,
) -> PResult<Vec<ChangeOp>> {
    let instance = &pref.instance;
    let mut out = Vec::new();
    for name in names {
        let sid = format!("{instance}.{name}");
        if !valid_id(&sid) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("series id `{sid}` is too long or malformed"),
            )
            .at(ptr(p, "id"))
            .expected(["^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$"])
            .hint("Choose a shorter instance id.")
            .build());
        }
        if find_collection(doc, &sid).is_some() {
            return Err(
                E::new(ErrorCode::Duplicate, format!("id `{sid}` already exists"))
                    .at(ptr(p, "id"))
                    .hint("Choose another instance id or omit it to have one assigned.")
                    .build(),
            );
        }
        out.push(ChangeOp::PutSeries {
            series: output_series(&sid, input, name, pref),
            index: None,
        });
    }
    Ok(out)
}

/// The output series `sid` of instance `pref` over bar series `input`.
fn output_series(sid: &str, input: &Series, output: &str, pref: &ProgramRef) -> Series {
    Series {
        id: sid.to_owned(),
        instrument: input.instrument.clone(),
        interval: input.interval.clone(),
        source: SeriesSource::Study {
            study: PROGRAM_STUDY.to_owned(),
            inputs: vec![input.id.clone()],
            params: BTreeMap::new(),
            output: Some(output.to_owned()),
            program: Some(Box::new(pref.clone())),
        },
        data: None,
    }
}

/// The instance name: the given id (checked, not taken), else `prog-<seq>-<n>`.
#[allow(clippy::result_large_err)]
fn instance_id(doc: &Document, id: Option<&String>, p: &str, ctx: &mut Ctx<'_>) -> PResult<String> {
    let name = match id {
        Some(g) => {
            if !valid_id(g) || g.starts_with("watch-") {
                return Err(
                    E::new(ErrorCode::Invalid, format!("invalid instance id `{g}`"))
                        .at(ptr(p, "id"))
                        .expected(["^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$"])
                        .build(),
                );
            }
            g.clone()
        }
        None => {
            let n = format!("prog-{}-{}", ctx.id_seq, ctx.n);
            ctx.n += 1;
            n
        }
    };
    if instance_names(doc).contains(&name.as_str()) || find_collection(doc, &name).is_some() {
        return Err(E::new(
            ErrorCode::Duplicate,
            format!("instance `{name}` already exists"),
        )
        .at(ptr(p, "id"))
        .hint("Choose another id or omit it to have one assigned.")
        .build());
    }
    Ok(name)
}

/// The live budget after `new` (applied to a copy of `doc`), against `ctx`'s log.
#[allow(clippy::result_large_err)]
fn budget_check(
    doc: &Document,
    new: &[ChangeOp],
    ctx: &Ctx<'_>,
    rules: Rules,
    p: &str,
) -> PResult<()> {
    let mut after = doc.clone();
    qp_ops::apply_all(&mut after, new).map_err(|m| qp_plane::error::invalid(m, "/ops"))?;
    let errs = over_budget(&after, ctx.log, new, &rules.limits, rules.lang);
    if errs.is_empty() {
        return Ok(());
    }
    Err(refuse(under(&errs, p, p)))
}

/// The program series of instance `name`, or `not_found` with close instance names.
#[allow(clippy::result_large_err)]
fn need_instance<'d>(doc: &'d Document, name: &str, p: &str) -> PResult<Vec<&'d Series>> {
    let outs = outputs_of(doc, name);
    if outs.is_empty() {
        return Err(
            E::new(ErrorCode::NotFound, format!("no program instance `{name}`"))
                .at(ptr(p, "program"))
                .dym(name, instance_names(doc))
                .build(),
        );
    }
    Ok(outs)
}

/// `set_program_params` (PS §3.1): merged values, re-checked; the same series ids (and
/// layers), the same IR and owner; a new `instance_id` and `since_t`.
#[allow(clippy::result_large_err)]
pub(crate) fn set_program_params(
    doc: &Document,
    name: &str,
    params: &BTreeMap<String, ProgramParamValue>,
    p: &str,
    ctx: &Ctx<'_>,
    rules: Rules,
) -> PResult<Vec<ChangeOp>> {
    let outs = need_instance(doc, name, p)?;
    let Some(old) = program_of(outs[0]) else {
        return Err(qp_plane::error::invalid(
            "not a program series",
            ptr(p, "program"),
        ));
    };
    let mut merged = old.params.clone();
    merged.extend(params.iter().map(|(k, v)| (k.clone(), *v)));
    let merged_v = serde_json::to_value(&merged).ok();
    let input = crate::inst::input_of(outs[0]).and_then(|i| find_series(doc, i));
    let cctx = CheckCtx {
        limits: rules.limits,
        binding: input.map(|s| binding(doc, s, ctx)),
        params: merged_v.as_ref(),
        lang: rules.lang,
        purpose: Purpose::Document,
    };
    let report = check_program(&old.ir, &cctx);
    let Some(checked) = report.checked.as_ref() else {
        return Err(refuse(under(&report.errors, p, &ptr(p, "params"))));
    };
    let pref = ProgramRef {
        instance_id: checked.instance_id.clone(),
        since_t: ctx.at,
        params: checked.params.clone(),
        ..old.clone()
    };
    let out: Vec<ChangeOp> = outs
        .iter()
        .map(|s| {
            let mut ns = (*s).clone();
            if let SeriesSource::Study { program, .. } = &mut ns.source {
                *program = Some(Box::new(pref.clone()));
            }
            ChangeOp::PutSeries {
                series: ns,
                index: None,
            }
        })
        .collect();
    debug_assert!(outs.iter().all(|s| output_name(s).is_some()));
    budget_check(doc, &out, ctx, rules, p)?;
    Ok(out)
}

/// `remove_program` (PS §3.1): each output series by the `remove` rule (the layers
/// drawing it first; `in_use` when a condition or an item rests on it).
#[allow(clippy::result_large_err)]
pub(crate) fn remove_program(
    doc: &Document,
    name: &str,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let ids: Vec<String> = need_instance(doc, name, p)?
        .iter()
        .map(|s| s.id.clone())
        .collect();
    let mut scratch = doc.clone();
    let mut out = Vec::new();
    for id in ids {
        let cs = expand_op(&scratch, &Op::Remove { id }, p, ctx).map_err(|mut e| {
            e.path = Some(ptr(p, "program"));
            e
        })?;
        qp_ops::apply_all(&mut scratch, &cs).map_err(|m| qp_plane::error::invalid(m, p))?;
        out.extend(cs);
    }
    Ok(out)
}
