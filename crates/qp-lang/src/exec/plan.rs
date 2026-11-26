//! The executor's plan of a checked program: one flat value slot per node output, input
//! slots resolved, nodes grouped by clock in topological order (clockless ones once, each
//! higher timeframe when its bar closes, the base clock on every base bar), restricted to
//! the cones of the outputs asked for (PS §1.3: a document series runs its own cone).

use qp_calendar::Interval;
use qp_schema::program::{Side, SignalSource};

use super::ExecError;
use crate::check::cone_of;
use crate::checked::{Checked, Clock, Op, OutputSpec, Rule, Slot};

/// A node in the plan.
#[derive(Debug, Clone)]
pub(super) struct PNode {
    pub(super) id: String,
    pub(super) op: Op,
    /// Flat input slots, in field order.
    pub(super) ins: Vec<usize>,
    /// First output slot.
    pub(super) out0: usize,
    /// Output count.
    pub(super) n_out: usize,
}

/// What an output emits.
#[derive(Debug, Clone)]
pub(super) enum OutKind {
    Series,
    Event,
    Signal {
        side: Side,
        rule: Rule,
        source: SignalSource,
    },
    State {
        dims: Vec<String>,
    },
}

/// An output in the plan.
#[derive(Debug, Clone)]
pub(super) struct POut {
    pub(super) name: String,
    pub(super) kind: OutKind,
    /// Flat slots it reads (a state's dimensions in order).
    pub(super) slots: Vec<usize>,
    /// The node it reads (the first dimension's for a state).
    pub(super) node: usize,
}

/// One higher timeframe's nodes.
#[derive(Debug, Clone)]
pub(super) struct TfGroup {
    pub(super) iv: Interval,
    pub(super) nodes: Vec<usize>,
}

/// The plan.
#[derive(Debug, Clone)]
pub(super) struct Plan {
    pub(super) nodes: Vec<PNode>,
    pub(super) free: Vec<usize>,
    pub(super) base: Vec<usize>,
    pub(super) tfs: Vec<TfGroup>,
    pub(super) n_slots: usize,
    pub(super) outs: Vec<POut>,
    /// Static work per base bar of what runs (PR §8.1 / §8.3).
    pub(super) per_bar: u64,
    /// The base interval, when bound.
    pub(super) base_iv: Option<Interval>,
    /// Index into `outs` of the first state output (its key goes on signal records).
    pub(super) first_state: Option<usize>,
}

impl Plan {
    /// The plan of `c`, restricted to the outputs named in `only` (all when `None`).
    pub(super) fn new(c: &Checked, only: Option<&[String]>) -> Result<Plan, ExecError> {
        let selected: Vec<usize> = match only {
            None => (0..c.outputs.len()).collect(),
            Some(names) => names
                .iter()
                .map(|n| {
                    c.outputs
                        .iter()
                        .position(|o| &o.name == n)
                        .ok_or_else(|| ExecError::UnknownOutput(n.clone()))
                })
                .collect::<Result<_, _>>()?,
        };
        let roots: Vec<Slot> = selected
            .iter()
            .flat_map(|&o| c.outputs[o].spec.inputs())
            .collect();
        let active = cone_of(c, &roots);
        let mut base0 = Vec::with_capacity(c.nodes.len());
        let mut n_slots = 0;
        for n in &c.nodes {
            base0.push(n_slots);
            n_slots += n.outputs.len();
        }
        let flat = |s: &Slot| base0[s.node] + s.out;
        let mut plan = Plan {
            nodes: Vec::with_capacity(c.nodes.len()),
            free: Vec::new(),
            base: Vec::new(),
            tfs: Vec::new(),
            n_slots,
            outs: Vec::new(),
            per_bar: 0,
            base_iv: c.base.as_ref().map(|t| t.interval),
            first_state: None,
        };
        for (j, n) in c.nodes.iter().enumerate() {
            plan.nodes.push(PNode {
                id: n.id.clone(),
                op: n.op.clone(),
                ins: n.op.inputs().iter().map(flat).collect(),
                out0: base0[j],
                n_out: n.outputs.len(),
            });
            if !active[j] {
                continue;
            }
            plan.per_bar = plan.per_bar.saturating_add(n.w);
            match &n.clock {
                Clock::Free => plan.free.push(j),
                Clock::Base => plan.base.push(j),
                Clock::Tf(tf) => {
                    if plan.base_iv.is_none() {
                        return Err(ExecError::Unbound(tf.name.clone()));
                    }
                    match plan.tfs.iter_mut().find(|g| g.iv == tf.interval) {
                        Some(g) => g.nodes.push(j),
                        None => plan.tfs.push(TfGroup {
                            iv: tf.interval,
                            nodes: vec![j],
                        }),
                    }
                }
            }
        }
        for o in selected {
            plan.push_output(c, o, &flat);
        }
        Ok(plan)
    }

    fn push_output(&mut self, c: &Checked, o: usize, flat: &impl Fn(&Slot) -> usize) {
        let out = &c.outputs[o];
        let ins = out.spec.inputs();
        let kind = match &out.spec {
            OutputSpec::Series(_) => OutKind::Series,
            OutputSpec::Event(_) => OutKind::Event,
            OutputSpec::Signal {
                side, rule, source, ..
            } => OutKind::Signal {
                side: *side,
                rule: *rule,
                source: source.clone(),
            },
            OutputSpec::State(dims) => {
                if self.first_state.is_none() {
                    self.first_state = Some(self.outs.len());
                }
                OutKind::State {
                    dims: dims.iter().map(|(n, _)| n.clone()).collect(),
                }
            }
        };
        self.per_bar = self.per_bar.saturating_add(1);
        self.outs.push(POut {
            name: out.name.clone(),
            kind,
            slots: ins.iter().map(flat).collect(),
            node: ins.first().map_or(0, |s| s.node),
        });
    }
}
