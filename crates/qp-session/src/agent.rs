//! (R5B, §25) The agent policy; [`agent_op`] with the standard subsystems' ops.

pub use qp_authority::agent::*;

/// (R5B, §25.3) The agent allow-list of the composed authority: an op a standard
/// subsystem owns, by its handler; any other, as [`qp_authority::agent::agent_op`].
#[must_use]
pub fn agent_op(doc: &qp_schema::Document, op: &qp_schema::protocol::Op) -> AgentOp {
    crate::STANDARD
        .list()
        .iter()
        .flat_map(|s| s.ops())
        .find(|h| h.owns(op))
        .map_or_else(
            || qp_authority::agent::agent_op(doc, op),
            |h| h.agent(doc, op),
        )
}
