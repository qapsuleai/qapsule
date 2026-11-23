//! Qapsule operations (RF1 S2, split out of `qp-session`): pure functions of the
//! document. Expansion of protocol ops into replayable changes ([`expand`], [`gops`],
//! [`watch`]), selectors ([`select`], [`locate`]), canonical forms ([`canon`]),
//! summaries ([`summary`]), applying changes and replay ([`apply`]), the presenter
//! ([`present`]) and the document provider ([`provider`]). Knows no session.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here.
#![allow(clippy::result_large_err)]

pub mod apply;
pub mod canon;
pub mod expand;
pub mod gops;
pub mod live;
pub mod locate;
pub mod present;
pub mod provider;
pub mod select;
pub mod summary;
pub mod watch;

pub use apply::{
    apply_all, apply_change, find_collection, keys_overlap, replay, touched_ids, touched_key,
    touched_keys, ENGINE_PRINCIPAL,
};
pub use canon::{canon_document, thresholds};
pub use present::profile_for;
pub use provider::{DocProvider, HeadlineSource};
