//! The shell binding (RG1/M1a: moved out of `lib.rs` as a whole). The module stays
//! inline so that its text, formatted at this depth, is unchanged by the move.

pub use shell_binding::Shell;

mod shell_binding {
    use wasm_bindgen::prelude::*;

    use crate::shell::ShellCore;

    fn err(e: impl std::fmt::Display) -> JsError {
        JsError::new(&e.to_string())
    }

    /// (RP3/S) A core's metrics between two cores (opaque to the page).
    #[wasm_bindgen]
    pub struct MetricsCarry {
        inner: crate::shell::MetricsCarry,
    }

    /// The demo shell's core.
    #[wasm_bindgen]
    pub struct Shell {
        inner: ShellCore,
        // (RP3/M12b) the `canvas` paint path's state (what each canvas shows now)
        canvas: crate::shell::paint_canvas::CanvasPainter,
        // (RP3/M12c) The GPU paint path's glyph atlas and last built frame.
        gpu: crate::shell::paint_gpu::GpuPainter,
    }

    #[wasm_bindgen]
    impl Shell {
        /// From the host's `start` frame (`{conn, principal, lang, data, events}`; RP3/M15
        /// `mode: "viewer"` with `instruments` instead of data: the viewer core).
        ///
        /// # Errors
        /// A malformed frame.
        #[wasm_bindgen(constructor)]
        pub fn new(start_json: &str) -> Result<Shell, JsError> {
            // (RP3/T4) Typed, without a JSON tree of the frame (see `core_from_start`).
            let inner = crate::shell::core_from_start(start_json).map_err(err)?;
            Ok(Self {
                inner,
                canvas: crate::shell::paint_canvas::CanvasPainter::default(),
                gpu: crate::shell::paint_gpu::GpuPainter::default(),
            })
        }

        /// (RP3/M15) The shell says capsule `id`'s face is (or is no longer) on screen.
        pub fn visible(&mut self, id: &str, on: bool) {
            self.inner.visible(id, on);
        }

        /// (RP3/M15, VERIFY 17) The device's data subscriptions, what it holds and the
        /// bytes it received (JSON).
        pub fn viewer(&self) -> String {
            self.inner.viewer_json().to_string()
        }

        /// (R5/M18, §23.5) The open instrument's story list the device holds (JSON; `null`
        /// unless the chart is open). For the story chart alternatives; no drawing here.
        pub fn stories(&self) -> String {
            self.inner.stories_json().to_string()
        }

        /// (R5/M18, K52 measurement only) `n` synthetic stories in place of the held list.
        pub fn bench_stories(&mut self, n: u32) -> u32 {
            self.inner.bench_stories(n)
        }

        /// (RP3/M15) Runs the open capsule's check on its own data again (JSON).
        pub fn check(&mut self) -> String {
            self.inner.check_now().to_string()
        }

        /// One frame of the host's stream, arrived at `t` ms (`performance.now()`; may be
        /// left out); returns what changed (JSON). (RP3/M14) The changes also wait for the
        /// next paint pass (`take_changes`).
        ///
        /// # Errors
        /// A malformed frame.
        pub fn on_frame(&mut self, text: &str, t: Option<f64>) -> Result<String, JsError> {
            self.inner
                .on_frame_at(text, t)
                .map(|v| v.to_string())
                .map_err(err)
        }

        /// (RP3/M14) The changes accumulated since the last paint pass (JSON), emptied;
        /// empty text when nothing changed.
        pub fn take_changes(&mut self) -> String {
            self.inner
                .take_changes()
                .map(|v| v.to_string())
                .unwrap_or_default()
        }

        /// (RP3/M14) Whether changes wait for a paint pass.
        pub fn has_changes(&self) -> bool {
            self.inner.has_changes()
        }

        /// (RP3/M14) The paint pass ran from `t0` to `t1` ms (span, counter, latency).
        pub fn metric_pass(&mut self, t0: f64, t1: f64) {
            self.inner.metric_pass(t0, t1);
        }

        /// (RP3/M14) The faces the paint pass repaints when changed (JSON array of ids):
        /// those on screen, none while the open layer covers the feed.
        pub fn faces_to_paint(&mut self) -> String {
            serde_json::Value::from(self.inner.faces_to_paint()).to_string()
        }

        /// (RP3/M14) A page fact for the summary (`secure_context`, `webgpu`).
        pub fn metric_flag(&mut self, name: &str, on: bool) {
            self.inner.metric_flag(name, on);
        }

        /// (RP3/M14) The JIT probe: a fixed WASM loop the page times once
        /// (`metric_span('jit_probe', …)`); the result means nothing.
        pub fn jit_probe(&self) -> u32 {
            crate::shell::freq::jit_probe()
        }

        /// The messages to send (JSON array).
        pub fn take_outbox(&mut self) -> String {
            serde_json::Value::Array(self.inner.take_outbox()).to_string()
        }

        /// Nothing outstanding.
        pub fn idle(&self) -> bool {
            self.inner.idle()
        }

        /// The feed (JSON).
        pub fn feed(&mut self) -> String {
            self.inner.feed_json().to_string()
        }

        /// A face's pixels (empty: unchanged or none).
        pub fn face_rgba(&mut self, id: &str, force: bool) -> Vec<u8> {
            self.inner.face_rgba(id, force)
        }

        /// The open scene's pixels (empty: unchanged or none).
        pub fn scene_rgba(&mut self, force: bool) -> Vec<u8> {
            self.inner.scene_rgba(force)
        }

        /// The open capsule (JSON).
        pub fn open(&mut self) -> String {
            self.inner.open_json().to_string()
        }

        /// (R5/m17-int, K49) The glossary explanation to show (JSON `{term, text, target}`
        /// or `null`).
        pub fn gloss(&self) -> String {
            self.inner.gloss_json().to_string()
        }

        /// (R5/m17-int) Closes the glossary explanation.
        pub fn close_gloss(&mut self) {
            self.inner.close_gloss();
        }

        /// (R5/m17-int) Capsule `id`'s face scene as the core builds it (JSON `SceneDesc`;
        /// empty: none): tests read its glossary `terms`.
        pub fn face_desc(&mut self, id: &str) -> String {
            self.inner
                .face(id)
                .and_then(|s| serde_json::to_string(&s).ok())
                .unwrap_or_default()
        }

        /// (R5/M20) The open capsule's scene as the core builds it (JSON `SceneDesc`;
        /// empty: none): tests read its panels and `terms`.
        pub fn open_desc(&mut self) -> String {
            self.inner
                .scene()
                .and_then(|s| serde_json::to_string(&s).ok())
                .unwrap_or_default()
        }

        /// Evidence rows (JSON).
        pub fn evidence(&self) -> String {
            self.inner.evidence_json().to_string()
        }

        /// (R5B) The open capsule's comments, written for the page (JSON `[{id, label,
        /// origin, text}]`).
        pub fn comments(&self) -> String {
            self.inner.comments_json().to_string()
        }

        /// (R5B) The pending proposals, written for the page (JSON `[{id, author, ops, reason}]`: each op with its arguments, the proposer's reason labelled as theirs).
        pub fn proposals(&self) -> String {
            self.inner.proposals_json().to_string()
        }

        /// (R5B, §25.2) The reader accepts (`true`) or rejects pending proposal `id` on
        /// this connection.
        pub fn decide(&mut self, id: &str, accept: bool) {
            self.inner.decide(id, accept);
        }

        /// The replay state, formatted (JSON).
        pub fn status(&self) -> String {
            self.inner.status_json().to_string()
        }

        /// The last error message (empty: none).
        pub fn take_error(&mut self) -> String {
            self.inner.take_error().unwrap_or_default()
        }

        /// The view state (JSON).
        pub fn look(&self) -> String {
            self.inner.look_json().to_string()
        }

        /// The document's instruments (JSON array).
        pub fn instruments(&self) -> String {
            serde_json::to_string(&self.inner.instruments()).unwrap_or_default()
        }

        /// The reader's audience (JSON or `null`).
        pub fn audience(&self) -> String {
            self.inner.audience().to_string()
        }

        /// A frame's size in CSS px (`face`, `scene`).
        pub fn set_frame(&mut self, key: &str, w: u32, h: u32) {
            self.inner.set_frame(key, w, h);
        }

        /// A replay control (JSON).
        ///
        /// # Errors
        /// Not JSON.
        pub fn replay(&mut self, control_json: &str) -> Result<(), JsError> {
            let c = serde_json::from_str(control_json).map_err(err)?;
            self.inner.replay_control(c);
            Ok(())
        }

        /// Seek to a date (`YYYY-MM-DD`) or a step number.
        pub fn seek(&mut self, date_or_step: &str) {
            self.inner.seek(date_or_step);
        }

        /// Sensitivity level.
        pub fn set_sensitivity(&mut self, level: &str) {
            self.inner.set_sensitivity(level);
        }

        /// Audience choices (JSON).
        ///
        /// # Errors
        /// Not JSON.
        pub fn set_audience(&mut self, choice_json: &str) -> Result<(), JsError> {
            let c = serde_json::from_str(choice_json).map_err(err)?;
            self.inner.set_audience(&c);
            Ok(())
        }

        /// Style: face, alphabet, theme (`""`: unchanged).
        pub fn set_style(&mut self, face: &str, alphabet: &str, theme: &str) {
            self.inner.set_style(face, alphabet, theme);
        }

        /// (R4/H-M8910) Style from a settings object (JSON: `face`, `alphabet`, `theme`,
        /// `feed`, `transition`, `outcome`; `""`: unchanged).
        ///
        /// # Errors
        /// Not JSON.
        pub fn set_look(&mut self, choice_json: &str) -> Result<(), JsError> {
            let c = serde_json::from_str(choice_json).map_err(err)?;
            self.inner.set_look(&c);
            Ok(())
        }

        /// (R4/H-M8910) Mark the reader's last look at the replay clock.
        pub fn mark_seen(&mut self) {
            self.inner.mark_seen();
        }

        /// (R4/S1) The page's `?seen=` (`now` or a step number): the last look is marked
        /// when the replay is there.
        pub fn seen_at(&mut self, step_or_now: &str) {
            self.inner.seen_at(step_or_now);
        }

        /// (R4/H-M8910) Open capsule `id` to its capsule depth.
        pub fn open_capsule(&mut self, id: &str) {
            self.inner.open_capsule(id);
        }

        /// (R4/H-M8910) The page's device pixel ratio; returns whether the paint scale
        /// changed (repaint then).
        pub fn set_dpr(&mut self, dpr: f64) -> bool {
            self.inner.set_dpr(dpr)
        }

        /// (R4/H-M8910) The names each setting accepts (JSON).
        pub fn choices(&self) -> String {
            ShellCore::choices_json().to_string()
        }

        /// (R4/H-M8910) The face frame (JSON: `css`, `px`, `style`).
        pub fn face_size(&self) -> String {
            self.inner.face_size_json().to_string()
        }

        /// (R4/H-M8910) Progress of the last transition's animation at shell time `t`.
        pub fn transition_progress(&mut self, t: f64) -> f64 {
            self.inner.transition_progress(t)
        }

        /// (R5/M21) The motion overlay of the display frame at page time `t` (JSON
        /// `{"marks": [{face, key, style}]}`), or empty text when nothing changes.
        pub fn motion_marks(&mut self, t: f64) -> String {
            self.inner
                .motion_marks(t)
                .map(|v| v.to_string())
                .unwrap_or_default()
        }

        /// (R4/H-M8910) The last transition's frame at progress `p` (JSON).
        pub fn transition_frame(
            &self,
            p: f64,
            from_left: f64,
            from_top: f64,
            to_left: f64,
            to_top: f64,
        ) -> String {
            self.inner
                .transition_frame(p, from_left, from_top, to_left, to_top)
                .to_string()
        }

        /// Open the open capsule to `to` (`chart`, `evidence`).
        pub fn open_to(&mut self, to: &str) {
            self.inner.open_to(to);
        }

        /// Close one depth.
        pub fn back(&mut self) {
            self.inner.back();
        }

        /// A raw pointer event; returns the gesture mode.
        pub fn pointer(&mut self, target: &str, phase: &str, id: i32, x: f64, y: f64) -> String {
            self.inner.pointer(target, phase, id, x, y)
        }

        /// A wheel notch; (R5/M20) `true` when it was the chart's zoom (else the page
        /// scrolls).
        pub fn wheel(&mut self, dy: f64, x: f64) -> bool {
            self.inner.wheel(dy, x)
        }

        /// The last transition in page coordinates (JSON).
        pub fn transition(
            &self,
            from_left: f64,
            from_top: f64,
            to_left: f64,
            to_top: f64,
        ) -> String {
            self.inner
                .transition_json(from_left, from_top, to_left, to_top)
                .to_string()
        }

        /// Where the last transition started (`sentence`, `capsule`, …).
        pub fn transition_from(&self) -> String {
            self.inner.transition_from().to_owned()
        }

        /// (R5/M21) The reader's reduced-motion setting (`prefers-reduced-motion` or the
        /// settings page): openings happen at once, face changes carry a lasting label.
        pub fn set_reduced_motion(&mut self, on: bool) {
            self.inner.set_reduced_motion(on);
        }

        /// (R5/M21) The faces whose change to animate now (JSON array of ids; emptied).
        pub fn take_face_motion(&mut self) -> String {
            self.inner.take_face_motion().to_string()
        }

        /// (R5/M21) Progress of capsule `id`'s change animation at shell time `t`.
        pub fn face_change_progress(&mut self, id: &str, t: f64) -> f64 {
            self.inner.face_change_progress(id, t)
        }

        /// (R5/M21) Capsule `id`'s change frame at progress `p`, the face at page
        /// (`left`, `top`) (JSON; `null` without a change).
        pub fn face_change_frame(&self, id: &str, p: f64, left: f64, top: f64) -> String {
            self.inner.face_change_frame(id, p, left, top).to_string()
        }

        /// (R5/M21) Pixels of capsule `id`'s face before its change (RGBA at the device
        /// scale; empty when none).
        pub fn face_before_rgba(&self, id: &str) -> Vec<u8> {
            self.inner.face_before_rgba(id)
        }

        /// (R5/M21) The faces' latest changes as facts (JSON `[{id, kind, label}]`).
        pub fn face_changes(&self) -> String {
            self.inner.face_changes_json().to_string()
        }

        /// A frame sample.
        pub fn metric_frame(&mut self, mode: &str, t: f64) {
            self.inner.metric_frame(mode, t);
        }

        /// A span.
        pub fn metric_span(&mut self, name: &str, t0: f64, t1: f64) {
            self.inner.metric_span(name, t0, t1);
        }

        /// A mark.
        pub fn metric_mark(&mut self, name: &str, t: f64) {
            self.inner.metric_mark(name, t);
        }

        /// Bundle sizes (JSON) and user agent.
        pub fn metric_env(&mut self, bundle_json: &str, agent: &str) {
            let b = serde_json::from_str(bundle_json).unwrap_or(serde_json::Value::Null);
            self.inner.metric_env(b, agent);
        }

        /// (R4/D) A device fact (`cores`, `device_memory_gb`, `screen_w`, `screen_h`).
        pub fn metric_device(&mut self, name: &str, value: f64) {
            self.inner.metric_device(name, value);
        }

        /// (R4/D) A memory sample: the page's JS heap (bytes, 0 if unknown); the core
        /// adds its own WASM memory.
        pub fn metric_memory(&mut self, js_heap: f64) {
            self.inner.metric_memory(js_heap);
        }

        /// (R4/D) Starts the frame and span series over.
        pub fn metric_reset(&mut self) {
            self.inner.metric_reset();
        }

        /// (RP3/T2) Takes over the page's previous core's metrics (opening timeline,
        /// series, paint path, a running comparison); see `ShellCore::metric_inherit`.
        pub fn metric_inherit(&mut self, prev: &mut Shell) {
            self.inner.metric_inherit(&mut prev.inner);
        }

        /// (RP3/S) Takes this core's metrics out, so the page can free this core before
        /// it builds the next one (`ShellCore::metric_detach`).
        pub fn metric_detach(&mut self) -> MetricsCarry {
            MetricsCarry {
                inner: self.inner.metric_detach(),
            }
        }

        /// (RP3/S) Takes over the metrics of a freed core (`ShellCore::metric_attach`).
        pub fn metric_attach(&mut self, carry: MetricsCarry) {
            self.inner.metric_attach(carry.inner);
        }

        /// (RP3/T2) The paint path in use: requested, used, fallback reason (empty: none).
        pub fn set_paint(&mut self, requested: &str, used: &str, reason: &str) {
            self.inner.set_paint(requested, used, reason);
        }

        /// (RP3/T2) The paint list (JSON, `shells/web/paint/README.md`) of capsule `id`'s
        /// face at the device scale; empty when unchanged (as `face_rgba`) or none.
        pub fn face_scene(&mut self, id: &str, force: bool) -> String {
            self.inner
                .face_paint(id, force)
                .map(|v| v.to_string())
                .unwrap_or_default()
        }

        /// (RP3/T2) The paint list of the open capsule's scene; empty when unchanged or
        /// none (as `scene_rgba`).
        pub fn scene_desc(&mut self, force: bool) -> String {
            self.inner
                .scene_paint(force)
                .map(|v| v.to_string())
                .unwrap_or_default()
        }

        /// (RP3/T2) The bytes of the font the reference renderer draws text with
        /// (IBM Plex Mono Regular, OFL): a canvas paint path registers exactly this file
        /// (`new FontFace('QpMono', core.font())`).
        pub fn font(&self) -> Vec<u8> {
            qp_render::cpu::FONT_BYTES.to_vec()
        }

        /// (R5/T3) The embedded font assets (JSON `[{id, family, weight}]`): a canvas paint
        /// path registers each (`new FontFace(family, core.font_bytes(id), {weight})`); no
        /// network.
        pub fn fonts(&self) -> String {
            ShellCore::fonts_json().to_string()
        }

        /// (R5/T3) The bytes of font asset `id` (empty: unknown).
        pub fn font_bytes(&self, id: &str) -> Vec<u8> {
            ShellCore::font_bytes(id)
        }

        /// (R5/T3) The chrome's CSS custom properties for the connection's look and theme
        /// (JSON `{name: value}`), or empty when unchanged since the last call: the page
        /// sets each on its root element verbatim.
        pub fn css(&mut self) -> String {
            self.inner
                .take_css()
                .map(|v| v.to_string())
                .unwrap_or_default()
        }

        /// (RP3/M12b) The `canvas` paint path's variant (the page's `?canvas=` value as
        /// it is: `run`, `nobatch`, `full`, comma-separated; empty = the default).
        pub fn canvas_config(&mut self, value: &str) {
            self.canvas
                .configure(crate::shell::paint_canvas::Config::parse(value));
        }

        /// (RP3/M12b) The Canvas 2D command stream of capsule `id`'s face
        /// (`shells/web/paint/README.md`); empty when unchanged (as `face_rgba`) or none.
        /// `force`: the canvas is new, paint all of it.
        pub fn canvas_face(&mut self, id: &str, force: bool) -> Vec<f32> {
            self.canvas.face(&mut self.inner, id, force)
        }

        /// (RP3/M12b) The command stream of the open capsule's scene (as `scene_rgba`).
        pub fn canvas_scene(&mut self, force: bool) -> Vec<f32> {
            self.canvas.scene(&mut self.inner, force)
        }

        /// (RP3/M12b) The glyph atlas rows (straight RGBA) the last command stream's
        /// `ATLAS` command replaces (`?canvas=atlas`).
        pub fn canvas_atlas_px(&mut self) -> Vec<u8> {
            self.canvas.atlas_px()
        }

        /// (RP3/M12b) The string table (JSON array) of the last command stream.
        pub fn canvas_strings(&self) -> String {
            self.canvas.strings_json()
        }

        /// (RP3/T2) Starts the automatic paint-path comparison (`{paths, current,
        /// phase_ms}` JSON) at `t` ms.
        pub fn bench_start(&mut self, config_json: &str, t: f64) {
            let c = serde_json::from_str(config_json).unwrap_or(serde_json::Value::Null);
            self.inner.bench_start(&c, t);
        }

        /// (RP3/T2) One frame of the comparison: the page's actions (JSON), empty when
        /// none runs.
        pub fn bench_tick(&mut self, t: f64, scroll_h: f64, client_h: f64) -> String {
            match self.inner.bench_tick(t, scroll_h, client_h) {
                serde_json::Value::Null => String::new(),
                v => v.to_string(),
            }
        }

        /// (RP3/T2) The comparison's table for the page (JSON; `null` until finished).
        pub fn bench_rows(&self) -> String {
            self.inner.bench_rows().to_string()
        }

        /// (RP3/T2) The comparison's summary (JSON; `null` until finished).
        pub fn bench_summary(&self) -> String {
            self.inner.bench_summary().to_string()
        }

        /// The metrics summary (JSON).
        pub fn metrics(&self) -> String {
            self.inner.metrics_json().to_string()
        }

        /// The metrics as rows (JSON).
        pub fn metrics_rows(&self) -> String {
            self.inner.metrics_rows().to_string()
        }

        /// Every message sent so far (JSON array).
        pub fn sent(&self) -> String {
            serde_json::Value::Array(self.inner.sent().to_vec()).to_string()
        }

        /// Parity digests (JSON).
        pub fn digests(&mut self) -> String {
            self.inner.digests().to_string()
        }

        /// PNG bytes of RGBA pixels (the core's encoder: byte-identical on every target).
        pub fn png(&self, rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
            qp_render::encode_png(rgba, w, h).unwrap_or_default()
        }

        /// (RP3/M12c) Builds capsule `id`'s face as GPU triangles (`shell/paint_gpu.rs`);
        /// false when unchanged (as `face_rgba`) or none.
        pub fn gpu_face(&mut self, id: &str, force: bool) -> bool {
            self.inner.face_gpu(id, force, &mut self.gpu)
        }

        /// (RP3/M12c) Builds the open capsule's scene as GPU triangles; false when
        /// unchanged (as `scene_rgba`) or none.
        pub fn gpu_scene(&mut self, force: bool) -> bool {
            self.inner.scene_gpu(force, &mut self.gpu)
        }

        /// (RP3/M12c) The last built frame's vertex bytes.
        pub fn gpu_vertices(&self) -> Vec<u8> {
            self.gpu.vertices().to_vec()
        }

        /// (RP3/M12c) A vertex buffer size that holds the last frame (bytes).
        pub fn gpu_capacity(&self) -> u32 {
            self.gpu.capacity()
        }

        /// (RP3/M12c) The last frame's draws, column `k`: 0 kinds, 1 first vertices,
        /// 2 vertex counts.
        pub fn gpu_draws(&self, k: usize) -> Vec<u32> {
            self.gpu.draw_column(k)
        }

        /// (RP3/M12c) The last frame's clear colour (premultiplied RGBA, 0..1).
        pub fn gpu_clear(&self) -> Vec<f64> {
            self.gpu.background().to_vec()
        }

        /// (RP3/M12c) The glyph atlas size `[w, h]` (an `r8unorm` texture).
        pub fn gpu_atlas_size(&self) -> Vec<u32> {
            vec![
                crate::shell::paint_gpu::ATLAS_W,
                crate::shell::paint_gpu::ATLAS_H,
            ]
        }

        /// (RP3/M12c) Atlas rows to upload, `[y, width, height]` (empty: none); `full`
        /// (a new texture) names every row.
        pub fn gpu_atlas_dirty(&mut self, full: bool) -> Vec<u32> {
            if full {
                self.gpu.mark_atlas();
            }
            self.gpu.atlas_dirty()
        }

        /// (RP3/M12c) The texels of those rows; clears the record.
        pub fn gpu_atlas_rows(&mut self) -> Vec<u8> {
            self.gpu.take_atlas_rows()
        }
    }
}
