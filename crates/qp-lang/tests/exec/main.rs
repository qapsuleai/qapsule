//! Executor tests (RL1 I4, I5): determinism, batch = streaming, the live bar, prefix
//! stability, `add_study` equivalence, higher-timeframe alignment, signals, budget, try.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

mod add_study;
mod align;
mod budget;
mod digest;
mod hostile;
mod modes;
mod prefix;
mod rx4_g8;
mod signals;
mod summary;
