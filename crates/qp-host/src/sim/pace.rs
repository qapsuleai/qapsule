//! Schedule, records, pace and driving ([`Pacer`], [`drive`], [`runtime`]; RG1: split out of `sim.rs`).

use super::*;

/// The clock points besides the trades (UTC ms, ascending): for every trading day in
/// `[from_ms, to_ms)` the session opens, every bar close of `interval` and the session
/// closes; the local midnight after each day; and `to_ms`.
#[must_use]
pub(crate) fn schedule(
    cal: &ExchangeCalendar,
    interval: Interval,
    from_ms: i64,
    to_ms: i64,
) -> Vec<i64> {
    let mut v = BTreeSet::new();
    let (d0, d1) = (cal.local_day(from_ms), cal.local_day(to_ms - 1));
    for day in d0..=d1 {
        for s in cal.sessions_on(day) {
            v.insert(s.open);
            v.insert(s.close);
        }
        v.insert(local_midnight(cal, day + 1));
    }
    for b in cal.bars_between(interval, from_ms, to_ms) {
        v.insert(b.close);
    }
    v.insert(to_ms);
    v.into_iter()
        .filter(|&t| t > from_ms && t <= to_ms)
        .collect()
}

/// The wire message of `t` as the trade endpoint sends it (level 1; buyer and seller
/// absent), its `timestamp` in `cfg.unit` (since-midnight: of the trade's local day).
#[must_use]
pub(crate) fn wire(t: &Trade, cfg: &TradeDecodeConfig) -> Vec<u8> {
    let ns = t.timestamp_ns;
    let off_ns = i64::from(cfg.utc_offset_min) * 60_000_000_000;
    let day_ns = 86_400_000_000_000i64;
    let ts = match cfg.unit {
        TradeTimeUnit::NanosFromMidnight | TradeTimeUnit::Auto => (ns + off_ns).rem_euclid(day_ns),
        TradeTimeUnit::EpochMillis => ns.div_euclid(1_000_000),
        TradeTimeUnit::EpochMicros => ns.div_euclid(1000),
        TradeTimeUnit::EpochNanos => ns,
    };
    #[allow(clippy::cast_possible_truncation)]
    let price = t.price as f32;
    TradeMessage {
        symbol: t.symbol.clone(),
        order_no: t.order_no.clone(),
        price: Some(price),
        quantity: t.quantity,
        active_bid_or_ask: t.side.as_ref().map(|s| match s {
            qp_matriks::Aggressor::Bid => "B".to_owned(),
            qp_matriks::Aggressor::Ask => "A".to_owned(),
            qp_matriks::Aggressor::Other(x) => x.clone(),
        }),
        timestamp: Some(ts),
        buyer: None,
        seller: None,
        is_trade_ex: Some(t.is_trade_ex),
    }
    .encode_to_vec()
}

/// The raw record of `t` arriving at its own exchange time (what the trade endpoint's
/// sink hands the hub).
#[must_use]
pub(crate) fn record(t: &Trade, cfg: &TradeDecodeConfig) -> RawRecord {
    let topic = if t.is_trade_ex {
        format!("mx/tradeex/{}", t.symbol)
    } else {
        format!("mx/trade/{}", t.symbol)
    };
    RawRecord::message(t.timestamp_ns.div_euclid(1000), topic, wire(t, cfg))
}

/// The trade endpoint's connection record at `t_ms`.
#[must_use]
pub(crate) fn conn_record(kind: RecordKind, t_ms: i64) -> RawRecord {
    RawRecord {
        t_us: t_ms * 1000,
        kind,
        topic: String::new(),
        payload: b"session 1".to_vec(),
    }
}

// ---------------------------------------------------------------------------------------
// Pace.

/// Real time for the pacer (a fake one in tests).
pub(crate) trait PaceClock {
    /// Real time since an arbitrary origin.
    fn now(&self) -> Duration;
    /// Waits `d`.
    fn sleep(&self, d: Duration);
}

/// The wall clock.
pub(crate) struct WallPace(Instant);

impl WallPace {
    /// Starting now.
    #[must_use]
    pub fn new() -> Self {
        Self(Instant::now())
    }
}

impl Default for WallPace {
    fn default() -> Self {
        Self::new()
    }
}

impl PaceClock for WallPace {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Sends simulation time at a pace ([`Speed`]): inside a session N× real time, a closed
/// stretch at most [`CLOSED_PAUSE`].
pub(crate) struct Pacer<'a> {
    speed: Speed,
    clock: &'a dyn PaceClock,
    cal: ExchangeCalendar,
    origin: Duration,
    /// Real time the simulation should have reached.
    target: Duration,
    last_ms: Option<i64>,
}

impl<'a> Pacer<'a> {
    /// A pacer starting at `clock.now()`.
    #[must_use]
    pub fn new(speed: Speed, clock: &'a dyn PaceClock, cal: ExchangeCalendar) -> Self {
        Self {
            speed,
            origin: clock.now(),
            clock,
            cal,
            target: Duration::ZERO,
            last_ms: None,
        }
    }

    /// Waits until simulation time `t_ms` is due.
    pub fn wait_until(&mut self, t_ms: i64) {
        let Speed::Factor(f) = self.speed else {
            return;
        };
        let Some(prev) = self.last_ms.replace(t_ms) else {
            return;
        };
        if t_ms <= prev {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let real = Duration::from_secs_f64((t_ms - prev) as f64 / 1000.0 / f);
        let real = if self.cal.in_session(prev) {
            real
        } else {
            real.min(CLOSED_PAUSE)
        };
        self.target += real;
        let elapsed = self.clock.now().saturating_sub(self.origin);
        if self.target > elapsed {
            self.clock.sleep(self.target - elapsed);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Driving.

/// Counts of a drive (no values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DriveStats {
    /// Trade records sent.
    pub trades: u64,
    /// Clock moves sent.
    pub clock_moves: u64,
    /// Local days the clock went through.
    pub days: u64,
}

/// Sends the simulation's inputs to `sink` in order: the clock to the start, the trade
/// connection opening, then every trade (as its wire record) and clock point
/// ([`schedule`]) in time order, each clock move paced by `pacer`. `sink` returns
/// `false` to stop early.
pub(crate) fn drive(
    input: &SimInput,
    interval: Interval,
    decode: &TradeDecodeConfig,
    pacer: &mut Pacer<'_>,
    sink: &mut dyn FnMut(LiveIn) -> bool,
) -> DriveStats {
    let mut st = DriveStats::default();
    let ticks = schedule(&input.cal, interval, input.from_ms, input.to_ms);
    let mut clock = input.from_ms;
    let mut day = input.cal.local_day(clock);
    st.days = 1;
    pacer.wait_until(clock);
    if !sink(LiveIn::Clock(clock * 1000)) {
        return st;
    }
    if !sink(LiveIn::Trade(conn_record(
        RecordKind::ConnectionOpened,
        clock,
    ))) {
        return st;
    }
    let mut j = 0usize;
    let mut trades = input.trades();
    loop {
        let trade_t = trades.peek_t_ms();
        let tick_t = ticks.get(j).copied();
        let (t, is_tick) = match (trade_t, tick_t) {
            (Some(a), Some(b)) if b <= a => (b, true),
            (Some(a), _) => (a, false),
            (None, Some(b)) => (b, true),
            (None, None) => break,
        };
        if t > clock {
            pacer.wait_until(t);
            clock = t;
            st.clock_moves += 1;
            let d = input.cal.local_day(t.min(input.to_ms - 1));
            if d != day {
                day = d;
                st.days += 1;
            }
            if !sink(LiveIn::Clock(t * 1000)) {
                return st;
            }
        }
        if is_tick {
            j += 1;
        } else {
            let Some(t) = trades.next() else { break };
            if !sink(LiveIn::Trade(record(&t, decode))) {
                return st;
            }
            st.trades += 1;
        }
    }
    st
}

/// A simulation's runtime: the live host over the history and the live runtime on the
/// simulated clock, watching every symbol's trades from the start.
///
/// # Errors
/// The host or the runtime refuses the history.
pub(crate) fn runtime(
    spec: &SimSpec,
    input: &SimInput,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
    decode: TradeDecodeConfig,
) -> Result<(ReplayHost, LiveRuntime), String> {
    let host = host(spec, input)?;
    let feed = MatriksFeed::with_trades(
        input.cal.clone(),
        &spec.interval,
        qp_matriks::TradeFeedConfig {
            decode,
            ..qp_matriks::TradeFeedConfig::default()
        },
    );
    let mut rt = LiveRuntime::new(
        &host,
        Box::new(feed),
        input.source.clone(),
        to_hub,
        input.cal.clone(),
        &spec.interval,
        0,
    )?;
    rt.use_sim_clock(input.from_ms * 1000);
    rt.set_trade_history(input.history.clone());
    rt.watch_trades(input.from_ms);
    rt.baseline_capsules(&host);
    Ok((host, rt))
}

/// The live host of a simulation (`crate::livehost::host_from_history`, as `qp demo
/// --live` builds it).
///
/// # Errors
/// The host refuses the history.
pub(crate) fn host(spec: &SimSpec, input: &SimInput) -> Result<ReplayHost, String> {
    let mut host = crate::livehost::host_from_history(
        input.hist.clone(),
        &spec.interval,
        &qp_shellcore::shell::DemoOptions {
            local: true,
            instruments: 0,
            history: spec.history,
            days: 260,
            speed: 4.0,
            from_t: None,
            running: true,
            feed_rule: Some(spec.feed_rule.clone()),
            engine: engine_field(&spec.rule),
        },
        &spec.root,
    )?;
    host.set_record_events(true);
    Ok(host)
}
