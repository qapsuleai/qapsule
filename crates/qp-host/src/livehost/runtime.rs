//! The runtime's inputs: records, drops, REST repair and trade fills ([`LiveRuntime::on_in`]; RG1: split out of `livehost.rs`).

use super::*;

impl LiveRuntime {
    /// One input from the connection thread or a repair thread.
    pub fn on_in(&mut self, host: &mut ReplayHost, input: LiveIn) -> Vec<OutboundLine> {
        let t0 = Instant::now();
        let lines = match input {
            LiveIn::Record(rec) => self.on_record(host, FeedSource::Market, rec),
            LiveIn::Trade(rec) => self.on_record(host, FeedSource::Trade, rec),
            LiveIn::TradeHistory {
                generation,
                results,
                fetch_ms,
            } => self.on_trade_history(host, generation, results, fetch_ms),
            LiveIn::Clock(t_us) => match self.sim_now_us {
                Some(now) => {
                    self.sim_now_us = Some(now.max(t_us));
                    self.tick(host)
                }
                None => Vec::new(),
            },
            LiveIn::Sync(done) => {
                let _ = done.send(());
                Vec::new()
            }
            LiveIn::Arrival(t_us) => {
                self.sim_arrival_us = Some(t_us);
                Vec::new()
            }
            LiveIn::Repaired {
                generation,
                bars,
                fetch_ms,
            } => self.on_repaired(host, generation, &bars, fetch_ms),
            LiveIn::Note(n) => {
                if self.conn_events.len() < 500 {
                    self.conn_events
                        .push(format!("{} {n}", crate::live::utc_stamp(self.now())));
                }
                Vec::new()
            }
            LiveIn::Halted(r) => {
                self.conn_events.push(format!("halted: {r}"));
                self.halted = Some(r);
                let mut lines = Vec::new();
                if !self.is_stale() {
                    lines = self.go_stale(host);
                }
                self.link = Link::Halted;
                lines
            }
        };
        self.busy += t0.elapsed();
        lines
    }

    fn on_record(
        &mut self,
        host: &mut ReplayHost,
        source: FeedSource,
        rec: RawRecord,
    ) -> Vec<OutboundLine> {
        if rec.kind == RecordKind::Message {
            self.messages += 1;
            // A bar repair holds the messages back; a trade fill does not (the feed holds
            // the gap symbols' trades itself, the cards go on).
            if self.link == Link::Repairing && !self.trade_mode {
                self.buffer.push((source, rec));
                return Vec::new();
            }
        }
        let arrival = rec.t_us;
        let out = self.feed.push_from(source, &rec);
        self.apply(host, out, Some(arrival))
    }

    fn go_stale(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        self.market_dirty = true;
        self.marked_stale = true;
        self.generation += 1;
        self.link = Link::Stale;
        self.stale_since = Some(Instant::now());
        self.stale_from_t = Some(self.clock);
        self.buffer.clear();
        self.conn_events
            .push(format!("lost at {}", crate::live::utc_stamp(self.now())));
        host.live_mark_stale()
    }

    pub(super) fn apply(
        &mut self,
        host: &mut ReplayHost,
        out: FeedOut,
        arrival_us: Option<i64>,
    ) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        if let Some(c) = out.connection {
            match c {
                ConnChange::Lost if self.trade_mode => {
                    // Bars come from trades: the market connection's loss stales nothing.
                    self.market_up = false;
                    self.market_dirty = true;
                    self.conn_events.push(format!(
                        "market lost at {}",
                        crate::live::utc_stamp(self.now())
                    ));
                }
                ConnChange::Opened { session } if self.trade_mode => {
                    self.market_up = true;
                    self.market_dirty = true;
                    self.conn_events.push(format!(
                        "market opened #{session} at {}",
                        crate::live::utc_stamp(self.now())
                    ));
                }
                ConnChange::Lost => {
                    self.market_up = false;
                    if matches!(self.link, Link::Live | Link::Connecting | Link::Repairing) {
                        lines.extend(self.go_stale(host));
                    }
                }
                ConnChange::Opened { session } => {
                    self.market_up = true;
                    self.conn_events.push(format!(
                        "opened #{session} at {}",
                        crate::live::utc_stamp(self.now())
                    ));
                    match self.link {
                        Link::Connecting => {
                            self.link = Link::Live;
                            self.market_dirty = true;
                        }
                        Link::Stale => self.start_repair(),
                        _ => {}
                    }
                }
            }
        }
        match out.trade_connection {
            Some(ConnChange::Opened { session }) => {
                self.trade_up = true;
                self.conn_events.push(format!(
                    "trade opened #{session} at {}",
                    crate::live::utc_stamp(self.now())
                ));
                lines.extend(self.on_trade_opened(host));
            }
            Some(ConnChange::Lost) => {
                self.trade_up = false;
                self.conn_events.push(format!(
                    "trade lost at {}",
                    crate::live::utc_stamp(self.now())
                ));
                if self.trade_mode
                    && matches!(self.link, Link::Live | Link::Connecting | Link::Repairing)
                {
                    lines.extend(self.go_stale(host));
                }
            }
            None => {}
        }
        if let (Some(c), Some(a)) = (out.clock_ms, arrival_us) {
            self.note_lag(a, c);
        }
        if let Some(c) = out.clock_ms {
            if c > self.clock && !self.is_stale() {
                self.clock = c;
                match host.session_mut().host_clock(c) {
                    Ok(l) => lines.extend(l),
                    Err(e) => self.conn_events.push(format!("clock: {}", e.message)),
                }
            }
        }
        for (sym, code) in &out.phases {
            *self.phases.entry(*code).or_default() += 1;
            self.sym_phase.insert(sym.clone(), *code);
            self.market_dirty = true;
        }
        self.finalized += out.finalized.len() as u64;
        if !out.events.is_empty() {
            if self.is_stale() {
                self.dropped_while_stale += out.events.len() as u64;
            } else {
                for ev in &out.events {
                    match host.live_event(ev) {
                        Ok(l) => {
                            self.events += 1;
                            lines.extend(l);
                        }
                        Err(_) => self.rejected += 1,
                    }
                }
                // Tick-to-frame is a wall-clock measure: on the simulated clock it is taken
                // from the paced simulation's wall-clock arrival (`LiveIn::Arrival`), if any.
                let sample = match self.sim_now_us {
                    None => arrival_us,
                    Some(_) => arrival_us.and(self.sim_arrival_us),
                };
                if let Some(a) = sample {
                    if self.arrivals.len() < MAX_SAMPLES {
                        self.arrivals.push(a);
                    }
                }
            }
        }
        lines
    }

    fn start_repair(&mut self) {
        self.link = Link::Repairing;
        self.repair_started = Some(Instant::now());
        let generation = self.generation;
        let src = self.source.clone();
        let to_hub = self.to_hub.clone();
        let symbols: Vec<String> = self.feeds.iter().map(|k| k.symbol.clone()).collect();
        let interval = self.interval.clone();
        // From the stale point's bar (that bar may have been live) to now.
        let from = self.stale_from_t.unwrap_or(self.clock) - 86_400_000;
        let to = self.exchange_now();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let bars = src.fetch(&symbols, &interval, from, to);
            to_hub(LiveIn::Repaired {
                generation,
                bars,
                fetch_ms: u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
        });
    }

    fn on_repaired(
        &mut self,
        host: &mut ReplayHost,
        generation: u64,
        bars: &[(String, Result<Vec<Bar>, String>)],
        fetch_ms: u64,
    ) -> Vec<OutboundLine> {
        if generation != self.generation || self.link != Link::Repairing {
            return Vec::new();
        }
        let cutoff = self.exchange_now();
        let (mut patched, mut added, mut failed) = (0, 0, 0);
        let by_symbol: BTreeMap<&str, &Result<Vec<Bar>, String>> =
            bars.iter().map(|(s, r)| (s.as_str(), r)).collect();
        for k in self.feeds.clone() {
            let Some((store, seq)) = host.live_feed(&k) else {
                continue;
            };
            match by_symbol.get(k.symbol.as_str()) {
                Some(Ok(settled)) => {
                    let settled = complete_by(settled.clone(), self.iv, &self.cal, cutoff);
                    let (evs, p, a) = repair_events(&k, &store, seq, &settled);
                    patched += p;
                    added += a;
                    for ev in &evs {
                        if host.live_event(ev).is_err() {
                            self.rejected += 1;
                        }
                    }
                }
                _ => failed += 1,
            }
        }
        let mut lines = host.live_refresh();
        let seeds: Vec<Seed> = self
            .feeds
            .iter()
            .filter_map(|k| {
                let (store, seq) = host.live_feed(k)?;
                Some((k.symbol.clone(), store.last().cloned(), seq + 1, None))
            })
            .collect();
        if let Err(e) = self.feed.seed_all(&seeds) {
            self.conn_events.push(format!("seed: {e}"));
        }
        self.link = Link::Live;
        self.market_dirty = true;
        let buffered = std::mem::take(&mut self.buffer);
        let n_buf = buffered.len();
        for (source, rec) in buffered {
            let a = rec.t_us;
            let out = self.feed.push_from(source, &rec);
            lines.extend(self.apply(host, out, Some(a)));
        }
        let ms = |d: Option<Instant>| {
            d.map_or(0, |i| {
                u64::try_from(i.elapsed().as_millis()).unwrap_or(u64::MAX)
            })
        };
        self.repairs.push(RepairStat {
            stale_ms: ms(self.stale_since),
            fetch_ms,
            total_ms: ms(self.repair_started),
            feeds: self.feeds.len(),
            patched,
            added,
            failed,
            buffered: n_buf,
        });
        self.stale_since = None;
        lines
    }

    /// (RP4, L-prep) The trade connection opened. On the simulated clock (or
    /// [`set_trade_fill_sync`](Self::set_trade_fill_sync)) the gaps are filled at once
    /// ([`fill_trade_gaps`](Self::fill_trade_gaps)). Live, the feeds stay stale
    /// (`repairing`) while the history is fetched on another thread
    /// ([`LiveIn::TradeHistory`]); the trades that arrive meanwhile wait in the feed.
    fn on_trade_opened(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        let was_stale = self.link == Link::Stale;
        if self.trade_fill_sync || self.trade_history.is_none() {
            // A feed whose bars come from trades is live once its trade endpoint is.
            if matches!(self.link, Link::Connecting | Link::Stale) {
                self.link = Link::Live;
                self.market_dirty = true;
            }
            let t0 = Instant::now();
            let failed_before = self.trade_repairs.1;
            let mut lines = self.fill_trade_gaps(host);
            if was_stale && self.marked_stale {
                self.marked_stale = false;
                lines.extend(host.live_refresh());
            }
            if self.trade_history.is_some() {
                let ms = u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX);
                self.trade_fills.push(TradeFillStat {
                    fetch_ms: ms,
                    total_ms: ms,
                    gaps: 0,
                    failed: usize::try_from(self.trade_repairs.1 - failed_before)
                        .unwrap_or(usize::MAX),
                    trades: 0,
                    after_loss: was_stale,
                });
            }
            return lines;
        }
        if matches!(self.link, Link::Connecting | Link::Stale) {
            self.link = Link::Repairing;
            self.repair_started = Some(Instant::now());
            self.market_dirty = true;
        }
        self.start_trade_fetch();
        Vec::new()
    }

    /// (RP4, L-prep) Starts fetching the open trade gaps' history on a thread.
    pub(super) fn start_trade_fetch(&mut self) {
        let Some(h) = self.trade_history.clone() else {
            return;
        };
        let now = self.exchange_now().max(self.clock);
        let (out, gaps) = self.feed.trade_gaps(now);
        self.finalized += out.finalized.len() as u64;
        self.trade_fetch_clock = self.trade_fetch_clock.max(out.clock_ms);
        self.trade_fetch = Some(Instant::now());
        self.trade_fetch_at_us = self.now();
        let generation = self.generation;
        let to_hub = self.to_hub.clone();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let results: Vec<(TradeGap, Result<Vec<Trade>, String>)> = gaps
                .into_iter()
                .map(|g| {
                    let r = h.trades(&g.symbol, g.from_ms, g.to_ms);
                    (g, r)
                })
                .collect();
            to_hub(LiveIn::TradeHistory {
                generation,
                results,
                fetch_ms: u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
        });
    }

    /// (RP4, L-prep) A trade history fetch finished: each gap is filled (history ∪ the
    /// trades held meanwhile, de-duplicated by trade number, in exchange order); the feeds
    /// are live again and the authority re-resolves them if they were marked stale. A
    /// failed gap stays open (its symbol's bars do not close; retried every 30 s). Results
    /// of an older stale period are dropped (the gaps stay for the next fetch).
    fn on_trade_history(
        &mut self,
        host: &mut ReplayHost,
        generation: u64,
        results: Vec<(TradeGap, Result<Vec<Trade>, String>)>,
        fetch_ms: u64,
    ) -> Vec<OutboundLine> {
        self.trade_fetch = None;
        if generation != self.generation
            || !matches!(self.link, Link::Repairing | Link::Live)
            || !self.trade_up
        {
            self.trade_fetch_clock = None;
            return Vec::new();
        }
        let after_loss = self.marked_stale;
        let mut out = FeedOut {
            clock_ms: self.trade_fetch_clock.take(),
            ..FeedOut::default()
        };
        let (gaps, mut failed, mut trades) = (results.len(), 0, 0);
        for (gap, r) in results {
            match r {
                Ok(t) => {
                    trades += t.len();
                    let o = self.feed.apply_trade_history(&gap, t);
                    out.events.extend(o.events);
                    out.finalized.extend(o.finalized);
                }
                Err(e) => {
                    failed += 1;
                    if self.conn_events.len() < 500 {
                        self.conn_events
                            .push(format!("trade history {}: {e}", gap.symbol));
                    }
                }
            }
        }
        self.trade_repairs.0 += 1;
        self.trade_repairs.1 += failed as u64;
        if self.link == Link::Repairing {
            self.link = Link::Live;
            self.market_dirty = true;
        }
        let mut lines = self.apply(host, out, None);
        if self.marked_stale {
            self.marked_stale = false;
            lines.extend(host.live_refresh());
        }
        let ms = |d: Option<Instant>| {
            d.map_or(0, |i| {
                u64::try_from(i.elapsed().as_millis()).unwrap_or(u64::MAX)
            })
        };
        self.trade_fills.push(TradeFillStat {
            fetch_ms,
            total_ms: ms(self.repair_started.take()),
            gaps,
            failed,
            trades,
            after_loss,
        });
        self.stale_since = None;
        lines
    }

    /// (RP4) The trade connection opened: the feed's clock goes to the exchange time and
    /// every open trade gap is filled from the trade history (synchronously; without a
    /// history the gaps stay open and the symbols stale).
    fn fill_trade_gaps(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        let Some(h) = self.trade_history.clone() else {
            return Vec::new();
        };
        let now = self.exchange_now().max(self.clock);
        let (out, errors) = self.feed.repair_trades(now, h.as_ref());
        self.trade_repairs.1 += errors.len() as u64;
        for (sym, e) in errors.iter().take(20) {
            self.conn_events.push(format!("trade history {sym}: {e}"));
        }
        self.trade_repairs.0 += 1;
        self.apply(host, out, None)
    }
}
