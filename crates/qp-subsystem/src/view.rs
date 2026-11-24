//! The narrow read view a subsystem gets of the authority.

use qp_plane::plane::DataPlane;
use qp_schema::data::FeedKey;
use qp_schema::market::MarketStatus;
use qp_schema::protocol::{LogEntry, Permissions, Principal, Role, Seq, SurfaceKind};
use qp_schema::Document;

/// What a subsystem may know of a connection.
#[derive(Debug, Clone)]
pub struct ConnView<'a> {
    /// Connection id.
    pub id: &'a str,
    /// Its principal.
    pub principal: &'a str,
    /// The principal's role.
    pub role: Role,
    /// The role's permissions.
    pub permissions: Permissions,
    /// The surface kind it said hello with.
    pub surface: SurfaceKind,
}

/// What kind of run the authority is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    /// Live data.
    Live,
    /// A replay of recorded data.
    Replay,
    /// A simulation.
    Simulation,
}

/// Read methods of the authority (`rounds/RF1/PLAN.md` §4.2, Y9). Writing goes through
/// [`crate::Outbox`]. Methods whose source the authority does not have yet answer with
/// their default (`None`).
pub trait SessionView {
    /// The current document (of the world the phase runs in).
    fn document(&self) -> &Document;
    /// The reading document: the document with ready bound bars.
    fn reading(&self) -> &Document;
    /// The log (of the world the phase runs in).
    fn log(&self) -> &[LogEntry];
    /// The log's length.
    fn seq(&self) -> Seq;
    /// The last clock value.
    fn clock(&self) -> i64;
    /// A principal of the session.
    fn principal(&self, id: &str) -> Option<&Principal>;
    /// A connection that said hello.
    fn conn(&self, id: &str) -> Option<ConnView<'_>>;
    /// The data plane.
    fn plane(&self) -> &DataPlane;
    /// The market status the host last gave.
    fn market(&self) -> Option<&MarketStatus>;
    /// Whether the phase runs in the public world (`docs/CONCEPTS.md`, two worlds).
    fn in_public(&self) -> bool;
    /// Whether `principal` is entitled to feed `feed` (§18.3).
    fn entitled(&self, principal: &str, feed: &FeedKey) -> bool {
        self.plane().entitled(principal, feed)
    }
    /// A parameter of the K78 registry (none yet).
    fn param(&self, _name: &str) -> Option<f64> {
        None
    }
    /// The kind of run (the authority is not told yet).
    fn run_kind(&self) -> Option<RunKind> {
        None
    }
    /// The replay position (the authority is not told yet).
    fn replay_position(&self) -> Option<i64> {
        None
    }
}
