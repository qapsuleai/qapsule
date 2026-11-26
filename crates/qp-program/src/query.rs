//! The program queries (PS §3.2), side-effect free: `check_program` (program errors are
//! in the answer), `try_program` (replay mode over the reader's closed bars) and
//! `explain_program` (canonical text, per node cost, a value's provenance).

use qp_lang::{
    check, try_program, Align, CheckCtx, CheckReport, ExecOptions, Executor, Purpose, RecordCtx,
    TryRequest,
};
use qp_plane::error::{invalid, PResult, E};
use qp_plane::plane::binding_of;
use qp_schema::program::{NodeCost, ProgramExplain, ProvenanceData};
use qp_schema::protocol::{ErrorCode, Query, QueryResult, Range};
use qp_schema::{Bar, Series};
use qp_subsystem::{QueryHandler, SessionCtx, SessionView};

use crate::asked::{
    bar_series, binding, closed_bars, lang, limits, need_input, principal, resolve, Asked, Resolved,
};
use crate::refuse::{refuse, under};

/// A program query handler: its wire tag.
pub(crate) struct ProgramQuery(&'static str);

/// `check_program`.
pub(crate) static CHECK_PROGRAM: ProgramQuery = ProgramQuery("check_program");
/// `try_program`.
pub(crate) static TRY_PROGRAM: ProgramQuery = ProgramQuery("try_program");
/// `explain_program`.
pub(crate) static EXPLAIN_PROGRAM: ProgramQuery = ProgramQuery("explain_program");

impl QueryHandler for ProgramQuery {
    fn kind(&self) -> &'static str {
        self.0
    }
    fn answer(
        &self,
        conn: &str,
        q: &Query,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<QueryResult, qp_schema::protocol::ProtocolError> {
        let view = ctx.view;
        match q {
            Query::CheckProgram {
                program,
                source,
                inputs,
                params,
                lang,
            } => {
                let a = Asked {
                    program: program.as_ref(),
                    source: source.as_ref(),
                    instance: None,
                    inputs,
                    params,
                    lang: *lang,
                };
                check_answer(view, conn, &a)
            }
            Query::TryProgram {
                program,
                source,
                instance,
                inputs,
                params,
                range,
                lang,
            } => {
                let a = Asked {
                    program: program.as_ref(),
                    source: source.as_ref(),
                    instance: instance.as_ref(),
                    inputs,
                    params,
                    lang: *lang,
                };
                try_answer(view, conn, &a, range.as_ref())
            }
            Query::ExplainProgram {
                program,
                source,
                instance,
                inputs,
                params,
                at,
                lang,
            } => {
                let a = Asked {
                    program: program.as_ref(),
                    source: source.as_ref(),
                    instance: instance.as_ref(),
                    inputs,
                    params,
                    lang: *lang,
                };
                explain_answer(view, conn, &a, at.as_ref())
            }
            _ => Err(invalid("not a program query", "/query")),
        }
    }
}

/// The checker's report for `r` bound to its bar series (if any) as `principal` reads it.
#[allow(clippy::result_large_err)]
fn report(
    view: &dyn SessionView,
    principal: &str,
    r: &Resolved,
    a: &Asked<'_>,
) -> PResult<(CheckReport, Option<Series>)> {
    let series = match &r.input {
        Some(id) => Some(bar_series(view, id)?.clone()),
        None => None,
    };
    let ctx = CheckCtx {
        limits: limits(view),
        binding: series.as_ref().map(|s| binding(view, principal, s)),
        params: Some(&r.params).filter(|p| !p.is_null()),
        lang: lang(a.lang).0,
        purpose: Purpose::Check,
    };
    let mut rep = check(&r.raw, &ctx);
    if let Some(map) = &r.map {
        map.attach(&mut rep.errors);
        map.attach(&mut rep.warnings);
    }
    Ok((rep, series))
}

/// `check_program`.
#[allow(clippy::result_large_err)]
fn check_answer(view: &dyn SessionView, conn: &str, a: &Asked<'_>) -> PResult<QueryResult> {
    let who = principal(view, conn)?;
    let lim = limits(view);
    let r = match resolve(view, a, &lim)? {
        Ok(r) => r,
        Err(errors) => {
            let mut ans = CheckReport {
                errors,
                truncated: false,
                warnings: Vec::new(),
                cost: None,
                outputs: Vec::new(),
                params: Vec::new(),
                program_id: None,
                instance_id: None,
                reject: None,
                checked: None,
                limits: lim,
            }
            .to_answer();
            ans.ok = false;
            return Ok(QueryResult::ProgramCheck(Box::new(ans)));
        }
    };
    let (rep, _) = report(view, who, &r, a)?;
    let mut ans = rep.to_answer();
    ans.ir = r.lowered;
    Ok(QueryResult::ProgramCheck(Box::new(ans)))
}

/// The resolved, checked program of a try or an explanation, or its refusal.
#[allow(clippy::result_large_err)]
fn checked_of(
    view: &dyn SessionView,
    who: &str,
    a: &Asked<'_>,
) -> PResult<(Resolved, CheckReport, Option<Series>)> {
    let r = match resolve(view, a, &limits(view))? {
        Ok(r) => r,
        Err(errors) => return Err(refuse(under(&errors, "/query/source", "/query/source"))),
    };
    let (rep, series) = report(view, who, &r, a)?;
    if rep.checked.is_none() {
        let path = if r.map.is_some() {
            "/query/source"
        } else {
            "/query/program"
        };
        let mut errs = under(&rep.errors, path, "/query/params");
        if errs.is_empty() {
            errs = under(
                &[qp_lang::Reason::NoOutput
                    .error(None, "a refusal is not a program to run".to_owned())],
                path,
                path,
            );
        }
        return Err(refuse(errs));
    }
    Ok((r, rep, series))
}

/// The record context of a run over bar series `s`.
fn record_ctx(view: &dyn SessionView, who: &str, r: &Resolved, s: &Series) -> RecordCtx {
    let b = binding_of(s);
    RecordCtx {
        instance: r
            .instance
            .as_ref()
            .map_or_else(|| "try".to_owned(), |p| p.instance.clone()),
        owner: r
            .instance
            .as_ref()
            .map_or_else(|| who.to_owned(), |p| p.owner.clone()),
        author: view.principal(who).map_or_else(
            || qp_schema::Author::Human {
                name: who.to_owned(),
            },
            |p| p.author.clone(),
        ),
        instrument: s.symbol().unwrap_or_default().to_owned(),
        period: s.interval.clone().unwrap_or_default(),
        series: s.id.clone(),
        anchor_t: b.as_ref().and_then(|b| b.anchor_t),
        feeds: b.map(|b| vec![b.feed_key()]).unwrap_or_default(),
    }
}

/// The session alignment of bar series `s` (its instrument's calendar), else UTC.
fn align_of(s: &Series) -> Align {
    Align::for_calendar(s.instrument.as_ref().and_then(|i| i.calendar.as_deref()))
}

/// `[from_t, to_t]` of `range` over `bars`.
fn window(range: Option<&Range>, bars: &[Bar]) -> (Option<i64>, Option<i64>) {
    let Some(r) = range else { return (None, None) };
    let at = |i: usize| bars.get(i).map(|b| b.t);
    let idx = |i: u64| usize::try_from(i).ok();
    let from = r
        .from_t
        .or_else(|| r.from_bar.and_then(idx).and_then(at))
        .or_else(|| {
            r.last
                .and_then(idx)
                .map(|n| bars.len().saturating_sub(n))
                .and_then(at)
        });
    let to = r.to_t.or_else(|| r.to_bar.and_then(idx).and_then(at));
    (from, to)
}

/// `try_program`.
#[allow(clippy::result_large_err)]
fn try_answer(
    view: &dyn SessionView,
    conn: &str,
    a: &Asked<'_>,
    range: Option<&Range>,
) -> PResult<QueryResult> {
    let who = principal(view, conn)?;
    let (r, rep, _) = checked_of(view, who, a)?;
    let s = bar_series(view, need_input(&r)?)?;
    let bars = closed_bars(view, who, s)?;
    let (from_t, to_t) = window(range, bars);
    let Some(checked) = rep.checked.as_ref() else {
        return Err(invalid("program did not check", "/query"));
    };
    let req = TryRequest {
        checked,
        bars,
        from_t,
        to_t,
        align: align_of(s),
        limits: limits(view),
        ctx: record_ctx(view, who, &r, s),
    };
    let t = try_program(&req).map_err(|e| invalid(e.to_string(), "/query"))?;
    Ok(QueryResult::ProgramTry(Box::new(t)))
}

/// `explain_program`.
#[allow(clippy::result_large_err)]
fn explain_answer(
    view: &dyn SessionView,
    conn: &str,
    a: &Asked<'_>,
    at: Option<&qp_schema::program::ExplainAt>,
) -> PResult<QueryResult> {
    let who = principal(view, conn)?;
    let (r, rep, series) = checked_of(view, who, a)?;
    let Some(c) = rep.checked.as_ref() else {
        return Err(invalid("program did not check", "/query"));
    };
    let text = qp_lang_text::print(&c.program.normalized()).map_err(|e| {
        E::new(
            ErrorCode::Invalid,
            format!("the program has no canonical text: {e:?}"),
        )
        .at("/query")
        .build()
    })?;
    let cost = c
        .program
        .nodes
        .iter()
        .zip(&c.nodes)
        .map(|(n, cn)| NodeCost {
            id: cn.id.clone(),
            op: serde_json::to_value(n)
                .ok()
                .and_then(|v| v.get("op").and_then(|o| o.as_str()).map(str::to_owned))
                .unwrap_or_default(),
            clock: cn.clock.name().to_owned(),
            w: cn.w,
        })
        .collect();
    let provenance = match at {
        None => None,
        Some(at) => {
            need_input(&r)?;
            let Some(s) = series.as_ref() else {
                return Err(invalid("no bar series", "/query/inputs"));
            };
            let bars = closed_bars(view, who, s)?;
            let opts = ExecOptions {
                align: align_of(s),
                ..ExecOptions::default()
            };
            let x = Executor::run_closed(c, bars, opts)
                .map_err(|e| invalid(e.to_string(), "/query"))?;
            let rc = record_ctx(view, who, &r, s);
            let data = ProvenanceData {
                series: rc.series,
                anchor_t: rc.anchor_t,
                feeds: rc.feeds,
            };
            let found = x
                .output(&at.output)
                .zip(x.index_of(at.t))
                .and_then(|(o, i)| x.provenance(o, i, &c.program_id, data));
            Some(found.ok_or_else(|| {
                E::new(
                    ErrorCode::NotFound,
                    format!("no value of `{}` at t={}", at.output, at.t),
                )
                .at("/query/at")
                .dym(&at.output, x.outputs())
                .build()
            })?)
        }
    };
    Ok(QueryResult::ProgramExplain(Box::new(ProgramExplain {
        text,
        program_id: c.program_id.clone(),
        instance_id: r.instance.as_ref().map(|p| p.instance_id.clone()),
        params: c.params.clone(),
        cost,
        outputs: rep.outputs.clone(),
        provenance,
    })))
}
