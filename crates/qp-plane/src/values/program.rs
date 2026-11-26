//! Program output series (RL1, `docs/PROGRAMS-SESSION.md` §1.2–§1.3): a
//! `SeriesSource::Study` carrying `program` is computed by the `qp-lang` executor, only
//! the cone of its named output. The stored IR is checked again (registry limits, bound
//! values); the alignment source is the input bar series' calendar.
//!
//! - A `series` output: the node value per bar, the transient value on the live bar.
//! - An `event` output: 1 on event bars, 0 on the other closed bars and on bars known at
//!   or before `since_t`, `None` on the live bar; [`Computed::len`] is the closed length
//!   (conditions read the last closed bar).
//!
//! Incremental: an appended bar is `push`ed (the previous live bar closes), a revised
//! live bar is `update_last`; anything else is a rebuild. Bit-equal to a rebuild: the
//! executor's batch run is the same pushes.

use qp_lang::{
    check_program, Align, Binding, CheckCtx, ExecOptions, Executor, Limits, OutputSpec, Purpose,
};
use qp_schema::program::ProgramRef;
use qp_schema::{Bar, Series, SeriesSource};

use super::stream::room;
use super::{Computed, Delta};

/// Streaming state of a program output series.
pub(super) struct ProgLive {
    exec: Executor,
    /// The output's cursor index.
    out: usize,
    /// An event output (closed length, `null` live bar).
    event: bool,
    since_t: i64,
    /// The input has a live bar (not a closed window).
    open: bool,
}

/// What a program series is computed from.
pub(super) struct ProgInput<'a> {
    /// The program.
    pub program: &'a ProgramRef,
    /// The output's name.
    pub output: &'a str,
    /// The input bar series.
    pub input: &'a Series,
    /// Its bars.
    pub bars: &'a [Bar],
}

/// True if the last bar of bar series `s` is live: every bar series except a bound one
/// with a closed window (`to_t`; `claims::closed`).
fn has_live_bar(s: &Series) -> bool {
    !matches!(&s.source, SeriesSource::Bind { window, .. } if window.to_t.is_some())
}

/// The interval of bar series `s`.
fn interval_of(s: &Series) -> Option<String> {
    match (&s.interval, &s.source) {
        (Some(iv), _) => Some(iv.clone()),
        (None, SeriesSource::Bind { interval, .. }) => Some(interval.clone()),
        _ => None,
    }
}

/// Computes a program output series from scratch: its values and streaming state.
///
/// # Errors
/// The stored program does not check (registry limits, bound values, the input's
/// interval), the output is unknown or not documentable, or the executor refuses it.
pub(super) fn run(p: &ProgInput<'_>) -> Result<(Computed, ProgLive), String> {
    let params = serde_json::to_value(&p.program.params).map_err(|e| e.to_string())?;
    let ctx = CheckCtx {
        limits: Limits::default(),
        binding: Some(Binding {
            interval: interval_of(p.input),
            bars: None,
        }),
        params: Some(&params),
        purpose: Purpose::Document,
        ..CheckCtx::default()
    };
    let report = check_program(&p.program.ir, &ctx);
    let checked = match report.checked {
        Some(c) if report.errors.is_empty() => c,
        _ => {
            let first = report.errors.first().map_or("", |e| e.reason.as_str());
            return Err(format!(
                "program `{}` does not check: {first}",
                p.program.instance
            ));
        }
    };
    let event = match checked.outputs.iter().find(|o| o.name == p.output) {
        Some(o) => match o.spec {
            OutputSpec::Series(_) => false,
            OutputSpec::Event(_) => true,
            _ => return Err(format!("output `{}` is not documentable", p.output)),
        },
        None => return Err(format!("program has no output `{}`", p.output)),
    };
    let calendar = p
        .input
        .instrument
        .as_ref()
        .and_then(|i| i.calendar.as_deref());
    let opts = ExecOptions {
        align: Align::for_calendar(calendar),
        outputs: Some(vec![p.output.to_owned()]),
        budget: None,
    };
    let mut exec = Executor::new(&checked, opts).map_err(|e| e.to_string())?;
    let open = has_live_bar(p.input);
    let n = p.bars.len();
    for (i, b) in p.bars.iter().enumerate() {
        let r = if open && i + 1 == n {
            exec.push(b.clone())
        } else {
            exec.push_closed(b.clone())
        };
        r.map_err(|e| e.to_string())?;
    }
    let out = exec
        .output(p.output)
        .ok_or_else(|| format!("program has no output `{}`", p.output))?;
    let live = ProgLive {
        exec,
        out,
        event,
        since_t: p.program.since_t,
        open,
    };
    let mut t = Vec::with_capacity(room(n));
    t.extend(p.bars.iter().map(|b| b.t));
    let mut col = Vec::with_capacity(room(n));
    col.extend((0..n).map(|i| live.value(i)));
    let mut outputs = std::collections::BTreeMap::new();
    outputs.insert(p.output.to_owned(), col);
    let mut c = Computed::new(t, outputs, p.output.to_owned());
    live.set_len(&mut c);
    Ok((c, live))
}

impl ProgLive {
    /// The document value at bar `i` (PS §1.2).
    fn value(&self, i: usize) -> Option<f64> {
        self.exec.doc_value_at(self.out, i, Some(self.since_t))
    }

    /// Records the evaluable length of an event series.
    fn set_len(&self, c: &mut Computed) {
        if self.event {
            c.closed = Some(self.exec.closed_len());
        }
    }
}

/// Advances a program series; false if it must be recomputed.
pub(super) fn update(p: &mut ProgLive, c: &mut Computed, bars: &[Bar], delta: Delta) -> bool {
    let n = bars.len();
    if !p.open || c.outputs.len() != 1 {
        return false;
    }
    let ok = match delta {
        Delta::Appended { from } if c.t.len() == from && from <= n && from > 0 => {
            append(p, c, bars, from)
        }
        Delta::Revised { index } if c.t.len() == n && index + 1 == n => {
            let ok = p.exec.update_last(bars[index].clone()).is_ok();
            if ok {
                let v = p.value(index);
                if let Some(col) = c.outputs.values_mut().next() {
                    col[index] = v;
                }
            }
            ok
        }
        _ => false,
    };
    if ok {
        p.set_len(c);
    }
    ok
}

/// New bars from `from` on: the live bar closes with its final data, every new bar but
/// the last is closed, the last is live.
fn append(p: &mut ProgLive, c: &mut Computed, bars: &[Bar], from: usize) -> bool {
    let prev = &bars[from - 1];
    if p.exec.live_bar() != Some(prev) && p.exec.update_last(prev.clone()).is_err() {
        return false;
    }
    let n = bars.len();
    for (i, b) in bars.iter().enumerate().skip(from) {
        let r = if i + 1 == n {
            p.exec.push(b.clone())
        } else {
            p.exec.push_closed(b.clone())
        };
        if r.is_err() {
            return false;
        }
    }
    let Some(col) = c.outputs.values_mut().next() else {
        return false;
    };
    // The bar that closed: its value is final now (an event becomes 0/1; `known_t` of a
    // bar without an interval becomes known).
    col[from - 1] = p.value(from - 1);
    for (i, b) in bars.iter().enumerate().skip(from) {
        c.t.push(b.t);
        col.push(p.value(i));
    }
    true
}
