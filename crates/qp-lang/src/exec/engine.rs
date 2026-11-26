//! The streaming core (PR §7): closed bars are pushed one at a time (higher bars first,
//! then the base clock, each node evaluated and committed in topological order — an
//! iterative walk, no recursion); the live bar is a pure evaluation on top of the
//! committed state (no copy). Events, signals and records are only born on closed bars.

use std::cell::Cell;

use qp_schema::Bar;

use super::node::{step, Ctx, In, State, LO_FREE};
use super::plan::{OutKind, PNode, Plan};
use super::record::{OutRec, Staged};
use super::tf::{Align, TfBuilder};
use super::{ExecError, ExecOptions, Stopped, WorkBudget};
use crate::checked::Checked;

/// A running program instance (see the module docs of `exec`).
#[derive(Debug)]
pub struct Executor {
    pub(super) plan: Plan,
    states: Vec<State>,
    vals: Vec<Option<f64>>,
    los: Vec<u64>,
    align: Align,
    tf: Vec<TfBuilder>,
    /// Closed bars: open time, close time (`known_t`; `None` until known).
    pub(super) t: Vec<i64>,
    pub(super) known: Vec<Option<i64>>,
    pub(super) rec: Vec<OutRec>,
    pub(super) live: Option<Bar>,
    /// Transient value of each series output on the live bar.
    pub(super) live_vals: Vec<Option<f64>>,
    anchor: Option<i64>,
    pub(super) budget: Option<WorkBudget>,
    /// Work charged for closed bars (reads are counted apart).
    pub(super) work: u64,
    pub(super) reads: Cell<u64>,
    pub(super) stopped: Option<Stopped>,
}

/// Runs `group`'s nodes on one bar of their clock.
fn run_nodes(
    nodes: &[PNode],
    group: &[usize],
    states: &mut [State],
    vals: &mut [Option<f64>],
    los: &mut [u64],
    cx: &Ctx<'_>,
    commit: bool,
) {
    for &j in group {
        let n = &nodes[j];
        let mut v = [None; 2];
        let mut l = [LO_FREE; 2];
        for (k, &s) in n.ins.iter().take(2).enumerate() {
            v[k] = vals[s];
            l[k] = los[s];
        }
        let (_, out) = vals.split_at_mut(n.out0);
        let (_, out_lo) = los.split_at_mut(n.out0);
        step(
            &n.op,
            &mut states[j],
            &In { v: &v, lo: &l },
            cx,
            &mut out[..n.n_out],
            &mut out_lo[..n.n_out],
            commit,
        );
    }
}

/// `lo` as published at bar `i`: a clockless value rests on bar `i` itself.
fn publish_lo(lo: u64, i: u64) -> u64 {
    lo.min(i)
}

impl Executor {
    /// A fresh instance of `c` (no bars yet).
    ///
    /// # Errors
    /// A higher timeframe without a bound base interval, an unknown output name, or a
    /// kernel the compute core refuses.
    pub fn new(c: &Checked, opts: ExecOptions) -> Result<Executor, ExecError> {
        let plan = Plan::new(c, opts.outputs.as_deref())?;
        let mut states = Vec::with_capacity(plan.nodes.len());
        let active: Vec<bool> = {
            let mut a = vec![false; plan.nodes.len()];
            for &j in plan.free.iter().chain(&plan.base) {
                a[j] = true;
            }
            for g in &plan.tfs {
                g.nodes.iter().for_each(|&j| a[j] = true);
            }
            a
        };
        for (n, on) in plan.nodes.iter().zip(&active) {
            states.push(if *on {
                State::new(&n.op).map_err(|e| ExecError::Kernel(n.id.clone(), e.to_string()))?
            } else {
                State::None
            });
        }
        let rec = plan.outs.iter().map(|o| empty_rec(&o.kind)).collect();
        let mut x = Executor {
            vals: vec![None; plan.n_slots],
            los: vec![LO_FREE; plan.n_slots],
            tf: plan.tfs.iter().map(|g| TfBuilder::new(g.iv)).collect(),
            live_vals: vec![None; plan.outs.len()],
            states,
            align: opts.align,
            t: Vec::new(),
            known: Vec::new(),
            rec,
            live: None,
            anchor: None,
            budget: opts.budget,
            work: 0,
            reads: Cell::new(0),
            stopped: None,
            plan,
        };
        // Clockless nodes are constants: evaluated once.
        let zero = Bar {
            t: 0,
            o: 0.0,
            h: 0.0,
            l: 0.0,
            c: 0.0,
            v: 0.0,
        };
        let cx = Ctx {
            i: 0,
            bar: &zero,
            lo: LO_FREE,
        };
        run_nodes(
            &x.plan.nodes,
            &x.plan.free,
            &mut x.states,
            &mut x.vals,
            &mut x.los,
            &cx,
            false,
        );
        Ok(x)
    }

    /// Runs `bars` as a closed window (every bar closed): the batch mode, which is
    /// byte-equal to pushing them one by one.
    ///
    /// # Errors
    /// As [`Executor::new`], or bars not strictly ascending.
    pub fn run_closed(c: &Checked, bars: &[Bar], opts: ExecOptions) -> Result<Executor, ExecError> {
        let mut x = Executor::new(c, opts)?;
        for b in bars {
            x.push_closed(b.clone())?;
        }
        Ok(x)
    }

    fn last_t(&self) -> Option<i64> {
        self.live
            .as_ref()
            .map(|b| b.t)
            .or_else(|| self.t.last().copied())
    }

    fn ascending(&self, t: i64) -> Result<(), ExecError> {
        match self.last_t() {
            Some(last) if t <= last => Err(ExecError::NotAscending { t, last }),
            _ => Ok(()),
        }
    }

    /// Appends a closed bar (a live bar, if any, closes first).
    ///
    /// # Errors
    /// `bar.t` not after the last bar.
    pub fn push_closed(&mut self, bar: Bar) -> Result<(), ExecError> {
        self.ascending(bar.t)?;
        self.close_live();
        self.close_bar(&bar);
        Ok(())
    }

    /// A new bar: the live bar (if any) closes, `bar` becomes the live bar and its
    /// transient values are computed (PR §7 `push`).
    ///
    /// # Errors
    /// `bar.t` not after the last bar.
    pub fn push(&mut self, bar: Bar) -> Result<(), ExecError> {
        self.ascending(bar.t)?;
        self.close_live();
        self.live = Some(bar);
        self.peek();
        Ok(())
    }

    /// Replaces the live bar (same `t`) and recomputes its transient values; without a
    /// live bar it is [`Executor::push`].
    ///
    /// # Errors
    /// A live bar with another `t`.
    pub fn update_last(&mut self, bar: Bar) -> Result<(), ExecError> {
        match &self.live {
            Some(l) if l.t != bar.t => Err(ExecError::NotLive {
                t: bar.t,
                live: l.t,
            }),
            Some(_) => {
                self.live = Some(bar);
                self.peek();
                Ok(())
            }
            None => self.push(bar),
        }
    }

    /// Closes the live bar, if any (the series became a closed window, `window.to_t`).
    pub fn close_live(&mut self) {
        if let Some(b) = self.live.take() {
            self.live_vals.iter_mut().for_each(|v| *v = None);
            self.close_bar(&b);
        }
    }

    /// Transient values of the live bar: base nodes evaluated without commit.
    fn peek(&mut self) {
        let Some(bar) = self.live.clone() else {
            return;
        };
        let i = self.t.len() as u64;
        if let Some(k) = self.known.last_mut().filter(|k| k.is_none()) {
            *k = Some(bar.t);
        }
        if self.stopped.is_some() {
            return;
        }
        let cx = Ctx {
            i,
            bar: &bar,
            lo: i,
        };
        run_nodes(
            &self.plan.nodes,
            &self.plan.base,
            &mut self.states,
            &mut self.vals,
            &mut self.los,
            &cx,
            false,
        );
        for (o, v) in self.plan.outs.iter().zip(self.live_vals.iter_mut()) {
            *v = match o.kind {
                OutKind::Series => self.vals[o.slots[0]],
                _ => None,
            };
        }
    }

    /// One closed base bar: higher bars it closes, the base clock, the budget, then the
    /// records.
    fn close_bar(&mut self, bar: &Bar) {
        if self.stopped.is_some() {
            return;
        }
        let i = self.t.len() as u64;
        if let Some(k) = self.known.last_mut().filter(|k| k.is_none()) {
            *k = Some(bar.t);
        }
        let anchor = *self.anchor.get_or_insert(bar.t);
        let close = self.plan.base_iv.map(|iv| self.align.close_of(iv, bar.t));
        self.higher(bar, i, close.unwrap_or(bar.t), anchor);
        let cx = Ctx { i, bar, lo: i };
        run_nodes(
            &self.plan.nodes,
            &self.plan.base,
            &mut self.states,
            &mut self.vals,
            &mut self.los,
            &cx,
            true,
        );
        let staged = self.stage(i);
        if !self.charge(i, bar.t, &staged) {
            return;
        }
        self.t.push(bar.t);
        self.known.push(close);
        let key = self.plan.first_state.and_then(|s| match &staged[s] {
            Staged::Key(k, _) => k.clone(),
            _ => None,
        });
        for (r, s) in self.rec.iter_mut().zip(staged) {
            r.push(i, bar, s, key.as_ref());
        }
    }

    /// Feeds every higher timeframe; runs each closed higher bar's nodes.
    fn higher(&mut self, bar: &Bar, i: u64, close: i64, anchor: i64) {
        for g in 0..self.tf.len() {
            for tb in self.tf[g]
                .feed(&self.align, bar, i, close, anchor)
                .into_iter()
                .flatten()
            {
                let cx = Ctx {
                    i: tb.k,
                    bar: &tb.bar,
                    lo: tb.first,
                };
                run_nodes(
                    &self.plan.nodes,
                    &self.plan.tfs[g].nodes,
                    &mut self.states,
                    &mut self.vals,
                    &mut self.los,
                    &cx,
                    true,
                );
            }
        }
    }

    /// What each output gives at closed bar `i`.
    fn stage(&self, i: u64) -> Vec<Staged> {
        let truthy = |v: Option<f64>| v.is_some_and(|x| x != 0.0);
        self.plan
            .outs
            .iter()
            .map(|o| {
                let s = o.slots[0];
                match o.kind {
                    OutKind::Series => Staged::Value(self.vals[s], publish_lo(self.los[s], i)),
                    OutKind::Event | OutKind::Signal { .. } => {
                        Staged::Flag(truthy(self.vals[s]), publish_lo(self.los[s], i))
                    }
                    OutKind::State { .. } => {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let key: Option<Vec<u32>> = o
                            .slots
                            .iter()
                            .map(|&d| self.vals[d].map(|v| v as u32))
                            .collect();
                        let lo = o
                            .slots
                            .iter()
                            .map(|&d| self.los[d])
                            .min()
                            .unwrap_or(LO_FREE);
                        Staged::Key(key, publish_lo(lo, i))
                    }
                }
            })
            .collect()
    }

    /// Charges closed bar `i` (PR §8.3); `false` (and stopped) when it would overrun.
    fn charge(&mut self, i: u64, t: i64, staged: &[Staged]) -> bool {
        let Some(b) = self.budget else {
            return true;
        };
        let born: u64 = staged.iter().map(Staged::born).sum();
        let open: u64 = self.rec.iter().map(OutRec::open).sum();
        let inc = self
            .plan
            .per_bar
            .saturating_add(born.saturating_mul(b.record))
            .saturating_add(open.saturating_mul(b.open_record));
        let total = self
            .work
            .saturating_add(inc)
            .saturating_add(self.reads.get().saturating_mul(b.read));
        if total > b.max_work {
            self.stopped = Some(Stopped {
                bar: i,
                t,
                work: total,
                limit: b.max_work,
            });
            return false;
        }
        self.work = self.work.saturating_add(inc);
        true
    }

    /// Base bars in no session (the largest count over the higher timeframes).
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.tf.iter().map(|b| b.dropped).max().unwrap_or(0)
    }
}

fn empty_rec(kind: &OutKind) -> OutRec {
    match kind {
        OutKind::Series => OutRec::Series {
            v: Vec::new(),
            lo: Vec::new(),
            defined: 0,
        },
        OutKind::Event => OutRec::Event {
            v: Vec::new(),
            lo: Vec::new(),
            bars: Vec::new(),
            runs: 0,
            isolated: 0,
            run: 0,
        },
        OutKind::Signal { side, rule, .. } => OutRec::Signal {
            side: *side,
            rule: *rule,
            hits: Vec::new(),
            levels: Vec::new(),
            open: Vec::new(),
        },
        OutKind::State { .. } => OutRec::State {
            keys: Vec::new(),
            lo: Vec::new(),
            counts: std::collections::BTreeMap::new(),
        },
    }
}
