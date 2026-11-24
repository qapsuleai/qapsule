//! The narrow read view a subsystem gets of the authority.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use qp_capsule::sentence::StudyMemo;
use qp_ops::HeadlineSource;
use qp_plane::capidx::ItemMemo;
use qp_plane::error::PResult;
use qp_plane::plane::DataPlane;
use qp_plane::proj::Proj;
use qp_schema::capsule::{MeaningKind, Story};
use qp_schema::data::FeedKey;
use qp_schema::market::MarketStatus;
use qp_schema::protocol::{
    AttentionDirective, CapsuleItem, CapsuleVoice, ChangeOp, ClaimEvidence, Language, LogEntry,
    OutboundLine, Permissions, Principal, Profile, Query, QueryResult, Range, Role, Seq,
    ServerComment, StoryRow, Surface, SurfaceKind, Topic, Viewport,
};
use qp_schema::{Calendar, Capsule, Document, Fact, Series};

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
    /// The whole surface it said hello with (kind, language, depth; RF1 S3.2).
    pub hello: &'a Surface,
    /// Its presentation profile (RF1 S3.2).
    pub profile: &'a Profile,
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
    /// The connections that said hello, in order.
    fn conn_ids(&self) -> Vec<&str>;
    /// Connection `conn`'s subscriptions (id, topic), in order.
    fn subs(&self, conn: &str) -> Vec<(&str, &Topic)>;
    /// Whether `principal` reads the public world (`docs/CONCEPTS.md`, two worlds).
    fn public_view(&self, principal: &str) -> bool;
    /// What `principal` may receive of the document (`None`: everything).
    fn reader_proj(&self, principal: &str) -> Option<Rc<Proj>>;
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

    // ---- Reading (RF1 S3.2, `qp-read`). A query or input handler runs inside the
    // authority's `as_reader`: the document, the reading document and everything below are
    // the asker's (its projection, its world, its language's headlines). Each method is
    // the authority's own function of that name, unchanged.

    /// The principals of the session, in order.
    fn principals(&self) -> &[Principal];
    /// Finding id → the capsule sentence the grammar provider speaks with for the reader.
    fn headlines(&self) -> &BTreeMap<String, String>;
    /// The reader's headline pieces and voice surfaces (`DocProvider::with_parts`).
    fn headline_source(&self) -> &dyn HeadlineSource;
    /// The projection of the reader being read for (`None`: it may see everything).
    fn proj(&self) -> Option<Rc<Proj>>;
    /// The attention directives sent to `principal` (`attend`), in order.
    fn directives(&self, principal: &str) -> Option<&Vec<AttentionDirective>>;
    /// Log entry `e` as a reader with projection `p` in `lang` may receive it.
    fn project_entry(&self, e: &LogEntry, p: &Proj, lang: Language) -> LogEntry;
    /// Claim evidence `ev` as `principal` may read it (data-plane values, §18.3).
    fn evidence_for(&self, principal: &str, ev: &ClaimEvidence) -> ClaimEvidence;
    /// The events a viewport change of `principal` sends (`viewport` presence, §7).
    fn viewport_events(&self, principal: &str, vp: &Viewport) -> Vec<OutboundLine>;
    /// Pending proposal `pid` accepted now by `decider` (as its reader would), not
    /// written: the document after it and its changes (the `preview` query).
    ///
    /// # Errors
    /// As accepting it would fail.
    fn accept_preview(&self, pid: &str, decider: &str) -> PResult<(Document, Vec<ChangeOp>)>;

    // ---- Capsule life (stays in the authority; `qp-read` asks). `conn`: a connection's
    // index in [`SessionView::conn_ids`] order.

    /// The `stories` query of connection `conn`.
    ///
    /// # Errors
    /// As the query.
    #[allow(clippy::too_many_arguments)]
    fn stories_query(
        &self,
        conn: usize,
        instrument: &str,
        series: Option<&str>,
        range: Option<&Range>,
        page: (Option<u32>, Option<&str>),
        path: &str,
    ) -> PResult<QueryResult>;
    /// The reader's story rows of bar series `series`.
    fn story_rows(&self, series: &str, principal: &str, lang: Language) -> Vec<StoryRow>;
    /// A story row's time, as the reader's clock writes it.
    fn row_stamp(&self, row: &StoryRow, lang: Language) -> Option<String>;
    /// One capsule as listed for a reader (`p`: its projection; `memo`: one pass's values).
    fn capsule_item(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleItem>;
    /// A capsule's voice surfaces for the reader's chosen voice.
    #[allow(clippy::too_many_arguments)]
    fn reader_voice(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        voice: &str,
        clock: bool,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleVoice>;
    /// The market note of a listed capsule (§24).
    fn market_item(&self, it: &mut CapsuleItem, lang: Language, principal: &str);
    /// Whether `story` passes every filter of the `capsules` query `q`.
    fn capsule_passes(&self, doc: &Document, principal: &str, story: &Story, q: &Query) -> bool;
    /// Whether `story` is hidden from the `capsules` query `q` only by the coin-flip rule.
    fn coin_flip_hidden_by(
        &self,
        doc: &Document,
        principal: &str,
        story: &Story,
        q: &Query,
    ) -> bool;
    /// Whether `instrument` is relevant to `principal` (its positions and intents).
    fn relevant(&self, doc: &Document, principal: &str, instrument: &str) -> bool;
    /// Whether the coin-flip rule marks `story` for `principal`.
    fn coin_flip_of(&self, principal: &str, story: &Story) -> bool;
    /// Whether a listed capsule's transition is current.
    fn is_current(&self, it: &CapsuleItem) -> bool;
    /// The series a capsule item reads its bars from.
    fn item_series<'d>(&'d self, doc: &'d Document, series: &str) -> Option<&'d Series>;
    /// A fact by id over the facts sentences read.
    fn fact_by_id(&self, id: &str) -> Option<&Fact>;
    /// The `capsules` items by exact input (entitled readers).
    fn item_memo(&self) -> &RefCell<ItemMemo>;
    /// The headline calendar the host set.
    fn headline_cal(&self) -> Option<&Calendar>;
    /// The server comments `principal` gets.
    fn comments_for(&self, principal: &str) -> Vec<ServerComment>;
    /// The `capsule` query of connection `conn`: capsule (or finding) `id`'s detail
    /// (stays with capsule life: the comment validator reads the same detail).
    ///
    /// # Errors
    /// No capsule with a story `id`, or its bars are not available.
    /// `look`: the connection's voice and clock (its view state).
    fn capsule_query(
        &self,
        conn: usize,
        id: &str,
        path: &str,
        look: (Option<&str>, bool),
    ) -> PResult<QueryResult>;
    /// The ledger rows with a key, retracted capsules left out (key, meaning, time,
    /// realized): the engine's when the authority runs one, else the document's.
    fn ledger_now(&self) -> Vec<(String, MeaningKind, i64, bool)>;
}
