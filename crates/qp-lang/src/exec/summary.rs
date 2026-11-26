//! The `try_program` summary (PS §3.2 `program_try`; PR §8.3 replay): a pure function of
//! the checked instance, the closed bars and the limits, so `qp-program`, MCP and the CLI
//! only call it. It runs a closed window under the work budget and lists, per output,
//! bounded counts and items (`program.try.max_items`); signal records come out in the PR
//! §6 form (`mode: "replay"`, `label: "simulated"`).

use std::collections::BTreeMap;

use qp_schema::data::FeedKey;
use qp_schema::program::{
    EventAt, IntArg, Num, OutcomeCounts, OutcomeKind, OutcomeRule, ProgramTry, RecordEvidence,
    RecordLabel, RecordMode, RecordProvenance, RecordSourceKind, RecordState, SignalOutcome,
    SignalRecord, SignalSource, StateCount, StopReason, TryEvent, TrySeries, TrySignal, TryState,
    TryStopped, TryWork,
};
use qp_schema::{Author, Bar, Point};

use super::plan::{OutKind, POut};
use super::record::{OutRec, SignalHit};
use super::{Align, ExecError, ExecOptions, Executor, WorkBudget};
use crate::checked::Checked;
use crate::limits::{Limit, Limits};

/// What a signal record says about where it comes from — the parts the executor does not
/// know (PR §6 `provenance`, `instrument`, `period`, `evidence`).
#[derive(Debug, Clone, PartialEq)]
pub struct RecordCtx {
    /// Instance name (`<instance>.<output>@<born_t>` is the record id).
    pub instance: String,
    /// The instance's owner.
    pub owner: String,
    /// Who wrote the program.
    pub author: Author,
    /// Instrument symbol.
    pub instrument: String,
    /// Bar interval.
    pub period: String,
    /// The bound bar series id.
    pub series: String,
    /// Its anchor (absent for an inline series).
    pub anchor_t: Option<i64>,
    /// Its bound feeds.
    pub feeds: Vec<FeedKey>,
}

/// One try or replay run.
#[derive(Debug, Clone)]
pub struct TryRequest<'a> {
    /// The checked instance.
    pub checked: &'a Checked,
    /// The bound series' closed bars, ascending (the live bar excluded).
    pub bars: &'a [Bar],
    /// First bar time to read (inclusive), if limited.
    pub from_t: Option<i64>,
    /// Last bar time to read (inclusive), if limited.
    pub to_t: Option<i64>,
    /// Bar boundaries of higher timeframes.
    pub align: Align,
    /// Limits in force (`program.try.*`, `program.work.*`).
    pub limits: Limits,
    /// Record context.
    pub ctx: RecordCtx,
}

/// The bars a try reads: those in `[from_t, to_t]`, the latest `program.try.max_bars`.
#[must_use]
pub fn try_window<'b>(req: &TryRequest<'b>) -> &'b [Bar] {
    let lo = req
        .from_t
        .map_or(0, |f| req.bars.partition_point(|b| b.t < f));
    let hi = req
        .to_t
        .map_or(req.bars.len(), |t| req.bars.partition_point(|b| b.t <= t));
    let all = &req.bars[lo..hi.max(lo)];
    let max = usize::try_from(req.limits.get(Limit::TryMaxBars)).unwrap_or(usize::MAX);
    &all[all.len().saturating_sub(max)..]
}

/// Runs a try (closed window, work budget) and summarizes it.
///
/// # Errors
/// As [`Executor::new`], or bars not strictly ascending.
pub fn try_program(req: &TryRequest<'_>) -> Result<ProgramTry, ExecError> {
    let bars = try_window(req);
    let budget = WorkBudget::from_limits(&req.limits);
    let opts = ExecOptions {
        align: req.align.clone(),
        outputs: None,
        budget: Some(budget),
    };
    let x = Executor::run_closed(req.checked, bars, opts)?;
    let items = usize::try_from(req.limits.get(Limit::TryMaxItems)).unwrap_or(usize::MAX);
    let mut s = ProgramTry {
        from_t: bars.first().map(|b| b.t),
        to_t: x.closed_len().checked_sub(1).and_then(|i| x.t_at(i)),
        series: Vec::new(),
        event: Vec::new(),
        signal: Vec::new(),
        state: Vec::new(),
        dropped: x.dropped(),
        cost: req.checked.cost.clone(),
        work: TryWork {
            used: 0,
            limit: budget.max_work,
        },
        stopped: None,
    };
    for (o, out) in x.plan.outs.iter().enumerate() {
        match &x.rec[o] {
            OutRec::Series { v, defined, .. } => s.series.push(x.series(out, v, *defined, items)),
            OutRec::Event { .. } => s.event.push(x.event(out, &x.rec[o], items)),
            OutRec::Signal { hits, .. } => s.signal.push(x.signal(req, out, hits, items)),
            OutRec::State { counts, .. } => s.state.push(x.state(out, counts, items)),
        }
    }
    s.work.used = x.work();
    s.stopped = x.stopped().map(|st| TryStopped {
        reason: StopReason::Budget,
        bar: st.bar,
        t: st.t,
        work: st.work,
        limit: st.limit,
    });
    Ok(s)
}

impl Executor {
    /// `items` reads charged at once.
    fn read_n(&self, n: usize) {
        self.reads.set(self.reads.get().saturating_add(n as u64));
    }

    fn event_at(&self, i: u64) -> EventAt {
        let ix = usize::try_from(i).unwrap_or(usize::MAX);
        let t = self.t_at(ix).unwrap_or(0);
        EventAt {
            t,
            known_t: self.known_at(ix).unwrap_or(t),
        }
    }

    fn series(&self, out: &POut, v: &[Option<f64>], defined: u64, items: usize) -> TrySeries {
        let from = v.len().saturating_sub(items);
        self.read_n(v.len() - from);
        TrySeries {
            name: out.name.clone(),
            defined,
            last: (from..v.len())
                .map(|i| Point {
                    t: self.t[i],
                    v: v[i],
                })
                .collect(),
        }
    }

    fn event(&self, out: &POut, rec: &OutRec, items: usize) -> TryEvent {
        let OutRec::Event {
            bars,
            runs,
            isolated,
            run,
            ..
        } = rec
        else {
            return TryEvent {
                name: out.name.clone(),
                events: 0,
                runs: 0,
                notifications: 0,
                consecutive: false,
                isolated: 0,
                first: Vec::new(),
                last: Vec::new(),
            };
        };
        let first: Vec<EventAt> = bars.iter().take(items).map(|&i| self.event_at(i)).collect();
        let tail = bars.len().saturating_sub(items);
        let last: Vec<EventAt> = bars[tail..].iter().map(|&i| self.event_at(i)).collect();
        self.read_n(first.len() + last.len());
        let events = bars.len() as u64;
        TryEvent {
            name: out.name.clone(),
            events,
            runs: *runs,
            notifications: *runs,
            consecutive: *runs < events,
            // A run still open at the window's end counts once it is known to be single.
            isolated: *isolated + u64::from(*run == 1),
            first,
            last,
        }
    }

    fn signal(
        &self,
        req: &TryRequest<'_>,
        out: &POut,
        hits: &[SignalHit],
        items: usize,
    ) -> TrySignal {
        let mut outcomes = OutcomeCounts::default();
        for h in hits {
            match h.outcome {
                OutcomeKind::Target => outcomes.target += 1,
                OutcomeKind::Stop => outcomes.stop += 1,
                OutcomeKind::Expired => outcomes.expired += 1,
                OutcomeKind::Open => outcomes.open += 1,
            }
        }
        let first: Vec<SignalRecord> = hits
            .iter()
            .take(items)
            .filter_map(|h| self.signal_record(req.checked, out, h, &req.ctx))
            .collect();
        self.read_n(first.len());
        TrySignal {
            name: out.name.clone(),
            count: hits.len() as u64,
            outcomes,
            first,
        }
    }

    fn state(&self, out: &POut, counts: &BTreeMap<Vec<u32>, u64>, items: usize) -> TryState {
        let mut top: Vec<(&Vec<u32>, u64)> = counts.iter().map(|(k, n)| (k, *n)).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        top.truncate(items);
        self.read_n(top.len());
        TryState {
            name: out.name.clone(),
            distinct: counts.len() as u64,
            top: top
                .into_iter()
                .map(|(k, count)| StateCount {
                    key: self.key_map(out, k),
                    count,
                })
                .collect(),
        }
    }

    /// Dimension name → bin index.
    fn key_map(&self, out: &POut, key: &[u32]) -> BTreeMap<String, u32> {
        match &out.kind {
            OutKind::State { dims } => dims.iter().cloned().zip(key.iter().copied()).collect(),
            _ => BTreeMap::new(),
        }
    }

    /// The record of signal output `o` born at closed bar `i`, in the PR §6 form
    /// (`mode: "replay"`, `label: "simulated"`).
    #[must_use]
    pub fn record_at(
        &self,
        checked: &Checked,
        o: usize,
        i: usize,
        ctx: &RecordCtx,
    ) -> Option<SignalRecord> {
        let h = self.signals_at(o, i)?;
        self.signal_record(checked, self.plan.outs.get(o)?, h, ctx)
    }

    /// A signal record in the PR §6 form.
    fn signal_record(
        &self,
        checked: &Checked,
        out: &POut,
        h: &SignalHit,
        ctx: &RecordCtx,
    ) -> Option<SignalRecord> {
        let OutKind::Signal { side, rule, source } = &out.kind else {
            return None;
        };
        let at = |i: u64| usize::try_from(i).ok().and_then(|i| self.t_at(i));
        let born_t = at(h.bar)?;
        let state = self.plan.first_state.and_then(|s| {
            let so = &self.plan.outs[s];
            Some(RecordState {
                output: so.name.clone(),
                key: self.key_map(so, h.state.as_deref()?),
            })
        });
        Some(SignalRecord {
            id: format!("{}.{}@{born_t}", ctx.instance, out.name),
            source_kind: RecordSourceKind::Signal,
            mode: RecordMode::Replay,
            label: Some(RecordLabel::Simulated),
            provenance: provenance(checked, source, ctx),
            instrument: ctx.instrument.clone(),
            period: ctx.period.clone(),
            born_t,
            known_t: usize::try_from(h.bar)
                .ok()
                .and_then(|i| self.known_at(i))
                .unwrap_or(born_t),
            side: *side,
            entry: h.entry,
            rule: OutcomeRule {
                target_pct: rule.target_pct.map(Num::Lit),
                stop_pct: rule.stop_pct.map(Num::Lit),
                max_bars: IntArg::Lit(i64::try_from(rule.max_bars).unwrap_or(i64::MAX)),
            },
            state,
            outcome: SignalOutcome {
                kind: h.outcome,
                exit_t: h.exit_bar.and_then(at),
                exit_price: h.exit_price,
            },
            evidence: RecordEvidence {
                series: ctx.series.clone(),
                from_t: at(h.lo).unwrap_or(born_t),
                to_t: born_t,
                anchor_t: ctx.anchor_t,
                feeds: ctx.feeds.clone(),
            },
        })
    }
}

fn provenance(checked: &Checked, source: &SignalSource, ctx: &RecordCtx) -> RecordProvenance {
    RecordProvenance {
        owner: ctx.owner.clone(),
        author: ctx.author.clone(),
        program_id: checked.program_id.clone(),
        instance_id: checked.instance_id.clone(),
        params: checked.params.clone(),
        strategy: source.strategy.clone(),
        program_version: source.version.clone(),
    }
}
