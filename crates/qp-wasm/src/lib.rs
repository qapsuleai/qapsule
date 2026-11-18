//! Qapsule web binding.
//!
//! A thin wasm-bindgen wrapper: every function calls the same core code (`qp-compute`,
//! `qp-grammar`, `qp-render`) that the native CLI and the tests use. No drawing, layout or
//! compute logic lives here: the binding only wires the R0 view together (study values
//! from the compute core -> scene description from the grammar compiler -> pixels from
//! the reference renderer, which draws the scene and nothing else).
//!
//! RP2/T8 (feature `replica`): `Replica` exposes `qp_session::replica::Replica` (the same engine code as the
//! authority: document mirror, data plane, live path, display rule, digests).

use std::collections::BTreeMap;

use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::scene::SceneDesc;
use qp_schema::Series;
use wasm_bindgen::prelude::*;

fn js_err(e: qp_render::RenderError) -> JsError {
    JsError::new(&e.to_string())
}

/// The R0 view (`rounds/R2/tasks/VIEWS.md` V0): candles of `px` and the SMA(20) line of
/// `sma20` in one linear panel.
const R0_VIEW: &str = r#"{"id":"r0","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}}],
  "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
            {"id":"sma","mark":"line","series":"sma20","panel":0}]}"#;

/// Compiles the R0 view over `series` at `width` x `height` (level of detail on).
///
/// # Errors
/// A series without bars, a study failure, or a compile error (e.g. size out of range).
fn r0_scene(series: &Series, width: u32, height: u32) -> Result<SceneDesc, JsError> {
    let bars = series
        .bars()
        .ok_or_else(|| JsError::new("series has no embedded bars"))?
        .to_vec();
    let params = BTreeMap::from([("period".to_owned(), 20.0)]);
    let line = qp_compute::compute_bars("sma", &params, qp_compute::Field::Close, &bars)
        .map_err(compute_err)?
        .outputs
        .into_values()
        .next()
        .unwrap_or_default();
    let provider = MemProvider::new()
        .with_bars("px", bars, series.symbol(), series.interval.as_deref())
        .with_values("sma20", "px", vec![("sma", line)], "sma", &["sma"])
        .with_label("sma20", "SMA20");
    let view = serde_json::from_str(R0_VIEW).map_err(|e| JsError::new(&e.to_string()))?;
    compile(&view, &provider, &CompileOptions::new(width, height))
        .map_err(|e| JsError::new(&e.to_string()))
}

fn r0_scene_json(json: &str, width: u32, height: u32) -> Result<SceneDesc, JsError> {
    let series = qp_render::parse_bars(json).map_err(js_err)?;
    r0_scene(&series, width, height)
}

fn compute_err(e: qp_compute::ComputeError) -> JsError {
    JsError::new(&e.to_string())
}

/// Runs a built-in study (`sma`, `ema`, `rsi`, `macd`, `bollinger`) over a bar series
/// document and returns a `StudyOutput` JSON string (one value per bar, warmup `null`).
/// `params_json` is an object of numeric parameters (`{}` for defaults); `field` is the
/// bar field to read (`close`, `open`, ...).
///
/// # Errors
/// Invalid JSON, bars, study name, parameter or field.
#[wasm_bindgen]
pub fn compute(
    study: &str,
    params_json: &str,
    field: &str,
    bars_json: &str,
) -> Result<String, JsError> {
    let params: std::collections::BTreeMap<String, f64> = serde_json::from_str(params_json)
        .map_err(|e| JsError::new(&format!("invalid params JSON: {e}")))?;
    let field = qp_compute::Field::parse(field).map_err(compute_err)?;
    let series = qp_schema::parse_bar_series(bars_json).map_err(|e| JsError::new(&e))?;
    let bars = series.bars().unwrap_or_default();
    let out = qp_compute::compute_bars(study, &params, field, bars).map_err(compute_err)?;
    serde_json::to_string(&out).map_err(|e| JsError::new(&e.to_string()))
}

/// Benchmark helper: one sweep of the R1 benchmark grid (1,000 parameter sets) of
/// `study` over `bars` deterministic closes (`qp_compute::sweep::bench_closes`). Returns
/// the sum of every variant's last value so the work cannot be optimized away; time it
/// from JavaScript.
///
/// # Errors
/// Unknown study.
#[wasm_bindgen]
pub fn sweep_bench(study: &str, bars: u32, seed: u32) -> Result<f64, JsError> {
    let x = qp_compute::sweep::bench_closes(bars as usize, u64::from(seed));
    let grid = qp_compute::sweep::bench_grid(study);
    let mut sink = 0.0;
    let last = x.len().saturating_sub(1);
    qp_compute::sweep::sweep_each(study, &grid, &x, |_, c| sink += c.columns[0][last])
        .map_err(compute_err)?;
    Ok(sink)
}

/// FNV-1a digest (hex) of all outputs of the R1 benchmark grid sweep, as printed by
/// `cargo run --release -p qp-compute --example sweep_bench`.
///
/// # Errors
/// Unknown study.
#[wasm_bindgen]
pub fn sweep_digest(study: &str, bars: u32, seed: u32) -> Result<String, JsError> {
    let x = qp_compute::sweep::bench_closes(bars as usize, u64::from(seed));
    let grid = qp_compute::sweep::bench_grid(study);
    let mut h = qp_compute::sweep::FNV_OFFSET;
    qp_compute::sweep::sweep_each(study, &grid, &x, |_, c| {
        h = qp_compute::sweep::fnv1a_columns(h, c);
    })
    .map_err(compute_err)?;
    Ok(format!("{h:016x}"))
}

/// The cross-target compute digest (hex), same as
/// `cargo run --release -p qp-compute --example digest`.
#[wasm_bindgen]
#[must_use]
pub fn compute_digest() -> String {
    format!("{:016x}", qp_compute::sweep::cross_target_digest())
}

/// Default output width (mirrors `qp_render::DEFAULT_WIDTH`).
#[wasm_bindgen]
#[must_use]
pub fn default_width() -> u32 {
    qp_render::DEFAULT_WIDTH
}

/// Default output height (mirrors `qp_render::DEFAULT_HEIGHT`).
#[wasm_bindgen]
#[must_use]
pub fn default_height() -> u32 {
    qp_render::DEFAULT_HEIGHT
}

/// Renders a bars JSON document (the R0 view: candles + SMA(20)) to straight (non-premultiplied) RGBA8 pixels, ready for
/// `new ImageData(new Uint8ClampedArray(bytes), width, height)`.
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[wasm_bindgen]
pub fn render_rgba(json: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_scene_rgba(&r0_scene_json(json, width, height)?).map_err(js_err)
}

/// Renders a bars JSON document (the R0 view) to PNG bytes (same encoder settings as native).
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[wasm_bindgen]
pub fn render_png(json: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_scene_png(&r0_scene_json(json, width, height)?).map_err(js_err)
}

/// Renders `n` deterministic synthetic bars (benchmark helper; see `SPIKE.md`).
///
/// # Errors
/// An out-of-range size or `n == 0`.
#[wasm_bindgen]
pub fn render_synthetic_rgba(
    n: u32,
    seed: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, JsError> {
    let bars = qp_render::synthetic_bars(n as usize, u64::from(seed));
    qp_render::render_scene_rgba(&r0_scene(&bars, width, height)?).map_err(js_err)
}

/// Renders the determinism stress scene (spike only; golden `fixtures/golden/stress.png`).
///
/// # Errors
/// Allocation failure.
#[wasm_bindgen]
pub fn render_stress_rgba(width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_stress_rgba(width, height).map_err(js_err)
}

/// Spike (feature `spike-vello-cpu`): render a bars JSON document with the vello_cpu backend.
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[cfg(feature = "spike-vello-cpu")]
#[wasm_bindgen]
pub fn render_rgba_vello_cpu(
    json: &str,
    width: u32,
    height: u32,
    quality: bool,
) -> Result<Vec<u8>, JsError> {
    use qp_render::spikes::vello_cpu::{rasterize, Config};
    let scene = r0_scene_json(json, width, height)?;
    rasterize(
        &scene,
        Config {
            quality,
            ..Config::default()
        },
    )
    .map_err(js_err)
}

/// Spike (feature `spike-vello-cpu`): the stress scene through vello_cpu.
///
/// # Errors
/// Out-of-range size.
#[cfg(feature = "spike-vello-cpu")]
#[wasm_bindgen]
pub fn render_stress_rgba_vello_cpu(
    width: u32,
    height: u32,
    quality: bool,
) -> Result<Vec<u8>, JsError> {
    use qp_render::spikes::vello_cpu::{rasterize, Config};
    let scene = qp_render::scene::stress_scene(width, height);
    rasterize(
        &scene,
        Config {
            quality,
            ..Config::default()
        },
    )
    .map_err(js_err)
}

/// Spike (feature `spike-vello-cpu`): `n` synthetic bars through vello_cpu (u8 pipeline).
///
/// # Errors
/// Out-of-range size or `n == 0`.
#[cfg(feature = "spike-vello-cpu")]
#[wasm_bindgen]
pub fn render_synthetic_rgba_vello_cpu(
    n: u32,
    seed: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, JsError> {
    use qp_render::spikes::vello_cpu::{rasterize, Config};
    let bars = qp_render::synthetic_bars(n as usize, u64::from(seed));
    let scene = r0_scene(&bars, width, height)?;
    rasterize(&scene, Config::default()).map_err(js_err)
}

#[cfg(feature = "spike-vello-gpu")]
mod vello_gpu_spike {
    use std::cell::RefCell;

    use qp_render::spikes::vello_gpu::{AaConfig, GpuRenderer};
    use wasm_bindgen::prelude::*;

    thread_local! {
        static GPU: RefCell<Option<GpuRenderer>> = const { RefCell::new(None) };
    }

    fn aa(mode: u8) -> AaConfig {
        match mode {
            1 => AaConfig::Msaa8,
            2 => AaConfig::Msaa16,
            _ => AaConfig::Area,
        }
    }

    /// Spike (feature `spike-vello-gpu`): create the WebGPU device and vello pipelines.
    ///
    /// # Errors
    /// No WebGPU adapter/device.
    #[wasm_bindgen]
    pub async fn vello_gpu_init() -> Result<(), JsError> {
        let r = GpuRenderer::new_async().await.map_err(super::js_err)?;
        GPU.with(|g| *g.borrow_mut() = Some(r));
        Ok(())
    }

    async fn with_gpu(scene: qp_schema::scene::SceneDesc, mode: u8) -> Result<Vec<u8>, JsError> {
        // Take the renderer out for the duration of the await (no RefCell borrow held).
        let mut r = GPU
            .with(|g| g.borrow_mut().take())
            .ok_or_else(|| JsError::new("call vello_gpu_init first"))?;
        let out = r.render_async(&scene, aa(mode)).await;
        GPU.with(|g| *g.borrow_mut() = Some(r));
        out.map_err(super::js_err)
    }

    /// Spike: render a bars JSON document with vello on WebGPU; `aa`: 0 area, 1 msaa8, 2 msaa16.
    ///
    /// # Errors
    /// Invalid input, missing init, or GPU failure.
    #[wasm_bindgen]
    pub async fn vello_gpu_render_rgba(
        json: String,
        width: u32,
        height: u32,
        aa: u8,
    ) -> Result<Vec<u8>, JsError> {
        let scene = super::r0_scene_json(&json, width, height)?;
        with_gpu(scene, aa).await
    }

    /// Spike: the stress scene with vello on WebGPU.
    ///
    /// # Errors
    /// Missing init or GPU failure.
    #[wasm_bindgen]
    pub async fn vello_gpu_render_stress_rgba(
        width: u32,
        height: u32,
        aa: u8,
    ) -> Result<Vec<u8>, JsError> {
        with_gpu(qp_render::scene::stress_scene(width, height), aa).await
    }

    /// Spike: `n` synthetic bars with vello on WebGPU.
    ///
    /// # Errors
    /// Invalid size, missing init or GPU failure.
    #[wasm_bindgen]
    pub async fn vello_gpu_render_synthetic_rgba(
        n: u32,
        seed: u32,
        width: u32,
        height: u32,
        aa: u8,
    ) -> Result<Vec<u8>, JsError> {
        let bars = qp_render::synthetic_bars(n as usize, u64::from(seed));
        let scene = super::r0_scene(&bars, width, height)?;
        with_gpu(scene, aa).await
    }
}

#[cfg(test)]
mod tests {
    /// The binding's R0 wiring (compute -> grammar -> render) reproduces the golden PNG
    /// natively; the browser step checks the same in wasm32.
    #[test]
    fn render_png_matches_golden() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let json = std::fs::read_to_string(format!("{root}/fixtures/bars/sample.json")).unwrap();
        let golden = std::fs::read(format!("{root}/fixtures/golden/sample.png")).unwrap();
        let png = super::render_png(&json, super::default_width(), super::default_height())
            .unwrap_or_else(|_| panic!("render failed"));
        assert!(
            png == golden,
            "binding render differs from fixtures/golden/sample.png"
        );
    }
}

#[cfg(feature = "replica")]
pub use replica::Replica;

/// R4/T5: the demo shell's core (feature `replica`).
#[cfg(feature = "replica")]
pub mod shell;

/// RP3/M15: the device as a viewer (feature `replica`; `docs/PROTOCOL.md` §22).
#[cfg(feature = "replica")]
pub mod viewer;

/// R4/T5: shell-less driving and the parity check (native only).
#[cfg(all(feature = "replica", not(target_arch = "wasm32")))]
pub mod drive;

#[cfg(feature = "replica")]
pub use shell_binding::Shell;

/// (R5/G) The comparison gallery: the host's painting of the candidates (native) and the
/// gallery page's core (feature `replica`; `shells/web/gallery.js`).
#[cfg(feature = "replica")]
pub mod gallery;

#[cfg(feature = "replica")]
pub use gallery_binding::Gallery;

/// (R5/G) The wasm-bindgen face of [`gallery::GalleryCore`]: the gallery page calls only
/// these methods (allow-list in `shells/web/ARCH.md`, `arch-check.mjs`).
#[cfg(feature = "replica")]
mod gallery_binding {
    use wasm_bindgen::prelude::*;

    use crate::gallery::GalleryCore;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    extern "C" {
        /// `Date.now()` (ms): the summary's date line (not compute: the device's clock).
        #[wasm_bindgen(js_namespace = Date, js_name = now)]
        fn date_now() -> f64;
    }

    /// The gallery page's core.
    #[wasm_bindgen]
    pub struct Gallery {
        inner: GalleryCore,
    }

    #[wasm_bindgen]
    impl Gallery {
        /// From the saved state (local storage; `""`: none) and the address parameters
        /// `lang`, `theme`, `pick`, `now` (as they are; `""`: absent).
        #[wasm_bindgen(constructor)]
        #[must_use]
        pub fn new(saved: &str, lang: &str, theme: &str, pick: &str, now: &str) -> Gallery {
            Self {
                inner: GalleryCore::new(saved, lang, theme, pick, now),
            }
        }

        /// The layout's address for the current language and theme.
        #[must_use]
        pub fn layout_url(&self) -> String {
            self.inner.layout_url()
        }

        /// The host's layout (JSON text).
        ///
        /// # Errors
        /// A malformed layout.
        pub fn load(&mut self, layout_json: &str) -> Result<(), JsError> {
            self.inner.load(layout_json).map_err(|e| JsError::new(&e))
        }

        /// Everything the page paints (JSON text; `GalleryCore::view`).
        #[must_use]
        pub fn view(&self) -> String {
            self.inner.view().to_string()
        }

        /// `{"topic/cand": picked}` (JSON text).
        #[must_use]
        pub fn marks(&self) -> String {
            self.inner.marks().to_string()
        }

        /// "I'd pick this" tapped.
        pub fn pick(&mut self, topic: &str, cand: &str) {
            self.inner.pick(topic, cand);
        }

        /// A candidate's note as typed.
        pub fn note(&mut self, topic: &str, cand: &str, text: &str) {
            self.inner.note(topic, cand, text);
        }

        /// The summary to copy. In the browser the date line reads the device's clock
        /// unless the address gave `now`.
        #[must_use]
        pub fn summary(&mut self) -> String {
            #[cfg(target_arch = "wasm32")]
            if !self.inner.has_now() {
                #[allow(clippy::cast_possible_truncation)]
                return self.inner.summary_at(date_now() as i64);
            }
            self.inner.summary()
        }

        /// A relative link with the same picks.
        #[must_use]
        pub fn share(&self) -> String {
            self.inner.share()
        }

        /// The state to keep in local storage.
        #[must_use]
        pub fn state(&self) -> String {
            self.inner.state()
        }

        /// The device line, from the page's raw values.
        pub fn set_device(&mut self, ua: &str, w: f64, h: f64, dpr: f64) {
            self.inner.set_device(ua, w, h, dpr);
        }

        /// Türkçe ↔ English.
        pub fn toggle_lang(&mut self) {
            self.inner.toggle_lang();
        }

        /// The theme select's value (`dark`, `light`).
        pub fn set_theme(&mut self, theme: &str) {
            self.inner.set_theme(theme);
        }

        /// `tr` or `en`.
        #[must_use]
        pub fn lang(&self) -> String {
            self.inner.lang().to_owned()
        }

        /// `dark` or `light`.
        #[must_use]
        pub fn theme(&self) -> String {
            self.inner.theme().to_owned()
        }

        /// The chrome's CSS custom properties (JSON `{name: value}`).
        #[must_use]
        pub fn css(&self) -> String {
            self.inner.css().to_string()
        }

        /// The embedded font assets `[{id, family, weight}]` (JSON text).
        #[must_use]
        pub fn fonts(&self) -> String {
            crate::shell::ShellCore::fonts_json().to_string()
        }

        /// The bytes of font asset `id`.
        #[must_use]
        pub fn font_bytes(&self, id: &str) -> Vec<u8> {
            crate::shell::ShellCore::font_bytes(id)
        }
    }
}

/// R4/T5: the wasm-bindgen face of [`shell::ShellCore`]. The web shell calls only these
/// methods (the allow-list is `shells/web/ARCH.md`): JSON text and pixels in and out.
#[cfg(feature = "replica")]
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

#[cfg(feature = "replica")]
mod replica {
    use wasm_bindgen::prelude::*;

    /// A replica of an authority's document (RP2/T8, `docs/PROTOCOL.md` §19): a thin wrapper
    /// over `qp_session::replica::Replica`. Messages and data events go in as JSON text; the
    /// display state and the client messages to send come out as JSON text.
    #[wasm_bindgen]
    pub struct Replica {
        inner: qp_session::Replica,
    }

    #[wasm_bindgen]
    impl Replica {
        /// A replica for `principal` whose own data comes from a memory provider described by
        /// `provider_json` (an `InitData`: feeds with `bars` or `synthetic`, entitlements).
        ///
        /// # Errors
        /// Invalid provider description.
        #[wasm_bindgen(constructor)]
        pub fn new(principal: &str, provider_json: &str) -> Result<Replica, JsError> {
            let v: serde_json::Value =
                serde_json::from_str(provider_json).map_err(|e| JsError::new(&e.to_string()))?;
            let p = qp_session::tape::memory_provider(&v).map_err(|e| JsError::new(&e))?;
            Ok(Self {
                inner: qp_session::Replica::with_memory(principal, p),
            })
        }

        /// Shows provisional claims on the live bar (off by default).
        pub fn set_live(&mut self, on: bool) {
            let mut o = self.inner.options();
            o.live = on;
            self.inner.set_options(o);
        }

        /// Watches a condition (`Condition` JSON) as subscription `sub`.
        ///
        /// # Errors
        /// Invalid condition JSON.
        pub fn watch(&mut self, sub: &str, condition_json: &str) -> Result<(), JsError> {
            let c =
                serde_json::from_str(condition_json).map_err(|e| JsError::new(&e.to_string()))?;
            self.inner.watch(sub, c);
            Ok(())
        }

        /// One message of the authority (`{"conn":…,"msg":…}` or the bare message).
        ///
        /// # Errors
        /// Not a server message.
        pub fn on_message(&mut self, line: &str) -> Result<(), JsError> {
            self.inner.on_line(line).map_err(|e| JsError::new(&e))
        }

        /// One data event of its own feed (`DataEvent` JSON).
        ///
        /// # Errors
        /// Invalid JSON or an event the §18.4 rules reject.
        pub fn data_event(&mut self, event_json: &str) -> Result<(), JsError> {
            let ev = serde_json::from_str(event_json).map_err(|e| JsError::new(&e.to_string()))?;
            self.inner
                .data_event(&ev)
                .map_err(|e| JsError::new(&e.to_string()))
        }

        /// Re-resolves its bindings from its provider (after a gap or a settled fetch).
        pub fn resync(&mut self) {
            self.inner.resync();
        }

        /// The client messages to send (JSON array).
        #[must_use]
        pub fn take_outbox(&mut self) -> String {
            serde_json::to_string(&self.inner.take_outbox()).unwrap_or_default()
        }

        /// The display state (JSON array).
        #[must_use]
        pub fn display(&self) -> String {
            serde_json::to_string(self.inner.display()).unwrap_or_default()
        }

        /// The display digest.
        #[must_use]
        pub fn display_digest(&self) -> String {
            self.inner.display_digest()
        }

        /// The mirrored document's digest (empty before the document arrives).
        #[must_use]
        pub fn document_digest(&self) -> String {
            self.inner.document_digest().unwrap_or_default()
        }

        /// The digest of view `view`'s scene over its own data.
        ///
        /// # Errors
        /// No document yet, unknown view or a compile error.
        pub fn view_digest(
            &mut self,
            view: &str,
            width: u32,
            height: u32,
        ) -> Result<String, JsError> {
            self.inner
                .view_digest(view, width, height)
                .map_err(|e| JsError::new(&e))
        }

        /// The mirrored log position.
        #[must_use]
        pub fn seq(&self) -> u64 {
            self.inner.seq()
        }
    }
}
