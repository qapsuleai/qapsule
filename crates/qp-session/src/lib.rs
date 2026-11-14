//! Qapsule reference authority for the domain protocol `qp/0` (`docs/PROTOCOL.md`).
//!
//! - [`Session`]: one document, its log, connections, subscriptions, proposals and
//!   presence. [`Session::handle_raw`] is the boundary every inbound message crosses
//!   (size limit, parse, schema with JSON pointer, then handling).
//! - [`replay`]: rebuilds a document from the initial document and a log, applying
//!   only the recorded changes (no study code, no presenter, no clock).
//! - [`conform`]: the conformance transcript runner, in-process and over `qp serve`.
//!
//! Determinism: no clock (time arrives as `clock` messages), no randomness, no hash
//! maps in outputs. Studies are computed with `qp-compute`.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here.
#![allow(clippy::result_large_err)]

mod apply;
mod canon;
pub mod conform;
mod error;
mod expand;
mod gops;
mod locate;
mod present;
mod query;
mod session;
mod suggest;
mod summary;
mod values;

pub use apply::{
    apply_all, apply_change, find_collection, keys_overlap, replay, touched_ids, touched_key,
    touched_keys,
};
pub use canon::{canon_document, thresholds};
pub use present::profile_for;
pub use session::{confine, default_limits, permissions, read_json, Session, ENGINE_PRINCIPAL};
pub use suggest::{did_you_mean, edit_distance};
