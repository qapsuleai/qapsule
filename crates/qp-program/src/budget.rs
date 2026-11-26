//! The per-owner live budget inside one document (PR §8.2, PS §2.3): instances, Σ
//! `live_per_bar` of the instances and the bytes of the owner's program series changes
//! in the log, each against its `program.live.doc.*` limit. Counted for every owner the
//! new changes write a program series for, over the document after them.

use std::collections::{BTreeMap, BTreeSet};

use qp_lang::{check_live_budget, check_program, CheckCtx, Lang, Limits, LiveUsage};
use qp_schema::protocol::{ChangeOp, LogEntry, ProgramError};
use qp_schema::Document;

use crate::inst::program_of;

/// Bytes of a change that writes a program series: its owner and size.
fn program_bytes(c: &ChangeOp) -> Option<(&str, u64)> {
    match c {
        ChangeOp::PutSeries { series, .. } => program_of(series).map(|p| {
            let n = serde_json::to_vec(series).map_or(0, |v| v.len() as u64);
            (p.owner.as_str(), n)
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

/// Program bytes per owner in a log, counted incrementally (one per world: the authority
/// and the public world keep separate logs). Derived from committed entries only, so
/// updating it in the `Check` phase never records anything of a refused command; a log
/// it does not continue (another world, a restart) is counted again from the start.
#[derive(Debug, Default)]
pub(crate) struct LogBytes {
    worlds: [Counted; 2],
}

/// The count of one log: entries counted, the last one's seq, bytes per owner.
#[derive(Debug, Default)]
struct Counted {
    len: usize,
    last_seq: Option<u64>,
    per_owner: BTreeMap<String, u64>,
}

impl LogBytes {
    /// Bytes per owner of `log` (world `public`), counting only entries not seen before.
    pub(crate) fn sync(&mut self, log: &[LogEntry], public: bool) -> &BTreeMap<String, u64> {
        let c = &mut self.worlds[usize::from(public)];
        let continues =
            c.len <= log.len() && (c.len == 0 || log.get(c.len - 1).map(|e| e.seq) == c.last_seq);
        if !continues {
            *c = Counted::default();
        }
        for (o, n) in log[c.len..]
            .iter()
            .flat_map(|e| e.changes.iter())
            .filter_map(program_bytes)
        {
            *c.per_owner.entry(o.to_owned()).or_default() += n;
        }
        c.len = log.len();
        c.last_seq = log.last().map(|e| e.seq);
        &c.per_owner
    }

    /// Bytes per owner of `log`, counted once (no slot to keep the count in).
    pub(crate) fn sync_owned(mut self, log: &[LogEntry]) -> BTreeMap<String, u64> {
        self.sync(log, false);
        std::mem::take(&mut self.worlds[0].per_owner)
    }
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
    fn log_bytes_count_incrementally_and_restart_on_another_log() {
        let one = serde_json::to_vec(&series("a", "arkin")).unwrap().len() as u64;
        let mut log = vec![entry(1, series("a", "arkin"))];
        let mut lb = LogBytes::default();
        assert_eq!(lb.sync(&log, false).get("arkin"), Some(&one));
        log.push(entry(2, series("a", "arkin")));
        assert_eq!(lb.sync(&log, false).get("arkin"), Some(&(2 * one)));
        assert_eq!(
            lb.sync(&log, false).get("arkin"),
            Some(&(2 * one)),
            "not twice"
        );
        // Another log of the same length (another world, a new session): counted afresh.
        let other = vec![
            entry(7, series("a", "deniz")),
            entry(8, series("b", "deniz")),
        ];
        let c = lb.sync(&other, false);
        assert_eq!(c.get("arkin"), None);
        assert!(c.get("deniz").is_some());
        // The public world keeps its own count.
        assert_eq!(lb.sync(&log, true).get("arkin"), Some(&(2 * one)));
    }
}
