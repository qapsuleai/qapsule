//! (R5B/T2, M28) Tool surfaces: what an MCP endpoint offers an agent, chosen with
//! `qp mcp serve --surface <name>` (`surfaces/README.md`).
//!
//! A surface is one file in this folder implementing [`ToolSurface`] (a large one keeps
//! its parts in a `<name>/` folder beside it) and one line in [`registry`]
//! (`registry.rs`). It never talks to the session directly: every read and write goes
//! through [`Ctx`] as `qp/0` protocol messages on the endpoint's own connection, so the
//! authority checks the principal's permissions and entitlements exactly as for any
//! other client. Input schemas are generated from Rust types ([`input_schema`],
//! `schemars`), never written by hand.

mod ctx;
pub mod mixed;
pub mod query;
mod registry;
mod surface;
pub mod task;
pub mod thin;

pub use ctx::{CallMeta, Ctx};
pub use registry::{by_name, names, registry, Factory};
pub use surface::{input_schema, Annotations, ToolDef, ToolSurface};
