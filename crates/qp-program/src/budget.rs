//! The per-owner live budget inside one document (PR §8.2, PS §2.3): instances, Σ
//! `live_per_bar` of the instances and the bytes of the owner's program series changes
//! in the log, each against its `program.live.doc.*` limit. Counted for every owner the
//! new changes write a program series for, over the document after them.

use std::collections::{BTreeMap, BTreeSet};

use qp_lang::{check_live_budget, check_program, CheckCtx, Lang, Limits, LiveUsage};
use qp_schema::protocol::{ChangeOp, LogEntry, ProgramError};
use qp_schema::Document;

use crate::inst::program_of;

/// Counts the bytes written to it.
struct Count(u64);

impl std::io::Write for Count {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Bytes of a change that writes a program series: its owner and its JSON size (counted,
/// not allocated).
fn program_bytes(c: &ChangeOp) -> Option<(&str, u64)> {
    match c {
        ChangeOp::PutSeries { series, .. } => program_of(series).map(|p| {
            let mut n = Count(0);
            let _ = serde_json::to_writer(&mut n, series);
            (p.owner.as_str(), n.0)
        }),
        _ => None,
    }
}

/// The owners `new` writes program series for, in order.
pub(crate) fn owners(new: &[ChangeOp]) -> BTreeSet<String> {
    new.iter()
        .filter_map(|c| program_bytes(c).map(|(o, _)| o.to_owned()))
        .collect()
}

/// Program bytes per owner in `log` (the world's own log: the authority's or the public
/// world's). One walk over the entries' changes; only program series are measured, and
/// each owner's are at most `program.live.doc.max_log_bytes` (this check keeps them
/// there), so the work is bounded by the budget, not by the log's other entries' size.
/// (No incremental count: the expansion, where this must hold on every path, gets no
/// subsystem state; see the crate README.)
pub(crate) fn logged(log: &[LogEntry]) -> BTreeMap<String, u64> {
    let mut out: BTreeMap<String, u64> = BTreeMap::new();
    for (o, n) in log
        .iter()
        .flat_map(|e| e.changes.iter())
        .filter_map(program_bytes)
    {
        *out.entry(o.to_owned()).or_default() += n;
    }
    out
}

/// `owner`'s use in `after` (the document with `new` applied): `prior` program bytes
/// already in the log plus those of `new`.
pub(crate) fn usage(
    after: &Document,
    prior: &BTreeMap<String, u64>,
    new: &[ChangeOp],
    owner: &str,
) -> LiveUsage {
    // One cost per instance (its outputs share the ref), with its bound values.
    let mut costs: BTreeMap<&str, u64> = BTreeMap::new();
    for p in after.series.iter().filter_map(program_of) {
        if p.owner != owner || costs.contains_key(p.instance.as_str()) {
            continue;
        }
        let params = serde_json::to_value(&p.params).ok();
        let ctx = CheckCtx {
            params: params.as_ref(),
            ..CheckCtx::default()
        };
        let cost = check_program(&p.ir, &ctx)
            .cost
            .map_or(0, |c| c.live_per_bar);
        costs.insert(&p.instance, cost);
    }
    let added: u64 = new
        .iter()
        .filter_map(program_bytes)
        .filter(|(o, _)| *o == owner)
        .map(|(_, n)| n)
        .sum();
    LiveUsage {
        instances: costs.len() as u64,
        cost_per_bar: costs.values().sum(),
        log_bytes: prior.get(owner).copied().unwrap_or(0).saturating_add(added),
    }
}

/// Every `E_BUDGET` error of the owners `new` writes for (none when within budget).
pub(crate) fn over_budget(
    after: &Document,
    prior: &BTreeMap<String, u64>,
    new: &[ChangeOp],
    limits: &Limits,
    lang: Lang,
) -> Vec<ProgramError> {
    owners(new)
        .iter()
        .flat_map(|o| check_live_budget(&usage(after, prior, new, o), limits, lang))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_lang::Limit;
    use qp_schema::Series;
    use serde_json::json;

    /// A program series of `owner` (instance `inst`, output `y = close`).
    fn series(inst: &str, owner: &str) -> Series {
        serde_json::from_value(json!({
            "id": format!("{inst}.y"),
            "source": {"kind":"study","study":"program","inputs":["px"],"params":{},"output":"y",
                "program": {"instance": inst, "program_id": "sha256:0", "instance_id": "sha256:0",
                    "scope":"document","owner":owner,"since_t":0,"params":{},
                    "ir": {"version":"qp-ir/0",
                           "nodes":[{"id":"c","op":"input","field":"close"}],
                           "outputs":[{"name":"y","kind":"series","node":"c"}]}}}
        }))
        .unwrap()
    }

    #[test]
    fn usage_counts_the_owners_instances_cost_and_bytes() {
        let mut doc: Document =
            serde_json::from_value(json!({"schema_version":"0","id":"d","series":[],"views":[]}))
                .unwrap();
        let new: Vec<ChangeOp> = [
            series("a", "arkin"),
            series("b", "arkin"),
            series("c", "deniz"),
        ]
        .into_iter()
        .map(|series| ChangeOp::PutSeries {
            series,
            index: None,
        })
        .collect();
        qp_ops::apply_all(&mut doc, &new).unwrap();
        let none = BTreeMap::new();
        let u = usage(&doc, &none, &new, "arkin");
        assert_eq!(u.instances, 2);
        assert!(u.cost_per_bar > 0);
        let bytes = |i| serde_json::to_vec(&series(i, "arkin")).unwrap().len() as u64;
        assert_eq!(u.log_bytes, bytes("a") + bytes("b"));
        assert_eq!(owners(&new).len(), 2);
        // Within the defaults; over a log-bytes limit of 10: one E_BUDGET per owner.
        assert!(over_budget(&doc, &none, &new, &Limits::default(), Lang::En).is_empty());
        let tight = Limits::default().with(Limit::LiveDocMaxLogBytes, 10);
        let errs = over_budget(&doc, &none, &new, &tight, Lang::En);
        assert_eq!(errs.len(), 2);
        assert!(errs
            .iter()
            .all(|e| e.reason == "E_BUDGET" && e.expected == ["10"]));
    }

    /// A log entry `seq` writing `series`.
    fn entry(seq: u64, series: Series) -> LogEntry {
        LogEntry {
            seq,
            at: 0,
            author: "arkin".to_owned(),
            approved_by: None,
            cause: None,
            reason: None,
            request: Vec::new(),
            changes: vec![ChangeOp::PutSeries {
                series,
                index: None,
            }],
            inverse: Vec::new(),
            undoes: None,
            summary: String::new(),
            evidence: None,
        }
    }

    #[test]
    fn logged_counts_each_owners_program_bytes() {
        let one = serde_json::to_vec(&series("a", "arkin")).unwrap().len() as u64;
        let log = vec![
            entry(1, series("a", "arkin")),
            entry(2, series("a", "arkin")),
            entry(3, series("b", "deniz")),
        ];
        let c = logged(&log);
        assert_eq!(c.get("arkin"), Some(&(2 * one)));
        assert_eq!(c.len(), 2);
    }
}
