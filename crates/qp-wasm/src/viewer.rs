//! The device as a viewer (RP3/M15, K48; `docs/PROTOCOL.md` §22): what the web shell's
//! core keeps in viewer mode instead of a replica of the whole document.
//!
//! - The feed and the ready faces come from the authority's `viewer` subscription (the
//!   device compiles no face and runs no capsule engine for the feed); faces are asked
//!   for by visibility (`visible`).
//! - The open capsule: its slice and the open instrument's own data only
//!   ([`qp_session::viewer::OpenSide`]: a replica of the slice, the display rule over its
//!   claims, the story check, the candidate).
//! - Traffic counters for the measurement (bytes per frame kind), in both modes.
//!
//! Work per tick and memory do not depend on the size of the universe: the feed is at
//! most `limit` rows, faces are only the visible ones, data only the open instrument's.

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::capsule::Audience;
use qp_schema::data::{DataEvent, FeedKey};
use qp_schema::protocol::{InitFeed, Language, ServerMsg, ViewerFace, ViewerItem, ViewerUpdate};
use qp_schema::scene::SceneDesc;
use qp_session::viewer::{open_provider, trust_text, OpenSide, TrustState};
use serde_json::{json, Value};

/// The `viewer` subscription id the shell core uses.
pub const SUB: &str = "viewer";

/// Bytes and frames received, by frame kind (`start`, `msg`, `data`, `bind`).
#[derive(Debug, Clone, Default)]
pub struct Traffic {
    by_kind: BTreeMap<&'static str, (u64, u64)>,
}

impl Traffic {
    /// Counts one frame of `bytes` bytes.
    pub fn record(&mut self, kind: &'static str, bytes: usize) {
        let e = self.by_kind.entry(kind).or_default();
        e.0 += 1;
        e.1 += bytes as u64;
    }

    /// Starts over (the metrics page's reset).
    pub fn reset(&mut self) {
        self.by_kind.clear();
    }

    /// `{kind: {frames, bytes}}`.
    #[must_use]
    pub fn json(&self) -> Value {
        Value::Object(
            self.by_kind
                .iter()
                .map(|(k, (n, b))| ((*k).to_owned(), json!({"frames": n, "bytes": b})))
                .collect(),
        )
    }
}

/// The viewer state of the shell core.
pub struct Viewer {
    principal: String,
    lang: Language,
    /// The feed rows as the authority sent them.
    pub feed: Vec<ViewerItem>,
    faces: BTreeMap<String, ViewerFace>,
    /// The data subscriptions the authority named.
    pub data: Vec<FeedKey>,
    /// The reader's audience (as the authority sent it).
    pub audience: Option<Audience>,
    /// Symbols of the universe (the settings page's list; from the start frame).
    pub instruments: Vec<String>,
    want: BTreeSet<String>,
    sent: Option<Vec<String>>,
    inflight: bool,
    /// Feeds the host served for the open instrument (initial bars, events since).
    bound: BTreeMap<FeedKey, (InitFeed, Vec<DataEvent>)>,
    open: Option<OpenSide>,
    seq: u64,
    calendar: Option<qp_schema::Calendar>,
    /// How many checks of the open capsule ran.
    pub checks: u64,
}

impl std::fmt::Debug for Viewer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Viewer")
            .field("feed", &self.feed.len())
            .field("faces", &self.faces.len())
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl Viewer {
    /// A viewer for `principal` reading in `lang`; `instruments`: the universe's symbols.
    #[must_use]
    pub fn new(principal: &str, lang: Language, instruments: Vec<String>) -> Self {
        Self {
            principal: principal.to_owned(),
            lang,
            feed: Vec::new(),
            faces: BTreeMap::new(),
            data: Vec::new(),
            audience: None,
            instruments,
            want: BTreeSet::new(),
            sent: None,
            inflight: false,
            bound: BTreeMap::new(),
            open: None,
            seq: 0,
            calendar: None,
            checks: 0,
        }
    }

    /// The headline-piece calendar of the open replica (the replay source's).
    pub fn set_calendar(&mut self, cal: Option<qp_schema::Calendar>) {
        self.calendar.clone_from(&cal);
        if let Some(o) = self.open.as_mut() {
            o.replica_mut().set_headline_calendar(cal);
        }
    }

    /// The capsules of the feed, in the authority's order.
    #[must_use]
    pub fn items(&self) -> Vec<qp_schema::protocol::CapsuleItem> {
        self.feed.iter().map(|r| r.capsule.clone()).collect()
    }

    /// The row of capsule `id`.
    #[must_use]
    pub fn row(&self, id: &str) -> Option<&ViewerItem> {
        self.feed.iter().find(|r| r.capsule.id == id)
    }

    /// The ready face of capsule `id` (as the authority compiled it).
    #[must_use]
    pub fn face(&self, id: &str) -> Option<&SceneDesc> {
        self.faces.get(id).map(|f| &*f.scene)
    }

    /// The digest of the ready face of `id`.
    #[must_use]
    pub fn face_digest(&self, id: &str) -> Option<&str> {
        self.faces.get(id).map(|f| f.digest.as_str())
    }

    /// The shell says capsule `id` is (or is no longer) on screen.
    pub fn visible(&mut self, id: &str, on: bool) {
        if on {
            self.want.insert(id.to_owned());
        } else {
            self.want.remove(id);
        }
    }

    /// The `visible` list to send now (feed order), if it changed and none is in flight.
    pub fn take_visible(&mut self) -> Option<Vec<String>> {
        if self.inflight {
            return None;
        }
        let list: Vec<String> = self
            .feed
            .iter()
            .map(|r| r.capsule.id.clone())
            .filter(|id| self.want.contains(id))
            .collect();
        if self.sent.as_ref() == Some(&list) {
            return None;
        }
        self.sent = Some(list.clone());
        self.inflight = true;
        Some(list)
    }

    /// The answer to the last `visible` came (or failed).
    pub fn visible_answered(&mut self) {
        self.inflight = false;
    }

    /// Applies what changed (a `viewer` event or a `visible` answer) at log position
    /// `seq`. Returns whether the open scene may have changed.
    pub fn apply(&mut self, up: &ViewerUpdate, seq: u64) -> bool {
        self.seq = seq;
        if let Some(f) = &up.feed {
            self.feed.clone_from(f);
            let ids: BTreeSet<&str> = f.iter().map(|r| r.capsule.id.as_str()).collect();
            self.faces.retain(|k, _| ids.contains(k.as_str()));
        }
        for f in &up.faces {
            self.faces.insert(f.capsule.clone(), f.clone());
        }
        if up.audience.is_some() {
            self.audience.clone_from(&up.audience);
        }
        self.data.clone_from(&up.data);
        let data = &self.data;
        self.bound.retain(|k, _| data.contains(k));
        let mut scene = false;
        if up.closed {
            self.open = None;
            scene = true;
        }
        if let Some(o) = &up.open {
            scene = true;
            match self.open.as_mut().filter(|x| x.open.capsule == o.capsule) {
                Some(side) => side.update(o.clone(), seq),
                None => self.open = self.make_open(o.clone(), seq),
            }
        }
        scene
    }

    fn make_open(&self, o: qp_schema::protocol::ViewerOpen, seq: u64) -> Option<OpenSide> {
        let feeds: Vec<InitFeed> = o
            .feeds
            .iter()
            .filter_map(|k| self.bound.get(k).map(|(f, _)| f.clone()))
            .collect();
        let events: Vec<DataEvent> = o
            .feeds
            .iter()
            .filter_map(|k| self.bound.get(k))
            .flat_map(|(_, e)| e.iter().cloned())
            .collect();
        let prov = open_provider(&self.principal, &feeds, &events).ok()?;
        Some(OpenSide::new(
            &self.principal,
            o,
            seq,
            prov,
            self.calendar.clone(),
        ))
    }

    /// A `bind` frame of the host: the initial bars of a feed it now serves this device
    /// and the data events since. Rebuilds the open side if it rests on that feed.
    pub fn bind(&mut self, feed: InitFeed, events: Vec<DataEvent>) {
        let key = feed.feed.clone();
        self.bound.insert(key.clone(), (feed, events));
        if let Some(o) = self.open.as_ref().filter(|o| o.open.feeds.contains(&key)) {
            let open = o.open.clone();
            self.open = self.make_open(open, self.seq);
        }
    }

    /// A data event of a feed the device subscribes to.
    pub fn data_event(&mut self, ev: &DataEvent) {
        if !self.data.contains(&ev.feed) {
            return;
        }
        match self.open.as_mut() {
            Some(o) if o.open.feeds.contains(&ev.feed) => o.data_event(ev),
            _ => {
                if let Some(b) = self.bound.get_mut(&ev.feed) {
                    b.1.push(ev.clone());
                }
            }
        }
    }

    /// A message of the authority on the viewer subscription (claims of the open
    /// instrument).
    pub fn on_message(&mut self, msg: &ServerMsg) {
        if let Some(o) = self.open.as_mut() {
            o.on_message(msg);
        }
    }

    /// The open side.
    pub fn open_mut(&mut self) -> Option<&mut OpenSide> {
        self.open.as_mut()
    }

    /// The open side (read access).
    #[must_use]
    pub fn open(&self) -> Option<&OpenSide> {
        self.open.as_ref()
    }

    /// The trust state of the open capsule (`None`: nothing open).
    pub fn trust(&mut self) -> Option<TrustState> {
        let t = self.open.as_mut().map(OpenSide::trust);
        if t.is_some() {
            self.checks += 1;
        }
        t
    }

    /// The trust mark's text for a feed row (the authority's settled claim).
    #[must_use]
    pub fn feed_trust(&self) -> &'static str {
        trust_text(TrustState::Server, self.lang)
    }

    /// The trust mark's text for the open capsule.
    pub fn open_trust(&mut self) -> Option<&'static str> {
        let lang = self.lang;
        self.trust().map(|t| trust_text(t, lang))
    }

    /// Forces the open capsule's check to run again on its own data (the measurement's
    /// handle): `{state, text, candidates}`.
    pub fn check_now(&mut self) -> Value {
        let lang = self.lang;
        let Some(o) = self.open.as_mut() else {
            return json!({"state": null});
        };
        let rebuilt = o.open.clone();
        o.update(rebuilt, self.seq);
        let t = o.trust();
        let n = o.candidates().len();
        self.checks += 1;
        json!({"state": t, "text": trust_text(t, lang), "candidates": n})
    }

    /// What the device subscribes to and holds (VERIFY 17): `{data, open, feed, faces,
    /// bars, checks}` — `bars`: the bars of every series its open replica holds.
    #[must_use]
    pub fn stats(&self) -> Value {
        let bars: usize = self
            .open
            .as_ref()
            .and_then(|o| {
                let r = o.replica();
                let doc = r.document()?;
                Some(
                    doc.series
                        .iter()
                        .filter_map(|s| r.bars(&s.id).map(<[_]>::len))
                        .sum(),
                )
            })
            .unwrap_or(0);
        json!({
            "data": self.data,
            "open": self.open.as_ref().map(|o| json!({
                "capsule": o.open.capsule, "instrument": o.open.instrument,
                "feeds": o.open.feeds, "slice": o.open.digest,
                "series": o.open.document.series.len(),
                "capsules": o.open.document.capsules.len(),
            })),
            "feed": self.feed.len(),
            "faces": self.faces.len(),
            "bound": self.bound.keys().collect::<Vec<_>>(),
            "bars": bars,
            "checks": self.checks,
        })
    }
}
