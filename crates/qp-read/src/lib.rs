//! Reading and presentation (`okuma ve sunum`; RF1 S3.2, `rounds/RF1/PLAN.md` §3.3 row 4):
//! the authority's queries (§5.4) and input gestures (§4.6, §7) as a subsystem — queries
//! ([`query`]), semantic reading (`screen`, `hit`: [`reading`]), interaction
//! ([`interact`]), the `preview` query ([`preview`]), scene explanation ([`explain`]),
//! binding queries ([`bindq`]) and the `capsules` / `ledger` queries ([`capsules`]).
//!
//! The authority knows none of it: it dispatches every query and gesture no other
//! subsystem registered to this one (`qp_subsystem::ANY`), inside its `as_reader` (the
//! asker's projection and world). Everything is read through `SessionView`; what a
//! reading changes is the subsystem's own state ([`ReadState`]: viewports, focus, input
//! counter) and the two pieces of the authority the call context lends (`SessionCtx`:
//! the values cache, the connections' view state). Nothing here writes the document or the
//! log.
//!
//! Capsule life stays in the authority: the `capsule` query's detail, the `stories`
//! query, the capsule items and sentences are asked through the view.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here
// (the same allowance as `qp-authority`, `qp-ops`, `qp-plane`).
#![allow(clippy::result_large_err)]

/// The reading document of a reading (`Rd::rdoc`: the document with ready bound bars).
macro_rules! rd {
    ($s:expr) => {
        $s.rdoc
    };
}

mod bindq;
mod capsules;
mod explain;
mod interact;
mod preview;
mod query;
mod reading;
mod rd;

/// The connection's look (`ViewState.look`; absent or unknown: `r4`): the viewer's rule.
pub(crate) fn look_of(look: &qp_schema::protocol::ViewState) -> &'static qp_look::look::Look {
    qp_look::look::or_default(look.look.as_deref())
}

pub use explain::explain_scene;
pub use reading::{hit_scene, screen_frame, HIT_HEIGHT, HIT_WIDTH};
pub use rd::{scene_for_range, ReadState};

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
        rd::reading(conn, ctx, |r, ci| r.query(ci, q))
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
        rd::reading(conn, ctx, |r, ci| {
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
        ctx.slots.put(ReadState::default());
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        QUERIES
    }
    fn inputs(&self) -> &'static [&'static dyn InputHandler] {
        INPUTS
    }
}
