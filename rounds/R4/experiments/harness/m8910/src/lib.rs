//! R4/H-M8910 — measurement harnesses for M8 (the capsule's closed face), M9 (the feed's
//! layout) and M10 (the opening transition).
//!
//! Binding specification: `rounds/R4/experiments/harness/SPEC.md` §0–§4, §6, §7. Written
//! **before any alternative existed**. Alternatives do not change this crate: they add a
//! design to one registry of the product's grammar (`qp_grammar::faces::FACES`,
//! `qp_grammar::feeds::LAYOUTS`, `qp_grammar::transitions::TRANSITIONS`) and the harness
//! measures every registered name (README).
//!
//! - [`world`]: the synthetic worlds (the demo's host; an hourly world) and in-process
//!   clients (a bare protocol client, the web shell's core);
//! - [`s8`]: the state set `S8` and its answer keys (from the engine, not the designs);
//! - [`m8`], [`m9`], [`m10`]: pages, keys, prompts and computed measures of each
//!   experiment; [`shots`] drives headless Chromium (`shoot.mjs`) for the screenshots;
//! - [`blind`]: codes and the forbidden-word check of prompts (SPEC §0.3);
//! - [`readers`]: the reader directories `run_readers.sh` fills;
//! - [`score`]: deterministic scoring from raw answers and the selection rules (SPEC §2–§4).

pub mod blind;
pub mod color;
pub mod io;
pub mod m10;
pub mod m8;
pub mod m9;
pub mod prompts;
pub mod readers;
pub mod rng;
pub mod s8;
pub mod score;
pub mod shots;
pub mod world;

/// Harness version written into every record.
pub const HARNESS_VERSION: &str = "r4-h-m8910/1";

/// The reference design name of each experiment.
pub const REFERENCE: &str = "r3";

/// The demo principal without data entitlement (`qp_wasm::shell::VISITOR`).
pub const VISITOR: &str = qp_wasm::shell::VISITOR;

/// The reader principal of the demo.
pub const READER: &str = "arkin";
