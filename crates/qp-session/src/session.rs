//! The reference authority: one document, its log, connections, subscriptions and
//! presence. Every inbound message goes through [`Session::handle_raw`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use qp_schema::protocol::{
    AckStatus, AttentionDirective, ChangeOp, ClientMsg, DocumentRef, ErrorCode, Event, Limits,
    LogEntry, Op, OutboundLine, Permissions, Principal, Profile, Proposal, ProposalState,
    ProtocolError, Role, Seq, ServerMsg, SessionInit, Surface, SurfaceKind, Target, Topic,
    Viewport, WriteMode, PROTOCOL_VERSION,
};
use qp_schema::{Author, CapsuleState, Document};
use serde_json::Value;

use crate::apply::{apply_all, first_overlap, touched_ids, touched_key, touched_keys};
use crate::error::{invalid, ptr, PResult, E};
use crate::expand::{
    check_condition, expand_ops, need_finding, need_proposal, summarize_ops, undone_set, Ctx,
};
use crate::locate::locate;
use crate::plane::{reading_doc, DataError, DataOutcome, DataPlane, Provider, Reader};
use crate::present::profile_for;
use crate::public::{PubNext, PubPending};
use crate::select;
use crate::values::{eval_condition, holds, Cache};

/// Principal id of the engine when it writes log entries itself.
pub const ENGINE_PRINCIPAL: &str = "qp";

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
    /// View id -> visible range.
    pub viewports: BTreeMap<String, (u64, u64)>,
    /// Last viewport set by input.
    pub current: Option<Viewport>,
    /// Input counter value of the last input (0 = none).
    pub last_input: u64,
    /// Clock at the last input.
    pub last_at: Option<i64>,
    /// (R4, §7) View state: open capsule, its depth and chart range, frame sizes.
    pub look: qp_schema::protocol::ViewState,
}

impl Conn {
    pub(crate) fn has_screen(&self) -> bool {
        matches!(
            self.surface.kind,
            SurfaceKind::Desktop | SurfaceKind::Phone | SurfaceKind::Watch
        )
    }
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
    pub(crate) cache: Cache,
    pub(crate) intent_state: BTreeMap<String, bool>,
    pub(crate) directives: BTreeMap<String, Vec<AttentionDirective>>,
    pub(crate) focus: BTreeMap<String, Target>,
    pub(crate) att_counter: u64,
    pub(crate) input_counter: u64,
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
    pub(crate) pending_views: Vec<(String, crate::values::Delta, Seq)>,
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
    pub(crate) proj: Option<std::rc::Rc<crate::capsule::Proj>>,
    /// (R3/perf) Fact positions by id, over the facts sentences read
    /// ([`Session::all_facts`]).
    pub(crate) fact_ix: std::cell::RefCell<crate::capidx::IdIndex>,
    /// (R3/perf) Capsule positions by id and finding (the stored document only).
    pub(crate) cap_ix: std::cell::RefCell<crate::capidx::IdIndex>,
    /// (R3/perf) Capsule sentences by exact realizer input.
    pub(crate) sentences: std::cell::RefCell<crate::capidx::SentenceMemo>,
    /// (RP4/T6) `capsules` items by exact input (entitled readers).
    pub(crate) item_memo: std::cell::RefCell<crate::capidx::ItemMemo>,
    /// (R3/perf) Readers' projections, kept per capsule.
    pub(crate) projs:
        std::cell::RefCell<std::collections::HashMap<String, crate::capidx::ReaderProj>>,
    /// (R3/perf) What [`Session::as_reader`] last gave the most recent readers.
    pub(crate) views: Vec<crate::capidx::ReaderView>,
    /// (R3/perf) What [`Session::headlines`] were last written from.
    pub(crate) head_state: crate::capidx::HeadState,
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
struct Accepted {
    doc: Document,
    changes: Vec<ChangeOp>,
    inverse: Vec<ChangeOp>,
    created: Vec<String>,
    proposer: String,
    reason: Option<String>,
    ops: Vec<Op>,
}

/// Resolves `rel` under `root`, refusing absolute paths and paths that leave `root`.
///
/// # Errors
/// The path escapes the root or does not exist.
pub fn confine(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute()
        || p.components()
            .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "path `{rel}` must be relative and stay inside the root"
        ));
    }
    let root_c = root
        .canonicalize()
        .map_err(|e| format!("root {}: {e}", root.display()))?;
    let full = root_c
        .join(p)
        .canonicalize()
        .map_err(|e| format!("{rel}: {e}"))?;
    if !full.starts_with(&root_c) {
        return Err(format!("path `{rel}` leaves the root"));
    }
    Ok(full)
}

/// Reads a JSON file into `T`.
///
/// # Errors
/// I/O or JSON errors.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
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

/// Ops that cannot become proposals (`docs/PROTOCOL.md` §4.7).
fn proposable(op: &Op) -> bool {
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
    )
}

fn op_name(op: &Op) -> String {
    serde_json::to_value(op)
        .ok()
        .and_then(|v| v.get("op").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

/// Whether `perms` allow writing `op` directly.
fn allowed(op: &Op, perms: &Permissions) -> bool {
    match op {
        Op::AppendBars { .. } | Op::UpdateLastBar { .. } => perms.feed || perms.write,
        Op::ApproveFinding { .. }
        | Op::RejectFinding { .. }
        | Op::AcceptProposal { .. }
        | Op::RejectProposal { .. } => perms.decide,
        // (R3, §20) A person's own capsule preferences: anyone who may read (another
        // person's: owner only, checked in the expansion).
        Op::SetAudience { .. } | Op::SetSensitivity { .. } => perms.read,
        _ => perms.write,
    }
}

impl Session {
    /// Starts an authority. A `DocumentRef::Path` is resolved under `repo_root` and may
    /// not leave it.
    ///
    /// # Errors
    /// The document cannot be read or parsed.
    pub fn new(init: SessionInit, repo_root: &Path) -> Result<Self, String> {
        Self::new_inner(init, repo_root, None)
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
    ) -> Result<Self, String> {
        Self::new_inner(init, repo_root, Some((receipt_key, audit_key)))
    }

    fn new_inner(
        init: SessionInit,
        repo_root: &Path,
        keys: Option<(Vec<u8>, Vec<u8>)>,
    ) -> Result<Self, String> {
        let provider = match &init.data {
            Some(d) => Provider::Memory(
                qp_source::MemoryProvider::from_init(d, repo_root)
                    .map_err(|e| format!("init.data: {e}"))?,
            ),
            None => Provider::None,
        };
        let settled = init.data.as_ref().and_then(|d| d.settled.clone());
        let mut s = Self::build_keys(init, repo_root, provider, keys)?;
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
    ) -> Result<Self, String> {
        if init.data.is_some() {
            return Err("give either init.data or a host provider, not both".to_owned());
        }
        Self::build(init, repo_root, Provider::Host(provider))
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
    ) -> Result<Self, String> {
        if init.data.is_some() {
            return Err("give either init.data or a host provider, not both".to_owned());
        }
        Self::build_keys(
            init,
            repo_root,
            Provider::Host(provider),
            Some((receipt_key, audit_key)),
        )
    }

    pub(crate) fn build(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
    ) -> Result<Self, String> {
        Self::build_keys(init, repo_root, provider, None)
    }

    fn build_keys(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
        keys: Option<(Vec<u8>, Vec<u8>)>,
    ) -> Result<Self, String> {
        let schema = schemars::schema_for!(ClientMsg).to_value();
        Self::build_with_keys(init, repo_root, provider, schema, keys)
    }

    /// [`Session::build`] with the inbound schema given (`Null` for a follower that never
    /// handles inbound messages: the replica, so that a WASM build does not carry the
    /// schema generator).
    pub(crate) fn build_with(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
        schema: Value,
    ) -> Result<Self, String> {
        Self::build_with_keys(init, repo_root, provider, schema, None)
    }

    fn build_with_keys(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
        schema: Value,
        trust_keys: Option<(Vec<u8>, Vec<u8>)>,
    ) -> Result<Self, String> {
        let capsules = init.capsules.clone();
        // (R5/M19, K53) The host's feed rule, by name.
        let coin_flip = match &init.feed_rule {
            None => None,
            Some(f) => Some(
                qp_capsule::feedrule::FeedRule::named(&f.rule, f.exempt_negative).ok_or_else(
                    || {
                        let names: Vec<&str> = qp_capsule::feedrule::FEED_RULES
                            .iter()
                            .map(|(n, _)| *n)
                            .collect();
                        format!(
                            "init.feed_rule.rule: unknown rule `{}` (one of {})",
                            f.rule,
                            names.join(", ")
                        )
                    },
                )?,
            ),
        };
        // (RP4) The capsule engine's named constant record.
        if let Some(name) = capsules.as_ref().and_then(|c| c.engine.as_deref()) {
            if qp_capsule::EngineConfig::named(name).is_none() {
                return Err(format!(
                    "init.capsules.engine: unknown record `{name}` (one of {})",
                    qp_capsule::ENGINES.join(", ")
                ));
            }
        }
        let doc: Document = match init.document {
            DocumentRef::Inline(d) => *d,
            DocumentRef::Path(p) => read_json(&confine(repo_root, &p)?)?,
        };
        // Canonical form of every view (GRAMMAR.md §11); the log replays from it.
        let doc = crate::canon::canon_document_owned(doc)?;
        let limits = default_limits();
        let host = init
            .principals
            .iter()
            .find(|p| p.role == Role::Host)
            .map(|p| p.id.clone());
        let plane = DataPlane::new(provider, host, limits.max_bind_bars);
        let mut s = Self {
            initial: doc.clone(),
            doc,
            log: Vec::new(),
            principals: init.principals,
            clock: init.clock,
            epoch: init.epoch,
            ttl: init.proposal_ttl_ms,
            limits,
            conns: Vec::new(),
            cache: Cache::default(),
            intent_state: BTreeMap::new(),
            directives: BTreeMap::new(),
            focus: BTreeMap::new(),
            att_counter: 0,
            input_counter: 0,
            schema,
            plane,
            rd: None,
            pending_stamps: Vec::new(),
            incremental: true,
            pending_views: Vec::new(),
            view_reads: None,
            claims: crate::claims::Claims::default(),
            struct_gen: 1,
            live_index: None,
            series_writer: BTreeMap::new(),
            invalidation_writer: BTreeMap::new(),
            intent_epoch: 1,
            settle: crate::dispute::Settle::default(),
            runner: None,
            trust_keys,
            proj: None,
            fact_ix: std::cell::RefCell::default(),
            cap_ix: std::cell::RefCell::default(),
            sentences: std::cell::RefCell::default(),
            item_memo: std::cell::RefCell::default(),
            projs: std::cell::RefCell::default(),
            head_state: crate::capidx::HeadState::default(),
            views: Vec::new(),
            real_facts: Vec::new(),
            headlines: BTreeMap::new(),
            headline_cal: None,
            audience_seen: BTreeSet::new(),
            other: None,
            in_public: false,
            pending_pub: None,
            pub_caps: BTreeSet::new(),
            full_to_pub: Vec::new(),
            pub_at: Vec::new(),
            prop_map: BTreeMap::new(),
            viewers: BTreeMap::new(),
            viewer_batch: false,
            coin_flip,
            coin_flip_for: BTreeMap::new(),
            market: None,
            agent: crate::agent::AgentState::default(),
        };
        s.audience_seen = s.doc.audiences.keys().cloned().collect();
        s.start_capsules(capsules.as_ref());
        // Resolve the bindings on load (§18.3); a first ready resolution is anchored by
        // an engine entry. No connection exists yet: its notifications go nowhere.
        s.pending_stamps = s.plane.sync(&s.doc);
        s.rd = reading_doc(&s.doc, &s.plane);
        if let Some(w) = s.other.as_mut() {
            w.rd = reading_doc(&w.doc, &s.plane);
        }
        if let Some(seq) = s.stamp_anchors() {
            let _ = s.invalidations(seq);
        }
        s.eval_intents(s.seq());
        let _ = s.eval_intents_public();
        // The capsule engine catches up with the final bars at load (R3, §20).
        let _ = s.run_capsules();
        s.pending_views.clear();
        // No connection exists at load: claims asserted now are registered (for their
        // finalisation) but announce nothing.
        let _ = s.claims.take();
        Ok(s)
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

    /// One line of the line transport (`{"conn":…,"msg":…}`). `msg` may be a JSON
    /// object (the message) or a JSON string (the raw text of the message, e.g. to send
    /// invalid JSON). Framing errors go to connection `""`.
    pub fn handle_frame(&mut self, line: &str) -> Vec<OutboundLine> {
        let frame_err = |code: ErrorCode, msg: String, path: Option<&str>| {
            let mut e = E::new(code, msg);
            if let Some(p) = path {
                e = e.at(p);
            }
            vec![err_line("", None, e.build())]
        };
        let limit = usize::try_from(self.limits.max_message_bytes)
            .unwrap_or(usize::MAX)
            .saturating_add(4096);
        if line.len() > limit {
            return frame_err(
                ErrorCode::Limit,
                format!("line exceeds {limit} bytes"),
                None,
            );
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return frame_err(ErrorCode::Parse, format!("frame is not JSON: {e}"), None),
        };
        let Some(conn) = v.get("conn").and_then(Value::as_str) else {
            return frame_err(
                ErrorCode::Schema,
                "frame needs a string `conn`".to_owned(),
                Some("/conn"),
            );
        };
        if let Some(k) = v
            .as_object()
            .and_then(|o| o.keys().find(|k| *k != "conn" && *k != "msg"))
        {
            let e = E::new(ErrorCode::Schema, format!("unknown frame field `{k}`"))
                .at(ptr("", k))
                .expected(["conn", "msg"])
                .build();
            return vec![err_line(conn, None, e)];
        }
        let text = match v.get("msg") {
            None => {
                let e = E::new(ErrorCode::Schema, "frame needs `msg`")
                    .at("/msg")
                    .build();
                return vec![err_line(conn, None, e)];
            }
            Some(Value::String(s)) => s.clone(),
            Some(m) => m.to_string(),
        };
        self.handle_raw(conn, &text)
    }

    /// A host frame of the line transport (`{"host":{"data":<DataEvent>}}`, §3, §18.4):
    /// transport level and host only — `qp serve` accepts it only with `--host-frames`,
    /// on the host's own stream. Not a protocol message; it never enters the log.
    /// Returns the notifications the event causes and the notices for the host (a gap,
    /// a binding that became stale; `qp serve` writes them as `{"host":{"gap":…}}`).
    ///
    /// # Errors
    /// A malformed frame or event, or a [`DataError`], as text (`qp serve` writes it as
    /// `{"host":{"error":…}}`).
    pub fn handle_host_frame(&mut self, line: &str) -> Result<DataOutcome, String> {
        let v: Value =
            serde_json::from_str(line).map_err(|e| format!("host frame is not JSON: {e}"))?;
        let host = v
            .as_object()
            .filter(|o| o.len() == 1)
            .and_then(|o| o.get("host"))
            .and_then(Value::as_object)
            .ok_or("a host frame is {\"host\":{\"data\":…}}")?;
        if host.len() != 1 {
            return Err("a host frame carries exactly one of: data".to_owned());
        }
        let ev = host
            .get("data")
            .ok_or("unknown host frame (expected `data`)")?;
        let ev = <qp_schema::data::DataEvent as serde::Deserialize>::deserialize(ev)
            .map_err(|e| format!("data event: {e}"))?;
        self.host_data_event(&ev).map_err(|e| e.to_string())
    }

    /// THE boundary: one raw inbound message on connection `conn`. Checks the size,
    /// parses (errors `parse`, `schema` with a JSON pointer, `limit`), then handles it.
    pub fn handle_raw(&mut self, conn: &str, line: &str) -> Vec<OutboundLine> {
        if line.len() as u64 > self.limits.max_message_bytes {
            let e = E::new(
                ErrorCode::Limit,
                format!(
                    "message is {} bytes; the limit is {}",
                    line.len(),
                    self.limits.max_message_bytes
                ),
            )
            .expected([self.limits.max_message_bytes.to_string()])
            .build();
            return vec![err_line(conn, None, e)];
        }
        if let Some(msg) = self.bar_fast_parse(line) {
            let mut lines = match self.check_counts(&msg) {
                Ok(()) => self.handle_resolved(conn, msg, BTreeMap::new()),
                Err(e) => vec![err_line(conn, msg_id(&msg), e)],
            };
            if self.has_pushers() {
                lines.extend(self.viewer_flush());
            }
            return lines;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let text = e.to_string();
                let code = if text.contains("recursion limit") {
                    ErrorCode::Limit
                } else {
                    ErrorCode::Parse
                };
                let pe = E::new(code, format!("not valid JSON: {text}"))
                    .hint("Send one JSON object per message.")
                    .build();
                return vec![err_line(conn, None, pe)];
            }
        };
        let id = value.get("id").and_then(Value::as_str).map(str::to_owned);
        let mut value = value;
        // Reference by meaning (§17): selectors become ids before the typed parse, so
        // the typed message, the log and proposals only ever hold ids.
        let reader = self
            .conn_index(conn)
            .map(|ci| (self.conns[ci].principal.clone(), self.lang_of(ci)));
        // (R4/S1) A `capsule` query by selector: kept to say what it may also match that
        // the reader cannot see (`CapsuleDetail.withheld`).
        let capsule_sel = (value.get("type").and_then(Value::as_str) == Some("query"))
            .then(|| value.pointer("/query/capsule").filter(|v| v.is_object()))
            .flatten()
            .and_then(|v| {
                serde_json::from_value::<qp_schema::protocol::CapsuleSel>(v.clone()).ok()
            });
        let resolved = match match reader {
            Some((p, lang)) => self.as_reader(&p, lang, |s| s.resolve_selectors(conn, &mut value)),
            None => self.resolve_selectors(conn, &mut value),
        } {
            Ok(r) => r,
            Err(e) => return vec![err_line(conn, id, e)],
        };
        // Plain typed parse first (the common case); the path-tracking parse runs only to
        // report an error (same error as before).
        let msg: ClientMsg = match <ClientMsg as serde::Deserialize>::deserialize(&value) {
            Ok(m) => m,
            Err(_) => match serde_path_to_error::deserialize(&value) {
                Ok(m) => m,
                Err(e) => return vec![err_line(conn, id, self.schema_error(&value, &e))],
            },
        };
        if let Err(e) = self.check_limits(&value, &msg) {
            return vec![err_line(conn, id, e)];
        }
        let mut lines = self.handle_resolved(conn, msg, resolved);
        // (RP3/M15, §22) What the message changed for viewers (their feed, faces, slice).
        if self.has_pushers() {
            lines.extend(self.viewer_flush());
        }
        let asker = self
            .conn_index(conn)
            .map(|ci| self.conns[ci].principal.clone());
        if let (Some(sel), Some(principal)) = (capsule_sel, asker) {
            for l in &mut lines {
                if let ServerMsg::Result {
                    data: qp_schema::protocol::QueryResult::CapsuleDetail { detail },
                    ..
                } = &mut l.msg
                {
                    detail.withheld = self.withheld_for(&principal, &sel, &detail.item);
                }
            }
        }
        lines
    }

    /// The live tick's fast parse (§18.8): a `command` whose ops are all bar writes
    /// (`append_bars`, `update_last_bar`), parsed straight into the typed message. Such a
    /// command cannot hold a selector (its id field, `series`, is a string: a selector
    /// object fails this parse) and, when the line is not longer than `max_string`, no
    /// string in it can break that limit; so the general path would resolve nothing and
    /// check nothing more. Anything else (or any parse error) takes the general path,
    /// which reports errors exactly as before.
    fn bar_fast_parse(&self, line: &str) -> Option<ClientMsg> {
        if line.len() > self.limits.max_string as usize
            || !(line.contains("\"append_bars\"") || line.contains("\"update_last_bar\""))
        {
            return None;
        }
        let msg: ClientMsg = serde_json::from_str(line).ok()?;
        match &msg {
            ClientMsg::Command { ops, .. }
                if !ops.is_empty()
                    && ops
                        .iter()
                        .all(|o| matches!(o, Op::AppendBars { .. } | Op::UpdateLastBar { .. })) =>
            {
                Some(msg)
            }
            _ => None,
        }
    }

    /// Finds the selectors of a raw message, resolves them against the document (for a
    /// command: op by op, each against the document as the earlier ops of the same
    /// command leave it) and writes the ids in their place. Returns `{pointer -> id}`.
    fn resolve_selectors(&self, conn: &str, msg: &mut Value) -> PResult<BTreeMap<String, String>> {
        let mut out = BTreeMap::new();
        let groups = select::sites(msg);
        let Some(first) = groups
            .iter()
            .flatten()
            .find(|s| select::is_selector(msg, s))
            .cloned()
        else {
            return Ok(out);
        };
        let Some(ci) = self.conn_index(conn) else {
            return Err(E::new(
                ErrorCode::Handshake,
                "the first message on a connection must be `hello`",
            )
            .hint("Send {\"type\":\"hello\",\"protocol\":\"qp/0\",\"principal\":…,\"surface\":{\"kind\":…}} first.")
            .build());
        };
        let role = self.conns[ci].role;
        let perms = permissions(role);
        if !perms.read {
            return Err(E::new(
                ErrorCode::Forbidden,
                format!(
                    "role `{}` may not read the document, so it cannot use selectors",
                    role_str(role)
                ),
            )
            .at(first.path)
            .hint("Use the item's id instead of a selector.")
            .build());
        }
        let is_command = msg.get("type").and_then(Value::as_str) == Some("command");
        if !is_command {
            let r = select::Resolver {
                doc: rd!(self),
                root: Some(&self.schema),
            };
            for g in &groups {
                select::resolve_group(&r, msg, g, &mut out)?;
            }
            return Ok(out);
        }
        if groups.len() > self.limits.max_ops as usize {
            // The op limit is reported after parsing.
            return Ok(out);
        }
        // Op by op on a scratch copy: a selector may name an item an earlier op of the
        // same command creates or changes. A failing earlier op leaves the copy as it
        // was; the command itself reports that failure.
        let principal = self.conns[ci].principal.clone();
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let seq = self.seq() + 1;
        let mut ctx = Ctx {
            author,
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: &principal,
            is_owner: role == Role::Owner,
            can_decide: perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        // Selectors read no bars (§17): they resolve over the document itself, never the
        // reading document, so a scratch copy never copies bound bars (`add_fact` reads
        // them through `ctx.data`).
        // The live document is used until an op before the last selector changes it;
        // only then is one scratch copy made, and later ops change that copy in place
        // (a failing op is rolled back with the inverses of its applied changes). No
        // copy at all for a message whose selectors are all in its first op that
        // changes nothing before them, e.g. a one-op command.
        let mut scratch: Option<Document> = None;
        let last = groups
            .iter()
            .rposition(|g| g.iter().any(|s| select::is_selector(msg, s)))
            .unwrap_or(0);
        for (i, g) in groups.iter().enumerate().take(last + 1) {
            if g.iter().any(|s| select::is_selector(msg, s)) {
                let r = select::Resolver {
                    doc: scratch.as_ref().unwrap_or(&self.doc),
                    root: Some(&self.schema),
                };
                select::resolve_group(&r, msg, g, &mut out)?;
            }
            if i == last {
                break;
            }
            let p = ptr("/ops", i);
            let Some(op) = msg
                .pointer(&p)
                .and_then(|v| serde_json::from_value::<Op>(v.clone()).ok())
            else {
                continue;
            };
            let cur = scratch.as_ref().unwrap_or(&self.doc);
            if let Ok(cs) = crate::expand::expand_op(cur, &op, &p, &mut ctx) {
                if cs.is_empty() {
                    continue;
                }
                let doc = scratch.get_or_insert_with(|| self.doc.clone());
                apply_or_roll_back(doc, &cs);
            }
        }
        Ok(out)
    }

    fn schema_error(
        &self,
        value: &Value,
        e: &serde_path_to_error::Error<serde_json::Error>,
    ) -> ProtocolError {
        if let Some(v) = locate(value, &self.schema, &self.schema) {
            return select::violation_error(&v, "");
        }
        let mut path = String::new();
        for seg in e.path().iter() {
            match seg {
                serde_path_to_error::Segment::Seq { index } => path = ptr(&path, index),
                serde_path_to_error::Segment::Map { key } => path = ptr(&path, key),
                serde_path_to_error::Segment::Enum { .. }
                | serde_path_to_error::Segment::Unknown => {}
            }
        }
        let inner = e.inner().to_string();
        let inner = inner.split(" at line ").next().unwrap_or(&inner).to_owned();
        E::new(ErrorCode::Schema, inner).at(path).build()
    }

    fn check_limits(&self, value: &Value, msg: &ClientMsg) -> PResult<()> {
        fn walk(v: &Value, path: &str, max: usize) -> Option<String> {
            match v {
                Value::String(s) if s.len() > max => Some(path.to_owned()),
                Value::Array(a) => a
                    .iter()
                    .enumerate()
                    .find_map(|(i, x)| walk(x, &ptr(path, i), max)),
                Value::Object(o) => o.iter().find_map(|(k, x)| walk(x, &ptr(path, k), max)),
                _ => None,
            }
        }
        /// True if a string longer than `max` is anywhere in `v` (no allocation; the
        /// path is built only when there is one).
        fn any_long(v: &Value, max: usize) -> bool {
            match v {
                Value::String(s) => s.len() > max,
                Value::Array(a) => a.iter().any(|x| any_long(x, max)),
                Value::Object(o) => o.values().any(|x| any_long(x, max)),
                _ => false,
            }
        }
        let max_string = self.limits.max_string as usize;
        if let Some(p) = any_long(value, max_string)
            .then(|| walk(value, "", max_string))
            .flatten()
        {
            return Err(E::new(
                ErrorCode::Limit,
                format!("string longer than {max_string} bytes"),
            )
            .at(p)
            .expected([max_string.to_string()])
            .build());
        }
        self.check_counts(msg)
    }

    /// The op and bar count limits of a command.
    fn check_counts(&self, msg: &ClientMsg) -> PResult<()> {
        if let ClientMsg::Command { ops, .. } = msg {
            if ops.len() > self.limits.max_ops as usize {
                return Err(E::new(
                    ErrorCode::Limit,
                    format!("{} ops; the limit is {}", ops.len(), self.limits.max_ops),
                )
                .at("/ops")
                .expected([self.limits.max_ops.to_string()])
                .build());
            }
            for (i, op) in ops.iter().enumerate() {
                let (n, path) = match op {
                    Op::AppendBars { bars, .. } => (bars.len(), "bars"),
                    Op::PutSeries { series } => match &series.data {
                        Some(qp_schema::SeriesData::Bars { bars }) => (bars.len(), "series"),
                        Some(qp_schema::SeriesData::Points { points }) => (points.len(), "series"),
                        None => continue,
                    },
                    _ => continue,
                };
                if n > self.limits.max_bars as usize {
                    return Err(E::new(
                        ErrorCode::Limit,
                        format!("{n} bars; the limit is {}", self.limits.max_bars),
                    )
                    .at(ptr(&ptr("/ops", i), path))
                    .expected([self.limits.max_bars.to_string()])
                    .build());
                }
            }
        }
        Ok(())
    }

    fn conn_index(&self, conn: &str) -> Option<usize> {
        self.conns.iter().position(|c| c.id == conn)
    }

    pub(crate) fn principal(&self, id: &str) -> Option<&Principal> {
        self.principals.iter().find(|p| p.id == id)
    }

    /// Principal id whose author is `a` (fallback: the author's name).
    pub(crate) fn principal_of(&self, a: &Author) -> String {
        if let Some(p) = self.principals.iter().find(|p| &p.author == a) {
            return p.id.clone();
        }
        match a {
            Author::Human { name } | Author::Agent { name, .. } | Author::Engine { name, .. } => {
                name.clone()
            }
        }
    }

    /// Handles a parsed message (its id fields hold ids; selectors are resolved by
    /// [`Session::handle_raw`]).
    pub fn handle(&mut self, conn: &str, msg: ClientMsg) -> Vec<OutboundLine> {
        self.handle_resolved(conn, msg, BTreeMap::new())
    }

    /// Handles a parsed message whose selectors resolved to `resolved` (§17).
    fn handle_resolved(
        &mut self,
        conn: &str,
        msg: ClientMsg,
        resolved: BTreeMap<String, String>,
    ) -> Vec<OutboundLine> {
        let ci = self.conn_index(conn);
        if let ClientMsg::Hello {
            protocol,
            principal,
            surface,
            features,
        } = msg
        {
            return self.hello(conn, ci, &protocol, &principal, surface, &features);
        }
        let Some(ci) = ci else {
            let id = msg_id(&msg);
            let e = E::new(
                ErrorCode::Handshake,
                "the first message on a connection must be `hello`",
            )
            .hint("Send {\"type\":\"hello\",\"protocol\":\"qp/0\",\"principal\":…,\"surface\":{\"kind\":…}} first.")
            .build();
            return vec![err_line(conn, id, e)];
        };
        match msg {
            ClientMsg::Hello { .. } => unreachable!("handled above"),
            ClientMsg::Command {
                id,
                ops,
                mode,
                base,
                reason,
            } => match self.command(ci, &id, &ops, mode, (base, reason), resolved) {
                Ok(lines) => lines,
                Err(e) => vec![err_line(conn, Some(id), e)],
            },
            ClientMsg::Query { id, query } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                match self.as_reader(&principal, lang, |s| s.query(ci, &query)) {
                    Ok(data) => vec![out(
                        conn,
                        ServerMsg::Result {
                            id,
                            seq: self.seq_to(&principal, self.seq()),
                            now: self.clock(),
                            data,
                            resolved,
                        },
                    )],
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Subscribe { id, topic } => {
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                let viewer = match &topic {
                    Topic::Viewer { limit, width } => Some((*limit, *width)),
                    _ => None,
                };
                let stories = matches!(topic, Topic::Stories { .. });
                let r = self.as_reader(&principal, lang, |s| s.subscribe(ci, &id, topic));
                match r {
                    Ok(()) => {
                        let mut a = ack(id.clone(), AckStatus::Subscribed);
                        if let ServerMsg::Ack { resolved: r, .. } = &mut a {
                            *r = resolved;
                        }
                        let mut v = vec![out(conn, a)];
                        if let Some((limit, width)) = viewer {
                            v.extend(self.viewer_start(ci, &id, limit, width));
                        }
                        if stories {
                            self.stories_start(ci, &id);
                        }
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Unsubscribe { id, sub } => {
                let c = &mut self.conns[ci];
                if let Some(i) = c.subs.iter().position(|s| s.id == sub) {
                    c.subs.remove(i);
                    self.viewer_unsubscribed(conn, &sub);
                    vec![out(conn, ack(id, AckStatus::Unsubscribed))]
                } else {
                    let e = E::new(ErrorCode::NotFound, format!("no subscription `{sub}`"))
                        .at("/sub")
                        .dym(&sub, c.subs.iter().map(|s| s.id.as_str()))
                        .build();
                    vec![err_line(conn, Some(id), e)]
                }
            }
            ClientMsg::Input { id, gesture } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                match self.as_reader(&principal, lang, |s| s.input(ci, &gesture)) {
                    Ok((data, mut events)) => {
                        let mut v = vec![out(
                            conn,
                            ServerMsg::Result {
                                id,
                                seq: self.seq_to(&principal, self.seq()),
                                now: self.clock(),
                                data,
                                resolved,
                            },
                        )];
                        v.append(&mut events);
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Attend { id, directive } => {
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                let r = self.as_reader(&principal, lang, |s| s.attend(ci, directive));
                match r {
                    Ok((did, mut events)) => {
                        let mut v = vec![out(
                            conn,
                            ServerMsg::Ack {
                                resolved,
                                id,
                                status: AckStatus::Delivered,
                                seq: None,
                                created: vec![did],
                                proposal: None,
                                undoes: None,
                            },
                        )];
                        v.append(&mut events);
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Clock { now } => match self.tick(ci, now) {
                Ok(lines) => lines,
                Err(e) => vec![err_line(conn, None, e)],
            },
            ClientMsg::Dispute {
                id,
                claim,
                t,
                values,
                anchor_t,
                digest,
            } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                let d = crate::dispute::DisputeIn {
                    claim,
                    t,
                    values,
                    anchor_t,
                    digest,
                };
                match self.dispute(ci, &id, d) {
                    Ok(lines) => lines,
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Visible { id, sub, visible } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                self.visible_msg(ci, id, &sub, visible)
            }
            // (R4, §21) Answered by a replay host (`crate::replayhost`) before it gets
            // here; a bare authority has no replay source.
            ClientMsg::Replay { id, .. } => vec![err_line(
                conn,
                Some(id),
                E::new(ErrorCode::Unsupported, "this authority has no replay source")
                    .hint("Replay controls work on a session started with `init.replay` (docs/PROTOCOL.md §21).")
                    .build(),
            )],
        }
    }

    pub(crate) fn seq(&self) -> Seq {
        self.log.len() as Seq
    }

    fn hello(
        &mut self,
        conn: &str,
        ci: Option<usize>,
        protocol: &str,
        principal: &str,
        surface: Surface,
        features: &[String],
    ) -> Vec<OutboundLine> {
        let fail = |e: E| vec![err_line(conn, None, e.build())];
        if ci.is_some() {
            return fail(E::new(
                ErrorCode::Handshake,
                format!("connection `{conn}` already said hello"),
            ));
        }
        if protocol != PROTOCOL_VERSION {
            return fail(
                E::new(
                    ErrorCode::Handshake,
                    format!("unsupported protocol `{protocol}`"),
                )
                .at("/protocol")
                .expected([PROTOCOL_VERSION]),
            );
        }
        let Some(p) = self.principal(principal).cloned() else {
            return fail(
                E::new(
                    ErrorCode::Handshake,
                    format!("unknown principal `{principal}`"),
                )
                .at("/principal")
                .dym(principal, self.principals.iter().map(|p| p.id.as_str())),
            );
        };
        let profile = profile_for(surface.kind, surface.max_depth);
        let mut granted: Vec<String> = Vec::new();
        for f in features {
            if FEATURES.contains(&f.as_str()) && !granted.contains(f) {
                granted.push(f.clone());
            }
        }
        self.conns.push(Conn {
            id: conn.to_owned(),
            principal: p.id.clone(),
            role: p.role,
            surface,
            profile: profile.clone(),
            subs: Vec::new(),
            viewports: BTreeMap::new(),
            current: None,
            last_input: 0,
            last_at: None,
            look: qp_schema::protocol::ViewState::default(),
        });
        vec![out(
            conn,
            ServerMsg::Welcome {
                // Only principals that may read get the summary (a feed may not).
                summary: permissions(p.role).read.then(|| {
                    let lang = self
                        .conns
                        .last()
                        .and_then(|c| c.surface.lang)
                        .unwrap_or_default();
                    let pid = p.id.clone();
                    self.as_reader(&pid, lang, |s| s.summary_for(&pid))
                }),
                protocol: PROTOCOL_VERSION.to_owned(),
                doc: self.doc.id.clone(),
                principal: p.id.clone(),
                role: p.role,
                permissions: permissions(p.role),
                seq: self.seq_to(&p.id, self.seq()),
                now: self.clock(),
                epoch: self.epoch,
                profile,
                features: granted,
                limits: self.limits.clone(),
            },
        )]
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    fn command(
        &mut self,
        ci: usize,
        id: &str,
        ops: &[Op],
        mode: WriteMode,
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        if ops.is_empty() {
            return Err(invalid("a command needs at least one op", "/ops"));
        }
        // (R5B, §25) Human decisions and the agent write rule, before roles and modes.
        self.agent_check(ci, ops)?;
        if ops.len() > 1 {
            if let Some(i) = ops
                .iter()
                .position(|o| matches!(o, Op::AcceptProposal { .. } | Op::Undo { .. }))
            {
                return Err(invalid(
                    format!("`{}` must be the only op of its command", op_name(&ops[i])),
                    ptr("/ops", i),
                ));
            }
        }
        let role = self.conns[ci].role;
        let perms = permissions(role);
        let principal = self.conns[ci].principal.clone();
        let forbid = |i: usize, why: String, hint: &str| -> ProtocolError {
            E::new(ErrorCode::Forbidden, why)
                .at(ptr("/ops", i))
                .hint(hint)
                .build()
        };
        let role_name = role_str(role);
        let first_denied = ops.iter().position(|o| !allowed(o, &perms));
        let first_unproposable = ops.iter().position(|o| !proposable(o));
        let direct = match mode {
            WriteMode::Direct => {
                if let Some(i) = first_denied {
                    let hint = if perms.propose && proposable(&ops[i]) {
                        "Use mode auto (or propose) to create a proposal instead."
                    } else {
                        "Ask a principal with the required role."
                    };
                    return Err(forbid(
                        i,
                        format!("role `{role_name}` may not `{}` directly", op_name(&ops[i])),
                        hint,
                    ));
                }
                true
            }
            WriteMode::Auto => match first_denied {
                None => true,
                Some(i) => {
                    if !perms.propose {
                        return Err(forbid(
                            i,
                            format!("role `{role_name}` may not `{}`", op_name(&ops[i])),
                            "Ask a principal with the required role.",
                        ));
                    }
                    if let Some(j) = first_unproposable {
                        return Err(forbid(
                            j,
                            format!(
                                "role `{role_name}` may not `{}`, and it cannot be proposed",
                                op_name(&ops[j])
                            ),
                            "Ask a principal with the required role.",
                        ));
                    }
                    false
                }
            },
            WriteMode::Propose => {
                if !perms.propose {
                    return Err(forbid(
                        0,
                        format!("role `{role_name}` may not propose"),
                        "Ask a principal with the propose permission.",
                    ));
                }
                if let Some(j) = first_unproposable {
                    return Err(forbid(
                        j,
                        format!("`{}` cannot be proposed", op_name(&ops[j])),
                        "Send it with mode direct if you have the permission.",
                    ));
                }
                false
            }
        };
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let seq = self.seq() + 1;
        let conn = self.conns[ci].id.clone();
        // (R3, §20) A principal that may not see some engine items gets the errors the
        // document it sees would give (no existence oracle): a dry run on it first.
        let public = self.public_view(&principal);
        if self.runner.is_some() && (public || self.proj_for(&principal).is_some()) {
            let lang = self.lang_of(ci);
            let author = author.clone();
            let pr = principal.clone();
            self.as_reader(&principal, lang, |s| {
                let mut ctx = Ctx {
                    author,
                    approved_by: None,
                    at: s.clock,
                    seq: if public { s.pub_len() + 1 } else { seq },
                    id_seq: if public { s.pub_len() + 1 } else { seq },
                    n: 0,
                    created: Vec::new(),
                    speaker: &pr,
                    is_owner: role == Role::Owner,
                    can_decide: perms.decide,
                    log: &s.log,
                    undoes: None,
                    data: s.reader(),
                    plane: &s.plane,
                };
                if ops
                    .iter()
                    .any(|o| matches!(o, Op::Undo { .. } | Op::AcceptProposal { .. }))
                {
                    return Ok(());
                }
                expand_ops(&s.doc, ops, "/ops", &mut ctx).map(|_| ())
            })?;
        }

        if direct {
            if self.incremental
                && ops
                    .iter()
                    .all(|o| matches!(o, Op::AppendBars { .. } | Op::UpdateLastBar { .. }))
            {
                return self.bar_command(ci, id, ops, (base, reason), resolved);
            }
            if let [Op::AcceptProposal { proposal, note }] = ops {
                return self.accept(
                    &conn,
                    id,
                    (proposal, note.clone()),
                    &principal,
                    (base, reason),
                    resolved,
                );
            }
            // (R3/K1b) With a public world: each world takes the command on its own.
            if self.other.is_some() {
                let w = Writer {
                    conn: &conn,
                    id,
                    principal: &principal,
                    author,
                    role,
                    perms: &perms,
                    public,
                };
                return self.direct_two_worlds(&w, ops, (base, reason), resolved);
            }
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: self.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: &principal,
                is_owner: role == Role::Owner,
                can_decide: perms.decide,
                log: &self.log,
                undoes: None,
                data: self.reader(),
                plane: &self.plane,
            };
            let ex = expand_ops(&self.doc, ops, "/ops", &mut ctx)?;
            let created = ctx.created;
            let undoes = ctx.undoes;
            self.base_check(base, &ex.changes, &ex.origin)?;
            let entry = LogEntry {
                reason: reason.clone(),
                seq,
                at: self.clock,
                author: principal.clone(),
                approved_by: None,
                cause: Some(id.to_owned()),
                request: ops.to_vec(),
                changes: ex.changes,
                inverse: ex.inverse,
                undoes,
                summary: match undoes {
                    Some(u) => format!("undo {u}"),
                    None => summarize_ops(ops, &created),
                },
                evidence: None,
            };
            self.commit(entry, ex.doc);
            let mut lines = vec![out(
                &conn,
                ServerMsg::Ack {
                    resolved,
                    id: id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes,
                },
            )];
            lines.extend(self.settle(seq));
            return Ok(lines);
        }

        // Proposal: validate by a dry run (ids assigned now, reused on acceptance).
        if self.other.is_some() {
            let w = Writer {
                conn: &conn,
                id,
                principal: &principal,
                author,
                role,
                perms: &perms,
                public,
            };
            return self.propose_two_worlds(&w, ops, (base, reason), resolved);
        }
        let mut ctx = Ctx {
            author: author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: &principal,
            is_owner: role == Role::Owner,
            can_decide: perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let ex = expand_ops(&self.doc, ops, "/ops", &mut ctx)?;
        let created = ctx.created;
        self.base_check(base, &ex.changes, &ex.origin)?;
        let pid = format!("prop-{seq}");
        let changes = vec![ChangeOp::PutProposal {
            proposal: self.new_proposal(&pid, ops, reason.clone(), author),
            index: None,
        }];
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        let inverse = crate::live::apply_in_place(&mut self.doc, &changes, restore)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops"))?;
        let entry = LogEntry {
            reason: reason.clone(),
            seq,
            at: self.clock,
            author: principal,
            approved_by: None,
            cause: Some(id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: None,
            summary: format!("propose {pid}: {}", summarize_ops(ops, &created)),
            evidence: None,
        };
        self.committed(entry);
        let mut lines = vec![out(
            &conn,
            ServerMsg::Ack {
                resolved,
                id: id.to_owned(),
                status: AckStatus::Proposed,
                seq: Some(seq),
                created,
                proposal: Some(pid),
                undoes: None,
            },
        )];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// A pending proposal `pid` of `ops` by `author`, made now.
    fn new_proposal(
        &self,
        pid: &str,
        ops: &[Op],
        reason: Option<String>,
        author: qp_schema::Author,
    ) -> Proposal {
        Proposal {
            id: pid.to_owned(),
            ops: ops.to_vec(),
            reason,
            state: ProposalState::Pending,
            provenance: qp_schema::Provenance {
                author,
                at: self.clock,
                based_on: Vec::new(),
                approved_by: None,
            },
            expires_at: self.ttl.map(|t| {
                self.clock
                    .saturating_add(i64::try_from(t).unwrap_or(i64::MAX))
            }),
            decided_by: None,
            note: None,
            decided_seq: None,
        }
    }

    /// (R3/K1b) `ops` of writer `w` expanded in the public world as its next entry (public
    /// sequence number and ids); `base` checked there when given.
    fn pub_expand(&mut self, w: &Writer<'_>, ops: &[Op], base: Option<Seq>) -> PResult<PubEx> {
        let author = w.author.clone();
        let (role, perms) = (w.role, w.perms.clone());
        self.in_world(true, |s| {
            let seq = s.seq() + 1;
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: s.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: w.principal,
                is_owner: role == Role::Owner,
                can_decide: perms.decide,
                log: &s.log,
                undoes: None,
                data: s.reader(),
                plane: &s.plane,
            };
            let ex = expand_ops(&s.doc, ops, "/ops", &mut ctx)?;
            s.base_check(base, &ex.changes, &ex.origin)?;
            Ok(PubEx {
                seq,
                changes: ex.changes,
                inverse: ex.inverse,
                created: ctx.created,
                undoes: ctx.undoes,
            })
        })
    }

    /// (R3/K1b) A direct command with a public world. Each world takes it on its own:
    /// the public world its own expansion (an entitled writer's `undo` mapped to the
    /// public entry it names), the authority's world its own (an entitled writer) or the
    /// public changes translated back, all or nothing (a public-view writer). The writer
    /// gets the answer of the world it reads; the other world's outcome leaves no trace
    /// in it.
    fn direct_two_worlds(
        &mut self,
        w: &Writer<'_>,
        ops: &[Op],
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let summary = |ops: &[Op], undoes: Option<Seq>, created: &[String]| match undoes {
            Some(u) => format!("undo {u}"),
            None => summarize_ops(ops, created),
        };
        let pub_entry = |s: &Self, ops: &[Op], p: &PubEx| LogEntry {
            reason: reason.clone(),
            seq: p.seq,
            at: s.clock,
            author: w.principal.to_owned(),
            approved_by: None,
            cause: Some(w.id.to_owned()),
            request: ops.to_vec(),
            changes: p.changes.clone(),
            inverse: Vec::new(),
            undoes: p.undoes,
            summary: summary(ops, p.undoes, &p.created),
            evidence: None,
        };
        let ack = |seq: Seq, created: Vec<String>, undoes: Option<Seq>| {
            out(
                w.conn,
                ServerMsg::Ack {
                    resolved: resolved.clone(),
                    id: w.id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes,
                },
            )
        };
        if w.public {
            let p = self.pub_expand(w, ops, base)?;
            let pe = pub_entry(self, ops, &p);
            let mut lines = vec![ack(p.seq, p.created.clone(), p.undoes)];
            lines.extend(self.commit_public_writer(pe, &p.inverse));
            return Ok(lines);
        }
        // An entitled writer.
        let pub_ops = self.ops_to_pub(ops);
        let pub_side = match pub_ops.as_deref() {
            Some(o) => self.pub_expand(w, o, None).ok().map(|p| (o.to_vec(), p)),
            None => None,
        };
        let seq = self.seq() + 1;
        let mut ctx = Ctx {
            author: w.author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: w.principal,
            is_owner: w.role == Role::Owner,
            can_decide: w.perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let full = expand_ops(&self.doc, ops, "/ops", &mut ctx)
            .map(|ex| (ex, ctx.created, ctx.undoes))
            .and_then(|(ex, c, u)| {
                self.base_check(base, &ex.changes, &ex.origin)?;
                Ok((ex, c, u))
            });
        match full {
            Ok((ex, created, undoes)) => {
                self.pending_pub = Some(match &pub_side {
                    Some((o, p)) => {
                        let pe = pub_entry(self, o, p);
                        PubNext::Entry(PubPending {
                            changes: pe.changes,
                            summary: Some(pe.summary),
                            evidence: None,
                            undoes: pe.undoes,
                            request: Some(pe.request),
                        })
                    }
                    None => PubNext::Skip,
                });
                let entry = LogEntry {
                    reason: reason.clone(),
                    seq,
                    at: self.clock,
                    author: w.principal.to_owned(),
                    approved_by: None,
                    cause: Some(w.id.to_owned()),
                    request: ops.to_vec(),
                    changes: ex.changes,
                    inverse: ex.inverse,
                    undoes,
                    summary: summary(ops, undoes, &created),
                    evidence: None,
                };
                self.commit(entry, ex.doc);
                let mut lines = vec![ack(seq, created, undoes)];
                lines.extend(self.settle(seq));
                Ok(lines)
            }
            Err(e) => {
                let mut lines = vec![err_line(w.conn, Some(w.id.to_owned()), e)];
                if let Some((o, p)) = pub_side {
                    let pe = pub_entry(self, &o, &p);
                    if let Some(ps) = self.commit_public(pe) {
                        lines.extend(self.entry_events_pub(ps));
                    }
                    lines.extend(self.invalidate_in_public());
                }
                Ok(lines)
            }
        }
    }

    /// (R3/K1b) A proposal with a public world: each world validates and stores it on its
    /// own (its own `prop-<seq>`); a public-view proposer's stays in the public world.
    fn propose_two_worlds(
        &mut self,
        w: &Writer<'_>,
        ops: &[Op],
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let pub_entry = |s: &Self, ops: &[Op], p: &PubEx| {
            let pid = format!("prop-{}", p.seq);
            LogEntry {
                reason: reason.clone(),
                seq: p.seq,
                at: s.clock,
                author: w.principal.to_owned(),
                approved_by: None,
                cause: Some(w.id.to_owned()),
                request: ops.to_vec(),
                changes: vec![ChangeOp::PutProposal {
                    proposal: s.new_proposal(&pid, ops, reason.clone(), w.author.clone()),
                    index: None,
                }],
                inverse: Vec::new(),
                undoes: None,
                summary: format!("propose {pid}: {}", summarize_ops(ops, &p.created)),
                evidence: None,
            }
        };
        let ack = |seq: Seq, created: Vec<String>| {
            out(
                w.conn,
                ServerMsg::Ack {
                    resolved: resolved.clone(),
                    id: w.id.to_owned(),
                    status: AckStatus::Proposed,
                    seq: Some(seq),
                    created,
                    proposal: Some(format!("prop-{seq}")),
                    undoes: None,
                },
            )
        };
        if w.public {
            let p = self.pub_expand(w, ops, base)?;
            let pe = pub_entry(self, ops, &p);
            let pid = format!("prop-{}", p.seq);
            let mut lines = vec![ack(p.seq, p.created.clone())];
            // (R3/K1c) It stays in the public world: `reverse_changes` never takes a
            // proposal.
            lines.extend(self.commit_public_writer(pe, &[]));
            let _ = pid;
            return Ok(lines);
        }
        let pub_ops = self.ops_to_pub(ops);
        let pub_side = match pub_ops.as_deref() {
            Some(o) => self.pub_expand(w, o, None).ok().map(|p| (o.to_vec(), p)),
            None => None,
        };
        let seq = self.seq() + 1;
        let mut ctx = Ctx {
            author: w.author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: w.principal,
            is_owner: w.role == Role::Owner,
            can_decide: w.perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let full = expand_ops(&self.doc, ops, "/ops", &mut ctx)
            .map(|ex| (ex, ctx.created))
            .and_then(|(ex, c)| {
                self.base_check(base, &ex.changes, &ex.origin)?;
                Ok(c)
            });
        let created = match full {
            Ok(c) => c,
            Err(e) => {
                let mut lines = vec![err_line(w.conn, Some(w.id.to_owned()), e)];
                if let Some((o, p)) = pub_side {
                    let pe = pub_entry(self, &o, &p);
                    if let Some(ps) = self.commit_public(pe) {
                        lines.extend(self.entry_events_pub(ps));
                    }
                }
                return Ok(lines);
            }
        };
        let pid = format!("prop-{seq}");
        let changes = vec![ChangeOp::PutProposal {
            proposal: self.new_proposal(&pid, ops, reason.clone(), w.author.clone()),
            index: None,
        }];
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        let inverse = crate::live::apply_in_place(&mut self.doc, &changes, restore)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops"))?;
        self.pending_pub = Some(match &pub_side {
            Some((o, p)) => {
                let pe = pub_entry(self, o, p);
                PubNext::Entry(PubPending {
                    changes: pe.changes,
                    summary: Some(pe.summary),
                    evidence: None,
                    undoes: None,
                    request: Some(pe.request),
                })
            }
            None => PubNext::Skip,
        });
        let entry = LogEntry {
            reason: reason.clone(),
            seq,
            at: self.clock,
            author: w.principal.to_owned(),
            approved_by: None,
            cause: Some(w.id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: None,
            summary: format!("propose {pid}: {}", summarize_ops(ops, &created)),
            evidence: None,
        };
        self.committed(entry);
        // (R3/K1c) Both worlds hold this proposal only when this command's own entries
        // (the authority's just committed, and the public one it made) put it.
        let put = |e: Option<&LogEntry>, id: &str| {
            e.is_some_and(|e| {
                e.changes.iter().any(
                    |c| matches!(c, ChangeOp::PutProposal { proposal, .. } if proposal.id == id),
                )
            })
        };
        if put(self.log.last().filter(|e| e.seq == seq), &pid) {
            if let Some(p) = self.pub_of(seq) {
                let ppid = format!("prop-{p}");
                let ok = self
                    .other
                    .as_ref()
                    .is_some_and(|w| put(w.log.last().filter(|e| e.seq == p), &ppid));
                if ok {
                    self.prop_map.insert(pid, ppid);
                }
            }
        }
        let mut lines = vec![ack(seq, created)];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// (R3/K1b) Commits public-view writer's entry `pe` (of the public world, not yet
    /// applied there; `inverse` its changes' inverse): the authority's world takes the
    /// changes translated back, all or nothing ([`Session::reverse_changes`]), as its own
    /// entry, which the public world then takes as `pe`; else only the public world
    /// takes `pe`. Returns the notifications.
    fn commit_public_writer(&mut self, pe: LogEntry, inverse: &[ChangeOp]) -> Vec<OutboundLine> {
        let rev = self
            .reverse_changes(&pe.changes, inverse)
            .filter(|r| !r.is_empty());
        let full_seq = self.seq() + 1;
        match rev.and_then(|r| self.apply_atomic(r)) {
            Some((changes, inverse)) => {
                self.pending_pub = Some(PubNext::Entry(PubPending {
                    changes: pe.changes.clone(),
                    summary: Some(pe.summary.clone()),
                    evidence: None,
                    undoes: pe.undoes,
                    request: Some(pe.request.clone()),
                }));
                self.committed(LogEntry {
                    seq: full_seq,
                    undoes: None,
                    changes,
                    inverse,
                    ..pe
                });
                self.settle(full_seq)
            }
            None => {
                let mut lines = Vec::new();
                if let Some(ps) = self.commit_public(pe) {
                    lines.extend(self.entry_events_pub(ps));
                }
                lines.extend(self.invalidate_in_public());
                lines
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn accept(
        &mut self,
        conn: &str,
        id: &str,
        (pid, note): (&str, Option<String>),
        decider: &str,
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let ack = |seq: Seq, created: Vec<String>| {
            out(
                conn,
                ServerMsg::Ack {
                    resolved: resolved.clone(),
                    id: id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes: None,
                },
            )
        };
        let entry_of = |a: &Accepted, seq: Seq, at: i64, pid: &str| LogEntry {
            reason: reason.clone().or_else(|| a.reason.clone()),
            seq,
            at,
            author: a.proposer.clone(),
            approved_by: Some(decider.to_owned()),
            cause: Some(id.to_owned()),
            request: a.ops.clone(),
            changes: a.changes.clone(),
            inverse: a.inverse.clone(),
            undoes: None,
            summary: format!("accept {pid}: {}", summarize_ops(&a.ops, &a.created)),
            evidence: None,
        };
        // (R3/K1b) With a public world each world accepts on its own: a public-view
        // decider's acceptance (of a public proposal) only in the public world, with its
        // answer; an entitled decider's in the authority's world (its answer) and, for
        // the public proposal the same entry made, in the public world.
        if self.other.is_some() && self.public_view(decider) {
            let n = note.clone();
            let a = self.in_world(true, |s| {
                let seq = s.seq() + 1;
                s.accept_expand(pid, n, decider, base, seq)
                    .map(|a| (a, seq))
            })?;
            let (a, ps) = a;
            let pe = entry_of(&a, ps, self.clock, pid);
            let mut lines = vec![ack(ps, a.created.clone())];
            lines.extend(self.commit_public_writer(pe, &a.inverse));
            return Ok(lines);
        }
        let seq = self.seq() + 1;
        let pub_side = if self.other.is_some() {
            self.proposal_to_pub(pid).and_then(|pp| {
                let n = note.clone();
                self.in_world(true, |s| {
                    let seq = s.seq() + 1;
                    s.accept_expand(&pp, n, decider, None, seq)
                        .ok()
                        .map(|a| (a, seq, pp.clone()))
                })
            })
        } else {
            None
        };
        let full = self.accept_expand(pid, note, decider, base, seq);
        let a = match full {
            Ok(a) => a,
            Err(e) => {
                let Some((p, ps, pp)) = pub_side else {
                    return Err(e);
                };
                let mut lines = vec![err_line(conn, Some(id.to_owned()), e)];
                let pe = entry_of(&p, ps, self.clock, &pp);
                if let Some(ps) = self.commit_public(pe) {
                    lines.extend(self.entry_events_pub(ps));
                }
                return Ok(lines);
            }
        };
        if self.other.is_some() {
            self.pending_pub = Some(match &pub_side {
                Some((p, ps, pp)) => {
                    let pe = entry_of(p, *ps, self.clock, pp);
                    PubNext::Entry(PubPending {
                        changes: pe.changes,
                        summary: Some(pe.summary),
                        evidence: None,
                        undoes: None,
                        request: Some(pe.request),
                    })
                }
                None => PubNext::Skip,
            });
        }
        let entry = entry_of(&a, seq, self.clock, pid);
        let created = a.created.clone();
        self.commit(entry, a.doc);
        let mut lines = vec![ack(seq, created)];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// The acceptance of proposal `pid` by `decider` as entry `seq`, expanded and
    /// `base`-checked in the world the fields hold.
    fn accept_expand(
        &self,
        pid: &str,
        note: Option<String>,
        decider: &str,
        base: Option<Seq>,
        seq: Seq,
    ) -> PResult<Accepted> {
        let pr = need_proposal(&self.doc, pid, "/ops/0/proposal")?.clone();
        let proposer = self.principal_of(&pr.provenance.author);
        let proposer_owner = self
            .principal(&proposer)
            .is_some_and(|p| p.role == Role::Owner);
        let decider_author = self
            .principal(decider)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let id_seq = pid
            .strip_prefix("prop-")
            .and_then(|s| s.parse::<Seq>().ok())
            .unwrap_or(seq);
        let mut ctx = Ctx {
            author: pr.provenance.author.clone(),
            approved_by: Some(decider_author.clone()),
            at: self.clock,
            seq,
            id_seq,
            n: 0,
            created: Vec::new(),
            speaker: &proposer,
            is_owner: proposer_owner,
            can_decide: false,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let ex = expand_ops(&self.doc, &pr.ops, "/ops", &mut ctx).map_err(|mut e| {
            e.message = format!(
                "proposal `{pid}` cannot be applied now: {} (at {} of the proposal)",
                e.message,
                e.path.as_deref().unwrap_or("/ops")
            );
            e.path = Some("/ops/0".to_owned());
            e.hint =
                Some("The proposal stays pending; reject it or fix the document first.".to_owned());
            e
        })?;
        let created = ctx.created;
        let mut decided = pr.clone();
        decided.state = ProposalState::Accepted;
        decided.decided_by = Some(decider_author);
        decided.note = note;
        decided.decided_seq = Some(seq);
        let mut doc = ex.doc;
        let mut changes = ex.changes;
        let tail = vec![ChangeOp::PutProposal {
            proposal: decided,
            index: None,
        }];
        let mut inverse = apply_all(&mut doc, &tail)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops/0"))?;
        inverse.extend(ex.inverse);
        changes.extend(tail);
        let origin = vec![0; changes.len()];
        self.base_check(base, &changes, &origin)?;
        Ok(Accepted {
            doc,
            changes,
            inverse,
            created,
            proposer,
            reason: pr.reason.clone(),
            ops: pr.ops,
        })
    }

    /// `base` check (§8.2): an item the command touches changed in an entry after
    /// `base` that is not undone (undo pairs both after `base` cancel out).
    pub(crate) fn base_check(
        &self,
        base: Option<Seq>,
        changes: &[ChangeOp],
        origin: &[usize],
    ) -> PResult<()> {
        let Some(b) = base else { return Ok(()) };
        if b > self.seq() {
            // A base from the future cannot be checked; accepting it would silently
            // switch conflict detection off (audit O2), e.g. for a stale epoch (§8.4).
            return Err(E::new(
                ErrorCode::Invalid,
                format!("base {b} is after the current log position {}", self.seq()),
            )
            .at("/base")
            .expected([format!("0..={}", self.seq())])
            .hint("Use the seq from `welcome` or from a `result`; after a hand-over, reconnect.")
            .build());
        }
        let undone = undone_set(&self.log);
        let mut conflicts = Vec::new();
        let mut first: Option<(String, Seq, String, usize)> = None;
        for e in self.log.iter().filter(|e| e.seq > b) {
            if undone.contains(&e.seq) || e.undoes.is_some_and(|x| x > b) {
                continue;
            }
            let t = touched_keys(&e.changes);
            for (k, c) in changes.iter().enumerate() {
                let key = std::iter::once(touched_key(c)).collect();
                if let Some(id) = first_overlap(&key, &t) {
                    if first.is_none() {
                        first = Some((id, e.seq, e.author.clone(), origin[k]));
                    }
                    conflicts.push(e.seq);
                    break;
                }
            }
        }
        let Some((id, s, by, op)) = first else {
            return Ok(());
        };
        conflicts.sort_unstable();
        conflicts.dedup();
        let latest = conflicts.last().copied().unwrap_or(s);
        Err(E::new(
            ErrorCode::Conflict,
            format!("`{id}` changed after base {b} (seq {s} by {by})"),
        )
        .at(ptr("/ops", op))
        .conflicts(conflicts)
        .hint(format!(
            "Read the item again and resend with base={latest}, or omit base."
        ))
        .build())
    }

    fn commit(&mut self, entry: LogEntry, doc: Document) {
        self.doc = doc;
        self.committed(entry);
    }

    /// Records `entry`, whose changes `self.doc` already holds (applied in place or by
    /// replacing the document): the data plane, the reading document, the values cache
    /// and the view index follow, then the entry joins the log.
    pub(crate) fn committed(&mut self, entry: LogEntry) {
        let series_set = entry
            .changes
            .iter()
            .any(|c| matches!(c, ChangeOp::PutSeries { .. } | ChangeOp::Delete { .. }));
        let bars = crate::live::bar_deltas(&self.doc, &entry.changes);
        if !self.incremental && (series_set || !bars.is_empty()) {
            self.cache.clear();
        }
        if series_set {
            // The data plane follows the document (§18.3): bindings put, changed or
            // deleted are resolved or dropped; the reading document mirrors the document.
            self.pending_stamps = self.plane.sync(&self.doc);
            self.rd = reading_doc(&self.doc, &self.plane);
            if self.incremental {
                let mut ids: Vec<&str> = entry
                    .changes
                    .iter()
                    .filter_map(|c| match c {
                        ChangeOp::PutSeries { series, .. } => Some(series.id.as_str()),
                        ChangeOp::Delete { id } => Some(id.as_str()),
                        _ => None,
                    })
                    .collect();
                ids.extend(bars.iter().map(|(id, _)| id.as_str()));
                self.cache.forget(rd!(self), &ids);
            }
            self.view_reads = None;
        } else {
            if let Some(rd) = self.rd.as_mut() {
                if apply_all(rd, &entry.changes).is_err() {
                    self.rd = reading_doc(&self.doc, &self.plane);
                    self.head_state.dirty_all = true;
                }
            }
            if !entry.changes.iter().all(crate::live::keeps_view_reads) {
                self.view_reads = None;
            }
            if self.incremental && !bars.is_empty() {
                self.cache.begin_tick();
                for (id, d) in &bars {
                    self.cache.apply_delta(rd!(self), id, *d);
                }
            }
        }
        if !bars.is_empty() {
            self.claims_after_bars(&bars);
        }
        for (id, d) in bars {
            self.pending_views.push((id, d, entry.seq));
        }
        if !entry.changes.iter().all(|c| {
            matches!(
                c,
                ChangeOp::AppendBars { .. }
                    | ChangeOp::SetLastBar { .. }
                    | ChangeOp::TruncateBars { .. }
            )
        }) {
            self.struct_gen += 1;
        }
        self.note_writers(&entry);
        for c in entry.changes.iter().chain(&entry.inverse) {
            if let ChangeOp::PutAudience { principal, .. } = c {
                self.audience_seen.insert(principal.clone());
            }
        }
        // (R3/K1) The public world takes its own version of the entry (same seq).
        debug_assert!(
            !self.in_public,
            "entries are committed in the authority's world"
        );
        let public = self.other.is_some().then(|| self.public_entry(&entry));
        self.log.push(entry);
        if let Some(pe) = public {
            let p = pe.and_then(|pe| self.commit_public(pe));
            self.full_to_pub.push(p);
            self.note_pub_len();
        }
    }

    /// Notes who wrote the series and finding invalidations `entry` puts (§18.3: the
    /// principals responsible for a condition). Engine entries write no meaning of their
    /// own (an anchor stamp keeps the binding's writer).
    pub(crate) fn note_writers(&mut self, entry: &LogEntry) {
        if entry.author == ENGINE_PRINCIPAL {
            return;
        }
        for c in &entry.changes {
            match c {
                ChangeOp::PutSeries { series, .. } => {
                    self.series_writer
                        .insert(series.id.clone(), entry.author.clone());
                }
                ChangeOp::PutFinding { finding, .. } => {
                    let before = entry.inverse.iter().find_map(|i| match i {
                        ChangeOp::PutFinding { finding: old, .. } if old.id == finding.id => {
                            Some(&old.invalidation)
                        }
                        _ => None,
                    });
                    if before != Some(&finding.invalidation) {
                        self.invalidation_writer
                            .insert(finding.id.clone(), entry.author.clone());
                    }
                }
                ChangeOp::Delete { id } => {
                    self.series_writer.remove(id);
                    self.invalidation_writer.remove(id);
                }
                _ => {}
            }
        }
    }

    /// The principals of this session responsible for finding `f`'s invalidation, who must
    /// all be entitled to every bound feed it reads for it to be evaluated (§18.3): the
    /// finding's author, whoever last wrote its invalidation, and whoever last wrote each
    /// series it rests on (`closure`, through every study input). Authors that are not
    /// principals of this session (another document, the engine) are left out: the
    /// authority's own view.
    pub(crate) fn responsible(&self, f: &qp_schema::Finding, closure: &[String]) -> Vec<String> {
        let mut who: Vec<String> = Vec::new();
        let mut add = |p: &str| {
            if self.principal(p).is_some() && !who.iter().any(|w| w == p) {
                who.push(p.to_owned());
            }
        };
        add(&self.principal_of(&f.provenance.author));
        if let Some(p) = self.invalidation_writer.get(&f.id) {
            add(p);
        }
        for id in closure {
            if let Some(p) = self.series_writer.get(id) {
                add(p);
            }
        }
        who
    }

    /// Writes the pending anchor stamps (§18.3) as one engine entry: `put_series` of each
    /// bound series with `anchor_t` set to the first bar of its first ready resolution.
    pub(crate) fn stamp_anchors(&mut self) -> Option<Seq> {
        let stamps = std::mem::take(&mut self.pending_stamps);
        let mut changes = Vec::new();
        let mut names = Vec::new();
        for (id, t) in stamps {
            let Some(mut s) = self.doc.series.iter().find(|s| s.id == id).cloned() else {
                continue;
            };
            if let qp_schema::SeriesSource::Bind { anchor_t, .. } = &mut s.source {
                if anchor_t.is_some() {
                    continue;
                }
                *anchor_t = Some(t);
            } else {
                continue;
            }
            names.push(format!("{id} at t={t}"));
            changes.push(ChangeOp::PutSeries {
                series: s,
                index: None,
            });
        }
        if changes.is_empty() {
            return None;
        }
        self.engine_entry(changes, format!("anchor {}", names.join(", ")))
    }

    /// The host's data path (§18.4): applies a data event to the bound series of its
    /// feed (duplicate, gap, stale, outside-window and rule checks by
    /// `qp_source::FeedStore`), then, if bars or a binding state changed, evaluates live
    /// capsules' invalidations, condition subscriptions and intents as a bar write does
    /// (engine entries and notifications in the order of §15). The event is not a
    /// protocol message: it never enters the log and causes no `change` by itself.
    ///
    /// Incremental (§18.8): the reading document takes the changed bar in place, the
    /// study series on the ticked bound series advance their streaming state, and only
    /// the watchers, live capsules and `view` subscriptions resting on it are evaluated.
    ///
    /// # Errors
    /// [`DataError`]: unknown feed or an event the rules reject (nothing applied to that
    /// store; the host re-resolves after the gap that follows, §18.4).
    pub fn data_event(
        &mut self,
        ev: &qp_schema::data::DataEvent,
    ) -> Result<Vec<OutboundLine>, DataError> {
        self.host_data_event(ev).map(|o| o.lines)
    }

    /// [`Session::data_event`] with the notices for the host (§18.4): a `seq` gap or a
    /// binding that became `stale` is not an error (the event was taken: nothing to
    /// apply), but the host must re-resolve ([`Session::refresh_bindings`]) or fill the
    /// gap. A host should use this entry point; `qp serve --host-frames` writes each
    /// notice as a host frame.
    ///
    /// # Errors
    /// As [`Session::data_event`].
    pub fn host_data_event(
        &mut self,
        ev: &qp_schema::data::DataEvent,
    ) -> Result<DataOutcome, DataError> {
        let outcome = self.plane.apply_event(ev)?;
        let notices = outcome.notices;
        if outcome.changed.is_empty() {
            return Ok(DataOutcome {
                lines: Vec::new(),
                notices,
            });
        }
        self.data_changed(outcome.changed);
        let mut lines = self.settle_data();
        lines.extend(self.serve_test_source());
        // (RP3/M15, §22) Viewers, unless the host batches (a replay host: once per step).
        if !self.viewer_batch && self.has_pushers() {
            lines.extend(self.viewer_flush());
        }
        Ok(DataOutcome { lines, notices })
    }

    /// Resolves again every bound series that is not ready (after a gap, a limit, a
    /// provider that answered late) and returns the notifications this causes (an anchor
    /// stamp entry, invalidations, conditions).
    pub fn refresh_bindings(&mut self) -> Vec<OutboundLine> {
        let (stamps, changed) = self.plane.refresh();
        if changed.is_empty() {
            return Vec::new();
        }
        self.pending_stamps = stamps;
        let changed = changed
            .into_iter()
            .map(|id| (id, crate::values::Delta::Rebuild { from: 0 }))
            .collect();
        self.data_changed(changed);
        match self.stamp_anchors() {
            Some(seq) => self.settle(seq),
            None => self.settle_data(),
        }
    }

    /// The host's binding state of a bound series (`None`: not a bound series).
    #[must_use]
    pub fn binding_state(&self, series: &str) -> Option<qp_schema::data::BindingState> {
        self.plane.host_state(series).map(|(s, _)| s)
    }

    /// After the data plane changed bound series (`changed`: series, how): the reading
    /// document takes the change in place (a new bar is pushed, a revised bar replaced;
    /// a state change re-injects or removes the bars), the values cache follows
    /// incrementally (or is cleared on the reference path), and the change waits for
    /// `view` subscribers.
    pub(crate) fn data_changed(&mut self, changed: Vec<(String, crate::values::Delta)>) {
        // (RP3/M15) The headlines' refresh compares the bars of these series only.
        self.head_state
            .dirty
            .extend(changed.iter().map(|(id, _)| id.clone()));
        if let Some(w) = self.other.as_mut() {
            w.head_state
                .dirty
                .extend(changed.iter().map(|(id, _)| id.clone()));
        }
        match self.rd.as_mut() {
            Some(rd) => {
                for (id, d) in &changed {
                    crate::live::mirror(rd, &self.plane, id, *d);
                }
            }
            None => self.rd = reading_doc(&self.doc, &self.plane),
        }
        // (R3/K1) The public world reads the same bars.
        debug_assert!(!self.in_public);
        if let Some(w) = self.other.as_mut() {
            match w.rd.as_mut() {
                Some(rd) => {
                    for (id, d) in &changed {
                        crate::live::mirror(rd, &self.plane, id, *d);
                    }
                }
                None => w.rd = reading_doc(&w.doc, &self.plane),
            }
        }
        if self.incremental {
            self.cache.begin_tick();
            for (id, d) in &changed {
                self.cache.apply_delta(rd!(self), id, *d);
            }
        } else {
            self.cache.clear();
        }
        self.claims_after_bars(&changed);
        self.note_closed(&changed);
        let seq = self.seq();
        for (id, d) in changed {
            self.pending_views.push((id, d, seq));
        }
    }

    /// After a data change without a log entry: watchers at the current position, then
    /// invalidations of every live capsule (§8.1, §18.4); incremental: only those resting
    /// on the changed series (§18.8).
    pub(crate) fn settle_data(&mut self) -> Vec<OutboundLine> {
        if self.incremental {
            return self.settle_live(None);
        }
        let seq = self.seq();
        let (mut conds, mut intents) = self.eval_watchers(seq);
        let mut entries = Vec::new();
        for s in self.invalidate(true, &BTreeSet::new()) {
            entries.push(s);
            let (c, i) = self.eval_watchers(s);
            conds.extend(c);
            intents.extend(i);
        }
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        lines.extend(self.invalidate_in_public());
        lines.extend(self.run_capsules());
        lines.extend(self.view_events());
        lines.extend(conds);
        lines.extend(intents);
        lines.extend(self.claim_events());
        lines
    }

    /// Reader of the data plane for command expansion.
    pub(crate) fn reader(&self) -> Option<Reader<'_>> {
        self.rd.as_ref().map(|rd| Reader {
            rd,
            plane: &self.plane,
        })
    }

    /// Writes an engine entry (author `qp`).
    fn engine_entry(&mut self, changes: Vec<ChangeOp>, summary: String) -> Option<Seq> {
        self.engine_entry_with(changes, summary, None)
    }

    /// Writes an engine entry (author `qp`) carrying a claim's evidence (§19).
    pub(crate) fn engine_entry_with(
        &mut self,
        changes: Vec<ChangeOp>,
        summary: String,
        evidence: Option<qp_schema::protocol::ClaimEvidence>,
    ) -> Option<Seq> {
        // In place, no document copy: a change that does not fit rolls the earlier ones
        // back.
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        let inverse = crate::live::apply_in_place(&mut self.doc, &changes, restore).ok()?;
        let seq = self.seq() + 1;
        self.committed(LogEntry {
            reason: None,
            seq,
            at: self.clock,
            author: ENGINE_PRINCIPAL.to_owned(),
            approved_by: None,
            cause: None,
            request: Vec::new(),
            changes,
            inverse,
            undoes: None,
            summary,
            evidence,
        });
        Some(seq)
    }

    /// After entry `seq`: engine follow-up entries and all notifications, in the order of
    /// §15 (per entry: change, finding_state, proposal; then condition_met, then
    /// intent_triggered).
    pub(crate) fn settle(&mut self, seq: Seq) -> Vec<OutboundLine> {
        if self.incremental && self.pending_stamps.is_empty() && self.bars_only(seq) {
            return self.settle_live(Some(seq));
        }
        let mut entries = vec![seq];
        let (mut conds, mut intents) = self.eval_watchers(seq);
        let mut data_entries = vec![seq];
        if let Some(s) = self.stamp_anchors() {
            entries.push(s);
            data_entries.push(s);
            let (c, i) = self.eval_watchers(s);
            conds.extend(c);
            intents.extend(i);
        }
        for d in data_entries {
            for s in self.invalidations(d) {
                entries.push(s);
                let (c, i) = self.eval_watchers(s);
                conds.extend(c);
                intents.extend(i);
            }
        }
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        lines.extend(self.invalidate_in_public());
        lines.extend(self.run_capsules());
        lines.extend(self.view_events());
        lines.extend(conds);
        lines.extend(intents);
        lines.extend(self.claim_events());
        lines
    }

    /// Engine invalidations triggered by entry `seq` (§8.1): after a change of series
    /// data or study parameters, every live capsule; after a finding change, that
    /// finding's live capsules.
    fn invalidations(&mut self, seq: Seq) -> Vec<Seq> {
        let entry = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        let all = entry.changes.iter().any(|c| {
            matches!(
                c,
                ChangeOp::PutSeries { .. }
                    | ChangeOp::AppendBars { .. }
                    | ChangeOp::TruncateBars { .. }
                    | ChangeOp::SetLastBar { .. }
            )
        });
        let findings: BTreeSet<String> = entry
            .changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::PutFinding { finding, .. } => Some(finding.id.clone()),
                _ => None,
            })
            .collect();
        // Capsules the entry makes live (an undo of an invalidation, a capsule put back):
        // evaluated now, so that a capsule never stays live with a holding invalidation
        // until a series it rests on ticks (RP2 T5 audit O1).
        let revived: BTreeSet<String> = entry
            .changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::SetCapsuleState {
                    capsule,
                    state: CapsuleState::Live,
                } => Some(capsule.clone()),
                ChangeOp::PutCapsule { capsule, .. } if capsule.state == CapsuleState::Live => {
                    Some(capsule.id.clone())
                }
                _ => None,
            })
            .collect();
        if !all && findings.is_empty() && revived.is_empty() {
            return Vec::new();
        }
        let candidates: Vec<(String, String)> = self
            .doc
            .capsules
            .iter()
            .filter(|c| {
                c.state == CapsuleState::Live
                    && (all || findings.contains(&c.finding) || revived.contains(&c.id))
            })
            .map(|c| (c.id.clone(), c.finding.clone()))
            .collect();
        self.invalidate_these(candidates)
    }

    /// Engine entries invalidating live capsules (all, or those of `findings`) whose
    /// invalidation holds now. A condition resting on a bound series that is not ready
    /// is not evaluated (§18.3).
    fn invalidate(&mut self, all: bool, findings: &BTreeSet<String>) -> Vec<Seq> {
        let candidates: Vec<(String, String)> = self
            .doc
            .capsules
            .iter()
            .filter(|c| c.state == CapsuleState::Live && (all || findings.contains(&c.finding)))
            .map(|c| (c.id.clone(), c.finding.clone()))
            .collect();
        self.invalidate_these(candidates)
    }

    /// Engine entries invalidating those of `candidates` (capsule, finding) whose
    /// invalidation holds now (not evaluated when resting on a bound series that is not
    /// ready).
    pub(crate) fn invalidate_these(&mut self, candidates: Vec<(String, String)>) -> Vec<Seq> {
        let mut out = Vec::new();
        for (cap, fnd) in candidates {
            // (R3, §20.1) A capsule of the running engine (a story on a watched series)
            // ends by the outcome rule only, never by a document condition (a person's
            // `update_finding` cannot make one). (R3/K1) So no invalidation entry is ever
            // about an engine item, whose world differs between the authority and the
            // public world. (A story capsule of a document without an engine keeps §8.1.)
            if self.runs_capsule(&cap) {
                continue;
            }
            let Some(f) = self.doc.findings.iter().find(|f| f.id == fnd) else {
                continue;
            };
            let cond = f.invalidation.clone();
            if self.cond_blocked(&cond) {
                continue;
            }
            // A principal of this session without the data entitlement of the feeds its
            // condition reads would learn them from its capsule's state (§18.3): its
            // condition is not evaluated unless every principal responsible for it (the
            // author, whoever last wrote the invalidation, whoever last wrote a series it
            // rests on) is entitled (e.g. its series was rebound after it was written, an
            // undo brought a binding back, an editor rewrote the invalidation).
            let closure = crate::live::closure_of(&self.doc, &cond);
            let who = self.responsible(f, &closure);
            if !who
                .iter()
                .all(|p| crate::live::hears(&self.plane, &self.doc, p, &cond))
            {
                continue;
            }
            if holds(rd!(self), &mut self.cache, &cond) {
                let changes = vec![ChangeOp::SetCapsuleState {
                    capsule: cap.clone(),
                    state: CapsuleState::Invalidated,
                }];
                let ev = crate::claims::evidence(
                    rd!(self),
                    &mut self.cache,
                    &self.claims,
                    &self.plane,
                    &cond,
                );
                if let Some(s) = self.engine_entry_with(
                    changes,
                    format!("invalidate {cap}: invalidation of {fnd} holds"),
                    ev.clone(),
                ) {
                    out.push(s);
                    if let Some(ev) = ev {
                        self.claims.assert(
                            crate::claims::Owner::Doc,
                            qp_schema::protocol::ClaimRef::Capsule {
                                capsule: cap.clone(),
                                finding: fnd.clone(),
                            },
                            cond,
                            ev,
                        );
                    }
                }
            }
        }
        out
    }

    /// Edge-triggered condition subscriptions and intents after entry `seq`.
    fn eval_watchers(&mut self, seq: Seq) -> (Vec<OutboundLine>, Vec<OutboundLine>) {
        let mut conds = Vec::new();
        for ci in 0..self.conns.len() {
            for si in 0..self.conns[ci].subs.len() {
                let Topic::Condition { condition } = &self.conns[ci].subs[si].topic else {
                    continue;
                };
                let condition = condition.clone();
                if !crate::live::hears(
                    &self.plane,
                    &self.doc,
                    &self.conns[ci].principal,
                    &condition,
                ) {
                    // Not evaluated for a subscriber without the data entitlement (§18.3).
                    continue;
                }
                if self.cond_blocked(&condition) {
                    // Not evaluated; the edge state is kept (§18.3).
                    continue;
                }
                let r = eval_condition(rd!(self), &mut self.cache, &condition);
                let now = r.is_some_and(|x| x.0);
                let prev = self.conns[ci].subs[si].last.replace(now);
                if prev == Some(false) && now {
                    if let Some((_, bar, t)) = r {
                        let evidence = self.claim_asserted(
                            crate::claims::Owner::Conn(self.conns[ci].id.clone()),
                            qp_schema::protocol::ClaimRef::Condition {
                                sub: self.conns[ci].subs[si].id.clone(),
                            },
                            &condition,
                        );
                        conds.push(out(
                            &self.conns[ci].id,
                            ServerMsg::Notify {
                                sub: self.conns[ci].subs[si].id.clone(),
                                event: Event::ConditionMet {
                                    bar,
                                    t,
                                    seq: self.seq_to(&self.conns[ci].principal, seq),
                                    evidence: evidence.map(|ev| {
                                        crate::claims::for_principal(
                                            &self.doc,
                                            &self.plane,
                                            &self.conns[ci].principal,
                                            &ev,
                                        )
                                    }),
                                },
                            },
                        ));
                    }
                }
            }
        }
        let fired = self.eval_intents(seq);
        let mut intents = self.intent_lines(fired);
        intents.extend(self.eval_intents_public());
        (conds, intents)
    }

    /// `intent_triggered` events to every `intents` subscriber entitled to the bound feeds
    /// the intent's trigger reads (§18.3), by connection and subscription order.
    pub(crate) fn intent_lines(&self, fired: Vec<Event>) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for ev in fired {
            let trigger = match &ev {
                Event::IntentTriggered { intent, .. } => self
                    .doc
                    .intents
                    .iter()
                    .find(|i| &i.id == intent)
                    .and_then(|i| i.trigger.as_ref()),
                _ => None,
            };
            for c in &self.conns {
                if let Some(t) = trigger {
                    if !crate::live::hears(&self.plane, &self.doc, &c.principal, t) {
                        continue;
                    }
                }
                // (R3/K1c) A public-view reader hears the public world's intents only
                // (`eval_intents_public`).
                if self.public_view(&c.principal) {
                    continue;
                }
                for s in &c.subs {
                    if matches!(s.topic, Topic::Intents {}) {
                        let mut event =
                            crate::claims::event_for(&self.doc, &self.plane, &c.principal, &ev);
                        if let Event::IntentTriggered { seq, .. } = &mut event {
                            *seq = self.seq_to(&c.principal, *seq);
                        }
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event,
                            },
                        ));
                    }
                }
            }
        }
        lines
    }

    fn eval_intents(&mut self, seq: Seq) -> Vec<Event> {
        let mut fired = Vec::new();
        let mut state = BTreeMap::new();
        let intents: Vec<_> = self
            .doc
            .intents
            .iter()
            .filter(|i| i.active)
            .filter_map(|i| i.trigger.clone().map(|t| (i.id.clone(), t)))
            .collect();
        for (id, trig) in intents {
            if matches!(trig, qp_schema::Condition::Text { .. }) {
                continue;
            }
            if self.cond_blocked(&trig) {
                if let Some(prev) = self.intent_state.get(&id) {
                    state.insert(id, *prev);
                }
                continue;
            }
            let r = eval_condition(rd!(self), &mut self.cache, &trig);
            let now = r.is_some_and(|x| x.0);
            if self.intent_state.get(&id) == Some(&false) && now {
                if let Some((_, bar, t)) = r {
                    let evidence = self.claim_asserted(
                        crate::claims::Owner::Doc,
                        qp_schema::protocol::ClaimRef::Intent { intent: id.clone() },
                        &trig,
                    );
                    fired.push(Event::IntentTriggered {
                        intent: id.clone(),
                        bar,
                        t,
                        seq,
                        evidence,
                    });
                }
            }
            state.insert(id, now);
        }
        self.intent_state = state;
        self.intent_epoch += 1;
        fired
    }

    /// Notifications for one log entry: those of the authority's world to the readers
    /// that see it, and (R3/K1) those of the public world's version of the entry to the
    /// public-view readers ([`Session::public_view`]). Each connection's lines come from
    /// one world, in the order of one world.
    pub(crate) fn entry_events(&mut self, seq: Seq) -> Vec<OutboundLine> {
        let mut parts = self.entry_events_in(seq, false);
        if self.other.is_some() && self.conns.iter().any(|c| self.public_view(&c.principal)) {
            // (R3/K1b) The public entry this entry made (its own number), if any.
            if let Some(p) = self.pub_of(seq) {
                let pubs = self.in_world(true, |s| s.entry_events_in(p, true));
                for (k, p) in pubs.into_iter().enumerate() {
                    parts[k].extend(p);
                }
            }
            // Change events in connection order, as one world gives them.
            parts[0].sort_by_key(|(ci, _)| *ci);
        }
        parts.into_iter().flatten().map(|(_, l)| l).collect()
    }

    /// [`Session::entry_events`] of the world the fields hold, for the connections of
    /// that world (`public`; without a public world, every connection), in three groups
    /// (changes, finding states, proposals), each line with its connection index.
    pub(crate) fn entry_events_in(
        &self,
        seq: Seq,
        public: bool,
    ) -> [Vec<(usize, OutboundLine)>; 3] {
        let e = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        let mut lines = Vec::new();
        let mut states = Vec::new();
        let mut props = Vec::new();
        let mine = |c: &Conn| self.other.is_none() || self.public_view(&c.principal) == public;
        let touched = touched_ids(&e.changes);
        for (ci, c) in self.conns.iter().enumerate() {
            if !mine(c) {
                continue;
            }
            // (R3, §20) What this subscriber may receive of the engine's items.
            let proj = if c
                .subs
                .iter()
                .any(|s| matches!(s.topic, Topic::Changes { .. }))
            {
                self.reader_proj(&c.principal)
            } else {
                None
            };
            let projected = proj
                .as_ref()
                .map(|p| self.project_entry(e, p, self.lang_of(ci)));
            let shown = projected.as_ref().unwrap_or(e);
            for s in &c.subs {
                if let Topic::Changes { ids } = &s.topic {
                    if ids.is_empty() || ids.iter().any(|i| touched.contains(i)) {
                        if proj.is_some() && !ids.is_empty() {
                            let vis = touched_ids(&shown.changes);
                            if !ids.iter().any(|i| vis.contains(i)) {
                                continue;
                            }
                        }
                        lines.push((
                            ci,
                            out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: s.id.clone(),
                                    event: Event::Change {
                                        seq,
                                        at: e.at,
                                        author: e.author.clone(),
                                        approved_by: e.approved_by.clone(),
                                        cause: shown.cause.clone(),
                                        changes: shown.changes.clone(),
                                        evidence: e.evidence.as_ref().map(|ev| {
                                            crate::claims::for_principal(
                                                &self.doc,
                                                &self.plane,
                                                &c.principal,
                                                ev,
                                            )
                                        }),
                                    },
                                },
                            ),
                        ));
                    }
                }
            }
        }
        for ch in &e.changes {
            let ChangeOp::SetCapsuleState { capsule, state } = ch else {
                continue;
            };
            let Some(finding) = self
                .doc
                .capsules
                .iter()
                .find(|c| &c.id == capsule)
                .map(|c| c.finding.clone())
            else {
                continue;
            };
            for (ci, c) in self.conns.iter().enumerate() {
                if !mine(c) {
                    continue;
                }
                for s in &c.subs {
                    if matches!(&s.topic, Topic::Finding { finding: f } if *f == finding) {
                        if self
                            .proj_for(&c.principal)
                            .is_some_and(|p| p.hidden.contains(capsule))
                        {
                            continue;
                        }
                        states.push((
                            ci,
                            out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: s.id.clone(),
                                    event: Event::FindingState {
                                        finding: finding.clone(),
                                        capsule: capsule.clone(),
                                        state: *state,
                                        seq,
                                        evidence: e.evidence.as_ref().map(|ev| {
                                            crate::claims::for_principal(
                                                &self.doc,
                                                &self.plane,
                                                &c.principal,
                                                ev,
                                            )
                                        }),
                                    },
                                },
                            ),
                        ));
                    }
                }
            }
        }
        for ch in &e.changes {
            let ChangeOp::PutProposal { proposal, .. } = ch else {
                continue;
            };
            let author = self.principal_of(&proposal.provenance.author);
            for (ci, c) in self.conns.iter().enumerate() {
                if !mine(c) {
                    continue;
                }
                for s in &c.subs {
                    if matches!(s.topic, Topic::Proposals {}) {
                        props.push((
                            ci,
                            out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: s.id.clone(),
                                    event: Event::Proposal {
                                        proposal: proposal.id.clone(),
                                        state: proposal.state,
                                        author: author.clone(),
                                        seq,
                                    },
                                },
                            ),
                        ));
                    }
                }
            }
        }
        [lines, states, props]
    }

    // -----------------------------------------------------------------------
    // Subscriptions, clock
    // -----------------------------------------------------------------------

    fn subscribe(&mut self, ci: usize, id: &str, topic: Topic) -> PResult<()> {
        if !permissions(self.conns[ci].role).read {
            return Err(forbidden_read());
        }
        let c = &self.conns[ci];
        if c.subs.iter().any(|s| s.id == id) {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("subscription `{id}` already exists on this connection"),
            )
            .at("/id")
            .build());
        }
        if c.subs.len() >= self.limits.max_subscriptions as usize {
            return Err(E::new(
                ErrorCode::Limit,
                format!(
                    "a connection holds at most {} subscriptions",
                    self.limits.max_subscriptions
                ),
            )
            .expected([self.limits.max_subscriptions.to_string()])
            .hint("Unsubscribe from something first.")
            .build());
        }
        let mut last = None;
        match &topic {
            Topic::Finding { finding } => {
                need_finding(&self.doc, finding, "/topic/finding")?;
            }
            Topic::Condition { condition } => {
                if matches!(condition, qp_schema::Condition::Text { .. }) {
                    return Err(invalid(
                        "text conditions are not machine-checkable",
                        "/topic/condition/kind",
                    ));
                }
                check_condition(&self.doc, condition, "/topic/condition")?;
                crate::expand::condition_entitled(
                    &self.plane,
                    &self.doc,
                    &self.conns[ci].principal,
                    condition,
                    "/topic/condition",
                )?;
                last = Some(holds(rd!(self), &mut self.cache, condition));
            }
            Topic::Attention { principal: Some(p) } => {
                if self.principal(p).is_none() {
                    return Err(
                        E::new(ErrorCode::NotFound, format!("unknown principal `{p}`"))
                            .at("/topic/principal")
                            .dym(p, self.principals.iter().map(|x| x.id.as_str()))
                            .build(),
                    );
                }
            }
            Topic::View { view } => {
                crate::expand::need_view(&self.doc, view, "/topic/view")?;
            }
            Topic::Viewer { limit, width } => {
                self.viewer_check(ci, *limit, *width)?;
            }
            Topic::Changes { .. }
            | Topic::Proposals {}
            | Topic::Attention { principal: None }
            | Topic::Intents {}
            | Topic::Claims {}
            | Topic::Capsules { .. } => {}
            Topic::Stories { instrument, series } => {
                self.stories_check(instrument, series.as_deref())?;
            }
        }
        self.conns[ci].subs.push(Sub {
            id: id.to_owned(),
            topic,
            last,
            memo: None,
            watch: None,
            stories: None,
        });
        Ok(())
    }

    /// (R4, §21) The host's own clock input, without a connection: what a `clock` message
    /// on the host's connection does (same checks, same outputs). The replay host gives
    /// the virtual time with it before each step's data events.
    ///
    /// # Errors
    /// The clock would go back (`invalid`).
    pub fn host_clock(&mut self, now: i64) -> Result<Vec<OutboundLine>, ProtocolError> {
        self.advance_clock(now)
    }

    /// (R6/T3, §24) The host's market status input: the exchange time, whether the market
    /// is open, each instrument's session phase, delayed/stale. Not the clock (that is
    /// still [`Session::host_clock`]); it changes no document and enters no log. When it
    /// differs from the last one, every `viewer` subscription is told (`market` in its
    /// event, and the capsules whose sentences now speak of the market); the `capsules`
    /// answers and events carry the market note from then on (`docs/LANGUAGE.md`).
    pub fn set_market_status(
        &mut self,
        status: qp_schema::market::MarketStatus,
    ) -> Vec<OutboundLine> {
        if self.market.as_ref() == Some(&status) {
            return Vec::new();
        }
        self.market = Some(status);
        self.viewer_flush()
    }

    /// (R6/T3, §24, §20.5) Whether `principal` may see the bars of instrument `symbol`: a
    /// bound series of it whose every feed the reader is entitled to.
    pub(crate) fn symbol_entitled(&self, principal: &str, symbol: &str) -> bool {
        self.doc
            .series
            .iter()
            .filter(|s| s.instrument.as_ref().is_some_and(|i| i.symbol == symbol))
            .any(|s| {
                crate::claims::feeds_of(&self.doc, &[s.id.as_str()]).is_some_and(|ks| {
                    !ks.is_empty() && ks.iter().all(|k| self.plane.entitled(principal, k))
                })
            })
    }

    /// (R6/T3, §24, §20.5) The market status as reader `principal` gets it: the phases of
    /// the instruments whose bars it may see, without the feed's numeric codes (`code`
    /// 0, not sent); open/closed, holiday, delayed and stale for everyone.
    #[must_use]
    pub fn market_for(&self, principal: &str) -> Option<qp_schema::market::MarketStatus> {
        let m = self.market.as_ref()?;
        let mut v = m.clone();
        v.phases = m
            .phases
            .iter()
            .filter(|(sym, _)| self.symbol_entitled(principal, sym))
            .map(|(sym, p)| {
                (
                    sym.clone(),
                    qp_schema::market::PhaseCode {
                        code: 0,
                        key: p.key.clone(),
                    },
                )
            })
            .collect();
        Some(v)
    }

    /// (R6/T3) The market status the host last gave.
    #[must_use]
    pub fn market_status(&self) -> Option<&qp_schema::market::MarketStatus> {
        self.market.as_ref()
    }

    /// (R6/T3, §24) Writes the market note into a capsule item for its reader: the R4
    /// sentence (`text`, `tr`, `en`) ends with the note's sentence; a voice's headline with
    /// its clause, its detail with its sentence, and its text face is rebuilt. Nothing
    /// without a market status or when the instrument trades continuously on fresh data.
    pub(crate) fn market_item(
        &self,
        it: &mut qp_schema::protocol::CapsuleItem,
        lang: qp_schema::protocol::Language,
        principal: &str,
    ) {
        use qp_capsule::market::{note_of, with_clause, with_sentence};
        use qp_schema::protocol::Language;
        let Some(m) = self.market.as_ref() else {
            return;
        };
        // (§24, §20.5) An instrument's phase speaks only to a reader entitled to its bars;
        // the others hear the market's open/closed and stale only.
        let note = if !m.phases.contains_key(&it.instrument)
            || self.symbol_entitled(principal, &it.instrument)
        {
            note_of(m, &it.instrument)
        } else {
            let general = qp_schema::market::MarketStatus {
                phases: BTreeMap::new(),
                ..m.clone()
            };
            note_of(&general, &it.instrument)
        };
        let Some(note) = note else {
            return;
        };
        let red = it.redacted;
        it.sentence.tr = with_sentence(&it.sentence.tr, note, Language::Tr, red);
        it.sentence.en = with_sentence(&it.sentence.en, note, Language::En, red);
        it.sentence.text = match lang {
            Language::Tr => it.sentence.tr.clone(),
            Language::En => it.sentence.en.clone(),
        };
        if let Some(v) = it.voice.as_mut() {
            v.headline = with_clause(&v.headline, note, lang);
            v.detail = with_sentence(&v.detail, note, lang, red);
            v.face = qp_capsule::voice::text_face(&qp_capsule::voice::Surfaces {
                headline: v.headline.clone(),
                detail: v.detail.clone(),
                ledger: v.ledger.clone(),
                glossary: Vec::new(),
            });
        }
    }

    fn tick(&mut self, ci: usize, now: i64) -> PResult<Vec<OutboundLine>> {
        if self.conns[ci].role != Role::Host {
            return Err(E::new(ErrorCode::Forbidden, "only the host sends `clock`")
                .hint("The authority's time comes from its host connection.")
                .build());
        }
        self.advance_clock(now)
    }

    fn advance_clock(&mut self, now: i64) -> PResult<Vec<OutboundLine>> {
        if now < self.clock {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("clock went back from {} to {now}", self.clock),
            )
            .at("/now")
            .expected([format!(">= {}", self.clock)])
            .build());
        }
        self.clock = now;
        let due: Vec<Proposal> = self
            .doc
            .proposals
            .iter()
            .filter(|p| p.state == ProposalState::Pending && p.expires_at.is_some_and(|e| now >= e))
            .cloned()
            .collect();
        let mut lines = Vec::new();
        for mut p in due {
            let seq = self.seq() + 1;
            p.state = ProposalState::Expired;
            p.decided_seq = Some(seq);
            let pid = p.id.clone();
            if let Some(s) = self.engine_entry(
                vec![ChangeOp::PutProposal {
                    proposal: p,
                    index: None,
                }],
                format!("expire {pid}"),
            ) {
                lines.extend(self.settle(s));
            }
        }
        Ok(lines)
    }

    /// True if `c` reads a series resting on a bound series that is not ready for the
    /// authority (pending, no data, stale): the condition is not evaluated (§18.3).
    pub(crate) fn cond_blocked(&self, c: &qp_schema::Condition) -> bool {
        crate::live::cond_blocked(&self.plane, &self.doc, c)
    }

    /// The `welcome` summary for `principal`: over the reading document (`len` is the
    /// number of bars the authority holds), each bound series with its binding state for
    /// that principal (§18.2).
    pub(crate) fn summary_for(&self, principal: &str) -> qp_schema::protocol::DocSummary {
        let mut sum = crate::summary::summarize_doc(rd!(self));
        for s in &mut sum.series {
            if let Some((state, _)) = self.plane.state_for(&s.id, Some(principal)) {
                s.binding = Some(state);
            }
        }
        sum
    }

    /// Notifications of a viewport change of `principal` to its attention watchers.
    pub(crate) fn viewport_events(&self, principal: &str, vp: &Viewport) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Attention { principal: p } = &s.topic {
                    if p.as_deref().unwrap_or(&c.principal) == principal {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Viewport {
                                    principal: principal.to_owned(),
                                    viewport: vp.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        lines
    }

    /// Attention directive delivery (§4.8).
    fn attend(
        &mut self,
        ci: usize,
        mut d: AttentionDirective,
    ) -> PResult<(String, Vec<OutboundLine>)> {
        if !permissions(self.conns[ci].role).attend {
            return Err(E::new(
                ErrorCode::Forbidden,
                format!(
                    "role `{}` may not send attention directives",
                    role_str(self.conns[ci].role)
                ),
            )
            .build());
        }
        if self.principal(&d.to).is_none() {
            return Err(
                E::new(ErrorCode::NotFound, format!("unknown principal `{}`", d.to))
                    .at("/directive/to")
                    .dym(&d.to, self.principals.iter().map(|x| x.id.as_str()))
                    .build(),
            );
        }
        if d.steps.is_empty() {
            return Err(invalid(
                "a directive needs at least one step",
                "/directive/steps",
            ));
        }
        for (j, s) in d.steps.iter().enumerate() {
            crate::query::check_target_pub(
                &self.doc,
                &s.target,
                &format!("/directive/steps/{j}/target"),
            )?;
        }
        if let Some(i) = &d.id {
            if !crate::expand::valid_id(i) {
                return Err(invalid(format!("invalid id `{i}`"), "/directive/id"));
            }
        }
        self.att_counter += 1;
        let id =
            d.id.clone()
                .unwrap_or_else(|| format!("att-{}", self.att_counter));
        d.id = Some(id.clone());
        d.from = Some(self.conns[ci].principal.clone());
        d.at = Some(self.clock);
        let list = self.directives.entry(d.to.clone()).or_default();
        list.push(d.clone());
        if list.len() > MAX_DIRECTIVES {
            let extra = list.len() - MAX_DIRECTIVES;
            list.drain(..extra);
        }
        let mut lines = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Attention { principal: p } = &s.topic {
                    if p.as_deref().unwrap_or(&c.principal) == d.to {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Attention {
                                    directive: d.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        Ok((id, lines))
    }
}

/// Applies `changes` to `doc` in place; if one does not fit, the ones applied before it
/// are undone with their inverses, so `doc` is left as it was (the same bytes, as undo
/// guarantees).
fn apply_or_roll_back(doc: &mut Document, changes: &[ChangeOp]) {
    let mut inverses = Vec::with_capacity(changes.len());
    for c in changes {
        match crate::apply::apply_change(doc, c) {
            Ok(inv) => inverses.push(inv),
            Err(_) => {
                for inv in inverses.iter().rev() {
                    let _ = crate::apply::apply_change(doc, inv);
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
