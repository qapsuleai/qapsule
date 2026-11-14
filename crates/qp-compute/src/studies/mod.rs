//! The five built-in studies. Each is a [`Core`](crate::Core) with single-input inherent
//! `peek1`/`commit1` functions that the generic wrapper and the sweep kernels share, so
//! every run mode executes the same arithmetic in the same order.
//!
//! Formulas (warmup = leading `None` count, `p` = period):
//!
//! | Study | Outputs | Warmup | Definition |
//! |-------|---------|--------|------------|
//! | `sma(p)` | `sma` | `p-1` | mean of the last `p` inputs |
//! | `ema(p)` | `ema` | `p-1` | seeded with `sma(p)` at bar `p-1`, then `x*a + prev*(1-a)`, `a = 2/(p+1)` |
//! | `rsi(p)` | `rsi` | `p` | Wilder: seed = mean gain/loss of the first `p` changes, then `(avg*(p-1)+g)/p`; `100 - 100/(1+rs)`; 100 if no loss, 50 if no move |
//! | `macd(f,s,g)` | `macd`, `signal`, `histogram` | `s-1`, `s+g-2`, `s+g-2` | `ema(f) - ema(s)`; signal = `ema(g)` of macd; histogram = macd - signal |
//! | `bollinger(p,k)` | `middle`, `upper`, `lower`, `bandwidth` | `p-1` each | middle = `sma(p)`; population std; `middle ± k*std`; bandwidth = `2*k*std / middle` (0 if middle is 0) |

mod bollinger;
mod ema;
mod macd;
mod rsi;
mod sma;

pub use bollinger::Bollinger;
pub use ema::Ema;
pub use macd::Macd;
pub use rsi::Rsi;
pub(crate) use rsi::{rsi_from_avg, split as rsi_split};
pub use sma::Sma;

/// `usize -> f64` for periods (validated `<= 1e9`, so exact).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn pf(p: usize) -> f64 {
    p as f64
}
