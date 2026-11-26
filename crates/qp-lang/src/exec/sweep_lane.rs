//! The shared path of a program sweep: the unswept nodes run once (one executor over
//! the shared slots), each swept kernel node runs over its shared input — compressed to
//! the defined inputs, which is the `Chain` rule (undefined input: undefined outputs, no
//! commit) — through `qp_compute::sweep` or stepped directly.

use std::collections::BTreeMap;

use qp_compute::sweep::{sweep_each, SweepColumns};

use super::kernel::Kernel;
use super::node::State;
use super::sweep::{Col, SweepError, SweepPath, Variation};
use super::{ExecError, ExecOptions, Executor};
use crate::checked::{Checked, CheckedOutput, Op, OutputSpec, Slot, WindowKind};
use qp_schema::Bar;

/// Bytes of kernel results a chunk of variations may hold when several kernel nodes are
/// swept together.
const CHUNK_BYTES: usize = 64 << 20;

/// A swept kernel node and its (unswept) input.
#[derive(Debug, Clone, Copy)]
pub(super) struct Dep {
    pub(super) node: usize,
    pub(super) x: Slot,
}

/// A swept node's input: the defined values, and each bar's position among them.
struct Input {
    dense: Vec<f64>,
    /// `usize::MAX` for an undefined input; `None` when every input is defined.
    map: Option<Vec<usize>>,
}

/// Where a selected output reads from: a shared column, or output `o` of swept node `d`.
#[derive(Debug, Clone, Copy)]
enum Src {
    Shared(usize),
    Dep(usize, usize),
}

/// One variation's results of one swept node.
enum Res {
    Lane {
        cols: Vec<Vec<f64>>,
        warmup: Vec<usize>,
        names: &'static [&'static str],
    },
    Opt(Vec<Vec<Option<f64>>>),
}

/// A shared-path sweep.
pub(super) struct Run<'a> {
    pub(super) cs: &'a [Checked],
    pub(super) sel: &'a [(String, Slot)],
    pub(super) names: &'a [String],
    pub(super) deps: &'a [Dep],
    pub(super) bars: &'a [Bar],
    pub(super) opts: &'a ExecOptions,
}

/// What every variation shares.
struct Shared {
    cols: Vec<Vec<Option<f64>>>,
    inputs: Vec<Input>,
    srcs: Vec<Src>,
}

impl Run<'_> {
    pub(super) fn go(
        &self,
        visit: &mut impl FnMut(&Variation<'_>),
    ) -> Result<SweepPath, SweepError> {
        let sh = self.shared()?;
        if let [d] = self.deps {
            if let Some(study) = study_of(&self.cs[0].nodes[d.node].op) {
                if self.stream(study, &sh, visit) {
                    return Ok(SweepPath::Shared {
                        swept: 1,
                        direct: 0,
                    });
                }
            }
        }
        self.chunked(&sh, visit)
    }

    /// Runs the unswept part once and wires the outputs.
    fn shared(&self) -> Result<Shared, SweepError> {
        let mut slots: Vec<Slot> = Vec::new();
        let mut push = |s: Slot| match slots.iter().position(|t| *t == s) {
            Some(k) => k,
            None => {
                slots.push(s);
                slots.len() - 1
            }
        };
        let ins: Vec<usize> = self.deps.iter().map(|d| push(d.x)).collect();
        let srcs: Vec<Src> = self
            .sel
            .iter()
            .map(
                |(_, s)| match self.deps.iter().position(|d| d.node == s.node) {
                    Some(d) => Src::Dep(d, s.out),
                    None => Src::Shared(push(*s)),
                },
            )
            .collect();
        let mut c = self.cs[0].clone();
        c.outputs = slots
            .iter()
            .enumerate()
            .map(|(k, s)| CheckedOutput {
                name: format!("s{k}"),
                spec: OutputSpec::Series(*s),
            })
            .collect();
        let o = ExecOptions {
            align: self.opts.align.clone(),
            outputs: None,
            budget: None,
        };
        let x = Executor::run_closed(&c, self.bars, o)?;
        let n = self.bars.len();
        let cols: Vec<Vec<Option<f64>>> = (0..slots.len())
            .map(|k| (0..n).map(|i| x.value_at(k, i)).collect())
            .collect();
        let inputs = ins.iter().map(|&k| compress(&cols[k])).collect();
        Ok(Shared { cols, inputs, srcs })
    }

    fn variation<'a>(&'a self, index: usize, cols: Vec<Col<'a>>) -> Variation<'a> {
        Variation {
            index,
            checked: &self.cs[index],
            names: self.names,
            len: self.bars.len(),
            cols,
        }
    }

    /// One swept node through the compute core's sweep, each variation visited from its
    /// reused buffer; `false` (nothing visited) when the core refuses the sets.
    fn stream(&self, study: &str, sh: &Shared, visit: &mut impl FnMut(&Variation<'_>)) -> bool {
        let node = self.deps[0].node;
        let sets: Vec<BTreeMap<String, f64>> = self
            .cs
            .iter()
            .map(|c| params_of(&c.nodes[node].op))
            .collect();
        let outn = out_names(&self.cs[0].nodes[node].op);
        let input = &sh.inputs[0];
        let r = sweep_each(study, &sets, &input.dense, |v, sc: &SweepColumns| {
            let cols = sh
                .srcs
                .iter()
                .map(|s| match *s {
                    Src::Shared(k) => Col::Opt(&sh.cols[k]),
                    Src::Dep(_, o) => {
                        let j = lane_index(sc.names, outn[o]);
                        Col::Lane {
                            v: &sc.columns[j],
                            warmup: sc.warmup[j],
                            map: input.map.as_deref(),
                        }
                    }
                })
                .collect();
            visit(&self.variation(v, cols));
        });
        r.is_ok()
    }

    /// Several swept nodes: chunks of variations, every node's results held per chunk.
    fn chunked(
        &self,
        sh: &Shared,
        visit: &mut impl FnMut(&Variation<'_>),
    ) -> Result<SweepPath, SweepError> {
        let width: usize = self
            .deps
            .iter()
            .map(|d| out_names(&self.cs[0].nodes[d.node].op).len())
            .sum();
        let per = (self.bars.len() * width * 16).max(1);
        let chunk = (CHUNK_BYTES / per).clamp(1, self.cs.len().max(1));
        let mut direct = vec![false; self.deps.len()];
        for a in (0..self.cs.len()).step_by(chunk) {
            let b = (a + chunk).min(self.cs.len());
            let mut res: Vec<Vec<Res>> = Vec::with_capacity(self.deps.len());
            for (k, d) in self.deps.iter().enumerate() {
                let (r, swept) = self.results(&self.cs[a..b], d, &sh.inputs[k])?;
                direct[k] |= !swept;
                res.push(r);
            }
            for v in a..b {
                let cols = sh
                    .srcs
                    .iter()
                    .map(|s| match *s {
                        Src::Shared(k) => Col::Opt(&sh.cols[k]),
                        Src::Dep(d, o) => res_col(
                            &res[d][v - a],
                            out_names(&self.cs[0].nodes[self.deps[d].node].op)[o],
                            o,
                            sh.inputs[d].map.as_deref(),
                        ),
                    })
                    .collect();
                visit(&self.variation(v, cols));
            }
        }
        let n_direct = direct.iter().filter(|d| **d).count();
        Ok(SweepPath::Shared {
            swept: self.deps.len() - n_direct,
            direct: n_direct,
        })
    }

    /// Node `d`'s results for the variations `cs`; `true` when the core's sweep ran them.
    fn results(
        &self,
        cs: &[Checked],
        d: &Dep,
        input: &Input,
    ) -> Result<(Vec<Res>, bool), SweepError> {
        if let Some(study) = study_of(&cs[0].nodes[d.node].op) {
            let sets: Vec<BTreeMap<String, f64>> =
                cs.iter().map(|c| params_of(&c.nodes[d.node].op)).collect();
            let mut out: Vec<Option<Res>> = (0..cs.len()).map(|_| None).collect();
            let r = sweep_each(study, &sets, &input.dense, |v, sc| {
                out[v] = Some(Res::Lane {
                    cols: sc.columns.clone(),
                    warmup: sc.warmup.clone(),
                    names: sc.names,
                });
            });
            if r.is_ok() && out.iter().all(Option::is_some) {
                return Ok((out.into_iter().flatten().collect(), true));
            }
        }
        let n = self.bars.len();
        let r = cs
            .iter()
            .map(|c| {
                let node = &c.nodes[d.node];
                direct(&node.op, input, n)
                    .map(Res::Opt)
                    .map_err(|e| ExecError::Kernel(node.id.clone(), e).into())
            })
            .collect::<Result<_, SweepError>>()?;
        Ok((r, false))
    }
}

fn res_col<'a>(r: &'a Res, name: &str, o: usize, map: Option<&'a [usize]>) -> Col<'a> {
    match r {
        Res::Lane {
            cols,
            warmup,
            names,
        } => {
            let j = lane_index(names, name);
            Col::Lane {
                v: &cols[j],
                warmup: warmup[j],
                map,
            }
        }
        Res::Opt(c) => Col::Opt(&c[o]),
    }
}

/// The defined inputs of a column and each bar's position among them.
fn compress(col: &[Option<f64>]) -> Input {
    if col.iter().all(Option::is_some) {
        return Input {
            dense: col.iter().flatten().copied().collect(),
            map: None,
        };
    }
    let mut dense = Vec::with_capacity(col.len());
    let map = col
        .iter()
        .map(|v| match v {
            Some(x) => {
                dense.push(*x);
                dense.len() - 1
            }
            None => usize::MAX,
        })
        .collect();
    Input {
        dense,
        map: Some(map),
    }
}

/// A kernel stepped over the defined inputs (the executor's `kernel`: peek, then commit),
/// spread back over the bars.
fn direct(op: &Op, input: &Input, n: usize) -> Result<Vec<Vec<Option<f64>>>, String> {
    let State::Kernel { mut k, .. } = State::new(op).map_err(|e| e.to_string())? else {
        return Err("not a kernel".to_owned());
    };
    let width = out_names(op).len();
    let mut dense: Vec<Vec<Option<f64>>> = vec![Vec::with_capacity(input.dense.len()); width];
    let k: &mut Kernel = &mut k;
    for &x in &input.dense {
        let o = k.peek(x);
        k.commit(x);
        for (col, v) in dense.iter_mut().zip(o) {
            col.push(v);
        }
    }
    Ok(match &input.map {
        None => dense,
        Some(map) => dense
            .iter()
            .map(|c| (0..n).map(|i| c.get(map[i]).copied().flatten()).collect())
            .collect(),
    })
}

/// The compute core's study of a kernel node, when it has a sweep.
fn study_of(op: &Op) -> Option<&'static str> {
    match op {
        Op::Window { kind, .. } => match kind {
            WindowKind::Sma => Some("sma"),
            WindowKind::Ema => Some("ema"),
            WindowKind::Rsi => Some("rsi"),
            _ => None,
        },
        Op::Macd { .. } => Some("macd"),
        Op::Bollinger { .. } => Some("bollinger"),
        _ => None,
    }
}

/// The study parameters of a kernel node.
#[allow(clippy::cast_precision_loss)] // periods are at most `program.max_window`
fn params_of(op: &Op) -> BTreeMap<String, f64> {
    let p = |k: &str, v: f64| (k.to_owned(), v);
    match *op {
        Op::Window { period, .. } => BTreeMap::from([p("period", period as f64)]),
        Op::Macd {
            fast, slow, signal, ..
        } => BTreeMap::from([
            p("fast", fast as f64),
            p("slow", slow as f64),
            p("signal", signal as f64),
        ]),
        Op::Bollinger { period, k, .. } => BTreeMap::from([p("period", period as f64), p("k", k)]),
        _ => BTreeMap::new(),
    }
}

/// A kernel node's output names in the executor's order (PR §4.2).
fn out_names(op: &Op) -> &'static [&'static str] {
    match op {
        Op::Macd { .. } => &["macd", "signal", "histogram"],
        Op::Bollinger { .. } => &["upper", "middle", "lower", "bandwidth"],
        _ => &["value"],
    }
}

/// Position of `name` among a sweep's output names (a single-output study: 0).
fn lane_index(names: &[&str], name: &str) -> usize {
    names.iter().position(|m| *m == name).unwrap_or(0)
}
