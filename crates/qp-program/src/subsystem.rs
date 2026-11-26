//! The program subsystem: its ops, its queries and its [`Phase::Check`].

use qp_schema::protocol::{Op, ProtocolError};
use qp_subsystem::{OpHandler, Phase, QueryHandler, SessionCtx, Subsystem};

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
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.slots.put(crate::budget::LogBytes::default());
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::Check]
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
