//! The reference authority: one document, its log, connections, subscriptions and
//! presence. Every inbound message goes through [`Session::handle_raw`].

mod accept;
mod clock;
mod command;
mod create;
mod dispatch;
mod events;
mod files;
mod intake;
mod phases;
mod presence;
mod settle;
mod worlds;

pub use files::{confine, read_json};
pub(crate) use phases::expand_ops;
pub use phases::{Lists, Subsystems};

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qp_schema::protocol::{
    AckStatus, AttentionDirective, ChangeOp, ClientMsg, DocumentRef, ErrorCode, Event, Limits,
    LogEntry, Op, OutboundLine, Permissions, Principal, Profile, Proposal, ProposalState,
    ProtocolError, Role, Seq, ServerMsg, SessionInit, Surface, Topic, Viewport, WriteMode,
    PROTOCOL_VERSION,
};
use qp_schema::{Author, CapsuleState, Document};
use serde_json::Value;

use crate::public::{PubNext, PubPending};
use qp_ops::apply::{apply_all, first_overlap, touched_ids, touched_key, touched_keys};
use qp_ops::expand::{
    check_condition, need_finding, need_proposal, summarize_ops, undone_set, Ctx, Expanded,
};
use qp_ops::locate::locate;
use qp_ops::present::profile_for;
use qp_ops::{select, ENGINE_PRINCIPAL};
use qp_plane::error::{invalid, ptr, PResult, E};
use qp_plane::plane::{reading_doc, DataError, DataOutcome, DataPlane, Provider, Reader};
use qp_plane::values::Cache;
use qp_subsystem::{op_name, Moment, Phase, Registry};

/// Features this authority grants when asked.
const FEATURES: [&str; 2] = ["attention", "narrative"];

/// Number of attention directives kept per principal.
const MAX_DIRECTIVES: usize = 16;

/// Default limits (`docs/PROTOCOL.md` §4.1).
#[must_use]
pub fn default_limits() -> Limits {
    Limits {
        max_message_bytes: 1_048_576,
        max_ops: 64,
        max_bars: 10_000,
        max_subscriptions: 32,
        max_string: 4096,
        max_bind_bars: 100_000,
        max_watches: None,
    }
}

/// Permissions of a role (`docs/PROTOCOL.md` §9).
#[must_use]
pub fn permissions(role: Role) -> Permissions {
    let p = |read, write, propose, decide, attend, feed| Permissions {
        read,
        write,
        propose,
        decide,
        attend,
        feed,
    };
    match role {
        Role::Owner => p(true, true, true, true, true, false),
        Role::Editor => p(true, true, true, false, true, false),
        Role::Proposer => p(true, false, true, false, true, false),
        Role::Viewer => p(true, false, false, false, false, false),
        Role::Feed => p(false, false, false, false, false, true),
        Role::Host => p(true, true, false, false, false, true),
    }
}

#[derive(Debug)]
pub(crate) struct Sub {
    pub id: String,
    pub topic: Topic,
    /// Last value of a watched condition (edge detection).
    pub last: Option<bool>,
    /// Cache slots of a condition's series, with the cache generation they belong to
    /// (incremental live path: which ticks affect the condition).
    pub memo: Option<(u64, [Option<usize>; 2])>,
    /// Where a condition sits in the structure, with the structure generation it was
    /// derived for (incremental live path).
    pub watch: Option<(u64, crate::live::CondInfo)>,
    /// (R5/M18, §23) A `stories` subscription: what it was last sent.
    pub stories: Option<crate::story::StoriesSent>,
}

#[derive(Debug)]
pub(crate) struct Conn {
    pub id: String,
    pub principal: String,
    pub role: Role,
    pub surface: Surface,
    pub profile: Profile,
    pub subs: Vec<Sub>,
}

/// The reference authority for one document.
pub struct Session {
    pub(crate) doc: Document,
    pub(crate) initial: Document,
    pub(crate) log: Vec<LogEntry>,
    pub(crate) principals: Vec<Principal>,
    pub(crate) clock: i64,
    pub(crate) epoch: u64,
    pub(crate) ttl: Option<u64>,
    pub(crate) limits: Limits,
    pub(crate) conns: Vec<Conn>,
    /// (R4, §7) The connections' view state (open capsule, its depth and chart range,
    /// frame sizes, style), by connection index: kept beside `conns` so that a subsystem
    /// call can borrow it (`SessionCtx::looks`; RF1 S3.2).
    pub(crate) looks: Vec<qp_schema::protocol::ViewState>,
    pub(crate) cache: Cache,
    pub(crate) intent_state: BTreeMap<String, bool>,
    pub(crate) directives: BTreeMap<String, Vec<AttentionDirective>>,
    pub(crate) att_counter: u64,
    pub(crate) schema: Value,
    /// The data plane (bound series resolutions; outside the document, §18).
    pub(crate) plane: DataPlane,
    /// The reading document: the document with ready bound bars (`None` without bound
    /// series). Never serialised, logged or sent.
    pub(crate) rd: Option<Document>,
    /// Anchors to stamp in an engine entry (§18.3), from the last plane sync.
    pub(crate) pending_stamps: Vec<(String, i64)>,
    /// Incremental live path on (default). Off: every bar change clears the values cache
    /// and every watcher and live capsule is evaluated (the from-scratch reference path,
    /// for tests and measurements).
    pub(crate) incremental: bool,
    /// Bar changes not yet announced to `view` subscribers: (bar series, delta, seq).
    pub(crate) pending_views: Vec<(String, qp_plane::values::Delta, Seq)>,
    /// View id -> every series it reads, with the inputs of studies among them
    /// (transitively); built on demand, dropped on structure changes.
    pub(crate) view_reads: Option<BTreeMap<String, BTreeSet<String>>>,
    /// (RP2/T8) Asserted data-derived claims, gap marks and queued `claim` events (§19).
    pub(crate) claims: crate::claims::Claims,
    /// Structure generation: bumped by every log entry that is not only bar writes (the
    /// live index and the subscriptions' condition info belong to one generation).
    pub(crate) struct_gen: u64,
    /// The live path's index of the structure (`None` or of an older generation: rebuilt
    /// on demand).
    pub(crate) live_index: Option<crate::live::LiveIndex>,
    /// Series id → the principal whose entry last wrote it (`put_series`; engine entries
    /// such as anchor stamps do not count). Series of the initial document have none.
    pub(crate) series_writer: BTreeMap<String, String>,
    /// Finding id → the principal whose entry last wrote its invalidation (§18.3).
    pub(crate) invalidation_writer: BTreeMap<String, String>,
    /// Bumped whenever `intent_state` is written outside the live path (the live index
    /// keeps a copy of each intent's state, valid for one epoch).
    pub(crate) intent_epoch: u64,
    /// (R3/M7) Disputes and settled-bar requests (§19.9).
    pub(crate) settle: crate::dispute::Settle,
    /// (R3, §20) The capsule engine (`init.capsules`); `None`: no engine.
    pub(crate) runner: Option<crate::capsule::Runner>,
    /// Host-supplied mühür keys (R3, `new_with_trust_keys`, `with_provider_and_trust_keys`, `set_trust_keys`).
    pub(crate) trust_keys: Option<(Vec<u8>, Vec<u8>)>,
    /// (R3, §20) Inside [`Session::as_reader`]: the reader's projection.
    pub(crate) proj: Option<std::rc::Rc<qp_plane::proj::Proj>>,
    /// (R3/perf) Fact positions by id, over the facts sentences read
    /// ([`Session::all_facts`]).
    pub(crate) fact_ix: std::cell::RefCell<qp_plane::capidx::IdIndex>,
    /// (R3/perf) Capsule positions by id and finding (the stored document only).
    pub(crate) cap_ix: std::cell::RefCell<qp_plane::capidx::IdIndex>,
    /// (R3/perf) Capsule sentences by exact realizer input.
    pub(crate) sentences: std::cell::RefCell<qp_plane::capidx::SentenceMemo>,
    /// (RP4/T6) `capsules` items by exact input (entitled readers).
    pub(crate) item_memo: std::cell::RefCell<qp_plane::capidx::ItemMemo>,
    /// (R3/perf) Readers' projections, kept per capsule.
    pub(crate) projs:
        std::cell::RefCell<std::collections::HashMap<String, qp_plane::capidx::ReaderProj>>,
    /// (R3/perf) What [`Session::as_reader`] last gave the most recent readers.
    pub(crate) views: Vec<qp_plane::capidx::ReaderView>,
    /// (R3/perf) What [`Session::headlines`] were last written from.
    pub(crate) head_state: qp_plane::capidx::HeadState,
    /// (R3, §20) Inside [`Session::as_reader`]: the real document's facts.
    pub(crate) real_facts: Vec<qp_schema::Fact>,
    /// (R3, §20) Finding id → the capsule sentence the grammar provider speaks with
    /// (the realizer's, for the current reader inside [`Session::as_reader`]); empty
    /// without an engine and outside a reader with another language.
    pub(crate) headlines: BTreeMap<String, String>,
    /// (R4/H-FP) The exchange calendar of the headline pieces' elapsed phrases for a bar
    /// series whose instrument names none ([`Session::set_headline_calendar`]); `None`:
    /// the built-in `XIST`.
    pub(crate) headline_cal: Option<qp_schema::Calendar>,
    /// (R3, §20) Principals whose capsule preferences a change ever wrote (or the initial
    /// document holds): other readers get documents, changes and log entries without
    /// them.
    pub(crate) audience_seen: BTreeSet<String>,
    /// (R3/K1, §20) The other capsule world ([`crate::public::World`]): with a licensed
    /// flow, the **public world** (its document and log follow the public-only engine)
    /// while the fields above hold the authority's; inside
    /// [`Session::in_world`]`(true, …)` the two are swapped. `None`: no licensed flow.
    pub(crate) other: Option<Box<crate::public::World>>,
    /// (R3/K1) The fields above hold the public world (see `other`).
    pub(crate) in_public: bool,
    /// (R3/K1) The public world's version of the entry being committed, when the writer
    /// made one itself (the engine's look, a command of a public-view reader).
    pub(crate) pending_pub: Option<crate::public::PubNext>,
    /// (R3/K1b) The public world's own capsules (people's, without a story): their §8.1
    /// invalidation is evaluated there.
    pub(crate) pub_caps: BTreeSet<String>,
    /// (R3/K1b) Authority entry `seq` (index `seq - 1`) → the public entry it made.
    pub(crate) full_to_pub: Vec<Option<Seq>>,
    /// (R3/K1b) Authority log length → the public log's length while it had it.
    pub(crate) pub_at: Vec<Seq>,
    /// (R3/K1b) A proposal both worlds hold: the authority's id → the public world's.
    pub(crate) prop_map: BTreeMap<String, String>,
    /// (RP3/M15, §22) Viewer connections: what each was sent (by connection id).
    pub(crate) viewers: BTreeMap<String, crate::viewer::ViewerConn>,
    /// (RP3/M15) The host computes viewer updates after a batch of data events.
    pub(crate) viewer_batch: bool,
    /// (R5/M18, K53; R5/M19) The host's coin-flip rule (`init.feed_rule`,
    /// `Session::set_coin_flip_rule`).
    pub(crate) coin_flip: Option<crate::story::CoinFlipRule>,
    /// (R5/M19) Per-principal rules the host set over its own (`None`: no rule for that
    /// principal; `Session::set_principal_coin_flip_rule`).
    pub(crate) coin_flip_for: BTreeMap<String, Option<crate::story::CoinFlipRule>>,
    /// (R6/T3, §24) The market status the host gave (`Session::set_market_status`);
    /// `None`: never given (every output as before).
    pub(crate) market: Option<qp_schema::market::MarketStatus>,
    /// (R5B, §25) Agent connections and the agent policy.
    pub(crate) agent: crate::agent::AgentState,
    /// (R5B, §25.4) The comment validator's real verdicts (host-side, for measurement;
    /// never sent): [`Session::comment_audit`].
    pub(crate) comment_audit: Vec<crate::comments::CommentAudit>,
    /// (RF1 S3.0) Registered subsystems, their state and their outbox (`qp-subsystem`).
    pub(crate) registry: qp_subsystem::Registry,
    pub(crate) slots: qp_subsystem::Slots,
    pub(crate) outbox: qp_subsystem::Outbox,
    /// The earliest clock a subsystem asked to be woken at ([`Session::next_wake`]).
    pub(crate) wake: Option<i64>,
    /// (RF1 S3.1) The subsystems it was built with: every authority it spawns gets them.
    pub(crate) subs: Subsystems,
    /// How deep the emit step is writing requested entries (audit-s30 B3).
    pub(crate) emit_depth: u32,
}

/// (R3/K1b) A writer of a command, for the two-world paths.
struct Writer<'a> {
    conn: &'a str,
    id: &'a str,
    principal: &'a str,
    author: qp_schema::Author,
    role: Role,
    perms: &'a Permissions,
    public: bool,
}

/// (R3/K1b) A command expanded in the public world as its entry `seq`.
struct PubEx {
    seq: Seq,
    changes: Vec<ChangeOp>,
    inverse: Vec<ChangeOp>,
    created: Vec<String>,
    undoes: Option<Seq>,
}

/// A proposal's acceptance expanded in one world ([`Session::accept`]).
pub(crate) struct Accepted {
    pub(crate) doc: Document,
    pub(crate) changes: Vec<ChangeOp>,
    inverse: Vec<ChangeOp>,
    created: Vec<String>,
    proposer: String,
    reason: Option<String>,
    ops: Vec<Op>,
}

pub(crate) fn out(conn: &str, msg: ServerMsg) -> OutboundLine {
    OutboundLine {
        conn: conn.to_owned(),
        msg,
    }
}

pub(crate) fn err_line(conn: &str, id: Option<String>, error: ProtocolError) -> OutboundLine {
    out(conn, ServerMsg::Error { id, error })
}

/// Ops that cannot become proposals (`docs/PROTOCOL.md` §4.7); a subsystem's op: its
/// handler's answer.
fn proposable(op: &Op, reg: &Registry) -> bool {
    if let Some(h) = reg.op(op) {
        return h.proposable();
    }
    !matches!(
        op,
        Op::AcceptProposal { .. }
            | Op::RejectProposal { .. }
            | Op::Undo { .. }
            | Op::AppendBars { .. }
            | Op::UpdateLastBar { .. }
            // (R3, §20) Personal preferences: written directly by the person (or an owner),
            // never carried in a proposal that every reader would see.
            | Op::SetAudience { .. }
            | Op::SetSensitivity { .. }
            // (R5B, §25.4) Comments are written directly, never proposed.
            | Op::AddComment { .. }
    )
}

/// Whether `perms` allow writing `op` directly (a subsystem's op: its handler's answer).
fn allowed(op: &Op, perms: &Permissions, reg: &Registry) -> bool {
    if let Some(h) = reg.op(op) {
        return h.allowed(perms);
    }
    match op {
        Op::AppendBars { .. } | Op::UpdateLastBar { .. } => perms.feed || perms.write,
        Op::ApproveFinding { .. }
        | Op::RejectFinding { .. }
        | Op::AcceptProposal { .. }
        | Op::RejectProposal { .. } => perms.decide,
        // (R3, §20) A person's own capsule preferences: anyone who may read (another
        // person's: owner only, checked in the expansion).
        Op::SetAudience { .. } | Op::SetSensitivity { .. } => perms.read,
        // (R5B, §25.4) A comment: anyone who may propose or write.
        Op::AddComment { .. } => perms.propose || perms.write,
        _ => perms.write,
    }
}

impl Session {
    /// Starts an authority with subsystems `subs` (the composition, `qp-session`, gives the
    /// standard ones). A `DocumentRef::Path` is resolved under `repo_root` and may not leave
    /// it.
    ///
    /// # Errors
    /// The document cannot be read or parsed; a subsystem's kind is registered twice.
    pub fn new(init: SessionInit, repo_root: &Path, subs: Subsystems) -> Result<Self, String> {
        Self::new_inner(init, repo_root, None, subs)
    }

    /// (R3, §20) [`Session::new`] with the host's secret mühür keys (receipt key, audit-log
    /// key) in place before the capsule runner starts, so no server comment is ever made
    /// with the public demo keys.
    ///
    /// # Errors
    /// As [`Session::new`].
    pub fn new_with_trust_keys(
        init: SessionInit,
        repo_root: &Path,
        receipt_key: Vec<u8>,
        audit_key: Vec<u8>,
        subs: Subsystems,
    ) -> Result<Self, String> {
        Self::new_inner(init, repo_root, Some((receipt_key, audit_key)), subs)
    }

    fn new_inner(
        init: SessionInit,
        repo_root: &Path,
        keys: Option<(Vec<u8>, Vec<u8>)>,
        subs: Subsystems,
    ) -> Result<Self, String> {
        let provider = match &init.data {
            Some(d) => Provider::Memory(
                qp_source::MemoryProvider::from_init(d, repo_root)
                    .map_err(|e| format!("init.data: {e}"))?,
            ),
            None => Provider::None,
        };
        let settled = init.data.as_ref().and_then(|d| d.settled.clone());
        let mut s = Self::build_keys(init, repo_root, provider, keys, subs)?;
        if let Some(t) = settled {
            s.settle.set_test_source(&t);
        }
        Ok(s)
    }

    /// Starts an authority whose bound series are resolved by the host's `provider`
    /// (§18.3) instead of `init.data` (which must then be absent).
    ///
    /// # Errors
    /// As [`Session::new`]; `init.data` given as well.
    pub fn with_provider(
        init: SessionInit,
        repo_root: &Path,
        provider: Box<dyn qp_source::SourceProvider>,
        subs: Subsystems,
    ) -> Result<Self, String> {
        if init.data.is_some() {
            return Err("give either init.data or a host provider, not both".to_owned());
        }
        Self::build_keys(init, repo_root, Provider::Host(provider), None, subs)
    }

    /// (R3, §20) [`Session::with_provider`] with the host's secret mühür keys (see
    /// [`Session::new_with_trust_keys`]); the constructor a real host with licensed data
    /// uses.
    ///
    /// # Errors
    /// As [`Session::with_provider`].
    pub fn with_provider_and_trust_keys(
        init: SessionInit,
        repo_root: &Path,
        provider: Box<dyn qp_source::SourceProvider>,
        receipt_key: Vec<u8>,
        audit_key: Vec<u8>,
        subs: Subsystems,
    ) -> Result<Self, String> {
        if init.data.is_some() {
            return Err("give either init.data or a host provider, not both".to_owned());
        }
        let keys = Some((receipt_key, audit_key));
        Self::build_keys(init, repo_root, Provider::Host(provider), keys, subs)
    }

    fn build_keys(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
        keys: Option<(Vec<u8>, Vec<u8>)>,
        subs: Subsystems,
    ) -> Result<Self, String> {
        let schema = schemars::schema_for!(ClientMsg).to_value();
        Self::build_with_keys(init, repo_root, provider, schema, keys, subs)
    }

    /// A session built with the inbound schema given (`Null` for a follower that never
    /// handles inbound messages: the replica, so that a WASM build does not carry the
    /// schema generator).
    pub(crate) fn build_with(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
        schema: Value,
        subs: Subsystems,
    ) -> Result<Self, String> {
        Self::build_with_keys(init, repo_root, provider, schema, None, subs)
    }

    /// Switches the incremental live path (§18.8) on (default) or off. Off, every bar
    /// change clears the values cache and every watcher and live capsule is evaluated
    /// after it: the from-scratch reference the incremental path is tested against
    /// (outputs, log and document are byte-identical). For tests and measurements.
    #[doc(hidden)]
    pub fn set_incremental(&mut self, on: bool) {
        self.incremental = on;
        self.cache.clear();
    }

    /// The current document.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// The initial document (before any log entry).
    #[must_use]
    pub fn initial_document(&self) -> &Document {
        &self.initial
    }

    /// The log.
    #[must_use]
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    /// The last clock value.
    #[must_use]
    pub fn clock(&self) -> i64 {
        self.clock
    }

    /// The limits this authority enforces.
    #[must_use]
    pub fn limits(&self) -> &Limits {
        &self.limits
    }
}

/// Applies `changes` to `doc` in place; if one does not fit, the ones applied before it
/// are undone with their inverses, so `doc` is left as it was (the same bytes, as undo
/// guarantees).
fn apply_or_roll_back(doc: &mut Document, changes: &[ChangeOp]) {
    let mut inverses = Vec::with_capacity(changes.len());
    for c in changes {
        match qp_ops::apply::apply_change(doc, c) {
            Ok(inv) => inverses.push(inv),
            Err(_) => {
                for inv in inverses.iter().rev() {
                    let _ = qp_ops::apply::apply_change(doc, inv);
                }
                return;
            }
        }
    }
}

fn ack(id: String, status: AckStatus) -> ServerMsg {
    ServerMsg::Ack {
        resolved: std::collections::BTreeMap::new(),
        id,
        status,
        seq: None,
        created: Vec::new(),
        proposal: None,
        undoes: None,
    }
}

fn forbidden_read() -> ProtocolError {
    E::new(ErrorCode::Forbidden, "this principal may not read")
        .hint("Reading needs a role with the read permission.")
        .build()
}

pub(crate) fn role_str(r: Role) -> &'static str {
    match r {
        Role::Owner => "owner",
        Role::Editor => "editor",
        Role::Proposer => "proposer",
        Role::Viewer => "viewer",
        Role::Feed => "feed",
        Role::Host => "host",
    }
}

fn msg_id(m: &ClientMsg) -> Option<String> {
    match m {
        ClientMsg::Command { id, .. }
        | ClientMsg::Query { id, .. }
        | ClientMsg::Subscribe { id, .. }
        | ClientMsg::Unsubscribe { id, .. }
        | ClientMsg::Input { id, .. }
        | ClientMsg::Attend { id, .. }
        | ClientMsg::Dispute { id, .. }
        | ClientMsg::Visible { id, .. }
        | ClientMsg::Replay { id, .. } => Some(id.clone()),
        ClientMsg::Hello { .. } | ClientMsg::Clock { .. } => None,
    }
}
