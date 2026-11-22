//! (RP3/T2, GOAL 8) The automatic paint-path comparison of the metrics page.
//!
//! One tap starts it; no manual input follows. Every paint path the page names (those
//! registered and supported, `shells/web/paint/`) runs in turn through the same
//! scenario, and one comparative summary comes out. The scenario driver lives here,
//! because the shell does no arithmetic: the page calls [`ShellCore::bench_tick`] once
//! per animation frame with the raw timestamp and the feed's scroll extent, and applies
//! the returned actions verbatim (`paint`: switch to that path, `tab`: show that page,
//! `scroll`: the feed's `scrollTop`, `done`). Replay control, opening the capsule and the
//! synthetic drag (pointer events into the core's own gesture recognizer) are done here
//! directly, through the same methods the page's controls use.
//!
//! Per path:
//! 1. `prepare` — switch the path, close an open capsule, pause, fixed speed
//!    ([`BENCH_SPEED`]), seek to the fixed step ([`BENCH_SEEK_STEP`]); wait until the
//!    core is idle at that step with a feed (stable for [`SETTLE_MS`]).
//! 2. `idle` — replay running, feed at the top, nothing touched ([`BENCH_PHASE_MS`]).
//! 3. `scroll` — replay paused, the feed scrolled up and down (a triangle wave of
//!    [`SCROLL_AMPLITUDE_PX`] over [`SCROLL_PERIOD_MS`]).
//! 4. `chart_ticks` — the first capsule opened to its chart (not measured), then the
//!    replay running with the chart open at its default range (which follows new bars).
//! 5. `drag` — replay paused, the chart zoomed in by [`BENCH_ZOOM_NOTCHES`] wheel notches
//!    (not measured; unzoomed, every bar is shown and a drag is clamped), then horizontal
//!    drag sweeps of [`DRAG_SWEEP_MS`] across the middle half of the chart in
//!    [`DRAG_STEP_PX`] steps.
//!
//! (The task listed drag before ticks-with-chart; they run the other way round because a
//! zoomed or panned range no longer follows the new bars, so ticks would hardly repaint.)
//!
//! Each measured phase keeps its frame intervals and the page's spans (`tick`, `face`,
//! `scene`); the page's own series are consumed (they restart after the comparison).

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use super::{
    round3, wasm_memory_bytes, OpenDepth, Refresh, Samples, ShellCore, FRAME_GAP_MS, LONG_FACTOR,
};

/// Length of one measured phase (ms; 4 phases per path).
pub const BENCH_PHASE_MS: f64 = 8000.0;
/// Shortest and longest phase a configuration may ask for (ms).
pub const BENCH_PHASE_RANGE: (f64, f64) = (1000.0, 20_000.0);
/// The replay step every path starts from (the screenshot moment of `screens.mjs`).
pub const BENCH_SEEK_STEP: u64 = 120;
/// The replay speed of every path (steps per second; the speed of Arkin's phone run).
pub const BENCH_SPEED: f64 = 4.0;
/// Longest wait for a non-measured stage (prepare, opening) before it is given up (ms).
pub const BENCH_WAIT_MS: f64 = 10_000.0;
/// The comparison ends by this time whatever is left (ms; GOAL 8: at most 3 minutes).
pub const BENCH_DEADLINE_MS: f64 = 175_000.0;
/// A waiting stage's condition must hold this long (ms).
pub const SETTLE_MS: f64 = 400.0;
/// Feed scroll: one down-and-up period (ms; at most one phase) and the largest amplitude
/// (CSS px).
pub const SCROLL_PERIOD_MS: f64 = 4000.0;
pub const SCROLL_AMPLITUDE_PX: f64 = 2000.0;
/// Chart drag: one sweep (ms), across the middle half of the chart's width.
pub const DRAG_SWEEP_MS: f64 = 1000.0;
/// Chart drag: the pointer moves in steps of this many CSS px. (R4's drag turns each
/// input into `round(dx / slot)` bars and drops the remainder, so the 1–2 px moves of a
/// 120 Hz display against a fast host move nothing; `rounds/RP3/experiments/T2.md`.)
pub const DRAG_STEP_PX: f64 = 12.0;
/// Wheel notches of zoom-in before the drag phase (each `WHEEL_FACTOR`).
pub const BENCH_ZOOM_NOTCHES: u32 = 4;
/// The measured phases, in order.
pub const PHASES: [&str; 4] = ["idle", "scroll", "chart_ticks", "drag"];
/// The spans the summary reports per path ((RP3/M14) `paint_pass`: one coalesced paint
/// pass; `frame_to_paint`: from a frame's arrival to the end of the pass that painted it).
pub const SPANS: [&str; 5] = ["tick", "face", "scene", "paint_pass", "frame_to_paint"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Switch,
    Prepare,
    Idle,
    Scroll,
    Open,
    Chart,
    Zoom,
    Drag,
    ChartTicks,
    Done,
}

impl Stage {
    fn phase(self) -> Option<&'static str> {
        match self {
            Self::Idle => Some(PHASES[0]),
            Self::Scroll => Some(PHASES[1]),
            Self::ChartTicks => Some(PHASES[2]),
            Self::Drag => Some(PHASES[3]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Phase {
    frames: Samples,
    spans: BTreeMap<String, Samples>,
    /// (RP3/M14) The page's counters during the phase.
    counts: BTreeMap<String, u64>,
    ms: f64,
    steps: [Option<u64>; 2],
}

#[derive(Debug, Clone, Default)]
struct PathRun {
    name: String,
    phases: BTreeMap<String, Phase>,
    memory: BTreeMap<String, f64>,
    errors: Vec<String>,
    /// The page's `set_paint` record while this path ran (`{requested, used, fallback}`).
    paint: Value,
    t0: f64,
    ms: f64,
}

/// The state of a comparison (kept with the metrics, so that it survives a new core
/// after a backward seek: [`ShellCore::metric_inherit`]).
#[derive(Debug, Clone)]
pub struct Bench {
    paths: Vec<String>,
    restore: String,
    phase_ms: f64,
    t_start: f64,
    stage: Stage,
    stage_t0: f64,
    stable_since: Option<f64>,
    last_t: Option<f64>,
    last_back: f64,
    path_i: usize,
    runs: Vec<PathRun>,
    cur: PathRun,
    phase: Phase,
    /// The drag sweep in progress: (sweep index, last x).
    sweep: Option<(u64, f64)>,
    summary: Option<Value>,
    rows: Value,
}

fn tri(u: f64) -> f64 {
    // 0 → 1 → 0 over one period.
    let f = u.fract();
    if f < 0.5 {
        2.0 * f
    } else {
        2.0 - 2.0 * f
    }
}

fn mb(x: f64) -> String {
    format!("{:.1} MB", x / 1_048_576.0)
}

impl Bench {
    fn new(cfg: &Value, t: f64) -> Self {
        let mut paths: Vec<String> = cfg["paths"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        // (RP3/S) The page's `?bench_paths=a,b` (raw) keeps only those of the supported
        // paths, in registry order; `?bench_order=reverse` runs them reversed (the order
        // effect: later paths start with a warm JIT and a grown WASM memory).
        let only: Vec<&str> = cfg["only"]
            .as_str()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !only.is_empty() {
            paths.retain(|p| only.contains(&p.as_str()));
        }
        if cfg["order"].as_str() == Some("reverse") {
            paths.reverse();
        }
        if paths.is_empty() {
            paths.push("ref".to_owned());
        }
        let phase_ms = match &cfg["phase_ms"] {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
        .filter(|x| x.is_finite())
        .map_or(BENCH_PHASE_MS, |x| {
            x.clamp(BENCH_PHASE_RANGE.0, BENCH_PHASE_RANGE.1)
        });
        let restore = cfg["current"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("ref")
            .to_owned();
        Self {
            paths,
            restore,
            phase_ms,
            t_start: t,
            stage: Stage::Switch,
            stage_t0: t,
            stable_since: None,
            last_t: None,
            last_back: f64::NEG_INFINITY,
            path_i: 0,
            runs: Vec::new(),
            cur: PathRun::default(),
            phase: Phase::default(),
            sweep: None,
            summary: None,
            rows: Value::Null,
        }
    }

    fn enter(&mut self, s: Stage, t: f64) {
        self.stage = s;
        self.stage_t0 = t;
        self.stable_since = None;
    }

    fn stable(&mut self, cond: bool, t: f64) -> bool {
        if cond {
            let s = *self.stable_since.get_or_insert(t);
            t - s >= SETTLE_MS
        } else {
            self.stable_since = None;
            false
        }
    }

    /// A new core took over (a backward seek): a waiting stage waits again.
    pub(super) fn on_new_core(&mut self) {
        self.stable_since = None;
        self.last_t = None;
        if let Some(p) = self.stage.phase() {
            self.cur
                .errors
                .push(format!("{p}: the page built a new core during the phase"));
        }
    }

    /// Whether the comparison has finished.
    #[must_use]
    pub fn done(&self) -> bool {
        self.summary.is_some()
    }
}

impl ShellCore {
    /// (RP3/T2) Starts the comparison at `t` (ms) with `config` `{paths: [names],
    /// current: name, phase_ms, only?, order?}` (phase length optional: [`BENCH_PHASE_MS`],
    /// clamped to [`BENCH_PHASE_RANGE`]; RP3/S `only`: the page's raw `?bench_paths=`,
    /// `order`: `reverse` runs the paths reversed).
    pub fn bench_start(&mut self, config: &Value, t: f64) {
        self.metrics.bench = Some(Bench::new(config, t));
    }

    fn step_now(&self) -> Option<u64> {
        self.replay.as_ref().map(|r| r.step)
    }

    fn begin_phase(&mut self, b: &mut Bench, s: Stage, t: f64) {
        self.metrics.spans.clear();
        self.metrics.counts.clear();
        b.phase = Phase {
            steps: [self.step_now(), None],
            ..Phase::default()
        };
        b.enter(s, t);
    }

    fn end_phase(&mut self, b: &mut Bench, t: f64) {
        let Some(name) = b.stage.phase() else {
            return;
        };
        let mut p = std::mem::take(&mut b.phase);
        p.spans = std::mem::take(&mut self.metrics.spans);
        p.counts = std::mem::take(&mut self.metrics.counts);
        p.ms = round3(t - b.stage_t0);
        p.steps[1] = self.step_now();
        b.cur.phases.insert(name.to_owned(), p);
    }

    fn replay_kind(&mut self, kind: &str) {
        self.replay_control(json!({"kind": kind}));
    }

    fn finish_path(&mut self, b: &mut Bench, t: f64) {
        if let Some(w) = wasm_memory_bytes() {
            b.cur.memory.insert("wasm".to_owned(), w);
        }
        if let Some(g) = self.metrics.memory.get("js_heap") {
            b.cur.memory.insert("js_heap".to_owned(), g.last);
        }
        b.cur.ms = round3(t - b.cur.t0);
        b.runs.push(std::mem::take(&mut b.cur));
        b.path_i += 1;
        if b.path_i < b.paths.len() && t - b.t_start < BENCH_DEADLINE_MS {
            b.enter(Stage::Switch, t);
        } else {
            for name in b.paths.iter().skip(b.path_i) {
                b.runs.push(PathRun {
                    name: name.clone(),
                    errors: vec!["not run: the 3-minute deadline".to_owned()],
                    ..PathRun::default()
                });
            }
            b.enter(Stage::Done, t);
        }
    }

    #[allow(clippy::cognitive_complexity)]
    /// (RP3/T2) One animation frame of the comparison at `t` (ms), with the feed's
    /// `scrollHeight` and `clientHeight` (CSS px). Returns the actions for the page
    /// (`{paint?, tab?, scroll?, done?}`), or `Null` when no comparison runs.
    #[allow(clippy::too_many_lines)]
    pub fn bench_tick(&mut self, t: f64, scroll_h: f64, client_h: f64) -> Value {
        let Some(mut b) = self.metrics.bench.take() else {
            return Value::Null;
        };
        if b.done() {
            self.metrics.bench = Some(b);
            return Value::Null;
        }
        let mut a = Map::new();
        if let (Some(prev), true) = (b.last_t, b.stage.phase().is_some()) {
            let dt = t - prev;
            if dt > 0.0 && dt < FRAME_GAP_MS {
                b.phase.frames.push(dt);
            }
        }
        b.last_t = Some(t);
        let el = t - b.stage_t0;
        if t - b.t_start > BENCH_DEADLINE_MS && b.stage != Stage::Done {
            b.cur.errors.push(format!(
                "deadline: stopped in stage {:?} after {:.0} ms",
                b.stage,
                t - b.t_start
            ));
            self.end_phase(&mut b, t);
            self.finish_path(&mut b, t);
            b.path_i = b.paths.len();
            b.enter(Stage::Done, t);
        }
        match b.stage {
            Stage::Switch => {
                let name = b.paths[b.path_i].clone();
                b.cur = PathRun {
                    name: name.clone(),
                    t0: t,
                    ..PathRun::default()
                };
                a.insert("paint".to_owned(), json!(name));
                a.insert("tab".to_owned(), json!("feed"));
                a.insert("scroll".to_owned(), json!(0));
                if self.opening.is_some() {
                    self.back();
                    b.last_back = t;
                }
                self.replay_kind("pause");
                self.replay_control(json!({"kind": "speed", "steps_per_s": BENCH_SPEED}));
                self.seek(&BENCH_SEEK_STEP.to_string());
                b.enter(Stage::Prepare, t);
            }
            Stage::Prepare => {
                if self.opening.is_some() && self.idle() && t - b.last_back > 500.0 {
                    self.back();
                    b.last_back = t;
                }
                let ready = self.idle()
                    && self.opening.is_none()
                    && !self.feed.is_empty()
                    && self.step_now() == Some(BENCH_SEEK_STEP);
                let ok = b.stable(ready, t);
                if ok || el > BENCH_WAIT_MS {
                    if !ok {
                        b.cur.errors.push(format!(
                            "prepare: not ready after {BENCH_WAIT_MS} ms (step {:?}, idle {}, feed {})",
                            self.step_now(),
                            self.idle(),
                            self.feed.len()
                        ));
                    }
                    a.insert("scroll".to_owned(), json!(0));
                    b.cur.paint = self.metrics.paint.clone().unwrap_or(Value::Null);
                    self.replay_kind("resume");
                    self.begin_phase(&mut b, Stage::Idle, t);
                }
            }
            Stage::Idle => {
                if el >= b.phase_ms {
                    self.end_phase(&mut b, t);
                    self.replay_kind("pause");
                    self.begin_phase(&mut b, Stage::Scroll, t);
                }
            }
            Stage::Scroll => {
                if el >= b.phase_ms {
                    self.end_phase(&mut b, t);
                    a.insert("scroll".to_owned(), json!(0));
                    match self.feed.first().map(|i| i.id.clone()) {
                        Some(id) => {
                            self.open_capsule(&id);
                            b.enter(Stage::Open, t);
                        }
                        None => {
                            b.cur.errors.push("open: the feed is empty".to_owned());
                            self.finish_path(&mut b, t);
                        }
                    }
                } else {
                    let max = (scroll_h - client_h).clamp(0.0, SCROLL_AMPLITUDE_PX);
                    a.insert(
                        "scroll".to_owned(),
                        json!((tri(el / SCROLL_PERIOD_MS.min(b.phase_ms)) * max).round()),
                    );
                }
            }
            Stage::Open | Stage::Chart | Stage::Zoom => {
                let want = match b.stage {
                    Stage::Open => self
                        .opening
                        .as_ref()
                        .is_some_and(|o| o.to == OpenDepth::Capsule),
                    _ => self.chart_open(),
                };
                let ok = b.stable(want && self.idle(), t);
                if ok && b.stage == Stage::Open {
                    self.open_to("chart");
                    b.enter(Stage::Chart, t);
                } else if ok && b.stage == Stage::Chart {
                    // Ticks with the chart open first, at the chart's default range (a
                    // zoomed range no longer follows the new bars, so it would hardly
                    // repaint on a tick).
                    self.replay_kind("resume");
                    self.begin_phase(&mut b, Stage::ChartTicks, t);
                } else if ok {
                    b.sweep = None;
                    self.begin_phase(&mut b, Stage::Drag, t);
                } else if el > BENCH_WAIT_MS {
                    b.cur.errors.push(format!(
                        "open: no {} after {BENCH_WAIT_MS} ms",
                        match b.stage {
                            Stage::Open => "capsule",
                            Stage::Chart => "chart",
                            _ => "zoomed chart",
                        }
                    ));
                    self.finish_path(&mut b, t);
                }
            }
            Stage::Drag => {
                let (w, h) = self.frames["scene"];
                let (w, y) = (f64::from(w), f64::from(h) / 2.0);
                let (left, right) = (0.25 * w, 0.75 * w);
                if el >= b.phase_ms {
                    if let Some((_, x)) = b.sweep.take() {
                        self.pointer("scene", "up", 1, x, y);
                    }
                    self.end_phase(&mut b, t);
                    self.back();
                    self.finish_path(&mut b, t);
                } else {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let k = (el / DRAG_SWEEP_MS).floor() as u64;
                    let f = (el % DRAG_SWEEP_MS) / DRAG_SWEEP_MS;
                    let (x0, x1) = if k.is_multiple_of(2) {
                        (left, right)
                    } else {
                        (right, left)
                    };
                    let mut last = None;
                    if b.sweep.map(|s| s.0) == Some(k) {
                        last = b.sweep.map(|s| s.1);
                    } else {
                        if let Some((_, x)) = b.sweep.take() {
                            self.pointer("scene", "up", 1, x, y);
                        }
                        self.pointer("scene", "down", 1, x0.round(), y);
                    }
                    // Moves in fixed steps, whatever the frame rate: the input stream (and
                    // so the work per path) does not depend on the display's refresh rate.
                    let x = (x0 + (((x1 - x0) * f) / DRAG_STEP_PX).trunc() * DRAG_STEP_PX).round();
                    if last != Some(x) {
                        self.pointer("scene", "move", 1, x, y);
                    }
                    b.sweep = Some((k, x));
                }
            }
            Stage::ChartTicks => {
                if el >= b.phase_ms {
                    self.end_phase(&mut b, t);
                    self.replay_kind("pause");
                    // Then zoom in (as a reader does) and drag: at the default range the
                    // chart shows every bar, so a drag alone would be clamped.
                    let x = 0.6 * f64::from(self.frames["scene"].0);
                    for _ in 0..BENCH_ZOOM_NOTCHES {
                        self.wheel(-1.0, x.round());
                    }
                    b.enter(Stage::Zoom, t);
                }
            }
            Stage::Done => {}
        }
        if b.stage == Stage::Done {
            b.summary = Some(self.bench_summary_of(&b, t));
            b.rows = Self::bench_rows_of(&b);
            a.insert("paint".to_owned(), json!(b.restore));
            a.insert("tab".to_owned(), json!("metrics"));
            a.insert("done".to_owned(), json!(true));
        }
        self.metrics.bench = Some(b);
        Value::Object(a)
    }

    fn bench_refresh(b: &Bench) -> Refresh {
        let idle: Vec<f64> = b
            .runs
            .iter()
            .filter_map(|r| r.phases.get(PHASES[0]))
            .flat_map(|p| p.frames.v.iter().copied())
            .collect();
        Refresh::estimate(&idle)
    }

    fn bench_summary_of(&self, b: &Bench, t: f64) -> Value {
        let refresh = Self::bench_refresh(b);
        let paths: Vec<Value> = b
            .runs
            .iter()
            .map(|r| {
                let phases: Map<String, Value> = r
                    .phases
                    .iter()
                    .map(|(k, p)| {
                        (
                            k.clone(),
                            json!({
                                "ms": p.ms,
                                "steps": p.steps,
                                "frame_ms": p.frames.frame_summary(refresh.ms),
                                "span_ms": p.spans.iter().map(|(s, v)| (s.clone(), v.summary()))
                                    .collect::<Map<String, Value>>(),
                                "counts": Self::counts_of(&p.counts, &p.spans),
                            }),
                        )
                    })
                    .collect();
                let spans: Map<String, Value> = SPANS
                    .iter()
                    .map(|s| {
                        let mut all = Samples::default();
                        for p in r.phases.values() {
                            if let Some(v) = p.spans.get(*s) {
                                for x in &v.v {
                                    all.push(*x);
                                }
                            }
                        }
                        ((*s).to_owned(), all.summary())
                    })
                    .collect();
                json!({
                    "name": r.name,
                    "ms": r.ms,
                    "span_ms": spans,
                    "phases": phases,
                    "memory_bytes": r.memory,
                    "errors": r.errors,
                    "paint": r.paint,
                })
            })
            .collect();
        json!({
            "kind": "qapsule-paint-compare",
            "v": 1,
            "config": {"paths": b.paths, "phase_ms": b.phase_ms, "seek_step": BENCH_SEEK_STEP,
                       "speed_steps_per_s": BENCH_SPEED, "scroll_period_ms": SCROLL_PERIOD_MS,
                       "scroll_amplitude_px": SCROLL_AMPLITUDE_PX, "drag_sweep_ms": DRAG_SWEEP_MS,
                       "phases": PHASES},
            "total_ms": round3(t - b.t_start),
            "refresh": refresh.json("bench idle phases"),
            "device": self.device_json(),
            "warnings": self.warnings(),
            "mode": self.mode_json(),
            "agent": self.metrics.agent,
            "bundle": self.metrics.bundle,
            "replay": self.replay.as_ref().map(|r| json!({"instruments": r.instruments, "synthetic": r.synthetic})),
            "look": self.look_effective(),
            "paths": paths,
        })
    }

    fn bench_rows_of(b: &Bench) -> Value {
        let refresh = Self::bench_refresh(b);
        let mut head = vec![json!("")];
        head.extend(b.runs.iter().map(|r| json!(r.name)));
        let mut rows: Vec<Value> = Vec::new();
        let row = |label: String, f: &dyn Fn(&PathRun) -> String| {
            let mut v = vec![json!(label)];
            v.extend(b.runs.iter().map(|r| json!(f(r))));
            Value::Array(v)
        };
        let ppp = |s: &Samples| {
            let v = s.summary();
            if v["n"] == 0 {
                "—".to_owned()
            } else {
                format!(
                    "{:.1} / {:.1} / {:.1}",
                    v["p50"].as_f64().unwrap_or(0.0),
                    v["p95"].as_f64().unwrap_or(0.0),
                    v["p99"].as_f64().unwrap_or(0.0)
                )
            }
        };
        for s in SPANS {
            rows.push(row(format!("{s} p50/p95/p99 ms"), &|r: &PathRun| {
                let mut all = Samples::default();
                for p in r.phases.values() {
                    if let Some(v) = p.spans.get(s) {
                        for x in &v.v {
                            all.push(*x);
                        }
                    }
                }
                ppp(&all)
            }));
        }
        for ph in PHASES {
            rows.push(row(format!("kare {ph} p50/p95/p99 ms"), &|r: &PathRun| {
                r.phases.get(ph).map_or("—".to_owned(), |p| ppp(&p.frames))
            }));
            rows.push(row(
                format!("kare {ph} uzun % (>{:.1} ms)", LONG_FACTOR * refresh.ms),
                &|r: &PathRun| {
                    r.phases.get(ph).map_or("—".to_owned(), |p| {
                        p.frames.frame_summary(refresh.ms)["long"]
                            .as_f64()
                            .map_or("—".to_owned(), |l| format!("{:.1}", l * 100.0))
                    })
                },
            ));
        }
        rows.push(row("bellek wasm".to_owned(), &|r: &PathRun| {
            r.memory.get("wasm").map_or("—".to_owned(), |x| mb(*x))
        }));
        rows.push(row("bellek js".to_owned(), &|r: &PathRun| {
            r.memory.get("js_heap").map_or("—".to_owned(), |x| mb(*x))
        }));
        rows.push(row("süre s".to_owned(), &|r: &PathRun| {
            format!("{:.1}", r.ms / 1000.0)
        }));
        rows.push(row("hata".to_owned(), &|r: &PathRun| {
            if r.errors.is_empty() {
                "—".to_owned()
            } else {
                r.errors.join("; ")
            }
        }));
        json!({"head": head, "rows": rows})
    }

    /// (RP3/T2) The comparison's summary (JSON), `Null` until it has finished.
    #[must_use]
    pub fn bench_summary(&self) -> Value {
        self.metrics
            .bench
            .as_ref()
            .and_then(|b| b.summary.clone())
            .unwrap_or(Value::Null)
    }

    /// (RP3/T2) The comparison as a table for the page: `{head: [..], rows: [[label,
    /// value per path]], notes: [text]}` (values formatted here); `Null` until finished.
    #[must_use]
    pub fn bench_rows(&self) -> Value {
        let Some(b) = self.metrics.bench.as_ref().filter(|b| b.done()) else {
            return Value::Null;
        };
        let mut v = b.rows.clone();
        let s = b.summary.as_ref().unwrap_or(&Value::Null);
        let mut notes = vec![json!(format!(
            "refresh {} Hz · toplam {:.1} s · DPR {} · ölçek {}",
            s["refresh"]["hz"],
            s["total_ms"].as_f64().unwrap_or(0.0) / 1000.0,
            s["device"]["dpr"],
            s["device"]["scale"]
        ))];
        if let Some(w) = s["warnings"].as_array() {
            notes.extend(w.iter().map(|x| x["text"].clone()));
        }
        v["notes"] = Value::Array(notes);
        v
    }

    /// (RP3/T2) Whether a comparison is running (started and not finished).
    #[must_use]
    pub fn bench_running(&self) -> bool {
        self.metrics.bench.as_ref().is_some_and(|b| !b.done())
    }
}
