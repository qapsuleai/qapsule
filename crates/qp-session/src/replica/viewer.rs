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

mod open;
mod updates;

pub use open::*;

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
    /// (R5/m17-int) The voice surfaces the authority wrote for this reader (the item's
    /// `voice`), for that finding.
    voice: Option<(&'a str, &'a qp_schema::protocol::CapsuleVoice)>,
    /// (R5/M20) The reader's language and the device's check of the open capsule (an
    /// open capsule design's labels and trust row).
    extra: OpenExtra,
}

/// (R5/M20) What the shell core adds to the open capsule's scene: the reader's language
/// and the device's own check of it (the viewer, K48). Default: neither (the authority's
/// view of it: "verified by the server").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenExtra {
    /// The reader's language.
    pub lang: Option<qp_schema::protocol::Language>,
    /// The device's check of the open capsule.
    pub check: Option<TrustState>,
}

impl TrustState {
    /// The grammar's form of it (an open capsule design's trust row).
    #[must_use]
    pub fn device_check(self) -> qp_grammar::designs::DeviceCheck {
        match self {
            TrustState::Server => qp_grammar::designs::DeviceCheck::Server,
            TrustState::Checking => qp_grammar::designs::DeviceCheck::Checking,
            TrustState::Device => qp_grammar::designs::DeviceCheck::Device,
            TrustState::Differs => qp_grammar::designs::DeviceCheck::Differs,
        }
    }
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
    fn voice(&self, finding: &str) -> Option<qp_schema::protocol::CapsuleVoice> {
        match self.voice {
            Some((f, v)) if f == finding => Some(v.clone()),
            _ => None,
        }
    }
    fn reader_lang(&self) -> Option<qp_schema::protocol::Language> {
        self.extra.lang.or_else(|| self.inner.reader_lang())
    }
    fn device_check(&self, finding: &str) -> Option<qp_grammar::designs::DeviceCheck> {
        let _ = finding;
        self.extra.check.map(TrustState::device_check)
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
    let mut o = qp_designs::options(w, h);
    o.range = range;
    o.theme.clone_from(&look.theme);
    o.alphabet.clone_from(&look.alphabet);
    o.outcome.clone_from(&look.outcome);
    o.look.clone_from(&look.look);
    o
}

/// (R5/T3) The connection's look (`ViewState.look`; absent or unknown: `r4`).
#[must_use]
pub fn look_of(look: &ViewState) -> &'static qp_look::look::Look {
    qp_look::look::or_default(look.look.as_deref())
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

#[allow(clippy::too_many_lines)]
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
        comments: Vec::new(),
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
    // (RP4) The detector set of the record whose rule resolved the story (default `r3`).
    let engine = story
        .resolution
        .as_ref()
        .map_or(qp_capsule::DEFAULT_ENGINE, |r| {
            qp_capsule::engine_of_rule(&r.rule)
        });
    let mut dets = qp_facts::detectors_named(engine).unwrap_or_else(qp_facts::default_detectors);
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
    /// (R6/T3) Digest of the market status last sent.
    market_sent: Option<u64>,
}

fn digest_of(x: &impl Serialize) -> u64 {
    fnv1a64(&serde_json::to_vec(x).unwrap_or_default())
}

/// (R5/M18) The first fact of the finding of every row in `rows` (one pass over the
/// document's findings; a story chart's cluster names its stories by these facts).
#[must_use]
pub fn first_facts(
    doc: &qp_schema::Document,
    rows: &[qp_schema::protocol::StoryRow],
) -> BTreeMap<String, String> {
    let want: BTreeSet<&str> = rows.iter().map(|r| r.finding.as_str()).collect();
    doc.findings
        .iter()
        .filter(|f| want.contains(f.id.as_str()))
        .filter_map(|f| Some((f.id.clone(), f.facts.first()?.clone())))
        .collect()
}

/// (R5/m18-pavka-perf) FNV-1a as a [`std::hash::Hasher`]: a fixed (not random-seeded)
/// hash for [`FirstFacts`]' sets of ids; it only decides where an id sits, never an
/// answer.
#[derive(Default)]
struct FnvHasher(u64);

impl std::hash::Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for b in bytes {
            h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = h;
    }
}

type Fnv = std::hash::BuildHasherDefault<FnvHasher>;

/// (R5/m18-pavka-perf) [`first_facts`] looked up only when first asked, without copying
/// the document's ids and with hashed sets: [`FirstFacts::get`]`(f)` is exactly
/// `first_facts(doc, rows).get(f).cloned()` for every `f` (the same pass over the same
/// findings in the same order, the same keys, the last finding with a fact winning), but
/// a story chart asks only for its clustered stories, so a scene without a cluster never
/// makes the pass. Measured on the JIT-less phone setting (WebKit, `useJIT=false`), the
/// eager map over 1,600 findings and 533 rows took ~10 ms per scene build (string
/// comparisons in ordered sets); the hashed pass ~1.5 ms.
pub struct FirstFacts<'a> {
    doc: &'a qp_schema::Document,
    rows: &'a [qp_schema::protocol::StoryRow],
    map: std::cell::OnceCell<std::collections::HashMap<&'a str, &'a str, Fnv>>,
}

impl<'a> FirstFacts<'a> {
    /// The lookup over `doc` for the findings of `rows` (nothing is read yet).
    #[must_use]
    pub fn new(doc: &'a qp_schema::Document, rows: &'a [qp_schema::protocol::StoryRow]) -> Self {
        Self {
            doc,
            rows,
            map: std::cell::OnceCell::new(),
        }
    }

    /// The first fact of finding `f` (`None`: not a row's finding, unknown, or no fact).
    #[must_use]
    pub fn get(&self, f: &str) -> Option<String> {
        let m = self.map.get_or_init(|| {
            let want: std::collections::HashSet<&str, Fnv> =
                self.rows.iter().map(|r| r.finding.as_str()).collect();
            self.doc
                .findings
                .iter()
                .filter(|x| want.contains(x.id.as_str()))
                .filter_map(|x| Some((x.id.as_str(), x.facts.first()?.as_str())))
                .collect()
        });
        m.get(f).map(|s| (*s).to_owned())
    }
}

/// (R5/M18) The bar series whose story list the open capsule's story chart draws: the bar
/// series under its finding's first fact (`None`: unknown finding, or no bars).
#[must_use]
pub fn story_series(s: &Session, o: &Opening) -> Option<String> {
    let doc = s.document();
    let f = doc.findings.iter().find(|f| f.id == o.finding)?;
    let fid = f.facts.first()?;
    let fact = doc.facts.iter().find(|x| &x.id == fid)?;
    crate::values::bar_series(doc, &fact.series).map(|b| b.id.clone())
}

#[cfg(test)]
mod tests {
    use super::{first_facts, FirstFacts};
    use qp_schema::protocol::StoryRow;

    /// (R5/m18-pavka-perf) The lazy, hashed lookup answers what the eager map did, for
    /// every id: listed findings with facts, a listed finding without facts, duplicate
    /// ids (one later without facts, one later with other facts), a finding not listed,
    /// a listed finding the document does not have, and an id nobody has.
    #[test]
    fn first_facts_lookup_equals_the_eager_map() {
        let mut doc: qp_schema::Document = serde_json::from_str(include_str!(
            "../../../../fixtures/schema/document.example.json"
        ))
        .unwrap();
        let base = doc.findings[0].clone();
        let mk = |id: &str, facts: &[&str]| {
            let mut f = base.clone();
            f.id = id.to_owned();
            f.facts = facts.iter().map(|s| (*s).to_owned()).collect();
            f
        };
        doc.findings = vec![
            mk("a", &["fa"]),
            mk("b", &[]),
            mk("c", &["fc1", "fc2"]),
            mk("d", &["fd"]),
            mk("d", &[]),
            mk("e", &["fe1"]),
            mk("e", &["fe2"]),
            mk("unlisted", &["fu"]),
        ];
        let row: StoryRow = serde_json::from_value(serde_json::json!({
            "id": "cap", "finding": "x", "series": "px", "kind": "breakout",
            "direction": "up", "tier": 1, "born_bar": 1, "born_t": 0, "last_bar": 1,
            "last_t": 0, "stage": "born", "validity": "valid", "in_feed": true,
            "coin_flip": false,
            "face": {"event": "e", "direction_word": "d", "when": "w", "moved": "m",
                     "validity_phrase": "v", "ledger_short": "l", "few_cases": false}
        }))
        .unwrap();
        let rows: Vec<StoryRow> = ["a", "b", "c", "d", "e", "missing"]
            .iter()
            .map(|f| StoryRow {
                finding: (*f).to_owned(),
                ..row.clone()
            })
            .collect();
        let eager = first_facts(&doc, &rows);
        let lazy = FirstFacts::new(&doc, &rows);
        for id in ["a", "b", "c", "d", "e", "unlisted", "missing", "nobody", ""] {
            assert_eq!(lazy.get(id), eager.get(id).cloned(), "{id}");
        }
        assert_eq!(lazy.get("e").as_deref(), Some("fe2"));
        assert_eq!(lazy.get("d").as_deref(), Some("fd"));
    }
}
