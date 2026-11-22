//! The wasm-bindgen bindings by topic (RG1/M1a: moved out of `lib.rs`; `lib.rs` re-exports
//! them under their old paths, the exported JavaScript names are unchanged).

pub(crate) mod compute;
pub(crate) mod render;

#[cfg(any(feature = "spike-vello-cpu", feature = "spike-vello-gpu"))]
pub(crate) mod spike;

/// (R5/G) The wasm-bindgen face of [`crate::gallery::GalleryCore`]: the gallery page calls
/// only these methods (allow-list in `shells/web/ARCH.md`, `arch-check.mjs`).
#[cfg(feature = "replica")]
pub(crate) mod gallery;

/// R4/T5: the wasm-bindgen face of [`crate::shell::ShellCore`]. The web shell calls only
/// these methods (the allow-list is `shells/web/ARCH.md`): JSON text and pixels in and out.
#[cfg(feature = "replica")]
pub(crate) mod shell;

#[cfg(feature = "replica")]
pub(crate) mod replica;
