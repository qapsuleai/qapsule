//! The runtime's clock tick, capsule watch and status ([`LiveRuntime::tick`], [`LiveRuntime::stats`]; RG1: split out of `livehost.rs`).

use super::*;

/// The process's resident memory (KiB) and CPU time (`ps`; `None` if unavailable).
fn process_usage() -> (Option<u64>, Option<String>) {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=,time=", "-p", &std::process::id().to_string()])
        .output();
    let Ok(o) = out else { return (None, None) };
    let s = String::from_utf8_lossy(&o.stdout);
    let mut it = s.split_whitespace();
    (
        it.next().and_then(|x| x.parse().ok()),
        it.next().map(str::to_owned),
    )
}

impl LiveRuntime {
    /// (T3, §24) The market status now: exchange time (the feed's clock), open and
    /// holiday from the XIST calendar, delayed, stale, each symbol's phase.
    #[must_use]
    pub fn market_status(&self) -> qp_schema::market::MarketStatus {
        let t = if self.is_stale() || self.clock > 0 && self.link == Link::Live {
            self.clock
        } else {
            self.exchange_now()
        };
        let now = if self.is_stale() {
            self.clock
        } else {
            self.exchange_now().max(self.clock)
        };
        let day = self.cal.local_day(now);
        let trading = self.cal.is_trading_day(day);
        let open = trading
            && self
                .cal
                .sessions_on(day)
                .iter()
                .any(|s| s.open <= now && now < s.close);
        let phases = self
            .sym_phase
            .iter()
            .filter_map(|(s, c)| {
                let p = qp_matriks::session_phase(*c)?;
                Some((
                    s.clone(),
                    qp_schema::market::PhaseCode {
                        code: *c,
                        key: p.key.to_owned(),
                    },
                ))
            })
            .collect();
        qp_schema::market::MarketStatus {
            exchange_time: t.max(0),
            open: Some(open),
            holiday: Some(!trading),
            source: qp_schema::market::MarketSource::Calendar,
            delayed: self.delayed,
            stale: self.is_stale(),
            phases,
        }
    }

    /// Gives the authority the market status when it changed (phases, link) or every
    /// 10 s (exchange time, open).
    fn publish_market(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        if !self.market_dirty && self.now() - self.market_at_us < 10_000_000 {
            return Vec::new();
        }
        let m = self.market_status();
        self.market_dirty = false;
        self.market_at_us = self.now();
        if self.market.as_ref() == Some(&m) {
            return Vec::new();
        }
        self.market = Some(m.clone());
        host.set_market_status(m)
    }

    /// Periodic work (call often): the market status when due; closes bars by the
    /// exchange clock once a second.
    pub fn tick(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        let mut lines = self.publish_market(host);
        if self.now() - self.last_advance_us < 1_000_000 || self.link != Link::Live {
            return lines;
        }
        self.last_advance_us = self.now();
        if !self.is_simulated() && self.last_print.elapsed() >= Duration::from_secs(30) {
            self.last_print = Instant::now();
            println!(
                "live: {} · {} messages · {} events · {} capsule events",
                self.link(),
                self.messages,
                self.events,
                self.capsule_log.len()
            );
        }
        let t0 = Instant::now();
        let now = self.exchange_now();
        let out = self.feed.advance_clock(now);
        lines.extend(self.apply(host, out, None));
        // (RP4, L-prep) A gap whose history failed is asked for again every 30 s.
        if self.trade_mode
            && !self.trade_fill_sync
            && self.trade_up
            && self.trade_fetch.is_none()
            && self.now() - self.trade_fetch_at_us >= 30_000_000
        {
            let (_, gaps) = self.feed.trade_gaps(now);
            if !gaps.is_empty() {
                self.start_trade_fetch();
            }
        }
        self.busy += t0.elapsed();
        lines
    }

    /// After a broadcast: the latency samples of the events just sent (arrival → frame).
    pub fn after_broadcast(&mut self) {
        if self.arrivals.is_empty() {
            return;
        }
        let now = now_us();
        for a in self.arrivals.drain(..) {
            if self.latency_us.len() >= MAX_SAMPLES {
                self.latency_us.remove(0);
            }
            self.latency_us.push(now - a);
        }
    }

    /// Notes capsules born or changed since the last call (no values; the symbol from
    /// the capsule's first fact's series).
    pub fn watch_capsules(&mut self, host: &ReplayHost) {
        let doc = host.session().document();
        if doc.capsules.len() == self.capsules.len()
            && doc.capsules.iter().all(|c| {
                self.capsules
                    .get(&c.id)
                    .is_some_and(|(s, r)| *r == c.revision && *s == format!("{:?}", c.state))
            })
        {
            return;
        }
        let stale = self.is_stale();
        for c in &doc.capsules {
            let state = format!("{:?}", c.state).to_lowercase();
            let symbol = || {
                let f = doc.findings.iter().find(|f| f.id == c.finding)?;
                let fact = doc.facts.iter().find(|x| f.facts.first() == Some(&x.id))?;
                let s = doc.series.iter().find(|s| s.id == fact.series)?;
                Some(s.instrument.as_ref()?.symbol.clone())
            };
            let prev = self.capsules.get(&c.id).cloned();
            let st = format!("{:?}", c.state);
            match prev {
                None => {
                    self.capsule_log.push(CapsuleEvent {
                        kind: "born",
                        state,
                        t: host.session().clock(),
                        symbol: symbol().unwrap_or_default(),
                        stale,
                    });
                }
                Some((s, _)) if s != st => {
                    self.capsule_log.push(CapsuleEvent {
                        kind: "state",
                        state,
                        t: host.session().clock(),
                        symbol: symbol().unwrap_or_default(),
                        stale,
                    });
                }
                _ => {}
            }
            self.capsules.insert(c.id.clone(), (st, c.revision));
        }
    }

    /// Takes the capsule set at start (capsules from history are not "born live").
    pub fn baseline_capsules(&mut self, host: &ReplayHost) {
        for c in &host.session().document().capsules {
            self.capsules
                .insert(c.id.clone(), (format!("{:?}", c.state), c.revision));
        }
    }

    /// Capsules born or changed while live, and those while stale (must be 0).
    #[must_use]
    pub fn capsule_counts(&self) -> (usize, usize) {
        let stale = self.capsule_log.iter().filter(|e| e.stale).count();
        (self.capsule_log.len() - stale, stale)
    }

    /// Aggregate statistics (counts, times; no market values).
    #[must_use]
    pub fn stats(&self, host: &ReplayHost) -> Value {
        let secs = self.started.elapsed().as_secs_f64().max(1e-3);
        #[allow(clippy::cast_precision_loss)]
        let rate = self.messages as f64 / secs;
        let (rss_kb, cpu) = process_usage();
        let repairs: Vec<Value> = self
            .repairs
            .iter()
            .map(|r| {
                json!({"stale_ms": r.stale_ms, "fetch_ms": r.fetch_ms, "repair_ms": r.total_ms,
                       "feeds": r.feeds, "patched": r.patched, "added": r.added,
                       "failed": r.failed, "buffered": r.buffered})
            })
            .collect();
        let caps: Vec<Value> = self
            .capsule_log
            .iter()
            .rev()
            .take(200)
            .map(|e| {
                json!({"kind": e.kind, "state": e.state, "t": e.t, "symbol": e.symbol,
                       "while_stale": e.stale})
            })
            .collect();
        let (live_caps, stale_caps) = self.capsule_counts();
        let trade_fills: Vec<Value> = self
            .trade_fills
            .iter()
            .map(|r| {
                json!({"fetch_ms": r.fetch_ms, "repair_ms": r.total_ms, "gaps": r.gaps,
                       "failed": r.failed, "trades": r.trades, "after_loss": r.after_loss})
            })
            .collect();
        json!({
            "link": self.link.name(),
            "trade_mode": self.trade_mode,
            "market_connected": self.market_up,
            "trade_connected": self.trade_up,
            "trade_fills": trade_fills,
            "trade_repairs": {"filled": self.trade_repairs.0, "failed": self.trade_repairs.1},
            "delayed": self.delayed,
            "feeds": self.feeds.len(),
            "interval": self.interval,
            "uptime_s": secs.round(),
            "messages": self.messages,
            "messages_per_s": (rate * 10.0).round() / 10.0,
            "events": self.events,
            "finalized_bars": self.finalized,
            "events_dropped_while_stale": self.dropped_while_stale,
            "events_rejected": self.rejected,
            "hub_busy_ms": u64::try_from(self.busy.as_millis()).unwrap_or(u64::MAX),
            "hub_busy_pct": ((self.busy.as_secs_f64() / secs) * 1000.0).round() / 10.0,
            "process_rss_kb": rss_kb,
            "process_cpu": cpu,
            "tick_to_frame_ms": {
                "n": self.latency_us.len(),
                "p50": pct(&self.latency_us, 0.5) as f64 / 1000.0,
                "p95": pct(&self.latency_us, 0.95) as f64 / 1000.0,
                "max": self.latency_us.iter().max().copied().unwrap_or(0) as f64 / 1000.0,
            },
            "exchange_lag_ms": self.lag_ms,
            "item_memo": {"reused": host.session().item_memo_counts().0,
                          "written": host.session().item_memo_counts().1},
            "bindings_not_ready": host.live_not_ready(),
            "feed": self.feed.stats_json(),
            "market": self.market.as_ref().map(|m| json!({"open": m.open, "holiday": m.holiday,
                "stale": m.stale, "delayed": m.delayed, "exchange_time": m.exchange_time,
                "phase_symbols": m.phases.len()})),
            "repairs": repairs,
            "drops_asked": self.drops_asked,
            "connection_events": self.conn_events,
            "halted": self.halted,
            "session_phases_seen": self.phases.iter().map(|(k, v)| (k.to_string(), *v)).collect::<BTreeMap<_, _>>(),
            "capsules": {"born_or_changed_live": live_caps, "born_or_changed_while_stale": stale_caps,
                          "recent": caps},
        })
    }
}
