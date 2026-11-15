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

/// How the bars of one bar series changed (the incremental live path, §18.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delta {
    /// Bars from index `from` on are new (appended at the end).
    Appended {
        /// Index of the first new bar.
        from: usize,
    },
    /// The bar at `index` was revised in place: the last bar (live revision) or an
    /// earlier one (correction: every value from it on may change).
    Revised {
        /// Index of the revised bar.
        index: usize,
    },
    /// Anything else (truncation, several writes in one entry, a binding state change):
    /// values from bar `from` on are recomputed from scratch.
    Rebuild {
        /// First bar index that may differ.
        from: usize,
    },
}

impl Delta {
    /// First bar index whose values may differ.
    pub(crate) fn first_bar(self) -> usize {
        match self {
            Self::Appended { from } | Self::Rebuild { from } => from,
            Self::Revised { index } => index,
        }
    }
}

/// Streaming state of a study series (`qp-compute`): the study after every bar of its
/// input, the last bar revisable.
struct Live {
    study: Box<dyn qp_compute::Study>,
    /// For each output column in `Computed::outputs` order, its slot in `out`.
    perm: Vec<usize>,
    out: Vec<Option<f64>>,
}

/// One cached series.
struct Slot {
    id: String,
    val: Rc<Computed>,
    /// Study series only.
    live: Option<Live>,
    /// The cache's `tick` when a delta last changed this slot.
    touched: u64,
}

/// Values of series by id: bar columns and study outputs, kept current incrementally.
///
/// - A study series' entry holds the study's streaming state; a new bar is `push`ed, a
///   live revision is `update_last` (both O(1)); a correction or a truncation recomputes
///   that study from scratch (the compute API cannot rewind; cost O(bars)).
/// - Once a bar series ticks, every study series on it is computed and then kept
///   current on every tick ([`Cache::apply_delta`]); series on other bar series are not
///   touched.
/// - Entries are dropped only for the series a structure change touches (`put_series`,
///   `delete`) and the studies on them ([`Cache::forget`]).
///
/// Bit-identical to computing from scratch: streaming, revised and batch runs of a study
/// are bit-identical by construction (`qp-compute`).
#[derive(Default)]
pub(crate) struct Cache {
    slots: Vec<Option<Slot>>,
    ids: BTreeMap<String, usize>,
    /// Bar series id -> slots of the study series computed from it (kept current).
    deps: BTreeMap<String, Vec<usize>>,
    /// Bumped when a slot is dropped (slot numbers held elsewhere become invalid).
    gen: u64,
    /// Bumped once per live change ([`Cache::begin_tick`]).
    tick: u64,
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("series", &self.ids.keys().collect::<Vec<_>>())
            .field("gen", &self.gen)
            .field("tick", &self.tick)
            .finish_non_exhaustive()
    }
}

impl Cache {
    pub(crate) fn clear(&mut self) {
        self.slots.clear();
        self.ids.clear();
        self.deps.clear();
        self.gen += 1;
    }

    /// Cached values of a series, if computed.
    pub(crate) fn get(&self, id: &str) -> Option<&Rc<Computed>> {
        self.slot_of(id).and_then(|i| self.val(i))
    }

    /// Slot number of a cached series.
    pub(crate) fn slot_of(&self, id: &str) -> Option<usize> {
        self.ids.get(id).copied()
    }

    /// Values in slot `i`.
    pub(crate) fn val(&self, i: usize) -> Option<&Rc<Computed>> {
        self.slots.get(i)?.as_ref().map(|s| &s.val)
    }

    /// True if slot `i` changed in the current tick.
    pub(crate) fn touched_now(&self, i: usize) -> bool {
        self.slots
            .get(i)
            .and_then(Option::as_ref)
            .is_some_and(|s| s.touched == self.tick)
    }

    /// Generation: slot numbers taken in an earlier generation are invalid.
    pub(crate) fn gen(&self) -> u64 {
        self.gen
    }

    fn insert(&mut self, id: &str, val: Computed, live: Option<Live>) -> Rc<Computed> {
        let rc = Rc::new(val);
        let slot = Slot {
            id: id.to_owned(),
            val: Rc::clone(&rc),
            live,
            touched: self.tick,
        };
        let i = self.slots.len();
        self.slots.push(Some(slot));
        self.ids.insert(id.to_owned(), i);
        rc
    }

    fn drop_slot(&mut self, i: usize) {
        if let Some(s) = self.slots.get_mut(i).and_then(Option::take) {
            self.ids.remove(&s.id);
            self.gen += 1;
        }
    }

    fn drop_id(&mut self, id: &str) {
        if let Some(i) = self.slot_of(id) {
            self.drop_slot(i);
        }
    }

    /// Drops the entries a structure change invalidates: the series `ids` and every
    /// cached study whose input is one of them or that no longer exists in `doc` as it
    /// was computed.
    pub(crate) fn forget(&mut self, doc: &Document, ids: &[&str]) {
        for id in ids {
            self.drop_id(id);
        }
        let stale: Vec<usize> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let s = s.as_ref()?;
                let keep = match find_series(doc, &s.id).map(|x| &x.source) {
                    None => false,
                    Some(SeriesSource::Study { inputs, .. }) => inputs
                        .first()
                        .is_some_and(|inp| !ids.contains(&inp.as_str())),
                    Some(_) => true,
                };
                (!keep).then_some(i)
            })
            .collect();
        for i in stale {
            self.drop_slot(i);
        }
        self.deps.clear();
    }

    /// Starts one live change: slots changed from now on count as touched by it.
    pub(crate) fn begin_tick(&mut self) {
        self.tick += 1;
    }

    /// Brings every value resting on bar series `bar` up to date after its bars changed
    /// by `delta` (`doc`: the reading document after the change). Its own columns (if
    /// cached) and every study series whose input it is: studies with state are advanced
    /// (`push` / `update_last`) or, for a correction or a rebuild, recomputed; studies
    /// not cached yet are computed now, so that every study on a ticking series is kept
    /// current. Series on other bar series are not touched.
    pub(crate) fn apply_delta(&mut self, doc: &Document, bar: &str, delta: Delta) {
        let bars = find_series(doc, bar).and_then(Series::bars);
        // The bar series' own columns.
        if let Some(i) = self.slot_of(bar) {
            let ok = bars.is_some_and(|b| update_columns(self.slot_mut(i), b, delta));
            if ok {
                self.touch(i);
            } else {
                self.drop_slot(i);
            }
        }
        // Study series on it, cached already: advance or drop.
        if let Some(list) = self.deps.get(bar).cloned() {
            for i in list {
                let ok = bars.is_some_and(|b| update_study(self.slot_mut(i), b, delta));
                if ok {
                    self.touch(i);
                } else {
                    self.drop_slot(i);
                    self.deps.remove(bar);
                }
            }
        } else {
            // First tick on `bar` since a structure change: advance the cached studies on
            // it, compute the others below.
            let on_bar: Vec<usize> = self
                .slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let s = s.as_ref()?;
                    s.live.as_ref()?;
                    study_input(doc, &s.id).filter(|inp| *inp == bar).map(|_| i)
                })
                .collect();
            for i in on_bar {
                let ok = bars.is_some_and(|b| update_study(self.slot_mut(i), b, delta));
                if ok {
                    self.touch(i);
                } else {
                    self.drop_slot(i);
                }
            }
        }
        if !self.deps.contains_key(bar) {
            self.ensure_deps(doc, bar);
        }
    }

    /// Computes (if needed) every study series of `doc` whose input is `bar` and records
    /// their slots.
    fn ensure_deps(&mut self, doc: &Document, bar: &str) {
        let mut list = Vec::new();
        for s in &doc.series {
            let SeriesSource::Study { inputs, .. } = &s.source else {
                continue;
            };
            if inputs.first().map(String::as_str) != Some(bar) || s.data.is_some() {
                continue;
            }
            if values(doc, self, &s.id).is_ok() {
                if let Some(i) = self.slot_of(&s.id) {
                    list.push(i);
                }
            }
        }
        self.deps.insert(bar.to_owned(), list);
    }

    fn slot_mut(&mut self, i: usize) -> &mut Slot {
        self.slots[i].as_mut().expect("live slot")
    }

    fn touch(&mut self, i: usize) {
        let t = self.tick;
        self.slot_mut(i).touched = t;
    }
}

/// The input bar series of study series `id`.
fn study_input<'a>(doc: &'a Document, id: &str) -> Option<&'a str> {
    match &find_series(doc, id)?.source {
        SeriesSource::Study { inputs, .. } => inputs.first().map(String::as_str),
        _ => None,
    }
}

/// Bar columns in `Computed::outputs` order (`c h l o v`).
fn bar_cols(b: &Bar) -> [f64; 5] {
    [b.c, b.h, b.l, b.o, b.v]
}

/// Updates a bar series' columns; false if they cannot be updated in place.
fn update_columns(slot: &mut Slot, bars: &[Bar], delta: Delta) -> bool {
    let n = bars.len();
    let c = Rc::make_mut(&mut slot.val);
    match delta {
        Delta::Appended { from } if c.t.len() == from && from <= n => {
            for b in &bars[from..] {
                c.t.push(b.t);
                for (col, v) in c.outputs.values_mut().zip(bar_cols(b)) {
                    col.push(Some(v));
                }
            }
            true
        }
        Delta::Revised { index } if c.t.len() == n && index < n => {
            let b = &bars[index];
            c.t[index] = b.t;
            for (col, v) in c.outputs.values_mut().zip(bar_cols(b)) {
                col[index] = Some(v);
            }
            true
        }
        _ => false,
    }
}

/// Advances a study series; false if it must be recomputed (correction, rebuild, out of
/// step).
fn update_study(slot: &mut Slot, bars: &[Bar], delta: Delta) -> bool {
    let n = bars.len();
    let Some(live) = slot.live.as_mut() else {
        return false;
    };
    let c = Rc::make_mut(&mut slot.val);
    match delta {
        Delta::Appended { from } if c.t.len() == from && from <= n => {
            for b in &bars[from..] {
                live.study.push(&[b.c], &mut live.out);
                c.t.push(b.t);
                for (col, &k) in c.outputs.values_mut().zip(&live.perm) {
                    col.push(live.out[k]);
                }
            }
            true
        }
        Delta::Revised { index } if c.t.len() == n && index + 1 == n => {
            let b = &bars[index];
            live.study.update_last(&[b.c], &mut live.out);
            for (col, &k) in c.outputs.values_mut().zip(&live.perm) {
                col[index] = live.out[k];
            }
            true
        }
        _ => false,
    }
}

/// Output name -> one value per bar.
type Outputs = BTreeMap<String, Vec<Option<f64>>>;

/// Runs a study over bars from scratch: its values and its streaming state (bit-identical
/// to `qp_compute::compute_bars`: same study, same pushes).
fn run_study(
    study: &str,
    params: &BTreeMap<String, f64>,
    bars: &[Bar],
) -> Result<(Outputs, Live), String> {
    let mut st =
        qp_compute::create(study, params, qp_compute::Field::Close).map_err(|e| e.to_string())?;
    let names = st.outputs();
    let mut cols: Vec<Vec<Option<f64>>> = names
        .iter()
        .map(|_| Vec::with_capacity(bars.len()))
        .collect();
    let mut out = vec![None; names.len()];
    for b in bars {
        st.push(&[b.c], &mut out);
        for (col, v) in cols.iter_mut().zip(&out) {
            col.push(*v);
        }
    }
    let outputs: BTreeMap<String, Vec<Option<f64>>> = names
        .iter()
        .zip(cols)
        .map(|(n, c)| ((*n).to_owned(), c))
        .collect();
    let perm = outputs
        .keys()
        .map(|k| names.iter().position(|n| n == k).unwrap_or(0))
        .collect();
    Ok((
        outputs,
        Live {
            study: st,
            perm,
            out,
        },
    ))
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
    if let Some(c) = cache.get(id) {
        return Ok(Rc::clone(c));
    }
    let s = find_series(doc, id).ok_or_else(|| format!("no series `{id}`"))?;
    if let Some(bars) = s.bars() {
        let col = |f: fn(&Bar) -> f64| bars.iter().map(|b| Some(f(b))).collect::<Vec<_>>();
        let mut outputs = BTreeMap::new();
        outputs.insert("o".to_owned(), col(|b| b.o));
        outputs.insert("h".to_owned(), col(|b| b.h));
        outputs.insert("l".to_owned(), col(|b| b.l));
        outputs.insert("c".to_owned(), col(|b| b.c));
        outputs.insert("v".to_owned(), col(|b| b.v));
        let computed = Computed {
            t: bars.iter().map(|b| b.t).collect(),
            outputs,
            main: "c".to_owned(),
        };
        return Ok(cache.insert(id, computed, None));
    }
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
            let (outputs, live) = run_study(study, params, bars)?;
            let computed = Computed {
                t: bars.iter().map(|b| b.t).collect(),
                outputs,
                main: main_output(study, output.as_ref()),
            };
            Ok(cache.insert(id, computed, Some(live)))
        }
        _ => Err(format!("series `{id}` has no data")),
    }
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
    match c {
        Condition::Threshold { series, op, value } => {
            let l = values(doc, cache, series).ok()?;
            eval_on(*op, &l, None, *value)
        }
        Condition::Compare { left, op, right } => {
            let l = values(doc, cache, left).ok()?;
            let r = values(doc, cache, right).ok()?;
            eval_on(*op, &l, Some(&r), 0.0)
        }
        Condition::Text { .. } => None,
    }
}

/// [`eval_condition`] on computed values: `right` for a comparison, else the threshold
/// `value`.
pub(crate) fn eval_on(
    op: CompareOp,
    left: &Computed,
    right: Option<&Computed>,
    value: f64,
) -> Option<(bool, u64, i64)> {
    let n = left.len();
    if n == 0 {
        return None;
    }
    let (lp, ll) = last_two(left.main_values());
    let (rp, rl) = match right {
        Some(r) => last_two(r.main_values()),
        None => (Some(value), Some(value)),
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
