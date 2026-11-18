//! (RP3/M14, GOAL criterion 6) Live update frequency: at most one paint pass per display
//! frame, and nothing painted that is not on screen.
//!
//! The page hands every frame of the host's stream to the core at once
//! ([`ShellCore::on_frame_at`]); the core applies it and **accumulates** what changed
//! (`feed`, `scene`, `status`, `error`, `transition`, …). The page paints once per
//! animation frame: it takes the accumulated changes ([`ShellCore::take_changes`]) and
//! repaints for them, so that several frames arriving within one display frame cost one
//! paint. Which faces that pass paints is the core's decision too
//! ([`ShellCore::faces_to_paint`]): only faces on screen (the page reports them with
//! [`ShellCore::visible`]), and none while the open layer covers the feed.
//!
//! Metrics: `tick` stays the core's work of one data frame (now without painting);
//! `paint_pass` is one coalesced paint pass; `frame_to_paint` the time from the first
//! frame that raised a change to the end of the pass that painted it (`pass_wait`: to its
//! start); counters (`data_frames`, `msg_frames`, `paint_passes`, `faces_skipped_hidden`,
//! `scene_builds`) in the summary.
//!
//! Also here: the JIT probe (a fixed WASM loop the page times once, [`jit_probe`]) and
//! its classification, and the page facts the summary must state (`secure_context`,
//! `webgpu`): on iOS a page that is not a secure context runs without the JIT and without
//! WebGPU (`rounds/RP3/CALIBRATION.md`).

use serde_json::{json, Map, Value};

use super::{round3, ShellCore};

/// Iterations of the JIT probe loop.
pub const JIT_PROBE_ITERATIONS: u32 = 1_000_000;
/// The probe (one call, [`JIT_PROBE_ITERATIONS`]) taking longer than this (ms) means the
/// WASM runs without the JIT (an interpreter). Measured on the Mac (rounds/RP3/
/// experiments/M14.md §3, per fresh page): WebKit 26.5 with the JIT 2–5 ms, without it
/// (`__XPC_JSC_useJIT=false`) 20–42 ms; Chromium 2.8–4.4 ms, CDP CPU throttling 4×
/// 9.7–11 ms, 16× 40–43 ms. A heuristic: the time also scales with the CPU, so a heavily
/// throttled desktop (≥ 6×) reads as "off"; a phone with the JIT is expected within 2–3×
/// of the desktop, one without it well above.
pub const JIT_OFF_MS: f64 = 15.0;

/// (RP3/M14) The JIT probe: a fixed, deterministic integer loop of
/// [`JIT_PROBE_ITERATIONS`] dependent steps (no allocation, no memory traffic). The page
/// times one call; its result only keeps the loop from being optimised away.
#[must_use]
pub fn jit_probe() -> u32 {
    let n = std::hint::black_box(JIT_PROBE_ITERATIONS);
    let mut x: u32 = std::hint::black_box(0x9e37_79b9);
    for i in 0..n {
        // A serial chain (each step needs the previous one): it cannot be vectorised or
        // folded, so its time is the engine's per-instruction cost.
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x = x.wrapping_add(i);
    }
    x
}

/// (R5/T0) The per-data-frame rates need at least this many data frames; fewer give
/// "insufficient samples" (`null`), not a number. One data frame in a phase of scrolling
/// is not a rate (a bench phase at 4 steps/s has ≈30).
pub const RATE_MIN_DATA_FRAMES: u64 = 10;

/// (R5/T0) The per-data-frame rates `(passes_per_data_frame, tick_plus_pass_ms)` from
/// the data frames, the passes that painted their changes, the timed ticks (count, total
/// ms) and those passes' total ms; `None` ("insufficient samples") below
/// [`RATE_MIN_DATA_FRAMES`] data frames or without a timed tick. Always finite.
#[must_use]
pub fn rates(
    frames: u64,
    data_passes: u64,
    ticks: usize,
    tick_ms: f64,
    data_pass_ms: f64,
) -> Option<(f64, f64)> {
    if frames < RATE_MIN_DATA_FRAMES || ticks == 0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let (f, t, p) = (frames as f64, ticks as f64, data_passes as f64);
    let r = (p / f, tick_ms / t + data_pass_ms / f);
    (r.0.is_finite() && r.1.is_finite()).then_some(r)
}

/// The probe's verdict for probe time `ms`.
fn jit_state(ms: Option<f64>) -> &'static str {
    match ms {
        None => "unknown",
        Some(m) if m > JIT_OFF_MS => "likely_off",
        Some(_) => "likely_on",
    }
}

impl ShellCore {
    /// (RP3/M14) A frame of the host's stream that arrived at `t` ms (the page's clock;
    /// `None`: not timed): applied at once ([`ShellCore::on_frame`]), its changes added to
    /// those waiting for the next paint pass. Counts the frame by kind and marks
    /// `connected` at the `welcome`. Returns the frame's own changes.
    ///
    /// # Errors
    /// As [`ShellCore::on_frame`].
    pub fn on_frame_at(&mut self, text: &str, t: Option<f64>) -> Result<Value, String> {
        let kind = if text.starts_with("{\"msg\"") {
            "msg_frames"
        } else {
            "data_frames"
        };
        *self.metrics.counts.entry(kind.to_owned()).or_default() += 1;
        // (R5/M21) The faces as shown before this bar (a marking motion design).
        if kind == "data_frames" {
            self.data_arrives();
        }
        let ch = self.on_frame(text)?;
        if let Some(o) = ch.as_object() {
            if let (Some(t), Some(true)) = (t, o.get("welcome").and_then(Value::as_bool)) {
                self.metric_mark("connected", t);
            }
            if !o.is_empty() && self.pending.is_empty() {
                self.pending_since = t;
            }
            if !o.is_empty() && kind == "data_frames" {
                self.pending_data = true;
            }
            for (k, v) in o {
                self.pending.insert(k.clone(), v.clone());
            }
        }
        Ok(ch)
    }

    /// (RP3/M14) Whether changes wait for a paint pass.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.pending.is_empty()
    }

    /// (RP3/M14) The changes accumulated since the last pass (`{feed?, scene?, status?,
    /// error?, transition?, welcome?}`), emptied; `None` when nothing changed (the page
    /// then paints nothing).
    pub fn take_changes(&mut self) -> Option<Value> {
        if self.pending.is_empty() {
            return None;
        }
        self.pass_since = self.pending_since.take();
        self.pass_data = std::mem::take(&mut self.pending_data);
        Some(Value::Object(std::mem::take(&mut self.pending)))
    }

    /// (RP3/M14) The page's paint pass for the changes it took ran from `t0` to `t1` ms:
    /// the `paint_pass` span, the pass counter and the latency `frame_to_paint` (from the
    /// arrival of the first frame whose change it painted to its end). (R5/T0) A pass
    /// that painted a data frame's changes is also a `data_pass` (span and counter): the
    /// per-data-frame rates count only those.
    pub fn metric_pass(&mut self, t0: f64, t1: f64) {
        self.metric_span("paint_pass", t0, t1);
        *self
            .metrics
            .counts
            .entry("paint_passes".to_owned())
            .or_default() += 1;
        if std::mem::take(&mut self.pass_data) {
            self.metric_span("data_pass", t0, t1);
            *self
                .metrics
                .counts
                .entry("data_passes".to_owned())
                .or_default() += 1;
        }
        if let Some(since) = self.pass_since.take() {
            self.metric_span("frame_to_paint", since, t1);
            // The wait for the animation frame alone (the latency the coalescing adds).
            self.metric_span("pass_wait", since, t0);
        }
    }

    /// (RP3/M14) The faces a paint pass repaints (when they changed), in feed order: the
    /// feed's capsules with a face scene that are on screen ([`ShellCore::visible`]) —
    /// none while a capsule is open, since the open layer covers the feed. The faces left
    /// out are counted (`faces_skipped_hidden`).
    pub fn faces_to_paint(&mut self) -> Vec<String> {
        let covered = self.opening.is_some();
        let mut out = Vec::new();
        let mut skipped = 0u64;
        for it in &self.feed {
            let has = match &self.viewer {
                Some(v) => v.row(&it.id).is_some_and(|r| r.face),
                None => self.replica.bars(&it.series).is_some_and(|b| !b.is_empty()),
            };
            if !has {
                continue;
            }
            if !covered && self.shown.contains(&it.id) {
                out.push(it.id.clone());
            } else {
                skipped += 1;
            }
        }
        *self
            .metrics
            .counts
            .entry("faces_skipped_hidden".to_owned())
            .or_default() += skipped;
        out
    }

    /// (RP3/M14) A page fact for the summary (`secure_context`, `webgpu`).
    pub fn metric_flag(&mut self, name: &str, on: bool) {
        self.metrics.flags.insert(name.to_owned(), on);
    }

    /// (RP3/M14) The counters with the derived rates: `{data_frames, msg_frames,
    /// paint_passes, data_passes, faces_painted, faces_skipped_hidden, scene_builds,
    /// scenes_painted, passes_per_data_frame, tick_plus_pass_ms, rate_frames_min}` —
    /// `faces_painted` is the `face` span's count (one per face painted, whatever the paint
    /// path); `passes_per_data_frame` the passes that painted a data frame's changes per
    /// data frame (coalescing: at most 1); `tick_plus_pass_ms` the mean core work per data
    /// frame including its share of those passes (`tick` mean + `data_pass` total / data
    /// frames; R4's `tick` included the repaint, so this is the comparable number).
    ///
    /// (R5/T0) Both rates are `null` ("insufficient samples") below
    /// [`RATE_MIN_DATA_FRAMES`] data frames or without a timed tick. Before, every pass
    /// (also those of scrolling, taps and the opening) was divided by the data frames: a
    /// phone's scroll phase with one data frame read 42 passes and 466 ms per data frame.
    pub(super) fn counts_json(&self) -> Value {
        Self::counts_of(&self.metrics.counts, &self.metrics.spans)
    }

    pub(super) fn counts_of(
        counts: &std::collections::BTreeMap<String, u64>,
        spans: &std::collections::BTreeMap<String, super::Samples>,
    ) -> Value {
        let c = |k: &str| counts.get(k).copied().unwrap_or(0);
        let n = |k: &str| spans.get(k).map_or(0, |s| s.v.len());
        let sum = |k: &str| spans.get(k).map_or(0.0, |s| s.v.iter().sum::<f64>());
        let mut o = Map::new();
        for k in [
            "data_frames",
            "msg_frames",
            "paint_passes",
            "data_passes",
            "faces_skipped_hidden",
            "scene_builds",
        ] {
            o.insert(k.to_owned(), json!(c(k)));
        }
        o.insert("faces_painted".to_owned(), json!(n("face")));
        o.insert("scenes_painted".to_owned(), json!(n("scene")));
        o.insert("rate_frames_min".to_owned(), json!(RATE_MIN_DATA_FRAMES));
        let rates = rates(
            c("data_frames"),
            c("data_passes"),
            n("tick"),
            sum("tick"),
            sum("data_pass"),
        );
        o.insert(
            "passes_per_data_frame".to_owned(),
            rates.map_or(Value::Null, |r| json!(round3(r.0))),
        );
        o.insert(
            "tick_plus_pass_ms".to_owned(),
            rates.map_or(Value::Null, |r| json!(round3(r.1))),
        );
        Value::Object(o)
    }

    /// (RP3/M14) What the page runs on: `{secure_context, webgpu, jit: {probe_ms, state,
    /// off_above_ms, iterations}}` (`null`: not reported yet).
    pub(super) fn mode_json(&self) -> Value {
        let f = |k: &str| self.metrics.flags.get(k).map_or(Value::Null, |b| json!(b));
        json!({
            "secure_context": f("secure_context"),
            "webgpu": f("webgpu"),
            "jit": {
                "probe_ms": self.metrics.probe.map(round3),
                "state": jit_state(self.metrics.probe),
                "off_above_ms": JIT_OFF_MS,
                "iterations": JIT_PROBE_ITERATIONS,
            },
        })
    }

    /// (RP3/M14) The warnings about the page's mode: not a secure context, JIT likely off.
    pub(super) fn mode_warnings(&self) -> Vec<Value> {
        let mut w = Vec::new();
        if self.metrics.flags.get("secure_context") == Some(&false) {
            w.push(json!({"code": "insecure_context",
                "text": "page is not a secure context (plain http:// to a LAN address): on iOS such a page runs JS and WASM without the JIT and has no WebGPU; both need HTTPS (or localhost)"}));
        }
        if jit_state(self.metrics.probe) == "likely_off" {
            w.push(json!({"code": "jit_off",
                "text": format!("JIT probe {:.1} ms > {JIT_OFF_MS} ms: WASM seems to run without the JIT (an interpreter); timings are those of the slow mode (on iOS: open the page over HTTPS)",
                    self.metrics.probe.unwrap_or(0.0))}));
        }
        w
    }

    /// (RP3/M14) The metrics page's rows about the page's mode and the counters.
    pub(super) fn mode_rows(&self) -> Vec<Value> {
        let mut rows = Vec::new();
        let yn = |k: &str| match self.metrics.flags.get(k) {
            Some(true) => "evet",
            Some(false) => "hayır",
            None => "—",
        };
        rows.push(json!(["secure_context", yn("secure_context")]));
        rows.push(json!(["webgpu", yn("webgpu")]));
        rows.push(json!([
            "jit",
            match self.metrics.probe {
                None => "ölçülmedi (açılıştan birkaç saniye sonra)".to_owned(),
                Some(ms) => format!(
                    "{} (sınama {ms:.1} ms; > {JIT_OFF_MS} ms kapalı sayılır)",
                    match jit_state(Some(ms)) {
                        "likely_off" => "büyük olasılıkla kapalı",
                        _ => "büyük olasılıkla açık",
                    }
                ),
            }
        ]));
        let c = self.counts_json();
        rows.push(json!([
            "güncelleme",
            format!(
                "veri çerçevesi {} · mesaj {} · boyama geçişi {} (veri için {}) · yüz boyandı {} · görünmediği için atlandı {}",
                c["data_frames"],
                c["msg_frames"],
                c["paint_passes"],
                c["data_passes"],
                c["faces_painted"],
                c["faces_skipped_hidden"]
            )
        ]));
        rows.push(json!(["tik + geçiş", rate_text(&c)]));
        rows
    }
}

/// (R5/T0) The metrics page's "tik + geçiş" row: the core work per data frame and the
/// passes per data frame, or "insufficient samples" with the count it has.
fn rate_text(c: &Value) -> String {
    match (
        c["tick_plus_pass_ms"].as_f64(),
        c["passes_per_data_frame"].as_f64(),
    ) {
        (Some(ms), Some(per)) => {
            format!("{ms:.2} ms / veri çerçevesi · {per:.2} geçiş / veri çerçevesi")
        }
        _ => format!(
            "yetersiz örnek (veri çerçevesi {}, en az {RATE_MIN_DATA_FRAMES} gerekir)",
            c["data_frames"]
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_probe_is_deterministic() {
        assert_eq!(jit_probe(), jit_probe());
    }

    #[test]
    fn rates_need_enough_data_frames() {
        // No data frame, one, or just below the minimum: insufficient, never a number.
        assert_eq!(rates(0, 0, 0, 0.0, 0.0), None);
        assert_eq!(rates(1, 1, 1, 2.0, 466.0), None);
        assert_eq!(rates(RATE_MIN_DATA_FRAMES - 1, 9, 9, 9.0, 9.0), None);
        // Enough data frames but no timed tick (frames counted, ticks not): insufficient.
        assert_eq!(rates(RATE_MIN_DATA_FRAMES, 10, 0, 0.0, 30.0), None);
        // Enough: tick mean + data-pass total / data frames; passes / data frames.
        let (per, ms) = rates(20, 10, 20, 40.0, 30.0).expect("rates");
        assert!((per - 0.5).abs() < 1e-12);
        assert!((ms - (2.0 + 1.5)).abs() < 1e-12);
    }

    #[test]
    fn the_rate_row_is_honest() {
        let thin =
            json!({"data_frames": 1, "passes_per_data_frame": null, "tick_plus_pass_ms": null});
        let t = rate_text(&thin);
        assert!(t.starts_with("yetersiz örnek"), "{t}");
        assert!(
            !t.contains("null") && !t.contains("NaN") && !t.contains("inf"),
            "{t}"
        );
        let ok =
            json!({"data_frames": 31, "passes_per_data_frame": 0.968, "tick_plus_pass_ms": 7.3});
        assert_eq!(
            rate_text(&ok),
            "7.30 ms / veri çerçevesi · 0.97 geçiş / veri çerçevesi"
        );
    }

    #[test]
    fn the_probe_verdict() {
        assert_eq!(jit_state(None), "unknown");
        assert_eq!(jit_state(Some(1.0)), "likely_on");
        assert_eq!(jit_state(Some(JIT_OFF_MS + 1.0)), "likely_off");
    }
}
