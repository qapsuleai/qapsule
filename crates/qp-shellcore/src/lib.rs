//! Qapsule shell core (`ShellCore`; RG1/M1a, moved out of `qp-wasm`).
//!
//! The pure-Rust core of the presentation shell over the `qp-session` replica: the shell
//! core (`shell`), the reader core (`viewer`), the shell-less native driver (`drive`) and
//! the gallery page's core (`gallery`). `qp-wasm` binds these to JavaScript and re-exports
//! them under their old paths (`qp_wasm::shell`, …).

/// R4/T5: the demo shell's core.
pub mod shell;

/// RP3/M15: the device as a viewer (`docs/PROTOCOL.md` §22).
pub mod viewer;

/// R4/T5: shell-less driving and the parity check (native only).
#[cfg(not(target_arch = "wasm32"))]
pub mod drive;

/// (R5/G) The comparison gallery: the host's painting of the candidates (native) and the
/// gallery page's core (`shells/web/gallery.js`).
pub mod gallery;
