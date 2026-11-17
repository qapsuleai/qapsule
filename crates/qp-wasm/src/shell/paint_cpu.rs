//! (RP3/M12a) The `cpu` paint path's core side (`shells/web/paint/cpu.js`): the reference
//! renderer's own rasteriser (tiny-skia + fontdue, [`qp_render::cpu::draw_prim`]) without
//! the needless work.
//!
//! - **Retained surfaces, dirty rectangles, prefix image** ([`qp_render::retained`]): each
//!   face and the open scene keep their last frame; only the region the changed
//!   primitives touch is painted again, starting from the image of the unchanged leading
//!   primitives (grid, history bars). The pixels equal the reference renderer's byte for
//!   byte.
//! - **No copy across the boundary:** the shell reads the frame straight out of the WASM
//!   memory (`cpu_memory`, the pointer in the answer) and `putImageData`s only the
//!   changed rectangle; nothing is demultiplied when the background is opaque (every
//!   alpha is then 255, so premultiplied = straight).
//! - **Canvas kept:** the answer tells the shell to keep the canvas when its size did not
//!   change (`canvas.width = …` reallocates and clears the backing store).
//! - **Reduced scale while moving:** while a drag or pinch on the open chart is in
//!   progress the scene is painted at scale 1; when it ends (`cpu_moving` false) the next
//!   paint is at the device scale, so the design at rest is the reference's.
//!
//! Every technique can be switched off (`cpu_options`) to measure its own share.

use std::collections::BTreeMap;

use qp_render::cpu::scale_prim;
use qp_render::retained::{Retained, RetainedOpts, Scratch};
use qp_schema::scene::{Prim, SceneDesc};

use super::ShellCore;

/// Faces whose last frame is kept (least recently painted dropped first). A phone shows
/// about four faces at a time; a dropped face is painted whole the next time.
pub const FACE_KEEP: usize = 10;

#[cfg(target_arch = "wasm32")]
mod clock {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        /// `performance.now()` (ms), for the path's own spans.
        #[wasm_bindgen(js_namespace = performance, js_name = now)]
        pub fn now() -> f64;
    }
}

/// The techniques in use (all on by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuOpts {
    /// Paint only the changed rectangle.
    pub dirty: bool,
    /// Keep the unchanged prefix's image.
    pub layer: bool,
    /// Scale 1 while a chart gesture is in progress.
    pub reduce: bool,
    /// Keep the canvas when its size is unchanged.
    pub reuse: bool,
    /// Hand the pixels over as a view into the WASM memory (off: a copy, as `ref`).
    pub view: bool,
    /// Faces kept.
    pub keep: usize,
}

impl Default for CpuOpts {
    fn default() -> Self {
        Self {
            dirty: true,
            layer: true,
            reduce: true,
            reuse: true,
            view: true,
            keep: FACE_KEEP,
        }
    }
}

impl CpuOpts {
    /// From a comma-separated list: `nodirty`, `nolayer`, `noreduce`, `noreuse`, `noview`,
    /// `off` (all five off), `keep=<n>`. Unknown words are ignored.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        let mut o = Self::default();
        for w in s.split(',').map(str::trim) {
            match w {
                "nodirty" => o.dirty = false,
                "nolayer" => o.layer = false,
                "noreduce" => o.reduce = false,
                "noreuse" => o.reuse = false,
                "noview" => o.view = false,
                "off" => {
                    o = Self {
                        keep: o.keep,
                        ..Self::off()
                    }
                }
                _ => {
                    if let Some(n) = w.strip_prefix("keep=").and_then(|n| n.parse().ok()) {
                        o.keep = n;
                    }
                }
            }
        }
        o
    }

    fn off() -> Self {
        Self {
            dirty: false,
            layer: false,
            reduce: false,
            reuse: false,
            view: false,
            keep: FACE_KEEP,
        }
    }

    fn retained(self) -> RetainedOpts {
        RetainedOpts {
            dirty: self.dirty,
            layer: self.layer,
        }
    }
}

/// Which surface the last answer was about.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shown {
    Face(String),
    Scene,
    Low,
}

/// The path's state, one per core.
#[derive(Debug, Default)]
pub struct CpuPaint {
    opts: CpuOpts,
    faces: BTreeMap<String, (u64, Retained)>,
    face_scratch: Scratch,
    scene: Retained,
    low: Retained,
    scene_scratch: Scratch,
    /// The scene canvas shows the reduced-scale frame (a full-scale paint is owed).
    low_shown: bool,
    clock: u64,
    last: Option<Shown>,
    /// Straight RGBA for a non-opaque frame or a copy hand-over.
    out: Vec<u8>,
}

/// The answer to the shell: `[ptr, len, w, h, dx, dy, dw, dh, settle, reuse, view]`
/// (`settle` 1: painted at the reduced scale, ask again when `cpu_moving` is false).
fn answer(
    ptr: usize,
    len: usize,
    w: u32,
    h: u32,
    d: [u32; 4],
    settle: bool,
    o: CpuOpts,
) -> Vec<f64> {
    #[allow(clippy::cast_precision_loss)]
    let (p, l) = (ptr as f64, len as f64);
    vec![
        p,
        l,
        f64::from(w),
        f64::from(h),
        f64::from(d[0]),
        f64::from(d[1]),
        f64::from(d[2]),
        f64::from(d[3]),
        f64::from(u8::from(settle)),
        f64::from(u8::from(o.reuse)),
        f64::from(u8::from(o.view)),
    ]
}

fn device_prims(scene: &SceneDesc, s: u32) -> Vec<Prim> {
    scene.prims().map(|p| scale_prim(p, s)).collect()
}

impl ShellCore {
    fn cpu_now(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            clock::now()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            0.0
        }
    }

    fn cpu_span(&mut self, name: &str, t0: f64, t1: f64) {
        if cfg!(target_arch = "wasm32") {
            self.metric_span(name, t0, t1);
        }
    }

    /// (RP3/M12a) Sets the `cpu` path's techniques ([`CpuOpts::parse`]) and drops its
    /// retained surfaces (the shell gives it fresh canvases).
    pub fn cpu_options(&mut self, s: &str) {
        self.cpu = CpuPaint {
            opts: CpuOpts::parse(s),
            ..CpuPaint::default()
        };
    }

    /// (RP3/M12a) A drag or pinch on the open chart is in progress.
    #[must_use]
    pub fn cpu_moving(&self) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|g| g.target == "scene" && (g.dragging || g.pinching) && !g.cancelled)
    }

    /// (RP3/M12a) Paints capsule `id`'s face on its retained surface; the answer (see
    /// [`answer`]) or empty when nothing changed (and not `force`) or there is no face.
    pub fn cpu_face(&mut self, id: &str, force: bool) -> Vec<f64> {
        let t0 = self.cpu_now();
        let Some(scene) = self.face(id) else {
            return Vec::new();
        };
        let s = self.scale;
        let prims = device_prims(&scene, s);
        let (w, h) = (scene.width * s, scene.height * s);
        let t1 = self.cpu_now();
        let c = &mut self.cpu;
        c.clock += 1;
        let opts = c.opts;
        if !c.faces.contains_key(id) && c.faces.len() >= opts.keep.max(1) {
            if let Some(old) = c
                .faces
                .iter()
                .min_by_key(|(_, (t, _))| *t)
                .map(|(k, _)| k.clone())
            {
                c.faces.remove(&old);
            }
        }
        let clock = c.clock;
        let entry = c
            .faces
            .entry(id.to_owned())
            .or_insert_with(|| (0, Retained::default()));
        entry.0 = clock;
        let changed = entry
            .1
            .paint(
                w,
                h,
                scene.background,
                prims,
                &mut c.face_scratch,
                opts.retained(),
            )
            .unwrap_or(None);
        let t2 = self.cpu_now();
        self.cpu_span("cpu_compile", t0, t1);
        self.cpu_span("cpu_raster", t1, t2);
        let d = match changed {
            _ if force => [0, 0, w, h],
            Some(d) => d,
            None => return Vec::new(),
        };
        self.cpu.last = Some(Shown::Face(id.to_owned()));
        self.cpu_hand_over(w, h, d, false)
    }

    /// (RP3/M12a) Paints the open capsule's scene; the answer or empty (as
    /// [`ShellCore::cpu_face`]). While [`ShellCore::cpu_moving`] (and the `reduce`
    /// technique is on) the frame is painted at scale 1 and the answer asks to be called
    /// again when the gesture ends.
    pub fn cpu_scene(&mut self, force: bool) -> Vec<f64> {
        let t0 = self.cpu_now();
        let Some(scene) = self.scene() else {
            return Vec::new();
        };
        let low = self.cpu.opts.reduce && self.scale > 1 && self.cpu_moving();
        let s = if low { 1 } else { self.scale };
        let prims = device_prims(&scene, s);
        let (w, h) = (scene.width * s, scene.height * s);
        let t1 = self.cpu_now();
        let c = &mut self.cpu;
        let opts = c.opts.retained();
        let surface = if low { &mut c.low } else { &mut c.scene };
        let changed = surface
            .paint(w, h, scene.background, prims, &mut c.scene_scratch, opts)
            .unwrap_or(None);
        // Back from the reduced scale: the canvas holds the small frame; repaint whole.
        let owed = !low && c.low_shown;
        let t2 = self.cpu_now();
        self.cpu_span("cpu_compile", t0, t1);
        self.cpu_span("cpu_raster", t1, t2);
        let d = match changed {
            _ if force || owed => [0, 0, w, h],
            Some(d) => d,
            None => return Vec::new(),
        };
        self.cpu.low_shown = low;
        self.cpu.last = Some(if low { Shown::Low } else { Shown::Scene });
        self.cpu_hand_over(w, h, d, low)
    }

    /// The answer for the surface just painted (`self.cpu.last`): a view into the WASM
    /// memory when possible, otherwise the straight pixels in `out` (`cpu_pixels`).
    fn cpu_hand_over(&mut self, w: u32, h: u32, d: [u32; 4], settle: bool) -> Vec<f64> {
        let c = &mut self.cpu;
        let opts = c.opts;
        let r = match &c.last {
            Some(Shown::Face(id)) => c.faces.get(id).map(|(_, r)| r),
            Some(Shown::Scene) => Some(&c.scene),
            Some(Shown::Low) => Some(&c.low),
            None => None,
        };
        let Some(r) = r else { return Vec::new() };
        let Some(pm) = r.pixmap() else {
            return Vec::new();
        };
        let data = pm.data();
        if opts.view && r.background()[3] == 255 {
            // Opaque: every alpha is 255 (source-over keeps an opaque destination
            // opaque), so the premultiplied frame is already straight RGBA.
            return answer(data.as_ptr() as usize, data.len(), w, h, d, settle, opts);
        }
        c.out = qp_render::cpu::demultiply(data);
        answer(c.out.as_ptr() as usize, c.out.len(), w, h, d, settle, opts)
    }

    /// (RP3/M12a) The straight pixels of the last answer, as a copy (the `noview`
    /// technique switch; with a view the shell reads the memory directly).
    pub fn cpu_pixels(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.cpu.out)
    }

    /// (RP3/M12a) Bytes held by the path's surfaces and scratch memory.
    #[must_use]
    pub fn cpu_bytes(&self) -> usize {
        let c = &self.cpu;
        c.faces.values().map(|(_, r)| r.bytes()).sum::<usize>()
            + c.scene.bytes()
            + c.low.bytes()
            + c.face_scratch.bytes()
            + c.scene_scratch.bytes()
            + c.out.len()
    }

    /// (RP3/M12a) What the last paint of the face `id` (or, for `None`, the scene) did:
    /// `{full, drawn, area, layer_built, layer_used}` (tests and measurements).
    #[must_use]
    pub fn cpu_stats(&self, id: Option<&str>) -> Option<qp_render::retained::PaintStats> {
        match id {
            Some(id) => self.cpu.faces.get(id).map(|(_, r)| r.stats()),
            None => Some(self.cpu.scene.stats()),
        }
    }

    /// (RP3/M12a) The pixels of the last answer's surface (premultiplied; tests).
    #[must_use]
    pub fn cpu_frame(&self) -> Option<Vec<u8>> {
        let c = &self.cpu;
        let r = match c.last.as_ref()? {
            Shown::Face(id) => &c.faces.get(id)?.1,
            Shown::Scene => &c.scene,
            Shown::Low => &c.low,
        };
        r.pixmap().map(|p| p.data().to_vec())
    }
}
