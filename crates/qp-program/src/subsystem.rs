//! The program subsystem: its ops, its queries and its [`Phase::Check`].

use qp_schema::protocol::{Op, ProtocolError};
use qp_subsystem::{Moment, OpHandler, Phase, QueryHandler, SessionCtx, Subsystem};

use crate::budget::LogBytes;

use crate::ops::{ADD_PROGRAM, REMOVE_PROGRAM, SET_PROGRAM_PARAMS};
use crate::query::{CHECK_PROGRAM, EXPLAIN_PROGRAM, TRY_PROGRAM};

/// The program subsystem (`qp-program`): register it with the authority.
pub struct Programs;

/// The program subsystem, as the composition registers it.
pub static PROGRAM: Programs = Programs;

/// The program ops, in this order.
static OPS: &[&dyn OpHandler] = &[&ADD_PROGRAM, &SET_PROGRAM_PARAMS, &REMOVE_PROGRAM];

/// The program queries, in this order.
static QUERIES: &[&dyn QueryHandler] = &[&CHECK_PROGRAM, &TRY_PROGRAM, &EXPLAIN_PROGRAM];

impl Subsystem for Programs {
    fn name(&self) -> &'static str {
        "program"
    }
    /// The log's program bytes per owner: one scan of the log there is.
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.slots.put(LogBytes::scan(ctx.view.log()));
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::Check, Phase::AfterCommit]
    }
    /// [`Phase::AfterCommit`]: the committed entry's program bytes (any author: this
    /// subsystem writes no entry of its own, so nothing loops). Entries are committed in
    /// the authority's world.
    fn collect(&self, phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        if phase != Phase::AfterCommit
            || !matches!(at, Moment::Entry { .. })
            || ctx.view.in_public()
        {
            return;
        }
        let log = ctx.view.log();
        match ctx.slots.get_mut::<LogBytes>() {
            Some(lb) => lb.on_commit(log),
            None => ctx.slots.put(LogBytes::scan(log)),
        }
    }
    fn check(&self, conn: &str, ops: &[Op], ctx: &mut SessionCtx<'_>) -> Result<(), ProtocolError> {
        crate::check::check(conn, ops, ctx)
    }
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        OPS
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        QUERIES
    }
}
