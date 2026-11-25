//! Warnings (PR §11.3), which never reject: an event output that may fire on consecutive
//! bars (D1 condition 2: live notifications merge, so the checker says so), and nodes
//! that reach no output. Whether events are isolated depends on data: that is `try`'s.

use super::cost::cone;
use super::Cx;
use crate::checked::{BinaryKind, CheckedOutput, CondKind, Op, OutputSpec, Slot};
use crate::diag::join_i;
use crate::message::Msg;
use crate::reason::Reason;

/// Per node, whether it is edge-shaped: true on a bar never implies true on the next
/// (`cross_*`, `once`, `rearm`, `cooldown` with `bars ≥ 1`, or `and` with such an
/// operand). One forward pass over the topological order: shared operands are not
/// re-walked, and no recursion depth grows with the program.
fn edge_shapes(ops: &[&Op]) -> Vec<bool> {
    let mut edge: Vec<bool> = Vec::with_capacity(ops.len());
    for op in ops {
        let e = match op {
            Op::Binary {
                kind: BinaryKind::CrossAbove | BinaryKind::CrossBelow,
                ..
            }
            | Op::Cond {
                kind: CondKind::Once,
                ..
            }
            | Op::Rearm { .. } => true,
            Op::Cooldown { bars, .. } => *bars >= 1,
            Op::Binary {
                kind: BinaryKind::And,
                a,
                b,
            } => {
                edge.get(a.node).copied().unwrap_or(false)
                    || edge.get(b.node).copied().unwrap_or(false)
            }
            _ => false,
        };
        edge.push(e);
    }
    edge
}

/// Adds the warnings of a program whose nodes all typed (`ids` by index).
pub(super) fn warn(
    cx: &mut Cx<'_>,
    ops: &[&Op],
    ids: &[String],
    outputs: &[(usize, &CheckedOutput)],
) {
    let edge = edge_shapes(ops);
    for (i, o) in outputs {
        if let OutputSpec::Event(s) = &o.spec {
            if !edge.get(s.node).copied().unwrap_or(false) {
                let at = format!("{}/node", join_i("/outputs", *i));
                cx.d.at(
                    Reason::ConsecutiveEvents,
                    &at,
                    &Msg::ConsecutiveEvents(o.name.clone()),
                )
                .expected([
                    "cross_above",
                    "cross_below",
                    "once",
                    "rearm",
                    "cooldown",
                ]);
            }
        }
    }
    let inputs: Vec<Vec<Slot>> = ops.iter().map(|op| op.inputs()).collect();
    let roots: Vec<Slot> = outputs.iter().flat_map(|(_, o)| o.spec.inputs()).collect();
    let used = cone(&inputs, &roots);
    for (j, u) in used.iter().enumerate() {
        if !u {
            let id = ids.get(j).cloned().unwrap_or_default();
            cx.d.at(
                Reason::UnusedNode,
                &join_i("/nodes", j),
                &Msg::UnusedNode(id),
            );
        }
    }
}
