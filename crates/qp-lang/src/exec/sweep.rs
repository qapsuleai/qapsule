//! Parameter sweep of a program (K17; RL1 GOAL §2: sweeps run from the program's parameter
//! definition): one program, many parameter sets, the same closed bars. Every variation's
//! series values are **bit-equal** to running [`Executor::run_closed`] with that set.
//!
//! The program is checked once per set; the nodes whose operation is the same in every
//! variation (they do not depend on the swept values) run **once**. Kernel nodes that do
//! (`sma`, `ema`, `rsi`, `macd`, `bollinger` over an unswept input) run through
//! `qp_compute::sweep` — the lane kernels and shared sub-computations of the K17 study
//! sweep, the same arithmetic as the executor's `peek1`/`commit1` (PR §14) — and the other
//! window kernels step directly over the shared input. Anything else that depends on the
//! swept values (a step over a swept kernel, a higher timeframe, a work budget) falls back
//! to one executor per variation; [`SweepPath`] says which way a sweep ran.
//!
//! A sweep publishes series outputs only (event, signal and state outputs are the
//! executor's); values, not lineage.

use qp_schema::protocol::ProgramError;
use qp_schema::Bar;
use serde_json::Value;

use super::sweep_lane::{self, Dep};
use super::{ExecError, ExecOptions, Executor};
use crate::check::{check, cone_of, CheckCtx};
use crate::checked::{Checked, Clock, Op, OutputSpec, Slot};

/// Why a sweep cannot run.
#[derive(Debug, Clone, PartialEq)]
pub enum SweepError {
    /// Parameter set `index` does not check (nothing ran).
    Check {
        /// Position in the sets.
        index: usize,
        /// The checker's errors.
        errors: Vec<ProgramError>,
    },
    /// An output asked for is not a series output.
    NotSeries(String),
    /// The executor refused (unknown output, unbound timeframe, bars not ascending ...).
    Exec(ExecError),
}

impl std::fmt::Display for SweepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SweepError::Check { index, errors } => {
                write!(f, "parameter set {index}: {} error(s)", errors.len())?;
                if let Some(e) = errors.first() {
                    write!(f, ", first {}", e.reason)?;
                }
                Ok(())
            }
            SweepError::NotSeries(o) => write!(f, "output `{o}` is not a series output"),
            SweepError::Exec(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SweepError {}

impl From<ExecError> for SweepError {
    fn from(e: ExecError) -> Self {
        SweepError::Exec(e)
    }
}

/// How a sweep ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepPath {
    /// Unswept nodes once; `swept` kernel nodes through `qp_compute::sweep`, `direct`
    /// kernel nodes stepped over the shared input.
    Shared {
        /// Kernel nodes run by the compute core's sweep.
        swept: usize,
        /// Kernel nodes stepped directly.
        direct: usize,
    },
    /// One executor per variation, and why.
    PerVariation(&'static str),
}

/// Where one series output's values of a variation are read from.
#[derive(Debug, Clone, Copy)]
pub(super) enum Col<'a> {
    /// A full-length column.
    Opt(&'a [Option<f64>]),
    /// A sweep column over the defined inputs: `NaN` warmup slots, then values; `map`
    /// takes a bar to its defined-input position (`usize::MAX`: undefined input), `None`
    /// when every input is defined.
    Lane {
        v: &'a [f64],
        warmup: usize,
        map: Option<&'a [usize]>,
    },
    /// An executor's output.
    Exec(&'a Executor, usize),
}

impl Col<'_> {
    #[inline]
    fn get(&self, i: usize) -> Option<f64> {
        match *self {
            Col::Opt(c) => c.get(i).copied().flatten(),
            Col::Lane { v, warmup, map } => {
                let k = match map {
                    None => i,
                    Some(m) => *m.get(i)?,
                };
                if k < warmup {
                    None
                } else {
                    v.get(k).copied()
                }
            }
            Col::Exec(x, o) => x.value_at(o, i),
        }
    }
}

/// One variation's series outputs.
#[derive(Debug)]
pub struct Variation<'a> {
    pub(super) index: usize,
    pub(super) checked: &'a Checked,
    pub(super) names: &'a [String],
    pub(super) len: usize,
    pub(super) cols: Vec<Col<'a>>,
}

impl Variation<'_> {
    /// Position of its parameter set.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The checked instance (bound values, instance id).
    #[must_use]
    pub fn checked(&self) -> &Checked {
        self.checked
    }

    /// Series output names, in IR order (or as asked for in `ExecOptions::outputs`).
    #[must_use]
    pub fn names(&self) -> &[String] {
        self.names
    }

    /// Bar count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// No bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Series output `j` at bar `i` — bit-equal to the executor's `value_at`.
    #[inline]
    #[must_use]
    pub fn value(&self, j: usize, i: usize) -> Option<f64> {
        self.cols.get(j)?.get(i)
    }

    /// Series output `j` as a column.
    #[must_use]
    pub fn column(&self, j: usize) -> Vec<Option<f64>> {
        (0..self.len).map(|i| self.value(j, i)).collect()
    }
}

/// Sweeps `ir` over parameter sets: each set (a JSON object) is laid over `ctx.params`
/// and checked; then every variation runs over the closed bars `bars` and is handed to
/// `visit` (order unspecified; [`Variation::index`] says which set). `opts.outputs`
/// selects series outputs (all series outputs when `None`).
///
/// # Errors
/// A set that does not check or a selected output that is not a series (before any
/// variation is visited), or what the executor refuses (bars not ascending, an unbound
/// higher timeframe, a kernel the compute core refuses).
pub fn sweep(
    ir: &Value,
    ctx: &CheckCtx<'_>,
    sets: &[Value],
    bars: &[Bar],
    opts: &ExecOptions,
    mut visit: impl FnMut(&Variation<'_>),
) -> Result<SweepPath, SweepError> {
    let cs = check_all(ir, ctx, sets)?;
    let Some(c0) = cs.first() else {
        return Ok(SweepPath::PerVariation("no parameter sets"));
    };
    let sel = selected(c0, opts.outputs.as_deref())?;
    let names: Vec<String> = sel.iter().map(|(n, _)| n.clone()).collect();
    let plan = if opts.budget.is_some() {
        Err("a work budget")
    } else {
        analyze(&cs, &sel)
    };
    match plan {
        Ok(deps) => {
            let run = sweep_lane::Run {
                cs: &cs,
                sel: &sel,
                names: &names,
                deps: &deps,
                bars,
                opts,
            };
            run.go(&mut visit)
        }
        Err(why) => {
            per_variation(&cs, &names, bars, opts, &mut visit)?;
            Ok(SweepPath::PerVariation(why))
        }
    }
}

/// [`sweep`] over one parameter: `key` takes each of `values` (the other parameters as
/// `ctx.params` binds them, or their defaults).
///
/// # Errors
/// As [`sweep`].
pub fn sweep_param(
    ir: &Value,
    ctx: &CheckCtx<'_>,
    key: &str,
    values: &[Value],
    bars: &[Bar],
    opts: &ExecOptions,
    visit: impl FnMut(&Variation<'_>),
) -> Result<SweepPath, SweepError> {
    let sets: Vec<Value> = values
        .iter()
        .map(|v| {
            let mut m = serde_json::Map::new();
            m.insert(key.to_owned(), v.clone());
            Value::Object(m)
        })
        .collect();
    sweep(ir, ctx, &sets, bars, opts, visit)
}

/// Checks every set laid over `ctx.params`.
fn check_all(ir: &Value, ctx: &CheckCtx<'_>, sets: &[Value]) -> Result<Vec<Checked>, SweepError> {
    let base = ctx
        .params
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::with_capacity(sets.len());
    for (index, set) in sets.iter().enumerate() {
        let merged = match set.as_object() {
            Some(m) => {
                let mut all = base.clone();
                all.extend(m.iter().map(|(k, v)| (k.clone(), v.clone())));
                Value::Object(all)
            }
            // Not an object: the checker reports it as it would for any message.
            None => set.clone(),
        };
        let cx = CheckCtx {
            params: Some(&merged),
            ..ctx.clone()
        };
        let r = check(ir, &cx);
        match r.checked {
            Some(c) => out.push(c),
            None => {
                return Err(SweepError::Check {
                    index,
                    errors: r.errors,
                })
            }
        }
    }
    Ok(out)
}

/// The series outputs asked for: name and slot.
fn selected(c: &Checked, only: Option<&[String]>) -> Result<Vec<(String, Slot)>, SweepError> {
    let series = |o: &crate::checked::CheckedOutput| match o.spec {
        OutputSpec::Series(s) => Some((o.name.clone(), s)),
        _ => None,
    };
    match only {
        None => Ok(c.outputs.iter().filter_map(series).collect()),
        Some(names) => names
            .iter()
            .map(|n| {
                let o = c
                    .outputs
                    .iter()
                    .find(|o| &o.name == n)
                    .ok_or_else(|| ExecError::UnknownOutput(n.clone()))?;
                series(o).ok_or_else(|| SweepError::NotSeries(n.clone()))
            })
            .collect(),
    }
}

/// The swept kernel nodes in the selected outputs' cones, or why the sweep cannot share.
fn analyze(cs: &[Checked], sel: &[(String, Slot)]) -> Result<Vec<Dep>, &'static str> {
    let c0 = &cs[0];
    let n = c0.nodes.len();
    if cs.iter().any(|c| c.nodes.len() != n) {
        return Err("the program's shape varies");
    }
    // Nodes in topological order: a node depends on the swept values when its own
    // operation varies or one of its inputs depends on them.
    let mut dep = vec![false; n];
    for j in 0..n {
        let varies = cs
            .iter()
            .any(|c| c.nodes[j].op != c0.nodes[j].op || c.nodes[j].clock != c0.nodes[j].clock);
        dep[j] = varies || c0.nodes[j].op.inputs().iter().any(|s| dep[s.node]);
    }
    let roots: Vec<Slot> = sel.iter().map(|(_, s)| *s).collect();
    let active = cone_of(c0, &roots);
    let mut deps = Vec::new();
    for j in (0..n).filter(|&j| active[j] && dep[j]) {
        let node = &c0.nodes[j];
        if node.clock != Clock::Base {
            return Err("a swept node off the base clock");
        }
        let x = match node.op {
            Op::Window { x, .. } | Op::Macd { x, .. } | Op::Bollinger { x, .. } => x,
            _ => return Err("a swept node that is not a kernel"),
        };
        if dep[x.node] {
            return Err("a kernel over a swept input");
        }
        deps.push(Dep { node: j, x });
    }
    Ok(deps)
}

/// One executor per variation.
fn per_variation(
    cs: &[Checked],
    names: &[String],
    bars: &[Bar],
    opts: &ExecOptions,
    visit: &mut impl FnMut(&Variation<'_>),
) -> Result<(), SweepError> {
    let o = ExecOptions {
        outputs: Some(names.to_vec()),
        ..opts.clone()
    };
    for (index, c) in cs.iter().enumerate() {
        let x = Executor::run_closed(c, bars, o.clone())?;
        visit(&Variation {
            index,
            checked: c,
            names,
            len: bars.len(),
            cols: (0..names.len()).map(|j| Col::Exec(&x, j)).collect(),
        });
    }
    Ok(())
}
