//! The simulation driver: identical for every model.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::common::{
    anomalies, concretize, concurrency_stats, validate, vc_max, Anomalies, Code,
    ConcurrencyStats, Intent, SemOp, Snap, Vc, Who, WriteRec, Workload, ALL,
};

/// What the writer experienced for one write.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Exp {
    /// Applied (authority ack, or local CRDT write).
    Applied,
    /// Applied after explicit errors and a re-read.
    AppliedAfterRetry(Vec<Code>),
    /// Explicitly refused; the writer was told why.
    Refused(Vec<Code>),
    /// Held locally while offline; the outcome comes on reconnect.
    Queued,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Wire {
    /// Bytes per write sent by the writer (command JSON / CRDT change).
    pub up: Vec<usize>,
    /// Bytes per write received by one subscriber (notify JSON / CRDT change).
    pub down: Vec<usize>,
    /// Bytes of replies (ack / error).
    pub replies: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Reconnect {
    pub queued: usize,
    pub applied: usize,
    pub applied_after_retry: usize,
    pub refused: usize,
    pub codes: BTreeMap<String, usize>,
    /// Bytes editor -> others.
    pub bytes_up: usize,
    /// Bytes others -> editor.
    pub bytes_down: usize,
    /// Round trips (messages pairs) needed.
    pub round_trips: usize,
    pub time_us: f64,
    /// Per queued write: "kind: outcome".
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UndoStats {
    pub attempted: usize,
    pub applied: usize,
    pub refused_conflict: usize,
    pub nothing_or_unsupported: usize,
    /// Undos that overwrote a value another writer had written after the undone change.
    pub clobbered_others: usize,
}

pub trait Model: Sized {
    const NAME: &'static str;
    fn new(doc: &qp_schema::Document) -> Self;
    /// The local view of `who`.
    fn view(&self, who: Who) -> Snap;
    /// `who` writes `op` (already validated against its own view). `online` is false
    /// while the writer is offline.
    /// `view` is the writer's current view (as returned by `view(who)`).
    fn write(&mut self, who: Who, it: &Intent, op: &SemOp, online: bool, view: &Snap) -> Exp;
    /// Deliver this round's changes among online replicas.
    fn end_round(&mut self, online: [bool; 3]);
    /// `who` comes back online.
    fn reconnect(&mut self, who: Who) -> Reconnect;
    /// Final state of every replica.
    fn snaps(&self) -> Vec<Snap>;
    /// Bytes that persist the document with full history (uncompressed).
    fn history_bytes(&mut self) -> Vec<u8>;
    /// Bytes of the current state only (no history), uncompressed.
    fn state_bytes(&mut self) -> usize;
    fn wire(&self) -> &Wire;
    fn undo_stats(&self) -> &UndoStats;
    /// Model-specific counters (conflicts, blind overwrites...).
    fn extra(&self) -> BTreeMap<String, usize>;
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub model: String,
    pub ops: usize,
    pub window: usize,
    pub idle: usize,
    pub locally_invalid: usize,
    pub applied: usize,
    pub applied_after_retry: usize,
    pub refused: usize,
    pub refused_codes: BTreeMap<String, usize>,
    pub converged: bool,
    pub anomalies: Anomalies,
    pub anomaly_total: usize,
    pub concurrency: ConcurrencyStats,
    pub reconnect: Reconnect,
    pub wire_up_mean: f64,
    pub wire_up_median: usize,
    pub wire_down_mean: f64,
    pub wire_down_median: usize,
    pub wire_up_total: usize,
    pub wire_down_total: usize,
    pub replies_total: usize,
    pub history_bytes: usize,
    pub history_gzip: usize,
    pub state_bytes: usize,
    pub replicate_time_us: f64,
    pub history_sha: String,
    pub final_snap_sha: String,
    pub undo: UndoStats,
    pub extra: BTreeMap<String, usize>,
    pub final_layers: usize,
    pub final_series: usize,
    pub final_findings: usize,
}

fn median(v: &[usize]) -> usize {
    if v.is_empty() {
        return 0;
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    s[s.len() / 2]
}

fn gzip_len(b: &[u8]) -> usize {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).expect("gzip");
    e.finish().expect("gzip").len()
}

/// Run one workload through one model.
pub fn run<M: Model>(w: &Workload, doc: &qp_schema::Document) -> RunResult {
    let mut m = M::new(doc);
    let mut idle = 0;
    let mut locally_invalid = 0;
    let mut applied = 0;
    let mut retried = 0;
    let mut refused = 0;
    let mut codes: BTreeMap<String, usize> = BTreeMap::new();
    let mut t = Duration::ZERO;
    let mut reconnect = Reconnect::default();
    let mut reconnected = false;

    // Vector clocks per replica (for model-independent concurrency accounting).
    let mut vc: [Vc; 3] = [[0; 3]; 3];
    let mut writes: Vec<WriteRec> = Vec::new();

    let mut i = 0;
    for round in 0..w.rounds {
        let editor_online = !(w.offline_from <= round && round < w.offline_to);
        if round == w.offline_to && !reconnected && w.offline_to > w.offline_from {
            let s = Instant::now();
            reconnect = m.reconnect(Who::Editor);
            t += s.elapsed();
            reconnected = true;
            let all = vc_max(&vc[0], &vc_max(&vc[1], &vc[2]));
            vc = [all; 3];
        }
        let online = [true, true, editor_online];
        while i < w.intents.len() && w.intents[i].round == round {
            let it = &w.intents[i];
            i += 1;
            let view = m.view(it.who);
            let Some(op) = concretize(&view, it) else {
                idle += 1;
                continue;
            };
            if validate(&view, it.who, &op).is_err() {
                locally_invalid += 1;
                continue;
            }
            let s = Instant::now();
            let exp = m.write(it.who, it, &op, online[it.who.idx()], &view);
            t += s.elapsed();
            vc[it.who.idx()][it.who.idx()] += 1;
            match &exp {
                Exp::Applied | Exp::Queued => {
                    applied += 1;
                    writes.push(WriteRec {
                        who: it.who,
                        vc: vc[it.who.idx()],
                        op: op.clone(),
                    });
                }
                Exp::AppliedAfterRetry(cs) => {
                    retried += 1;
                    for c in cs {
                        *codes.entry(format!("{c:?}")).or_default() += 1;
                    }
                }
                Exp::Refused(cs) => {
                    refused += 1;
                    for c in cs {
                        *codes.entry(format!("{c:?}")).or_default() += 1;
                    }
                }
            }
        }
        let s = Instant::now();
        m.end_round(online);
        t += s.elapsed();
        // Online replicas now know each other's writes.
        let mut on = [0u32; 3];
        for who in ALL {
            if online[who.idx()] {
                on = vc_max(&on, &vc[who.idx()]);
            }
        }
        for who in ALL {
            if online[who.idx()] {
                vc[who.idx()] = on;
            }
        }
    }
    if !reconnected && w.offline_to > w.offline_from {
        let s = Instant::now();
        reconnect = m.reconnect(Who::Editor);
        t += s.elapsed();
    }
    let s = Instant::now();
    m.end_round([true; 3]);
    t += s.elapsed();

    let snaps = m.snaps();
    let first = snaps[0].to_json();
    let converged = snaps.iter().all(|s| s.to_json() == first);
    let an = anomalies(&snaps[0]);
    let hist = m.history_bytes();
    let wire = m.wire().clone();
    RunResult {
        model: M::NAME.to_string(),
        ops: w.intents.len(),
        window: w.offline_to - w.offline_from,
        idle,
        locally_invalid,
        applied,
        applied_after_retry: retried,
        refused,
        refused_codes: codes,
        converged,
        anomaly_total: an.total(),
        anomalies: an,
        concurrency: concurrency_stats(&writes),
        reconnect,
        wire_up_mean: wire.up.iter().sum::<usize>() as f64 / wire.up.len().max(1) as f64,
        wire_up_median: median(&wire.up),
        wire_down_mean: wire.down.iter().sum::<usize>() as f64 / wire.down.len().max(1) as f64,
        wire_down_median: median(&wire.down),
        wire_up_total: wire.up.iter().sum(),
        wire_down_total: wire.down.iter().sum(),
        replies_total: wire.replies,
        history_bytes: hist.len(),
        history_gzip: gzip_len(&hist),
        state_bytes: m.state_bytes(),
        replicate_time_us: t.as_secs_f64() * 1e6,
        history_sha: crate::common::sha(&hist),
        final_snap_sha: crate::common::sha(first.as_bytes()),
        undo: m.undo_stats().clone(),
        extra: m.extra(),
        final_layers: snaps[0].layers.len(),
        final_series: snaps[0].series.len(),
        final_findings: snaps[0].findings.len(),
    }
}

/// A scripted step for the named cases.
#[derive(Debug, Clone)]
pub enum Step {
    Write(Who, SemOp),
    EndRound { editor_online: bool },
    Reconnect,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub model: String,
    pub experiences: Vec<(String, String, String)>,
    pub reconnect: Reconnect,
    pub converged: bool,
    pub final_snap: Snap,
    pub anomalies: Anomalies,
    pub undo: UndoStats,
    pub extra: BTreeMap<String, usize>,
}

/// Run a scripted case. Writes are validated against the writer's view first; a write
/// that fails local validation is recorded as such and not sent.
pub fn run_case<M: Model>(steps: &[Step], doc: &qp_schema::Document) -> (CaseResult, M) {
    let mut m = M::new(doc);
    let mut exps = Vec::new();
    let mut editor_online = true;
    let mut reconnect = Reconnect::default();
    let dummy = Intent {
        round: 0,
        who: Who::Arkin,
        n: 0,
        r: [0; 5],
    };
    for st in steps {
        match st {
            Step::Write(who, op) => {
                let view = m.view(*who);
                if let Err(c) = validate(&view, *who, op) {
                    exps.push((who.name().into(), op.kind().into(), format!("local {c:?}")));
                    continue;
                }
                let online = *who != Who::Editor || editor_online;
                let e = m.write(*who, &dummy, op, online, &view);
                exps.push((who.name().into(), op.kind().into(), format!("{e:?}")));
            }
            Step::EndRound { editor_online: eo } => {
                editor_online = *eo;
                m.end_round([true, true, *eo]);
            }
            Step::Reconnect => {
                reconnect = m.reconnect(Who::Editor);
                editor_online = true;
                m.end_round([true; 3]);
            }
        }
    }
    m.end_round([true; 3]);
    let snaps = m.snaps();
    let first = snaps[0].to_json();
    let r = CaseResult {
        model: M::NAME.to_string(),
        experiences: exps,
        reconnect,
        converged: snaps.iter().all(|s| s.to_json() == first),
        anomalies: anomalies(&snaps[0]),
        final_snap: snaps[0].clone(),
        undo: m.undo_stats().clone(),
        extra: m.extra(),
    };
    (r, m)
}
