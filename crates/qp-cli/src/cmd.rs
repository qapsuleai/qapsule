//! The `qp` commands' bodies, one file per topic (RG1: split out of `main.rs`); the
//! argument types and the dispatch stay in `main.rs`.

mod compute;
mod program;
mod replay;
mod scene;
mod schema;
mod serve;

pub(crate) use compute::*;
pub(crate) use program::{program_cmd, ProgramCommand};
pub(crate) use replay::*;
pub(crate) use scene::*;
pub(crate) use schema::*;
pub(crate) use serve::*;
