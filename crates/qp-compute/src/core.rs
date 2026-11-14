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

    fn reset(&mut self) {
        self.core.reset();
        self.has_pending = false;
    }
}

/// Fixed-capacity FIFO of the last `cap` committed values.
#[derive(Debug, Clone)]
pub(crate) struct Ring {
    buf: Vec<f64>,
    head: usize,
    len: usize,
}

impl Ring {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            buf: vec![0.0; cap],
            head: 0,
            len: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Appends `x`; returns the evicted value when the ring was full. With capacity 0
    /// `x` itself is "evicted".
    pub(crate) fn push(&mut self, x: f64) -> Option<f64> {
        let cap = self.buf.len();
        if cap == 0 {
            return Some(x);
        }
        if self.len < cap {
            let idx = (self.head + self.len) % cap;
            self.buf[idx] = x;
            self.len += 1;
            None
        } else {
            let old = self.buf[self.head];
            self.buf[self.head] = x;
            self.head = (self.head + 1) % cap;
            Some(old)
        }
    }

    pub(crate) fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}
