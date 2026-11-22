//! Schema v0 types. Names follow `docs/CONCEPTS.md`.
//!
//! Conventions:
//! - Field names are `snake_case`; enums are tagged with `kind` (or `op` for commands).
//! - Timestamps are `i64` epoch milliseconds, UTC.
//! - Prices and computed values are `f64`.
//! - Maps are `BTreeMap` so serialization order is deterministic.
//! - References between items are by `id` string.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grammar::View;

mod document;
mod series;

pub use document::*;
pub use series::*;

/// Schema version written into every [`Document`].
pub const SCHEMA_VERSION: &str = "0";

/// Epoch milliseconds, UTC.
pub type TimestampMs = i64;
