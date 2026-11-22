//! The shell core's metrics (RG1: moved out of `shell.rs`): samples, refresh-class
//! estimate, gauges, the metrics state and the `metric_*` / `metrics_*` methods.

use super::*;

/// Running statistics of a series of millisecond samples.
#[derive(Debug, Clone, Default)]
pub(super) struct Samples {
    pub(super) v: Vec<f64>,
}

impl Samples {
    const MAX: usize = 4096;
    pub(super) fn push(&mut self, x: f64) {
        if x.is_finite() && x >= 0.0 {
            if self.v.len() == Self::MAX {
                self.v.remove(0);
            }
            self.v.push(x);
        }
    }
    pub(super) fn summary(&self) -> Value {
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
    pub(super) fn frame_summary(&self, refresh_ms: f64) -> Value {
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
pub(super) const FRAME_GAP_MS: f64 = 1000.0;

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

    pub(super) fn json(&self, source: &str) -> Value {
        json!({"hz": self.hz, "ms": round3(self.ms), "median_ms": self.median_ms.map(round3),
               "n": self.n, "source": if self.median_ms.is_some() { source } else { "assumed" },
               "rule": format!("long: interval > {LONG_FACTOR} x refresh ms; very_long: > {VERY_LONG_FACTOR} x")})
    }
}

/// (R4/D) A gauge of the metrics page (device facts, memory): last and largest value.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Gauge {
    pub(super) last: f64,
    pub(super) max: f64,
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
pub(super) fn wasm_memory_bytes() -> Option<f64> {
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

pub(super) fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Metrics of the metrics page (`docs/CONCEPTS.md` "Ölçüm ekranı"): raw timestamps in
/// (from `performance.now()`), statistics out.
#[derive(Debug, Clone, Default)]
pub(super) struct Metrics {
    /// Frame intervals by mode (`idle`, `scroll`, `drag`).
    pub(super) frames: BTreeMap<String, Samples>,
    pub(super) last_frame: Option<(String, f64)>,
    /// Spans by name (`tick`, `face`, `scene`, `step`).
    pub(super) spans: BTreeMap<String, Samples>,
    /// Marks of the opening timeline (ms since navigation start).
    pub(super) marks: BTreeMap<String, f64>,
    pub(super) bundle: Value,
    pub(super) agent: String,
    /// (R4/D) Device facts the page reports (`cores`, `device_memory_gb`, `screen_w`,
    /// `screen_h`, `dpr`, …): last value wins.
    pub(super) device: BTreeMap<String, f64>,
    /// (R4/D) Frame gaps (intervals of a second or more) by mode.
    pub(super) gaps: BTreeMap<String, u64>,
    /// (R4/D) Memory samples in bytes (`js_heap` from the page, `wasm` from the core).
    pub(super) memory: BTreeMap<String, Gauge>,
    /// (RP3/T2) Marks that came again after their first occurrence (a later start frame
    /// after a reconnect or a backward seek): counted, never overwriting the first.
    pub(super) repeats: BTreeMap<String, u64>,
    /// (RP3/T2) The paint path: `{requested, used, fallback}` (`shells/web/paint/`).
    pub(super) paint: Option<Value>,
    /// (RP3/T2) The automatic comparison (`bench`), when one runs or ran.
    pub(super) bench: Option<bench::Bench>,
    /// (RP3/M14) Event counters (`data_frames`, `msg_frames`, `paint_passes`,
    /// `faces_skipped_hidden`); restart with the series.
    pub(super) counts: BTreeMap<String, u64>,
    /// (RP3/M14) The JIT probe's time (ms; kept over a reset of the series).
    pub(super) probe: Option<f64>,
    /// (RP3/M14) Page facts (`secure_context`, `webgpu`).
    pub(super) flags: BTreeMap<String, bool>,
}

impl ShellCore {
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
    pub(super) fn look_effective(&self) -> Value {
        json!({
            "face": self.look.face,
            "feed": self.look.feed,
            "transition": self.look.transition,
            "outcome": self.look.outcome,
            "alphabet": self.look.alphabet,
        })
    }

    /// (R4/D) The device facts plus the paint scale the core chose (`scale`).
    pub(super) fn device_json(&self) -> Value {
        let mut o: serde_json::Map<String, Value> = self
            .metrics
            .device
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
        o.insert("scale".to_owned(), json!(self.scale));
        Value::Object(o)
    }
}

/// (RP3/S) A core's metrics in transit between two cores (`metric_detach` →
/// `metric_attach`); opaque.
#[derive(Debug, Default)]
pub struct MetricsCarry(Metrics);
