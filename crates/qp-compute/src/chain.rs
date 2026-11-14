//! Feeding one study's output into another ("a study may read another study's output").

use std::collections::BTreeMap;

use crate::Study;

/// `second(first.output[which])`: `second` (single input) reads output `which` of
/// `first`. While `first` is warming up, `second` receives nothing and outputs `None`; its
/// own warmup starts at `first`'s first value. All three run modes stay bit-identical.
///
/// Assumes `first`'s warmup is index-based (true for every built-in study): revising the
/// last bar never turns a defined value into `None`.
pub struct Chain {
    first: Box<dyn Study>,
    second: Box<dyn Study>,
    which: usize,
    first_out: Vec<Option<f64>>,
    second_has_last: bool,
}

impl Chain {
    /// # Panics
    /// If `which` is not an output of `first` or `second` does not take exactly one input.
    #[must_use]
    pub fn new(first: Box<dyn Study>, which: usize, second: Box<dyn Study>) -> Self {
        assert!(which < first.outputs().len(), "no such output");
        assert_eq!(second.inputs().len(), 1, "second study must take one input");
        let n = first.outputs().len();
        Self {
            first,
            second,
            which,
            first_out: vec![None; n],
            second_has_last: false,
        }
    }

    fn none(out: &mut [Option<f64>]) {
        out.iter_mut().for_each(|o| *o = None);
    }
}

impl Study for Chain {
    fn name(&self) -> &'static str {
        self.second.name()
    }
    fn params(&self) -> BTreeMap<String, f64> {
        let mut p = BTreeMap::new();
        for (k, v) in self.first.params() {
            p.insert(format!("{}.{k}", self.first.name()), v);
        }
        for (k, v) in self.second.params() {
            p.insert(format!("{}.{k}", self.second.name()), v);
        }
        p
    }
    fn inputs(&self) -> Vec<String> {
        self.first.inputs()
    }
    fn outputs(&self) -> &'static [&'static str] {
        self.second.outputs()
    }
    fn warmup(&self) -> Vec<usize> {
        let base = self.first.warmup()[self.which];
        self.second.warmup().into_iter().map(|w| base + w).collect()
    }
    fn push(&mut self, input: &[f64], out: &mut [Option<f64>]) {
        self.first.push(input, &mut self.first_out);
        self.second_has_last = false;
        match self.first_out[self.which] {
            Some(v) => {
                self.second.push(&[v], out);
                self.second_has_last = true;
            }
            None => Self::none(out),
        }
    }
    fn update_last(&mut self, input: &[f64], out: &mut [Option<f64>]) {
        self.first.update_last(input, &mut self.first_out);
        match self.first_out[self.which] {
            Some(v) if self.second_has_last => self.second.update_last(&[v], out),
            Some(v) => {
                self.second.push(&[v], out);
                self.second_has_last = true;
            }
            None => Self::none(out),
        }
    }
    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
        self.second_has_last = false;
    }
}
