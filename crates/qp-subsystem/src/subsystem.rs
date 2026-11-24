//! The `Subsystem` trait and its call context.

use qp_schema::protocol::{Op, ProtocolError};

use crate::handlers::{
    ClaimKind, ConditionKind, InputHandler, OpHandler, QueryHandler, TopicHandler, ValueSource,
};
use crate::{Moment, Outbox, Phase, SessionView, Slots};

/// A call's context: a split borrow of the authority (read view, the subsystems' state,
/// the outbox).
pub struct SessionCtx<'a> {
    /// The authority, read only.
    pub view: &'a dyn SessionView,
    /// Every subsystem's state.
    pub slots: &'a mut Slots,
    /// Where writes go.
    pub out: &'a mut Outbox,
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
