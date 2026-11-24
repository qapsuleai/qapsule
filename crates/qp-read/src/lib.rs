//! Reading and presentation (`okuma ve sunum`; RF1 S3.2, `rounds/RF1/PLAN.md` §3.3 row 4):
//! the authority's queries (§5.4) and input gestures (§4.6, §7) as a subsystem — queries
//! ([`query`]), semantic reading (`screen`, `hit`: [`reading`]), interaction
//! ([`interact`]), the `preview` query ([`preview`]), scene explanation ([`explain`]),
//! binding queries ([`bindq`]) and the `capsules` / `ledger` queries ([`capsules`]).
//!
//! The authority knows none of it: it dispatches every query and gesture no other
//! subsystem registered to this one (`qp_subsystem::ANY`), inside its `as_reader` (the
//! asker's projection and world). Everything is read through `ReadView`; what a
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
// The reading code moved from `impl Session` (RF1 S3.2) and kept its bodies: `&self.doc`,
// `&mut self.cache` were field borrows there and are borrows of `Rd`'s references here
// (auto-dereferenced, the same value).
#![allow(clippy::needless_borrow)]

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
mod rd;
mod reading;
mod subsystem;

pub use explain::explain_scene;
pub(crate) use rd::look_of;
pub use rd::{scene_for_range, ReadState};
pub use reading::{hit_scene, screen_frame, HIT_HEIGHT, HIT_WIDTH};
pub use subsystem::{Read, READ};
