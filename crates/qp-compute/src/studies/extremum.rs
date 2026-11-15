//! Rolling highest / lowest value over a window (R3/T3).

use std::collections::{BTreeMap, VecDeque};

use super::pf;
use crate::{ComputeError, Core};

/// Monotonic deque over the last `period - 1` committed inputs. `MAX = true` keeps a
/// strictly decreasing deque (front = window maximum), `false` a strictly increasing one.
#[derive(Debug, Clone)]
struct Mono<const MAX: bool> {
    period: usize,
    /// `(bar index, value)`; indices strictly increasing, values strictly monotonic.
    q: VecDeque<(usize, f64)>,
    /// Committed bars.
    n: usize,
}

impl<const MAX: bool> Mono<MAX> {
    fn new(name: &str, period: usize) -> Result<Self, ComputeError> {
        crate::check_period(name, "period", period)?;
        Ok(Self {
            period,
            q: VecDeque::with_capacity(period.min(1024)),
            n: 0,
        })
    }

    /// `true` when `a` is at least as extreme as `b` (`a >= b` for a maximum).
    #[inline]
    fn dominates(a: f64, b: f64) -> bool {
        if MAX {
            a >= b
        } else {
            a <= b
        }
    }

    #[inline]
    fn peek1(&self, x: f64) -> Option<f64> {
        if self.n + 1 < self.period {
            return None;
        }
        Some(match self.q.front() {
            Some(&(_, f)) if !Self::dominates(x, f) => f,
            _ => x,
        })
    }

    #[inline]
    fn commit1(&mut self, x: f64) {
        while let Some(&(_, b)) = self.q.back() {
            if Self::dominates(x, b) {
                self.q.pop_back();
            } else {
                break;
            }
        }
        self.q.push_back((self.n, x));
        self.n += 1;
        // Keep only the last `period - 1` committed bars: indices >= n - (period - 1).
        let keep = self.period - 1;
        while let Some(&(i, _)) = self.q.front() {
            if i + keep < self.n {
                self.q.pop_front();
            } else {
                break;
            }
        }
    }

    fn reset(&mut self) {
        self.q.clear();
        self.n = 0;
    }
}

macro_rules! extremum {
    ($ty:ident, $max:expr, $name:literal, $doc:literal) => {
        #[doc = $doc]
        ///
        /// Committed state: a monotonic deque over the last `period - 1` committed inputs
        /// (O(1) amortized per commit, at most `period - 1` entries); a bar's value is the
        /// extreme of the deque's front and the bar's own input (O(1) peek), so a revised
        /// last bar costs O(1). The value is one of the inputs (no arithmetic), so it is
        /// exact. Warmup `period - 1`.
        #[derive(Debug, Clone)]
        pub struct $ty(Mono<$max>);

        impl $ty {
            /// # Errors
            /// `period` of 0 or above [`crate::MAX_PERIOD`].
            pub fn new(period: usize) -> Result<Self, ComputeError> {
                Ok(Self(Mono::new($name, period)?))
            }

            /// Period.
            #[must_use]
            pub fn period(&self) -> usize {
                self.0.period
            }

            /// Value of a bar with input `x` on top of the committed state.
            #[inline]
            #[must_use]
            pub fn peek1(&self, x: f64) -> Option<f64> {
                self.0.peek1(x)
            }

            /// Commits a bar with input `x`.
            #[inline]
            pub fn commit1(&mut self, x: f64) {
                self.0.commit1(x);
            }
        }

        impl Core for $ty {
            const NAME: &'static str = $name;
            const OUTPUTS: &'static [&'static str] = &[$name];
            const N_IN: usize = 1;

            fn params(&self) -> BTreeMap<String, f64> {
                BTreeMap::from([("period".to_owned(), pf(self.0.period))])
            }
            fn warmup(&self) -> Vec<usize> {
                vec![self.0.period - 1]
            }
            fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
                out[0] = self.0.peek1(x[0]);
            }
            fn commit(&mut self, x: &[f64]) {
                self.0.commit1(x[0]);
            }
            fn reset(&mut self) {
                self.0.reset();
            }
        }
    };
}

extremum!(
    Highest,
    true,
    "highest",
    "`highest(period)`: the largest of the last `period` inputs."
);
extremum!(
    Lowest,
    false,
    "lowest",
    "`lowest(period)`: the smallest of the last `period` inputs."
);
