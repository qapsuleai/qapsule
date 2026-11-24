//! (RF1 S3.0) The authority's side of the subsystem contract (`qp-subsystem`): the read
//! view subsystems get, registration, the phases' collect and emit steps, and the
//! dispatchers that ask the registry first (a kind a subsystem owns) and otherwise run
//! the built-in path unchanged (today's code, today's errors). With an empty registry
//! every check here misses at once.

use qp_capsule::sentence::StudyMemo;
use qp_schema::capsule::Story;
use qp_schema::market::MarketStatus;
use qp_schema::protocol::{
    AttentionDirective, CapsuleItem, CapsuleVoice, ClaimEvidence, Gesture, Language, Query,
    QueryResult, Range, ServerComment, StoryRow, Viewport,
};
use qp_subsystem::{ConnView, ReadView, SessionCtx, SessionView, Slots, Subsystem, MAX_EMIT_DEPTH};

use super::*;

/// The subsystems an authority is built with (RF1 S3.1), in registration order. The
/// composition (`qp-session`) gives its standard list to the authorities it builds; every
/// authority spawned from one (a replay host's rebuilt session, a replica's follower, a
/// viewer's open side, the tape's sessions) gets the same list. The authority knows none
/// of them. [`Subsystems::NONE`]: no subsystem (the authority alone).
#[derive(Clone, Copy, Default)]
pub struct Subsystems(&'static [&'static dyn Subsystem]);

impl Subsystems {
    /// No subsystem.
    pub const NONE: Self = Self(&[]);

    /// The subsystems `list`, in registration order.
    #[must_use]
    pub const fn new(list: &'static [&'static dyn Subsystem]) -> Self {
        Self(list)
    }

    /// The list, in registration order.
    #[must_use]
    pub fn list(self) -> &'static [&'static dyn Subsystem] {
        self.0
    }
}

/// (RF1 S3.2) Two lists: an authority's and its followers' (replica, open side; the
/// composition's `STANDARD` and `FOLLOWER`), as the replica scenario (`tape`) takes them.
pub type Lists = (Subsystems, Subsystems);

impl std::fmt::Debug for Subsystems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|s| s.name()))
            .finish()
    }
}

/// [`Phase::Expand`]: `ops` expanded against `doc`, an op a subsystem owns by its handler,
/// every other op by `qp_ops::expand::expand_ops` ([`Registry::expand_ops`]).
pub(crate) fn expand_ops(
    reg: &Registry,
    doc: &Document,
    ops: &[Op],
    base: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Expanded> {
    reg.expand_ops(doc, ops, base, ctx)
}

/// A query or input no registered subsystem serves (an authority without the reading
/// subsystem).
fn unserved(what: &str) -> ProtocolError {
    E::new(
        ErrorCode::Unsupported,
        format!("`{what}` is not served here: no subsystem reads"),
    )
    .build()
}

impl SessionView for Session {
    fn document(&self) -> &Document {
        &self.doc
    }
    fn reading(&self) -> &Document {
        rd!(self)
    }
    fn log(&self) -> &[LogEntry] {
        &self.log
    }
    fn seq(&self) -> Seq {
        Session::seq(self)
    }
    fn clock(&self) -> i64 {
        self.clock
    }
    fn principal(&self, id: &str) -> Option<&Principal> {
        Session::principal(self, id)
    }
    fn conn(&self, id: &str) -> Option<ConnView<'_>> {
        let c = self.conns.iter().find(|c| c.id == id)?;
        let (id, principal, role) = (c.id.as_str(), c.principal.as_str(), c.role);
        let (permissions, surface) = (permissions(role), c.surface.kind);
        Some(ConnView {
            id,
            principal,
            role,
            permissions,
            surface,
            hello: &c.surface,
            profile: &c.profile,
        })
    }
    fn conn_ids(&self) -> Vec<&str> {
        self.conns.iter().map(|c| c.id.as_str()).collect()
    }
    fn subs(&self, conn: &str) -> Vec<(&str, &Topic)> {
        self.conns
            .iter()
            .find(|c| c.id == conn)
            .map(|c| c.subs.iter().map(|s| (s.id.as_str(), &s.topic)).collect())
            .unwrap_or_default()
    }
    fn public_view(&self, principal: &str) -> bool {
        Session::public_view(self, principal)
    }
    fn reader_proj(&self, principal: &str) -> Option<std::rc::Rc<qp_plane::proj::Proj>> {
        Session::reader_proj(self, principal)
    }
    fn plane(&self) -> &DataPlane {
        &self.plane
    }
    fn market(&self) -> Option<&MarketStatus> {
        self.market.as_ref()
    }
    fn in_public(&self) -> bool {
        self.in_public
    }
}

impl ReadView for Session {
    fn principals(&self) -> &[Principal] {
        &self.principals
    }
    fn headlines(&self) -> &BTreeMap<String, String> {
        &self.headlines
    }
    fn headline_source(&self) -> &dyn qp_ops::HeadlineSource {
        self
    }
    fn proj(&self) -> Option<std::rc::Rc<qp_plane::proj::Proj>> {
        self.proj.clone()
    }
    fn directives(&self, principal: &str) -> Option<&Vec<AttentionDirective>> {
        self.directives.get(principal)
    }
    fn project_entry(&self, e: &LogEntry, p: &qp_plane::proj::Proj, lang: Language) -> LogEntry {
        Session::project_entry(self, e, p, lang)
    }
    fn evidence_for(&self, principal: &str, ev: &ClaimEvidence) -> ClaimEvidence {
        crate::claims::for_principal(&self.doc, &self.plane, principal, ev)
    }
    fn viewport_events(&self, principal: &str, vp: &Viewport) -> Vec<OutboundLine> {
        Session::viewport_events(self, principal, vp)
    }
    fn accept_preview(&self, pid: &str, decider: &str) -> PResult<(Document, Vec<ChangeOp>)> {
        let acc = self.accept_expand(pid, None, decider, None, Session::seq(self) + 1)?;
        Ok((acc.doc, acc.changes))
    }
    fn stories_query(
        &self,
        conn: usize,
        instrument: &str,
        series: Option<&str>,
        range: Option<&Range>,
        page: (Option<u32>, Option<&str>),
        path: &str,
    ) -> PResult<QueryResult> {
        Session::stories_query(self, conn, instrument, series, range, page, path)
    }
    fn story_rows(&self, series: &str, principal: &str, lang: Language) -> Vec<StoryRow> {
        Session::story_rows(self, series, principal, lang)
    }
    fn row_stamp(&self, row: &StoryRow, lang: Language) -> Option<String> {
        Session::row_stamp(self, row, lang)
    }
    fn capsule_item(
        &self,
        doc: &Document,
        c: &qp_schema::Capsule,
        p: Option<&qp_plane::proj::Proj>,
        lang: Language,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleItem> {
        self.capsule_item_with(doc, c, p, lang, principal, memo)
    }
    fn reader_voice(
        &self,
        doc: &Document,
        c: &qp_schema::Capsule,
        p: Option<&qp_plane::proj::Proj>,
        lang: Language,
        voice: &str,
        clock: bool,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleVoice> {
        Session::reader_voice(self, doc, c, p, lang, voice, clock, memo)
    }
    fn market_item(&self, it: &mut CapsuleItem, lang: Language, principal: &str) {
        Session::market_item(self, it, lang, principal);
    }
    fn capsule_passes(&self, doc: &Document, principal: &str, story: &Story, q: &Query) -> bool {
        Session::capsule_passes(self, doc, principal, story, q)
    }
    fn coin_flip_hidden_by(
        &self,
        doc: &Document,
        principal: &str,
        story: &Story,
        q: &Query,
    ) -> bool {
        Session::coin_flip_hidden_by(self, doc, principal, story, q)
    }
    fn relevant(&self, doc: &Document, principal: &str, instrument: &str) -> bool {
        Session::relevant(self, doc, principal, instrument)
    }
    fn coin_flip_of(&self, principal: &str, story: &Story) -> bool {
        Session::coin_flip_of(self, principal, story)
    }
    fn is_current(&self, it: &CapsuleItem) -> bool {
        Session::is_current(self, it)
    }
    fn item_series<'d>(&'d self, doc: &'d Document, series: &str) -> Option<&'d qp_schema::Series> {
        self.reader_item_series(doc, series)
    }
    fn fact_by_id(&self, id: &str) -> Option<&qp_schema::Fact> {
        Session::fact_by_id(self, id)
    }
    fn item_memo(&self) -> &std::cell::RefCell<qp_plane::capidx::ItemMemo> {
        &self.item_memo
    }
    fn headline_cal(&self) -> Option<&qp_schema::Calendar> {
        self.headline_cal.as_ref()
    }
    fn comments_for(&self, principal: &str) -> Vec<ServerComment> {
        Session::comments_for(self, principal)
    }
    fn capsule_query(
        &self,
        conn: usize,
        id: &str,
        path: &str,
        look: (Option<&str>, bool),
    ) -> PResult<QueryResult> {
        Session::capsule_query(self, conn, id, path, look)
    }
    fn ledger_now(&self) -> Vec<(String, qp_schema::capsule::MeaningKind, i64, bool)> {
        Session::ledger_now(self)
    }
}

impl Session {
    /// (RF1 S3.0) Registers subsystem `sub` after the ones before it and lets it put its
    /// initial state. The composition (`qp-session`) registers the standard subsystems.
    ///
    /// # Errors
    /// One of its kinds (op, query, input, topic, condition, claim) is already registered.
    #[doc(hidden)]
    pub fn register(&mut self, sub: &'static dyn Subsystem) -> Result<(), String> {
        self.registry.add(sub)?;
        self.with_ctx(|_, ctx| sub.init(ctx));
        Ok(())
    }

    /// (RF1 S3.0) The earliest clock a subsystem asked to be woken at (`None`: none).
    #[must_use]
    pub fn next_wake(&self) -> Option<i64> {
        self.wake
    }

    /// (RF1 S3.1) The subsystems it was built with.
    #[must_use]
    pub fn subsystems(&self) -> Subsystems {
        self.subs
    }

    /// (RF1 S3.1) The subsystems' state (the composition's `sub::<T>()`).
    #[doc(hidden)]
    #[must_use]
    pub fn slots(&self) -> &Slots {
        &self.slots
    }

    /// (RF1 S3.1) The subsystems' state, to change (the composition's `sub_mut::<T>()`).
    #[doc(hidden)]
    pub fn slots_mut(&mut self) -> &mut Slots {
        &mut self.slots
    }

    /// (RF1 S3.2) Runs `f` with a call context (the composition's façade methods that read
    /// through a subsystem: `qp_session::Session::scene_for`).
    #[doc(hidden)]
    pub fn with_subsystems<R>(&mut self, f: impl FnOnce(&mut SessionCtx<'_>) -> R) -> R {
        self.with_read_ctx(|_, ctx| f(ctx))
    }

    /// Runs `f` with the registry and a call context: a split borrow of the authority
    /// (the read view; the slots, the outbox, the values cache and the connections' view
    /// state taken out for the call and put back).
    pub(crate) fn with_ctx<R>(&mut self, f: impl FnOnce(&Registry, &mut SessionCtx<'_>) -> R) -> R {
        self.with_ctx_as::<false, R>(f)
    }

    /// [`Session::with_ctx`] with the read view (`SessionCtx::read`): a query's or a
    /// gesture's call (RF1 S3.2).
    pub(crate) fn with_read_ctx<R>(
        &mut self,
        f: impl FnOnce(&Registry, &mut SessionCtx<'_>) -> R,
    ) -> R {
        self.with_ctx_as::<true, R>(f)
    }

    /// `READ` is a constant so that a phase's call never names the read view's table (a
    /// build without queries leaves it out).
    fn with_ctx_as<const READ: bool, R>(
        &mut self,
        f: impl FnOnce(&Registry, &mut SessionCtx<'_>) -> R,
    ) -> R {
        let (mut slots, mut out) = (
            std::mem::take(&mut self.slots),
            std::mem::take(&mut self.outbox),
        );
        let (mut cache, mut looks) = (
            std::mem::take(&mut self.cache),
            std::mem::take(&mut self.looks),
        );
        let mut ctx = SessionCtx {
            view: &*self,
            slots: &mut slots,
            out: &mut out,
            cache: &mut cache,
            looks: &mut looks,
            read: None,
        };
        if READ {
            ctx.read = Some(&*self);
        }
        let r = f(&self.registry, &mut ctx);
        (self.slots, self.outbox) = (slots, out);
        (self.cache, self.looks) = (cache, looks);
        r
    }

    /// The collect step of `phase`: the subsystems that named it, in registration order.
    pub(crate) fn phase(&mut self, phase: Phase, at: Moment<'_>) {
        if self.registry.wants(phase) {
            self.with_ctx(|r, ctx| r.collect(phase, &at, ctx));
        }
    }

    /// [`Phase::AfterData`], the capsule engine's look at one series in the world the
    /// fields hold (`public`: the public world's engine): the changes the subsystems add
    /// to the look's engine entry (watches, §25.5).
    pub(crate) fn look_phase(
        &mut self,
        events: &[qp_capsule::StoryEvent],
        public: bool,
    ) -> Vec<ChangeOp> {
        if !self.registry.wants(Phase::AfterData) {
            return Vec::new();
        }
        self.phase(Phase::AfterData, Moment::Look { events, public });
        self.outbox.take_changes()
    }

    /// [`Phase::AfterData`], the look's engine entry `seq` was written: the subsystems'
    /// notifications at this line (`intent_matched`, §25.5).
    pub(crate) fn written_phase(&mut self, seq: Seq) -> Vec<OutboundLine> {
        if !self.registry.wants(Phase::AfterData) {
            return Vec::new();
        }
        let public = self.pub_of(seq);
        self.phase(Phase::AfterData, Moment::Written { seq, public });
        self.outbox.take_lines()
    }

    /// The emit step: what the subsystems asked for, in request order — engine entries
    /// (each settled as any engine entry; depth first, at most [`MAX_EMIT_DEPTH`] levels
    /// deep: audit-s30 B3), then their notifications. Called at the end of every settle
    /// point and of every message.
    pub(crate) fn emit(&mut self) -> Vec<OutboundLine> {
        if self.outbox.is_empty() {
            return Vec::new();
        }
        let (entries, notes, wake) = self.outbox.drain();
        self.wake = wake.into_iter().chain(self.wake).min();
        let mut lines = Vec::new();
        if self.emit_depth < MAX_EMIT_DEPTH {
            self.emit_depth += 1;
            for e in entries {
                if let Some(s) = self.engine_entry(e.changes, e.summary) {
                    lines.extend(self.settle(s));
                }
            }
            self.emit_depth -= 1;
        }
        lines.extend(notes);
        lines
    }

    /// A message ([`Session::handle_resolved`]), then the emit step of its phases.
    pub(super) fn handle_msg(
        &mut self,
        conn: &str,
        msg: ClientMsg,
        resolved: BTreeMap<String, String>,
    ) -> Vec<OutboundLine> {
        let mut lines = self.handle_resolved(conn, msg, resolved);
        lines.extend(self.emit());
        lines
    }

    /// The command's own checks ([`Session::check_comments`]), then [`Phase::Check`]: the
    /// subsystems may refuse it.
    pub(super) fn check_command(&mut self, ci: usize, ops: &[Op]) -> PResult<()> {
        self.check_comments(ci, ops)?;
        if !self.registry.wants(Phase::Check) {
            return Ok(());
        }
        let conn = self.conns[ci].id.clone();
        self.with_ctx(|r, ctx| r.check(&conn, ops, ctx))
    }

    /// `hello`, then [`Phase::Connect`] when it connected.
    pub(super) fn dispatch_hello(
        &mut self,
        conn: &str,
        ci: Option<usize>,
        protocol: &str,
        principal: &str,
        surface: Surface,
        features: &[String],
    ) -> Vec<OutboundLine> {
        let lines = self.hello(conn, ci, protocol, principal, surface, features);
        if ci.is_none() && self.conn_index(conn).is_some() {
            self.phase(Phase::Connect, Moment::Conn { conn });
        }
        lines
    }

    /// A query: its kind's handler (the reading subsystem answers every kind no other
    /// subsystem took: `qp-read`, RF1 S3.2); none: `unsupported`.
    pub(crate) fn dispatch_query(&mut self, ci: usize, q: &Query) -> PResult<QueryResult> {
        let Some(h) = self.registry.query(q) else {
            return Err(unserved("query"));
        };
        let conn = self.conns[ci].id.clone();
        self.with_read_ctx(|_, ctx| h.answer(&conn, q, ctx))
    }

    /// A gesture: its kind's handler (the reading subsystem, as for queries); none:
    /// `unsupported`.
    pub(super) fn dispatch_input(
        &mut self,
        ci: usize,
        g: &Gesture,
    ) -> PResult<(QueryResult, Vec<OutboundLine>)> {
        let Some(h) = self.registry.input(g) else {
            return Err(unserved("input"));
        };
        let conn = self.conns[ci].id.clone();
        self.with_read_ctx(|_, ctx| h.input(&conn, g, ctx))
    }

    /// A subscription: the built-in checks, then its topic kind's handler (a refusal
    /// drops the subscription again and undoes the built-in subscribe's side effects:
    /// audit-s30 B5), then [`Phase::Subscribe`].
    pub(super) fn dispatch_subscribe(&mut self, ci: usize, id: &str, topic: Topic) -> PResult<()> {
        let h = self.registry.topic(&topic).map(|h| (h, topic.clone()));
        self.subscribe(ci, id, topic)?;
        let conn = self.conns[ci].id.clone();
        if let Some((h, t)) = h {
            if let Err(e) = self.with_ctx(|_, ctx| h.subscribe(&conn, id, &t, ctx)) {
                self.conns[ci].subs.retain(|s| s.id != id);
                self.viewer_unsubscribed(&conn, id);
                return Err(e);
            }
        }
        self.phase(
            Phase::Subscribe,
            Moment::Sub {
                conn: &conn,
                sub: id,
            },
        );
        Ok(())
    }

    /// After subscription `sub` of `conn` was dropped: viewers, then [`Phase::Unsubscribe`].
    pub(super) fn unsubscribed(&mut self, conn: &str, sub: &str) {
        self.viewer_unsubscribed(conn, sub);
        self.phase(Phase::Unsubscribe, Moment::Sub { conn, sub });
    }

    /// A dispute: its claim kind's verdict, else the built-in dispute.
    pub(super) fn dispatch_dispute(
        &mut self,
        ci: usize,
        id: &str,
        d: crate::dispute::DisputeIn,
    ) -> PResult<Vec<OutboundLine>> {
        let Some(k) = self.registry.claim(&d.claim) else {
            return self.dispute(ci, id, d);
        };
        let (conn, p) = (self.conns[ci].id.clone(), self.conns[ci].principal.clone());
        let (seq, now) = (self.seq_to(&p, self.seq()), self.clock());
        let seen = (
            conn.clone(),
            d.claim.clone(),
            d.t,
            d.digest.clone().unwrap_or_default(),
        );
        let v = match self.dispute_guard(&seen, d.digest.as_deref())? {
            Some(v) => v,
            None => {
                let v = self.with_ctx(|_, ctx| k.verdict(&conn, &d.claim, d.t, ctx));
                self.settle.seen_insert(seen, v);
                v
            }
        };
        Ok(vec![crate::dispute::verdict_line(
            &conn, id, seq, now, v, None,
        )])
    }

    /// (audit-s30 B5) The built-in dispute's guards, for a claim kind a subsystem owns: a
    /// malformed digest is an error; a dispute made already is `duplicate`; past the
    /// per-connection or per-claim limit, `limited`.
    fn dispute_guard(
        &self,
        k: &crate::dispute::SeenKey,
        digest: Option<&str>,
    ) -> PResult<Option<qp_schema::protocol::DisputeVerdict>> {
        use crate::dispute::{valid_digest, MAX_DIGESTS, MAX_PER_CONN};
        use qp_schema::protocol::DisputeVerdict;
        if digest.is_some_and(|d| !valid_digest(d)) {
            let m = "`digest` must be 16 lowercase hex digits (data_digest)";
            return Err(E::new(ErrorCode::Invalid, m).at("/digest").build());
        }
        let seen = &self.settle.seen;
        let mine = || seen.keys().filter(|s| s.0 == k.0);
        Ok(if seen.contains_key(k) {
            Some(DisputeVerdict::Duplicate)
        } else if mine().count() >= MAX_PER_CONN
            || mine().filter(|s| s.1 == k.1 && s.2 == k.2).count() >= MAX_DIGESTS
        {
            Some(DisputeVerdict::Limited)
        } else {
            None
        })
    }

    /// The settled answer's capsule outcomes ([`Session::capsule_settled`]), then
    /// [`Phase::Settled`] (D1; emitted at the settle point that follows).
    pub(crate) fn settled_phase(
        &mut self,
        series: &str,
        lo: i64,
        hi: i64,
        corrected: bool,
    ) -> Vec<OutboundLine> {
        let lines = self.capsule_settled(series, lo, hi, corrected);
        self.phase(
            Phase::Settled,
            Moment::Settled {
                series,
                lo,
                hi,
                corrected,
            },
        );
        lines
    }

    /// (RF1 S3.0, draft hook for RS1, K88) A corporate action on `symbol`: each subsystem
    /// gives its plan ([`Phase::Adjust`]); the authority writes one engine entry with every
    /// plan's changes, then the subsystems' notifications.
    #[doc(hidden)]
    pub fn host_adjust(&mut self, symbol: &str, factor: f64, at: i64) -> Vec<OutboundLine> {
        self.phase(Phase::Adjust, Moment::Adjust { symbol, factor, at });
        let (entries, notes, wake) = self.outbox.drain();
        self.wake = wake.into_iter().chain(self.wake).min();
        let changes: Vec<ChangeOp> = entries.into_iter().flat_map(|e| e.changes).collect();
        let mut lines = Vec::new();
        if !changes.is_empty() {
            if let Some(s) = self.engine_entry(changes, format!("adjust {symbol}")) {
                lines.extend(self.settle(s));
            }
        }
        lines.extend(notes);
        lines
    }
}
