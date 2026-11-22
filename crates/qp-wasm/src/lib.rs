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
//!
//! RG1/M1a: the bindings live in `bind/` by topic; the shell core (`shell`, `gallery`,
//! `viewer`, `drive`) is the `qp-shellcore` crate, re-exported here under its old paths.

mod bind;

pub use bind::compute::{compute, compute_digest, sweep_bench, sweep_digest};
pub use bind::render::{
    default_height, default_width, render_png, render_rgba, render_stress_rgba,
    render_synthetic_rgba,
};

#[cfg(feature = "spike-vello-cpu")]
pub use bind::spike::{
    render_rgba_vello_cpu, render_stress_rgba_vello_cpu, render_synthetic_rgba_vello_cpu,
};

#[cfg(feature = "replica")]
pub use bind::replica::Replica;

/// R4/T5: the demo shell's core (feature `replica`; `qp-shellcore`).
#[cfg(feature = "replica")]
pub use qp_shellcore::{shell, viewer};

/// R4/T5: shell-less driving and the parity check (native only).
#[cfg(all(feature = "replica", not(target_arch = "wasm32")))]
pub use qp_shellcore::drive;

#[cfg(feature = "replica")]
pub use bind::shell::Shell;

/// (R5/G) The comparison gallery (feature `replica`; `qp-shellcore`).
#[cfg(feature = "replica")]
pub use qp_shellcore::gallery;

#[cfg(feature = "replica")]
pub use bind::gallery::Gallery;
