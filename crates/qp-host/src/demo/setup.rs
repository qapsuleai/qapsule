//! Building the demo host: recorded, live and simulated ([`live_setup`], [`sim_setup`]; RG1: split out of `demo.rs`).

use super::*;

/// Builds the demo's replay host.
pub(super) fn host(a: &DemoArgs) -> Result<ReplayHost, String> {
    let from_t = match &a.from {
        Some(d) => Some(qp_source::date_start(d).map_err(|e| format!("--from: {e}"))?),
        None => None,
    };
    let o = DemoOptions {
        local: !a.synthetic,
        instruments: a.instruments,
        history: a.history,
        days: a.days,
        speed: a.speed,
        from_t,
        running: !a.paused,
        feed_rule: Some(a.feed_rule.clone()),
        engine: crate::sim::engine_field(&a.rule),
    };
    let mut init = demo_session_init(&o);
    if a.mcp {
        crate::mcp_serve::add_agent_principals(&mut init);
    }
    let dir = if a.synthetic {
        PathBuf::new()
    } else {
        a.data.clone().unwrap_or_else(qp_source::local_bars_dir)
    };
    // (R4/T6) `--history` also bounds the history before `--from` (else every earlier
    // bar: on the full local history the page's first capsule took seconds).
    let mut h = ReplayHost::with_window(init, &a.root, &dir, Some(a.history as usize))?;
    h.set_record_events(true);
    Ok(h)
}

/// (R6/T7) What the live demo needs once the hub exists.
pub(super) struct LiveParts {
    pub(super) control: String,
    pub(super) delayed: bool,
    pub(super) symbols: usize,
    interval: String,
    delay_ms: i64,
    cal: qp_calendar::ExchangeCalendar,
    source: std::sync::Arc<dyn crate::livehost::BarSource>,
    to_hub: std::sync::Arc<dyn Fn(crate::livehost::LiveIn) + Send + Sync>,
    spec: crate::livehost::ConnSpec,
    sink: std::sync::Arc<crate::livehost::HubSink>,
    /// (RP4, L-prep) The trade connection, its sink, the trade history and decoding.
    trade: Option<TradeParts>,
}

/// (RP4, L-prep) The trade side of `qp demo --live --trades`.
pub(super) struct TradeParts {
    spec: crate::livehost::ConnSpec,
    sink: std::sync::Arc<crate::livehost::HubSink>,
    history: std::sync::Arc<dyn qp_matriks::TradeHistory + Send + Sync>,
    decode: qp_matriks::TradeDecodeConfig,
}

/// (RP4, L-prep) `--trade-time-unit`.
fn parse_trade_unit(s: &str) -> Result<qp_matriks::TradeTimeUnit, String> {
    use qp_matriks::TradeTimeUnit as U;
    [
        U::NanosFromMidnight,
        U::EpochMillis,
        U::EpochMicros,
        U::EpochNanos,
        U::Auto,
    ]
    .into_iter()
    .find(|u| u.name() == s)
    .ok_or_else(|| {
        format!("--trade-time-unit `{s}`: ns_from_midnight, epoch_ms, epoch_us, epoch_ns, auto")
    })
}

impl LiveParts {
    /// The runtime (feeds seeded at the history's last bars) and the connection threads
    /// (the market endpoint; with `--trades` also the trade endpoint), by name.
    #[allow(clippy::type_complexity)]
    pub(super) fn start(
        self,
        host: &ReplayHost,
    ) -> Result<
        (
            crate::livehost::LiveRuntime,
            Vec<(&'static str, Sender<crate::livehost::ConnCtl>)>,
        ),
        String,
    > {
        let feed = match &self.trade {
            Some(t) => crate::livehost::MatriksFeed::with_trades(
                self.cal.clone(),
                &self.interval,
                qp_matriks::TradeFeedConfig {
                    decode: t.decode,
                    ..qp_matriks::TradeFeedConfig::default()
                },
            ),
            None => crate::livehost::MatriksFeed::new(self.cal.clone(), &self.interval),
        };
        let mut rt = crate::livehost::LiveRuntime::new(
            host,
            Box::new(feed),
            self.source,
            self.to_hub.clone(),
            self.cal,
            &self.interval,
            self.delay_ms,
        )?;
        rt.baseline_capsules(host);
        let mut ctls = Vec::new();
        let trade = match self.trade {
            Some(t) => {
                rt.set_trade_history(t.history);
                // Trades after each symbol's last history bar are the opening gap.
                rt.watch_trades_after_seams(host.session().clock());
                Some((t.spec, t.sink))
            }
            None => None,
        };
        ctls.push((
            "market",
            crate::livehost::spawn_connection(self.spec, self.sink, self.to_hub.clone())?,
        ));
        if let Some((spec, sink)) = trade {
            match crate::livehost::spawn_connection(spec, sink, self.to_hub) {
                Ok(c) => ctls.push(("trade", c)),
                Err(e) => {
                    // Close the market connection before giving up (one process, no leak).
                    let (tx, rx) = mpsc::channel();
                    let _ = ctls[0].1.send(crate::livehost::ConnCtl::Stop(tx));
                    let _ = rx.recv_timeout(Duration::from_secs(10));
                    return Err(format!("trade endpoint: {e}"));
                }
            }
        }
        Ok((rt, ctls))
    }
}

/// (R6/T7) The live demo's host: the universe's history from REST is the initial
/// resolution; the connection is opened later ([`LiveParts::start`]).
pub(super) fn live_setup(
    a: &DemoArgs,
    tx: &Sender<Cmd>,
) -> Result<(ReplayHost, LiveParts), String> {
    use std::sync::Arc;
    let symbols = crate::live::parse_symbols(&a.symbols).map_err(|e| {
        format!("--symbols: {e} (default: the universe file fixtures/local/live/u30.txt)")
    })?;
    crate::livehost::rest_period(&a.interval)
        .ok_or_else(|| format!("--interval: no REST period for `{}`", a.interval))?;
    // (R6 audit B2) The identity lock before any token request (REST history included);
    // held for the life of the process (released by the OS on exit).
    std::mem::forget(crate::live::identity_lock()?);
    let cfg = qp_live::LiveConfig::from_env().map_err(|e| format!("--live: {e}"))?;
    if cfg.hardware_id_warning() {
        eprintln!("live: warning: MATRIKS_HARDWARE_ID does not start with `qapsule-`");
    }
    let xist = qp_calendar::builtin("XIST").ok_or("no XIST calendar")?;
    let cal = qp_calendar::ExchangeCalendar::new(&xist).map_err(|e| e.to_string())?;
    let delayed = cfg.mode() == qp_live::Mode::Delayed;
    let delay_ms = if delayed { 15 * 60_000 } else { 0 };
    let auth = Arc::new(qp_live::AuthClient::new(
        &cfg,
        qp_live::AuthPolicy::default(),
    ));
    let source: Arc<dyn crate::livehost::BarSource> = match &a.live_bars {
        Some(dir) => Arc::new(crate::livehost::DirSource { dir: dir.clone() }),
        None => Arc::new(crate::livehost::RestSource {
            cfg: cfg.clone(),
            auth: auth.clone(),
            cal: cal.clone(),
        }),
    };
    println!(
        "geçmiş / history: {} symbols x {} bars of {} ({}, unadjusted)…",
        symbols.len(),
        a.history,
        a.interval,
        if a.live_bars.is_some() {
            "local files"
        } else {
            "REST"
        }
    );
    let _ = std::io::stdout().flush();
    let cutoff = crate::live::now_us() / 1000 - delay_ms;
    let (hist, missing) = crate::livehost::history(
        source.as_ref(),
        &symbols,
        &a.interval,
        a.history as usize,
        cutoff,
        &cal,
    )?;
    if !missing.is_empty() {
        println!("geçmiş yok / no history: {}", missing.join(", "));
    }
    if hist.is_empty() {
        return Err("--live: no symbol has history".to_owned());
    }
    let n = hist.len();
    let topics: Vec<String> = hist
        .iter()
        .map(|(s, _)| format!("mx/symbol/{s}@lvl2"))
        .collect();
    let syms: Vec<String> = hist.iter().map(|(s, _)| s.clone()).collect();
    let mut h = crate::livehost::host_from_history(
        hist,
        &a.interval,
        &DemoOptions {
            local: true,
            instruments: 0,
            history: a.history,
            days: a.days,
            speed: a.speed,
            from_t: None,
            running: true,
            feed_rule: Some(a.feed_rule.clone()),
            engine: crate::sim::engine_field(&a.rule),
        },
        &a.root,
    )?;
    h.set_note(Some(format!(
        "canlı / live · {n} · {}{}{}",
        a.interval,
        if a.trades { " · işlem / trades" } else { "" },
        if delayed {
            " · gecikmeli 15 dk / delayed 15 min"
        } else {
            ""
        }
    )));
    // Raw log and lock under `--root` (the repository root: `scripts/demo.sh` runs there).
    let root = a.root.clone();
    let raw_path = root.join("fixtures/local/live/raw").join(format!(
        "{}-demo.qpraw",
        crate::live::utc_stamp(crate::live::now_us())
    ));
    let txc = Mutex::new(tx.clone());
    let to_hub: Arc<dyn Fn(crate::livehost::LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = txc
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(Cmd::Live(i));
    });
    let sink = Arc::new(crate::livehost::HubSink::new(
        crate::live::RawSink::create(&raw_path)?,
        to_hub.clone(),
    ));
    let control = match std::env::var("QP_LIVE_CONTROL") {
        Ok(v) if v.len() >= 16 => v,
        _ => {
            let s = secret()?;
            println!("canlı denetim belirteci / live control secret: {s}");
            s
        }
    };
    let trade = if a.trades {
        let decode = qp_matriks::TradeDecodeConfig {
            unit: parse_trade_unit(&a.trade_time_unit)?,
            ..qp_matriks::TradeDecodeConfig::default()
        };
        let mut trade_topics = Vec::new();
        for s in &syms {
            trade_topics
                .extend(qp_matriks::trade_topics(s, a.tradeex).map_err(|e| format!("{s}: {e}"))?);
        }
        let trade_raw = root.join("fixtures/local/live/raw").join(format!(
            "{}-demo-trade.qpraw",
            crate::live::utc_stamp(crate::live::now_us())
        ));
        let history: Arc<dyn qp_matriks::TradeHistory + Send + Sync> = match &a.live_trades {
            Some(dir) => Arc::new(crate::livehost::RestTradeHistory::dir(
                dir.clone(),
                cal.clone(),
            )),
            None => Arc::new(crate::livehost::RestTradeHistory::rest(
                cfg.clone(),
                auth.clone(),
                cal.clone(),
                Some(root.join("fixtures/local/trades/requests.jsonl")),
            )),
        };
        Some(TradeParts {
            spec: crate::livehost::ConnSpec {
                cfg: cfg.clone(),
                auth: auth.clone(),
                topics: trade_topics,
                lock_dir: qp_live::lock::default_lock_dir(),
                trade: true,
            },
            sink: Arc::new(crate::livehost::HubSink::for_trades(
                crate::live::RawSink::create(&trade_raw)?.for_trade_endpoint(),
                to_hub.clone(),
            )),
            history,
            decode,
        })
    } else {
        None
    };
    let parts = LiveParts {
        control,
        delayed,
        symbols: n,
        interval: a.interval.clone(),
        delay_ms,
        cal,
        source,
        to_hub,
        spec: crate::livehost::ConnSpec {
            cfg,
            auth,
            topics,
            lock_dir: qp_live::lock::default_lock_dir(),
            trade: false,
        },
        sink,
        trade,
    };
    Ok((h, parts))
}

/// (RP4/T4) What the simulated demo needs once the hub exists.
pub(super) struct SimParts {
    pub(super) control: String,
    pub(super) symbols: usize,
    pub(super) speed: crate::sim::Speed,
    pub(super) input: crate::sim::SimInput,
    pub(super) runtime: crate::livehost::LiveRuntime,
    pub(super) to_hub: std::sync::Arc<dyn Fn(crate::livehost::LiveIn) + Send + Sync>,
}

/// (RP4/T4) The simulated demo: the caches' history is the initial resolution, the live
/// runtime runs on the simulated clock (`crate::sim`); the feeder starts with the hub.
pub(super) fn sim_setup(a: &DemoArgs, tx: &Sender<Cmd>) -> Result<(ReplayHost, SimParts), String> {
    use std::sync::Arc;
    let start = a.start.as_deref().ok_or("--sim needs --start YYYY-MM-DD")?;
    let speed = crate::sim::Speed::parse(&a.sim_speed)?;
    let spec = crate::sim::SimSpec::new(
        &a.symbols,
        start,
        a.end.as_deref(),
        &a.interval,
        a.history,
        &a.feed_rule,
        &a.rule,
        (&a.sim_trades, &a.sim_rest, &a.root),
    )?
    .with_at(a.sim_at.as_deref())?;
    println!(
        "benzetim / simulation: {} symbols, {}{}, {} (caches under fixtures/local)…",
        spec.symbols.len(),
        start,
        a.end
            .as_deref()
            .map(|e| format!("..{e}"))
            .unwrap_or_default(),
        a.interval
    );
    let _ = std::io::stdout().flush();
    let input = crate::sim::load(&spec)?;
    if !input.missing.is_empty() {
        println!("geçmiş yok / no history: {} symbols", input.missing.len());
    }
    let txc = Mutex::new(tx.clone());
    let to_hub: Arc<dyn Fn(crate::livehost::LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = txc
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(Cmd::Live(i));
    });
    let (mut h, rt) = crate::sim::runtime(
        &spec,
        &input,
        to_hub.clone(),
        qp_matriks::TradeDecodeConfig::default(),
    )?;
    let n = input.hist.len();
    h.set_note(Some(format!(
        "benzetim / simulation · {n} · {} · {start}{} · {}",
        a.interval,
        a.end
            .as_deref()
            .map(|e| format!("..{e}"))
            .unwrap_or_default(),
        speed.label()
    )));
    let control = match std::env::var("QP_LIVE_CONTROL") {
        Ok(v) if v.len() >= 16 => v,
        _ => {
            let s = secret()?;
            println!("denetim belirteci / control secret: {s}");
            s
        }
    };
    Ok((
        h,
        SimParts {
            control,
            symbols: n,
            speed,
            input,
            runtime: rt,
            to_hub,
        },
    ))
}

/// The machine's address on the local network (a routing lookup; no packet is sent).
pub(super) fn local_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("10.254.254.254:1").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}
