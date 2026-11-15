//! The world: the source's true event stream per binding, seeded, in virtual time.
//!
//! History bars come from a regime-switching random walk (drift follows a triangle
//! wave of 12–24 bars, so RSI(14) regularly crosses 70 and 30). Live bars are built
//! tick by tick: the first tick of a bar is a new-bar event, every later tick is a
//! patch carrying only the fields that changed (`c`; `h`/`l` when exceeded; `v`
//! cumulative), each with the feed's next source `seq`. A steering plan (`Shape::Steer`)
//! instead aims RSI at levels (hover around 70, close races, invalidation at 60) by
//! inverting Wilder's recursion — the inversion is only used to shape the path; claims
//! are always judged with `qp_compute`.
//!
//! Only exact IEEE operations and `sqrt` are used (no `sin`/`exp`/`ln`), so a seed
//! gives the same world on every platform.

use qp_compute::{Field, Rsi, Streaming};
use qp_schema::data::{BarPatch, DataChange, DataEvent, FeedKey};
use qp_schema::Bar;

use crate::rng::Rng;
use crate::spec::{ms, CorrectionKind, Race, ScenarioSpec, Shape, VTime, BASE_T, LIVE0};

/// Extra facts about a data step (used by feed models and the oracle; not delivered).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickInfo {
    /// The last tick of its bar (decides the close).
    pub last_tick: bool,
    /// Sets a new bar high.
    pub sets_high: bool,
    /// A correction or rescale of history.
    pub correction: bool,
}

/// What happens in the world.
#[derive(Debug, Clone, PartialEq)]
pub enum StepKind {
    /// A source data event (tick, new bar, correction patch, rescale patch).
    Data {
        /// The event as the source emits it (`f64`, true values).
        ev: DataEvent,
        /// Extra facts.
        info: TickInfo,
    },
    /// The source sends a full snapshot: a spontaneous `REFRESH`, or a split /
    /// rescale of the whole history (`correction: true`; a real source re-sends the
    /// adjusted history rather than one patch per bar).
    Snapshot {
        /// Sequence number included.
        seq: u64,
        /// All bars.
        bars: Vec<Bar>,
        /// A history adjustment (counts as a correction for M11).
        correction: bool,
    },
    /// A bar closes (marker for the oracle; feeds do not deliver it — a receiver sees
    /// a bar as closed when the next bar arrives).
    Close {
        /// Bar time.
        bar_t: i64,
    },
}

/// One world step.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldStep {
    /// Virtual time.
    pub at: VTime,
    /// Binding index.
    pub binding: usize,
    /// What.
    pub kind: StepKind,
}

/// The generated world.
#[derive(Debug, Clone)]
pub struct World {
    /// Feed key per binding.
    pub feeds: Vec<FeedKey>,
    /// Series id per binding (`px0`, …).
    pub ids: Vec<String>,
    /// History bars per binding (before the live phase).
    pub history: Vec<Vec<Bar>>,
    /// All steps, time-ordered (ties: binding, then generation order).
    pub steps: Vec<WorldStep>,
    /// Time of the last step.
    pub end: VTime,
    /// Bar interval (ms).
    pub interval_ms: i64,
    /// Generation notes (e.g. a correction that could not be placed).
    pub notes: Vec<String>,
}

impl World {
    /// Virtual time at which the bar opening at `bar_t` closes (≤ 0 for history bars).
    #[must_use]
    pub fn close_vtime(&self, bar_t: i64) -> VTime {
        LIVE0 + (bar_t - BASE_T + self.interval_ms) * 1000
    }
}

fn round_cents(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Triangle wave in [-1, 1] of a phase in turns.
fn tri(phase: f64) -> f64 {
    let p = phase - phase.floor();
    if p < 0.25 {
        4.0 * p
    } else if p < 0.75 {
        2.0 - 4.0 * p
    } else {
        4.0 * p - 4.0
    }
}

/// Wilder averages after the given closes: `(avg_gain, avg_loss, last_close)`.
fn wilder(closes: &[f64], n: usize) -> Option<(f64, f64, f64)> {
    if closes.len() < n + 1 {
        return None;
    }
    let (mut g, mut l) = (0.0, 0.0);
    for i in 1..=n {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 {
            g += d;
        } else {
            l -= d;
        }
    }
    let nf = n as f64;
    let (mut ag, mut al) = (g / nf, l / nf);
    for i in (n + 1)..closes.len() {
        let d = closes[i] - closes[i - 1];
        let (gi, li) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        ag = (ag * (nf - 1.0) + gi) / nf;
        al = (al * (nf - 1.0) + li) / nf;
    }
    Some((ag, al, closes[closes.len() - 1]))
}

fn rsi_of(ag: f64, al: f64) -> f64 {
    if al == 0.0 {
        if ag == 0.0 {
            50.0
        } else {
            100.0
        }
    } else {
        100.0 - 100.0 / (1.0 + ag / al)
    }
}

/// Price of the next bar that puts RSI at `target` on top of state `(ag, al, prev)`.
fn invert(state: (f64, f64, f64), n: usize, target: f64) -> f64 {
    let (ag, al, prev) = state;
    let r = target.clamp(2.0, 98.0);
    let rs = r / (100.0 - r);
    let nf = n as f64;
    let a = ag * (nf - 1.0);
    let c = al * (nf - 1.0);
    let x = if c * rs > a {
        prev + (rs * c - a)
    } else {
        prev - (a / rs - c)
    };
    x.clamp(prev * 0.85, prev * 1.15)
}

fn batch_rsi(closes: &[f64], period: u32) -> Vec<Option<f64>> {
    let mut s = Streaming::new(Rsi::new(period as usize).expect("period"), Field::Close);
    qp_compute::run(&mut s, &[closes]).expect("rsi").remove(0)
}

fn cross_at(rsi: &[Option<f64>], k: usize, up: f64, lo: f64) -> Option<bool> {
    if k == 0 || k >= rsi.len() {
        return None;
    }
    match (rsi[k - 1], rsi[k]) {
        (Some(p), Some(c)) if p <= up && c > up => Some(true),
        (Some(p), Some(c)) if p >= lo && c < lo => Some(false),
        _ => None,
    }
}

enum Intervention {
    Correction(CorrectionKind),
    Rescale(f64),
    Refresh,
}

struct Gen<'a> {
    spec: &'a ScenarioSpec,
    b: usize,
    feed: FeedKey,
    rng: Rng,
    bars: Vec<Bar>,
    seq: u64,
    steps: Vec<WorldStep>,
    notes: Vec<String>,
    // Free-walk regime.
    amp: f64,
    period: f64,
    phase: f64,
    // Steering.
    plan_idx: usize,
    held: u32,
}

impl Gen<'_> {
    fn n(&self) -> usize {
        self.spec.detector.period as usize
    }

    fn round(&self, x: f64) -> f64 {
        let tick = self.spec.price_tick;
        if tick > 0.0 {
            (x / tick).round() * tick
        } else {
            x
        }
    }

    fn drift(&self, k: usize) -> f64 {
        self.amp * tri(k as f64 / self.period + self.phase)
    }

    fn data(&mut self, at: VTime, change: DataChange, info: TickInfo) {
        self.seq += 1;
        self.steps.push(WorldStep {
            at,
            binding: self.b,
            kind: StepKind::Data {
                ev: DataEvent {
                    feed: self.feed.clone(),
                    seq: self.seq,
                    change,
                },
                info,
            },
        });
    }

    fn history(&mut self) {
        let sigma = self.spec.volatility;
        let mut p = 100.0 * self.rng.range_f64(0.6, 1.6);
        let h = self.spec.history_bars as usize;
        let iv = self.spec.interval_ms;
        let sub = 6.0_f64;
        for k in 0..h {
            let d = self.drift(k);
            let o = round_cents(p);
            let (mut hi, mut lo, mut x) = (o, o, o);
            for _ in 0..6 {
                x *= 1.0 + d / sub + sigma / sub.sqrt() * self.rng.normal();
                hi = hi.max(x);
                lo = lo.min(x);
            }
            let c = round_cents(x);
            self.bars.push(Bar {
                t: BASE_T - (h - k) as i64 * iv,
                o,
                h: round_cents(hi).max(o.max(c)),
                l: round_cents(lo).min(o.min(c)),
                c,
                v: (self.rng.range_f64(1000.0, 5000.0)).round(),
            });
            p = c;
        }
    }

    fn closed_closes(&self, live: bool) -> Vec<f64> {
        let n = if live {
            self.bars.len() - 1
        } else {
            self.bars.len()
        };
        self.bars[..n].iter().map(|b| b.c).collect()
    }

    fn patch_bar(&mut self, at: VTime, idx: usize, new: Bar, info: TickInfo) {
        let old = self.bars[idx].clone();
        let mut f = BarPatch::default();
        if new.o != old.o {
            f.o = Some(new.o);
        }
        if new.h != old.h {
            f.h = Some(new.h);
        }
        if new.l != old.l {
            f.l = Some(new.l);
        }
        if new.c != old.c {
            f.c = Some(new.c);
        }
        if f.is_empty() {
            return;
        }
        let t = old.t;
        self.bars[idx] = new;
        self.data(at, DataChange::Patch { t, fields: f }, info);
    }

    fn with_close(bar: &Bar, c: f64) -> Bar {
        let mut b = bar.clone();
        b.c = c;
        b.h = b.h.max(c);
        b.l = b.l.min(c);
        b
    }

    fn correction(&mut self, at: VTime, kind: &CorrectionKind) {
        let n = self.n();
        let up = self.spec.detector.upper;
        let lo = self.spec.detector.lower;
        let closes = self.closed_closes(true);
        let rsi = batch_rsi(&closes, self.spec.detector.period);
        let info = TickInfo {
            correction: true,
            ..TickInfo::default()
        };
        let latest = (n + 2..closes.len())
            .rev()
            .find_map(|k| cross_at(&rsi, k, up, lo).map(|d| (k, d)));
        match kind {
            CorrectionKind::DropClaim => {
                let Some((k, is_up)) = latest else {
                    self.notes
                        .push(format!("b{}: drop_claim: no cross", self.b));
                    return;
                };
                let state = wilder(&closes[..k], n).expect("history");
                for d in [3.0, 6.0, 10.0, 15.0, 25.0] {
                    let target = if is_up { up - d } else { lo + d };
                    let x = round_cents(invert(state, n, target));
                    let mut c2 = closes.clone();
                    c2[k] = x;
                    let r2 = batch_rsi(&c2, self.spec.detector.period);
                    if cross_at(&r2, k, up, lo) != Some(is_up) {
                        let nb = Self::with_close(&self.bars[k], x);
                        self.patch_bar(at, k, nb, info);
                        return;
                    }
                }
                self.notes
                    .push(format!("b{}: drop_claim: not placed", self.b));
            }
            CorrectionKind::KeepClaim => {
                let Some((k, is_up)) = latest else {
                    self.notes
                        .push(format!("b{}: keep_claim: no cross", self.b));
                    return;
                };
                for back in 1..n.min(k) {
                    let i = k - back;
                    for dx in [0.02, -0.02, 0.05, -0.05, 0.1, -0.1] {
                        let mut c2 = closes.clone();
                        c2[i] = round_cents(c2[i] + dx);
                        let r2 = batch_rsi(&c2, self.spec.detector.period);
                        if cross_at(&r2, k, up, lo) == Some(is_up)
                            && r2[k].map(f64::to_bits) != rsi[k].map(f64::to_bits)
                        {
                            let nb = Self::with_close(&self.bars[i], c2[i]);
                            self.patch_bar(at, i, nb, info);
                            return;
                        }
                    }
                }
                self.notes
                    .push(format!("b{}: keep_claim: not placed", self.b));
            }
            CorrectionKind::CreateClaim => {
                let last = closes.len();
                for k in (n + 2..last).rev().take(40) {
                    let (Some(p), Some(_)) = (rsi[k - 1], rsi[k]) else {
                        continue;
                    };
                    if p > up - 1.0 || cross_at(&rsi, k, up, lo).is_some() {
                        continue;
                    }
                    let state = wilder(&closes[..k], n).expect("history");
                    let x = round_cents(invert(state, n, up + 3.0));
                    let mut c2 = closes.clone();
                    c2[k] = x;
                    let r2 = batch_rsi(&c2, self.spec.detector.period);
                    if cross_at(&r2, k, up, lo) == Some(true) {
                        let nb = Self::with_close(&self.bars[k], x);
                        self.patch_bar(at, k, nb, info);
                        return;
                    }
                }
                self.notes
                    .push(format!("b{}: create_claim: not placed", self.b));
            }
            CorrectionKind::Old { bars_back } => {
                let live = self.bars.len() - 1;
                let Some(i) = live.checked_sub(*bars_back as usize).filter(|&i| i > 0) else {
                    self.notes
                        .push(format!("b{}: old: not enough history", self.b));
                    return;
                };
                let x = round_cents(self.bars[i].c * 1.003);
                let nb = Self::with_close(&self.bars[i], x);
                self.patch_bar(at, i, nb, info);
            }
        }
    }

    fn rescale(&mut self, at: VTime, factor: f64) {
        for b in &mut self.bars {
            b.o *= factor;
            b.h *= factor;
            b.l *= factor;
            b.c *= factor;
        }
        self.steps.push(WorldStep {
            at,
            binding: self.b,
            kind: StepKind::Snapshot {
                seq: self.seq,
                bars: self.bars.clone(),
                correction: true,
            },
        });
    }

    #[allow(clippy::too_many_lines)]
    fn live_bar(&mut self, j: u32, interventions: &[(u32, Intervention)]) {
        let spec = self.spec;
        let iv = spec.interval_ms;
        let n = self.n();
        let v0 = LIVE0 + i64::from(j) * iv * 1000;
        let v_close = v0 + iv * 1000;
        let bar_t = BASE_T + i64::from(j) * iv;
        let k_global = self.bars.len();
        let ticks = ((f64::from(spec.ticks_per_bar) * self.rng.range_f64(0.7, 1.3)).round()
            as usize)
            .max(2);
        // Steering decision for this bar.
        let state = wilder(&self.closed_closes(false), n);
        let steer = match &spec.shape {
            Shape::Free => None,
            Shape::Steer { plan, race } if !plan.is_empty() => state.map(|st| {
                let r0 = rsi_of(st.0, st.1);
                let step = plan[self.plan_idx % plan.len()].clone();
                let near = (r0 - step.level).abs() < 10.0;
                let mut race_now: Option<Race> = None;
                let rc = if near {
                    self.held += 1;
                    if self.held >= step.bars {
                        self.plan_idx += 1;
                        self.held = 0;
                    }
                    match race {
                        Some(rc) if r0 <= step.level => {
                            race_now = Some(rc.clone());
                            step.level
                        }
                        Some(rc) => step.level - rc.margin - 2.0,
                        None => step.level + self.rng.range_f64(-1.0, 1.0) * step.close_spread,
                    }
                } else {
                    r0 + 8.0 * (step.level - r0).signum()
                };
                (r0, rc, step, near, race_now)
            }),
            Shape::Steer { .. } => None,
        };
        let race_t = steer
            .as_ref()
            .and_then(|s| s.4.as_ref())
            .map(|r| v_close - ms(r.before_close_ms).max(1000));
        let last_slot = race_t.unwrap_or(v_close - 1000);
        let mut times: Vec<VTime> = (1..ticks)
            .map(|_| v0 + 1000 + self.rng.below((last_slot - v0 - 1000).max(1) as u64) as i64)
            .collect();
        times.sort_unstable();
        if let Some(rt) = race_t {
            if let Some(l) = times.last_mut() {
                *l = rt;
            }
        }
        times.insert(0, v0);
        let psi = self.rng.f64();
        let cross_now = steer
            .as_ref()
            .and_then(|s| s.4.as_ref())
            .is_some_and(|r| self.rng.chance(r.cross_p));
        let sigma = spec.volatility;
        let d_tick = self.drift(k_global) / ticks as f64;
        let s_tick = sigma / (ticks as f64).sqrt();
        let mut x_prev = self.bars.last().map_or(100.0, |b| b.c);
        let mut pending: Vec<&(u32, Intervention)> =
            interventions.iter().filter(|(b, _)| *b == j).collect();
        let v_int = v0 + iv * 450;
        let mut state = state;
        for (k, &at) in times.iter().enumerate() {
            if at >= v_int && !pending.is_empty() {
                x_prev *= self.intervene(v_int, &mut pending);
                state = wilder(&self.closed_closes(true), n);
            }
            let f = (at - v0) as f64 / (iv * 1000) as f64;
            let last = k + 1 == times.len();
            let x = match (&steer, state) {
                (Some((r0, rc, step, near, race_now)), Some(st)) => {
                    let target = if let Some(rc_spec) = race_now {
                        if last {
                            if cross_now {
                                step.level + rc_spec.margin
                            } else {
                                step.level - 1.5 * rc_spec.margin
                            }
                        } else {
                            step.level - rc_spec.margin * (1.0 + 0.3 * tri(step.osc * f + psi))
                        }
                    } else {
                        let env = 1.0 - f * f * f * f;
                        let osc = if *near {
                            step.amp * tri(step.osc * f + psi) * env
                        } else {
                            0.0
                        };
                        if last {
                            *rc
                        } else {
                            r0 + (rc - r0) * f + osc + step.noise * self.rng.normal() * env
                        }
                    };
                    invert(st, n, target)
                }
                _ => {
                    if k == 0 {
                        x_prev * (1.0 + 0.2 * s_tick * self.rng.normal())
                    } else {
                        x_prev * (1.0 + d_tick + s_tick * self.rng.normal())
                    }
                }
            };
            let x = self.round(x.max(0.05));
            let q = (self.rng.range_f64(10.0, 200.0)).round();
            if k == 0 {
                let bar = Bar {
                    t: bar_t,
                    o: x,
                    h: x,
                    l: x,
                    c: x,
                    v: q,
                };
                self.bars.push(bar.clone());
                self.data(
                    at,
                    DataChange::Bar { bar },
                    TickInfo {
                        last_tick: false,
                        sets_high: false,
                        correction: false,
                    },
                );
            } else {
                let li = self.bars.len() - 1;
                let cur = self.bars[li].clone();
                let mut p = BarPatch::default();
                let mut nb = cur.clone();
                if x != cur.c {
                    p.c = Some(x);
                    nb.c = x;
                }
                let sets_high = x > cur.h;
                if sets_high {
                    p.h = Some(x);
                    nb.h = x;
                }
                if x < cur.l {
                    p.l = Some(x);
                    nb.l = x;
                }
                nb.v = cur.v + q;
                p.v = Some(nb.v);
                self.bars[li] = nb;
                self.data(
                    at,
                    DataChange::Patch {
                        t: bar_t,
                        fields: p,
                    },
                    TickInfo {
                        last_tick: last,
                        sets_high,
                        correction: false,
                    },
                );
            }
            x_prev = x;
        }
        if !pending.is_empty() {
            self.intervene(v_int, &mut pending);
        }
        self.steps.push(WorldStep {
            at: v_close,
            binding: self.b,
            kind: StepKind::Close { bar_t },
        });
    }

    /// Applies the pending interventions at `at`; returns the price factor applied.
    fn intervene(&mut self, at: VTime, pending: &mut Vec<&(u32, Intervention)>) -> f64 {
        let mut factor = 1.0;
        for (_, kind) in pending.drain(..) {
            match kind {
                Intervention::Correction(kind) => self.correction(at, kind),
                Intervention::Rescale(f) => {
                    self.rescale(at, *f);
                    factor *= *f;
                }
                Intervention::Refresh => {
                    self.steps.push(WorldStep {
                        at,
                        binding: self.b,
                        kind: StepKind::Snapshot {
                            seq: self.seq,
                            bars: self.bars.clone(),
                            correction: false,
                        },
                    });
                }
            }
        }
        factor
    }

    fn opening_tick(&mut self, j: u32) {
        let iv = self.spec.interval_ms;
        let at = LIVE0 + i64::from(j) * iv * 1000;
        let x = self.bars.last().map_or(100.0, |b| b.c);
        let bar = Bar {
            t: BASE_T + i64::from(j) * iv,
            o: x,
            h: x,
            l: x,
            c: x,
            v: 1.0,
        };
        self.bars.push(bar.clone());
        self.data(at, DataChange::Bar { bar }, TickInfo::default());
    }
}

/// Generates the world of a scenario for a seed.
#[must_use]
pub fn generate(spec: &ScenarioSpec, seed: u64) -> World {
    let mut feeds = Vec::new();
    let mut ids = Vec::new();
    let mut history = Vec::new();
    let mut steps = Vec::new();
    let mut notes = Vec::new();
    let mut interventions: Vec<(u32, Intervention)> = Vec::new();
    for c in &spec.corrections {
        interventions.push((c.at_bar, Intervention::Correction(c.kind.clone())));
    }
    for r in &spec.rescales {
        interventions.push((r.at_bar, Intervention::Rescale(r.factor)));
    }
    for &r in &spec.refreshes {
        interventions.push((r, Intervention::Refresh));
    }
    for b in 0..spec.bindings as usize {
        let feed = FeedKey {
            symbol: format!("SYM{b}"),
            interval: spec.interval_name(),
            feed: None,
        };
        let mut rng = Rng::derive(seed, &format!("world/{b}"));
        let sigma = spec.volatility;
        let mut g = Gen {
            spec,
            b,
            feed: feed.clone(),
            amp: sigma * rng.range_f64(0.5, 0.9),
            period: rng.range_f64(12.0, 24.0),
            phase: rng.f64(),
            rng,
            bars: Vec::new(),
            seq: 0,
            steps: Vec::new(),
            notes: Vec::new(),
            plan_idx: 0,
            held: 0,
        };
        g.history();
        history.push(g.bars.clone());
        for j in 0..spec.live_bars {
            g.live_bar(j, &interventions);
        }
        g.opening_tick(spec.live_bars);
        feeds.push(feed);
        ids.push(format!("px{b}"));
        steps.extend(g.steps.into_iter().enumerate().map(|(i, s)| (b, i, s)));
        notes.extend(g.notes);
    }
    steps.sort_by_key(|(b, i, s)| (s.at, *b, *i));
    let steps: Vec<WorldStep> = steps.into_iter().map(|(_, _, s)| s).collect();
    let end = steps.last().map_or(0, |s| s.at);
    World {
        feeds,
        ids,
        history,
        steps,
        end,
        interval_ms: spec.interval_ms,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inversion_hits_target() {
        let closes: Vec<f64> = (0..60).map(|i| 100.0 + f64::from(i % 7) - 3.0).collect();
        let st = wilder(&closes, 14).unwrap();
        for target in [45.0, 55.0, 60.0, 65.0] {
            let x = invert(st, 14, target);
            let mut c2 = closes.clone();
            c2.push(x);
            let r = batch_rsi(&c2, 14);
            let got = r.last().unwrap().unwrap();
            assert!((got - target).abs() < 1e-6, "target {target} got {got}");
        }
    }

    #[test]
    fn world_has_crosses_and_is_deterministic() {
        let spec = ScenarioSpec::default();
        let a = generate(&spec, 1);
        let b = generate(&spec, 1);
        assert_eq!(a.steps, b.steps);
        let closes: Vec<f64> = a.history[0].iter().map(|b| b.c).collect();
        let rsi = batch_rsi(&closes, 14);
        let crosses = (1..rsi.len())
            .filter(|&k| cross_at(&rsi, k, 70.0, 30.0).is_some())
            .count();
        assert!(crosses >= 4, "only {crosses} crosses in history");
    }
}
