//! What the executor publishes, closed bar by closed bar (PR §5, §6, §9): series values,
//! event bars (1/0, with run counts kept incrementally for `try`), signal records with
//! their outcome (resolved on later closed bars only), state keys and their counts — each
//! with its provenance `lo`. Nothing here is written for a live bar.

use std::collections::BTreeMap;

use qp_schema::program::{OutcomeKind, Side};
use qp_schema::Bar;

use crate::checked::Rule;

/// A signal record as the executor sees it (bar indexes; PR §6 fields come from
/// [`crate::SignalCtx`] when it is published).
#[derive(Debug, Clone, PartialEq)]
pub struct SignalHit {
    /// Signal bar.
    pub bar: u64,
    /// Entry: the signal bar's close.
    pub entry: f64,
    /// Provenance: the first base bar the signal rests on.
    pub lo: u64,
    /// The first state output's key at the signal bar, if the program has one.
    pub state: Option<Vec<u32>>,
    /// How it ended (`Open` until resolved).
    pub outcome: OutcomeKind,
    /// Exit bar (the outcome's provenance is `[bar + 1, exit_bar]`).
    pub exit_bar: Option<u64>,
    /// Exit price: the stop or target level, or the close at expiry.
    pub exit_price: Option<f64>,
}

/// Levels of an open record.
#[derive(Debug, Clone, Copy)]
pub(super) struct Levels {
    target: Option<f64>,
    stop: Option<f64>,
}

/// One output's record.
#[derive(Debug, Clone)]
pub(super) enum OutRec {
    Series {
        v: Vec<Option<f64>>,
        lo: Vec<u64>,
        defined: u64,
    },
    Event {
        v: Vec<bool>,
        lo: Vec<u64>,
        bars: Vec<u64>,
        runs: u64,
        isolated: u64,
        /// Length of the run ending at the last closed bar (0: no event there).
        run: u64,
    },
    Signal {
        side: Side,
        rule: Rule,
        hits: Vec<SignalHit>,
        levels: Vec<Levels>,
        /// Indexes into `hits` of the open records.
        open: Vec<usize>,
    },
    State {
        keys: Vec<Option<Vec<u32>>>,
        lo: Vec<u64>,
        counts: BTreeMap<Vec<u32>, u64>,
    },
}

/// What one output gives at one closed bar.
#[derive(Debug, Clone)]
pub(super) enum Staged {
    Value(Option<f64>, u64),
    Flag(bool, u64),
    Key(Option<Vec<u32>>, u64),
}

impl Staged {
    /// Records born by it (event bar, signal record, state key; PR §8.3).
    pub(super) fn born(&self) -> u64 {
        match self {
            Staged::Value(..) => 0,
            Staged::Flag(f, _) => u64::from(*f),
            Staged::Key(k, _) => u64::from(k.is_some()),
        }
    }
}

impl OutRec {
    /// Open signal records (charged per bar while they stay open).
    pub(super) fn open(&self) -> u64 {
        match self {
            OutRec::Signal { open, .. } => open.len() as u64,
            _ => 0,
        }
    }

    /// Records closed bar `i` (`bar`): first resolves open signals on it, then appends.
    pub(super) fn push(&mut self, i: u64, bar: &Bar, s: Staged, state: Option<&Vec<u32>>) {
        match (self, s) {
            (OutRec::Series { v, lo, defined }, Staged::Value(x, l)) => {
                *defined += u64::from(x.is_some());
                v.push(x);
                lo.push(l);
            }
            (
                OutRec::Event {
                    v,
                    lo,
                    bars,
                    runs,
                    isolated,
                    run,
                },
                Staged::Flag(f, l),
            ) => {
                if f {
                    bars.push(i);
                    if *run == 0 {
                        *runs += 1;
                    }
                    *run += 1;
                } else {
                    if *run == 1 {
                        *isolated += 1;
                    }
                    *run = 0;
                }
                v.push(f);
                lo.push(l);
            }
            (
                OutRec::Signal {
                    side,
                    rule,
                    hits,
                    levels,
                    open,
                },
                Staged::Flag(f, l),
            ) => {
                resolve(*side, rule, hits, levels, open, i, bar);
                if f {
                    levels.push(levels_of(*side, rule, bar.c));
                    open.push(hits.len());
                    hits.push(SignalHit {
                        bar: i,
                        entry: bar.c,
                        lo: l,
                        state: state.cloned(),
                        outcome: OutcomeKind::Open,
                        exit_bar: None,
                        exit_price: None,
                    });
                }
            }
            (OutRec::State { keys, lo, counts }, Staged::Key(k, l)) => {
                if let Some(k) = &k {
                    *counts.entry(k.clone()).or_insert(0) += 1;
                }
                keys.push(k);
                lo.push(l);
            }
            _ => {}
        }
    }
}

/// Target and stop levels (RX4 §6; the reference's operation order).
fn levels_of(side: Side, rule: &Rule, entry: f64) -> Levels {
    let up = |p: f64| entry * (1.0 + p / 100.0);
    let down = |p: f64| entry * (1.0 - p / 100.0);
    match side {
        Side::Al => Levels {
            target: rule.target_pct.map(up),
            stop: rule.stop_pct.map(down),
        },
        Side::Sat => Levels {
            target: rule.target_pct.map(down),
            stop: rule.stop_pct.map(up),
        },
    }
}

/// Resolves open records on closed bar `j` (PR §6): stop before target (both in one bar
/// is a stop), else expiry at `bar + max_bars` (exit at that close).
fn resolve(
    side: Side,
    rule: &Rule,
    hits: &mut [SignalHit],
    levels: &[Levels],
    open: &mut Vec<usize>,
    j: u64,
    bar: &Bar,
) {
    open.retain(|&ix| {
        let (h, lv) = (&mut hits[ix], levels[ix]);
        let (stop_hit, target_hit) = match side {
            Side::Al => (
                lv.stop.is_some_and(|s| bar.l <= s),
                lv.target.is_some_and(|t| bar.h >= t),
            ),
            Side::Sat => (
                lv.stop.is_some_and(|s| bar.h >= s),
                lv.target.is_some_and(|t| bar.l <= t),
            ),
        };
        let end = if stop_hit {
            Some((OutcomeKind::Stop, lv.stop))
        } else if target_hit {
            Some((OutcomeKind::Target, lv.target))
        } else if j >= h.bar + rule.max_bars as u64 {
            Some((OutcomeKind::Expired, Some(bar.c)))
        } else {
            None
        };
        match end {
            Some((kind, price)) => {
                h.outcome = kind;
                h.exit_bar = Some(j);
                h.exit_price = price;
                false
            }
            None => true,
        }
    });
}
