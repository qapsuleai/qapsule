//! The single-threaded deterministic scheduler and one run (scenario × seed × approach).
//!
//! A priority queue ordered by `(VTime, class, tiebreak, insertion)`. Classes at the same
//! instant: world steps (0, the truth moves first), feed deliveries (1), settled-bar
//! answers (2), channel messages and channel state (3), timers (4). The tiebreak is
//! seeded (except world steps, which keep generation order). There is no wall clock in
//! the simulation; the same seed gives a byte-identical display log.
//!
//! The replica is sampled (display + visible data) once at the end of every virtual
//! instant in which anything happened — the screen is rendered between instants.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use qp_schema::data::{DataBinding, Window as BindWindow};

use crate::adapter::{
    Approach, AuthorityAdapter, BindingSetup, ChannelEvent, Ctx, FetchReply, ReplicaAdapter, Setup,
    Side, SourceEvent,
};
use crate::channel::Channel;
use crate::detector::DetectorCfg;
use crate::feed::{deliver, FeedStats};
use crate::metrics::Metrics;
use crate::oracle::Oracle;
use crate::rng::Rng;
use crate::spec::{ms, FeedSpec, ScenarioSpec, VTime, LIVE0};
use crate::world::{generate, World};

enum Ev {
    World(usize),
    Feed(Side, usize, SourceEvent),
    Fetch(Side, FetchReply),
    Chan(Side, Vec<u8>),
    ChanState(ChannelEvent),
    Timer(Side, u64),
}

struct Item {
    key: (VTime, u8, u64, u64),
    ev: Ev,
}

impl PartialEq for Item {
    fn eq(&self, o: &Self) -> bool {
        self.key == o.key
    }
}
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.key.cmp(&o.key)
    }
}

struct Queue {
    heap: BinaryHeap<Reverse<Item>>,
    rng: Rng,
    ins: u64,
}

impl Queue {
    fn push(&mut self, at: VTime, class: u8, tie: Option<u64>, ev: Ev) {
        let tie = tie.unwrap_or_else(|| self.rng.next_u64());
        self.ins += 1;
        self.heap.push(Reverse(Item {
            key: (at, class, tie, self.ins),
            ev,
        }));
    }
}

/// Result of one run.
#[derive(Debug, Clone)]
pub struct RunOutput {
    /// Metrics.
    pub metrics: Metrics,
    /// Display log (bytes).
    pub log: Vec<u8>,
    /// World generation notes.
    pub notes: Vec<String>,
    /// Feed counters per side and binding.
    pub feed_stats: Vec<(Side, usize, FeedStats)>,
}

fn setups(spec: &ScenarioSpec, world: &World) -> (Setup, Setup) {
    let det = DetectorCfg::new(spec.detector.clone());
    let mk = |side: Side| {
        let bindings = (0..world.ids.len())
            .map(|b| {
                let fs: &FeedSpec = match side {
                    Side::Authority => spec.authority_feed_for(b as u32),
                    Side::Replica => spec.replica_feed_for(b as u32),
                };
                let n = fs.window_last.unwrap_or(spec.history_bars);
                BindingSetup {
                    id: world.ids[b].clone(),
                    binding: DataBinding {
                        symbol: world.feeds[b].symbol.clone(),
                        interval: world.feeds[b].interval.clone(),
                        window: BindWindow::last(n),
                        feed: None,
                    },
                    feed: world.feeds[b].clone(),
                    interval_ms: world.interval_ms,
                }
            })
            .collect();
        Setup {
            side,
            bindings,
            detector: det.clone(),
            extra_studies: spec.extra_studies,
        }
    };
    (mk(Side::Authority), mk(Side::Replica))
}

struct Sides {
    auth: Box<dyn AuthorityAdapter>,
    rep: Box<dyn ReplicaAdapter>,
    a2r: Channel,
    r2a: Channel,
    next_fetch: [u64; 2],
}

/// Runs one scenario for one seed with one approach.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn run_one(spec: &ScenarioSpec, approach: &Approach, seed: u64) -> RunOutput {
    let world = generate(spec, seed);
    let (sa, sr) = setups(spec, &world);
    let mut sides = Sides {
        auth: (approach.authority)(&sa),
        rep: (approach.replica)(&sr),
        a2r: Channel::new(
            spec.a2r.clone(),
            spec.disconnects.clone(),
            Rng::derive(seed, "chan/a2r"),
        ),
        r2a: Channel::new(
            spec.r2a.clone(),
            spec.disconnects.clone(),
            Rng::derive(seed, "chan/r2a"),
        ),
        next_fetch: [1, 1],
    };
    let mut q = Queue {
        heap: BinaryHeap::new(),
        rng: Rng::derive(seed, "sched"),
        ins: 0,
    };
    for (i, s) in world.steps.iter().enumerate() {
        q.push(s.at, 0, Some(i as u64), Ev::World(i));
    }
    let mut feed_stats = Vec::new();
    for b in 0..world.ids.len() {
        for side in [Side::Authority, Side::Replica] {
            let (fs, label) = match side {
                Side::Authority => (spec.authority_feed_for(b as u32), "a"),
                Side::Replica => (spec.replica_feed_for(b as u32), "r"),
            };
            let mut rng = Rng::derive(seed, &format!("feed/{label}/{b}"));
            let (evs, st) = deliver(&world, b, fs, &mut rng);
            feed_stats.push((side, b, st));
            for (at, ev) in evs {
                q.push(at, 1, None, Ev::Feed(side, b, ev));
            }
        }
    }
    for w in &spec.disconnects {
        q.push(
            LIVE0 + ms(w.from_ms),
            3,
            None,
            Ev::ChanState(ChannelEvent::Disconnected),
        );
        q.push(w.end(), 3, None, Ev::ChanState(ChannelEvent::Reconnected));
    }
    let end = world.end + ms(spec.drain_ms);
    let mut oracle = Oracle::new(spec, &world);
    let mut cur: VTime = 0;
    let mut any = false;
    let mut touched: BTreeSet<usize> = BTreeSet::new();
    let mut touched_all = false;
    let mut work_events: u64 = 0;
    let mut wall_ns: u128 = 0;
    let rest_delay = ms(spec.rest.delay_ms);
    let settle = ms(spec.rest.settle_lag_ms);
    while let Some(Reverse(item)) = q.heap.pop() {
        let at = item.key.0;
        if at > end {
            break;
        }
        if at > cur && any {
            oracle.assess(cur, sides.rep.as_ref(), (!touched_all).then_some(&touched));
            touched.clear();
            touched_all = false;
        }
        cur = at;
        any = true;
        // Dispatch.
        let mut out: Vec<(Side, Ctx)> = Vec::new();
        match item.ev {
            Ev::World(i) => oracle.on_world(at, &world.steps[i]),
            Ev::Feed(side, b, ev) => {
                let mut ctx = Ctx::new(at, sides.next_fetch[side as usize]);
                match side {
                    Side::Authority => sides.auth.on_source(&mut ctx, &ev),
                    Side::Replica => {
                        touched.insert(b);
                        let t0 = std::time::Instant::now();
                        sides.rep.on_source(&mut ctx, &ev);
                        wall_ns += t0.elapsed().as_nanos();
                        work_events += 1;
                    }
                }
                out.push((side, ctx));
            }
            Ev::Fetch(side, reply) => {
                let mut ctx = Ctx::new(at, sides.next_fetch[side as usize]);
                match side {
                    Side::Authority => sides.auth.on_fetch(&mut ctx, &reply),
                    Side::Replica => {
                        touched_all = true;
                        let t0 = std::time::Instant::now();
                        sides.rep.on_fetch(&mut ctx, &reply);
                        wall_ns += t0.elapsed().as_nanos();
                        work_events += 1;
                    }
                }
                out.push((side, ctx));
            }
            Ev::Chan(to, msg) => {
                let mut ctx = Ctx::new(at, sides.next_fetch[to as usize]);
                match to {
                    Side::Authority => sides.auth.on_replica_msg(&mut ctx, &msg),
                    Side::Replica => {
                        touched_all = true;
                        let t0 = std::time::Instant::now();
                        sides.rep.on_authority_msg(&mut ctx, &msg);
                        wall_ns += t0.elapsed().as_nanos();
                        work_events += 1;
                    }
                }
                out.push((to, ctx));
            }
            Ev::ChanState(e) => {
                let mut ca = Ctx::new(at, sides.next_fetch[0]);
                sides.auth.on_channel(&mut ca, e);
                out.push((Side::Authority, ca));
                let mut cr = Ctx::new(at, sides.next_fetch[1]);
                touched_all = true;
                sides.rep.on_channel(&mut cr, e);
                out.push((Side::Replica, cr));
            }
            Ev::Timer(side, token) => {
                let mut ctx = Ctx::new(at, sides.next_fetch[side as usize]);
                match side {
                    Side::Authority => sides.auth.on_timer(&mut ctx, token),
                    Side::Replica => {
                        touched_all = true;
                        let t0 = std::time::Instant::now();
                        sides.rep.on_timer(&mut ctx, token);
                        wall_ns += t0.elapsed().as_nanos();
                        work_events += 1;
                    }
                }
                out.push((side, ctx));
            }
        }
        // Outboxes.
        for (side, ctx) in out {
            sides.next_fetch[side as usize] = ctx.next_fetch();
            let (chan, to) = match side {
                Side::Authority => (&mut sides.a2r, Side::Replica),
                Side::Replica => (&mut sides.r2a, Side::Authority),
            };
            for m in ctx.msgs {
                for (t, bytes) in chan.send(at, m) {
                    q.push(t, 3, None, Ev::Chan(to, bytes));
                }
            }
            for (t, token) in ctx.timers {
                if t <= end {
                    q.push(t, 4, None, Ev::Timer(side, token));
                }
            }
            for f in ctx.fetches {
                let bars = if f.binding < world.ids.len() {
                    oracle
                        .truth_bars(f.binding)
                        .iter()
                        .filter(|x| {
                            x.t >= f.from_t
                                && x.t <= f.to_t
                                && world.close_vtime(x.t) + settle <= at
                        })
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                };
                q.push(
                    at + rest_delay,
                    2,
                    None,
                    Ev::Fetch(
                        side,
                        FetchReply {
                            id: f.id,
                            binding: f.binding,
                            bars,
                        },
                    ),
                );
            }
        }
    }
    if any {
        oracle.assess(cur, sides.rep.as_ref(), (!touched_all).then_some(&touched));
    }
    sides.a2r.finish();
    sides.r2a.finish();
    let (mut m, log) = oracle.finish(end);
    m.seed = seed;
    m.m9_a2r = sides.a2r.stats.clone();
    m.m9_r2a = sides.r2a.stats.clone();
    #[allow(clippy::cast_precision_loss)]
    {
        m.live_secs = (world.end - LIVE0) as f64 / 1e6;
    }
    m.m10_work = sides.rep.work_units();
    m.m10_replica_events = work_events;
    m.m10_wall_ns = u64::try_from(wall_ns).unwrap_or(u64::MAX);
    RunOutput {
        metrics: m,
        log,
        notes: world.notes,
        feed_stats,
    }
}
