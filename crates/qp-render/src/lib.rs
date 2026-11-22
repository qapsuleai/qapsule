//! Qapsule render core.
//!
//! A deterministic headless *reference renderer* (tiny-skia + fontdue, scalar paths only)
//! that rasterizes a scene description ([`SceneDesc`], produced by the grammar compiler
//! `qp-grammar`) and nothing else: no layout, no data access, no study computation here.
//! The same code is compiled to wasm32 by `qp-wasm`. See `SPIKE.md` for the render-stack
//! experiment and its determinism rules.
//!
//! Output pixel format of [`render_scene_rgba`]: **straight (non-premultiplied) RGBA8**,
//! row-major, top-left origin, `width * height * 4` bytes — exactly what
//! `ImageData`/`putImageData` expects. (The background is opaque, so every pixel currently
//! has alpha 255 and straight == premultiplied, but callers must not rely on that.)

mod bars;
pub mod cpu;
mod error;
pub mod fonts;
mod output;
pub mod retained;
pub mod scene;
#[cfg(any(feature = "spike-vello-cpu", feature = "spike-vello-gpu"))]
pub mod spikes;

pub use bars::parse_bars;
pub use error::RenderError;
pub use output::{
    encode_png, render_scene_png, render_scene_rgba, render_scene_rgba_scaled, render_stress_rgba,
    DEFAULT_HEIGHT, DEFAULT_WIDTH,
};
pub use qp_schema::scene::SceneDesc;
pub use qp_schema::{Bar, Series};
