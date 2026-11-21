//! Qapsule reference authority for the domain protocol `qp/0` (`docs/PROTOCOL.md`).
//!
//! - [`Session`]: one document, its log, connections, subscriptions, proposals and
//!   presence. [`Session::handle_raw`] is the boundary every inbound message crosses
//!   (size limit, parse, schema with JSON pointer, then handling).
//! - [`replay`]: rebuilds a document from the initial document and a log, applying
//!   only the recorded changes (no study code, no presenter, no clock).
//! - [`conform`]: the conformance transcript runner, in-process and over `qp serve`.
//! - Semantic reading (R2): [`Session::scene`] compiles a view with `qp-grammar` over
//!   the document ([`DocProvider`]); `screen` and `hit` answer from that scene; the
//!   presenter applies the grammar's depth rules.
//!
//! Determinism: no clock (time arrives as `clock` messages), no randomness, no hash
//! maps in outputs. Studies are computed with `qp-compute`.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here.
#![allow(clippy::result_large_err)]

/// The reading document of a session (`Session::rd`, the document with ready bound bars;
/// `plane.rs`), as a field-level borrow so that `&mut self.cache` can be taken alongside.
macro_rules! rd {
    ($s:expr) => {
        $s.rd.as_ref().unwrap_or(&$s.doc)
    };
}

pub mod agent;
mod apply;
mod bindq;
mod canon;
mod capidx;
pub mod capsule;
mod claims;
mod comments;
pub use comments::CommentAudit;
pub mod conform;
mod dispute;
mod error;
mod expand;
mod explain;
mod gops;
pub mod headparts;
mod interact;
mod live;
mod locate;
mod plane;
mod present;
mod preview;
mod provider;
mod public;
mod query;
mod reading;
pub mod replayhost;
pub mod replica;
mod select;
mod session;
mod story;
mod suggest;
mod summary;
#[doc(hidden)]
pub mod tape;
mod values;
pub mod viewer;
mod watch;

pub use story::CoinFlipRule;

pub use apply::{
    apply_all, apply_change, find_collection, keys_overlap, replay, touched_ids, touched_key,
    touched_keys,
};
pub use canon::{canon_document, thresholds};
pub use dispute::{
    SettleRequest, MAX_DIGESTS as MAX_DISPUTE_DIGESTS, MAX_OPEN as MAX_OPEN_DISPUTES,
    MAX_PER_CONN as MAX_DISPUTES_PER_CONN,
};
pub use explain::explain_scene;
pub use plane::{DataError, DataOutcome, HostNotice};
pub use present::profile_for;
pub use provider::DocProvider;
/// (R5/m17-int) The capsule voices a connection may choose (`style` `voice`).
pub use qp_capsule::voice as voices;
pub use qp_grammar::CompileOptions;
pub use reading::{hit_scene, screen_frame, HIT_HEIGHT, HIT_WIDTH};
pub use replayhost::{demo_init, ReplayHost};
pub use replica::{document_digest, view_digest, Replica, ReplicaOptions};
pub use session::{confine, default_limits, permissions, read_json, Session, ENGINE_PRINCIPAL};
pub use suggest::{did_you_mean, edit_distance};
