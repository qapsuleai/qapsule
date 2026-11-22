//! Qapsule domain protocol v0 (`qp/0`): the messages exchanged between the engine
//! (the document *authority*), user interfaces and agents. See `docs/PROTOCOL.md`.
//!
//! Single source of truth: these Rust types. The JSON Schema of every message is
//! generated from them ([`crate::json_schemas`]); nothing here is written by hand twice.
//!
//! Shape of the conversation:
//! - a client opens a connection and sends [`ClientMsg::Hello`] (who it is, what surface);
//! - it then sends commands, queries, subscriptions, input and attention directives;
//! - the authority answers each request (`ack`, `result`, `error`) and pushes
//!   notifications (`notify`) for the client's subscriptions.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::capsule::{
    Audience, Direction, EvidenceFamily, GlyphSpec, LedgerStat, LicenseClass, MeaningKind,
    Sensitivity, Stage, Story, TimeKind, Transition, Trust, Validity,
};
use crate::data::{BindingState, DataBinding, DataEvent, DataPosition, EvidenceRef, FeedKey};
use crate::grammar::{Layer, View};
use crate::types::{
    Author, Bar, CapsuleState, Condition, Depth, Document, Fact, Finding, Glyph, Intent,
    LedgerEntry, Series, TimestampMs,
};

mod capsule;
mod change;
mod envelope;
mod input;
mod notify;
mod ops;
mod query;
mod replay;
mod reply;
mod selector;
mod story;
mod summary;
mod transcript;
mod viewer;
mod wire;

pub use capsule::*;
pub use change::*;
pub use envelope::*;
pub use input::*;
pub use notify::*;
pub use ops::*;
pub use query::*;
pub use replay::*;
pub use reply::*;
pub use selector::*;
pub use story::*;
pub use summary::*;
pub use transcript::*;
pub use viewer::*;
pub use wire::*;

/// The protocol version string of this module.
pub const PROTOCOL_VERSION: &str = "qp/0";

/// Position in a document's operation log. The first entry is 1; 0 means "before any
/// entry" (the initial document).
pub type Seq = u64;

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}
