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
pub mod bench;
pub mod freq;
pub mod paint;
pub mod paint_canvas;
pub mod paint_gpu;

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

/// (RP3/T4) A data frame of the host's stream (`{"data":[…],"replay":…}`), typed. `msg`
/// only tells a message frame apart (it takes the message path).
#[derive(serde::Deserialize)]
struct DataFrame {
    #[serde(default)]
    msg: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    data: Option<Vec<DataEvent>>,
    #[serde(default)]
    replay: Option<ReplayState>,
}

/// (RP3/M15, §22) The host's `bind` frame (`{"bind":{feeds, events}}`), typed.
#[derive(serde::Deserialize)]
struct BindFrame {
    bind: BindBody,
}

/// (RP3/M15) The body of a `bind` frame.
#[derive(serde::Deserialize)]
struct BindBody {
    feeds: Vec<qp_schema::protocol::InitFeed>,
    #[serde(default)]
    events: Vec<DataEvent>,
}

/// (RP3/T4) The host's `start` frame (`{conn, principal, lang, data, events, …}`), typed.
#[derive(serde::Deserialize)]
struct StartFrame {
    #[serde(default)]
    principal: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    data: InitData,
    #[serde(default)]
    events: Vec<DataEvent>,
    /// (RP3/M15) `viewer`: the viewer core (no bars, no events; `instruments`).
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    instruments: Vec<String>,
}

/// (RP3/T4) The core of a page from the host's `start` frame (`{conn, principal, lang,
/// data, events}`; principal `arkin` and language `tr` when absent or not text, no events
/// when they are malformed), parsed typed in one pass; a frame that does not parse typed
/// takes the JSON-tree path, with the same result. The tree of a start frame costs some
/// 35 times its text (natively 200 MB for the 5.8 MB frame at step 1,250) and the WASM
/// memory keeps that peak (`rounds/RP3/experiments/T4.md`).
///
/// # Errors
/// A malformed frame or provider description.
pub fn core_from_start(start_json: &str) -> Result<ShellCore, String> {
    let mut core = core_from_start_parsed(start_json)?;
    core.count_frame("start", start_json.len());
    Ok(core)
}

fn core_from_start_parsed(start_json: &str) -> Result<ShellCore, String> {
    if let Ok(st) = serde_json::from_str::<StartFrame>(start_json) {
        let principal = st.principal.as_deref().unwrap_or("arkin");
        let lang = st.lang.as_deref().unwrap_or("tr");
        // (RP3/M15, §22) A viewer's start frame carries no data of the universe.
        if st.mode.as_deref() == Some("viewer") {
            return ShellCore::new_viewer(principal, lang, &st.instruments);
        }
        return ShellCore::new(principal, lang, &st.data, &st.events);
    }
    let v: Value = serde_json::from_str(start_json).map_err(|e| e.to_string())?;
    if v["mode"] == "viewer" {
        let instruments: Vec<String> =
            serde_json::from_value(v["instruments"].clone()).unwrap_or_default();
        return ShellCore::new_viewer(
            v["principal"].as_str().unwrap_or("arkin"),
            v["lang"].as_str().unwrap_or("tr"),
            &instruments,
        );
    }
    let data: InitData = serde_json::from_value(v["data"].clone()).map_err(|e| e.to_string())?;
    let events: Vec<DataEvent> = serde_json::from_value(v["events"].clone()).unwrap_or_default();
    ShellCore::new(
        v["principal"].as_str().unwrap_or("arkin"),
        v["lang"].as_str().unwrap_or("tr"),
        &data,
        &events,
    )
}

/// (RP3/T4) The record of sent messages keeps at least this many of the latest (see
/// `ShellCore::sent`); a short session (every test) keeps all of them.
pub const SENT_KEEP: usize = 1024;

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
               "p99": round3(q(0.99)), "max": round3(s[n - 1])})
    }
    /// A frame-interval summary: [`Samples::summary`] plus the share of long frames
    /// measured against the display's refresh interval `refresh_ms` (RP3/T2, GOAL 9; it
    /// replaces R4's fixed 16.7 / 33.3 ms thresholds `long_16_7` / `long_33_3`, which
    /// counted normal 60 Hz frames as long when the browser reports integer intervals
    /// such as 16 / 17 ms): `long` = share of intervals over [`LONG_FACTOR`] ×
    /// `refresh_ms` (at least one missed vsync), `very_long` = share over
    /// [`VERY_LONG_FACTOR`] × `refresh_ms` (two or more missed). The half-interval margin
    /// covers timer quantisation and jitter.
    fn frame_summary(&self, refresh_ms: f64) -> Value {
        let mut v = self.summary();
        if let Some(o) = v.as_object_mut() {
            if !self.v.is_empty() {
                #[allow(clippy::cast_precision_loss)]
                let n = self.v.len() as f64;
                #[allow(clippy::cast_precision_loss)]
                let share = |limit: f64| self.v.iter().filter(|&&x| x > limit).count() as f64 / n;
                o.insert(
                    "long".to_owned(),
                    json!(round3(share(LONG_FACTOR * refresh_ms))),
                );
                o.insert(
                    "very_long".to_owned(),
                    json!(round3(share(VERY_LONG_FACTOR * refresh_ms))),
                );
            }
        }
        v
    }

    fn median(&self) -> Option<f64> {
        if self.v.is_empty() {
            return None;
        }
        let mut s = self.v.clone();
        s.sort_by(f64::total_cmp);
        let n = s.len();
        Some(if n % 2 == 1 {
            s[n / 2]
        } else {
            f64::midpoint(s[n / 2 - 1], s[n / 2])
        })
    }
}

/// (R4/D) A frame interval this long or longer is a gap (not a frame): counted apart.
const FRAME_GAP_MS: f64 = 1000.0;

/// (RP3/T2) The refresh classes a measured frame cadence is snapped to (Hz).
pub const REFRESH_CLASSES_HZ: [f64; 4] = [60.0, 90.0, 120.0, 144.0];
/// (RP3/T2) A frame is long when its interval exceeds this many refresh intervals (one
/// missed vsync; the half interval absorbs integer-millisecond timers and jitter).
pub const LONG_FACTOR: f64 = 1.5;
/// (RP3/T2) A frame is very long beyond this many refresh intervals (two missed vsyncs).
pub const VERY_LONG_FACTOR: f64 = 2.5;
/// (RP3/T2) Fewest intervals the refresh estimate is made from; below it 60 Hz is assumed.
pub const REFRESH_MIN_SAMPLES: usize = 20;
/// (RP3/T2) An opening mark later than this (ms since navigation start) is flagged as
/// implausible for an opening time.
pub const OPEN_PLAUSIBLE_MS: f64 = 60_000.0;

/// (RP3/T2) The display's frame cadence (the `requestAnimationFrame` rhythm) estimated
/// from a frame-interval series: the median interval snapped to the nearest refresh class
/// ([`REFRESH_CLASSES_HZ`], log distance).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Refresh {
    /// The refresh class (Hz).
    pub hz: f64,
    /// Its interval (ms).
    pub ms: f64,
    /// The measured median interval (`None`: too few samples, 60 Hz assumed).
    pub median_ms: Option<f64>,
    /// Intervals the estimate is made from.
    pub n: usize,
}

impl Refresh {
    /// The estimate from `samples` (ms intervals).
    #[must_use]
    pub fn estimate(samples: &[f64]) -> Self {
        let n = samples.len();
        let median = if n >= REFRESH_MIN_SAMPLES {
            Samples {
                v: samples.to_vec(),
            }
            .median()
            .filter(|m| *m > 0.0)
        } else {
            None
        };
        let hz = median.map_or(60.0, |m| {
            let f = 1000.0 / m;
            REFRESH_CLASSES_HZ
                .iter()
                .copied()
                .min_by(|a, b| (a / f).ln().abs().total_cmp(&(b / f).ln().abs()))
                .unwrap_or(60.0)
        });
        Self {
            hz,
            ms: 1000.0 / hz,
            median_ms: median,
            n,
        }
    }

    fn json(&self, source: &str) -> Value {
        json!({"hz": self.hz, "ms": round3(self.ms), "median_ms": self.median_ms.map(round3),
               "n": self.n, "source": if self.median_ms.is_some() { source } else { "assumed" },
               "rule": format!("long: interval > {LONG_FACTOR} x refresh ms; very_long: > {VERY_LONG_FACTOR} x")})
    }
}

/// (R4/D) A gauge of the metrics page (device facts, memory): last and largest value.
#[derive(Debug, Clone, Copy, Default)]
struct Gauge {
    last: f64,
    max: f64,
}

impl Gauge {
    fn set(&mut self, x: f64) {
        self.last = x;
        if x > self.max {
            self.max = x;
        }
    }
}

/// (R4/D) The size of the WASM linear memory in bytes (`None` off the web).
fn wasm_memory_bytes() -> Option<f64> {
    #[cfg(target_arch = "wasm32")]
    {
        #[allow(clippy::cast_precision_loss)]
        Some(core::arch::wasm32::memory_size(0) as f64 * 65536.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
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
    /// (R4/D) Device facts the page reports (`cores`, `device_memory_gb`, `screen_w`,
    /// `screen_h`, `dpr`, …): last value wins.
    device: BTreeMap<String, f64>,
    /// (R4/D) Frame gaps (intervals of a second or more) by mode.
    gaps: BTreeMap<String, u64>,
    /// (R4/D) Memory samples in bytes (`js_heap` from the page, `wasm` from the core).
    memory: BTreeMap<String, Gauge>,
    /// (RP3/T2) Marks that came again after their first occurrence (a later start frame
    /// after a reconnect or a backward seek): counted, never overwriting the first.
    repeats: BTreeMap<String, u64>,
    /// (RP3/T2) The paint path: `{requested, used, fallback}` (`shells/web/paint/`).
    paint: Option<Value>,
    /// (RP3/T2) The automatic comparison (`bench`), when one runs or ran.
    bench: Option<bench::Bench>,
    /// (RP3/M14) Event counters (`data_frames`, `msg_frames`, `paint_passes`,
    /// `faces_skipped_hidden`); restart with the series.
    counts: BTreeMap<String, u64>,
    /// (RP3/M14) The JIT probe's time (ms; kept over a reset of the series).
    probe: Option<f64>,
    /// (RP3/M14) Page facts (`secure_context`, `webgpu`).
    flags: BTreeMap<String, bool>,
}

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
    pass_since: Option<f64>,
    /// (R5/T0) A data frame raised some of the waiting changes / of those the pass being
    /// painted took (the pass is then a data frame's cost, `data_pass`).
    pending_data: bool,
    pass_data: bool,
    /// (RP3/M14) The capsules whose face the page reports on screen (both modes).
    shown: BTreeSet<String>,
}

impl std::fmt::Debug for ShellCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellCore")
            .field("principal", &self.principal)
            .field("feed", &self.feed.len())
            .finish_non_exhaustive()
    }
}

/// (R5/M18, K52 measurement) `n` synthetic story rows over `bars` bars of `series`.
fn synthetic_rows(n: u64, bars: u64, series: &str) -> Vec<qp_schema::protocol::StoryRow> {
    use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
    use qp_schema::protocol::{StoryFace, StoryOutcome, StoryRow};
    let outcomes = [
        Stage::Realized,
        Stage::Invalidated,
        Stage::Expired,
        Stage::Realized,
        Stage::Retracted,
    ];
    let t = |b: u64| i64::try_from(b).unwrap_or(0);
    (0..n)
        .map(|i| {
            let born = (i * (bars - 5) / n.max(1)) % (bars - 5);
            let len = 3 + (i * 7) % 25;
            let open = born + len >= bars - 1 || i + 3 >= n;
            let stage = if open {
                Stage::Born
            } else {
                outcomes[usize::try_from(i % 5).unwrap_or(0)]
            };
            let last = if open { born + len / 2 } else { born + len };
            StoryRow {
                id: format!("bench.{i}"),
                finding: format!("bench.f{i}"),
                series: series.to_owned(),
                kind: MeaningKind::Breakout,
                direction: [Direction::Up, Direction::Down, Direction::Neutral]
                    [usize::try_from(i % 3).unwrap_or(0)],
                tier: 2,
                born_bar: born,
                born_t: t(born),
                last_bar: last,
                last_t: t(last),
                closed_bar: (!open).then_some(last),
                closed_t: (!open).then(|| t(last)),
                stage,
                outcome: StoryOutcome::of(stage),
                validity: if open {
                    Validity::Valid
                } else {
                    Validity::Closed
                },
                in_feed: true,
                hidden: None,
                coin_flip: i % 7 == 0,
                face: StoryFace {
                    event: "kırılım".to_owned(),
                    direction_word: String::new(),
                    when: String::new(),
                    moved: String::new(),
                    validity_phrase: if open { "geçerli" } else { "kapandı" }.to_owned(),
                    ledger_short: String::new(),
                    few_cases: false,
                    ledger_n: None,
                },
                redacted: false,
            }
        })
        .collect()
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

/// (R4/S1) The feed rows of `items` by layout `feed` (`qp_grammar::feeds`), with the local
/// wall times the layout reads filled from the display zone: the shell core's feed and the
/// shell-less driver use the same rows.
#[must_use]
pub fn feed_rows(
    feed: Option<&str>,
    items: &[CapsuleItem],
    lang: Language,
    seen: Option<i64>,
    fresh: &[bool],
    now: Option<i64>,
) -> Vec<qp_grammar::feeds::FeedRow> {
    let local: Vec<qp_grammar::feeds::LocalStamp> =
        items.iter().map(|it| local_stamp(it.t)).collect();
    qp_grammar::feeds::layout(
        feed,
        &qp_grammar::feeds::FeedCtx {
            items,
            lang,
            seen,
            fresh,
            now,
            local: &local,
            now_local: now.map(local_stamp),
            seen_local: seen.map(local_stamp),
        },
    )
}

/// (R4/S1) The local wall time of `t` (ms) in the display time zone, from the zone's
/// embedded rules (`qp_calendar::Zone`); UTC if the zone were missing.
#[must_use]
pub fn local_stamp(t: i64) -> qp_grammar::feeds::LocalStamp {
    let off = qp_calendar::Zone::get(DISPLAY_TZ).map_or(0, |z| z.offset_at(t.div_euclid(1000)));
    qp_grammar::feeds::LocalStamp::at_offset(t, i64::from(off) * 1000)
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

/// (R5/T3c) The `FontFace` registrations of the embedded `files` (`(id, asset)`; an id
/// without an asset is the paint path's default family at 400): each file under its own
/// CSS family and weight, and a file of another weight than 400 also under the CSS family
/// of the upright 400 file of the same published family in `assets` (if that family is a
/// different CSS family), so that family's `font-weight` uses the real file.
#[must_use]
pub fn font_faces(
    files: &[(&str, Option<&qp_grammar::look::FontAsset>)],
    assets: &[qp_grammar::look::FontAsset],
) -> Vec<Value> {
    let mut out = Vec::new();
    for (id, asset) in files {
        let Some(f) = asset else {
            out.push(
                json!({"id": id, "family": crate::shell::paint::PAINT_FONT_FAMILY,
                "weight": "400"}),
            );
            continue;
        };
        out.push(json!({"id": id, "family": f.css_family, "weight": f.weight.to_string()}));
        if f.weight != 400 && !f.italic {
            if let Some(base) = assets.iter().find(|g| {
                g.family == f.family && g.weight == 400 && !g.italic && g.css_family != f.css_family
            }) {
                out.push(json!({"id": id, "family": base.css_family,
                    "weight": f.weight.to_string()}));
            }
        }
    }
    out
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
            feed_order: None,
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
            pass_since: None,
            pending_data: false,
            pass_data: false,
            shown: BTreeSet::new(),
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

    /// (R4/S1) The `style` gesture of the presentation's defaults: feed layout, depth
    /// transition and outcome mark picked in M9–M11 (`qp_grammar::feeds::PRESENTED_FEED`,
    /// `transitions::PRESENTED_TRANSITION`, `glyph::outcome::PRESENTED_OUTCOME`) and the
    /// face (`qp_grammar::faces::PRESENTED_FACE`). (R5/S) R5's picks: the look and its
    /// companion face (M16, `look::PRESENTED_LOOK`), the voice (M17), the story chart
    /// (M18, `stories::PRESENTED_STORY`) and the open capsule design (M20,
    /// `open::PRESENTED_OPEN`). The protocol's own defaults (a bare connection) stay `r3`
    /// / `r4`.
    #[must_use]
    pub fn presented_look() -> Value {
        json!({"kind":"style",
               "face": qp_grammar::faces::PRESENTED_FACE,
               "feed": qp_grammar::feeds::PRESENTED_FEED,
               "transition": qp_grammar::transitions::PRESENTED_TRANSITION,
               "outcome": qp_grammar::glyph::outcome::PRESENTED_OUTCOME,
               // (R5/m17-int) M17's voice (`qp_session::voices::PRESENTED_VOICE`).
               "voice": qp_session::voices::PRESENTED_VOICE,
               "look": qp_grammar::look::PRESENTED_LOOK,
               "story": qp_grammar::stories::PRESENTED_STORY,
               "open": qp_grammar::open::PRESENTED_OPEN})
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

    /// One message of the authority on this connection. Returns what changed:
    /// `feed`, `scene`, `status`, `error`, `transition`.
    pub fn on_message(&mut self, msg: &ServerMsg) -> Value {
        let mut ch = json!({});
        if self.viewer.is_none() {
            self.replica.on_message(msg);
        }
        match msg {
            ServerMsg::Welcome { profile, .. } => {
                self.welcomed = true;
                self.max_points = profile.max_points;
                ch["welcome"] = json!(true);
                if self.viewer.is_some() {
                    // (RP3/M15, §22) The visibility subscription instead of the feed
                    // topic and the document: the feed comes with its faces.
                    let w = self.frames["face"].0;
                    self.push(json!({"type":"subscribe","id":crate::viewer::SUB,
                        "topic":{"kind":"viewer","limit":FEED_LIMIT,"width":w}}));
                } else {
                    self.push(
                        json!({"type":"subscribe","id":"feed-topic","topic":{"kind":"capsules"}}),
                    );
                    self.feed_dirty = true;
                }
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
                // (R4/H-M8910) The other remembered settings, only when one is set (the
                // message above stays byte-identical for the T5 settings).
                let mut more = json!({"kind":"style"});
                for (k, v) in [
                    ("feed", &self.look.feed),
                    ("transition", &self.look.transition),
                    ("outcome", &self.look.outcome),
                    ("look", &self.look.look),
                    ("voice", &self.look.voice),
                    ("story", &self.look.story),
                    ("open", &self.look.open),
                ] {
                    if let Some(v) = v {
                        more[k] = json!(v);
                    }
                }
                if self.look.hide_closed {
                    more["hide_closed"] = json!(true);
                }
                if more.as_object().is_some_and(|o| o.len() > 1) {
                    self.ask("in", Asked::Input, json!({"type":"input","gesture":more}));
                }
                ch["status"] = json!(true);
            }
            ServerMsg::Result { id, data, .. } => {
                let kind = self.asked.remove(id);
                match (kind, data) {
                    (
                        Some(Asked::Feed),
                        QueryResult::Capsules {
                            capsules,
                            coin_flip_hidden,
                            ..
                        },
                    ) => {
                        self.feed_inflight = false;
                        self.feed.clone_from(capsules);
                        self.feed_known = true;
                        self.coin_flip_hidden = *coin_flip_hidden;
                        // (RP3/T4) Only a face in the feed can be painted: the digests of
                        // the faces that left it are forgotten (bounded by the feed).
                        let feed = &self.feed;
                        self.face_shown
                            .retain(|id, _| feed.iter().any(|i| &i.id == id));
                        ch["feed"] = json!(true);
                    }
                    (Some(Asked::Input), QueryResult::Interaction { interaction }) => {
                        self.look = interaction.state.clone();
                        // (R5/m17-int, K49) A tap on a glossary term: its explanation
                        // shows (any other answer closes it).
                        let g = interaction
                            .hit
                            .as_ref()
                            .and_then(|h| h.term.clone())
                            .map(|t| (self.tapped.clone(), t));
                        if g != self.gloss {
                            self.gloss = g;
                            ch["gloss"] = json!(true);
                        }
                        // (R4/H-M8910) The face design states its height (R5/T3: in the
                        // connection's look).
                        let fh = qp_grammar::faces::height_for(
                            qp_session::viewer::look_of(&self.look),
                            self.look.face.as_deref(),
                            self.look.voice.is_some(),
                        );
                        if let Some(f) = self.frames.get_mut("face") {
                            if f.1 != fh {
                                f.1 = fh;
                                self.face_shown.clear();
                            }
                        }
                        if let Some(o) = &interaction.opening {
                            self.trans_t0 = None;
                            self.shared_from = depth_name(o.from).to_owned();
                            self.shared.clone_from(&o.shared);
                            if o.from == OpenDepth::Sentence {
                                self.open_headline = self
                                    .feed
                                    .iter()
                                    .find(|i| i.id == o.capsule)
                                    .map(|i| i.sentence.text.clone());
                                self.open_voice = self
                                    .feed
                                    .iter()
                                    .find(|i| i.id == o.capsule)
                                    .and_then(|i| i.voice.clone());
                            }
                            if o.to == OpenDepth::Sentence {
                                self.opening = None;
                                self.open_headline = None;
                                self.open_voice = None;
                            } else {
                                self.opening = Some(o.clone());
                            }
                            ch["transition"] = json!(true);
                        }
                        ch["scene"] = json!(true);
                        ch["feed"] = json!(true);
                        self.flush_gesture();
                    }
                    (Some(Asked::Visible), QueryResult::Viewer { viewer }) => {
                        if let Some(v) = self.viewer.as_mut() {
                            v.visible_answered();
                        }
                        let seq = self.viewer_seq;
                        self.apply_viewer(viewer, seq, &mut ch);
                    }
                    (Some(Asked::Stories), d @ QueryResult::Stories { .. }) => {
                        self.stories_page(d, &mut ch);
                    }
                    (Some(Asked::Replay), QueryResult::Replay { replay }) => {
                        self.set_calendar(replay);
                        self.replay = Some(replay.clone());
                        self.seen_when_due();
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
                        Some(Asked::Visible) => {
                            if let Some(v) = self.viewer.as_mut() {
                                v.visible_answered();
                            }
                        }
                        _ => {}
                    }
                }
                self.error = Some(error.message.clone());
                ch["error"] = json!(true);
            }
            // (R5/M18, §23.4) The story list's changes.
            ServerMsg::Notify { event, sub } if sub == STORIES_SUB => {
                if let Some(st) = self.viewer.as_mut().and_then(|v| v.stories.as_mut()) {
                    match event {
                        Event::Story { story, .. } => st.upsert((**story).clone()),
                        Event::ServerComment { comment, .. } => st.comments.push(comment.clone()),
                        _ => {}
                    }
                    ch["stories"] = json!(true);
                }
            }
            ServerMsg::Notify { event, .. } => match event {
                Event::Capsule { .. } | Event::Change { .. } | Event::ServerComment { .. } => {
                    self.feed_dirty = true;
                    self.version += 1;
                    ch["scene"] = json!(true);
                }
                Event::Viewer { viewer, seq } => {
                    self.viewer_seq = *seq;
                    self.apply_viewer(viewer, *seq, &mut ch);
                }
                Event::Claim { .. } => {
                    if let Some(v) = self.viewer.as_mut() {
                        v.on_message(msg);
                        self.version += 1;
                        ch["scene"] = json!(true);
                    }
                }
                _ => {}
            },
        }
        self.sync_stories(&mut ch);
        ch
    }

    /// (R5/M18, K48, §23.5) Asks for the open instrument's story list when the chart
    /// opens (chart or evidence depth with an open capsule) and drops it when the chart
    /// closes or the open capsule moves to another instrument. Nothing is asked on the
    /// feed screen or at capsule depth.
    fn sync_stories(&mut self, ch: &mut Value) {
        if !self.welcomed {
            return;
        }
        let chart = matches!(self.look.depth, OpenDepth::Chart | OpenDepth::Evidence);
        let Some(v) = self.viewer.as_mut() else {
            return;
        };
        let want = v.stories_target(chart);
        let held = v.stories_held();
        if want == held {
            return;
        }
        if held.is_some() {
            v.stories = None;
            self.ask(
                "unst",
                Asked::Unsubscribe,
                json!({"type":"unsubscribe","sub":STORIES_SUB}),
            );
        }
        if let Some((instrument, series)) = want {
            if let Some(v) = self.viewer.as_mut() {
                v.stories_asked += 1;
                v.stories = Some(Stories {
                    instrument: instrument.clone(),
                    series: series.clone(),
                    ..Stories::default()
                });
            }
            self.ask(
                "st",
                Asked::Stories,
                json!({"type":"query","query":{"kind":"stories","instrument":instrument,
                    "series":series,"limit":qp_schema::protocol::STORIES_MAX_LIMIT}}),
            );
            self.push(json!({"type":"subscribe","id":STORIES_SUB,
                "topic":{"kind":"stories","instrument":instrument,"series":series}}));
        }
        ch["stories"] = json!(true);
    }

    /// (R5/M18) A page of the story list arrived: kept if it is still the list wanted;
    /// the next page is asked for.
    fn stories_page(&mut self, data: &QueryResult, ch: &mut Value) {
        let QueryResult::Stories {
            instrument,
            series,
            stories,
            total,
            next,
            comments,
        } = data
        else {
            return;
        };
        let Some(st) = self
            .viewer
            .as_mut()
            .and_then(|v| v.stories.as_mut())
            .filter(|s| &s.instrument == instrument && &s.series == series)
        else {
            return;
        };
        for r in stories {
            st.upsert(r.clone());
        }
        st.comments.clone_from(comments);
        st.total = *total;
        st.complete = next.is_none();
        ch["stories"] = json!(true);
        if let Some(after) = next {
            if let Some(v) = self.viewer.as_mut() {
                v.stories_asked += 1;
            }
            let q = json!({"type":"query","query":{"kind":"stories","instrument":instrument,
                "series":series,"limit":qp_schema::protocol::STORIES_MAX_LIMIT,"after":after}});
            self.ask("st", Asked::Stories, q);
        }
    }

    /// (R5/M18, §23.5) The open instrument's story list as the device holds it (JSON;
    /// `null`: none — not at chart depth, or the full replica mode).
    #[must_use]
    pub fn stories_json(&self) -> Value {
        self.viewer
            .as_ref()
            .and_then(|v| v.stories.as_ref())
            .map_or(Value::Null, Stories::json)
    }

    /// (R5/M18) How many `stories` queries this device sent (0 in the full replica mode).
    #[must_use]
    pub fn stories_asked(&self) -> u64 {
        self.viewer.as_ref().map_or(0, |v| v.stories_asked)
    }

    /// (R5/M18, K52 measurement only) Replaces the held story list with `n` synthetic
    /// stories spread over the open chart's bars (deterministic: lives of 3–27 bars,
    /// outcomes in turn, the last three open, every 7th a coin flip), so that the paint
    /// budget can be measured with 5 / 50 / 200 stories in view. Returns how many rows
    /// the list now holds (0: no viewer list, e.g. the chart is not open).
    pub fn bench_stories(&mut self, n: u32) -> u32 {
        let bars = self.scene().map_or(0, |s| s.bars);
        let Some(st) = self.viewer.as_mut().and_then(|v| v.stories.as_mut()) else {
            return 0;
        };
        if bars < 8 {
            return 0;
        }
        st.rows = synthetic_rows(u64::from(n), bars, &st.series);
        st.total = n;
        self.version += 1;
        u32::try_from(st.rows.len()).unwrap_or(u32::MAX)
    }

    /// (RP3/M15) Applies a viewer update: the feed rows (the feed the rest of the core
    /// reads), the faces, the open capsule's slice.
    fn apply_viewer(&mut self, up: &qp_schema::protocol::ViewerUpdate, seq: u64, ch: &mut Value) {
        let Some(v) = self.viewer.as_mut() else {
            return;
        };
        let scene = v.apply(up, seq);
        if up.feed.is_some() {
            self.feed = v.items();
            self.feed_known = true;
            self.coin_flip_hidden = up.coin_flip_hidden;
        }
        if up.feed.is_some() || !up.faces.is_empty() || up.audience.is_some() {
            ch["feed"] = json!(true);
        }
        if scene {
            ch["scene"] = json!(true);
        }
        self.version += 1;
    }

    /// One frame of the host's stream (`{"msg":…}`, `{"data":[…],"replay":…}`); returns
    /// what changed (see [`ShellCore::on_message`]).
    ///
    /// # Errors
    /// Not JSON, or a message that is not a server message.
    pub fn on_frame(&mut self, text: &str) -> Result<Value, String> {
        let kind = if text.starts_with("{\"msg\"") {
            "msg"
        } else if text.starts_with("{\"bind\"") {
            "bind"
        } else {
            "data"
        };
        self.traffic.record(kind, text.len());
        // (R4/T6) A message frame as the host writes it (`{"msg":…}`, compact) is parsed
        // straight into the message: no intermediate JSON tree and no copy of it (the
        // document answer is megabytes). Anything else takes the general path.
        if let Some(inner) = text
            .strip_prefix("{\"msg\":")
            .and_then(|r| r.strip_suffix('}'))
        {
            if let Ok(msg) = serde_json::from_str::<ServerMsg>(inner) {
                return Ok(self.on_message(&msg));
            }
        }
        // (RP3/M15, §22) The host serves a feed the viewer now subscribes to (before the
        // data-frame path, which would take a `bind` frame for an empty data frame).
        if kind == "bind" {
            let b: BindFrame = serde_json::from_str(text).map_err(|e| e.to_string())?;
            if let Some(vw) = self.viewer.as_mut() {
                for f in b.bind.feeds {
                    let k = f.feed.clone();
                    let evs: Vec<DataEvent> = b
                        .bind
                        .events
                        .iter()
                        .filter(|e| e.feed == k)
                        .cloned()
                        .collect();
                    vw.bind(f, evs);
                }
            }
            return Ok(json!({"scene": true}));
        }
        // (RP3/T4) A data frame parsed typed in one pass: a JSON tree of a large frame (a
        // forward seek: thousands of events) costs some 20 times its text in memory, and
        // the WASM memory keeps that peak. A frame that does not parse typed (a malformed
        // event or replay state) or carries a message takes the general path below, which
        // skips what is malformed: the result is the same either way.
        if let Ok(f) = serde_json::from_str::<DataFrame>(text) {
            if f.msg.is_none() {
                let mut ch = json!({});
                if let Some(evs) = &f.data {
                    for ev in evs {
                        self.data_event(ev);
                    }
                    self.data_changed(&mut ch, evs.len());
                }
                if let Some(st) = f.replay {
                    self.set_replay(st);
                    ch["status"] = json!(true);
                }
                return Ok(ch);
            }
        }
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if let Some(m) = v.get("msg") {
            let msg: ServerMsg = serde_json::from_value(m.clone()).map_err(|e| e.to_string())?;
            return Ok(self.on_message(&msg));
        }
        let mut ch = json!({});
        if let Some(evs) = v.get("data").and_then(Value::as_array) {
            let mut n = 0;
            for e in evs {
                if let Ok(ev) = serde_json::from_value::<DataEvent>(e.clone()) {
                    self.data_event(&ev);
                    n += 1;
                }
            }
            self.data_changed(&mut ch, n);
        }
        if let Some(r) = v.get("replay") {
            if let Ok(st) = serde_json::from_value::<ReplayState>(r.clone()) {
                self.set_replay(st);
                ch["status"] = json!(true);
            }
        }
        Ok(ch)
    }

    /// (RP3/M14) What `n` data events applied by a data frame may have changed: nothing
    /// when there were none (a frame that only carries the replay state); the open scene
    /// (it reads the device's own data) and, in the full replica, the feed (its faces are
    /// compiled from the replica's data; a viewer's faces come ready from the authority).
    fn data_changed(&self, ch: &mut Value, n: usize) {
        if n == 0 {
            return;
        }
        ch["scene"] = json!(true);
        if self.viewer.is_none() {
            ch["feed"] = json!(true);
        }
    }

    // --- what to show --------------------------------------------------------

    /// The feed rows, top to bottom, by the connection's feed layout (`look.feed`,
    /// `qp_grammar::feeds`; `r3`: the authority's `capsules` order): a capsule row is
    /// `{id, target, text, scene, relevant, closed, mark?}` (R4/S1 `closed`: the story is
    /// closed, its glyph shows the outcome mark) — `text` is the reader's sentence (shown
    /// as text when there is no face scene), `mark` the layout's short mark; a layout row
    /// is `{header}` (its text, written by the layout).
    pub fn feed_json(&mut self) -> Value {
        let lang = self.lang;
        let now = self.replay.as_ref().map(|r| r.t);
        let fresh: Vec<bool> = self
            .feed
            .iter()
            .map(|it| self.look.seen.is_some_and(|s| self.known_at(it) > s))
            .collect();
        let rows = if self.feed.is_empty() && self.feed_known {
            // (R5/M19, K53) The empty feed says so, and why (the core writes it).
            qp_grammar::feeds::empty_rows(lang, self.coin_flip_hidden)
        } else if let Some(o) = &self.feed_order {
            let local: Vec<qp_grammar::feeds::LocalStamp> =
                self.feed.iter().map(|it| local_stamp(it.t)).collect();
            qp_grammar::feeds::order(
                o,
                &qp_grammar::feeds::FeedCtx {
                    items: &self.feed,
                    lang,
                    seen: self.look.seen,
                    fresh: &fresh,
                    now,
                    local: &local,
                    now_local: now.map(local_stamp),
                    seen_local: self.look.seen.map(local_stamp),
                },
            )
            .unwrap_or_default()
        } else {
            feed_rows(
                self.look.feed.as_deref(),
                &self.feed,
                lang,
                self.look.seen,
                &fresh,
                now,
            )
        };
        let out: Vec<Value> = rows
            .into_iter()
            .filter_map(|r| match r {
                qp_grammar::feeds::FeedRow::Header { text } => Some(json!({"header": text})),
                qp_grammar::feeds::FeedRow::Capsule { id, mark } => {
                    let it = self.feed.iter().find(|i| i.id == id)?;
                    // A face scene needs the reader's own bars of the capsule's series
                    // (RP3/M15: a viewer's comes ready when the reader may see them).
                    let has = match &self.viewer {
                        Some(v) => v.row(&it.id).is_some_and(|r| r.face),
                        None => self.replica.bars(&it.series).is_some_and(|b| !b.is_empty()),
                    };
                    // (R5/m17-int, A32) Without a face scene the text is the voice's text
                    // face (headline, ledger, detail; no digits for a redacted reader).
                    let text = it
                        .voice
                        .as_ref()
                        .map_or_else(|| it.sentence.text.clone(), |v| v.face.clone());
                    let mut v = json!({"id": it.id, "target": format!("face:{}", it.id),
                        "text": text, "scene": has, "relevant": it.relevant,
                        "closed": it.validity == qp_schema::capsule::Validity::Closed});
                    if let Some(m) = mark {
                        v["mark"] = json!(m);
                    }
                    // (RP3/M15, K48) The trust mark of a feed capsule: the authority's
                    // settled claim (text in the reader's language).
                    if let Some(vw) = &self.viewer {
                        v["trust"] = json!(vw.feed_trust());
                    }
                    Some(v)
                }
            })
            .collect();
        Value::Array(out)
    }

    /// (R4/H-M8910) When the reader could first know capsule `it`'s latest transition:
    /// the open time of the bar after its transition bar (the bar closed then), or — the
    /// transition being on the newest bar — `i64::MAX` (now).
    #[must_use]
    pub fn known_at(&self, it: &CapsuleItem) -> i64 {
        if let Some(v) = &self.viewer {
            return v.row(&it.id).and_then(|r| r.known_at).unwrap_or(i64::MAX);
        }
        let next = usize::try_from(it.last_bar)
            .ok()
            .and_then(|b| b.checked_add(1));
        self.replica
            .bars(&it.series)
            .and_then(|b| next.and_then(|n| b.get(n)))
            .map_or(i64::MAX, |b| b.t)
    }

    /// (R4/H-M8910) The face frame: `{css:[w,h], px:[w,h], style:{width,height}}` — the
    /// design's height (`qp_grammar::faces::height`), the backing size at the device scale
    /// and the CSS lengths (the shell sizes its canvases with these, computes nothing).
    #[must_use]
    pub fn face_size_json(&self) -> Value {
        let (w, h) = self.frames["face"];
        self.size_json(w, h)
    }

    fn size_json(&self, w: u32, h: u32) -> Value {
        json!({"css": [w, h], "px": [w * self.scale, h * self.scale],
               "style": {"width": format!("{w}px"), "height": format!("{h}px")}})
    }

    /// (R4/H-M8910) The names each setting accepts, from the registries (the settings
    /// page lists them; nothing is hard-coded in the shell): `{face, alphabet, feed,
    /// transition, outcome, look, voice}`, the default first.
    #[must_use]
    pub fn choices_json() -> Value {
        json!({
            "face": qp_grammar::faces::names(),
            "alphabet": qp_grammar::glyph::ALPHABETS,
            "feed": qp_grammar::feeds::names(),
            "transition": qp_grammar::transitions::names(),
            "outcome": qp_grammar::glyph::outcome::names(),
            "look": qp_grammar::look::names(),
            // (R5/M19) The feed orders (feed layouts) and the host's feed rules M19
            // measured, best first.
            // `layout`: no order chosen, the feed layout orders the feed.
            "feed_order": std::iter::once("layout").chain(qp_grammar::feeds::FEED_ORDERS).collect::<Vec<_>>(),
            "feed_rule": qp_capsule::feedrule::FEED_RULES.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            "voice": qp_session::voices::names(),
            "story": qp_grammar::stories::names(),
            // (R5/M20) The open capsule designs (`r4`: today's).
            "open": qp_grammar::open::names(),
        })
    }

    /// (R5/m17-int, K49) The glossary explanation to show: `{term, text, target}` (the
    /// target tapped, `face:<capsule>` or `scene`), or `null` when none is open. The page
    /// shows it as it is; the texts are the voice's (`docs/LANGUAGE.md`).
    #[must_use]
    pub fn gloss_json(&self) -> Value {
        match &self.gloss {
            Some((target, t)) => json!({"term": t.term, "text": t.text, "target": target}),
            None => Value::Null,
        }
    }

    /// (R5/m17-int) Closes the glossary explanation (the page's dismiss).
    pub fn close_gloss(&mut self) {
        self.gloss = None;
    }

    /// (R5/T3) The chrome's CSS custom properties for the connection's look and theme
    /// (`qp_grammar::look::css_vars`) as a `{name: value}` object, or `None` when they did
    /// not change since the last hand-out (the page sets them verbatim on its root
    /// element; its stylesheet holds no colour, font, size or radius of its own).
    /// (R5/T3b) Also the frames the page lays out: `--qp-phone-w`/`-h` (the phone
    /// surface), `--qp-face-w`/`-h` and `--qp-scene-w`/`-h` (a canvas's CSS size before
    /// the page sizes it from [`Self::face_size_json`] or the open scene).
    pub fn take_css(&mut self) -> Option<Value> {
        let (fw, fh) = self.frames["face"];
        let (sw, sh) = self.frames["scene"];
        let key = (
            self.look.look.clone(),
            self.look.theme.clone(),
            [fw, fh, sw, sh],
        );
        if self.css_shown.as_ref() == Some(&key) {
            return None;
        }
        let look = qp_session::viewer::look_of(&self.look);
        let mut vars = qp_grammar::look::css_vars(look, self.look.theme.as_deref());
        for (k, v) in [
            ("--qp-phone-w", PHONE_SCREEN.0),
            ("--qp-phone-h", PHONE_SCREEN.1),
            ("--qp-face-w", fw),
            ("--qp-face-h", fh),
            ("--qp-scene-w", sw),
            ("--qp-scene-h", sh),
        ] {
            vars.push((k.to_owned(), format!("{v}px")));
        }
        self.css_shown = Some(key);
        let obj: serde_json::Map<String, Value> = vars
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        Some(Value::Object(obj))
    }

    /// (R5/T3) The embedded font assets a paint path registers (`FontFace`): `[{id,
    /// family, weight}]`, `family` being the CSS family the core's font strings name and
    /// `weight` the file's CSS weight (the `FontFace` descriptor, a string). Every asset
    /// is listed, so a look change needs no new download; the bytes come from
    /// [`ShellCore::font_bytes`] (no network). (R5/T3c) A file of another weight than 400
    /// whose published family has a 400 file is listed once more under that file's CSS
    /// family with its weight ([`font_faces`]): the chrome's body family then has a real
    /// semibold face and the browser does not synthesize one.
    #[must_use]
    pub fn fonts_json() -> Value {
        let files: Vec<(&str, Option<&qp_grammar::look::FontAsset>)> = qp_render::fonts::FILES
            .iter()
            .map(|(id, _)| (*id, qp_grammar::look::font(id)))
            .collect();
        Value::Array(font_faces(&files, qp_grammar::look::FONTS))
    }

    /// (R5/T3) The bytes of embedded font asset `id` (empty if unknown).
    #[must_use]
    pub fn font_bytes(id: &str) -> Vec<u8> {
        qp_render::fonts::bytes(id)
            .map(<[u8]>::to_vec)
            .unwrap_or_default()
    }

    /// (R4/H-M8910) The device pixel ratio the page reports; the core paints at the nearest
    /// integer scale in `1..=MAX_SCALE`. Returns whether the scale changed.
    pub fn set_dpr(&mut self, dpr: f64) -> bool {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let s = if dpr.is_finite() && dpr > 0.0 {
            (dpr.round() as u32).clamp(1, MAX_SCALE)
        } else {
            1
        };
        let changed = s != self.scale;
        self.scale = s;
        if changed {
            self.face_shown.clear();
            self.scene_shown = None;
        }
        changed
    }

    /// The point budget of the connection's profile (from `welcome`).
    #[must_use]
    pub fn max_points(&self) -> u32 {
        self.max_points
    }

    /// The device scale scenes are painted at.
    #[must_use]
    pub fn scale(&self) -> u32 {
        self.scale
    }

    fn pixels(&self, scene: &SceneDesc) -> Vec<u8> {
        qp_render::render_scene_rgba_scaled(scene, self.scale).unwrap_or_default()
    }

    /// The face scene of capsule `id` at the face frame.
    #[must_use]
    pub fn face(&mut self, id: &str) -> Option<SceneDesc> {
        let (w, h) = self.frames["face"];
        if let Some(v) = &self.viewer {
            // (RP3/M15) A viewer's face is the scene the authority compiled.
            return v.face(id).cloned();
        }
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
        if !self.face_changed(id, &scene, force) {
            return Vec::new();
        }
        self.pixels(&scene)
    }

    /// The open capsule's scene.
    pub fn scene(&mut self) -> Option<SceneDesc> {
        let o = self.opening.clone()?;
        // (RP3/M14) Builds of the open scene (unchanged ones = builds − scene paints).
        *self
            .metrics
            .counts
            .entry("scene_builds".to_owned())
            .or_default() += 1;
        let look = self.look.clone();
        let mp = self.max_points;
        let head = self.open_headline.clone();
        let voice = self.open_voice.clone();
        // (RP3/S) The build's own time (`scene_build`), apart from the change check.
        let t0 = core_now();
        let lang = self.lang;
        let story = look.story.is_some() && o.to >= OpenDepth::Chart;
        // (R5/M20) An open capsule design reads the reader's language and (viewer) the
        // device's own check; today's card reads neither.
        let designed = look.open.is_some();
        let mut extra = qp_session::viewer::OpenExtra {
            lang: designed.then_some(self.lang),
            check: None,
        };
        let scene = if let Some(v) = self.viewer.as_mut() {
            // (R5/M18, §23.5) The story chart draws the list the authority sent.
            // (Taken out and put back: not copied on every build.)
            let rows = if story {
                v.stories
                    .as_mut()
                    .map(|st| std::mem::take(&mut st.rows))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            // (RP3/M15) The open capsule over the device's own data of its instrument.
            let scene = v
                .open_mut()
                .filter(|s| s.open.capsule == o.capsule)
                .and_then(|side| {
                    if designed {
                        extra.check = Some(side.trust());
                    }
                    side.replica_mut().session_mut()
                })
                .and_then(|s| {
                    open_scene_with(
                        s,
                        &o,
                        head.as_deref(),
                        voice.as_ref(),
                        &look,
                        mp,
                        (&rows, lang),
                        extra,
                    )
                });
            if let Some(st) = v.stories.as_mut() {
                if st.rows.is_empty() {
                    st.rows = rows;
                }
            }
            scene
        } else {
            let principal = self.principal.clone();
            let s = self.replica.session_mut()?;
            // (R5/M18) The full replica lists the stories itself (the same rule).
            let rows = if story {
                story_series(s, &o)
                    .map(|bs| s.story_rows(&bs, &principal, lang))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            open_scene_with(
                s,
                &o,
                head.as_deref(),
                voice.as_ref(),
                &look,
                mp,
                (&rows, lang),
                extra,
            )
        };
        let t1 = core_now();
        self.core_span("scene_build", t0, t1);
        scene
    }

    /// (RP3/S) Whether capsule `id`'s face `scene` differs from the one last handed out
    /// (always with `force`); records it. The check's time is the `change_check` span.
    pub(crate) fn face_changed(&mut self, id: &str, scene: &SceneDesc, force: bool) -> bool {
        let t0 = core_now();
        let k = shown_key(scene);
        let t1 = core_now();
        self.core_span("change_check", t0, t1);
        if !force && self.face_shown.get(id) == Some(&k) {
            return false;
        }
        self.face_shown.insert(id.to_owned(), k);
        true
    }

    /// (RP3/S) As [`ShellCore::face_changed`], for the open capsule's scene.
    pub(crate) fn scene_changed(&mut self, scene: &SceneDesc, force: bool) -> bool {
        let t0 = core_now();
        let k = shown_key(scene);
        let t1 = core_now();
        self.core_span("change_check", t0, t1);
        if !force && self.scene_shown.as_ref() == Some(&k) {
            return false;
        }
        self.scene_shown = Some(k);
        true
    }

    /// (RP3/S) Records a span of the core's own work (in wasm only: elsewhere there is no
    /// clock and nothing is recorded).
    pub(crate) fn core_span(&mut self, name: &str, t0: f64, t1: f64) {
        if cfg!(target_arch = "wasm32") {
            self.metric_span(name, t0, t1);
        }
    }

    /// Pixels of the open capsule's scene (empty when unchanged or none).
    pub fn scene_rgba(&mut self, force: bool) -> Vec<u8> {
        let Some(scene) = self.scene() else {
            self.scene_shown = None;
            return Vec::new();
        };
        if !self.scene_changed(&scene, force) {
            return Vec::new();
        }
        self.pixels(&scene)
    }

    /// The open capsule: `{depth, capsule, text, frame:[w,h], scene, px?, style?, trust?}`
    /// (`px`/`style`: the scene's backing size at the device scale and its CSS lengths;
    /// RP3/M15 `trust`: a viewer's trust mark of the open capsule, checked on its own data);
    /// depth `sentence` when nothing is open.
    pub fn open_json(&mut self) -> Value {
        let mut v = self.open_json_base();
        if v["depth"] != "sentence" {
            let state = self.viewer.as_mut().and_then(Viewer::trust);
            if let Some(t) = state {
                v["trust"] = json!(qp_session::viewer::trust_text(t, self.lang));
            }
            self.open_design_text(&mut v, state);
        }
        v
    }

    /// (R5/M20) Under an open capsule design, the open layer's text at the capsule depth:
    /// above a scene the design's (`tesir`: none, the scene writes the headline); without
    /// a scene (no data entitlement, A32) the design's text column from the feed item and
    /// the opening (the same reader rule). The scene's CSS lets the page scroll the layer
    /// over it (`touch-action: pan-y`) when the design states its own height.
    fn open_design_text(&self, v: &mut Value, state: Option<qp_session::viewer::TrustState>) {
        let Some(o) = self.opening.as_ref().filter(|o| o.to == OpenDepth::Capsule) else {
            return;
        };
        let design = qp_grammar::open::by_name(self.look.open.as_deref());
        if self.look.open.is_none() {
            return;
        }
        if o.frame.is_some() {
            if let Some(f) = design.scene_text {
                v["text"] = json!(f());
            }
            if design.height.is_some() {
                v["style"]["touchAction"] = json!("pan-y");
            }
            return;
        }
        let (Some(f), Some(it)) = (design.text, self.feed.iter().find(|i| i.id == o.capsule))
        else {
            return;
        };
        let t = qp_grammar::open::TextInput {
            lang: self.lang,
            headline: &it.sentence.text,
            voice: self.open_voice.as_ref(),
            facts: &o.presentation.facts,
            trust: it.trust,
            check: state.map(qp_session::viewer::TrustState::device_check),
            ledger: it.ledger_at_birth,
            validity: it.validity,
        };
        v["text"] = json!(f(&t));
    }

    fn open_json_base(&self) -> Value {
        match &self.opening {
            None => json!({"depth": "sentence"}),
            Some(o) => {
                let mut v = json!({
                    "depth": depth_name(o.to),
                    "capsule": o.capsule,
                    // (R5/m17-int) Under a voice the open layer's title is its headline.
                    "text": self.open_voice.as_ref().map(|v| v.headline.clone())
                        .or_else(|| self.open_headline.clone())
                        .unwrap_or_else(|| o.presentation.text.clone()),
                    "frame": o.frame,
                    "scene": o.frame.is_some(),
                });
                // (R5/M20 `nobra`) The open capsule design's own hint for its depth (absent:
                // the page's).
                if let Some(h) = o.frame.and_then(|_| {
                    qp_grammar::open::hint(self.look.open.as_deref(), o.to, self.lang)
                }) {
                    v["hint"] = json!(h);
                }
                // (R5/M20 `nobra`) Without a scene (no data of the instrument, A32) the
                // design's question cards as text, `[{key, q, a, d}]`, from the feed item.
                if o.frame.is_none() {
                    let rows = self
                        .feed
                        .iter()
                        .find(|i| i.id == o.capsule)
                        .map(|i| {
                            qp_grammar::open::text_rows(
                                self.look.open.as_deref(),
                                o.to,
                                i,
                                self.lang,
                            )
                        })
                        .unwrap_or_default();
                    if !rows.is_empty() {
                        v["asks"] = Value::Array(
                            rows.iter()
                                .map(|r| json!({"key": r.key, "q": r.q, "a": r.a, "d": r.d}))
                                .collect(),
                        );
                    }
                }
                if let Some([w, h]) = o.frame {
                    let sz = self.size_json(w, h);
                    v["px"] = sz["px"].clone();
                    v["style"] = sz["style"].clone();
                }
                v
            }
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

    /// (R5/M19) The view state as JSON with the presentation's feed order and feed rule
    /// (`feed_order`, `feed_rule`; absent: none chosen) — what the settings page shows.
    #[must_use]
    pub fn look_json(&self) -> Value {
        let mut v = serde_json::to_value(&self.look).unwrap_or(Value::Null);
        if let Some(o) = &self.feed_order {
            v["feed_order"] = json!(o);
        }
        if let Some(r) = &self.feed_rule {
            v["feed_rule"] = json!(r);
        }
        v
    }

    /// Symbols of the document's bar series (document order): the settings page's
    /// watchlist and positions.
    #[must_use]
    pub fn instruments(&self) -> Vec<String> {
        if let Some(v) = &self.viewer {
            return v.instruments.clone();
        }
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
        if let Some(v) = &self.viewer {
            return v.audience.as_ref().map_or(Value::Null, |a| {
                serde_json::to_value(a).unwrap_or(Value::Null)
            });
        }
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
    /// (R4/S1) A list may also be one comma-separated string, as the page's URL gives it
    /// (`?watch=SYN01,SYN02&long=SYN09`); (R4/S2) such a list keeps only symbols of the
    /// host's feeds, each once (a typo or a foreign symbol is dropped, not watched).
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
            match choice.get(k) {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect(),
                Some(Value::String(t)) => {
                    let mut out: Vec<String> = Vec::new();
                    for x in t.split(',').map(str::trim) {
                        if self.symbols.contains(x) && !out.iter().any(|o| o == x) {
                            out.push(x.to_owned());
                        }
                    }
                    out
                }
                _ => Vec::new(),
            }
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

    /// (R4/H-M8910) The `style` gesture from a settings object `{face?, alphabet?, theme?,
    /// feed?, transition?, outcome?}` (absent or `""`: unchanged).
    pub fn set_look(&mut self, choice: &Value) {
        let mut g = json!({"kind":"style"});
        for k in [
            "face",
            "alphabet",
            "theme",
            "feed",
            "transition",
            "outcome",
            "look",
            "voice",
            "story",
            "open",
        ] {
            if let Some(v) = choice.get(k).and_then(Value::as_str) {
                if !v.is_empty() {
                    g[k] = json!(v);
                }
            }
        }
        // (R5/M19) The feed order and the feed rule (presentation settings, not protocol
        // values): a known name is kept; `""` leaves it unchanged.
        if let Some(v) = choice.get("feed_order").and_then(Value::as_str) {
            if v == "layout" || qp_grammar::feeds::FEED_ORDERS.contains(&v) {
                self.feed_order = (v != "layout").then(|| v.to_owned());
                // The page repaints the feed at its next paint pass.
                self.pending.insert("feed".to_owned(), json!(true));
            }
        }
        if let Some(v) = choice.get("feed_rule").and_then(Value::as_str) {
            if qp_capsule::feedrule::FEED_RULES
                .iter()
                .any(|(n, _)| *n == v)
            {
                self.feed_rule = Some(v.to_owned());
            }
        }
        // (R5/M18, K54) Show or hide the closed stories of the story chart.
        if let Some(b) = choice.get("hide_closed").and_then(Value::as_bool) {
            g["hide_closed"] = json!(b);
        }
        // (R5/S) A look brings its companion face, unless the same choice names a face.
        if g.get("face").is_none() {
            if let Some(f) = g
                .get("look")
                .and_then(Value::as_str)
                .and_then(qp_grammar::look::companion_face)
            {
                g["face"] = json!(f);
            }
        }
        if g.as_object().is_some_and(|o| o.len() > 1) {
            self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
        }
    }

    /// (R5/G) How many capsules the authority's feed rule hid from the last feed answer
    /// (`coin_flip_hidden`, K53).
    #[must_use]
    pub fn coin_flip_hidden(&self) -> u32 {
        self.coin_flip_hidden
    }

    /// (R5/G) Asks for the feed again at the next outbox (a replica core: the feed items
    /// carry the connection's voice, which a `style` change alone does not resend). The
    /// gallery's painter calls it after applying a candidate's setting.
    pub fn requery_feed(&mut self) {
        if self.viewer.is_none() {
            self.feed_dirty = true;
        }
    }

    /// (R4/H-M8910) "I looked now": the `style` gesture's `seen` at the replay clock (the
    /// feed layout may mark what changed since). Nothing without a replay state.
    pub fn mark_seen(&mut self) {
        let Some(t) = self.replay.as_ref().map(|r| r.t) else {
            return;
        };
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"style","seen":t}}),
        );
    }

    /// (R4/H-M8910) Open capsule `id` from the feed to its capsule depth (the `open`
    /// gesture; what a tap on its face does, by id).
    pub fn open_capsule(&mut self, id: &str) {
        let (w, h) = self.frame_at(OpenDepth::Capsule);
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"open","capsule":id,"to":"capsule","width":w,"height":h}}),
        );
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
        let (w, h) = self.frame_at(if to == "capsule" {
            OpenDepth::Capsule
        } else {
            OpenDepth::Chart
        });
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"open","capsule":cap,"to":to,"width":w,"height":h}}),
        );
    }

    /// Back: close one depth (to the feed from the capsule).
    pub fn back(&mut self) {
        let Some(to) = self.opening.as_ref().map(|o| match o.to {
            OpenDepth::Evidence => OpenDepth::Chart,
            OpenDepth::Chart => OpenDepth::Capsule,
            _ => OpenDepth::Sentence,
        }) else {
            return;
        };
        let (w, h) = self.frame_at(to);
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
            self.frame_at(self.opening.as_ref().map_or(OpenDepth::Chart, |o| o.to))
        }
    }

    /// (R5/M20) The scene frame at open depth `d`: the surface's scene frame, except at
    /// the capsule depth under an open capsule design that states its own height
    /// (`qp_grammar::open::height_in`; the page scrolls the layer).
    fn frame_at(&self, d: OpenDepth) -> (u32, u32) {
        let (w, h) = self.frames["scene"];
        if d != OpenDepth::Capsule {
            return (w, h);
        }
        let look = qp_session::viewer::look_of(&self.look);
        qp_grammar::open::height_in(look, self.look.open.as_deref()).map_or((w, h), |oh| (w, oh))
    }

    fn tap(&mut self, target: &str, x: f64, y: f64) {
        target.clone_into(&mut self.tapped);
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
    /// (R5/M20) Returns whether the wheel was the chart's (zoom); otherwise the page lets
    /// it scroll (the open capsule's column).
    pub fn wheel(&mut self, dy: f64, x: f64) -> bool {
        if !self.chart_open() || !dy.is_finite() || dy == 0.0 {
            return false;
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
        true
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
        let k = i32::try_from(self.scale).unwrap_or(1);
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
                        "src": [b.left * k, b.top * k, (b.right - b.left).max(1) * k,
                                (b.bottom - b.top).max(1) * k]}))
                })
                .collect(),
        )
    }

    /// (R4/H-M8910) Progress (0..=1) of the last transition's animation at shell time `t`
    /// (ms, `performance.now()`); the first call after a transition starts the clock.
    pub fn transition_progress(&mut self, t: f64) -> f64 {
        let t0 = *self.trans_t0.get_or_insert(t);
        qp_grammar::transitions::progress_in(
            qp_session::viewer::look_of(&self.look),
            self.look.transition.as_deref(),
            t0,
            t,
        )
    }

    /// (R4/H-M8910) The last transition's frame at progress `p` by the connection's
    /// transition design (`qp_grammar::transitions`): `{done, scene_opacity, elements:
    /// [{id, source: "before"|"after", src:[x,y,w,h], style:{left,top,width,height,
    /// opacity}}]}` — `src` in backing px of the source canvas (device scale), `style` as
    /// CSS values; the old scene at page (`from_left`, `from_top`), the new at (`to_left`,
    /// `to_top`).
    #[must_use]
    pub fn transition_frame(
        &self,
        p: f64,
        from_left: f64,
        from_top: f64,
        to_left: f64,
        to_top: f64,
    ) -> Value {
        let d = qp_grammar::transitions::by_name(self.look.transition.as_deref());
        let p = if p.is_finite() {
            p.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let f = (d.frame)(&self.shared, (from_left, from_top), (to_left, to_top), p);
        let k = i32::try_from(self.scale).unwrap_or(1);
        let els: Vec<Value> = f
            .elements
            .iter()
            .map(|e| {
                let r = e.src;
                json!({"id": e.id,
                    "source": match e.source {
                        qp_grammar::transitions::Source::Before => "before",
                        qp_grammar::transitions::Source::After => "after",
                    },
                    "src": [r.left * k, r.top * k, (r.right - r.left).max(1) * k,
                            (r.bottom - r.top).max(1) * k],
                    "style": {"left": format!("{}px", e.rect[0]), "top": format!("{}px", e.rect[1]),
                              "width": format!("{}px", e.rect[2]), "height": format!("{}px", e.rect[3]),
                              "opacity": format!("{}", e.opacity.clamp(0.0, 1.0))}})
            })
            .collect();
        json!({"done": p >= 1.0, "scene_opacity": format!("{}", f.scene_opacity.clamp(0.0, 1.0)),
               "elements": els})
    }

    /// The last transition's shared elements (the authority's `Opening.shared`).
    #[must_use]
    pub fn shared(&self) -> &[qp_schema::protocol::SharedElement] {
        &self.shared
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
            // (R4/D) Intervals up to a second count (a saturated main thread gives 100+ ms
            // frames, which the summary must show); longer gaps (background tab, a pause
            // of the page) are only counted.
            if prev_mode == mode && dt > 0.0 {
                if dt < FRAME_GAP_MS {
                    m.frames.entry(mode.to_owned()).or_default().push(dt);
                } else {
                    *m.gaps.entry(mode.to_owned()).or_default() += 1;
                }
            }
        }
        m.last_frame = Some((mode.to_owned(), t));
    }

    /// A timed span (`tick`, `face`, `scene`, …) from `t0` to `t1` ms.
    pub fn metric_span(&mut self, name: &str, t0: f64, t1: f64) {
        // (RP3/M14) The JIT probe is a fact of the page, not a series: kept apart.
        if name == "jit_probe" {
            self.metrics.probe = Some(t1 - t0);
            return;
        }
        self.metrics
            .spans
            .entry(name.to_owned())
            .or_default()
            .push(t1 - t0);
    }

    /// A mark of the opening timeline (`wasm_ready`, `start_frame`, `connected`, `feed`,
    /// `first_capsule`) at `t` ms since navigation start. The first occurrence per page
    /// load wins (RP3/T2: the timeline survives a new core, see
    /// [`ShellCore::metric_inherit`]); a later one is only counted (`open_repeats`).
    pub fn metric_mark(&mut self, name: &str, t: f64) {
        let m = &mut self.metrics;
        if m.marks.contains_key(name) {
            *m.repeats.entry(name.to_owned()).or_default() += 1;
        } else {
            m.marks.insert(name.to_owned(), round3(t));
        }
    }

    /// (RP3/T2) Takes over the metrics of the page's previous core `prev` (a later start
    /// frame — reconnect, language change, backward seek — makes the page build a new
    /// core): the opening timeline, the series, the device facts, memory, the paint path
    /// and a running comparison carry on; `prev` is left with empty metrics.
    pub fn metric_inherit(&mut self, prev: &mut ShellCore) {
        let carry = prev.metric_detach();
        self.metric_attach(carry);
    }

    /// (RP3/S) Takes this core's metrics out (to be given to the next core with
    /// [`ShellCore::metric_attach`]) so that the page can free this core **before** it
    /// builds the next one: two replicas never live in the WASM memory at once (a peak
    /// the memory never gives back; rounds/RP3/experiments/T4-verify.md: 88 → 62 MB).
    pub fn metric_detach(&mut self) -> MetricsCarry {
        MetricsCarry(std::mem::take(&mut self.metrics))
    }

    /// (RP3/S) Takes over the metrics a previous core gave away
    /// ([`ShellCore::metric_detach`]).
    pub fn metric_attach(&mut self, carry: MetricsCarry) {
        self.metrics = carry.0;
        // The series of the new core start with its first frame.
        self.metrics.last_frame = None;
        if let Some(b) = self.metrics.bench.as_mut() {
            b.on_new_core();
        }
    }

    /// (RP3/T2) The paint path in use (`shells/web/paint/`): the one the page asked for
    /// (`?paint=`), the one it uses and why it fell back (empty: no fallback).
    pub fn set_paint(&mut self, requested: &str, used: &str, reason: &str) {
        self.metrics.paint = Some(json!({
            "requested": requested,
            "used": used,
            "fallback": if reason.is_empty() { Value::Null } else { json!(reason) },
        }));
    }

    /// (RP3/T2) The refresh estimate of the page's frames: from the `idle` series when it
    /// is long enough, else from every mode's intervals together.
    fn refresh(&self) -> (Refresh, &'static str) {
        let m = &self.metrics;
        if let Some(idle) = m.frames.get("idle") {
            if idle.v.len() >= REFRESH_MIN_SAMPLES {
                return (Refresh::estimate(&idle.v), "idle");
            }
        }
        let all: Vec<f64> = m
            .frames
            .values()
            .flat_map(|s| s.v.iter().copied())
            .collect();
        (Refresh::estimate(&all), "all")
    }

    /// (RP3/T2) Warnings a reader of the summary must see: browser zoom or fractional
    /// scaling (device pixel ratio below 1 or not an integer) and a paint scale other
    /// than the DPR (the browser resamples the scenes). `[{code, text}]`.
    #[must_use]
    pub fn warnings(&self) -> Vec<Value> {
        let mut w = Vec::new();
        w.extend(self.paint_warnings());
        let Some(&dpr) = self.metrics.device.get("dpr") else {
            w.extend(self.mode_warnings());
            return w;
        };
        if dpr < 1.0 {
            w.push(json!({"code": "dpr_below_1",
                "text": format!("DPR {dpr:.3} < 1: browser zoomed out (or a low-density screen); timings are not those of the default zoom")}));
        }
        if (dpr - dpr.round()).abs() > 1e-6 {
            w.push(json!({"code": "dpr_fraction",
                "text": format!("DPR {dpr:.3} is not an integer: browser zoom or fractional display scaling")}));
        }
        if (f64::from(self.scale) - dpr).abs() > 1e-6 {
            w.push(json!({"code": "scale_differs",
                "text": format!("paint scale {} differs from DPR {dpr:.3}: the browser resamples the scenes", self.scale)}));
        }
        w.extend(self.mode_warnings());
        w
    }

    /// (R5/T0) The paint path the page asked for was removed (K51: `cpu`); the page
    /// paints with its stand-in (`canvas`). A reader of an old address must not take the
    /// numbers for the removed path's.
    fn paint_warnings(&self) -> Vec<Value> {
        let Some(p) = &self.metrics.paint else {
            return Vec::new();
        };
        if p["fallback"].as_str() != Some("removed") {
            return Vec::new();
        }
        vec![json!({"code": "paint_removed",
            "text": format!("paint path `{}` was removed (K51); the page paints with `{}`",
                p["requested"].as_str().unwrap_or_default(),
                p["used"].as_str().unwrap_or_default())})]
    }

    /// (RP3/T2) Opening marks that cannot be opening times: later than
    /// [`OPEN_PLAUSIBLE_MS`] after navigation start, or out of order (`wasm_ready` ≤
    /// `start_frame` ≤ `connected` ≤ `feed`, `first_capsule` after `start_frame`).
    fn open_flags(&self) -> Vec<String> {
        let m = &self.metrics.marks;
        let mut f: Vec<String> = m
            .iter()
            .filter(|(_, v)| **v > OPEN_PLAUSIBLE_MS)
            .map(|(k, v)| format!("{k} {v} ms > {OPEN_PLAUSIBLE_MS} ms: not an opening time"))
            .collect();
        let order = [
            ("wasm_ready", "start_frame"),
            ("start_frame", "connected"),
            ("connected", "feed"),
            ("start_frame", "first_capsule"),
        ];
        for (a, b) in order {
            if let (Some(x), Some(y)) = (m.get(a), m.get(b)) {
                if x > y {
                    f.push(format!("{a} {x} ms after {b} {y} ms"));
                }
            }
        }
        f
    }

    /// The bundle sizes (`pkg/sizes.json`) and the user agent, for the summary.
    pub fn metric_env(&mut self, bundle: Value, agent: &str) {
        self.metrics.bundle = bundle;
        agent.clone_into(&mut self.metrics.agent);
    }

    /// (R4/D) A device fact the page reports (`cores`, `device_memory_gb`, `screen_w`,
    /// `screen_h`, …). Non-finite values are ignored.
    pub fn metric_device(&mut self, name: &str, value: f64) {
        if value.is_finite() {
            self.metrics.device.insert(name.to_owned(), value);
        }
    }

    /// (R4/D) A memory sample: the page's JS heap in bytes (`0` when the browser does not
    /// report it) and, from the core itself, the WASM linear memory.
    pub fn metric_memory(&mut self, js_heap: f64) {
        if js_heap.is_finite() && js_heap > 0.0 {
            self.metrics
                .memory
                .entry("js_heap".to_owned())
                .or_default()
                .set(js_heap);
        }
        if let Some(w) = wasm_memory_bytes() {
            self.metrics
                .memory
                .entry("wasm".to_owned())
                .or_default()
                .set(w);
        }
    }

    /// (R4/D) Starts the frame and span series over (the opening marks, the bundle, the
    /// device facts and memory stay): a measurement phase can be repeated cleanly.
    pub fn metric_reset(&mut self) {
        let m = &mut self.metrics;
        m.frames.clear();
        m.gaps.clear();
        m.spans.clear();
        m.counts.clear();
        m.last_frame = None;
    }

    /// The metrics summary (the copyable JSON).
    #[must_use]
    pub fn metrics_json(&self) -> Value {
        let m = &self.metrics;
        let (refresh, source) = self.refresh();
        let frames: serde_json::Map<String, Value> = m
            .frames
            .iter()
            .map(|(k, v)| {
                let mut f = v.frame_summary(refresh.ms);
                if let (Some(o), Some(g)) = (f.as_object_mut(), m.gaps.get(k)) {
                    o.insert("gaps_1s".to_owned(), json!(g));
                }
                (k.clone(), f)
            })
            .collect();
        let spans: serde_json::Map<String, Value> = m
            .spans
            .iter()
            .map(|(k, v)| (k.clone(), v.summary()))
            .collect();
        json!({
            "kind": "qapsule-demo-metrics",
            "refresh": refresh.json(source),
            "frame_ms": frames,
            "span_ms": spans,
            "open_ms": m.marks,
            "open_repeats": m.repeats,
            "open_flags": self.open_flags(),
            "warnings": self.warnings(),
            "mode": self.mode_json(),
            "counts": self.counts_json(),
            "paint": m.paint,
            "bundle": m.bundle,
            "agent": m.agent,
            "device": self.device_json(),
            "memory_bytes": m.memory.iter().map(|(k, g)| (k.clone(), json!({"last": g.last, "max": g.max})))
                .collect::<serde_json::Map<String, Value>>(),
            "look": self.look_effective(),
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
        // (RP3/T2) Warnings first: a reader must not miss a zoomed page.
        for w in self.warnings() {
            rows.push(json!(["uyarı", w["text"].as_str().unwrap_or_default()]));
        }
        for f in self.open_flags() {
            rows.push(json!(["uyarı.open", f]));
        }
        if let Some(p) = &m.paint {
            let used = p["used"].as_str().unwrap_or_default();
            let text = match p["fallback"].as_str() {
                Some(r) => format!(
                    "{used} (istenen {}: {r})",
                    p["requested"].as_str().unwrap_or_default()
                ),
                None => used.to_owned(),
            };
            rows.push(json!(["paint", text]));
        }
        rows.extend(self.mode_rows());
        let (refresh, source) = self.refresh();
        rows.push(json!([
            "refresh",
            match refresh.median_ms {
                Some(med) => format!(
                    "{} Hz ({:.2} ms; ortanca {med:.2} ms, {source}, n={})",
                    refresh.hz, refresh.ms, refresh.n
                ),
                None => format!("{} Hz (varsayılan; az kare)", refresh.hz),
            }
        ]));
        for (k, v) in &m.frames {
            rows.push(json!([format!("frame.{k}"), fmt(v)]));
            let f = v.frame_summary(refresh.ms);
            if let (Some(p99), Some(l)) = (f["p99"].as_f64(), f["long"].as_f64()) {
                rows.push(json!([
                    format!("frame.{k}.long"),
                    format!(
                        "p99 {p99:.2} ms · >{:.1} ms %{:.1}",
                        LONG_FACTOR * refresh.ms,
                        l * 100.0
                    )
                ]));
            }
        }
        for (k, v) in &m.spans {
            rows.push(json!([format!("span.{k}"), fmt(v)]));
        }
        for (k, v) in &m.marks {
            rows.push(json!([format!("open.{k}"), format!("{v:.1} ms")]));
        }
        for (k, n) in &m.repeats {
            rows.push(json!([
                format!("open.{k}.tekrar"),
                format!("{n} (ilk değer korunur)")
            ]));
        }
        for (k, g) in &m.memory {
            rows.push(json!([
                format!("memory.{k}"),
                format!(
                    "{:.1} MB (en çok {:.1} MB)",
                    g.last / 1_048_576.0,
                    g.max / 1_048_576.0
                )
            ]));
        }
        if let Some(o) = self.device_json().as_object() {
            for (k, v) in o {
                let text = v.as_f64().map_or_else(|| v.to_string(), |n| format!("{n}"));
                rows.push(json!([format!("device.{k}"), text]));
            }
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

    /// (R4/D) The look of the connection (`null`: not set yet, before `welcome`).
    fn look_effective(&self) -> Value {
        json!({
            "face": self.look.face,
            "feed": self.look.feed,
            "transition": self.look.transition,
            "outcome": self.look.outcome,
            "alphabet": self.look.alphabet,
        })
    }

    /// (R4/D) The device facts plus the paint scale the core chose (`scale`).
    fn device_json(&self) -> Value {
        let mut o: serde_json::Map<String, Value> = self
            .metrics
            .device
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
        o.insert("scale".to_owned(), json!(self.scale));
        Value::Object(o)
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
    /// (R5/M19, K53) The host's feed rule by name (`qp_capsule::feedrule::FEED_RULES`;
    /// default the presentation's, `PRESENTED_FEED_RULE`, with the K42 exemption); `None`:
    /// no rule (the protocol's default).
    pub feed_rule: Option<String>,
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
            feed_rule: Some(qp_capsule::feedrule::PRESENTED_FEED_RULE.to_owned()),
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
    init.feed_rule = o
        .feed_rule
        .as_ref()
        .map(|rule| qp_schema::protocol::FeedRuleInit {
            rule: rule.clone(),
            exempt_negative: true,
        });
    init
}

/// (RP3/S) A core's metrics in transit between two cores (`metric_detach` →
/// `metric_attach`); opaque.
#[derive(Debug, Default)]
pub struct MetricsCarry(Metrics);

#[cfg(test)]
mod font_tests {
    use super::*;
    use qp_grammar::look::FontAsset;

    fn asset(id: &'static str, family: &'static str, weight: u16, css: &'static str) -> FontAsset {
        FontAsset {
            id,
            family,
            weight,
            css_family: css,
            ..qp_grammar::look::FONTS[0]
        }
    }

    /// (R5/T3c) A semibold file is also the body family's `font-weight: 600` face; the
    /// embedded set lists each file with its weight.
    #[test]
    fn semibold_is_registered_under_the_body_family() {
        let assets = [
            qp_grammar::look::FONTS[0],
            asset("sans", "Some Sans", 400, "QpSans"),
            asset("sans-600", "Some Sans", 600, "QpSansSemi"),
        ];
        let files: Vec<(&str, Option<&FontAsset>)> =
            assets.iter().map(|a| (a.id, Some(a))).collect();
        let faces = font_faces(&files, &assets);
        let got: Vec<(String, String, String)> = faces
            .iter()
            .map(|f| {
                (
                    f["id"].as_str().unwrap().to_owned(),
                    f["family"].as_str().unwrap().to_owned(),
                    f["weight"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let row = |a: &str, b: &str, c: &str| (a.to_owned(), b.to_owned(), c.to_owned());
        assert_eq!(
            got,
            vec![
                row("plex-mono", "QpMono", "400"),
                row("sans", "QpSans", "400"),
                row("sans-600", "QpSansSemi", "600"),
                row("sans-600", "QpSans", "600"),
            ]
        );
        let real = ShellCore::fonts_json();
        let real = real.as_array().unwrap();
        assert!(real.len() >= qp_render::fonts::FILES.len());
        for f in real {
            assert!(f["weight"].is_string() && f["family"].is_string(), "{f}");
        }
    }
}
