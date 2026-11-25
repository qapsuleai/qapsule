//! Session composition (`rounds/RF1/PLAN.md` §3.3 row 8): the public façade of the `qp/0`
//! authority. Every caller (hosts, MCP, shells, CLI, WASM) keeps using `qp_session::…`;
//! the authority itself lives in `qp-authority`, which knows no subsystem. The standard
//! subsystems are registered here ([`STANDARD`]: one line per subsystem), and every
//! authority built through this façade — a session, a replay host, a replica, a viewer's
//! open side, the tape's sessions — is built with them ([`compose`]).
//!
//! [`conform`] (the conformance transcript runner) lives here since S3.0: it runs
//! transcripts against the composed session, with the standard subsystems registered.
//!
//! The re-exports below are the façade, intended: the same items under the same paths as
//! before the split (`rounds/RF1/experiments/paths.md`, S3.0, S3.1).

pub mod agent;
pub mod compose;
pub mod conform;
pub mod replayhost;
pub mod replica;
#[doc(hidden)]
pub mod tape;
pub mod viewer;

pub use compose::{OpenSide, ReplayHost, Replica, Session, FOLLOWER, STANDARD};
pub use qp_authority::{capsule, voices};
pub use qp_authority::{
    confine, default_limits, permissions, read_json, CoinFlipRule, CommentAudit, CompileOptions,
    SettleRequest, MAX_DISPUTES_PER_CONN, MAX_DISPUTE_DIGESTS, MAX_OPEN_DISPUTES,
};
/// The authority alone (no subsystem), and the subsystem list type.
pub use qp_authority::{Session as Authority, Subsystems};
pub use qp_read::{explain_scene, hit_scene, screen_frame, HIT_HEIGHT, HIT_WIDTH};
pub use qp_replayhost::demo_init;
pub use qp_replica::{document_digest, view_digest, ReplicaOptions};
