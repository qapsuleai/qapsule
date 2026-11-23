//! (RF1/G2c) The grammar's shared test fixtures (`qp-grammar/tests/support`): the design
//! tests that moved here from `qp-grammar` use them unchanged (one copy, not two).

#[path = "../../../qp-grammar/tests/support/mod.rs"]
mod shared;

#[allow(unused_imports)]
// not every test binary uses every fixture (as `#![allow(dead_code)]` there)
pub use shared::*;
