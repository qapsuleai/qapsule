//! A reading: the split borrow one query or gesture runs on (RF1 S3.2, critique-1 B9).
//! The authority's document, reading document, data plane and headlines are read through
//! the view; the values cache and the connections' view state are lent by the call
//! context; viewports, focus and the input counter are this subsystem's own state.

use std::collections::BTreeMap;
use std::rc::Rc;

use qp_plane::plane::DataPlane;
use qp_plane::proj::Proj;
use qp_plane::values::Cache;
use qp_schema::protocol::{
    CapsuleItem, CapsuleVoice, ErrorCode, Language, LogEntry, OutboundLine, Permissions, Principal,
    Profile, Query, QueryResult, Range, Role, ServerComment, StoryRow, Surface, SurfaceKind,
    Target, ViewState, Viewport,
};
use qp_schema::{Capsule, Document, Fact, Series};
use qp_subsystem::{SessionCtx, SessionView};

use qp_capsule::sentence::StudyMemo;
use qp_plane::error::{PResult, E};

/// What a connection's readings left behind (presence, §7): its viewports, the last one
/// set by input, and when it last gave input.
#[derive(Debug, Default, Clone)]
pub(crate) struct ConnRead {
    /// View id -> visible range.
    pub viewports: BTreeMap<String, (u64, u64)>,
    /// Last viewport set by input.
    pub current: Option<Viewport>,
    /// Input counter value of the last input (0 = none).
    pub last_input: u64,
    /// Clock at the last input.
    pub last_at: Option<i64>,
}

/// The reading subsystem's state (one slot, the same in both capsule worlds).
#[derive(Debug, Default)]
pub struct ReadState {
    /// Principal -> its focus (the last tapped bar, §7).
    pub(crate) focus: BTreeMap<String, Target>,
    /// Counts inputs (the order of `last_input`).
    pub(crate) input_counter: u64,
    /// Connection id -> what its readings left behind.
    pub(crate) conns: BTreeMap<String, ConnRead>,
}

/// A connection as a reading sees it.
#[derive(Debug, Clone)]
pub(crate) struct Conn {
    pub id: String,
    pub principal: String,
    pub role: Role,
    pub permissions: Permissions,
    pub surface: Surface,
    pub profile: Profile,
    pub viewports: BTreeMap<String, (u64, u64)>,
    pub current: Option<Viewport>,
    pub last_input: u64,
    pub last_at: Option<i64>,
}

impl Conn {
    pub(crate) fn has_screen(&self) -> bool {
        matches!(
            self.surface.kind,
            SurfaceKind::Desktop | SurfaceKind::Phone | SurfaceKind::Watch
        )
    }
}

/// One reading's context: the fields the authority's reading code read on the session,
/// split so that the values cache can be warmed next to the reading document.
pub(crate) struct Rd<'a> {
    pub v: &'a dyn SessionView,
    pub doc: &'a Document,
    pub rdoc: &'a Document,
    pub plane: &'a DataPlane,
    pub headlines: &'a BTreeMap<String, String>,
    pub principals: &'a [Principal],
    pub clock: i64,
    pub proj: Option<Rc<Proj>>,
    pub conns: Vec<Conn>,
    pub looks: &'a mut Vec<ViewState>,
    pub cache: &'a mut Cache,
    pub st: &'a mut ReadState,
}

/// Runs `f` on a reading of connection `conn` (its index in the authority's order).
pub(crate) fn reading<R>(
    conn: &str,
    ctx: &mut SessionCtx<'_>,
    f: impl FnOnce(&mut Rd<'_>, usize) -> PResult<R>,
) -> PResult<R> {
    let mut own = ReadState::default();
    let st = match ctx.slots.get_mut::<ReadState>() {
        Some(st) => st,
        None => &mut own,
    };
    let mut r = Rd::new(ctx.view, ctx.cache, ctx.looks, st);
    let ci = r.conns.iter().position(|c| c.id == conn).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("unknown connection `{conn}`")).build()
    })?;
    f(&mut r, ci)
}

/// (`qp_session::Session::scene_for`) The view a reading of connection `conn` is about
/// (`view_id`, else its current view) and its visible range there.
///
/// # Errors
/// Unknown connection or view.
pub fn scene_for_range(
    ctx: &mut SessionCtx<'_>,
    conn: &str,
    view_id: Option<&str>,
) -> PResult<(String, Option<(u64, u64)>)> {
    reading(conn, ctx, |r, ci| {
        let vid = r.view_of(ci, view_id, "/view")?;
        let range = r.hit_range(&r.conns[ci], &vid);
        Ok((vid, range))
    })
}

impl<'a> Rd<'a> {
    fn new(
        v: &'a dyn SessionView,
        cache: &'a mut Cache,
        looks: &'a mut Vec<ViewState>,
        st: &'a mut ReadState,
    ) -> Self {
        let conns = v
            .conn_ids()
            .into_iter()
            .filter_map(|id| {
                let c = v.conn(id)?;
                let r = st.conns.get(id).cloned().unwrap_or_default();
                Some(Conn {
                    id: c.id.to_owned(),
                    principal: c.principal.to_owned(),
                    role: c.role,
                    permissions: c.permissions,
                    surface: c.hello.clone(),
                    profile: c.profile.clone(),
                    viewports: r.viewports,
                    current: r.current,
                    last_input: r.last_input,
                    last_at: r.last_at,
                })
            })
            .collect();
        Self {
            v,
            doc: v.document(),
            rdoc: v.reading(),
            plane: v.plane(),
            headlines: v.headlines(),
            principals: v.principals(),
            clock: v.clock(),
            proj: v.proj(),
            conns,
            looks,
            cache,
            st,
        }
    }

    /// Keeps what an input changed on connection `ci` (its presence).
    pub(crate) fn keep(&mut self, ci: usize) {
        let c = &self.conns[ci];
        self.st.conns.insert(
            c.id.clone(),
            ConnRead {
                viewports: c.viewports.clone(),
                current: c.current.clone(),
                last_input: c.last_input,
                last_at: c.last_at,
            },
        );
    }

    /// The view state of connection `c`.
    pub(crate) fn look_at(&self, c: &Conn) -> &ViewState {
        let ci = self.conns.iter().position(|x| x.id == c.id).unwrap_or(0);
        &self.looks[ci]
    }

    // The authority's functions a reading calls (through the view).

    /// The language of connection `ci`'s capsule sentences.
    pub(crate) fn lang_of(&self, ci: usize) -> Language {
        self.conns[ci].surface.lang.unwrap_or_default()
    }
    pub(crate) fn principal(&self, id: &str) -> Option<&Principal> {
        self.v.principal(id)
    }
    pub(crate) fn project_entry(&self, e: &LogEntry, p: &Proj, lang: Language) -> LogEntry {
        self.v.project_entry(e, p, lang)
    }
    pub(crate) fn viewport_events(&self, principal: &str, vp: &Viewport) -> Vec<OutboundLine> {
        self.v.viewport_events(principal, vp)
    }
    pub(crate) fn stories_query(
        &self,
        ci: usize,
        instrument: &str,
        series: Option<&str>,
        range: Option<&Range>,
        page: (Option<u32>, Option<&str>),
        path: &str,
    ) -> PResult<QueryResult> {
        self.v
            .stories_query(ci, instrument, series, range, page, path)
    }
    pub(crate) fn story_rows(
        &self,
        series: &str,
        principal: &str,
        lang: Language,
    ) -> Vec<StoryRow> {
        self.v.story_rows(series, principal, lang)
    }
    pub(crate) fn row_stamp(&self, row: &StoryRow, lang: Language) -> Option<String> {
        self.v.row_stamp(row, lang)
    }
    pub(crate) fn capsule_query(&self, ci: usize, id: &str, path: &str) -> PResult<QueryResult> {
        let look = &self.looks[ci];
        self.v
            .capsule_query(ci, id, path, (look.voice.as_deref(), look.clock))
    }
    pub(crate) fn capsule_item_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleItem> {
        self.v.capsule_item(doc, c, p, lang, principal, memo)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reader_voice(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        voice: &str,
        clock: bool,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleVoice> {
        self.v.reader_voice(doc, c, p, lang, voice, clock, memo)
    }
    pub(crate) fn market_item(&self, it: &mut CapsuleItem, lang: Language, principal: &str) {
        self.v.market_item(it, lang, principal);
    }
    pub(crate) fn capsule_passes(
        &self,
        doc: &Document,
        principal: &str,
        story: &qp_schema::capsule::Story,
        q: &Query,
    ) -> bool {
        self.v.capsule_passes(doc, principal, story, q)
    }
    pub(crate) fn coin_flip_hidden_by(
        &self,
        doc: &Document,
        principal: &str,
        story: &qp_schema::capsule::Story,
        q: &Query,
    ) -> bool {
        self.v.coin_flip_hidden_by(doc, principal, story, q)
    }
    pub(crate) fn relevant(&self, doc: &Document, principal: &str, instrument: &str) -> bool {
        self.v.relevant(doc, principal, instrument)
    }
    pub(crate) fn coin_flip_of(&self, principal: &str, story: &qp_schema::capsule::Story) -> bool {
        self.v.coin_flip_of(principal, story)
    }
    pub(crate) fn is_current(&self, it: &CapsuleItem) -> bool {
        self.v.is_current(it)
    }
    pub(crate) fn item_series(&self, doc: &'a Document, series: &str) -> Option<&'a Series> {
        // The view's document outlives the reading; the series is looked up in `doc`.
        let v: &'a dyn SessionView = self.v;
        v.item_series(doc, series)
    }
    pub(crate) fn fact_by_id(&self, id: &str) -> Option<&'a Fact> {
        let v: &'a dyn SessionView = self.v;
        v.fact_by_id(id)
    }
    pub(crate) fn ledger_now(&self) -> Vec<(String, qp_schema::capsule::MeaningKind, i64, bool)> {
        self.v.ledger_now()
    }
    pub(crate) fn comments_for(&self, principal: &str) -> Vec<ServerComment> {
        self.v.comments_for(principal)
    }
}
