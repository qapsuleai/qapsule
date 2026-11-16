//! The interface an approach implements: two adapters (authority and replica) and a
//! registry entry. The harness knows nothing about an approach's internals; it measures
//! only the replica's display (keyed by meaning), the replica's visible data and the
//! bytes on the protocol channel.
//!
//! # Contract (stable — approaches are written against it in separate branches)
//!
//! - **Determinism.** Adapters must not read the clock, the OS RNG or the environment.
//!   Time is `ctx.now()` (virtual microseconds). Iterate ordered maps (`BTreeMap`), not
//!   `HashMap`.
//! - **Same engine code.** Both sides keep their data and run the reference detector
//!   through [`crate::engine::Engine`] (GOAL invariant). An approach may add bookkeeping
//!   (digests, state machines) on top.
//! - **Data plane vs meaning plane (K23).** Market data reaches a side only through its
//!   own feed (`on_source`) and its own settled-bar fetches (`ctx.fetch_settled`). The
//!   protocol channel carries meaning; it must not carry bars. (The channel is opaque to
//!   the harness; this rule is checked by review, and bytes are measured.)
//! - **Messages** are opaque bytes; JSON is the canonical encoding (Ö4) and bytes are
//!   measured raw and deflated.
//! - **Display.** After every virtual instant the harness reads `display()` and
//!   `visible_data()` of every binding. `visible_data` must be the bars the user's chart
//!   shows *now* (normally `engine.bars(b)`), not the data the approach wishes it had.
//! - **Work.** `work_units()` returns a deterministic counter (normally
//!   `engine.work()` plus the approach's own units via `Engine::add_work`).

use qp_schema::data::{DataBinding, DataEvent, FeedKey};
use qp_schema::Bar;

use crate::detector::DetectorCfg;
use crate::display::DisplayState;
use crate::spec::VTime;

/// Which side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// The authority (server).
    Authority,
    /// The replica (device).
    Replica,
}

/// What a feed delivers.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceEvent {
    /// A field-level data event (`qp_schema::data::DataEvent`: new bar or patch, with
    /// the feed's source `seq`; a dropped event leaves a gap in `seq`).
    Data(DataEvent),
    /// A full snapshot of the source's bars for the binding's window (initial
    /// resolution, Matriks `REFRESH` on reconnect). `seq` is the source sequence number
    /// the snapshot is current with; the next data event has `seq + 1`. (A harness
    /// extension: initial bars come from binding resolution, `docs/PROTOCOL.md` §18.3.)
    Snapshot {
        /// The feed.
        feed: FeedKey,
        /// Source sequence number the snapshot includes.
        seq: u64,
        /// Bars, oldest first; the last one is the live bar.
        bars: Vec<Bar>,
    },
}

impl SourceEvent {
    /// The feed of the event.
    #[must_use]
    pub fn feed(&self) -> &FeedKey {
        match self {
            Self::Data(d) => &d.feed,
            Self::Snapshot { feed, .. } => feed,
        }
    }
}

/// One binding as a side sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingSetup {
    /// Series id (`px0`, `px1`, …) — also the `binding` of claim keys.
    pub id: String,
    /// The data binding this side resolved (its window may differ per side, S12).
    pub binding: DataBinding,
    /// Its feed.
    pub feed: FeedKey,
    /// Bar interval (ms).
    pub interval_ms: i64,
}

/// What an adapter is constructed with.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// Which side.
    pub side: Side,
    /// Bindings, in index order.
    pub bindings: Vec<BindingSetup>,
    /// The reference detector.
    pub detector: DetectorCfg,
    /// Extra load studies per binding to give the engine (S17); pass through to
    /// `Engine::new` (it reads this field).
    pub extra_studies: u32,
}

impl Setup {
    /// Binding index of a feed.
    #[must_use]
    pub fn binding_of(&self, feed: &FeedKey) -> Option<usize> {
        self.bindings.iter().position(|b| &b.feed == feed)
    }
}

/// Channel state changes (both sides are told).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelEvent {
    /// The protocol connection dropped; messages sent now are held.
    Disconnected,
    /// The connection is back; held messages are being delivered.
    Reconnected,
}

/// Answer of the settled-bar source.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchReply {
    /// The id `ctx.fetch_settled` returned.
    pub id: u64,
    /// Binding index.
    pub binding: usize,
    /// Settled bars in the requested range (closed at least `rest.settle_lag_ms` before
    /// the request, corrections applied as of the request), oldest first, `f64`.
    pub bars: Vec<Bar>,
}

/// A request for settled bars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchRequest {
    /// Id.
    pub id: u64,
    /// Binding index.
    pub binding: usize,
    /// First bar time (inclusive).
    pub from_t: i64,
    /// Last bar time (inclusive).
    pub to_t: i64,
}

/// What an adapter can do during a callback.
#[derive(Debug)]
pub struct Ctx {
    now: VTime,
    next_fetch: u64,
    pub(crate) msgs: Vec<Vec<u8>>,
    pub(crate) timers: Vec<(VTime, u64)>,
    pub(crate) fetches: Vec<FetchRequest>,
}

impl Ctx {
    pub(crate) fn new(now: VTime, next_fetch: u64) -> Self {
        Self {
            now,
            next_fetch,
            msgs: Vec::new(),
            timers: Vec::new(),
            fetches: Vec::new(),
        }
    }

    pub(crate) fn next_fetch(&self) -> u64 {
        self.next_fetch
    }

    /// Current virtual time (µs).
    #[must_use]
    pub fn now(&self) -> VTime {
        self.now
    }

    /// Sends a protocol message to the other side.
    pub fn send(&mut self, msg: Vec<u8>) {
        self.msgs.push(msg);
    }

    /// Asks for `on_timer(token)` at virtual time `at` (clamped to now).
    pub fn set_timer(&mut self, at: VTime, token: u64) {
        self.timers.push((at.max(self.now), token));
    }

    /// Requests settled bars of binding `binding` in `[from_t, to_t]` from the
    /// provider's settled source (the "REST" refetch). The answer comes later through
    /// `on_fetch` with the returned id.
    pub fn fetch_settled(&mut self, binding: usize, from_t: i64, to_t: i64) -> u64 {
        let id = self.next_fetch;
        self.next_fetch += 1;
        self.fetches.push(FetchRequest {
            id,
            binding,
            from_t,
            to_t,
        });
        id
    }
}

/// The authority side of an approach.
pub trait AuthorityAdapter {
    /// An event from the authority's own feed.
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent);
    /// A message from the replica.
    fn on_replica_msg(&mut self, ctx: &mut Ctx, msg: &[u8]);
    /// A timer set with `ctx.set_timer`.
    fn on_timer(&mut self, _ctx: &mut Ctx, _token: u64) {}
    /// An answer of `ctx.fetch_settled`.
    fn on_fetch(&mut self, _ctx: &mut Ctx, _reply: &FetchReply) {}
    /// The protocol channel dropped or came back.
    fn on_channel(&mut self, _ctx: &mut Ctx, _ev: ChannelEvent) {}
    /// Deterministic work counter.
    fn work_units(&self) -> u64 {
        0
    }
}

/// The replica side of an approach.
pub trait ReplicaAdapter {
    /// An event from the replica's own feed.
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent);
    /// A message from the authority.
    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]);
    /// A timer set with `ctx.set_timer`.
    fn on_timer(&mut self, _ctx: &mut Ctx, _token: u64) {}
    /// An answer of `ctx.fetch_settled` (the replica's refetch from its own source).
    fn on_fetch(&mut self, _ctx: &mut Ctx, _reply: &FetchReply) {}
    /// The protocol channel dropped or came back.
    fn on_channel(&mut self, _ctx: &mut Ctx, _ev: ChannelEvent) {}
    /// What the user sees now.
    fn display(&self) -> &DisplayState;
    /// The bars the user's chart shows now for binding index `binding`.
    fn visible_data(&self, binding: usize) -> &[Bar];
    /// Deterministic work counter.
    fn work_units(&self) -> u64;
}

/// A registered approach.
#[derive(Debug, Clone, Copy)]
pub struct Approach {
    /// Name used on the command line (`n`, `a`, `b`, …).
    pub name: &'static str,
    /// One-line description.
    pub summary: &'static str,
    /// Builds the authority side.
    pub authority: fn(&Setup) -> Box<dyn AuthorityAdapter>,
    /// Builds the replica side.
    pub replica: fn(&Setup) -> Box<dyn ReplicaAdapter>,
}
