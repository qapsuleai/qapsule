//! R0 render-stack spike backends (feature-gated, comparison only). See `SPIKE.md`.

#[cfg(feature = "spike-vello-cpu")]
pub mod vello_cpu;
#[cfg(feature = "spike-vello-gpu")]
pub mod vello_gpu;
