//! RP3/T1 probe build of the demo shell's core: the R4 binding (`qp-wasm` `shell_binding`,
//! copied verbatim at 6877bda) plus `probe_*` methods that split one paint into its parts.
//! Experiment only (rounds/RP3/CALIBRATION.md); served with an instrumented copy of
//! main.js (main-probe.js). Exported to JS as `ProbeShell`: the dependency's own `Shell`
//! class is linked into the same module too.
#![allow(clippy::all)]

pub use binding::Shell;

mod binding {
    use wasm_bindgen::prelude::*;

    use qp_wasm::shell::ShellCore;

    fn err(e: impl std::fmt::Display) -> JsError {
        JsError::new(&e.to_string())
    }

    /// The demo shell's core.
    #[wasm_bindgen(js_name = ProbeShell)]
    pub struct Shell {
        inner: ShellCore,
        probe_scene: Option<qp_schema::scene::SceneDesc>,
        probe_px: Vec<u8>,
    }

    #[wasm_bindgen(js_class = ProbeShell)]
    impl Shell {
        /// From the host's `start` frame (`{conn, principal, lang, data, events}`).
        ///
        /// # Errors
        /// A malformed frame.
        #[wasm_bindgen(constructor)]
        pub fn new(start_json: &str) -> Result<Shell, JsError> {
            let v: serde_json::Value = serde_json::from_str(start_json).map_err(err)?;
            let data = serde_json::from_value(v["data"].clone()).map_err(err)?;
            let events: Vec<qp_schema::data::DataEvent> =
                serde_json::from_value(v["events"].clone()).unwrap_or_default();
            let inner = ShellCore::new(
                v["principal"].as_str().unwrap_or("arkin"),
                v["lang"].as_str().unwrap_or("tr"),
                &data,
                &events,
            )
            .map_err(err)?;
            Ok(Self { inner, probe_scene: None, probe_px: Vec::new() })
        }

        /// One frame of the host's stream; returns what changed (JSON).
        ///
        /// # Errors
        /// A malformed frame.
        pub fn on_frame(&mut self, text: &str) -> Result<String, JsError> {
            self.inner
                .on_frame(text)
                .map(|v| v.to_string())
                .map_err(err)
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
        pub fn open(&self) -> String {
            self.inner.open_json().to_string()
        }

        /// Evidence rows (JSON).
        pub fn evidence(&self) -> String {
            self.inner.evidence_json().to_string()
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
            serde_json::to_string(self.inner.look()).unwrap_or_default()
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

        /// A wheel notch.
        pub fn wheel(&mut self, dy: f64, x: f64) {
            self.inner.wheel(dy, x);
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
    }

    /// RP3/T1 probe: the paint path split into its parts, each a separate call so the
    /// page can time it (`performance.now()` around each): scene build (grammar compile in
    /// the core), rasterisation (tiny-skia + demultiply, kept inside WASM), copy out
    /// (the `Vec<u8>` handed to JS). Experiment only; not part of the shell's surface.
    #[wasm_bindgen(js_class = ProbeShell)]
    impl Shell {
        /// Builds the open scene (or face `id` when not empty) and keeps it; returns
        /// whether there is one.
        pub fn probe_build(&mut self, id: &str) -> bool {
            let s = if id.is_empty() { self.inner.scene() } else { self.inner.face(id) };
            let ok = s.is_some();
            self.probe_scene = s;
            ok
        }

        /// Rasterises the kept scene at `scale` and keeps the pixels; returns their length.
        pub fn probe_raster(&mut self, scale: u32) -> u32 {
            let Some(s) = &self.probe_scene else { return 0 };
            self.probe_px = qp_render::render_scene_rgba_scaled(s, scale).unwrap_or_default();
            u32::try_from(self.probe_px.len()).unwrap_or(0)
        }

        /// The kept pixels, copied out to JS.
        pub fn probe_take(&self) -> Vec<u8> {
            self.probe_px.clone()
        }

        /// The kept scene's size `[w, h]` (CSS px) as JSON.
        pub fn probe_size(&self) -> String {
            self.probe_scene
                .as_ref()
                .map_or_else(|| "null".into(), |s| format!("[{},{}]", s.width, s.height))
        }

        /// The device scale the core paints at.
        pub fn probe_scale(&self) -> u32 {
            self.inner.scale()
        }
    }
}
