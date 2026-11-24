//! The subsystem contract (`rounds/RF1/PLAN.md` §4; `docs/CONCEPTS.md` "Alt sistem
//! sözleşmesi"): the one interface between the authority (`qp-authority`) and the
//! subsystems that plug into it. The authority knows no subsystem; it owns a
//! [`Registry`], the subsystems' state ([`Slots`]) and an [`Outbox`], and calls the
//! registered subsystems at named, fixed [`Phase`]s.
//!
//! - Behaviour is stateless: a subsystem is a `&'static dyn` [`Subsystem`]; its state lives
//!   in the authority's [`Slots`] ([`PerWorld`] for state per capsule world).
//! - A subsystem reads through [`SessionView`] and writes only through [`Outbox`] (engine
//!   entry, notification, wake) or through an op's expansion ([`OpHandler`]).
//! - No method of a `dyn` trait here is generic.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here
// (the same allowance as `qp-authority`, `qp-ops`, `qp-plane`).
#![allow(clippy::result_large_err)]

mod handlers;
mod outbox;
mod phase;
mod registry;
mod slots;
mod subsystem;
mod view;

pub use handlers::{
    op_name, AgentOp, ClaimKind, ConditionKind, InputHandler, OpHandler, QueryHandler,
    TopicHandler, ValueSource,
};
pub use outbox::{EngineEntry, Outbox, MAX_EMIT_DEPTH};
pub use phase::{Moment, Phase};
pub use registry::{Registry, ANY};
pub use slots::{PerWorld, Slots};
pub use subsystem::{SessionCtx, Subsystem};
pub use view::{ConnView, ReadView, RunKind, SessionView};
