//! Static cost (PR §8.1), known before running: node weights from the PR §4 tables with
//! bound values, `per_bar = Σ w + outputs`, `live_per_bar = Σ_output (cone Σ w + 1)`.
//! Higher-timeframe nodes count undiscounted. The weights are language constants (PR
//! §8.4), not parameters.

use qp_schema::program::ProgramCost;

use crate::checked::{Op, Slot};
use crate::limits::{Limit, Limits};

/// Cost weight of a node per bar.
pub(crate) fn weight(op: &Op) -> u64 {
    use crate::checked::WindowKind::{Percentrank, Stdev};
    match op {
        Op::Input(_)
        | Op::Const(_)
        | Op::BarIndex
        | Op::Time
        | Op::Align { .. }
        | Op::Line { .. } => 0,
        Op::Window {
            kind: Stdev | Percentrank,
            period,
            ..
        } => *period as u64,
        Op::Window { .. } | Op::Atr { .. } => 2,
        Op::Macd { .. } | Op::Bollinger { .. } => 3,
        Op::Binary { .. }
        | Op::Unary { .. }
        | Op::Lag { .. }
        | Op::Cond { .. }
        | Op::Rearm { .. }
        | Op::Cooldown { .. }
        | Op::Bucket { .. } => 1,
    }
}

/// Which nodes the `roots` depend on (themselves included).
pub(crate) fn cone(inputs: &[Vec<Slot>], roots: &[Slot]) -> Vec<bool> {
    let mut mark = vec![false; inputs.len()];
    for r in roots {
        if let Some(m) = mark.get_mut(r.node) {
            *m = true;
        }
    }
    for j in (0..inputs.len()).rev() {
        if mark[j] {
            for s in &inputs[j] {
                if let Some(m) = mark.get_mut(s.node) {
                    *m = true;
                }
            }
        }
    }
    mark
}

/// The cost of nodes (`weights`, `inputs`) and outputs (their roots), with the bound bar
/// count if any.
pub(crate) fn static_cost(
    weights: &[u64],
    inputs: &[Vec<Slot>],
    outputs: &[Vec<Slot>],
    bars: Option<u64>,
    limits: &Limits,
) -> ProgramCost {
    let n_out = outputs.len() as u64;
    let per_bar = weights
        .iter()
        .fold(0u64, |a, w| a.saturating_add(*w))
        .saturating_add(n_out);
    let live_per_bar = outputs.iter().fold(0u64, |acc, roots| {
        let mark = cone(inputs, roots);
        let w = weights
            .iter()
            .zip(&mark)
            .filter(|(_, m)| **m)
            .fold(0u64, |a, (w, _)| a.saturating_add(*w));
        acc.saturating_add(w).saturating_add(1)
    });
    ProgramCost {
        per_bar,
        live_per_bar,
        bars,
        total: bars.map(|b| per_bar.saturating_mul(b)),
        limit: limits.get(Limit::MaxCostPerBar),
    }
}
