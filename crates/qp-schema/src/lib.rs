//! Qapsule schema v0. Rust types are the single source of truth.
//!
//! JSON Schema is generated from these types with [`json_schemas`] /
//! [`export_schemas`]; it is never written by hand.

pub mod capsule;
pub mod comment;
pub mod data;
mod export;
pub mod grammar;
pub mod market;
pub mod protocol;
pub mod scene;
mod types;
mod validation;

pub use export::{export_schemas, json_schema, json_schemas, root_type_names};
pub use grammar::{Encoding, Layer, Mark, Panel, Scale, ScaleKind, View};
pub use types::*;
pub use validation::{parse_bar_series, validate_bars};
#[cfg(feature = "validate")]
pub use validation::{validate, validate_with_schema};
