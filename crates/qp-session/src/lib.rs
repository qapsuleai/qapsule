//! Session composition (`rounds/RF1/PLAN.md` §3.3 row 8): the public façade of the `qp/0`
//! authority. Every caller (hosts, MCP, shells, CLI, WASM) keeps using `qp_session::…`;
//! the authority itself lives in `qp-authority`, which knows no subsystem. The standard
//! subsystems are registered here (RF1 S3.1 onwards); today the registry is empty.
//!
//! The re-exports below are the façade, intended: the same items under the same paths as
//! before the split (`rounds/RF1/experiments/paths.md`, S3.0).

#[doc(hidden)]
pub use qp_authority::tape;
pub use qp_authority::{agent, capsule, conform, replayhost, replica, viewer, voices};
pub use qp_authority::{
    confine, default_limits, demo_init, document_digest, explain_scene, hit_scene, permissions,
    read_json, screen_frame, view_digest, CoinFlipRule, CommentAudit, CompileOptions, ReplayHost,
    Replica, ReplicaOptions, Session, SettleRequest, HIT_HEIGHT, HIT_WIDTH, MAX_DISPUTES_PER_CONN,
    MAX_DISPUTE_DIGESTS, MAX_OPEN_DISPUTES,
};
