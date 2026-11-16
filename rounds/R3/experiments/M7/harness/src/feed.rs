//! Feed model: turns the world's true source stream of one binding into what one
//! receiver (authority or replica) gets, with seeded delay, jitter, reordering, loss,
//! duplication, outages (+ snapshot on reconnect), conflation, price representation and
//! glitches. Deliveries are `SourceEvent`s in the `qp_schema::data::DataEvent` form;
//! a lost event leaves a gap in `seq`.
//!
//! Delivery is FIFO (a delivery is never earlier than the previous one, +1 µs) except
//! for events selected for reordering and duplicates.

use qp_schema::data::{BarPatch, DataChange, DataEvent};
use qp_schema::Bar;

use crate::adapter::SourceEvent;
use crate::rng::Rng;
use crate::spec::{FeedSpec, PriceRepr, VTime};
use crate::world::{StepKind, TickInfo, World};

/// Counters of what a feed did (for reports).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeedStats {
    /// Events the source emitted for this binding.
    pub source_events: u64,
    /// Events delivered (including duplicates and snapshots).
    pub delivered: u64,
    /// Events lost.
    pub dropped: u64,
    /// Duplicates delivered.
    pub duplicated: u64,
    /// Events delivered out of order.
    pub reordered: u64,
    /// Snapshots delivered.
    pub snapshots: u64,
}

struct Out<'a> {
    spec: &'a FeedSpec,
    rng: &'a mut Rng,
    last: VTime,
    out: Vec<(VTime, SourceEvent)>,
    stats: FeedStats,
}

impl Out<'_> {
    fn emit(&mut self, at: VTime, ev: SourceEvent) {
        let d = self.spec.delay.sample(self.rng);
        let t = if self.rng.chance(self.spec.reorder_p) {
            self.stats.reordered += 1;
            at + d + crate::spec::ms(self.rng.range_f64(0.0, self.spec.reorder_ms))
        } else {
            let t = (at + d).max(self.last + 1);
            self.last = t;
            t
        };
        if self.rng.chance(self.spec.dup_p) {
            self.stats.duplicated += 1;
            let lag = crate::spec::ms(self.rng.range_f64(0.0, self.spec.dup_ms)) + 1;
            self.out.push((t + lag, ev.clone()));
        }
        self.stats.delivered += 1;
        self.out.push((t, ev));
    }
}

fn repr(x: f64, r: PriceRepr) -> f64 {
    match r {
        PriceRepr::F64 => x,
        #[allow(clippy::cast_possible_truncation)]
        PriceRepr::F32 => f64::from(x as f32),
    }
}

fn convert_bar(b: &Bar, r: PriceRepr) -> Bar {
    Bar {
        t: b.t,
        o: repr(b.o, r),
        h: repr(b.h, r),
        l: repr(b.l, r),
        c: repr(b.c, r),
        v: b.v,
    }
}

fn map_prices(change: &mut DataChange, f: impl Fn(f64) -> f64) {
    match change {
        DataChange::Bar { bar } => {
            bar.o = f(bar.o);
            bar.h = f(bar.h);
            bar.l = f(bar.l);
            bar.c = f(bar.c);
        }
        DataChange::Patch { fields, .. } => {
            for x in [&mut fields.o, &mut fields.h, &mut fields.l, &mut fields.c]
                .into_iter()
                .flatten()
            {
                *x = f(*x);
            }
        }
    }
}

fn apply_source(bars: &mut Vec<Bar>, change: &DataChange) {
    match change {
        DataChange::Bar { bar } => match bars.binary_search_by_key(&bar.t, |b| b.t) {
            Ok(i) => bars[i] = bar.clone(),
            Err(i) => bars.insert(i, bar.clone()),
        },
        DataChange::Patch { t, fields } => {
            if let Ok(i) = bars.binary_search_by_key(t, |b| b.t) {
                fields.apply_to(&mut bars[i]);
            }
        }
    }
}

fn merge(into: &mut BarPatch, p: &BarPatch) {
    if p.o.is_some() {
        into.o = p.o;
    }
    if p.h.is_some() {
        into.h = p.h;
    }
    if p.l.is_some() {
        into.l = p.l;
    }
    if p.c.is_some() {
        into.c = p.c;
    }
    if p.v.is_some() {
        into.v = p.v;
    }
}

/// Delivers binding `b` of `world` through `spec`. Returns time-stamped deliveries
/// (unsorted; the scheduler orders them) and counters.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn deliver(
    world: &World,
    b: usize,
    spec: &FeedSpec,
    rng: &mut Rng,
) -> (Vec<(VTime, SourceEvent)>, FeedStats) {
    let feed = world.feeds[b].clone();
    let r = spec.price_repr;
    let conflate = crate::spec::ms(spec.conflate_ms);
    let mut o = Out {
        spec,
        rng,
        last: -1,
        out: Vec::new(),
        stats: FeedStats::default(),
    };
    let mut source: Vec<Bar> = world.history[b].clone();
    let mut source_seq: u64 = 0;
    let mut renum: u64 = 0;
    let renumber = conflate > 0;
    // Initial snapshot (binding resolution).
    let init: Vec<Bar> = match spec.window_last {
        Some(n) => {
            let n = (n as usize).min(source.len());
            source[source.len() - n..].to_vec()
        }
        None => source.clone(),
    };
    let window_from = init.first().map_or(i64::MIN, |x| x.t);
    o.stats.snapshots += 1;
    o.emit(
        0,
        SourceEvent::Snapshot {
            feed: feed.clone(),
            seq: 0,
            bars: init.iter().map(|x| convert_bar(x, r)).collect(),
        },
    );
    let snapshot_of = |source: &[Bar], seq: u64| SourceEvent::Snapshot {
        feed: feed.clone(),
        seq,
        bars: source
            .iter()
            .filter(|x| x.t >= window_from)
            .map(|x| convert_bar(x, r))
            .collect(),
    };
    let mut pending_refresh: Option<VTime> = None;
    // Conflation buffer: (window end, bar t, merged patch).
    let mut buf: Option<(VTime, i64, BarPatch)> = None;
    let flush =
        |o: &mut Out, buf: &mut Option<(VTime, i64, BarPatch)>, now: VTime, renum: &mut u64| {
            if let Some((end, t, p)) = buf.take() {
                *renum += 1;
                o.emit(
                    end.min(now),
                    SourceEvent::Data(DataEvent {
                        feed: feed.clone(),
                        seq: *renum,
                        change: DataChange::Patch { t, fields: p },
                    }),
                );
            }
        };
    for step in world.steps.iter().filter(|s| s.binding == b) {
        if let Some(end) = pending_refresh {
            if step.at >= end {
                pending_refresh = None;
                flush(&mut o, &mut buf, end, &mut renum);
                if spec.refresh_on_reconnect {
                    o.stats.snapshots += 1;
                    let seq = if renumber { renum } else { source_seq };
                    o.emit(end, snapshot_of(&source, seq));
                }
            }
        }
        let outage = spec.outages.iter().find(|w| w.contains(step.at));
        match &step.kind {
            StepKind::Close { .. } => {}
            StepKind::Snapshot { seq, bars, .. } => {
                source.clone_from(bars);
                source_seq = *seq;
                if outage.is_none() {
                    flush(&mut o, &mut buf, step.at, &mut renum);
                    o.stats.snapshots += 1;
                    let seq = if renumber { renum } else { source_seq };
                    o.emit(step.at, snapshot_of(&source, seq));
                }
            }
            StepKind::Data { ev, info } => {
                o.stats.source_events += 1;
                apply_source(&mut source, &ev.change);
                source_seq = ev.seq;
                if let Some(w) = outage {
                    o.stats.dropped += 1;
                    pending_refresh = Some(pending_refresh.map_or(w.end(), |p| p.max(w.end())));
                    continue;
                }
                let TickInfo {
                    last_tick,
                    sets_high,
                    correction,
                } = *info;
                if !correction
                    && (o.rng.chance(spec.drop_p)
                        || (last_tick && o.rng.chance(spec.drop_last_tick_p))
                        || (sets_high && o.rng.chance(spec.drop_high_tick_p)))
                {
                    o.stats.dropped += 1;
                    continue;
                }
                let mut change = ev.change.clone();
                if o.rng.chance(spec.glitch_p) {
                    let k = if o.rng.chance(0.5) {
                        1.0 + spec.glitch_size
                    } else {
                        1.0 - spec.glitch_size
                    };
                    map_prices(&mut change, |x| (x * k * 100.0).round() / 100.0);
                }
                map_prices(&mut change, |x| repr(x, r));
                if renumber {
                    match &change {
                        DataChange::Patch { t, fields } if !correction => {
                            if let Some((end, bt, p)) = &mut buf {
                                if *bt == *t && step.at < *end {
                                    merge(p, fields);
                                    continue;
                                }
                            }
                            flush(&mut o, &mut buf, step.at, &mut renum);
                            let end = (step.at / conflate + 1) * conflate;
                            buf = Some((end, *t, fields.clone()));
                        }
                        _ => {
                            flush(&mut o, &mut buf, step.at, &mut renum);
                            renum += 1;
                            o.emit(
                                step.at,
                                SourceEvent::Data(DataEvent {
                                    feed: feed.clone(),
                                    seq: renum,
                                    change,
                                }),
                            );
                        }
                    }
                } else {
                    o.emit(
                        step.at,
                        SourceEvent::Data(DataEvent {
                            feed: feed.clone(),
                            seq: ev.seq,
                            change,
                        }),
                    );
                }
            }
        }
    }
    flush(&mut o, &mut buf, VTime::MAX, &mut renum);
    if let Some(end) = pending_refresh {
        if spec.refresh_on_reconnect {
            o.stats.snapshots += 1;
            let seq = if renumber { renum } else { source_seq };
            o.emit(end, snapshot_of(&source, seq));
        }
    }
    (o.out, o.stats)
}
