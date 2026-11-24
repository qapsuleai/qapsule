//! The `Subsystem` trait and its call context.

use qp_plane::values::Cache;
use qp_schema::protocol::{Op, ProtocolError, ViewState};

use crate::handlers::{
    ClaimKind, ConditionKind, InputHandler, OpHandler, QueryHandler, TopicHandler, ValueSource,
};
use crate::{Moment, Outbox, Phase, SessionView, Slots};

/// A call's context: a split borrow of the authority (read view, the subsystems' state,
/// the outbox, and the two pieces of the authority a reading changes: the values cache
/// and the connections' view state; RF1 S3.2, critique-1 B9).
pub struct SessionCtx<'a> {
    /// The authority, read only.
    pub view: &'a dyn SessionView,
    /// Every subsystem's state.
    pub slots: &'a mut Slots,
    /// Where writes go.
    pub out: &'a mut Outbox,
    /// The values cache (study values over the reading document; [`SessionView::reading`]):
    /// taken out of the authority for the call, so that it can be warmed next to the view.
    pub cache: &'a mut Cache,
    /// The connections' view state (`look`, §7: open capsule, depth, range, frames,
    /// style), by connection index ([`SessionView::conn_ids`] order).
    pub looks: &'a mut Vec<ViewState>,
}

/// A subsystem: a stateless behaviour object (`&'static dyn Subsystem`) registered with
/// the authority. Everything has a default: a subsystem says only what it does.
pub trait Subsystem: Sync {
    /// A short name (registration errors, diagnostics).
    fn name(&self) -> &'static str;
    /// Puts the subsystem's initial state into the slots (at registration).
    fn init(&self, _ctx: &mut SessionCtx<'_>) {}
    /// The phases it is called at.
    fn phases(&self) -> &'static [Phase] {
        &[]
    }
    /// The collect step of a phase it named (not [`Phase::Check`] / [`Phase::Expand`]).
    fn collect(&self, _phase: Phase, _at: &Moment<'_>, _ctx: &mut SessionCtx<'_>) {}
    /// [`Phase::Check`]: refuses a command of `conn` before its expansion.
    ///
    /// # Errors
    /// The command is refused.
    fn check(
        &self,
        _conn: &str,
        _ops: &[Op],
        _ctx: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError> {
        Ok(())
    }
    /// Its op handlers (expansion and policy).
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        &[]
    }
    /// Its query handlers.
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        &[]
    }
    /// Its input (gesture) handlers.
    fn inputs(&self) -> &'static [&'static dyn InputHandler] {
        &[]
    }
    /// Its topic handlers.
    fn topics(&self) -> &'static [&'static dyn TopicHandler] {
        &[]
    }
    /// Its condition kinds (live path).
    fn conditions(&self) -> &'static [&'static dyn ConditionKind] {
        &[]
    }
    /// Its value sources (data plane; D3).
    fn value_sources(&self) -> &'static [&'static dyn ValueSource] {
        &[]
    }
    /// Its claim kinds (D1).
    fn claims(&self) -> &'static [&'static dyn ClaimKind] {
        &[]
    }
}
