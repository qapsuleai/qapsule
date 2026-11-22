//! The demo shell's core (R4/T5; `docs/ARCHITECTURE.md` "Kabuk sınırı (R4)").
//!
//! Everything the web shell would otherwise have to decide lives here, in Rust, so that
//! the JavaScript only moves bytes and pixels: the replica (document mirror and own data,
//! `qp_session::Replica`), the protocol messages a user's action becomes (including the
//! gesture recognizer that turns raw pointer events into `tap_at` / `drag` / `pinch`),
//! the scenes of the capsule faces and of the open capsule (grammar compile over the
//! replica's own data, the connection's style), the pixels (`qp-render`), the text shown
//! for the replay state and the evidence (formatted here, never in JavaScript), the
//! transition boxes, and the metrics (raw timestamps in, statistics out).
//!
//! [`ShellCore`] is plain Rust (native tests and the shell-less driver use it);
//! [`crate::Shell`] is its wasm-bindgen wrapper. The pure scene functions
//! ([`face_scene`], [`open_scene`]) are what both the shell and the shell-less driver
//! digest (`docs/PROTOCOL.md` §19.6 `view_digest`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{
    CapsuleItem, Event, EvidenceAccess, InitData, Language, OpenDepth, Opening, QueryResult,
    ReplayState, ServerMsg, ViewState,
};
use qp_schema::scene::SceneDesc;
use qp_session::Replica;
use serde_json::{json, Value};

use crate::viewer::{Stories, Traffic, Viewer, STORIES_SUB};
pub mod agent;
pub mod bench;
pub mod freq;
pub mod live;
pub mod motion;
pub mod paint;
pub mod paint_canvas;
pub mod paint_gpu;

// RG1: `shell.rs` split by topic; the moved items keep their `crate::shell::…` paths.
mod feed;
mod inbound;
mod input;
mod look;
mod metrics;
mod scene;
mod start;

use feed::{depth_name, lang_name, synthetic_rows};
pub use feed::{display_time, feed_rows, local_stamp};
use metrics::{round3, wasm_memory_bytes, Metrics, Samples, FRAME_GAP_MS};
pub use metrics::{
    MetricsCarry, Refresh, LONG_FACTOR, OPEN_PLAUSIBLE_MS, REFRESH_CLASSES_HZ, REFRESH_MIN_SAMPLES,
    VERY_LONG_FACTOR,
};
pub use start::{core_from_start, demo_session_init, DemoOptions, Frames, VISITOR};
use start::{BindFrame, DataFrame, MarketFrame};

/// Face frame height of the default face design (`r3`, 240 CSS px). (R4/H-M8910) The
/// height a face is laid out at is the design's own (`qp_grammar::faces::height`), read
/// by [`ShellCore::face_size_json`]; the shell never computes it. Reported with every face
/// gesture (the authority's own default is 140).
pub const FACE_HEIGHT: u32 = 240;
/// Largest device scale the core paints at (HiDPI).
pub const MAX_SCALE: u32 = 3;
/// Phone frame (`screen`'s surface frame for phones).
pub const PHONE_FRAME: (u32, u32) = (390, 300);
/// (R5/T3b) The phone surface's screen (`hello` `surface.screen`; the demo page's phone
/// frame, `--qp-phone-w`/`--qp-phone-h`).
pub const PHONE_SCREEN: (u32, u32) = (390, 844);
/// Capsules asked for per feed query.
pub const FEED_LIMIT: u32 = 40;
/// Pointer movement below which a press is a tap (CSS px).
pub const TAP_SLOP: f64 = 8.0;
/// Pinch factor of one wheel notch.
pub const WHEEL_FACTOR: f64 = 1.25;
/// The display time zone of the demo (BIST).
pub const DISPLAY_TZ: &str = "Europe/Istanbul";

// (RP3/M15) The pure scene functions live in `qp_session::viewer`: the authority compiles
// a viewer's faces with the very same code (§22).
pub use qp_session::viewer::{face_scene, open_scene, open_scene_with, scene_digest, story_series};

#[cfg(target_arch = "wasm32")]
mod clock {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        /// `performance.now()` (ms), for the core's own spans.
        #[wasm_bindgen(js_namespace = performance, js_name = now)]
        pub fn now() -> f64;
    }
}

/// (RP3/S) `performance.now()` in wasm; 0 elsewhere (no span is recorded there).
#[must_use]
pub(crate) fn core_now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        clock::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

/// (RP3/S) The key of the hand-out change check ("did this face/scene change since it was
/// last painted?"); internal, never a digest anyone else sees (parity and `digests()` keep
/// [`scene_digest`], FNV over the JSON).
pub(crate) type ShownKey = u64;

/// (RP3/S) The change key of `scene`: [`qp_render::retained::scene_fingerprint`], a 64-bit
/// SipHash of size, background and every primitive's fields in paint order — no JSON
/// serialisation (measured on the JIT-less phone setting: the JSON digest was 7–8 ms of a
/// 14 ms paint pass with the chart open, `rounds/RP3/experiments/S.md` §6.2).
#[must_use]
pub(crate) fn shown_key(scene: &SceneDesc) -> ShownKey {
    qp_render::retained::scene_fingerprint(scene)
}

// ---------------------------------------------------------------------------
// The shell core
// ---------------------------------------------------------------------------

/// What an outstanding request was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    Feed,
    Input,
    Replay,
    Command,
    /// (RP3/M15) A `visible` message.
    Visible,
    /// (R5/M18) A `stories` query (a page of the open instrument's story list).
    Stories,
    /// (R5/M18) The `stories` unsubscription.
    Unsubscribe,
    /// (R5B) The pending proposals (`list`).
    Proposals,
    /// (R5B) The open capsule's `capsule` answer (its comments).
    Comments,
    /// (R5B) The reader's decision on a proposal.
    Decision,
}

/// The gesture in progress on one target.
#[derive(Debug, Clone, Default)]
struct Gesture {
    target: String,
    /// Pointer id → (start x, start y, x, y).
    ptrs: Vec<(i32, f64, f64, f64, f64)>,
    dragging: bool,
    pinching: bool,
    /// Drag: x already sent (or accumulated) up to; pinch: distance up to.
    last_x: f64,
    last_d: f64,
    /// Movement waiting for the in-flight input's answer (coalesced).
    acc_dx: f64,
    acc_factor: f64,
    acc_x: Option<f64>,
    cancelled: bool,
}

/// (RP3/T4) The record of sent messages keeps at least this many of the latest (see
/// `ShellCore::sent`); a short session (every test) keeps all of them.
pub const SENT_KEEP: usize = 1024;

/// The core of the web shell (see the module docs).
pub struct ShellCore {
    principal: String,
    lang: Language,
    replica: Replica,
    welcomed: bool,
    max_points: u32,
    feed: Vec<CapsuleItem>,
    /// (R5/M19) The authority answered the feed at least once (an empty feed is then
    /// said, not left blank), and how many stories its coin-flip rule kept out.
    feed_known: bool,
    coin_flip_hidden: u32,
    /// (R5/M19) The page's feed order (`?feed_order=`, `qp_grammar::feeds::ORDERS`): when
    /// set, it orders the feed instead of the connection's layout (a presentation setting,
    /// not a protocol value). And the feed rule the page asked its host for
    /// (`?feed_rule=`; the host applies it on connect), shown by the settings page.
    feed_order: Option<String>,
    feed_rule: Option<String>,
    feed_dirty: bool,
    feed_inflight: bool,
    look: ViewState,
    opening: Option<Opening>,
    /// The open capsule's sentence (from the feed when it was opened).
    open_headline: Option<String>,
    /// (R5/m17-int) The open capsule's voice surfaces (from the feed when it was opened).
    open_voice: Option<qp_schema::protocol::CapsuleVoice>,
    /// (R5/m17-int, K49) The glossary term the last tap landed on (the authority's
    /// `hit.term`) and the target tapped: the page shows its explanation.
    gloss: Option<(String, qp_schema::scene::TermMark)>,
    /// The target of the last tap (`face:<capsule>` or `scene`).
    tapped: String,
    /// The shared elements of the last transition.
    shared: Vec<qp_schema::protocol::SharedElement>,
    shared_from: String,
    replay: Option<ReplayState>,
    outbox: Vec<Value>,
    next_id: u64,
    asked: BTreeMap<String, Asked>,
    error: Option<String>,
    frames: BTreeMap<String, (u32, u32)>,
    gesture: Option<Gesture>,
    metrics: Metrics,
    /// Last change key of each face handed out (a face is redrawn only when it changed;
    /// RP3/S [`shown_key`]).
    face_shown: BTreeMap<String, ShownKey>,
    scene_shown: Option<ShownKey>,
    /// (R5/T3) The (look, theme, (R5/T3b) face and scene frames) whose CSS custom
    /// properties were last handed out ([`ShellCore::take_css`]).
    css_shown: Option<(Option<String>, Option<String>, [u32; 4])>,
    version: u64,
    /// The messages sent, for the record and the parity check: every message of a short
    /// session; (RP3/T4) on a long one only the latest [`SENT_KEEP`] to `2 × SENT_KEEP`
    /// (the oldest are dropped in blocks, so that the record does not grow without bound).
    sent: Vec<Value>,
    /// (R4/H-M8910) Device scale the scenes are painted at (1: CSS px).
    scale: u32,
    /// (R4/H-M8910) Start (shell timestamp, ms) of the running transition's animation.
    trans_t0: Option<f64>,
    /// (R4/S1) A last look the page asked for (`?seen=`) that waits for the replay: `None`
    /// now (at the first known replay clock), `Some(n)` when the replay reaches step `n`.
    seen_pending: Option<Option<u64>>,
    /// (R4/S2) The symbols of the host's feeds (initial data): what a URL audience list
    /// may name.
    symbols: std::collections::BTreeSet<String>,
    /// (RP3/M15, §22) Viewer mode: the feed and its faces come ready from the authority,
    /// only the open instrument's data and slice live here (`None`: the full replica).
    viewer: Option<Viewer>,
    /// (RP3/M15) Bytes received by frame kind (both modes; the measurement's).
    traffic: Traffic,
    /// (RP3/M15) The log position of the last viewer event.
    viewer_seq: u64,
    /// (RP3/M14) Changes waiting for the page's next paint pass, the arrival time of the
    /// first of them, and that of the pass being painted.
    pending: serde_json::Map<String, Value>,
    pending_since: Option<f64>,
    /// (R5/M21) The motion design, reduced motion and the faces' changes (`motion`).
    pub(crate) mo: motion::MotionState,
    pass_since: Option<f64>,
    /// (R5/T0) A data frame raised some of the waiting changes / of those the pass being
    /// painted took (the pass is then a data frame's cost, `data_pass`).
    pending_data: bool,
    pass_data: bool,
    /// (RP3/M14) The capsules whose face the page reports on screen (both modes).
    shown: BTreeSet<String>,
    /// (R6/T3, §24) The market status the host gave (a `{"market":…}` frame or the
    /// `viewer` event's `market`); `None`: never given (everything as before).
    market: Option<qp_schema::market::MarketStatus>,
    /// (R5B) The open capsule's comments and the pending proposals (`agent`).
    agent: agent::AgentTrace,
}

impl std::fmt::Debug for ShellCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellCore")
            .field("principal", &self.principal)
            .field("feed", &self.feed.len())
            .finish_non_exhaustive()
    }
}

impl ShellCore {
    /// A shell core for `principal` reading in `lang` whose replica's data comes from
    /// `data` (the host's initial bars and entitlements) plus the `events` the host
    /// applied since; queues the `hello`.
    ///
    /// # Errors
    /// An invalid provider description.
    pub fn new(
        principal: &str,
        lang: &str,
        data: &InitData,
        events: &[DataEvent],
    ) -> Result<Self, String> {
        let lang = if lang == "en" {
            Language::En
        } else {
            Language::Tr
        };
        let mut prov = qp_source::MemoryProvider::from_init(data, std::path::Path::new("."))
            .map_err(|e| e.to_string())?;
        for ev in events {
            let _ = prov.apply(ev);
        }
        let replica = Replica::with_memory(principal, prov);
        let mut me = Self {
            principal: principal.to_owned(),
            lang,
            replica,
            welcomed: false,
            max_points: 60,
            feed: Vec::new(),
            feed_known: false,
            coin_flip_hidden: 0,
            // (R5/fix47) The presentation's feed order (live and new first); `?feed_order=
            // layout` gives the connection's layout back.
            feed_order: Some(qp_grammar::feeds::PRESENTED_FEED_ORDER.to_owned()),
            feed_rule: None,
            feed_dirty: false,
            feed_inflight: false,
            look: ViewState::default(),
            opening: None,
            open_headline: None,
            open_voice: None,
            gloss: None,
            tapped: String::new(),
            shared: Vec::new(),
            shared_from: String::new(),
            replay: None,
            outbox: Vec::new(),
            next_id: 0,
            asked: BTreeMap::new(),
            error: None,
            frames: BTreeMap::from([
                (
                    "face".to_owned(),
                    (PHONE_FRAME.0, qp_grammar::faces::height(None)),
                ),
                ("scene".to_owned(), PHONE_FRAME),
            ]),
            gesture: None,
            metrics: Metrics::default(),
            face_shown: BTreeMap::new(),
            scene_shown: None,
            css_shown: None,
            version: 0,
            sent: Vec::new(),
            scale: 1,
            trans_t0: None,
            seen_pending: None,
            symbols: data.feeds.iter().map(|f| f.feed.symbol.clone()).collect(),
            viewer: None,
            traffic: Traffic::default(),
            viewer_seq: 0,
            pending: serde_json::Map::new(),
            pending_since: None,
            // (R5/S) The presentation's motion design (M21: `surek`, provisional); its
            // transition goes with the presented `style` gesture (`presented_look`).
            mo: motion::MotionState {
                design: Some(qp_grammar::motion::PRESENTED_MOTION.to_owned()),
                ..motion::MotionState::default()
            },
            pass_since: None,
            pending_data: false,
            pass_data: false,
            shown: BTreeSet::new(),
            market: None,
            agent: agent::AgentTrace::default(),
        };
        me.push(json!({"type":"hello","protocol":"qp/0","principal":principal,
            "surface":{"kind":"phone","screen":[PHONE_SCREEN.0, PHONE_SCREEN.1],"input":["touch"],"lang":lang_name(lang)}}));
        // (R4/S1) The presentation's own settings: the shell asks for them right after
        // `hello`; whatever the page chose (URL, settings page) is sent after and wins.
        me.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":Self::presented_look()}),
        );
        Ok(me)
    }

    /// (RP3/M15, §22) A shell core in viewer mode: no replica of the document and no data
    /// of the universe; `instruments`: the universe's symbols (the settings page's list).
    /// Queues the `hello`.
    ///
    /// # Errors
    /// None today (the signature matches [`ShellCore::new`]).
    pub fn new_viewer(principal: &str, lang: &str, instruments: &[String]) -> Result<Self, String> {
        let mut me = Self::new(principal, lang, &InitData::default(), &[])?;
        me.symbols = instruments.iter().cloned().collect();
        me.viewer = Some(Viewer::new(principal, me.lang, instruments.to_vec()));
        Ok(me)
    }

    /// (RP3/M15) Whether the core runs in viewer mode.
    #[must_use]
    pub fn is_viewer(&self) -> bool {
        self.viewer.is_some()
    }

    /// (RP3/M15) Counts a frame of the host's stream (`start`, `msg`, `data`, `bind`).
    pub fn count_frame(&mut self, kind: &'static str, bytes: usize) {
        self.traffic.record(kind, bytes);
    }

    /// (RP3/M15) The shell says capsule `id`'s face is (or is no longer) on screen: a
    /// viewer asks the authority for the faces on screen only.
    pub fn visible(&mut self, id: &str, on: bool) {
        // (RP3/M14) Kept in both modes: the paint pass paints the faces on screen only.
        if on {
            self.shown.insert(id.to_owned());
        } else {
            self.shown.remove(id);
        }
        if let Some(v) = self.viewer.as_mut() {
            v.visible(id, on);
        }
    }

    /// (RP3/M15, VERIFY 17) What the device subscribes to and holds, and the traffic:
    /// `{mode, data, open, feed, faces, bound, bars, checks, traffic}`.
    #[must_use]
    pub fn viewer_json(&self) -> Value {
        let mut v = match &self.viewer {
            Some(x) => x.stats(),
            None => json!({
                "data": self.symbols.len(),
                "bars": self.replica.document().map_or(0, |d| d.series.iter()
                    .filter_map(|s| self.replica.bars(&s.id).map(<[_]>::len)).sum::<usize>()),
            }),
        };
        v["mode"] = json!(if self.viewer.is_some() {
            "viewer"
        } else {
            "replica"
        });
        v["traffic"] = self.traffic.json();
        v
    }

    /// (RP3/M15) Runs the open capsule's check on its own data again (the measurement's
    /// handle): `{state, text, candidates}` (`state` null: nothing open or full replica).
    pub fn check_now(&mut self) -> Value {
        match self.viewer.as_mut() {
            Some(v) => v.check_now(),
            None => json!({"state": null}),
        }
    }

    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    fn push(&mut self, m: Value) {
        self.outbox.push(m);
    }

    fn ask(&mut self, prefix: &str, kind: Asked, mut m: Value) {
        let id = self.id(prefix);
        m["id"] = Value::String(id.clone());
        self.asked.insert(id, kind);
        self.push(m);
    }

    fn input_in_flight(&self) -> bool {
        self.asked.values().any(|a| *a == Asked::Input)
    }

    /// The messages to send, in order (a JSON array); empties the outbox.
    pub fn take_outbox(&mut self) -> Vec<Value> {
        for m in self.replica.take_outbox() {
            self.outbox.push(m);
        }
        if self.welcomed {
            if let Some(list) = self.viewer.as_mut().and_then(Viewer::take_visible) {
                self.ask(
                    "vis",
                    Asked::Visible,
                    json!({"type":"visible","sub":crate::viewer::SUB,"visible":list}),
                );
            }
        }
        if self.viewer.is_some() {
            self.feed_dirty = false;
        }
        self.agent_outbox();
        if self.feed_dirty && !self.feed_inflight && self.welcomed {
            self.feed_dirty = false;
            self.feed_inflight = true;
            self.ask(
                "feed",
                Asked::Feed,
                json!({"type":"query","query":{"kind":"capsules","audience":true,"limit":FEED_LIMIT}}),
            );
        }
        let out = std::mem::take(&mut self.outbox);
        self.sent.extend(out.iter().cloned());
        if self.sent.len() > 2 * SENT_KEEP {
            let old = self.sent.len() - SENT_KEEP;
            self.sent.drain(..old);
        }
        out
    }

    /// Whether nothing is outstanding (no unanswered request, nothing queued).
    #[must_use]
    pub fn idle(&self) -> bool {
        self.asked.is_empty() && self.outbox.is_empty() && !self.feed_dirty
    }

    /// A data event of the replica's own feed (`DataEvent`).
    pub fn data_event(&mut self, ev: &DataEvent) {
        match self.viewer.as_mut() {
            Some(v) => v.data_event(ev),
            None => {
                let _ = self.replica.data_event(ev);
            }
        }
        self.version += 1;
    }

    /// The host's replay state that came with a data frame.
    pub fn set_replay(&mut self, st: ReplayState) {
        self.set_calendar(&st);
        self.replay = Some(st);
        self.seen_when_due();
    }

    /// (R4/S1) The page's `?seen=` setting: `now` marks the last look at the replay clock
    /// as soon as it is known; a step number marks it when the replay first stands at or
    /// past that step (at that moment's clock). Anything else is ignored.
    pub fn seen_at(&mut self, step_or_now: &str) {
        self.seen_pending = match step_or_now.trim() {
            "now" => Some(None),
            s => match s.parse::<u64>() {
                Ok(n) => Some(Some(n)),
                Err(_) => return,
            },
        };
        self.seen_when_due();
    }

    fn seen_when_due(&mut self) {
        let (Some(want), Some(r)) = (self.seen_pending, self.replay.as_ref()) else {
            return;
        };
        if want.is_none_or(|n| r.step >= n) {
            self.seen_pending = None;
            self.mark_seen();
        }
    }

    /// (R4/H-FP) The replica's headline-piece calendar: the replay source's.
    fn set_calendar(&mut self, st: &ReplayState) {
        if self.replay.as_ref().map(|r| r.source) != Some(st.source) {
            let cal = qp_session::headparts::replay_calendar(st.source);
            if let Some(v) = self.viewer.as_mut() {
                v.set_calendar(cal.clone());
            }
            self.replica.set_headline_calendar(cal);
        }
    }

    // --- parity ----------------------------------------------------------------

    /// Digests for the shell-less parity check: the document, every face in the feed
    /// (feed order) and the open scene.
    pub fn digests(&mut self) -> Value {
        let ids: Vec<String> = self.feed.iter().map(|i| i.id.clone()).collect();
        let faces: Vec<Value> = ids
            .iter()
            .map(|id| json!({"id": id, "digest": self.face(id).map(|s| scene_digest(&s))}))
            .collect();
        let open = self.scene().map(|s| scene_digest(&s));
        if let Some(v) = &self.viewer {
            // (RP3/M15) A viewer holds no document: `document` is the open capsule's
            // slice digest (none on the feed screen); faces only those on screen.
            return json!({
                "mode": "viewer",
                "document": v.open().map(|o| o.open.digest.clone()),
                "seq": self.viewer_seq,
                "faces": faces,
                "open": {"depth": self.opening.as_ref().map_or("sentence", |o| depth_name(o.to)),
                         "capsule": self.opening.as_ref().map(|o| o.capsule.clone()),
                         "digest": open},
                "look": self.look,
                "data": v.data,
            });
        }
        json!({
            "document": self.replica.document_digest(),
            "seq": self.replica.seq(),
            "faces": faces,
            "open": {"depth": self.opening.as_ref().map_or("sentence", |o| depth_name(o.to)),
                     "capsule": self.opening.as_ref().map(|o| o.capsule.clone()),
                     "digest": open},
            "look": self.look,
        })
    }

    /// The messages sent so far: all of them, or on a long session (more than
    /// `2 × SENT_KEEP`) the latest `SENT_KEEP` or more.
    #[must_use]
    pub fn sent(&self) -> &[Value] {
        &self.sent
    }

    /// The feed (as the authority sent it).
    #[must_use]
    pub fn feed(&self) -> &[CapsuleItem] {
        &self.feed
    }

    /// The replica (read access for drivers and tests).
    pub fn replica_mut(&mut self) -> &mut Replica {
        &mut self.replica
    }

    /// A change counter (data events and notifications).
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }
}
