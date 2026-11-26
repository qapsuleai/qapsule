//! `qp program run` and `explain` bodies: the try summary, the full bar-by-bar output of a
//! closed window (PS §3.3), and the canonical text with per node cost.

use std::collections::BTreeMap;

use qp_lang::{
    try_program, try_window, Align, Binding, CheckReport, ExecOptions, Executor, RecordCtx,
    TryRequest, WorkBudget,
};
use qp_schema::program::{NodeCost, ProgramExplain, ProgramOutputKind};
use qp_schema::protocol::QueryResult;
use qp_schema::{Bar, Series};
use serde_json::{json, Map, Value};

use super::{limits, prepare, print_json, print_refused, print_value, Prepared, RunArgs};
use crate::Outcome;

/// Who a CLI run's simulated records name as owner and author.
const CLI_PRINCIPAL: &str = "cli";

/// Reads a bar series file (`fixtures/bars` format).
fn load_series(path: &std::path::Path) -> Result<Series, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let s: Series = serde_json::from_str(&text)
        .map_err(|e| format!("{}: not a bar series: {e}", path.display()))?;
    match s.bars() {
        Some(b) if !b.is_empty() => Ok(s),
        _ => Err(format!(
            "{}: the series has no embedded bars",
            path.display()
        )),
    }
}

/// The record context of a CLI run over `s`.
fn record_ctx(s: &Series) -> RecordCtx {
    RecordCtx {
        instance: "try".to_owned(),
        owner: CLI_PRINCIPAL.to_owned(),
        author: qp_schema::Author::Human {
            name: CLI_PRINCIPAL.to_owned(),
        },
        instrument: s.symbol().unwrap_or_default().to_owned(),
        period: s.interval.clone().unwrap_or_default(),
        series: s.id.clone(),
        anchor_t: None,
        feeds: Vec::new(),
    }
}

pub(super) fn run_cmd(a: &RunArgs) -> Result<Outcome, String> {
    let series = load_series(&a.data)?;
    let bars: &[Bar] = series.bars().unwrap_or_default();
    let binding = Binding {
        interval: series.interval.clone(),
        bars: Some(bars.len() as u64),
    };
    let rep = match prepare(&a.file, a.lang.as_deref(), Some(binding))? {
        Prepared::Checked(rep) => rep,
        Prepared::Answer(ans) => return print_refused(&ans, a.json),
    };
    let Some(checked) = rep.checked.as_ref() else {
        return Err("internal: program did not check".to_owned());
    };
    // A closed window (PS §3.3): every bar of the file is closed, the last one too.
    let req = TryRequest {
        checked,
        bars,
        from_t: None,
        to_t: None,
        align: Align::for_calendar(
            series
                .instrument
                .as_ref()
                .and_then(|i| i.calendar.as_deref()),
        ),
        limits: limits(),
        ctx: record_ctx(&series),
    };
    if !a.full {
        let t = try_program(&req).map_err(|e| e.to_string())?;
        let answer = QueryResult::ProgramTry(Box::new(t));
        if a.json {
            print_json(&answer)?;
        } else {
            print!("{}", summary_text(&answer));
        }
        return Ok(Outcome::Ok);
    }
    let out = full_output(&rep, &req)?;
    if a.json {
        print_value(&out)?;
    } else {
        print!("{}", full_text(&out));
    }
    Ok(Outcome::Ok)
}

/// Every output of a closed-window run, bar by bar (PS §3.3 `--full`).
fn full_output(rep: &CheckReport, req: &TryRequest<'_>) -> Result<Value, String> {
    let bars = try_window(req);
    let budget = WorkBudget::from_limits(&req.limits);
    let opts = ExecOptions {
        align: req.align.clone(),
        outputs: None,
        budget: Some(budget),
    };
    let x = Executor::run_closed(req.checked, bars, opts).map_err(|e| e.to_string())?;
    let n = x.closed_len();
    let mut outputs = Map::new();
    for info in &rep.outputs {
        let Some(o) = x.output(&info.name) else {
            continue;
        };
        let v = match info.kind {
            ProgramOutputKind::Series => {
                let values: Vec<Option<f64>> = (0..n).map(|i| x.value_at(o, i)).collect();
                json!({"kind": "series", "values": values})
            }
            ProgramOutputKind::Event => {
                let at: Vec<usize> = (0..n).filter(|&i| x.value_at(o, i) == Some(1.0)).collect();
                let t: Vec<Option<i64>> = at.iter().map(|&i| x.t_at(i)).collect();
                json!({"kind": "event", "bars": at, "t": t})
            }
            ProgramOutputKind::Signal => {
                let mut records = Vec::new();
                for i in 0..n {
                    let Some(hit) = x.signals_at(o, i) else {
                        continue;
                    };
                    let exit_bar = hit.exit_bar;
                    let Some(rec) = x.record_at(req.checked, o, i, &req.ctx) else {
                        continue;
                    };
                    let mut r = serde_json::to_value(&rec).map_err(|e| e.to_string())?;
                    r["bar"] = json!(i);
                    r["exit_bar"] = json!(exit_bar);
                    records.push(r);
                }
                json!({"kind": "signal", "records": records})
            }
            ProgramOutputKind::State => {
                let keys: Vec<Option<Vec<u32>>> = (0..n)
                    .map(|i| x.state_at(o, i).map(<[u32]>::to_vec))
                    .collect();
                let mut counts: BTreeMap<&[u32], u64> = BTreeMap::new();
                for k in keys.iter().flatten() {
                    *counts.entry(k.as_slice()).or_default() += 1;
                }
                let counts: Vec<Value> = counts
                    .into_iter()
                    .map(|(key, count)| json!({"key": key, "count": count}))
                    .collect();
                json!({"kind": "state", "keys": keys, "counts": counts})
            }
        };
        outputs.insert(info.name.clone(), v);
    }
    let stopped = x.stopped().map(
        |s| json!({"reason": "budget", "bar": s.bar, "t": s.t, "work": s.work, "limit": s.limit}),
    );
    let mut v = json!({
        "kind": "program_run",
        "program_id": req.checked.program_id,
        "bars": n,
        "from_t": bars.first().map(|b| b.t),
        "to_t": n.checked_sub(1).and_then(|i| x.t_at(i)),
        "outputs": outputs,
        "work": {"used": x.work(), "limit": budget.max_work},
    });
    if let Some(s) = stopped {
        v["stopped"] = s;
    }
    Ok(v)
}

/// A try summary as plain text: one line per output.
fn summary_text(answer: &QueryResult) -> String {
    use std::fmt::Write as _;
    let QueryResult::ProgramTry(t) = answer else {
        return String::new();
    };
    let mut s = String::new();
    let t_of = |t: Option<i64>| t.map_or_else(|| "-".to_owned(), |t| t.to_string());
    let _ = writeln!(
        s,
        "from_t {} to_t {} work {}/{}",
        t_of(t.from_t),
        t_of(t.to_t),
        t.work.used,
        t.work.limit
    );
    for x in &t.series {
        let _ = writeln!(s, "series {}: {}", x.name, json!(x));
    }
    for x in &t.event {
        let _ = writeln!(s, "event {}: {}", x.name, json!(x));
    }
    for x in &t.signal {
        let _ = writeln!(s, "signal {}: {}", x.name, json!(x));
    }
    for x in &t.state {
        let _ = writeln!(s, "state {}: {}", x.name, json!(x));
    }
    if let Some(st) = &t.stopped {
        let _ = writeln!(s, "stopped: {}", json!(st));
    }
    s
}

/// A full run as plain text: one line per output with its counts.
fn full_text(v: &Value) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "bars {}", v["bars"]);
    if let Some(outs) = v["outputs"].as_object() {
        for (name, o) in outs {
            let count = |k: &str| o[k].as_array().map_or(0, Vec::len);
            let what = match o["kind"].as_str().unwrap_or_default() {
                "series" => format!("{} values", count("values")),
                "event" => format!("{} events", count("bars")),
                "signal" => format!("{} records", count("records")),
                _ => format!("{} distinct keys", count("counts")),
            };
            let _ = writeln!(s, "{} {name}: {what}", o["kind"].as_str().unwrap_or("?"));
        }
    }
    s
}

/// The `program_explain` answer of a checked program (no instance, no `at`).
pub(super) fn explain(rep: &CheckReport) -> Result<QueryResult, String> {
    let Some(c) = rep.checked.as_ref() else {
        return Err("internal: program did not check".to_owned());
    };
    let text = qp_lang_text::print(&c.program.normalized())
        .map_err(|e| format!("the program has no canonical text: {e:?}"))?;
    let cost = c
        .program
        .nodes
        .iter()
        .zip(&c.nodes)
        .map(|(n, cn)| NodeCost {
            id: cn.id.clone(),
            op: serde_json::to_value(n)
                .ok()
                .and_then(|v| v.get("op").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_default(),
            clock: cn.clock.name().to_owned(),
            w: cn.w,
        })
        .collect();
    Ok(QueryResult::ProgramExplain(Box::new(ProgramExplain {
        text,
        program_id: c.program_id.clone(),
        instance_id: None,
        params: c.params.clone(),
        cost,
        outputs: rep.outputs.clone(),
        provenance: None,
    })))
}
