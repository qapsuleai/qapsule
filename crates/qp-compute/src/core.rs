//! The `Core` building block and the generic [`Streaming`] study wrapper.

use std::collections::BTreeMap;

use crate::{Field, Study};

/// A study written as committed state + `peek`/`commit` (see the crate docs).
///
/// Contract: `peek` must not change observable state; `commit(x)` must leave the state
/// exactly as if a bar with input `x` had been added. Both are O(1).
pub trait Core: Clone {
    /// Study name.
    const NAME: &'static str;
    /// Output names.
    const OUTPUTS: &'static [&'static str];
    /// Number of inputs per bar.
    const N_IN: usize;

    /// Parameters.
    fn params(&self) -> BTreeMap<String, f64>;
    /// Warmup length per output.
    fn warmup(&self) -> Vec<usize>;
    /// Outputs of a bar with input `x` on top of the committed state.
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]);
    /// Folds a bar with input `x` into the committed state.
    fn commit(&mut self, x: &[f64]);
    /// Back to the empty state.
    fn reset(&mut self);
}

/// Wraps a [`Core`] into a [`Study`] with a pending last bar.
#[derive(Debug, Clone)]
pub struct Streaming<C: Core> {
    core: C,
    inputs: Vec<String>,
    pending: Vec<f64>,
    has_pending: bool,
}

impl<C: Core> Streaming<C> {
    /// A study reading one bar field (for single-input cores).
    #[must_use]
    pub fn new(core: C, field: Field) -> Self {
        Self::with_inputs(core, vec![field.name().to_owned()])
    }

    /// A study with explicitly named inputs (bar fields or other studies' outputs).
    ///
    /// # Panics
    /// If `inputs.len() != C::N_IN`.
    #[must_use]
    pub fn with_inputs(core: C, inputs: Vec<String>) -> Self {
        assert_eq!(
            inputs.len(),
            C::N_IN,
            "{} takes {} input(s)",
            C::NAME,
            C::N_IN
        );
        Self {
            core,
            inputs,
            pending: vec![0.0; C::N_IN],
            has_pending: false,
        }
    }

    /// The wrapped core (committed state only).
    #[must_use]
    pub fn core(&self) -> &C {
        &self.core
    }
}

impl<C: Core> Study for Streaming<C> {
    fn name(&self) -> &'static str {
        C::NAME
    }

    fn params(&self) -> BTreeMap<String, f64> {
        self.core.params()
    }

    fn inputs(&self) -> Vec<String> {
        self.inputs.clone()
    }

    fn outputs(&self) -> &'static [&'static str] {
        C::OUTPUTS
    }

    fn warmup(&self) -> Vec<usize> {
        self.core.warmup()
    }

    fn push(&mut self, input: &[f64], out: &mut [Option<f64>]) {
        if self.has_pending {
            self.core.commit(&self.pending);
        }
        self.pending.copy_from_slice(input);
        self.has_pending = true;
        self.core.peek(input, out);
    }

    fn update_last(&mut self, input: &[f64], out: &mut [Option<f64>]) {
        self.pending.copy_from_slice(input);
        self.has_pending = true;
        self.core.peek(input, out);
    }

    fn discard_last(&mut self) {
        // The pending bar never reached the committed state; forgetting it is enough.
        self.has_pending = false;
    }

    fn reset(&mut self) {
        self.core.reset();
        self.has_pending = false;
    }

    /// Commit-only fast path (RP1/E5): the same `commit`s as `push`ing each bar, in the
    /// same order, without computing the history's outputs (`peek`).
    fn extend(&mut self, columns: &[&[f64]]) {
        assert_eq!(columns.len(), C::N_IN, "one column per input");
        let len = columns.first().map_or(0, |c| c.len());
        assert!(
            columns.iter().all(|c| c.len() == len),
            "columns differ in length"
        );
        if len == 0 {
            return;
        }
        if self.has_pending {
            self.core.commit(&self.pending);
        }
        if C::N_IN == 1 {
            for &x in &columns[0][..len - 1] {
                self.core.commit(&[x]);
            }
        } else {
            let mut x = vec![0.0; C::N_IN];
            for i in 0..len - 1 {
                for (slot, col) in x.iter_mut().zip(columns) {
                    *slot = col[i];
                }
                self.core.commit(&x);
            }
        }
        for (slot, col) in self.pending.iter_mut().zip(columns) {
            *slot = col[len - 1];
        }
        self.has_pending = true;
    }
}

/// Fixed-capacity FIFO of the last `cap` committed values (no integer division on the
/// hot path: indices wrap with a compare).
#[derive(Debug, Clone)]
pub(crate) struct Ring {
    buf: Vec<f64>,
    /// Next write position (== oldest value once full).
    pos: usize,
    len: usize,
}

impl Ring {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            buf: vec![0.0; cap],
            pos: 0,
            len: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Appends `x`; returns the evicted value when the ring was full. With capacity 0
    /// `x` itself is "evicted".
    #[inline]
    pub(crate) fn push(&mut self, x: f64) -> Option<f64> {
        let cap = self.buf.len();
        if self.len == cap {
            // Steady state (also the capacity-0 case: `x` itself is evicted).
            if cap == 0 {
                return Some(x);
            }
            let old = std::mem::replace(&mut self.buf[self.pos], x);
            self.pos += 1;
            if self.pos == cap {
                self.pos = 0;
            }
            Some(old)
        } else {
            self.buf[self.pos] = x;
            self.pos += 1;
            if self.pos == cap {
                self.pos = 0;
            }
            self.len += 1;
            None
        }
    }

    /// Whether the ring holds `cap > 0` values.
    #[inline]
    pub(crate) fn is_full_nonempty(&self) -> bool {
        self.len == self.buf.len() && self.len > 0
    }

    /// [`Ring::push`] for a full, non-empty ring: returns the evicted value.
    #[inline]
    pub(crate) fn push_full(&mut self, x: f64) -> f64 {
        let old = std::mem::replace(&mut self.buf[self.pos], x);
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        old
    }

    pub(crate) fn clear(&mut self) {
        self.pos = 0;
        self.len = 0;
    }
}
