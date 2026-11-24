//! Qapsule reference authority for the domain protocol `qp/0` (`docs/PROTOCOL.md`).
//!
//! - [`Session`]: one document, its log, connections, subscriptions, proposals and
//!   presence. [`Session::handle_raw`] is the boundary every inbound message crosses
//!   (size limit, parse, schema with JSON pointer, then handling).
//! - [`replay`]: rebuilds a document from the initial document and a log, applying
//!   only the recorded changes (no study code, no presenter, no clock).
//! - Semantic reading (R2): [`Session::scene`] compiles a view with `qp-grammar` over
//!   the document ([`DocProvider`]); `screen` and `hit` answer from that scene; the
//!   presenter applies the grammar's depth rules.
//!
//! Determinism: no clock (time arrives as `clock` messages), no randomness, no hash
//! maps in outputs. Studies are computed with `qp-compute`.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here.
#![allow(clippy::result_large_err)]

#[macro_use]
mod macros;

mod authority;
pub mod capsule;
mod data;
mod read;

pub use authority::agent;
pub(crate) use authority::session;
pub(crate) use capsule::{claims, comments, dispute, public, story};
/// (RF1 S3.3) The claims' pure functions a follower and the viewer use (`qp-replica`,
/// `qp-viewer`).
#[doc(hidden)]
pub use claims::{anchor_of, closed, eval_at, feeds_of};
pub use comments::CommentAudit;
pub(crate) use data::live;

pub use story::CoinFlipRule;

pub use dispute::{
    SettleRequest, MAX_DIGESTS as MAX_DISPUTE_DIGESTS, MAX_OPEN as MAX_OPEN_DISPUTES,
    MAX_PER_CONN as MAX_DISPUTES_PER_CONN,
};
pub use qp_grammar::CompileOptions;
/// (R5/m17-int) The capsule voices a connection may choose (`style` `voice`).
pub use qp_voice as voices;
pub use session::{confine, default_limits, permissions, read_json, Lists, Session, Subsystems};
