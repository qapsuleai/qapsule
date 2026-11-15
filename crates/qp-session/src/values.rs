//! Series values: bar columns and study outputs (computed with `qp-compute`, cached),
//! and condition evaluation (`docs/PROTOCOL.md` §5.1).

use std::collections::BTreeMap;
use std::rc::Rc;

use qp_schema::{Bar, CompareOp, Condition, Document, Series, SeriesSource};

/// Values of one series, aligned with its bars.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Computed {
    /// Bar times.
    pub t: Vec<i64>,
    /// Output name -> one value per bar.
    pub outputs: BTreeMap<String, Vec<Option<f64>>>,
    /// The output used in conditions.
    pub main: String,
}

impl Computed {
    pub(crate) fn len(&self) -> usize {
        self.t.len()
    }

    pub(crate) fn main_values(&self) -> &[Option<f64>] {
        self.outputs.get(&self.main).map_or(&[], Vec::as_slice)
    }
}

/// Cache of study outputs by series id. Cleared whenever series or bars change.
#[derive(Debug, Default)]
pub(crate) struct Cache {
    map: BTreeMap<String, Rc<Computed>>,
}

impl Cache {
    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    /// Cached values of a series, if computed.
    pub(crate) fn get(&self, id: &str) -> Option<&Rc<Computed>> {
        self.map.get(id)
    }
}

pub(crate) fn find_series<'a>(doc: &'a Document, id: &str) -> Option<&'a Series> {
    doc.series.iter().find(|s| s.id == id)
}

/// The bar series a series is aligned with: itself if it has bars, else the first
/// input of its study (recursively).
pub(crate) fn bar_series<'a>(doc: &'a Document, id: &str) -> Option<&'a Series> {
    let mut cur = find_series(doc, id)?;
    for _ in 0..64 {
        if cur.bars().is_some() {
            return Some(cur);
        }
        match &cur.source {
            SeriesSource::Study { inputs, .. } => cur = find_series(doc, inputs.first()?)?,
            _ => return None,
        }
    }
    None
}

pub(crate) fn bars_of<'a>(doc: &'a Document, id: &str) -> Option<&'a [Bar]> {
    bar_series(doc, id).and_then(Series::bars)
}

/// Main output name of a study: `source.output`, else the study's own name, else
/// `middle` for bollinger.
pub(crate) fn main_output(study: &str, output: Option<&String>) -> String {
    if let Some(o) = output {
        return o.clone();
    }
    match study {
        "bollinger" => "middle".to_owned(),
        s => s.to_owned(),
    }
}

/// Output names of a built-in study.
pub(crate) fn study_outputs(study: &str) -> Vec<String> {
    qp_compute::default_params(study)
        .and_then(|p| qp_compute::create(study, &p, qp_compute::Field::Close).ok())
        .map(|s| s.outputs().iter().map(|o| (*o).to_owned()).collect())
        .unwrap_or_default()
}

/// Values of a series (bar columns `o h l c v`, or study outputs).
pub(crate) fn values(doc: &Document, cache: &mut Cache, id: &str) -> Result<Rc<Computed>, String> {
    if let Some(c) = cache.map.get(id) {
        return Ok(Rc::clone(c));
    }
    let s = find_series(doc, id).ok_or_else(|| format!("no series `{id}`"))?;
    let computed = if let Some(bars) = s.bars() {
        let col = |f: fn(&Bar) -> f64| bars.iter().map(|b| Some(f(b))).collect::<Vec<_>>();
        let mut outputs = BTreeMap::new();
        outputs.insert("o".to_owned(), col(|b| b.o));
        outputs.insert("h".to_owned(), col(|b| b.h));
        outputs.insert("l".to_owned(), col(|b| b.l));
        outputs.insert("c".to_owned(), col(|b| b.c));
        outputs.insert("v".to_owned(), col(|b| b.v));
        Computed {
            t: bars.iter().map(|b| b.t).collect(),
            outputs,
            main: "c".to_owned(),
        }
    } else {
        match &s.source {
            SeriesSource::Study {
                study,
                inputs,
                params,
                output,
            } => {
                let input = inputs
                    .first()
                    .ok_or_else(|| format!("study series `{id}` has no input"))?;
                let bars = find_series(doc, input)
                    .and_then(Series::bars)
                    .ok_or_else(|| format!("input `{input}` of `{id}` is not a bar series"))?;
                let out = qp_compute::compute_bars(study, params, qp_compute::Field::Close, bars)
                    .map_err(|e| e.to_string())?;
                Computed {
                    t: out.t,
                    outputs: out.outputs,
                    main: main_output(study, output.as_ref()),
                }
            }
            _ => return Err(format!("series `{id}` has no data")),
        }
    };
    let rc = Rc::new(computed);
    cache.map.insert(id.to_owned(), Rc::clone(&rc));
    Ok(rc)
}

/// Series ids a condition names.
pub(crate) fn condition_series(c: &Condition) -> Vec<(&'static str, &str)> {
    match c {
        Condition::Threshold { series, .. } => vec![("series", series.as_str())],
        Condition::Compare { left, right, .. } => {
            vec![("left", left.as_str()), ("right", right.as_str())]
        }
        Condition::Text { .. } => Vec::new(),
    }
}

fn cmp(op: CompareOp, prev: Option<(f64, f64)>, last: (f64, f64)) -> bool {
    let (a, b) = last;
    match op {
        CompareOp::Lt => a < b,
        CompareOp::Le => a <= b,
        CompareOp::Gt => a > b,
        CompareOp::Ge => a >= b,
        CompareOp::CrossAbove => prev.is_some_and(|(pa, pb)| pa < pb) && a >= b,
        CompareOp::CrossBelow => prev.is_some_and(|(pa, pb)| pa > pb) && a <= b,
    }
}

fn last_two(v: &[Option<f64>]) -> (Option<f64>, Option<f64>) {
    let n = v.len();
    let last = if n >= 1 { v[n - 1] } else { None };
    let prev = if n >= 2 { v[n - 2] } else { None };
    (prev, last)
}

/// Evaluates a condition at the last bar. Returns `(true_now, last bar index, its time)`;
/// `None` when it cannot be evaluated (text, missing series, no values).
pub(crate) fn eval_condition(
    doc: &Document,
    cache: &mut Cache,
    c: &Condition,
) -> Option<(bool, u64, i64)> {
    let (op, left, right): (CompareOp, Rc<Computed>, Option<Rc<Computed>>) = match c {
        Condition::Threshold { series, op, .. } => (*op, values(doc, cache, series).ok()?, None),
        Condition::Compare { left, op, right } => (
            *op,
            values(doc, cache, left).ok()?,
            Some(values(doc, cache, right).ok()?),
        ),
        Condition::Text { .. } => return None,
    };
    let n = left.len();
    if n == 0 {
        return None;
    }
    let (lp, ll) = last_two(left.main_values());
    let (rp, rl) = match (&right, c) {
        (Some(r), _) => last_two(r.main_values()),
        (None, Condition::Threshold { value, .. }) => (Some(*value), Some(*value)),
        _ => return None,
    };
    let bar = (n - 1) as u64;
    let t = left.t[n - 1];
    let (Some(a), Some(b)) = (ll, rl) else {
        return Some((false, bar, t));
    };
    let prev = match (lp, rp) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    Some((cmp(op, prev, (a, b)), bar, t))
}

/// Whether a condition holds now (false if it cannot be evaluated).
pub(crate) fn holds(doc: &Document, cache: &mut Cache, c: &Condition) -> bool {
    eval_condition(doc, cache, c).is_some_and(|(b, _, _)| b)
}
