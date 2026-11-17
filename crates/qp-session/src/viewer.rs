//! The viewer (RP3/M15, K48; `docs/PROTOCOL.md` §22): the device is a viewer, not a
//! compute site.
//!
//! - **Authority side** ([`Session`]): a `viewer` subscription and the `visible` message.
//!   For every viewer connection the authority keeps what the device has been sent and,
//!   after every input (message, data step), pushes only what changed: the reader's feed
//!   (its audience, at most `limit`), the **ready faces** of the capsules on screen (the
//!   scene the authority compiled for that reader: face design, look, language, data),
//!   the open capsule's **slice** (the document's items about its instrument) and the
//!   claims resting on its feeds. It names the device's **data subscriptions**: none on
//!   the feed screen, the open instrument's feeds while a capsule is open. A host serves
//!   a viewer's data plane from these ([`Session::viewer_data`]).
//! - **Device side** ([`OpenSide`]): the open capsule only — a [`Replica`] started from
//!   the slice, its own data of that instrument, the display rule (§19.4) over the claims
//!   of that instrument, the candidate and a check of the capsule's story on its own
//!   bars ([`check_story`]). The device runs no capsule engine for the feed: its work per
//!   tick and its memory do not depend on how many instruments the universe has.
//! - **Scenes** ([`face_scene`], [`open_scene`]): the pure scene functions both sides
//!   compile with (moved here from the web shell's core so that the authority compiles a
//!   face exactly as a replica would: same provider, same reader sentence, same headline
//!   pieces).

use std::collections::{BTreeMap, BTreeSet};

use qp_grammar::{compile, depth, CompileOptions, DataProvider, SeriesKind};
use qp_schema::capsule::GlyphSpec;
use qp_schema::data::{DataEvent, FeedKey};
use qp_schema::protocol::{
    CapsuleItem, ErrorCode, Event, Language, OpenDepth, Opening, OutboundLine, Query, QueryResult,
    ServerMsg, TrustMark, ViewState, ViewerFace, ViewerItem, ViewerOpen, ViewerUpdate,
};
use qp_schema::scene::SceneDesc;
use qp_schema::{Bar, Capsule, Document, Fact, Finding, Point, Series, SeriesSource, View};
use serde::Serialize;
use serde_json::Value;

use crate::error::E;
use crate::replica::{document_digest, fnv1a64, Replica, Shown};
use crate::session::{out, Session};

/// Feed length of a `viewer` subscription without `limit`.
pub const DEFAULT_LIMIT: u32 = 40;
/// Largest `limit`.
pub const MAX_LIMIT: u32 = 200;
/// Face frame width (CSS px) of a `viewer` subscription without `width` (the phone frame).
pub const DEFAULT_FACE_WIDTH: u32 = 390;

// ---------------------------------------------------------------------------
// Scenes (shared by the authority and every device)
// ---------------------------------------------------------------------------

/// FNV-1a 64 of a scene's JSON bytes (`view_digest`, §19.6).
#[must_use]
pub fn scene_digest(scene: &SceneDesc) -> String {
    let bytes = serde_json::to_vec(scene).unwrap_or_default();
    format!("{:016x}", fnv1a64(&bytes))
}

/// A data provider that speaks with the reader's sentences: a session's provider
/// (document, bars, values) with the headline of each finding replaced by the sentence
/// the authority wrote for this reader (the `capsules` answer's `sentence.text`: its
/// language and entitlement). (R4/H-FP) On a face, it also gives the headline pieces of
/// that capsule for the same reader (`Session::item_headline_parts`).
struct Reader<'a> {
    inner: crate::DocProvider<'a>,
    heads: &'a BTreeMap<String, String>,
    parts: Option<(&'a str, qp_grammar::HeadlineParts)>,
}

impl DataProvider for Reader<'_> {
    fn series_kind(&self, s: &str) -> Option<SeriesKind> {
        self.inner.series_kind(s)
    }
    fn bars(&self, s: &str) -> Option<std::borrow::Cow<'_, [Bar]>> {
        self.inner.bars(s)
    }
    fn input_bars(&self, s: &str) -> Option<String> {
        self.inner.input_bars(s)
    }
    fn fields(&self, s: &str) -> Vec<String> {
        self.inner.fields(s)
    }
    fn main_field(&self, s: &str) -> Option<String> {
        self.inner.main_field(s)
    }
    fn plot_fields(&self, s: &str) -> Vec<String> {
        self.inner.plot_fields(s)
    }
    fn values(&self, s: &str, f: &str) -> Option<std::borrow::Cow<'_, [Option<f64>]>> {
        self.inner.values(s, f)
    }
    fn points(&self, s: &str) -> Option<std::borrow::Cow<'_, [Point]>> {
        self.inner.points(s)
    }
    fn label(&self, s: &str) -> Option<String> {
        self.inner.label(s)
    }
    fn symbol(&self, s: &str) -> Option<String> {
        self.inner.symbol(s)
    }
    fn interval(&self, s: &str) -> Option<String> {
        self.inner.interval(s)
    }
    fn fact(&self, id: &str) -> Option<std::borrow::Cow<'_, Fact>> {
        self.inner.fact(id)
    }
    fn finding(&self, id: &str) -> Option<std::borrow::Cow<'_, Finding>> {
        self.inner.finding(id)
    }
    fn headline(&self, finding: &str) -> Option<String> {
        self.heads
            .get(finding)
            .cloned()
            .or_else(|| self.inner.headline(finding))
    }
    fn threshold(&self, name: &str) -> Option<f64> {
        self.inner.threshold(name)
    }
    fn capsule(&self, c: &str) -> Option<std::borrow::Cow<'_, Capsule>> {
        self.inner.capsule(c)
    }
    fn glyph(&self, c: &str) -> Option<std::borrow::Cow<'_, GlyphSpec>> {
        self.inner.glyph(c)
    }
    fn headline_parts(&self, finding: &str) -> Option<qp_grammar::HeadlineParts> {
        match &self.parts {
            Some((f, hp)) if *f == finding => Some(hp.clone()),
            _ => self.inner.headline_parts(finding),
        }
    }
}

/// The language of the reader's sentence in `item`: the authority wrote `sentence.text` in
/// the reader's language (`en` when it is the English sentence and not the Turkish one).
fn item_lang(item: &CapsuleItem) -> Language {
    if item.sentence.text == item.sentence.en && item.sentence.text != item.sentence.tr {
        Language::En
    } else {
        Language::Tr
    }
}

fn options(look: &ViewState, w: u32, h: u32, range: Option<(u64, u64)>) -> CompileOptions {
    let mut o = CompileOptions::new(w, h);
    o.range = range;
    o.theme.clone_from(&look.theme);
    o.alphabet.clone_from(&look.alphabet);
    o.outcome.clone_from(&look.outcome);
    o.look.clone_from(&look.look);
    o
}

/// (R5/T3) The connection's look (`ViewState.look`; absent or unknown: `r4`).
#[must_use]
pub fn look_of(look: &ViewState) -> &'static qp_grammar::look::Look {
    qp_grammar::look::or_default(look.look.as_deref())
}

/// The scene of a capsule's face in the feed (open depth `sentence`): the connection's
/// face design (`look.face`, `qp_grammar::depth::face_view`) over the session's data,
/// at `w` × `h`, with the reader's sentence. `None` when the reader gets no face scene
/// (no data for the finding's series: the shell then shows the sentence as text).
#[must_use]
pub fn face_scene(
    s: &mut Session,
    item: &CapsuleItem,
    look: &ViewState,
    max_points: u32,
    w: u32,
    h: u32,
) -> Option<SceneDesc> {
    let heads = BTreeMap::from([(item.finding.clone(), item.sentence.text.clone())]);
    // (R4/H-FP) The headline pieces only for a design with a headline template.
    let parts = qp_grammar::faces::headline_template(look.face.as_deref())
        .ok()
        .flatten()
        .and_then(|_| s.item_headline_parts(item, item_lang(item)))
        .map(|hp| (item.finding.as_str(), hp));
    let p = Reader {
        inner: s.provider(),
        heads: &heads,
        parts,
    };
    let id = format!("capsule-{}", item.finding);
    let view = depth::face_view_in(
        look_of(look),
        look.face.as_deref(),
        &item.finding,
        max_points,
        &p,
        &id,
    )
    .ok()?;
    let mut o = options(look, w, h, None);
    o.face.clone_from(&look.face);
    compile(&view, &p, &o).ok()
}

/// The open capsule's view at its depth, rebuilt over the session's current document and
/// data by the rule the authority's `opening.presentation` used (the card at `capsule`;
/// at `chart`/`evidence` the capsule's stored view, else the presenter's view with the id
/// the authority gave it).
fn open_view(
    p: &dyn DataProvider,
    stored: Option<View>,
    o: &Opening,
    look: &ViewState,
    max_points: u32,
) -> Option<View> {
    match o.to {
        OpenDepth::Sentence => None,
        OpenDepth::Capsule => {
            let id = o.presentation.capsule_view.as_ref()?.id.clone();
            depth::capsule_view_in(look_of(look), &o.finding, max_points, p, &id).ok()
        }
        OpenDepth::Chart | OpenDepth::Evidence => {
            let given = o.presentation.view.as_ref()?;
            if stored.is_some() {
                return stored;
            }
            let id = look.view.clone().unwrap_or_else(|| given.id.clone());
            depth::chart_view(&o.finding, p, &id).ok()
        }
    }
}

/// The scene of the open capsule (the opening's depth and frame, the view state's chart
/// range and style) over the session's own data; `None` when nothing is open or the
/// reader gets no scene at that depth (no data: the authority sent no view).
#[must_use]
pub fn open_scene(
    s: &mut Session,
    o: &Opening,
    headline: Option<&str>,
    look: &ViewState,
    max_points: u32,
) -> Option<SceneDesc> {
    if o.to == OpenDepth::Sentence {
        return None;
    }
    let [w, h] = o.frame?;
    let heads: BTreeMap<String, String> = headline
        .map(|t| (o.finding.clone(), t.to_owned()))
        .into_iter()
        .collect();
    let stored = o
        .presentation
        .view
        .as_ref()
        .and_then(|v| s.document().views.iter().find(|x| x.id == v.id).cloned());
    let p = Reader {
        inner: s.provider(),
        heads: &heads,
        parts: None,
    };
    let view = open_view(&p, stored, o, look, max_points)?;
    let range = look
        .range
        .as_ref()
        .filter(|r| o.to >= OpenDepth::Chart && r.view == view.id)
        .map(|r| (r.from_bar, r.to_bar));
    compile(&view, &p, &options(look, w, h, range)).ok()
}

// ---------------------------------------------------------------------------
// The slice
// ---------------------------------------------------------------------------

/// Every value of a `"series"` key anywhere in `v`.
fn series_refs(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                if k == "series" {
                    if let Some(s) = x.as_str() {
                        out.insert(s.to_owned());
                    }
                }
                series_refs(x, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                series_refs(x, out);
            }
        }
        _ => {}
    }
}

/// Whether every series `x` names is in `ids` (and it names at least one).
fn rests_on(x: &impl Serialize, ids: &BTreeSet<String>) -> bool {
    let mut refs = BTreeSet::new();
    series_refs(&serde_json::to_value(x).unwrap_or(Value::Null), &mut refs);
    !refs.is_empty() && refs.iter().all(|r| ids.contains(r))
}

/// The series of `doc` about `instrument`: those with that instrument, the studies over
/// them and the inputs of those studies, to a fixed point (document order).
fn slice_series(doc: &Document, instrument: &str) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = doc
        .series
        .iter()
        .filter(|s| {
            s.instrument
                .as_ref()
                .is_some_and(|i| i.symbol == instrument)
        })
        .map(|s| s.id.clone())
        .collect();
    loop {
        let n = ids.len();
        for s in &doc.series {
            if let SeriesSource::Study { inputs, .. } = &s.source {
                if inputs.iter().any(|i| ids.contains(i)) || ids.contains(&s.id) {
                    ids.insert(s.id.clone());
                    ids.extend(inputs.iter().cloned());
                }
            }
        }
        if ids.len() == n {
            return ids;
        }
    }
}

/// The open capsule's slice of `doc` for `principal` (§22.3): the document's own fields,
/// the series about `instrument` (with the studies over them and their inputs), their
/// facts, the findings resting on those facts and their capsules (and the findings of
/// those capsules), the views and intents resting only on those series, the ledger rows of
/// those capsules and findings, and the reader's own audience. Document order is kept, so
/// a scene compiled over the slice is the one compiled over the whole document. Also the
/// bound feeds the slice rests on.
#[must_use]
pub fn slice(doc: &Document, principal: &str, instrument: &str) -> (Document, Vec<FeedKey>) {
    let ids = slice_series(doc, instrument);
    let series: Vec<Series> = doc
        .series
        .iter()
        .filter(|s| ids.contains(&s.id))
        .cloned()
        .collect();
    let facts: Vec<Fact> = doc
        .facts
        .iter()
        .filter(|f| ids.contains(&f.series))
        .cloned()
        .collect();
    let fact_ids: BTreeSet<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    let mut finding_ids: BTreeSet<String> = doc
        .findings
        .iter()
        .filter(|f| f.facts.iter().any(|x| fact_ids.contains(x.as_str())))
        .map(|f| f.id.clone())
        .collect();
    let capsules: Vec<Capsule> = doc
        .capsules
        .iter()
        .filter(|c| {
            finding_ids.contains(&c.finding)
                || c.story.as_ref().is_some_and(|s| s.instrument == instrument)
        })
        .cloned()
        .collect();
    finding_ids.extend(capsules.iter().map(|c| c.finding.clone()));
    let capsule_ids: BTreeSet<&str> = capsules.iter().map(|c| c.id.as_str()).collect();
    let stored_views: BTreeSet<&str> = capsules.iter().filter_map(|c| c.view.as_deref()).collect();
    let views: Vec<View> = doc
        .views
        .iter()
        .filter(|v| stored_views.contains(v.id.as_str()) || rests_on(v, &ids))
        .cloned()
        .collect();
    let mut ledger = doc.ledger.clone();
    ledger.entries.retain(|e| {
        finding_ids.contains(&e.finding)
            || e.capsule
                .as_deref()
                .is_some_and(|c| capsule_ids.contains(c))
    });
    let mut feeds: Vec<FeedKey> = series
        .iter()
        .filter_map(Series::binding)
        .map(|b| b.feed_key())
        .collect();
    feeds.sort();
    feeds.dedup();
    let out = Document {
        schema_version: doc.schema_version.clone(),
        id: doc.id.clone(),
        title: doc.title.clone(),
        display_tz: doc.display_tz.clone(),
        series,
        views,
        findings: doc
            .findings
            .iter()
            .filter(|f| finding_ids.contains(&f.id))
            .cloned()
            .collect(),
        facts,
        capsules,
        intents: doc
            .intents
            .iter()
            .filter(|i| i.trigger.as_ref().is_some_and(|t| rests_on(t, &ids)))
            .cloned()
            .collect(),
        ledger,
        narratives: Vec::new(),
        proposals: Vec::new(),
        audiences: doc
            .audiences
            .iter()
            .filter(|(p, _)| p.as_str() == principal)
            .map(|(p, a)| (p.clone(), a.clone()))
            .collect(),
    };
    (out, feeds)
}

// ---------------------------------------------------------------------------
// The device's check of the open capsule
// ---------------------------------------------------------------------------

/// The device's check of the open capsule's story on its own data (§22.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryCheck {
    /// Its own data does not cover the story's facts yet (or not with closed bars).
    Pending,
    /// Every checkable fact of the story comes out of the public detectors on its own
    /// bars with the same evidence.
    Held,
    /// A fact does not come out (or with other evidence) on its own closed bars.
    Differs,
    /// The story rests only on facts the device cannot check (server facts, K33) or it
    /// has no data of the series (no entitlement): the authority's word stands.
    Server,
}

/// Checks capsule `capsule`'s story in `doc` (a slice) on `bars` (the device's own bars
/// of the story's bar series, from the document's anchor): the public detectors run over
/// the closed bars (all but the last) and each of the story's facts that is not a server
/// fact must come out with the same id (instrument, detector, code, bar) and evidence.
#[must_use]
pub fn check_story(doc: &Document, capsule: &str, bars: Option<&[Bar]>) -> StoryCheck {
    let Some(story) = doc
        .capsules
        .iter()
        .find(|c| c.id == capsule)
        .and_then(|c| c.story.as_ref())
    else {
        return StoryCheck::Server;
    };
    let want: Vec<&Fact> = crate::capsule::story_fact_ids(story)
        .iter()
        .filter_map(|id| doc.facts.iter().find(|f| &f.id == id))
        .filter(|f| !f.meaning.as_ref().is_some_and(|m| m.server))
        .collect();
    if want.is_empty() {
        return StoryCheck::Server;
    }
    let Some(bars) = bars.filter(|b| !b.is_empty()) else {
        return StoryCheck::Server;
    };
    // Only closed bars: the last own bar may still move.
    let closed = bars.len() - 1;
    let latest = want.iter().map(|f| f.t).max().unwrap_or(i64::MIN);
    let Some(upto) = bars[..closed].iter().position(|b| b.t == latest) else {
        return StoryCheck::Pending;
    };
    let mut dets = qp_facts::default_detectors();
    let mut found: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for (i, b) in bars[..=upto].iter().enumerate() {
        for det in &mut dets {
            for d in det.push(b, None) {
                let f = qp_facts::to_fact(
                    det.as_ref(),
                    &d,
                    &story.instrument,
                    &story.series,
                    i as u64,
                    b.t,
                    qp_capsule::pipeline::ENGINE_VERSION,
                );
                found.insert(f.id, f.evidence);
            }
        }
    }
    if want
        .iter()
        .all(|f| found.get(&f.id).is_some_and(|e| *e == f.evidence))
    {
        StoryCheck::Held
    } else {
        StoryCheck::Differs
    }
}

/// The trust state the device shows for the open capsule (its mark's text comes from
/// [`trust_text`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustState {
    /// The authority's settled claim ("verified by the server").
    Server,
    /// Its own data does not cover it yet ("checking on this device").
    Checking,
    /// Checked on its own data ("checked on this device").
    Device,
    /// Its own data contradicts it ("sources differ").
    Differs,
}

impl TrustState {
    /// The protocol's mark for it (`device` only once checked).
    #[must_use]
    pub fn mark(self) -> TrustMark {
        match self {
            TrustState::Device => TrustMark::Device,
            _ => TrustMark::Server,
        }
    }
}

/// The trust mark's text in the reader's language (the shell shows it as it is).
#[must_use]
pub fn trust_text(t: TrustState, lang: Language) -> &'static str {
    match (t, lang) {
        (TrustState::Server, Language::Tr) => "Sunucu doğruladı",
        (TrustState::Server, Language::En) => "Verified by the server",
        (TrustState::Checking, Language::Tr) => "Bu cihazda sınanıyor",
        (TrustState::Checking, Language::En) => "Checking on this device",
        (TrustState::Device, Language::Tr) => "Bu cihazda sınandı",
        (TrustState::Device, Language::En) => "Checked on this device",
        (TrustState::Differs, Language::Tr) => "Kaynaklar farklı",
        (TrustState::Differs, Language::En) => "Sources differ",
    }
}

/// The device's side of the open capsule (§22.4): a replica of the slice over its own data
/// of the open instrument only.
pub struct OpenSide {
    /// The slice as the authority sent it.
    pub open: ViewerOpen,
    replica: Replica,
    check: StoryCheck,
    /// The last check's data version (re-checked only when its data changed).
    checked_at: Option<u64>,
    data_version: u64,
}

impl std::fmt::Debug for OpenSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenSide")
            .field("capsule", &self.open.capsule)
            .field("check", &self.check)
            .finish_non_exhaustive()
    }
}

impl OpenSide {
    /// The open side of `open` at log position `seq`, its replica's own data from
    /// `provider` (the open instrument's feeds the host served).
    #[must_use]
    pub fn new(
        principal: &str,
        open: ViewerOpen,
        seq: u64,
        provider: qp_source::MemoryProvider,
        calendar: Option<qp_schema::Calendar>,
    ) -> Self {
        let mut replica = Replica::with_memory(principal, provider);
        replica.set_headline_calendar(calendar);
        replica.start((*open.document).clone(), seq);
        Self {
            open,
            replica,
            check: StoryCheck::Pending,
            checked_at: None,
            data_version: 0,
        }
    }

    /// A new slice of the same capsule (the authority's document changed): the replica
    /// reloads it and keeps its data and the claims it knows.
    pub fn update(&mut self, open: ViewerOpen, seq: u64) {
        self.replica.start((*open.document).clone(), seq);
        self.open = open;
        self.checked_at = None;
    }

    /// A data event of the open instrument's feed (its own data).
    pub fn data_event(&mut self, ev: &DataEvent) {
        let _ = self.replica.data_event(ev);
        self.data_version += 1;
    }

    /// A message of the authority for the open capsule (its claims).
    pub fn on_message(&mut self, msg: &ServerMsg) {
        if let ServerMsg::Notify {
            event: Event::Claim { .. },
            ..
        } = msg
        {
            self.replica.on_message(msg);
            self.checked_at = None;
        }
    }

    /// The replica (its session compiles the open scene; its display holds the claims).
    pub fn replica_mut(&mut self) -> &mut Replica {
        &mut self.replica
    }

    /// The replica (read access).
    #[must_use]
    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// Runs the check if its data or claims changed since the last one; returns the
    /// story check.
    pub fn check(&mut self) -> &StoryCheck {
        if self.checked_at != Some(self.data_version) {
            let bars = self.replica.bars(&self.open.series).map(<[Bar]>::to_vec);
            self.check = check_story(&self.open.document, &self.open.capsule, bars.as_deref());
            self.checked_at = Some(self.data_version);
        }
        &self.check
    }

    /// The trust state: the story check, overruled by the display rule over the claims on
    /// this capsule (a disputed claim: sources differ).
    pub fn trust(&mut self) -> TrustState {
        let cap = self.open.capsule.clone();
        let disputed = self.replica.display().iter().any(|d| {
            d.state == Shown::Disputed
                && matches!(&d.claim, qp_schema::protocol::ClaimRef::Capsule { capsule, .. } if *capsule == cap)
        });
        if disputed {
            return TrustState::Differs;
        }
        match self.check() {
            StoryCheck::Held => TrustState::Device,
            StoryCheck::Pending => TrustState::Checking,
            StoryCheck::Differs => TrustState::Differs,
            StoryCheck::Server => TrustState::Server,
        }
    }

    /// Capsule candidates on the open bar of the open series, from its own data (K28).
    #[must_use]
    pub fn candidates(&self) -> Vec<qp_capsule::CandidateStory> {
        self.replica.candidates(&self.open.series)
    }
}

/// A memory provider over the feeds a host served for the open instrument: their initial
/// bars and the data events since (§22.5 `bind`).
///
/// # Errors
/// An invalid feed description.
pub fn open_provider(
    principal: &str,
    feeds: &[qp_schema::protocol::InitFeed],
    events: &[DataEvent],
) -> Result<qp_source::MemoryProvider, String> {
    let init = qp_schema::protocol::InitData {
        feeds: feeds.to_vec(),
        entitlements: BTreeMap::from([(
            principal.to_owned(),
            feeds.iter().map(|f| f.feed.symbol.clone()).collect(),
        )]),
        pending: Vec::new(),
        settled: None,
    };
    let mut p = qp_source::MemoryProvider::from_init(&init, std::path::Path::new("."))
        .map_err(|e| e.to_string())?;
    for ev in events {
        let _ = p.apply(ev);
    }
    Ok(p)
}

// ---------------------------------------------------------------------------
// The authority's side
// ---------------------------------------------------------------------------

/// What the authority keeps for one viewer connection.
#[derive(Debug, Clone, Default)]
pub(crate) struct ViewerConn {
    /// The `viewer` subscription id.
    pub sub: String,
    pub limit: u32,
    pub width: u32,
    /// Capsules on screen.
    pub visible: Vec<String>,
    /// Digest of the feed last sent.
    feed_sent: Option<u64>,
    /// Face digests last sent, by capsule.
    faces_sent: BTreeMap<String, String>,
    /// The open capsule and the digest of its slice last sent.
    open_sent: Option<(String, String)>,
    /// Digest of the audience last sent.
    audience_sent: Option<u64>,
    /// The data subscriptions last named.
    pub data: Vec<FeedKey>,
}

fn digest_of(x: &impl Serialize) -> u64 {
    fnv1a64(&serde_json::to_vec(x).unwrap_or_default())
}

impl Session {
    /// Whether any connection holds a `viewer` subscription.
    #[must_use]
    pub fn has_viewers(&self) -> bool {
        !self.viewers.is_empty()
    }

    /// (§22) The data subscriptions of viewer connection `conn`: the feeds its host serves
    /// it (`None`: not a viewer — such a connection gets the whole data plane).
    #[must_use]
    pub fn viewer_data(&self, conn: &str) -> Option<&[FeedKey]> {
        self.viewers.get(conn).map(|v| v.data.as_slice())
    }

    /// (§22) Viewer updates are computed by the host after each batch of data events
    /// (`true`: a replay host, once per step) instead of after each event.
    pub fn set_viewer_batch(&mut self, on: bool) {
        self.viewer_batch = on;
    }

    /// The `viewer` subscription of connection `ci` was made (`Session::subscribe`): checks
    /// it is the only one and its fields.
    pub(crate) fn viewer_check(
        &self,
        ci: usize,
        limit: Option<u32>,
        width: Option<u32>,
    ) -> crate::error::PResult<()> {
        if self.viewers.contains_key(&self.conns[ci].id) {
            return Err(E::new(
                ErrorCode::Duplicate,
                "a connection holds one `viewer` subscription",
            )
            .at("/topic/kind")
            .hint("Send `visible` to change what it shows.")
            .build());
        }
        if limit.is_some_and(|l| l == 0 || l > MAX_LIMIT) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("`limit` must be 1..={MAX_LIMIT}"),
            )
            .at("/topic/limit")
            .expected([format!("1..={MAX_LIMIT}")])
            .build());
        }
        if width.is_some_and(|w| !(64..=4096).contains(&w)) {
            return Err(E::new(ErrorCode::Invalid, "`width` must be 64..=4096")
                .at("/topic/width")
                .expected(["64..=4096"])
                .build());
        }
        Ok(())
    }

    /// The `viewer` subscription `sub` of connection `ci` is in place: remembers it and
    /// returns its first event (the feed).
    pub(crate) fn viewer_start(
        &mut self,
        ci: usize,
        sub: &str,
        limit: Option<u32>,
        width: Option<u32>,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        self.viewers.insert(
            conn,
            ViewerConn {
                sub: sub.to_owned(),
                limit: limit.unwrap_or(DEFAULT_LIMIT),
                width: width.unwrap_or(DEFAULT_FACE_WIDTH),
                ..ViewerConn::default()
            },
        );
        self.viewer_lines(ci, true)
    }

    /// A `visible` message of connection `ci`.
    pub(crate) fn visible_msg(
        &mut self,
        ci: usize,
        id: String,
        sub: &str,
        visible: Vec<String>,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let Some(v) = self.viewers.get_mut(&conn).filter(|v| v.sub == sub) else {
            let e = E::new(
                ErrorCode::NotFound,
                format!("no `viewer` subscription `{sub}`"),
            )
            .at("/sub")
            .hint("Subscribe to {\"kind\":\"viewer\"} first.")
            .build();
            return vec![crate::session::err_line(&conn, Some(id), e)];
        };
        let mut vis = visible;
        vis.dedup();
        vis.truncate(v.limit as usize);
        v.visible = vis;
        let principal = self.conns[ci].principal.clone();
        let update = self.viewer_update(ci, false);
        vec![out(
            &conn,
            ServerMsg::Result {
                id,
                seq: self.seq_to(&principal, self.seq()),
                now: self.clock(),
                data: QueryResult::Viewer {
                    viewer: Box::new(update),
                },
                resolved: BTreeMap::new(),
            },
        )]
    }

    /// The viewer subscription was removed.
    pub(crate) fn viewer_unsubscribed(&mut self, conn: &str, sub: &str) {
        if self.viewers.get(conn).is_some_and(|v| v.sub == sub) {
            self.viewers.remove(conn);
        }
    }

    /// (§22) What changed for every viewer connection since it was last told, as `viewer`
    /// events (none when nothing changed). A host calls it after a batch of data events
    /// ([`Session::set_viewer_batch`]); the authority itself after every message.
    pub fn viewer_flush(&mut self) -> Vec<OutboundLine> {
        if self.viewers.is_empty() {
            return Vec::new();
        }
        let cis: Vec<usize> = (0..self.conns.len())
            .filter(|&ci| self.viewers.contains_key(&self.conns[ci].id))
            .collect();
        let mut lines = Vec::new();
        for ci in cis {
            lines.extend(self.viewer_lines(ci, false));
        }
        lines
    }

    /// The `viewer` event of connection `ci` (always one when `force`).
    fn viewer_lines(&mut self, ci: usize, force: bool) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let principal = self.conns[ci].principal.clone();
        let before = self.viewers.get(&conn).map(|v| v.data.clone());
        let update = self.viewer_update(ci, force);
        if !force && update.is_empty() && before.as_ref() == Some(&update.data) {
            return Vec::new();
        }
        let Some(sub) = self.viewers.get(&conn).map(|v| v.sub.clone()) else {
            return Vec::new();
        };
        vec![out(
            &conn,
            ServerMsg::Notify {
                sub,
                event: Event::Viewer {
                    viewer: Box::new(update),
                    seq: self.seq_to(&principal, self.seq()),
                },
            },
        )]
    }

    /// Computes what changed for viewer connection `ci` (in the reader's world) and records
    /// it as sent. `all`: the whole feed even if unchanged.
    fn viewer_update(&mut self, ci: usize, all: bool) -> ViewerUpdate {
        let conn = self.conns[ci].id.clone();
        let Some(vc) = self.viewers.get(&conn).cloned() else {
            return ViewerUpdate::default();
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let look = self.conns[ci].look.clone();
        let mp = self.conns[ci].profile.max_points;
        let open_cap = (look.depth != OpenDepth::Sentence)
            .then(|| look.capsule.clone())
            .flatten();
        let (update, vc) = self.as_reader(&principal, lang, |s| {
            s.viewer_compute(ci, vc, all, &look, mp, open_cap.as_deref())
        });
        self.viewers.insert(conn, vc);
        update
    }

    /// [`Session::viewer_update`] inside the reader's world.
    fn viewer_compute(
        &mut self,
        ci: usize,
        mut vc: ViewerConn,
        all: bool,
        look: &ViewState,
        mp: u32,
        open_cap: Option<&str>,
    ) -> (ViewerUpdate, ViewerConn) {
        let mut up = ViewerUpdate::default();
        let principal = self.conns[ci].principal.clone();
        // The feed: the reader's audience, as the `capsules` query lists it.
        let q = Query::Capsules {
            instrument: None,
            stages: Vec::new(),
            open: None,
            meaning: None,
            since_bar: None,
            since_t: None,
            born_since_bar: None,
            born_since_t: None,
            latest: false,
            audience: true,
            limit: Some(vc.limit),
        };
        let caps = match self.query(ci, &q) {
            Ok(QueryResult::Capsules { capsules, .. }) => capsules,
            _ => Vec::new(),
        };
        let items: Vec<ViewerItem> = caps
            .into_iter()
            .map(|it| {
                let (face, known_at) = self.viewer_row(&principal, &it);
                ViewerItem {
                    capsule: it,
                    known_at,
                    face,
                }
            })
            .collect();
        let fd = digest_of(&items);
        if all || vc.feed_sent != Some(fd) {
            vc.feed_sent = Some(fd);
            up.feed = Some(items.clone());
        }
        // The faces on screen.
        let fh = qp_grammar::faces::height_in(look_of(look), look.face.as_deref());
        let mut sent: BTreeMap<String, String> = BTreeMap::new();
        for id in &vc.visible {
            let Some(row) = items.iter().find(|r| &r.capsule.id == id && r.face) else {
                continue;
            };
            let Some(scene) = face_scene(self, &row.capsule, look, mp, vc.width, fh) else {
                continue;
            };
            let d = scene_digest(&scene);
            if vc.faces_sent.get(id) != Some(&d) {
                up.faces.push(ViewerFace {
                    capsule: id.clone(),
                    digest: d.clone(),
                    scene: Box::new(scene),
                    trust: TrustMark::Server,
                });
            }
            sent.insert(id.clone(), d);
        }
        vc.faces_sent = sent;
        // The reader's audience.
        let aud = self.doc.audiences.get(&principal).cloned();
        let ad = digest_of(&aud);
        if vc.audience_sent != Some(ad) {
            vc.audience_sent = Some(ad);
            up.audience = aud;
        }
        // The open capsule's slice and data subscriptions.
        let open = open_cap.and_then(|cap| {
            let c = self.doc.capsules.iter().find(|c| c.id == cap)?;
            let story = c.story.as_ref();
            let series = story.map(|s| s.series.clone()).or_else(|| {
                let f = self.doc.findings.iter().find(|f| f.id == c.finding)?;
                let fact = self.doc.facts.iter().find(|x| f.facts.contains(&x.id))?;
                Some(fact.series.clone())
            })?;
            let instrument = story.map(|s| s.instrument.clone()).or_else(|| {
                self.doc
                    .series
                    .iter()
                    .find(|s| s.id == series)
                    .and_then(|s| s.instrument.as_ref().map(|i| i.symbol.clone()))
            })?;
            let (doc, feeds) = slice(&self.doc, &principal, &instrument);
            let feeds: Vec<FeedKey> = feeds
                .into_iter()
                .filter(|k| self.plane.entitled(&principal, k))
                .collect();
            Some(ViewerOpen {
                capsule: cap.to_owned(),
                finding: c.finding.clone(),
                instrument,
                series,
                digest: document_digest(&doc),
                document: Box::new(doc),
                feeds,
            })
        });
        match open {
            Some(o) => {
                let key = (o.capsule.clone(), o.digest.clone());
                vc.data.clone_from(&o.feeds);
                if vc.open_sent.as_ref() != Some(&key) {
                    vc.open_sent = Some(key);
                    up.open = Some(o);
                }
            }
            None => {
                vc.data.clear();
                if vc.open_sent.take().is_some() {
                    up.closed = true;
                }
            }
        }
        up.data.clone_from(&vc.data);
        (up, vc)
    }

    /// Whether the reader gets a face scene for `it` (entitled to every feed its series
    /// rests on, with bars) and when it could first know its latest transition.
    fn viewer_row(&self, principal: &str, it: &CapsuleItem) -> (bool, Option<i64>) {
        let rd = rd!(self);
        let bars = crate::values::find_series(rd, &it.series).and_then(Series::bars);
        let entitled = crate::claims::feeds_of(&self.doc, &[it.series.as_str()])
            .is_some_and(|ks| ks.iter().all(|k| self.plane.entitled(principal, k)));
        let face = entitled && bars.is_some_and(|b| !b.is_empty());
        let known = if entitled {
            usize::try_from(it.last_bar)
                .ok()
                .and_then(|b| b.checked_add(1))
                .and_then(|n| bars.and_then(|b| b.get(n)))
                .map(|b| b.t)
        } else {
            None
        };
        (face, known)
    }

    /// (§22) The viewer subscription of connection `conn` whose open capsule rests on
    /// every feed in `feeds` (a claim about the open instrument goes there too).
    pub(crate) fn viewer_claim_sub(&self, conn: &str, feeds: &[FeedKey]) -> Option<&str> {
        let v = self.viewers.get(conn)?;
        (!feeds.is_empty() && !v.data.is_empty() && feeds.iter().all(|k| v.data.contains(k)))
            .then_some(v.sub.as_str())
    }
}
