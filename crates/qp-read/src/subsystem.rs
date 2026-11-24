//! The reading subsystem: its query and input handlers (every kind no other subsystem
//! registered) and its slot.

use qp_schema::protocol::{Gesture, OutboundLine, ProtocolError, Query, QueryResult};
use qp_subsystem::{InputHandler, QueryHandler, SessionCtx, Subsystem, ANY};

/// The reading subsystem: register it with the authority.
pub struct Read;

/// The reading subsystem, as the composition registers it.
pub static READ: Read = Read;

/// Answers every query kind no other subsystem registered.
struct Queries;
/// Handles every gesture kind no other subsystem registered.
struct Inputs;

static QUERIES: &[&dyn QueryHandler] = &[&Queries];
static INPUTS: &[&dyn InputHandler] = &[&Inputs];

impl QueryHandler for Queries {
    fn kind(&self) -> &'static str {
        ANY
    }
    fn answer(
        &self,
        conn: &str,
        q: &Query,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<QueryResult, ProtocolError> {
        crate::rd::reading(conn, ctx, |r, ci| r.query(ci, q))
    }
}

impl InputHandler for Inputs {
    fn kind(&self) -> &'static str {
        ANY
    }
    fn input(
        &self,
        conn: &str,
        g: &Gesture,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(QueryResult, Vec<OutboundLine>), ProtocolError> {
        crate::rd::reading(conn, ctx, |r, ci| {
            let got = r.input(ci, g);
            r.keep(ci);
            got
        })
    }
}

impl Subsystem for Read {
    fn name(&self) -> &'static str {
        "read"
    }
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.slots.put(crate::ReadState::default());
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        QUERIES
    }
    fn inputs(&self) -> &'static [&'static dyn InputHandler] {
        INPUTS
    }
}
