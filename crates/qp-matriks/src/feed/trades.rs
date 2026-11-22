//! Rule 9 (trades): the trade-stream settings and the [`LiveFeed`](super::LiveFeed)
//! methods that watch, buffer, repair and apply trades (split out of `feed.rs` for size).

use super::*;

/// Which connection a record came from (rule 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedSource {
    /// The market endpoint (price cards, clock, notices).
    #[default]
    Market,
    /// The trade endpoint (`/trade`).
    Trade,
}

/// Trade input settings (rule 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeFeedConfig {
    /// Decoding (time unit).
    pub decode: TradeDecodeConfig,
    /// Whether special (reported) trades join the bars (default: the `1d` bar only, RP4 M27).
    pub special: SpecialTrades,
    /// How long trade numbers are remembered behind the newest trade, for de-duplication
    /// (ms; default 30 minutes).
    pub dedup_window_ms: i64,
}

impl Default for TradeFeedConfig {
    fn default() -> Self {
        Self {
            decode: TradeDecodeConfig::default(),
            special: SpecialTrades::default(),
            dedup_window_ms: 30 * 60_000,
        }
    }
}

/// A trade gap to fill from history: `[from_ms, to_ms]`, UTC ms, inclusive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TradeGap {
    /// Symbol.
    pub symbol: String,
    /// From (the time trades were known complete).
    pub from_ms: i64,
    /// To (the clock or the newest buffered trade, whichever is later).
    pub to_ms: i64,
}

impl LiveFeed {
    /// A feed whose bars come from the trade stream (rule 9). `cfg.method` is ignored
    /// (every builder uses [`BarMethod::Trades`]); `cfg.volume` applies.
    #[must_use]
    pub fn with_trades(
        cfg: LiveFeedConfig,
        cal: ExchangeCalendar,
        trades: TradeFeedConfig,
    ) -> Self {
        let mut f = Self::new(cfg, cal);
        f.trades = Some(TradeState {
            cfg: trades,
            live: false,
            sessions: 0,
            syms: BTreeMap::new(),
        });
        f
    }

    /// Whether bars come from trades.
    #[must_use]
    pub fn trades_enabled(&self) -> bool {
        self.trades.is_some()
    }

    /// Starts watching `symbol`'s trades: everything from `history_from_ms` (UTC ms; e.g.
    /// the close of the last complete REST bar, or the day's start) until the trade
    /// connection delivers is a gap to fill from history. A symbol already watched is
    /// unchanged. No effect without trades.
    pub fn watch_trades(&mut self, symbol: &str, history_from_ms: i64) {
        if let Some(ts) = self.trades.as_mut() {
            ts.syms
                .entry(symbol.to_owned())
                .or_insert_with(|| TradeSym {
                    gap_from: Some(history_from_ms),
                    covered_ms: Some(history_from_ms),
                    ..TradeSym::default()
                });
        }
    }

    /// The instant from which `symbol`'s trades are not in its seeded bars: the absorbing
    /// end ([`LiveBars::final_at`]) of the bar its builders hold (the seam bar after a
    /// [`seed`](Self::seed)); the latest over its intervals. `None`: no builder holds a bar.
    #[must_use]
    pub fn seam_end(&self, symbol: &str) -> Option<i64> {
        self.feeds
            .iter()
            .filter(|(k, _)| k.symbol == symbol)
            .filter_map(|(_, lb)| {
                let (b, _) = lb.current()?;
                Some(lb.final_at(lb.bar_of(b.t)?))
            })
            .max()
    }

    /// [`watch_trades`](Self::watch_trades) from the symbol's seam
    /// ([`seam_end`](Self::seam_end)), or from `fallback_ms` without a seeded bar.
    pub fn watch_trades_after_seam(&mut self, symbol: &str, fallback_ms: i64) {
        let from = self.seam_end(symbol).unwrap_or(fallback_ms);
        self.watch_trades(symbol, from);
    }

    /// (RP4, L-prep) Takes the trade state of `old` (the feed this one replaces after a
    /// REST bar repair): the trade connection state (open, session count) and every
    /// watched symbol. A symbol this feed was seeded for gets a gap from its seam
    /// ([`seam_end`](Self::seam_end)): the trades the old feed applied after the seam are
    /// not in the new feed's bars, so they are forgotten (de-duplication memory too) and
    /// come back from history; trades the old feed buffered stay buffered. No effect unless
    /// both feeds take trades.
    pub fn carry_trades_from(&mut self, old: &LiveFeed) {
        let (Some(mine), Some(theirs)) = (self.trades.as_ref(), old.trades.as_ref()) else {
            return;
        };
        let mut ts = theirs.clone();
        ts.cfg = mine.cfg;
        let seams: Vec<(String, i64)> = ts
            .syms
            .keys()
            .filter_map(|s| Some((s.clone(), self.seam_end(s)?)))
            .collect();
        for (sym, seam) in seams {
            if let Some(st) = ts.syms.get_mut(&sym) {
                st.gap_from = Some(seam);
                st.covered_ms = Some(seam);
                st.seen.clear();
                st.recent.clear();
                st.buffer.retain(|t| t.t_ms() >= seam);
            }
        }
        self.trades = Some(ts);
    }

    /// Whether the trade connection is open.
    #[must_use]
    pub fn trade_connected(&self) -> bool {
        self.trades.as_ref().is_some_and(|t| t.live)
    }

    /// Symbols whose trade data is stale: all watched symbols while the trade connection is
    /// down, else those with an open gap. Empty without trades.
    #[must_use]
    pub fn trade_stale_symbols(&self) -> Vec<String> {
        let Some(ts) = &self.trades else {
            return Vec::new();
        };
        ts.syms
            .iter()
            .filter(|(_, s)| !ts.live || s.gap_from.is_some())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// The gaps to fill now (only while the trade connection is open; rule 9). A gap ends
    /// at the later of the clock and the newest buffered trade: give the feed the exchange
    /// time first ([`advance_clock`](Self::advance_clock)) or a quiet symbol's gap ends
    /// where it starts. Trades between that end and the first live trade are covered only
    /// if the history already has them (de-duplication makes a second request harmless).
    #[must_use]
    pub fn pending_trade_repairs(&self) -> Vec<TradeGap> {
        let Some(ts) = self.trades.as_ref().filter(|t| t.live) else {
            return Vec::new();
        };
        ts.syms
            .iter()
            .filter_map(|(sym, s)| {
                let from = s.gap_from?;
                let newest = s.buffer.iter().map(Trade::t_ms).max();
                let to = [Some(from), self.clock_ms, newest]
                    .into_iter()
                    .flatten()
                    .max()
                    .unwrap_or(from);
                Some(TradeGap {
                    symbol: sym.clone(),
                    from_ms: from,
                    to_ms: to,
                })
            })
            .collect()
    }

    /// Fills `gap` with `history` (any order, duplicates allowed): history trades of the
    /// symbol at or after the gap start and the trades buffered during the gap are
    /// de-duplicated, sorted into exchange order and applied. The gap closes if the trade
    /// connection is open (else a new one starts where trades are now complete). Trades of
    /// another symbol are ignored.
    pub fn apply_trade_history(&mut self, gap: &TradeGap, history: Vec<Trade>) -> FeedOut {
        let mut out = FeedOut::default();
        let Some(ts) = self.trades.as_mut() else {
            return out;
        };
        let live = ts.live;
        let window = ts.cfg.dedup_window_ms;
        let Some(st) = ts.syms.get_mut(&gap.symbol) else {
            return out;
        };
        let from = st.gap_from.unwrap_or(gap.from_ms);
        let mut all: Vec<Trade> = Vec::with_capacity(history.len() + st.buffer.len());
        for t in history {
            if t.symbol != gap.symbol {
                continue;
            }
            if t.t_ms() < from {
                self.stats.trades.history_before_gap += 1;
                continue;
            }
            all.push(t);
        }
        let n_hist = all.len();
        let buffered = std::mem::take(&mut st.buffer);
        all.extend(buffered);
        all.sort_by(Trade::exchange_cmp);
        let mut to_apply = Vec::with_capacity(all.len());
        for t in all {
            let k = t.dedup_key();
            if st.seen(&k) {
                self.stats.trades.duplicates += 1;
                continue;
            }
            st.remember(t.t_ms(), k, window);
            to_apply.push(t);
        }
        let newest = to_apply.iter().map(Trade::t_ms).max();
        let covered = [st.covered_ms, Some(gap.to_ms), newest]
            .into_iter()
            .flatten()
            .max();
        st.covered_ms = covered;
        st.gap_from = if live { None } else { covered };
        self.stats.trades.repairs += 1;
        self.stats.trades.history_applied += n_hist as u64;
        let symbol = gap.symbol.clone();
        for t in &to_apply {
            self.push_trade_tick(&symbol, t, &mut out);
        }
        if live {
            if let Some(c) = self.clock_ms {
                let until = c - self.grace_ms;
                for (k, lb) in &mut self.feeds {
                    if k.symbol == symbol {
                        if let Some(b) = lb.close_until(until) {
                            out.finalized.push((k.clone(), b.t));
                            self.stats.finalized += 1;
                        }
                    }
                }
            }
        }
        out
    }

    /// Fills every pending gap from `history` ([`pending_trade_repairs`](Self::pending_trade_repairs)
    /// then [`apply_trade_history`](Self::apply_trade_history)). A failed request leaves its
    /// gap open (counted); its error text is returned with the symbol.
    pub fn repair_trades(
        &mut self,
        history: &dyn TradeHistory,
    ) -> (FeedOut, Vec<(String, String)>) {
        let mut out = FeedOut::default();
        let mut errors = Vec::new();
        for gap in self.pending_trade_repairs() {
            match history.trades(&gap.symbol, gap.from_ms, gap.to_ms) {
                Ok(trades) => {
                    let o = self.apply_trade_history(&gap, trades);
                    out.events.extend(o.events);
                    out.finalized.extend(o.finalized);
                }
                Err(e) => {
                    self.stats.trades.repair_errors += 1;
                    errors.push((gap.symbol.clone(), e));
                }
            }
        }
        (out, errors)
    }

    pub(super) fn on_trade_conn(&mut self, kind: RecordKind, out: &mut FeedOut) {
        let Some(ts) = self.trades.as_mut() else {
            self.stats.other_messages += 1;
            return;
        };
        match kind {
            RecordKind::ConnectionOpened => {
                ts.sessions += 1;
                ts.live = true;
                self.stats.trades.connections_opened += 1;
                out.trade_connection = Some(ConnChange::Opened {
                    session: ts.sessions,
                });
            }
            RecordKind::ConnectionLost => {
                ts.live = false;
                for s in ts.syms.values_mut() {
                    if s.gap_from.is_none() {
                        s.gap_from = s.covered_ms.or(self.clock_ms);
                    }
                }
                self.stats.trades.connections_lost += 1;
                out.trade_connection = Some(ConnChange::Lost);
            }
            RecordKind::SubscribeRejected => self.stats.subscribe_rejected += 1,
            RecordKind::Note => self.stats.notes += 1,
            RecordKind::Subscribed | RecordKind::Message => {}
        }
    }

    pub(super) fn on_trade(
        &mut self,
        symbol: &str,
        special_topic: bool,
        payload: &[u8],
        arrival: i64,
        out: &mut FeedOut,
    ) {
        let Some(ts) = self.trades.as_mut() else {
            self.stats.other_messages += 1;
            return;
        };
        let st = &mut self.stats.trades;
        st.messages += 1;
        let (mut trade, unit) = match decode_trade(payload, &ts.cfg.decode, arrival, special_topic)
        {
            Ok(x) => x,
            Err(TradeError::Decode(_)) => {
                st.decode_errors += 1;
                return;
            }
            Err(TradeError::NoPrice) => {
                st.no_price += 1;
                return;
            }
            Err(TradeError::NoTime) => {
                st.no_time += 1;
                return;
            }
        };
        match unit {
            TradeTimeUnit::NanosFromMidnight => st.unit_ns_from_midnight += 1,
            TradeTimeUnit::EpochMillis => st.unit_epoch_ms += 1,
            TradeTimeUnit::EpochMicros => st.unit_epoch_us += 1,
            TradeTimeUnit::EpochNanos => st.unit_epoch_ns += 1,
            TradeTimeUnit::Auto => {}
        }
        if trade.symbol != symbol {
            st.symbol_mismatch += 1;
            symbol.clone_into(&mut trade.symbol);
        }
        if trade.is_trade_ex {
            st.special += 1;
        }
        let t = trade.t_ms();
        if (t - arrival).abs() > 60_000 {
            st.far_from_arrival += 1;
        }
        let window = ts.cfg.dedup_window_ms;
        let live = ts.live;
        let sym = ts.syms.entry(symbol.to_owned()).or_insert_with(|| {
            // Not watched: the day so far is a gap (no initial state on this stream).
            let off = i64::from(ts.cfg.decode.utc_offset_min) * 60_000;
            let midnight = (t + off).div_euclid(86_400_000) * 86_400_000 - off;
            TradeSym {
                gap_from: Some(midnight),
                covered_ms: Some(midnight),
                ..TradeSym::default()
            }
        });
        if !live && sym.gap_from.is_none() {
            sym.gap_from = sym.covered_ms;
        }
        if sym.gap_from.is_some() {
            st.buffered += 1;
            sym.buffer.push(trade);
        } else {
            let k = trade.dedup_key();
            if sym.seen(&k) {
                st.duplicates += 1;
            } else {
                sym.remember(t, k, window);
                sym.covered_ms = Some(sym.covered_ms.map_or(t, |c| c.max(t)));
                self.push_trade_tick(symbol, &trade, out);
            }
        }
        if self.cfg.time == TimeSource::UpdateDate {
            if t <= arrival + MAX_LEAD_MS {
                self.move_clock(t, self.grace_ms, out);
            } else {
                self.stats.clock_lead_rejected += 1;
            }
        }
    }

    fn push_trade_tick(&mut self, symbol: &str, trade: &Trade, out: &mut FeedOut) {
        let tick = trade.tick();
        self.stats.ticks += 1;
        for i in 0..self.intervals.len() {
            let key = self.key(symbol, &self.intervals[i]);
            if !self.feeds.contains_key(&key) {
                let Ok(lb) = self.builder(&key, 1) else {
                    continue;
                };
                self.feeds.insert(key.clone(), lb);
                self.stats.feeds += 1;
            }
            let Some(lb) = self.feeds.get_mut(&key) else {
                continue;
            };
            let prev = lb.current().map(|(b, fin)| (b.t, fin));
            let events = lb.push_trade(&tick);
            if let Some((pt, false)) = prev {
                if events
                    .iter()
                    .any(|e| matches!(e.change, DataChange::Bar { .. }))
                {
                    out.finalized.push((key.clone(), pt));
                    self.stats.finalized += 1;
                }
            }
            out.events.extend(events);
        }
    }
}
