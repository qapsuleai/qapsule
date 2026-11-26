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

/// Adds the program bytes of `entries` to `per_owner`.
fn count(entries: &[LogEntry], per_owner: &mut BTreeMap<String, u64>) {
    for (o, n) in entries
        .iter()
        .flat_map(|e| e.changes.iter())
        .filter_map(program_bytes)
    {
        *per_owner.entry(o.to_owned()).or_default() += n;
    }
}

/// Program bytes per owner in the authority's log (its slot, PS §2.3): one scan at
/// registration, then each committed entry's changes in [`Phase::AfterCommit`]
/// (`crate::subsystem`), read by the `Check` phase. Every entry is committed in the
/// authority's world (the public world mirrors it), so this one count is the strictest
/// for both worlds. A log it does not continue (cleared, replaced: a different length or
/// last seq than it counted) is scanned again before it is read — never read stale.
///
/// [`Phase::AfterCommit`]: qp_subsystem::Phase::AfterCommit
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LogBytes {
    len: usize,
    last_seq: Option<u64>,
    per_owner: BTreeMap<String, u64>,
}

impl LogBytes {
    /// The count of `log`, from the start.
    pub(crate) fn scan(log: &[LogEntry]) -> LogBytes {
        let mut per_owner = BTreeMap::new();
        count(log, &mut per_owner);
        LogBytes {
            len: log.len(),
            last_seq: log.last().map(|e| e.seq),
            per_owner,
        }
    }

    /// Whether it counted exactly `log`'s entries so far (a prefix of `n` entries).
    fn counted(&self, log: &[LogEntry], n: usize) -> bool {
        self.len == n
            && n <= log.len()
            && (n == 0 || log.get(n - 1).map(|e| e.seq) == self.last_seq)
    }

    /// After an entry was committed to `log`: counts the entries after the ones it
    /// counted, or `log` again when it does not continue it.
    pub(crate) fn on_commit(&mut self, log: &[LogEntry]) {
        if !self.counted(log, self.len) {
            *self = LogBytes::scan(log);
            return;
        }
        count(&log[self.len..], &mut self.per_owner);
        self.len = log.len();
        self.last_seq = log.last().map(|e| e.seq);
    }

    /// The bytes per owner of `log` (counted again first unless it counted all of it).
    pub(crate) fn current(&mut self, log: &[LogEntry]) -> &BTreeMap<String, u64> {
        if !self.counted(log, log.len()) {
            *self = LogBytes::scan(log);
        }
        &self.per_owner
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
    fn the_slot_follows_commits_and_rebuilds_on_another_log() {
        let one = serde_json::to_vec(&series("a", "arkin")).unwrap().len() as u64;
        let mut log = Vec::new();
        let mut lb = LogBytes::scan(&log);
        for k in 1..=50 {
            log.push(entry(k, series("a", "arkin")));
            lb.on_commit(&log);
        }
        assert_eq!(lb, LogBytes::scan(&log), "incremental = a full scan");
        assert_eq!(lb.current(&log).get("arkin"), Some(&(50 * one)));
        // An undo writes the series again: its bytes count too.
        log.push(entry(51, series("b", "arkin")));
        lb.on_commit(&log);
        assert_eq!(lb.current(&log).get("arkin"), Some(&(51 * one)));
        // Cleared (a follower restart): rebuilt, not stale.
        log.clear();
        assert!(lb.current(&log).is_empty());
        // Replaced by another log of the same length: rebuilt by its last seq.
        let a: Vec<LogEntry> = (1..=3).map(|k| entry(k, series("a", "arkin"))).collect();
        let b: Vec<LogEntry> = (7..=9).map(|k| entry(k, series("a", "deniz"))).collect();
        let mut lb = LogBytes::scan(&a);
        assert!(lb.current(&b).get("arkin").is_none());
        assert!(lb.current(&b).get("deniz").is_some());
        // A commit after a missed one: scanned again.
        let mut lb = LogBytes::scan(&a[..1]);
        lb.on_commit(&a);
        assert_eq!(lb, LogBytes::scan(&a));
    }
}
