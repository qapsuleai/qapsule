//! The streaming state of a computed series and its incremental update (§18.8): bar
//! columns, `qp-compute` studies and (RL1) programs.

use std::collections::BTreeMap;
use std::rc::Rc;

use qp_schema::Bar;

use super::program::{self, ProgLive};
use super::{Delta, Slot};

/// Streaming state of a computed series.
pub(super) enum Live {
    /// A `qp-compute` study.
    Study(StudyLive),
    /// A program output (`qp-lang`).
    Program(Box<ProgLive>),
}

/// Streaming state of a study series (`qp-compute`): the study after every bar of its
/// input, the last bar revisable.
pub(super) struct StudyLive {
    study: Box<dyn qp_compute::Study>,
    /// For each output column in `Computed::outputs` order, its slot in `out`.
    perm: Vec<usize>,
    out: Vec<Option<f64>>,
}

/// Bar columns in `Computed::outputs` order (`c h l o v`).
fn bar_cols(b: &Bar) -> [f64; 5] {
    [b.c, b.h, b.l, b.o, b.v]
}

/// Updates a bar series' columns; false if they cannot be updated in place.
pub(super) fn update_columns(slot: &mut Slot, bars: &[Bar], delta: Delta) -> bool {
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

/// Advances a computed series; false if it must be recomputed (correction, rebuild, out
/// of step).
pub(super) fn update_study(slot: &mut Slot, bars: &[Bar], delta: Delta) -> bool {
    let n = bars.len();
    let c = Rc::make_mut(&mut slot.val);
    let live = match slot.live.as_mut() {
        Some(Live::Study(live)) => live,
        Some(Live::Program(p)) => return program::update(p, c, bars, delta),
        None => return false,
    };
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

/// Capacity for a column of `n` bars: room for live growth, so that new bars do not
/// reallocate every column of every study at once (a multi-millisecond tick at 500
/// studies) until the series has grown by a quarter.
pub(super) fn room(n: usize) -> usize {
    n + n / 4 + 64
}

pub(super) fn times(bars: &[Bar]) -> Vec<i64> {
    let mut t = Vec::with_capacity(room(bars.len()));
    t.extend(bars.iter().map(|b| b.t));
    t
}

/// Output name -> one value per bar.
pub(super) type Outputs = BTreeMap<String, Vec<Option<f64>>>;

/// Runs a study over bars from scratch: its values and its streaming state (bit-identical
/// to `qp_compute::compute_bars`: same study, same pushes).
pub(super) fn run_study(
    study: &str,
    params: &BTreeMap<String, f64>,
    bars: &[Bar],
) -> Result<(Outputs, Live), String> {
    let mut st =
        qp_compute::create(study, params, qp_compute::Field::Close).map_err(|e| e.to_string())?;
    let names = st.outputs();
    let mut cols: Vec<Vec<Option<f64>>> = names
        .iter()
        .map(|_| Vec::with_capacity(room(bars.len())))
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
        Live::Study(StudyLive {
            study: st,
            perm,
            out,
        }),
    ))
}
