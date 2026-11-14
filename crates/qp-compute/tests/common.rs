#![allow(dead_code)]

use std::collections::BTreeMap;

use qp_compute::{create, run, Field, Study};

pub fn params(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

pub fn study(name: &str, kv: &[(&str, f64)]) -> Box<dyn Study> {
    create(name, &params(kv), Field::Close).unwrap()
}

/// Batch outputs of a built-in study over one column.
pub fn batch(name: &str, kv: &[(&str, f64)], x: &[f64]) -> Vec<Vec<Option<f64>>> {
    let mut s = study(name, kv);
    run(s.as_mut(), &[x]).unwrap()
}

/// |a - b| <= tol * max(1, |b|).
pub fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}
