//! The output cursor (PR §7, RX5 §7/2): the value of an output at bar `i`, the events and
//! signal records born at bar `i`, a state key, and the provenance of each (PR §9). Reads
//! are O(1) or O(log n) — never a scan per birth — and each one is counted for the budget.

use qp_schema::program::{ProvenanceData, ValueProvenance};
use qp_schema::Bar;

use super::record::{OutRec, SignalHit};
use super::{Executor, Stopped};

/// Where a published value comes from: its output, node and base bars `[lo, bar]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    /// Output name.
    pub output: String,
    /// The node it reads (a state's first dimension).
    pub node: String,
    /// Bar index.
    pub bar: u64,
    /// The first base bar it depends on.
    pub lo: u64,
}

impl Executor {
    fn read(&self) {
        self.reads.set(self.reads.get().saturating_add(1));
    }

    /// Output names in run order (the indexes the cursor takes).
    #[must_use]
    pub fn outputs(&self) -> Vec<&str> {
        self.plan.outs.iter().map(|o| o.name.as_str()).collect()
    }

    /// The cursor index of output `name`.
    #[must_use]
    pub fn output(&self, name: &str) -> Option<usize> {
        self.plan.outs.iter().position(|o| o.name == name)
    }

    /// Closed bars recorded (a budget stop ends it early).
    #[must_use]
    pub fn closed_len(&self) -> usize {
        self.t.len()
    }

    /// The live bar, if any (its index is [`Executor::closed_len`]).
    #[must_use]
    pub fn live_bar(&self) -> Option<&Bar> {
        self.live.as_ref()
    }

    /// Open time of closed bar `i`.
    #[must_use]
    pub fn t_at(&self, i: usize) -> Option<i64> {
        self.t.get(i).copied()
    }

    /// Close time (`known_t`, PR §1) of closed bar `i`: the calendar close, `t` + the
    /// interval in UTC, or the next bar's `t` without an interval (`None` until known).
    #[must_use]
    pub fn known_at(&self, i: usize) -> Option<i64> {
        self.known.get(i).copied().flatten()
    }

    /// Value of output `o` at bar `i`: a series value (the transient value on the live
    /// bar), 1/0 for an event or signal on a closed bar (`None` on the live bar), `None`
    /// for a state (see [`Executor::state_at`]).
    #[must_use]
    pub fn value_at(&self, o: usize, i: usize) -> Option<f64> {
        self.read();
        let live = i == self.t.len() && self.live.is_some();
        match self.rec.get(o)? {
            OutRec::Series { v, .. } if live => self.live_vals.get(o).copied().flatten(),
            OutRec::Series { v, .. } => v.get(i).copied().flatten(),
            OutRec::Event { v, .. } => v.get(i).map(|f| if *f { 1.0 } else { 0.0 }),
            OutRec::Signal { hits, .. } => {
                (i < self.t.len()).then(|| f64::from(u8::from(born_at(hits, i as u64).is_some())))
            }
            OutRec::State { .. } => None,
        }
    }

    /// Output `o`'s value at bar `i` as the document series holds it (PS §1.2): a series as
    /// [`Executor::value_at`]; an event 1/0 on closed bars, `0` where `known_t ≤ since_t`
    /// (no event before the instance existed), `None` on the live bar.
    #[must_use]
    pub fn doc_value_at(&self, o: usize, i: usize, since_t: Option<i64>) -> Option<f64> {
        let v = self.value_at(o, i)?;
        match (self.rec.get(o)?, since_t, self.known_at(i)) {
            (OutRec::Event { .. }, Some(s), Some(k)) if k <= s => Some(0.0),
            _ => Some(v),
        }
    }

    /// The event outputs with an event at closed bar `i`.
    #[must_use]
    pub fn events_at(&self, i: usize) -> Vec<usize> {
        self.read();
        self.rec
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, OutRec::Event { v, .. } if v.get(i) == Some(&true)))
            .map(|(o, _)| o)
            .collect()
    }

    /// The signal record of output `o` born at closed bar `i`, if any.
    #[must_use]
    pub fn signals_at(&self, o: usize, i: usize) -> Option<&SignalHit> {
        self.read();
        match self.rec.get(o)? {
            OutRec::Signal { hits, .. } => born_at(hits, i as u64),
            _ => None,
        }
    }

    /// The key of state output `o` at closed bar `i` (`None` when a dimension is undefined).
    #[must_use]
    pub fn state_at(&self, o: usize, i: usize) -> Option<&[u32]> {
        self.read();
        match self.rec.get(o)? {
            OutRec::State { keys, .. } => keys.get(i)?.as_deref(),
            _ => None,
        }
    }

    /// The provenance of output `o`'s value at closed bar `i` (PR §9). For a signal, the
    /// record born at `i` (its outcome rests on `[i + 1, exit_bar]` besides).
    #[must_use]
    pub fn lineage(&self, o: usize, i: usize) -> Option<Lineage> {
        self.read();
        let out = self.plan.outs.get(o)?;
        let lo = match self.rec.get(o)? {
            OutRec::Series { lo, .. } | OutRec::Event { lo, .. } | OutRec::State { lo, .. } => {
                *lo.get(i)?
            }
            OutRec::Signal { hits, .. } => born_at(hits, i as u64)?.lo,
        };
        Some(Lineage {
            output: out.name.clone(),
            node: self.plan.nodes.get(out.node)?.id.clone(),
            bar: i as u64,
            lo,
        })
    }

    /// [`Executor::lineage`] in the protocol's form, with the program and data it rests on.
    #[must_use]
    pub fn provenance(
        &self,
        o: usize,
        i: usize,
        program_id: &str,
        data: ProvenanceData,
    ) -> Option<ValueProvenance> {
        let l = self.lineage(o, i)?;
        Some(ValueProvenance {
            output: l.output,
            node: l.node,
            bar: l.bar,
            t: self.t_at(i)?,
            range: [l.lo, l.bar],
            program_id: program_id.to_owned(),
            data,
        })
    }

    /// Work used: closed bars charged plus reads (PR §8.3; 0 without a budget).
    #[must_use]
    pub fn work(&self) -> u64 {
        let read = self.budget.map_or(0, |b| b.read);
        self.work
            .saturating_add(self.reads.get().saturating_mul(read))
    }

    /// Where the budget stopped the run, if it did.
    #[must_use]
    pub fn stopped(&self) -> Option<Stopped> {
        self.stopped
    }
}

/// The record born at bar `i` (records are in bar order, at most one per bar).
fn born_at(hits: &[SignalHit], i: u64) -> Option<&SignalHit> {
    hits.binary_search_by_key(&i, |h| h.bar)
        .ok()
        .map(|k| &hits[k])
}

impl Executor {
    /// A canonical text of everything published on closed bars (and the live bar's
    /// transient values): one line per output and bar, numbers as their IEEE bits, with
    /// provenance. Two runs are byte-equal exactly when these texts are; the determinism
    /// digest hashes it. Not counted as reads.
    #[must_use]
    pub fn canonical(&self) -> String {
        use std::fmt::Write as _;
        let num =
            |v: Option<f64>| v.map_or_else(|| "-".to_owned(), |x| format!("{:016x}", x.to_bits()));
        let mut s = String::new();
        for (o, (out, rec)) in self.plan.outs.iter().zip(&self.rec).enumerate() {
            let _ = writeln!(s, "output {} {}", o, out.name);
            match rec {
                OutRec::Series { v, lo, .. } => {
                    for (i, (x, l)) in v.iter().zip(lo).enumerate() {
                        let _ = writeln!(s, "{i} {} {} {l}", self.t[i], num(*x));
                    }
                }
                OutRec::Event { v, lo, .. } => {
                    for (i, (f, l)) in v.iter().zip(lo).enumerate() {
                        let _ = writeln!(s, "{i} {} {} {l}", self.t[i], u8::from(*f));
                    }
                }
                OutRec::Signal { hits, .. } => {
                    for h in hits {
                        let _ = writeln!(
                            s,
                            "{} {} {} {:?} {:?} {} {}",
                            h.bar,
                            num(Some(h.entry)),
                            h.lo,
                            h.state,
                            h.outcome,
                            h.exit_bar.map_or_else(|| "-".to_owned(), |b| b.to_string()),
                            num(h.exit_price),
                        );
                    }
                }
                OutRec::State { keys, lo, .. } => {
                    for (i, (k, l)) in keys.iter().zip(lo).enumerate() {
                        let _ = writeln!(s, "{i} {} {k:?} {l}", self.t[i]);
                    }
                }
            }
            if self.live.is_some() {
                let _ = writeln!(s, "live {}", num(self.live_vals.get(o).copied().flatten()));
            }
        }
        for (i, k) in self.known.iter().enumerate() {
            let _ = writeln!(s, "known {i} {k:?}");
        }
        if let Some(st) = self.stopped {
            let _ = writeln!(s, "stopped {st:?}");
        }
        s
    }
}
