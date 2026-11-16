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

use std::collections::{BTreeMap, VecDeque};

use qp_grammar::{compile, depth, CompileOptions, DataProvider, SeriesKind};
use qp_schema::capsule::GlyphSpec;
use qp_schema::data::DataEvent;
use qp_schema::protocol::{
    CapsuleItem, Event, EvidenceAccess, InitData, Language, OpenDepth, Opening, QueryResult,
    ReplayState, ServerMsg, ViewState,
};
use qp_schema::scene::SceneDesc;
use qp_schema::{Bar, Capsule, Fact, Finding, Point, View};
use qp_session::{Replica, Session};
use serde_json::{json, Value};

/// Face frame height the shell lays faces out at (reported with every face gesture; the
/// authority's own default is 140, too small for the §18.4 card M8 will replace).
pub const FACE_HEIGHT: u32 = 240;
/// Phone frame (`screen`'s surface frame for phones).
pub const PHONE_FRAME: (u32, u32) = (390, 300);
/// Capsules asked for per feed query.
pub const FEED_LIMIT: u32 = 40;
/// Pointer movement below which a press is a tap (CSS px).
pub const TAP_SLOP: f64 = 8.0;
/// Pinch factor of one wheel notch.
pub const WHEEL_FACTOR: f64 = 1.25;
/// The display time zone of the demo (BIST).
pub const DISPLAY_TZ: &str = "Europe/Istanbul";

/// FNV-1a 64 of a scene's JSON bytes (`view_digest`, §19.6).
#[must_use]
pub fn scene_digest(scene: &SceneDesc) -> String {
    let bytes = serde_json::to_vec(scene).unwrap_or_default();
    format!("{:016x}", qp_session::replica::fnv1a64(&bytes))
}

/// A data provider that speaks with the reader's sentences: the follower session's
/// provider (document, own bars, values) with the headline of each finding replaced by
/// the sentence the authority wrote for this reader (the `capsules` answer's
/// `sentence.text`: its language and entitlement).
struct Reader<'a> {
    inner: qp_session::DocProvider<'a>,
    heads: &'a BTreeMap<String, String>,
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
}

fn options(look: &ViewState, w: u32, h: u32, range: Option<(u64, u64)>) -> CompileOptions {
    let mut o = CompileOptions::new(w, h);
    o.range = range;
    o.theme.clone_from(&look.theme);
    o.alphabet.clone_from(&look.alphabet);
    o
}

/// The scene of a capsule's face in the feed (open depth `sentence`): the connection's
/// face design (`look.face`, `qp_grammar::depth::face_view`) over the replica's own data,
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
    let p = Reader {
        inner: s.provider(),
        heads: &heads,
    };
    let id = format!("capsule-{}", item.finding);
    let view = depth::face_view(look.face.as_deref(), &item.finding, max_points, &p, &id).ok()?;
    compile(&view, &p, &options(look, w, h, None)).ok()
}

/// The open capsule's view at its depth, rebuilt over the replica's current document and
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
            depth::capsule_view(&o.finding, max_points, p, &id).ok()
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
/// range and style) over the replica's own data; `None` when nothing is open or the
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
// The shell core
// ---------------------------------------------------------------------------

/// What an outstanding request was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    Feed,
    Input,
    Replay,
    Command,
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

/// Running statistics of a series of millisecond samples.
#[derive(Debug, Clone, Default)]
struct Samples {
    v: Vec<f64>,
}

impl Samples {
    const MAX: usize = 4096;
    fn push(&mut self, x: f64) {
        if x.is_finite() && x >= 0.0 {
            if self.v.len() == Self::MAX {
                self.v.remove(0);
            }
            self.v.push(x);
        }
    }
    fn summary(&self) -> Value {
        if self.v.is_empty() {
            return json!({"n": 0});
        }
        let mut s = self.v.clone();
        s.sort_by(f64::total_cmp);
        let n = s.len();
        let q = |p: f64| {
            // Nearest rank.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let k = ((p * n as f64).ceil() as usize).clamp(1, n);
            s[k - 1]
        };
        #[allow(clippy::cast_precision_loss)]
        let mean = s.iter().sum::<f64>() / n as f64;
        json!({"n": n, "mean": round3(mean), "p50": round3(q(0.5)), "p95": round3(q(0.95)),
               "max": round3(s[n - 1])})
    }
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Metrics of the metrics page (`docs/CONCEPTS.md` "Ölçüm ekranı"): raw timestamps in
/// (from `performance.now()`), statistics out.
#[derive(Debug, Clone, Default)]
struct Metrics {
    /// Frame intervals by mode (`idle`, `scroll`, `drag`).
    frames: BTreeMap<String, Samples>,
    last_frame: Option<(String, f64)>,
    /// Spans by name (`tick`, `face`, `scene`, `step`).
    spans: BTreeMap<String, Samples>,
    /// Marks of the opening timeline (ms since navigation start).
    marks: BTreeMap<String, f64>,
    bundle: Value,
    agent: String,
}

/// The core of the web shell (see the module docs).
pub struct ShellCore {
    principal: String,
    lang: Language,
    replica: Replica,
    welcomed: bool,
    max_points: u32,
    feed: Vec<CapsuleItem>,
    feed_dirty: bool,
    feed_inflight: bool,
    look: ViewState,
    opening: Option<Opening>,
    /// The open capsule's sentence (from the feed when it was opened).
    open_headline: Option<String>,
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
    /// Last digest of each face handed out (a face is redrawn only when it changed).
    face_shown: BTreeMap<String, String>,
    scene_shown: Option<String>,
    version: u64,
    /// Every message sent (for the record and the parity check).
    sent: Vec<Value>,
}

impl std::fmt::Debug for ShellCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellCore")
            .field("principal", &self.principal)
            .field("feed", &self.feed.len())
            .finish_non_exhaustive()
    }
}

fn lang_name(l: Language) -> &'static str {
    match l {
        Language::Tr => "tr",
        Language::En => "en",
    }
}

fn depth_name(d: OpenDepth) -> &'static str {
    match d {
        OpenDepth::Sentence => "sentence",
        OpenDepth::Capsule => "capsule",
        OpenDepth::Chart => "chart",
        OpenDepth::Evidence => "evidence",
    }
}

/// `YYYY-MM-DD HH:MM` of `t` in the display time zone.
#[must_use]
pub fn display_time(t: i64) -> String {
    match qp_calendar::format_display(t, DISPLAY_TZ) {
        Ok(s) if s.len() >= 16 => format!("{} {}", &s[..10], &s[11..16]),
        Ok(s) => s,
        Err(_) => t.to_string(),
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
            feed_dirty: false,
            feed_inflight: false,
            look: ViewState::default(),
            opening: None,
            open_headline: None,
            shared: Vec::new(),
            shared_from: String::new(),
            replay: None,
            outbox: Vec::new(),
            next_id: 0,
            asked: BTreeMap::new(),
            error: None,
            frames: BTreeMap::from([
                ("face".to_owned(), (PHONE_FRAME.0, FACE_HEIGHT)),
                ("scene".to_owned(), PHONE_FRAME),
            ]),
            gesture: None,
            metrics: Metrics::default(),
            face_shown: BTreeMap::new(),
            scene_shown: None,
            version: 0,
            sent: Vec::new(),
        };
        me.push(json!({"type":"hello","protocol":"qp/0","principal":principal,
            "surface":{"kind":"phone","screen":[390,844],"input":["touch"],"lang":lang_name(lang)}}));
        Ok(me)
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
        out
    }

    /// Whether nothing is outstanding (no unanswered request, nothing queued).
    #[must_use]
    pub fn idle(&self) -> bool {
        self.asked.is_empty() && self.outbox.is_empty() && !self.feed_dirty
    }

    /// A data event of the replica's own feed (`DataEvent`).
    pub fn data_event(&mut self, ev: &DataEvent) {
        let _ = self.replica.data_event(ev);
        self.version += 1;
    }

    /// The host's replay state that came with a data frame.
    pub fn set_replay(&mut self, st: ReplayState) {
        self.replay = Some(st);
    }

    /// One message of the authority on this connection. Returns what changed:
    /// `feed`, `scene`, `status`, `error`, `transition`.
    pub fn on_message(&mut self, msg: &ServerMsg) -> Value {
        let mut ch = json!({});
        self.replica.on_message(msg);
        match msg {
            ServerMsg::Welcome { profile, .. } => {
                self.welcomed = true;
                self.max_points = profile.max_points;
                ch["welcome"] = json!(true);
                self.push(
                    json!({"type":"subscribe","id":"feed-topic","topic":{"kind":"capsules"}}),
                );
                self.feed_dirty = true;
                self.ask(
                    "replay",
                    Asked::Replay,
                    json!({"type":"replay","control":{"kind":"status"}}),
                );
                if self.look.face.is_some()
                    || self.look.alphabet.is_some()
                    || self.look.theme.is_some()
                {
                    let st = json!({"kind":"style","face":self.look.face,"alphabet":self.look.alphabet,
                        "theme":self.look.theme.clone().unwrap_or_else(|| "view".to_owned())});
                    self.ask("in", Asked::Input, json!({"type":"input","gesture":st}));
                }
                ch["status"] = json!(true);
            }
            ServerMsg::Result { id, data, .. } => {
                let kind = self.asked.remove(id);
                match (kind, data) {
                    (Some(Asked::Feed), QueryResult::Capsules { capsules, .. }) => {
                        self.feed_inflight = false;
                        self.feed.clone_from(capsules);
                        ch["feed"] = json!(true);
                    }
                    (Some(Asked::Input), QueryResult::Interaction { interaction }) => {
                        self.look = interaction.state.clone();
                        if let Some(o) = &interaction.opening {
                            self.shared_from = depth_name(o.from).to_owned();
                            self.shared.clone_from(&o.shared);
                            if o.from == OpenDepth::Sentence {
                                self.open_headline = self
                                    .feed
                                    .iter()
                                    .find(|i| i.id == o.capsule)
                                    .map(|i| i.sentence.text.clone());
                            }
                            if o.to == OpenDepth::Sentence {
                                self.opening = None;
                                self.open_headline = None;
                            } else {
                                self.opening = Some(o.clone());
                            }
                            ch["transition"] = json!(true);
                        }
                        ch["scene"] = json!(true);
                        ch["feed"] = json!(true);
                        self.flush_gesture();
                    }
                    (Some(Asked::Replay), QueryResult::Replay { replay }) => {
                        self.replay = Some(replay.clone());
                        ch["status"] = json!(true);
                    }
                    (Some(Asked::Input), _) => self.flush_gesture(),
                    _ => {}
                }
            }
            ServerMsg::Ack { id, .. } => {
                if self.asked.remove(id) == Some(Asked::Command) {
                    self.feed_dirty = true;
                }
            }
            ServerMsg::Error { id, error } => {
                if let Some(id) = id {
                    match self.asked.remove(id) {
                        Some(Asked::Feed) => self.feed_inflight = false,
                        Some(Asked::Input) => self.flush_gesture(),
                        _ => {}
                    }
                }
                self.error = Some(error.message.clone());
                ch["error"] = json!(true);
            }
            ServerMsg::Notify { event, .. } => match event {
                Event::Capsule { .. } | Event::Change { .. } | Event::ServerComment { .. } => {
                    self.feed_dirty = true;
                    self.version += 1;
                    ch["scene"] = json!(true);
                }
                _ => {}
            },
        }
        ch
    }

    /// One frame of the host's stream (`{"msg":…}`, `{"data":[…],"replay":…}`); returns
    /// what changed (see [`ShellCore::on_message`]).
    ///
    /// # Errors
    /// Not JSON, or a message that is not a server message.
    pub fn on_frame(&mut self, text: &str) -> Result<Value, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if let Some(m) = v.get("msg") {
            let msg: ServerMsg = serde_json::from_value(m.clone()).map_err(|e| e.to_string())?;
            return Ok(self.on_message(&msg));
        }
        let mut ch = json!({});
        if let Some(evs) = v.get("data").and_then(Value::as_array) {
            for e in evs {
                if let Ok(ev) = serde_json::from_value::<DataEvent>(e.clone()) {
                    self.data_event(&ev);
                }
            }
            ch["scene"] = json!(true);
            ch["feed"] = json!(true);
        }
        if let Some(r) = v.get("replay") {
            if let Ok(st) = serde_json::from_value::<ReplayState>(r.clone()) {
                self.set_replay(st);
                ch["status"] = json!(true);
            }
        }
        Ok(ch)
    }

    // --- what to show --------------------------------------------------------

    /// The feed as the authority ordered it: `[{id, text, scene}]` — `text` is the
    /// reader's sentence (shown as text when there is no face scene).
    pub fn feed_json(&mut self) -> Value {
        let out: Vec<Value> = self
            .feed
            .iter()
            .map(|it| {
                // A face scene needs the reader's own bars of the capsule's series.
                let has = self.replica.bars(&it.series).is_some_and(|b| !b.is_empty());
                json!({"id": it.id, "target": format!("face:{}", it.id),
                    "text": it.sentence.text, "scene": has, "relevant": it.relevant})
            })
            .collect();
        Value::Array(out)
    }

    /// The face scene of capsule `id` at the face frame.
    #[must_use]
    pub fn face(&mut self, id: &str) -> Option<SceneDesc> {
        let (w, h) = self.frames["face"];
        let it = self.feed.iter().find(|i| i.id == id)?.clone();
        let look = self.look.clone();
        let mp = self.max_points;
        face_scene(self.replica.session_mut()?, &it, &look, mp, w, h)
    }

    /// Pixels (RGBA) of capsule `id`'s face, or empty when it has not changed since the
    /// last call (or has no scene).
    pub fn face_rgba(&mut self, id: &str, force: bool) -> Vec<u8> {
        let Some(scene) = self.face(id) else {
            return Vec::new();
        };
        let d = scene_digest(&scene);
        if !force && self.face_shown.get(id) == Some(&d) {
            return Vec::new();
        }
        self.face_shown.insert(id.to_owned(), d);
        qp_render::render_scene_rgba(&scene).unwrap_or_default()
    }

    /// The open capsule's scene.
    pub fn scene(&mut self) -> Option<SceneDesc> {
        let o = self.opening.clone()?;
        let look = self.look.clone();
        let mp = self.max_points;
        let head = self.open_headline.clone();
        open_scene(self.replica.session_mut()?, &o, head.as_deref(), &look, mp)
    }

    /// Pixels of the open capsule's scene (empty when unchanged or none).
    pub fn scene_rgba(&mut self, force: bool) -> Vec<u8> {
        let Some(scene) = self.scene() else {
            self.scene_shown = None;
            return Vec::new();
        };
        let d = scene_digest(&scene);
        if !force && self.scene_shown.as_deref() == Some(d.as_str()) {
            return Vec::new();
        }
        self.scene_shown = Some(d);
        qp_render::render_scene_rgba(&scene).unwrap_or_default()
    }

    /// The open capsule: `{depth, capsule, text, frame:[w,h], scene}`; depth `sentence`
    /// when nothing is open.
    #[must_use]
    pub fn open_json(&self) -> Value {
        match &self.opening {
            None => json!({"depth": "sentence"}),
            Some(o) => json!({
                "depth": depth_name(o.to),
                "capsule": o.capsule,
                "text": self.open_headline.clone().unwrap_or_else(|| o.presentation.text.clone()),
                "frame": o.frame,
                "scene": o.frame.is_some(),
            }),
        }
    }

    /// The evidence rows at evidence depth: `[{fact, family, values:[[name, text]],
    /// requires, rests_on}]` — values formatted here (shortest round-trip), the
    /// requirement as the authority wrote it.
    #[must_use]
    pub fn evidence_json(&self) -> Value {
        let Some(o) = self
            .opening
            .as_ref()
            .filter(|o| o.to == OpenDepth::Evidence)
        else {
            return json!([]);
        };
        Value::Array(o.evidence.iter().map(evidence_row).collect())
    }

    /// The replay state, formatted: `{time, step, speed, paused, done, synthetic, note,
    /// source, days}`.
    #[must_use]
    pub fn status_json(&self) -> Value {
        let Some(r) = &self.replay else {
            return json!({});
        };
        json!({
            "time": display_time(r.t),
            "step": format!("{} / {}", r.step, r.steps),
            "speed": format!("{}", r.speed),
            "paused": r.paused,
            "done": r.done,
            "synthetic": r.synthetic,
            "note": r.note,
            "source": format!("{:?}", r.source).to_lowercase(),
            "instruments": r.instruments.to_string(),
            "days": format!("{} / {} / {}", r.days.fine, r.days.ohlc, r.days.bar),
            "epoch": r.epoch,
        })
    }

    /// The last error message (as the authority wrote it), cleared by reading.
    pub fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }

    /// The language the connection reads in (`tr`, `en`).
    #[must_use]
    pub fn lang(&self) -> &'static str {
        lang_name(self.lang)
    }

    /// The connection's view state.
    #[must_use]
    pub fn look(&self) -> &ViewState {
        &self.look
    }

    /// Symbols of the document's bar series (document order): the settings page's
    /// watchlist and positions.
    #[must_use]
    pub fn instruments(&self) -> Vec<String> {
        let Some(doc) = self.replica.document() else {
            return Vec::new();
        };
        doc.series
            .iter()
            .filter_map(|s| s.instrument.as_ref().map(|i| i.symbol.clone()))
            .collect()
    }

    /// The reader's own audience as the document gives it to them (`null` if none).
    #[must_use]
    pub fn audience(&self) -> Value {
        self.replica
            .document()
            .and_then(|d| d.audiences.get(&self.principal))
            .map_or(Value::Null, |a| {
                serde_json::to_value(a).unwrap_or(Value::Null)
            })
    }

    // --- actions ---------------------------------------------------------------

    /// The shell lays out a frame (`face`, `scene`) at `w` × `h` CSS px.
    pub fn set_frame(&mut self, key: &str, w: u32, h: u32) {
        self.frames.insert(key.to_owned(), (w, h));
        if key == "face" {
            self.face_shown.clear();
        }
    }

    /// A replay control (`ReplayControl` JSON) as a `replay` message.
    pub fn replay_control(&mut self, control: Value) {
        self.ask(
            "replay",
            Asked::Replay,
            json!({"type":"replay","control":control}),
        );
    }

    /// Seek to a date `YYYY-MM-DD` (as the date input gives it) or a step number.
    pub fn seek(&mut self, date_or_step: &str) {
        let c = match date_or_step.parse::<u64>() {
            Ok(n) => json!({"kind":"seek","step":n}),
            Err(_) => json!({"kind":"seek","date":date_or_step}),
        };
        self.replay_control(c);
    }

    /// `set_sensitivity` (`important`, `balanced`, `all`; anything else: the group's
    /// default).
    pub fn set_sensitivity(&mut self, level: &str) {
        let s = match level {
            "important" | "balanced" | "all" => Value::String(level.to_owned()),
            _ => Value::Null,
        };
        self.ask(
            "cmd",
            Asked::Command,
            json!({"type":"command","ops":[{"op":"set_sensitivity","sensitivity":s}]}),
        );
    }

    /// `set_audience` from the settings page's choices: `{group, sensitivity?, watch:
    /// [symbol], long: [symbol], short: [symbol]}` (a position is one unit long or short).
    pub fn set_audience(&mut self, choice: &Value) {
        let group = choice
            .get("group")
            .and_then(Value::as_str)
            .filter(|g| *g == "future")
            .unwrap_or("habituated");
        let mut aud = json!({"group": group});
        if let Some(s) = choice.get("sensitivity").and_then(Value::as_str) {
            if ["important", "balanced", "all"].contains(&s) {
                aud["sensitivity"] = json!(s);
            }
        }
        let list = |k: &str| -> Vec<String> {
            choice
                .get(k)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        let watch = list("watch");
        if !watch.is_empty() {
            aud["watchlist"] = json!(watch);
        }
        let mut pos = serde_json::Map::new();
        for s in list("long") {
            pos.insert(s, json!(1.0));
        }
        for s in list("short") {
            pos.insert(s, json!(-1.0));
        }
        if !pos.is_empty() {
            aud["positions"] = Value::Object(pos);
        }
        self.ask(
            "cmd",
            Asked::Command,
            json!({"type":"command","ops":[{"op":"set_audience","audience":aud}]}),
        );
    }

    /// The `style` gesture: face design, alphabet, theme (`""`: unchanged).
    pub fn set_style(&mut self, face: &str, alphabet: &str, theme: &str) {
        let mut g = json!({"kind":"style"});
        for (k, v) in [("face", face), ("alphabet", alphabet), ("theme", theme)] {
            if !v.is_empty() {
                g[k] = json!(v);
            }
        }
        self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
    }

    /// Open the open capsule straight to `to` (`chart`, `evidence`): the way down when the
    /// reader gets no scene to tap (no data entitlement).
    pub fn open_to(&mut self, to: &str) {
        let Some(cap) = self.opening.as_ref().map(|o| o.capsule.clone()) else {
            return;
        };
        if !["capsule", "chart", "evidence"].contains(&to) {
            return;
        }
        let (w, h) = self.frames["scene"];
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"open","capsule":cap,"to":to,"width":w,"height":h}}),
        );
    }

    /// Back: close one depth (to the feed from the capsule).
    pub fn back(&mut self) {
        if self.opening.is_none() {
            return;
        }
        let (w, h) = self.frames["scene"];
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"close","width":w,"height":h}}),
        );
    }

    fn frame_of(&self, target: &str) -> (u32, u32) {
        if target.starts_with("face:") {
            self.frames["face"]
        } else {
            self.frames["scene"]
        }
    }

    fn tap(&mut self, target: &str, x: f64, y: f64) {
        let (w, h) = self.frame_of(target);
        let mut g = json!({"kind":"tap_at","x":x,"y":y,"width":w,"height":h});
        if let Some(cap) = target.strip_prefix("face:") {
            g["target"] = json!({"capsule": cap});
        }
        self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
    }

    fn chart_open(&self) -> bool {
        self.opening
            .as_ref()
            .is_some_and(|o| o.to >= OpenDepth::Chart && o.frame.is_some())
    }

    /// Sends the movement accumulated while an input was in flight.
    fn flush_gesture(&mut self) {
        if self.input_in_flight() {
            return;
        }
        let Some(g) = self.gesture.as_mut() else {
            return;
        };
        let (dx, f, x) = (g.acc_dx, g.acc_factor, g.acc_x.take());
        g.acc_dx = 0.0;
        g.acc_factor = 1.0;
        if dx != 0.0 {
            self.send_drag(dx);
        } else if f != 1.0 && f > 0.0 {
            self.send_pinch(f, x);
        }
    }

    fn send_drag(&mut self, dx: f64) {
        let (w, h) = self.frames["scene"];
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"drag","dx":dx,"width":w,"height":h}}),
        );
    }

    fn send_pinch(&mut self, factor: f64, x: Option<f64>) {
        let (w, h) = self.frames["scene"];
        let mut g = json!({"kind":"pinch","factor":factor,"width":w,"height":h});
        if let Some(x) = x {
            g["x"] = json!(x);
        }
        self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
    }

    /// A raw pointer event on `target` (`face:<capsule>` or `scene`): `phase` `down`,
    /// `move`, `up` or `cancel`; `x`, `y` in CSS px inside the target. The recognizer
    /// (here, not in the shell) turns presses into `tap_at`, horizontal movement on an
    /// open chart into `drag`, two pointers into `pinch`; movement is coalesced while an
    /// input is in flight. Returns the gesture mode (`tap`, `drag`, `pinch`, `none`).
    pub fn pointer(&mut self, target: &str, phase: &str, id: i32, x: f64, y: f64) -> String {
        if !(x.is_finite() && y.is_finite()) {
            return "none".to_owned();
        }
        let chart = target == "scene" && self.chart_open();
        match phase {
            "down" => {
                let g = self.gesture.get_or_insert_with(|| Gesture {
                    target: target.to_owned(),
                    acc_factor: 1.0,
                    ..Gesture::default()
                });
                if g.target != target {
                    return "none".to_owned();
                }
                g.ptrs.retain(|p| p.0 != id);
                g.ptrs.push((id, x, y, x, y));
                if g.ptrs.len() == 2 && chart {
                    g.pinching = true;
                    g.dragging = false;
                    g.last_d = dist(&g.ptrs);
                }
                "none".to_owned()
            }
            "move" => {
                let Some(g) = self.gesture.as_mut() else {
                    return "none".to_owned();
                };
                let Some(p) = g.ptrs.iter_mut().find(|p| p.0 == id) else {
                    return "none".to_owned();
                };
                p.3 = x;
                p.4 = y;
                let (x0, y0) = (p.1, p.2);
                if g.pinching && g.ptrs.len() == 2 {
                    let d = dist(&g.ptrs);
                    if g.last_d > 0.0 && d > 0.0 {
                        g.acc_factor *= d / g.last_d;
                        g.acc_x = Some(((g.ptrs[0].3 + g.ptrs[1].3) / 2.0).round());
                        g.last_d = d;
                    }
                    self.flush_gesture();
                    return "pinch".to_owned();
                }
                let moved = (x - x0).abs().max((y - y0).abs());
                if !g.dragging && moved > TAP_SLOP {
                    if chart && (x - x0).abs() >= (y - y0).abs() {
                        g.dragging = true;
                        g.last_x = x0;
                    } else {
                        g.cancelled = true;
                    }
                }
                if g.dragging {
                    g.acc_dx += x - g.last_x;
                    g.last_x = x;
                    self.flush_gesture();
                    return "drag".to_owned();
                }
                "none".to_owned()
            }
            "up" | "cancel" => {
                let Some(mut g) = self.gesture.take() else {
                    return "none".to_owned();
                };
                let p = g.ptrs.iter().find(|p| p.0 == id).copied();
                g.ptrs.retain(|p| p.0 != id);
                let mode = if g.pinching {
                    "pinch"
                } else if g.dragging {
                    "drag"
                } else {
                    "tap"
                };
                if !g.ptrs.is_empty() {
                    // One finger of a pinch lifted: the gesture goes on with the other.
                    self.gesture = Some(g);
                    return mode.to_owned();
                }
                if mode == "tap" && phase == "up" && !g.cancelled {
                    if let Some((_, x0, y0, _, _)) = p {
                        self.tap(target, x0, y0);
                    }
                } else if g.acc_dx != 0.0 || g.acc_factor != 1.0 {
                    // Flush what is left once the in-flight input answers.
                    self.gesture = Some(Gesture {
                        target: g.target.clone(),
                        acc_dx: g.acc_dx,
                        acc_factor: g.acc_factor,
                        acc_x: g.acc_x,
                        ..Gesture::default()
                    });
                    self.flush_gesture();
                    if self
                        .gesture
                        .as_ref()
                        .is_some_and(|g| g.acc_dx == 0.0 && g.acc_factor == 1.0)
                    {
                        self.gesture = None;
                    }
                }
                mode.to_owned()
            }
            _ => "none".to_owned(),
        }
    }

    /// A wheel notch on the open chart (`dy` < 0: zoom in) at column `x`.
    pub fn wheel(&mut self, dy: f64, x: f64) {
        if !self.chart_open() || !dy.is_finite() || dy == 0.0 {
            return;
        }
        let f = if dy < 0.0 {
            WHEEL_FACTOR
        } else {
            1.0 / WHEEL_FACTOR
        };
        if self.input_in_flight() {
            let g = self.gesture.get_or_insert_with(|| Gesture {
                target: "scene".to_owned(),
                acc_factor: 1.0,
                ..Gesture::default()
            });
            g.acc_factor *= f;
            g.acc_x = Some(x);
        } else {
            self.send_pinch(f, Some(x));
        }
    }

    // --- transition ------------------------------------------------------------

    /// The last transition's moving elements in page coordinates: the before boxes are
    /// inside the element at (`from_left`, `from_top`), the after boxes inside the one
    /// at (`to_left`, `to_top`). `[{id, from:{left,top,width,height}, to:{…}, src:[x, y,
    /// w, h]}]` — `from`/`to` as CSS lengths, `src` the before box inside the old canvas
    /// (what to copy).
    #[must_use]
    pub fn transition_json(
        &self,
        from_left: f64,
        from_top: f64,
        to_left: f64,
        to_top: f64,
    ) -> Value {
        let css = |r: &qp_schema::scene::PixelRect, l: f64, t: f64| {
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            json!({"left": format!("{}px", f64::from(r.left) + l),
                   "top": format!("{}px", f64::from(r.top) + t),
                   "width": format!("{}px", w.max(1)), "height": format!("{}px", h.max(1))})
        };
        Value::Array(
            self.shared
                .iter()
                .filter_map(|s| {
                    let (b, a) = (s.before.as_ref()?, s.after.as_ref()?);
                    Some(json!({"id": s.id, "from": css(b, from_left, from_top),
                        "to": css(a, to_left, to_top),
                        "src": [b.left, b.top, (b.right - b.left).max(1), (b.bottom - b.top).max(1)]}))
                })
                .collect(),
        )
    }

    /// Where the last transition started (`sentence`: from a face in the feed).
    #[must_use]
    pub fn transition_from(&self) -> &str {
        &self.shared_from
    }

    // --- metrics ---------------------------------------------------------------

    /// An animation frame at `t` ms in mode `idle`, `scroll` or `drag`.
    pub fn metric_frame(&mut self, mode: &str, t: f64) {
        let m = &mut self.metrics;
        if let Some((prev_mode, prev)) = &m.last_frame {
            let dt = t - prev;
            if prev_mode == mode && dt > 0.0 && dt < 250.0 {
                m.frames.entry(mode.to_owned()).or_default().push(dt);
            }
        }
        m.last_frame = Some((mode.to_owned(), t));
    }

    /// A timed span (`tick`, `face`, `scene`, …) from `t0` to `t1` ms.
    pub fn metric_span(&mut self, name: &str, t0: f64, t1: f64) {
        self.metrics
            .spans
            .entry(name.to_owned())
            .or_default()
            .push(t1 - t0);
    }

    /// A mark of the opening timeline (`wasm_ready`, `connected`, `first_capsule`) at
    /// `t` ms since navigation start (first one wins).
    pub fn metric_mark(&mut self, name: &str, t: f64) {
        self.metrics
            .marks
            .entry(name.to_owned())
            .or_insert(round3(t));
    }

    /// The bundle sizes (`pkg/sizes.json`) and the user agent, for the summary.
    pub fn metric_env(&mut self, bundle: Value, agent: &str) {
        self.metrics.bundle = bundle;
        agent.clone_into(&mut self.metrics.agent);
    }

    /// The metrics summary (the copyable JSON).
    #[must_use]
    pub fn metrics_json(&self) -> Value {
        let m = &self.metrics;
        let frames: serde_json::Map<String, Value> = m
            .frames
            .iter()
            .map(|(k, v)| (k.clone(), v.summary()))
            .collect();
        let spans: serde_json::Map<String, Value> = m
            .spans
            .iter()
            .map(|(k, v)| (k.clone(), v.summary()))
            .collect();
        json!({
            "kind": "qapsule-demo-metrics",
            "frame_ms": frames,
            "span_ms": spans,
            "open_ms": m.marks,
            "bundle": m.bundle,
            "agent": m.agent,
            "feed": self.feed.len(),
            "replay": self.replay.as_ref().map(|r| json!({"step": r.step, "instruments": r.instruments,
                "synthetic": r.synthetic, "speed": r.speed})),
        })
    }

    /// The metrics as text lines `[[label, value]]` for the page (values formatted
    /// here).
    #[must_use]
    pub fn metrics_rows(&self) -> Value {
        let m = &self.metrics;
        let mut rows = Vec::new();
        let fmt = |s: &Samples| {
            let v = s.summary();
            if v["n"] == 0 {
                "—".to_owned()
            } else {
                format!(
                    "n={} ort {:.2} · p50 {:.2} · p95 {:.2} ms",
                    v["n"],
                    v["mean"].as_f64().unwrap_or(0.0),
                    v["p50"].as_f64().unwrap_or(0.0),
                    v["p95"].as_f64().unwrap_or(0.0)
                )
            }
        };
        for (k, v) in &m.frames {
            rows.push(json!([format!("frame.{k}"), fmt(v)]));
        }
        for (k, v) in &m.spans {
            rows.push(json!([format!("span.{k}"), fmt(v)]));
        }
        for (k, v) in &m.marks {
            rows.push(json!([format!("open.{k}"), format!("{v:.1} ms")]));
        }
        if let Some(o) = m.bundle.as_object() {
            for (k, v) in o {
                let n = v.as_f64().unwrap_or(0.0);
                rows.push(json!([
                    format!("bundle.{k}"),
                    format!("{:.1} KB", n / 1024.0)
                ]));
            }
        }
        Value::Array(rows)
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

    /// Every message sent so far.
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

fn dist(p: &[(i32, f64, f64, f64, f64)]) -> f64 {
    let dx = p[0].3 - p[1].3;
    let dy = p[0].4 - p[1].4;
    (dx * dx + dy * dy).sqrt()
}

fn evidence_row(e: &EvidenceAccess) -> Value {
    let values: Vec<Value> = e
        .values
        .iter()
        // Two decimals, the grammar's value-label convention (`docs/GRAMMAR.md` §14).
        .map(|(k, v)| json!([k, format!("{v:.2}")]))
        .collect();
    json!({
        "fact": e.fact,
        "family": format!("{:?}", e.family).to_lowercase(),
        "values": values,
        "requires": e.requires,
        "rests_on": e.rests_on,
    })
}

/// Queue of frames (used by drivers that hold frames between steps).
pub type Frames = VecDeque<String>;

// ---------------------------------------------------------------------------
// The demo session
// ---------------------------------------------------------------------------

/// Principal of the demo whose data entitlement is withheld ("no data entitlement"
/// mode of the settings page; an owner, so it may still drive the replay).
pub const VISITOR: &str = "visitor";

/// What `qp demo` starts (host configuration, not protocol).
#[derive(Debug, Clone)]
pub struct DemoOptions {
    /// Local data (falls back to the synthetic market with a note) or synthetic.
    pub local: bool,
    /// Instruments.
    pub instruments: u32,
    /// Bars of history before the first replayed bar.
    pub history: u32,
    /// Synthetic trading days.
    pub days: u32,
    /// Steps per wall second.
    pub speed: f64,
    /// Start of the replay (ms), if given.
    pub from_t: Option<i64>,
    /// Start running (else paused).
    pub running: bool,
}

impl Default for DemoOptions {
    fn default() -> Self {
        Self {
            local: false,
            instruments: 30,
            history: 120,
            days: 260,
            speed: 4.0,
            from_t: None,
            running: false,
        }
    }
}

/// The demo's session init: `qp_session::demo_init` (owner `arkin`, host, agent `scout`,
/// viewer `guest`) plus the owner [`VISITOR`] without data entitlement; every feed bound,
/// the capsule engine on all of them; daily bars moved by hourly bars.
#[must_use]
pub fn demo_session_init(o: &DemoOptions) -> qp_schema::protocol::SessionInit {
    use qp_schema::protocol::{ReplayInit, ReplaySourceKind};
    let mut ent = BTreeMap::new();
    for p in ["host", "arkin", "scout", "guest"] {
        ent.insert(p.to_owned(), vec!["*".to_owned()]);
    }
    ent.insert(VISITOR.to_owned(), Vec::new());
    let mut init = qp_session::demo_init(ReplayInit {
        source: if o.local {
            ReplaySourceKind::Local
        } else {
            ReplaySourceKind::Synthetic
        },
        instruments: Some(o.instruments),
        days: Some(o.days),
        history: Some(o.history),
        fine: Some("1h".to_owned()),
        from_t: o.from_t,
        speed: Some(o.speed),
        running: o.running,
        entitlements: ent,
        ..ReplayInit::default()
    });
    let visitor: qp_schema::protocol::Principal = serde_json::from_value(json!(
        {"id": VISITOR, "author": {"kind": "human", "name": VISITOR}, "role": "owner"}))
    .expect("visitor principal");
    init.principals.push(visitor);
    init
}
