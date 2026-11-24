//! Phases: the named, fixed moments at which the authority calls the subsystems.

use qp_capsule::StoryEvent;
use qp_plane::values::Delta;
use qp_schema::protocol::{Op, Seq};

/// A named, fixed moment of the authority (`rounds/RF1/PLAN.md` §4.3, §11 D1). Each is
/// called at the line where that work happens today; collecting ([`crate::Subsystem::collect`])
/// and emitting (the authority applying the [`crate::Outbox`]) are separate steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// A command, before its expansion ([`crate::Subsystem::check`]; may refuse it).
    Check,
    /// An op's expansion ([`crate::OpHandler::expand`]); not a collect phase.
    Expand,
    /// A log entry was committed.
    AfterCommit,
    /// Bound series changed (data plane), before conditions and capsules are evaluated
    /// ([`Moment::Data`]); the capsule engine looked at the new final bars of one series
    /// ([`Moment::Look`], once per world) and its engine entry was written
    /// ([`Moment::Written`]).
    AfterData,
    /// The clock advanced (proposal expiry).
    Proposals,
    /// A subscription was made.
    Subscribe,
    /// A subscription was dropped.
    Unsubscribe,
    /// A connection said hello.
    Connect,
    /// A connection closed. No call point: `qp/0` has no close message and the authority
    /// no close path (a host drops a whole session). When one comes, it calls this phase
    /// and then [`crate::Slots::drop_conn`] (audit-s30 B4).
    Close,
    /// A claim's outcome was settled (a settled-bar answer decided disputes; D1).
    Settled,
    /// A corporate action (K88): each subsystem gives its plan, the authority writes one
    /// engine entry and one notification (draft hook, RS1).
    Adjust,
}

/// What a phase is about (given to [`crate::Subsystem::collect`]).
#[derive(Debug, Clone, Copy)]
pub enum Moment<'a> {
    /// [`Phase::Check`]: a command of connection `conn` (reserved: the check is called as
    /// [`crate::Subsystem::check`] with the same arguments).
    Command {
        /// Connection id.
        conn: &'a str,
        /// The command's ops.
        ops: &'a [Op],
    },
    /// [`Phase::AfterCommit`]: log entry `seq` (read it from the view's log).
    Entry {
        /// The entry's seq.
        seq: Seq,
    },
    /// [`Phase::AfterData`]: bound series changed at log position `seq`.
    Data {
        /// Log position.
        seq: Seq,
        /// The changed series and how.
        changed: &'a [(String, Delta)],
    },
    /// [`Phase::AfterData`]: the capsule engine's look at the new final bars of one
    /// series in one world (`public`: the public world's engine), its story `events` in
    /// the engine's order. Changes go into that look's engine entry ([`crate::Outbox::amend`]).
    Look {
        /// The look's story events.
        events: &'a [StoryEvent],
        /// Whether it is the public world's look.
        public: bool,
    },
    /// [`Phase::AfterData`]: the look's engine entry was written: `seq`, and `public` the
    /// public entry it made. Notifications go out at this line ([`crate::Outbox::say`]).
    Written {
        /// The entry's seq.
        seq: Seq,
        /// The public world's entry, if one was made.
        public: Option<Seq>,
    },
    /// [`Phase::Proposals`]: the clock is now `now`.
    Clock {
        /// The new clock.
        now: i64,
    },
    /// [`Phase::Subscribe`] / [`Phase::Unsubscribe`]: subscription `sub` of `conn`.
    Sub {
        /// Connection id.
        conn: &'a str,
        /// Subscription id.
        sub: &'a str,
    },
    /// [`Phase::Connect`] / [`Phase::Close`]: connection `conn`.
    Conn {
        /// Connection id.
        conn: &'a str,
    },
    /// [`Phase::Settled`]: the answer for `series` bars `lo..=hi` decided its claims.
    Settled {
        /// The bar series.
        series: &'a str,
        /// First bar time covered.
        lo: i64,
        /// Last bar time covered.
        hi: i64,
        /// Whether the answer corrected the data.
        corrected: bool,
    },
    /// [`Phase::Adjust`]: a corporate action on `symbol` effective at `at`.
    Adjust {
        /// Instrument symbol.
        symbol: &'a str,
        /// Adjustment factor.
        factor: f64,
        /// Effective time.
        at: i64,
    },
}
