//! The feed engine over `qp_matriks::LiveFeed` ([`MatriksFeed`]; RG1: split out of `livehost.rs`).

use super::*;

// ---------------------------------------------------------------------------------------
// The feed: `qp_matriks::LiveFeed` (the one pipeline shared with raw-log replay).

/// [`FeedEngine`] over `qp_matriks::LiveFeed` on one interval.
pub struct MatriksFeed {
    cfg: qp_matriks::LiveFeedConfig,
    cal: ExchangeCalendar,
    interval: String,
    inner: qp_matriks::LiveFeed,
    /// (RP4) Bars from the trade stream (`LiveFeed::with_trades`) with these settings.
    trades: Option<qp_matriks::TradeFeedConfig>,
    /// Stats of the feeds replaced by a repair (records, events).
    pub retired: (u64, u64),
}

impl MatriksFeed {
    /// A feed building `interval` bars (other settings: `LiveFeedConfig` defaults —
    /// ticks, TL value, `dailyVolume`, `updateDate` time).
    #[must_use]
    pub fn new(cal: ExchangeCalendar, interval: &str) -> Self {
        let cfg = qp_matriks::LiveFeedConfig {
            intervals: vec![interval.to_owned()],
            ..Default::default()
        };
        Self {
            inner: qp_matriks::LiveFeed::new(cfg.clone(), cal.clone()),
            cfg,
            cal,
            interval: interval.to_owned(),
            trades: None,
            retired: (0, 0),
        }
    }

    /// (RP4) A feed building `interval` bars from the trade stream
    /// (`qp_matriks::LiveFeed::with_trades`; TL value volume, `updateDate` time for the
    /// card's clock). The price card, if any, gives the phases and the clock only.
    #[must_use]
    pub fn with_trades(
        cal: ExchangeCalendar,
        interval: &str,
        trades: qp_matriks::TradeFeedConfig,
    ) -> Self {
        let mut f = Self::new(cal, interval);
        f.inner = qp_matriks::LiveFeed::with_trades(f.cfg.clone(), f.cal.clone(), trades);
        f.trades = Some(trades);
        f
    }

    /// The underlying feed.
    #[cfg(test)]
    #[must_use]
    pub fn inner(&self) -> &qp_matriks::LiveFeed {
        &self.inner
    }

    fn fresh(&self) -> qp_matriks::LiveFeed {
        match self.trades {
            Some(t) => qp_matriks::LiveFeed::with_trades(self.cfg.clone(), self.cal.clone(), t),
            None => qp_matriks::LiveFeed::new(self.cfg.clone(), self.cal.clone()),
        }
    }
}

impl FeedEngine for MatriksFeed {
    fn seed_all(&mut self, seeds: &[Seed]) -> Result<(), String> {
        // (RP4, L-prep) A trade-built bar counts as started too (else the second seed is
        // refused and the repair never reseeds a trade feed).
        let started = self
            .inner
            .feeds()
            .any(|(_, lb)| lb.stats().ticks > 0 || lb.stats().trades > 0);
        if started {
            // After a repair: a new feed seeded at the repaired bars (`LiveFeed` seeds
            // only before a feed's first tick); each day's counter goes on from the
            // card's last value; the next message per symbol is its initial (as after any
            // reconnect).
            let fresh = self.fresh();
            let old = std::mem::replace(&mut self.inner, fresh);
            let st = old.stats();
            self.retired.0 += st.records;
            self.retired.1 += st.events;
            for (sym, last, seq, _) in seeds {
                let cum = old.book().state(sym).and_then(|c| c.daily_volume);
                self.inner
                    .seed_with_counter(sym, &self.interval, last.clone(), *seq, cum)?;
            }
            // (RP4, L-prep; T4's known limit) The trade state goes on: the watched symbols
            // (with a gap from the new seam) and the trade connection's state.
            self.inner.carry_trades_from(&old);
            return Ok(());
        }
        for (sym, last, seq, cum) in seeds {
            self.inner
                .seed_with_counter(sym, &self.interval, last.clone(), *seq, *cum)?;
        }
        Ok(())
    }

    fn push(&mut self, rec: &RawRecord) -> FeedOut {
        self.inner.push(rec)
    }

    fn push_from(&mut self, source: FeedSource, rec: &RawRecord) -> FeedOut {
        self.inner.push_from(source, rec)
    }

    fn watch_trades(&mut self, symbols: &[String], from_ms: i64) {
        for s in symbols {
            self.inner.watch_trades(s, from_ms);
        }
    }

    fn trades_enabled(&self) -> bool {
        self.inner.trades_enabled()
    }

    fn watch_trades_after_seams(&mut self, symbols: &[String], fallback_ms: i64) {
        for s in symbols {
            self.inner.watch_trades_after_seam(s, fallback_ms);
        }
    }

    fn trade_gaps(&mut self, now_ms: i64) -> (FeedOut, Vec<TradeGap>) {
        let out = self.inner.advance_clock(now_ms);
        (out, self.inner.pending_trade_repairs())
    }

    fn apply_trade_history(&mut self, gap: &TradeGap, trades: Vec<Trade>) -> FeedOut {
        self.inner.apply_trade_history(gap, trades)
    }

    fn repair_trades(
        &mut self,
        now_ms: i64,
        history: &dyn TradeHistory,
    ) -> (FeedOut, Vec<(String, String)>) {
        let mut out = self.inner.advance_clock(now_ms);
        let (o, errors) = self.inner.repair_trades(history);
        out.events.extend(o.events);
        out.finalized.extend(o.finalized);
        (out, errors)
    }

    fn advance_clock(&mut self, now_ms: i64) -> FeedOut {
        self.inner.advance_clock(now_ms)
    }

    fn stats_json(&self) -> Value {
        let s = self.inner.stats();
        let t = &s.trades;
        let trades = self.trades.map(|_| {
            json!({"messages": t.messages, "decode_errors": t.decode_errors,
                   "no_price": t.no_price, "no_time": t.no_time, "special": t.special,
                   "duplicates": t.duplicates, "buffered": t.buffered,
                   "far_from_arrival": t.far_from_arrival, "repairs": t.repairs,
                   "repair_errors": t.repair_errors, "history_applied": t.history_applied,
                   "connections_opened": t.connections_opened,
                   "connections_lost": t.connections_lost})
        });
        json!({
            "records": s.records + self.retired.0, "events": s.events + self.retired.1,
            "current": {
                "records": s.records, "symbol_messages": s.symbol_messages,
                "initials": s.initials, "decode_errors": s.decode_errors,
                "no_bar_fields": s.no_bar_fields, "time_direct": s.time_direct,
                "time_derived": s.time_derived, "time_unknown": s.time_unknown,
                "behind_clock": s.behind_clock, "max_behind_ms": s.max_behind_ms,
                "bad_update_date": s.bad_update_date,
                "clock_lead_rejected": s.clock_lead_rejected, "ticks": s.ticks,
                "connections_opened": s.connections_opened,
                "connections_lost": s.connections_lost, "resubscribes": s.resubscribes,
                "feeds": s.feeds, "finalized": s.finalized, "off_session": s.off_session,
                "late": s.late, "no_price": s.no_price, "invalid_fields": s.invalid_fields,
                "volume_regressions": s.volume_regressions,
                "volume_rebases": s.volume_rebases, "bars_opened": s.bars_opened,
                "events": s.events,
            },
            "trades": trades,
        })
    }
}
