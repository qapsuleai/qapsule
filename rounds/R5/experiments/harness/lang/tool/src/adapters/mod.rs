//! Language adapters (SPEC §1). One file per language; each only calls product
//! functions (or, for `bozuk`, transforms another adapter's text).

pub mod bozuk;
pub mod marla;
pub mod r4;
pub mod selvi;
