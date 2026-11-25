//! Qapsule compute core (`Study` = "Hesap" in `docs/CONCEPTS.md`).
//!
//! Every study is a deterministic function from input columns to named output columns:
//!
//! - **f64 only**, no clock, network, randomness or LLM.
//! - **Same length as the input (K13):** one output value per input bar; warmup values
//!   are `None` (`null` in JSON). Nothing is trimmed.
//! - **Three run modes, bit-identical:** batch ([`run`]), streaming ([`Study::push`]) and
//!   "revise the last bar" ([`Study::update_last`], for a live bar whose price keeps
//!   changing). A new bar and a revision are both O(1).
//! - **Cross-target bit-identical:** only IEEE basic operations (`+ - * /`, `sqrt`) in a
//!   fixed order; no `mul_add`, `powf`, `exp`, `ln`. Rust never contracts `a * b + c` into
//!   an FMA on its own, so aarch64, x86_64 and wasm32 produce the same bits (tested by the
//!   cross-target digest, see `examples/digest.rs`).
//!
//! # How "revise the last bar" works
//!
//! Each study is written as a [`Core`]: a *committed* state that covers every bar before
//! the last one, plus two pure-ish operations. `peek(state, x)` computes the outputs of a
//! bar with input `x` on top of the committed state without changing it; `commit(x)`
//! folds `x` into the committed state. [`Streaming`] keeps the last bar's input pending:
//!
//! - `push(x)`: commit the pending input (if any), make `x` pending, return `peek(x)`.
//! - `update_last(x')`: replace the pending input, return `peek(x')`.
//!
//! The output of a bar therefore only depends on the committed state and the bar's
//! *final* input, whichever way it got there; batch, streaming and revised runs are
//! bit-identical by construction (and tested). Both operations are O(1); ring buffers
//! hold windows.

pub mod address;
mod chain;
mod core;
mod error;
mod field;
pub mod graph;
mod lanes;
pub mod late;
pub mod par;
#[doc(hidden)]
pub mod profiles;
mod registry;
pub mod studies;
mod study;
pub mod sweep;

pub use address::{Address, AddressError};
pub use chain::Chain;
pub use core::{Core, Streaming};
pub use error::ComputeError;
pub use field::Field;
pub(crate) use registry::{check_period, period_param};
pub use registry::{compute_bars, create, default_params, MAX_PERIOD, STUDY_NAMES};
pub use studies::{Atr, Bollinger, Ema, Highest, Lowest, Macd, PercentRank, Rsi, Sma, Stdev};
pub use study::{run, Study};
