//! The single-input kernels of `qp-compute` behind one interface (PR §4.2): `peek` (a bar
//! on top of the committed state, no change) and `commit` (fold the bar in). The `Chain`
//! rule is the caller's: an undefined input gives undefined outputs and no commit. Each
//! arm calls the kernel's own `peek1`/`commit1` — the same arithmetic `add_study` runs
//! through `qp_compute::Streaming` — so values are bit-equal to it (PR §14).

use qp_compute::{
    Bollinger, ComputeError, Ema, Highest, Lowest, Macd, PercentRank, Rsi, Sma, Stdev,
};

use crate::checked::WindowKind;

/// Outputs of one kernel step, in the checker's output order (unused slots `None`).
pub(super) type Outs = [Option<f64>; 4];

/// A kernel's committed state.
#[derive(Debug, Clone)]
pub(super) enum Kernel {
    Sma(Sma),
    Ema(Ema),
    Rsi(Rsi),
    Highest(Highest),
    Lowest(Lowest),
    Stdev(Stdev),
    PercentRank(PercentRank),
    /// Outputs `macd`, `signal`, `histogram`.
    Macd(Macd),
    /// Outputs `upper`, `middle`, `lower`, `bandwidth` (PR §4.2 order).
    Bollinger(Bollinger),
}

impl Kernel {
    /// A window kernel.
    pub(super) fn window(kind: WindowKind, period: usize) -> Result<Kernel, ComputeError> {
        Ok(match kind {
            WindowKind::Sma => Kernel::Sma(Sma::new(period)?),
            WindowKind::Ema => Kernel::Ema(Ema::new(period)?),
            WindowKind::Rsi => Kernel::Rsi(Rsi::new(period)?),
            WindowKind::Highest => Kernel::Highest(Highest::new(period)?),
            WindowKind::Lowest => Kernel::Lowest(Lowest::new(period)?),
            WindowKind::Stdev => Kernel::Stdev(Stdev::new(period)?),
            WindowKind::Percentrank => Kernel::PercentRank(PercentRank::new(period)?),
        })
    }

    /// How many earlier defined inputs a value rests on (PR §9 `lo` of a window kernel),
    /// or `None` for a recursive kernel (its `lo` is the first bar).
    pub(super) fn lookback(kind: WindowKind, period: usize) -> Option<usize> {
        match kind {
            WindowKind::Sma | WindowKind::Highest | WindowKind::Lowest | WindowKind::Stdev => {
                Some(period - 1)
            }
            WindowKind::Percentrank => Some(period),
            WindowKind::Ema | WindowKind::Rsi => None,
        }
    }

    /// Outputs of a bar with defined input `x` on top of the committed state.
    #[inline]
    pub(super) fn peek(&self, x: f64) -> Outs {
        match self {
            Kernel::Sma(k) => [k.peek1(x), None, None, None],
            Kernel::Ema(k) => [k.peek1(x), None, None, None],
            Kernel::Rsi(k) => [k.peek1(x), None, None, None],
            Kernel::Highest(k) => [k.peek1(x), None, None, None],
            Kernel::Lowest(k) => [k.peek1(x), None, None, None],
            Kernel::Stdev(k) => [k.peek1(x), None, None, None],
            Kernel::PercentRank(k) => [k.peek1(x), None, None, None],
            Kernel::Macd(k) => {
                let v = k.peek1(x);
                [v.macd, v.signal, v.histogram, None]
            }
            Kernel::Bollinger(k) => match k.peek1(x) {
                Some(v) => [
                    Some(v.upper),
                    Some(v.middle),
                    Some(v.lower),
                    Some(v.bandwidth),
                ],
                None => [None; 4],
            },
        }
    }

    /// Folds a bar with defined input `x` into the committed state.
    #[inline]
    pub(super) fn commit(&mut self, x: f64) {
        match self {
            Kernel::Sma(k) => k.commit1(x),
            Kernel::Ema(k) => k.commit1(x),
            Kernel::Rsi(k) => k.commit1(x),
            Kernel::Highest(k) => k.commit1(x),
            Kernel::Lowest(k) => k.commit1(x),
            Kernel::Stdev(k) => k.commit1(x),
            Kernel::PercentRank(k) => k.commit1(x),
            Kernel::Macd(k) => k.commit1(x),
            Kernel::Bollinger(k) => k.commit1(x),
        }
    }
}
