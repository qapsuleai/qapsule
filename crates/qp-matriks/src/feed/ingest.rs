//! Record intake of a [`LiveFeed`](super::LiveFeed): the clock, message dispatch and the
//! price-card path (rules 1–4; split out of `feed.rs` for size).

use super::*;

impl LiveFeed {
    pub(super) fn move_clock(&mut self, t: i64, grace: i64, out: &mut FeedOut) {
        if self.clock_ms.is_some_and(|c| c >= t) {
            return;
        }
        self.clock_ms = Some(t);
        out.clock_ms = Some(t);
        let until = t - grace;
        let gaps: BTreeSet<&str> = self.trades.as_ref().map_or_else(BTreeSet::new, |ts| {
            ts.syms
                .iter()
                .filter(|(_, s)| s.gap_from.is_some())
                .map(|(k, _)| k.as_str())
                .collect()
        });
        for (k, lb) in &mut self.feeds {
            if gaps.contains(k.symbol.as_str()) {
                continue;
            }
            if let Some(b) = lb.close_until(until) {
                out.finalized.push((k.clone(), b.t));
                self.stats.finalized += 1;
            }
        }
    }

    pub(super) fn on_message(&mut self, rec: &RawRecord, arrival: i64, out: &mut FeedOut) {
        let topic = parse_topic(&rec.topic);
        match topic.unwrap_mirror() {
            Topic::Symbol { symbol, .. } => {
                let symbol = symbol.clone();
                self.on_symbol(&symbol, &rec.payload, arrival, out);
            }
            Topic::Trade {
                symbol, special, ..
            } => {
                let (symbol, special) = (symbol.clone(), *special);
                self.on_trade(&symbol, special, &rec.payload, arrival, out);
            }
            Topic::Timestamp => {
                self.stats.time_messages += 1;
                match decode_time(&rec.payload) {
                    Ok(tick) if self.cfg.time == TimeSource::UpdateDate && tick.micros > 0 => {
                        let ms = tick.millis();
                        if ms <= arrival + MAX_LEAD_MS {
                            self.move_clock(ms, self.grace_ms, out);
                        } else {
                            self.stats.clock_lead_rejected += 1;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => self.stats.decode_errors += 1,
                }
            }
            _ => self.stats.other_messages += 1,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn on_symbol(&mut self, symbol: &str, payload: &[u8], arrival: i64, out: &mut FeedOut) {
        let Ok(upd) = self.book.apply(symbol, payload) else {
            self.stats.decode_errors += 1;
            return;
        };
        let Ok(m) = SymbolMessage::decode(payload) else {
            self.stats.decode_errors += 1;
            return;
        };
        self.stats.symbol_messages += 1;
        self.fresh_since_lost.insert(symbol.to_owned());
        let initial = upd.kind == UpdateKind::Initial;
        if initial {
            self.stats.initials += 1;
        }
        if let Some(code) = m.stock_status {
            if self.phases.get(symbol) != Some(&code) {
                self.phases.insert(symbol.to_owned(), code);
                out.phases.push((symbol.to_owned(), code));
            }
        }

        // Time (rule 3) and the clock candidate (rule 4).
        let mut clock_t = None;
        let t = match self.cfg.time {
            TimeSource::Arrival => Some((arrival, false)),
            TimeSource::UpdateDate => {
                let ud = m.update_date.as_deref().map(parse_update_date);
                if matches!(ud, Some(None)) {
                    self.stats.bad_update_date += 1;
                }
                if let Some(Some(ms)) = ud {
                    if ms <= arrival + MAX_LEAD_MS {
                        clock_t = Some(ms);
                    } else {
                        self.stats.clock_lead_rejected += 1;
                    }
                    if !initial {
                        self.anchor = Some((ms, arrival));
                    }
                    // An initial's `updateDate` is the symbol's last trade, possibly long
                    // ago: the snapshot describes now, so it is not placed before the clock.
                    let t = if initial {
                        self.clock_ms.map_or(ms, |c| c.max(ms))
                    } else {
                        ms
                    };
                    Some((t, false))
                } else if let Some((ud0, a0)) = self.anchor {
                    Some((ud0 + (arrival - a0), true))
                } else {
                    None
                }
            }
        };

        let (cum_v, cum_q) = match self.cfg.volume_field {
            VolumeField::DailyVolume => (m.daily_volume, m.daily_quantity),
            VolumeField::Volume => (m.volume, m.quantity),
        };
        if self.trades.is_some() {
            // Rule 9: bars come from trades; the card gives the phase and the clock only.
            if let Some(c) = clock_t {
                self.move_clock(c, self.grace_ms, out);
            }
            return;
        }
        let has_bar_fields = m.last.is_some()
            || m.open.is_some()
            || m.daily_high.is_some()
            || m.daily_low.is_some()
            || cum_v.is_some()
            || cum_q.is_some();
        if !(has_bar_fields || initial) {
            self.stats.no_bar_fields += 1;
        } else if let Some((t, derived)) = t {
            self.stats.ticks += 1;
            if derived {
                self.stats.time_derived += 1;
            } else {
                self.stats.time_direct += 1;
            }
            if let Some(c) = self.clock_ms.filter(|&c| c > t && !initial) {
                self.stats.behind_clock += 1;
                self.stats.max_behind_ms = self.stats.max_behind_ms.max(c - t);
            }
            let tick = CardTick {
                t_ms: t,
                initial,
                last: m.last,
                day_open: m.open,
                day_high: m.daily_high,
                day_low: m.daily_low,
                cum_volume: cum_v,
                cum_quantity: cum_q,
                phase: m.stock_status,
            };
            self.push_tick(symbol, &tick, out);
        } else {
            self.stats.time_unknown += 1;
        }

        if let Some(c) = clock_t {
            self.move_clock(c, self.grace_ms, out);
        }
    }

    fn push_tick(&mut self, symbol: &str, tick: &CardTick, out: &mut FeedOut) {
        for i in 0..self.intervals.len() {
            let key = self.key(symbol, &self.intervals[i]);
            if !self.feeds.contains_key(&key) {
                let Ok(mut lb) = self.builder(&key, 1) else {
                    continue;
                };
                if let Some(c) = self.clock_ms {
                    let _ = lb.close_until(c - self.grace_ms);
                }
                self.feeds.insert(key.clone(), lb);
                self.stats.feeds += 1;
            }
            let Some(lb) = self.feeds.get_mut(&key) else {
                continue;
            };
            let prev = lb.current().map(|(b, fin)| (b.t, fin));
            let events = lb.push(tick);
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
