//! (R3/perf, `rounds/R3/experiments/S/S-D.md` §6.3) Caches of the authority's capsule
//! work. None of them changes an output: each holds a function of document state that is
//! recomputed when that state changes (followed through the log, which every document
//! change goes through) and, where the answer also depends on something outside the
//! document (data entitlements), checked against it on every use.
//!
//! - [`IdIndex`]: positions of facts or capsules by id (first occurrence, as
//!   `iter().find` gives it).
//! - [`SentenceMemo`]: a capsule sentence pair per exact realizer input.
//! - [`ReaderProj`]: one reader's projection ([`Proj`]), kept up to date per capsule.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use qp_schema::capsule::Story;
use qp_schema::data::FeedKey;
use qp_schema::protocol::{ChangeOp, LogEntry};
use qp_schema::{Capsule, Fact};

use crate::capsule::Proj;

// ---------------------------------------------------------------------------
// Id indexes
// ---------------------------------------------------------------------------

/// Positions of a collection's items by id (and, for capsules, by finding), kept in step
/// with the log. The log entries since the last look are scanned: items put without an
/// index are appended or replaced in place (the index then only grows); anything else (a
/// delete, a put at an index, a capsule that changes its finding) rebuilds it.
#[derive(Debug, Default)]
pub(crate) struct IdIndex {
    built: bool,
    log_len: usize,
    n: usize,
    /// Address of the collection's buffer (another vector: rebuilt).
    at: usize,
    id: HashMap<String, usize>,
    /// Capsules only: finding id → first capsule with it.
    finding: HashMap<String, usize>,
}

/// What an [`IdIndex`] indexes.
pub(crate) trait Indexed {
    fn key(&self) -> &str;
    fn finding_key(&self) -> Option<&str> {
        None
    }
    /// Whether `c` may move positions or change this collection's keys (rebuild).
    fn breaks(ix: &IdIndex, c: &ChangeOp) -> bool;
}

impl Indexed for Fact {
    fn key(&self) -> &str {
        &self.id
    }
    fn breaks(_: &IdIndex, c: &ChangeOp) -> bool {
        matches!(
            c,
            ChangeOp::Delete { .. } | ChangeOp::PutFact { index: Some(_), .. }
        )
    }
}

impl Indexed for Capsule {
    fn key(&self) -> &str {
        &self.id
    }
    fn finding_key(&self) -> Option<&str> {
        Some(&self.finding)
    }
    fn breaks(ix: &IdIndex, c: &ChangeOp) -> bool {
        match c {
            ChangeOp::Delete { .. } | ChangeOp::PutCapsule { index: Some(_), .. } => true,
            // A capsule replaced in place keeps its position; its finding must stay the
            // one the index has for it.
            ChangeOp::PutCapsule {
                capsule,
                index: None,
            } => ix
                .id
                .get(&capsule.id)
                .is_some_and(|i| ix.finding.get(&capsule.finding) != Some(i)),
            _ => false,
        }
    }
}

impl IdIndex {
    /// Brings the index up to `items` after `log`.
    pub(crate) fn refresh<T: Indexed>(&mut self, log: &[LogEntry], items: &[T]) {
        let full = !self.built
            || self.at != items.as_ptr() as usize
            || log.len() < self.log_len
            || items.len() < self.n
            || log[self.log_len..]
                .iter()
                .any(|e| e.changes.iter().any(|c| T::breaks(self, c)));
        if full {
            self.id.clear();
            self.finding.clear();
            self.n = 0;
            self.built = true;
        }
        for (i, it) in items.iter().enumerate().skip(self.n) {
            self.id.entry(it.key().to_owned()).or_insert(i);
            if let Some(f) = it.finding_key() {
                self.finding.entry(f.to_owned()).or_insert(i);
            }
        }
        self.n = items.len();
        self.at = items.as_ptr() as usize;
        self.log_len = log.len();
    }

    /// Position of `id` in `items` (as `items.iter().position(|x| x.key() == id)`); call
    /// [`IdIndex::refresh`] first.
    pub(crate) fn get<T: Indexed>(&self, items: &[T], id: &str) -> Option<usize> {
        match self.id.get(id) {
            Some(&i) if items.get(i).is_some_and(|x| x.key() == id) => Some(i),
            // Not where the index says: the slow, always right answer.
            Some(_) => items.iter().position(|x| x.key() == id),
            None => None,
        }
    }

    /// Position of the first capsule of finding `f`.
    pub(crate) fn by_finding<T: Indexed>(&self, items: &[T], f: &str) -> Option<usize> {
        match self.finding.get(f) {
            Some(&i) if items.get(i).is_some_and(|x| x.finding_key() == Some(f)) => Some(i),
            Some(_) => items.iter().position(|x| x.finding_key() == Some(f)),
            None => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Sentences
// ---------------------------------------------------------------------------

/// The realizer's input for one sentence pair (`qp_capsule::SentenceInput`, owned): the
/// realizer is a pure function of it.
#[derive(Debug, Clone)]
pub(crate) struct SentenceKey {
    pub story: Story,
    pub lead: Fact,
    pub latest: Fact,
    pub now_bar: u64,
    pub now_t: i64,
    pub still_holds: Option<bool>,
    /// `current`, bit for bit.
    pub current: Option<u64>,
    pub redacted: bool,
}

/// Borrowed form of [`SentenceKey`] (compared without copying).
pub(crate) struct SentenceKeyRef<'a> {
    pub story: &'a Story,
    pub lead: &'a Fact,
    pub latest: &'a Fact,
    pub now_bar: u64,
    pub now_t: i64,
    pub still_holds: Option<bool>,
    pub current: Option<u64>,
    pub redacted: bool,
}

impl SentenceKeyRef<'_> {
    fn matches(&self, k: &SentenceKey) -> bool {
        self.now_bar == k.now_bar
            && self.now_t == k.now_t
            && self.still_holds == k.still_holds
            && self.current == k.current
            && self.redacted == k.redacted
            && *self.story == k.story
            && *self.lead == k.lead
            && *self.latest == k.latest
    }

    fn owned(&self) -> SentenceKey {
        SentenceKey {
            story: self.story.clone(),
            lead: self.lead.clone(),
            latest: self.latest.clone(),
            now_bar: self.now_bar,
            now_t: self.now_t,
            still_holds: self.still_holds,
            current: self.current,
            redacted: self.redacted,
        }
    }
}

/// Entries kept per capsule (readers with and without entitlement, two "now"s).
const PER_CAPSULE: usize = 4;

/// A sentence pair (TR, EN).
type Pair = (String, String);

/// Sentence pairs (TR, EN) by capsule, each with the exact input it was written from.
#[derive(Debug, Default)]
pub(crate) struct SentenceMemo {
    by: HashMap<String, Vec<(SentenceKey, Pair)>>,
}

impl SentenceMemo {
    pub(crate) fn get(&self, capsule: &str, k: &SentenceKeyRef<'_>) -> Option<(String, String)> {
        self.by
            .get(capsule)?
            .iter()
            .find(|(x, _)| k.matches(x))
            .map(|(_, v)| v.clone())
    }

    pub(crate) fn put(&mut self, capsule: &str, k: &SentenceKeyRef<'_>, v: (String, String)) {
        let e = self.by.entry(capsule.to_owned()).or_default();
        if e.len() >= PER_CAPSULE {
            e.remove(0);
        }
        e.push((k.owned(), v));
    }
}

// ---------------------------------------------------------------------------
// The authority's headlines
// ---------------------------------------------------------------------------

/// A series' bars as the sentences read them, bit for bit (`t` and the five values).
pub(crate) type BarsSnap = Option<Vec<[u64; 6]>>;

/// Bit-exact copy of `bars`.
pub(crate) fn snap(bars: Option<&[qp_schema::Bar]>) -> BarsSnap {
    bars.map(|b| {
        b.iter()
            .map(|x| {
                [
                    u64::from_ne_bytes(x.t.to_ne_bytes()),
                    x.o.to_bits(),
                    x.h.to_bits(),
                    x.l.to_bits(),
                    x.c.to_bits(),
                    x.v.to_bits(),
                ]
            })
            .collect()
    })
}

/// What the authority's headlines (`Session::headlines`, TR, entitled) were written
/// from, so that a refresh rewrites only the capsules whose input may have changed: the
/// capsules a log entry put (or whose facts it put) and those whose series' bars
/// changed. Valid while capsules are only appended or replaced and no two capsules
/// with a story share a finding; otherwise every headline is rewritten.
#[derive(Debug, Default)]
pub(crate) struct HeadState {
    pub built: bool,
    pub log_len: usize,
    /// Story series → its bars in the reading document and in the document.
    pub snaps: HashMap<String, (BarsSnap, BarsSnap)>,
    /// Capsule (with a story) → (series, finding, fact ids).
    pub caps: HashMap<String, (String, String, Vec<String>)>,
    pub by_series: HashMap<String, BTreeSet<String>>,
    pub by_fact: HashMap<String, BTreeSet<String>>,
    /// Finding → the capsule with a story that writes it.
    pub owner: HashMap<String, String>,
    /// (RP3/M15) Story series → the bar series its bars come from (the same in the
    /// reading document and the document; `None`: different or none — always compared).
    pub roots: HashMap<String, Option<String>>,
    /// (RP3/M15) Bar series whose bars changed since the last refresh (data events;
    /// `Session::data_changed`): a refresh compares the snapshots of these only.
    pub dirty: BTreeSet<String>,
    /// (RP3/M15) The reading document was rebuilt: compare every snapshot.
    pub dirty_all: bool,
}

impl HeadState {
    /// Forgets capsule `id`; returns the finding it wrote under.
    pub(crate) fn unregister(&mut self, id: &str) -> Option<String> {
        let (series, finding, facts) = self.caps.remove(id)?;
        if let Some(s) = self.by_series.get_mut(&series) {
            s.remove(id);
        }
        for f in &facts {
            if let Some(s) = self.by_fact.get_mut(f) {
                s.remove(id);
            }
        }
        if self.owner.get(&finding).is_some_and(|o| o == id) {
            self.owner.remove(&finding);
        }
        Some(finding)
    }

    /// Records capsule `id`; `false` when it is already recorded or another capsule
    /// already writes `finding`.
    pub(crate) fn register(
        &mut self,
        id: &str,
        series: &str,
        finding: &str,
        facts: Vec<String>,
    ) -> bool {
        if self.caps.contains_key(id) || self.owner.get(finding).is_some_and(|o| o != id) {
            return false;
        }
        self.owner.insert(finding.to_owned(), id.to_owned());
        self.by_series
            .entry(series.to_owned())
            .or_default()
            .insert(id.to_owned());
        for f in &facts {
            self.by_fact
                .entry(f.clone())
                .or_default()
                .insert(id.to_owned());
        }
        self.caps.insert(
            id.to_owned(),
            (series.to_owned(), finding.to_owned(), facts),
        );
        true
    }
}

// ---------------------------------------------------------------------------
// Reader projections
// ---------------------------------------------------------------------------

/// What one capsule (with a story) adds to a reader's projection (see
/// `Session::proj_scratch`, the reference computation).
#[derive(Debug, Clone, Default)]
pub(crate) struct Contrib {
    /// Hidden as a whole (born of a licensed fact the reader may not see).
    pub full: bool,
    /// Ids it hides (before the "shown elsewhere" rule).
    pub hidden: Vec<String>,
    pub redact: Vec<String>,
    /// Fact ids it shows (when not hidden as a whole).
    pub shown: Vec<String>,
    pub rewrite: bool,
    pub story: Option<Story>,
    /// Every fact id of its story (to follow fact changes).
    pub facts: Vec<String>,
}

/// One reader's projection, kept per capsule: counts of the ids each capsule hides,
/// redacts and shows, and the [`Proj`] they make. Valid for the entitlement answers it
/// was made with (checked on every use) and the log position it saw.
#[derive(Debug, Default)]
pub(crate) struct ReaderProj {
    pub built: bool,
    pub log_len: usize,
    /// An id hidden by one capsule names another capsule: the per-capsule rule does not
    /// hold, the reference computation is used.
    pub degenerate: bool,
    /// Instrument → its licensed flow feed (`None`: none) and whether the reader may read
    /// it, as used.
    pub flow: BTreeMap<String, (Option<FeedKey>, bool)>,
    /// Series → its bar feed (`None`: inline or unknown) and the answer, as used.
    pub bars: BTreeMap<String, (Option<FeedKey>, bool)>,
    contrib: HashMap<String, Contrib>,
    fact_caps: HashMap<String, BTreeSet<String>>,
    hidden_n: HashMap<String, u32>,
    shown_n: HashMap<String, u32>,
    redact_n: HashMap<String, u32>,
    proj: Rc<Proj>,
}

fn bump(m: &mut HashMap<String, u32>, k: &str, up: bool) {
    if up {
        *m.entry(k.to_owned()).or_insert(0) += 1;
    } else if let Some(n) = m.get_mut(k) {
        *n -= 1;
        if *n == 0 {
            m.remove(k);
        }
    }
}

impl ReaderProj {
    /// Empty, for `principal`.
    pub(crate) fn new(principal: &str) -> Self {
        Self {
            proj: Rc::new(Proj {
                principal: principal.to_owned(),
                ..Proj::default()
            }),
            ..Self::default()
        }
    }

    /// Whether capsule `cap` has a part.
    pub(crate) fn contains(&self, cap: &str) -> bool {
        self.contrib.contains_key(cap)
    }

    /// Fact id → capsules whose story names it.
    pub(crate) fn capsules_of_fact(&self, fact: &str) -> Vec<String> {
        self.fact_caps
            .get(fact)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn sync_hidden(&mut self, id: &str) {
        let on = self.hidden_n.contains_key(id) && !self.shown_n.contains_key(id);
        let p = Rc::make_mut(&mut self.proj);
        if on {
            if !p.hidden.contains(id) {
                p.hidden.insert(id.to_owned());
            }
        } else {
            p.hidden.remove(id);
        }
    }

    fn sync_redact(&mut self, id: &str) {
        let on = self.redact_n.contains_key(id);
        let p = Rc::make_mut(&mut self.proj);
        if on {
            if !p.redact.contains(id) {
                p.redact.insert(id.to_owned());
            }
        } else {
            p.redact.remove(id);
        }
    }

    fn apply(&mut self, cap: &str, c: &Contrib, up: bool) {
        for f in &c.facts {
            let e = self.fact_caps.entry(f.clone()).or_default();
            if up {
                e.insert(cap.to_owned());
            } else {
                e.remove(cap);
                if e.is_empty() {
                    self.fact_caps.remove(f);
                }
            }
        }
        for h in &c.hidden {
            bump(&mut self.hidden_n, h, up);
            self.sync_hidden(h);
        }
        if !c.full {
            for s in &c.shown {
                bump(&mut self.shown_n, s, up);
                self.sync_hidden(s);
            }
        }
        for r in &c.redact {
            bump(&mut self.redact_n, r, up);
            self.sync_redact(r);
        }
        let p = Rc::make_mut(&mut self.proj);
        if c.rewrite {
            if up {
                p.rewrite.insert(cap.to_owned());
            } else {
                p.rewrite.remove(cap);
            }
        }
        if let Some(s) = &c.story {
            if up {
                p.stories.insert(cap.to_owned(), s.clone());
            } else {
                p.stories.remove(cap);
            }
        }
    }

    /// Removes capsule `cap`'s part.
    pub(crate) fn remove(&mut self, cap: &str) {
        if let Some(c) = self.contrib.remove(cap) {
            self.apply(cap, &c, false);
        }
    }

    /// Sets capsule `cap`'s part (after [`ReaderProj::remove`]).
    pub(crate) fn add(&mut self, cap: &str, c: Contrib) {
        // The per-capsule rule needs: a capsule id is hidden only by that capsule.
        if self.hidden_n.contains_key(cap)
            || c.hidden
                .iter()
                .any(|h| h != cap && self.contrib.contains_key(h.as_str()))
        {
            self.degenerate = true;
        }
        self.apply(cap, &c, true);
        self.contrib.insert(cap.to_owned(), c);
    }

    /// The projection (`None`: everything goes as stored).
    pub(crate) fn result(&self) -> Option<Rc<Proj>> {
        (!self.proj.hidden.is_empty() || !self.proj.redact.is_empty())
            .then(|| Rc::clone(&self.proj))
    }
}

/// Whether any of `entries` may change what a reader's projection is made of beyond the
/// capsules and facts it puts: a series (binding) put or anything deleted.
pub(crate) fn proj_breaks(entries: &[LogEntry]) -> bool {
    entries.iter().any(|e| {
        e.changes
            .iter()
            .any(|c| matches!(c, ChangeOp::PutSeries { .. } | ChangeOp::Delete { .. }))
    })
}

// ---------------------------------------------------------------------------
// Readers' views
// ---------------------------------------------------------------------------

/// The reading document's bars, per series (in order), bit for bit; `None`: no reading
/// document. The reading document is the document (followed by the log) plus the bound
/// series' bars, so this and the log position say when it is unchanged.
pub(crate) type RdSnap = Option<Vec<BarsSnap>>;

pub(crate) fn rd_snap(rd: Option<&qp_schema::Document>) -> RdSnap {
    rd.map(|d| d.series.iter().map(|s| snap(s.bars())).collect())
}

/// Whether `rd` is the reading document `s` was taken of (bars bit for bit).
pub(crate) fn rd_same(s: &RdSnap, rd: Option<&qp_schema::Document>) -> bool {
    match (s, rd) {
        (None, None) => true,
        (Some(v), Some(d)) => {
            v.len() == d.series.len()
                && v.iter().zip(&d.series).all(|(a, x)| match (a, x.bars()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => {
                        a.len() == b.len()
                            && a.iter().zip(b).all(|(p, q)| {
                                p[0] == u64::from_ne_bytes(q.t.to_ne_bytes())
                                    && p[1] == q.o.to_bits()
                                    && p[2] == q.h.to_bits()
                                    && p[3] == q.l.to_bits()
                                    && p[4] == q.c.to_bits()
                                    && p[5] == q.v.to_bits()
                            })
                    }
                    _ => false,
                })
        }
        _ => false,
    }
}

/// What `Session::as_reader` gives one reader (headlines, projected document and
/// reading document), with what it was made of: valid while the log position, the
/// projection, the language and the reading document's bars are the same.
#[derive(Debug)]
pub(crate) struct ReaderView {
    pub principal: String,
    pub lang: qp_schema::protocol::Language,
    pub log_len: usize,
    pub lines: bool,
    pub proj: Option<Rc<Proj>>,
    pub rd: RdSnap,
    pub heads: Option<BTreeMap<String, String>>,
    pub doc: Option<qp_schema::Document>,
    pub rdp: Option<qp_schema::Document>,
}

/// Views kept (the most recent readers).
pub(crate) const VIEWS: usize = 4;

// ---------------------------------------------------------------------------
// Capsule items (RP4/T6)
// ---------------------------------------------------------------------------

/// Everything a `capsules` item's sentence, validity and voice are written from, for an
/// entitled reader (no projection): the capsule (its story), the story's facts as the
/// sentences find them, the bar series "now" is read on (its bars, bit for bit, through
/// `PartialEq`), the language, the voice and clock asked for, the headline calendar and the
/// display zone. The reader's `relevant`, `coin_flip` and the market note are not part of
/// it: they are applied to the item on every use.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemKey {
    pub(crate) capsule: Capsule,
    pub(crate) facts: Vec<Option<Fact>>,
    pub(crate) series: Option<qp_schema::Series>,
    pub(crate) lang: qp_schema::protocol::Language,
    pub(crate) voice: Option<String>,
    pub(crate) clock: bool,
    pub(crate) cal: Option<qp_schema::Calendar>,
    pub(crate) tz: Option<String>,
}

/// [`ItemKey`] by reference (a lookup clones nothing).
pub(crate) struct ItemKeyRef<'a> {
    pub(crate) capsule: &'a Capsule,
    pub(crate) facts: &'a [Option<&'a Fact>],
    pub(crate) series: Option<&'a qp_schema::Series>,
    pub(crate) lang: qp_schema::protocol::Language,
    pub(crate) voice: Option<&'a str>,
    pub(crate) clock: bool,
    pub(crate) cal: Option<&'a qp_schema::Calendar>,
    pub(crate) tz: Option<&'a str>,
}

impl ItemKeyRef<'_> {
    fn matches(&self, k: &ItemKey) -> bool {
        self.lang == k.lang
            && self.clock == k.clock
            && self.voice == k.voice.as_deref()
            && self.tz == k.tz.as_deref()
            && self.cal == k.cal.as_ref()
            && *self.capsule == k.capsule
            && self.series == k.series.as_ref()
            && self.facts.len() == k.facts.len()
            && self
                .facts
                .iter()
                .zip(&k.facts)
                .all(|(a, b)| *a == b.as_ref())
    }

    fn owned(&self) -> ItemKey {
        ItemKey {
            capsule: self.capsule.clone(),
            facts: self.facts.iter().map(|f| f.cloned()).collect(),
            series: self.series.cloned(),
            lang: self.lang,
            voice: self.voice.map(str::to_owned),
            clock: self.clock,
            cal: self.cal.cloned(),
            tz: self.tz.map(str::to_owned),
        }
    }
}

/// Entries kept per capsule (languages, voices).
const ITEMS_PER_CAPSULE: usize = 4;

/// (RP4/T6) `capsules` items (before the reader's `relevant`, `coin_flip` and market note)
/// by capsule, each with the exact input it was written from: a viewer's feed is
/// recomputed only for the capsules whose story, facts or bars changed.
#[derive(Debug, Default)]
pub(crate) struct ItemMemo {
    by: HashMap<String, Vec<(ItemKey, Option<qp_schema::protocol::CapsuleItem>)>>,
    /// Lookups that found / did not find their entry (counters only).
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl ItemMemo {
    pub(crate) fn get(
        &mut self,
        k: &ItemKeyRef<'_>,
    ) -> Option<Option<qp_schema::protocol::CapsuleItem>> {
        let v = self
            .by
            .get(&k.capsule.id)
            .and_then(|e| e.iter().find(|(x, _)| k.matches(x)))
            .map(|(_, v)| v.clone());
        if v.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        v
    }

    pub(crate) fn put(&mut self, k: &ItemKeyRef<'_>, v: Option<qp_schema::protocol::CapsuleItem>) {
        let e = self.by.entry(k.capsule.id.clone()).or_default();
        e.retain(|(x, _)| {
            !(x.lang == k.lang
                && x.clock == k.clock
                && x.voice.as_deref() == k.voice
                && x.tz.as_deref() == k.tz
                && x.cal.as_ref() == k.cal)
        });
        if e.len() >= ITEMS_PER_CAPSULE {
            e.remove(0);
        }
        e.push((k.owned(), v));
    }

    /// Forgets every entry (the counters stay).
    pub(crate) fn clear(&mut self) {
        self.by.clear();
    }

    /// Drops the entries of capsules not in `ids` once there are many more entries than
    /// capsules.
    pub(crate) fn prune<'a>(&mut self, ids: impl Iterator<Item = &'a str>, n: usize) {
        if self.by.len() <= 2 * n + 64 {
            return;
        }
        let keep: BTreeSet<&str> = ids.collect();
        self.by.retain(|id, _| keep.contains(id.as_str()));
    }
}

#[cfg(test)]
mod item_memo_tests {
    use std::collections::BTreeMap;

    use qp_schema::market::{MarketSource, MarketStatus, PhaseCode};
    use qp_schema::protocol::{OutboundLine, ReplayInit, ReplaySourceKind};
    use serde_json::json;

    use crate::{demo_init, ReplayHost};

    /// The demo world (synthetic, as `qp demo`), small; `visitor` reads without data.
    fn host() -> ReplayHost {
        let mut ent = BTreeMap::new();
        for p in ["host", "arkin"] {
            ent.insert(p.to_owned(), vec!["*".to_owned()]);
        }
        ent.insert("visitor".to_owned(), Vec::new());
        let mut init = demo_init(ReplayInit {
            source: ReplaySourceKind::Synthetic,
            instruments: Some(8),
            days: Some(260),
            history: Some(120),
            fine: Some("1h".to_owned()),
            entitlements: ent,
            ..ReplayInit::default()
        });
        init.principals.push(
            serde_json::from_value(json!(
                {"id": "visitor", "author": {"kind": "human", "name": "visitor"}, "role": "owner"}))
            .expect("visitor"),
        );
        let none = std::env::temp_dir().join("qp-t6-no-local-data");
        ReplayHost::with_local_dir(init, std::path::Path::new("."), &none).expect("demo host")
    }

    fn text(lines: &[OutboundLine]) -> String {
        serde_json::to_string(lines).expect("json")
    }

    fn send(h: &mut ReplayHost, conn: &str, m: &serde_json::Value) -> String {
        text(&h.handle_raw(conn, &m.to_string()))
    }

    fn market(t: i64, open: bool, phase: Option<(&str, i32)>) -> MarketStatus {
        MarketStatus {
            exchange_time: t,
            open: Some(open),
            holiday: Some(false),
            source: MarketSource::Calendar,
            delayed: false,
            stale: false,
            phases: phase
                .map(|(s, c)| {
                    (
                        s.to_owned(),
                        PhaseCode {
                            code: c,
                            key: format!("p{c}"),
                        },
                    )
                })
                .into_iter()
                .collect(),
        }
    }

    /// (RP4/T6) Incremental = from scratch: two identical hosts take the same steps,
    /// messages and market statuses in a fixed order; one keeps its item memo, the other
    /// forgets it before every input. Every line both send (viewer events, `capsules`
    /// answers, notifications) is byte-equal, over readers with and without a voice, a
    /// clock, both languages and a reader without data (projection: no memo).
    #[test]
    fn incremental_items_equal_from_scratch() {
        let mut a = host();
        let mut b = host();
        let setup = [
            (
                "h",
                json!({"type":"hello","protocol":"qp/0","principal":"host","surface":{"kind":"phone","lang":"tr"}}),
            ),
            (
                "h",
                json!({"type":"input","id":"s1","gesture":{"kind":"style","voice":"marla","clock":true}}),
            ),
            (
                "h",
                json!({"type":"subscribe","id":"v","topic":{"kind":"viewer","limit":40}}),
            ),
            (
                "e",
                json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone","lang":"en"}}),
            ),
            (
                "e",
                json!({"type":"subscribe","id":"v","topic":{"kind":"viewer"}}),
            ),
            (
                "x",
                json!({"type":"hello","protocol":"qp/0","principal":"visitor","surface":{"kind":"phone","lang":"tr"}}),
            ),
            (
                "x",
                json!({"type":"subscribe","id":"v","topic":{"kind":"viewer"}}),
            ),
        ];
        for (c, m) in &setup {
            assert_eq!(send(&mut a, c, m), send(&mut b, c, m), "setup {m}");
        }
        let query = json!({"type":"query","id":"q","query":{"kind":"capsules"}});
        let mut steps = 0;
        for i in 0..160 {
            // Inputs other than data, at fixed steps.
            let extra: Option<(&str, serde_json::Value)> = match i {
                20 => Some((
                    "h",
                    json!({"type":"input","id":"s2","gesture":{"kind":"style","voice":"selvi"}}),
                )),
                45 => Some((
                    "e",
                    json!({"type":"input","id":"s3","gesture":{"kind":"style","voice":"marla","clock":true}}),
                )),
                70 => Some((
                    "h",
                    json!({"type":"input","id":"s4","gesture":{"kind":"style","voice":"r4","clock":false}}),
                )),
                100 => Some((
                    "h",
                    json!({"type":"input","id":"s5","gesture":{"kind":"style","voice":"marla"}}),
                )),
                _ => None,
            };
            if let Some((c, m)) = extra {
                b.session().forget_item_memo();
                assert_eq!(send(&mut a, c, &m), send(&mut b, c, &m), "step {i}: {m}");
            }
            if i % 30 == 10 {
                let t = a.session().clock();
                let m = market(t, i % 60 == 10, (i % 60 == 40).then_some(("S1", 3)));
                b.session().forget_item_memo();
                assert_eq!(
                    text(&a.set_market_status(m.clone())),
                    text(&b.set_market_status(m)),
                    "step {i}: market"
                );
            }
            b.session().forget_item_memo();
            let (sa, sb) = (a.take_step(), b.take_step());
            let (Some((_, la)), Some((_, lb))) = (sa, sb) else {
                break;
            };
            steps += 1;
            assert_eq!(text(&la), text(&lb), "step {i}");
            if i % 7 == 3 {
                for c in ["h", "e", "x"] {
                    b.session().forget_item_memo();
                    assert_eq!(
                        send(&mut a, c, &query),
                        send(&mut b, c, &query),
                        "step {i}: {c}"
                    );
                }
            }
        }
        assert!(steps > 100, "the replay ran ({steps} steps)");
        let m = a.session().item_memo.borrow();
        // Every step moves every instrument's bars here (all 8 step together), so most
        // lookups miss; the reuse is between steps' messages and the queries.
        assert!(
            m.hits > 1000 && m.misses > 0,
            "the memo was used: {} / {}",
            m.hits,
            m.misses
        );
    }
}
