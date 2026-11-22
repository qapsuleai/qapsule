//! [`LiveBars::push`] (price cards) and [`LiveBars::push_trade`] (trades): the tick
//! paths of the live bar builder (split out of `livebar.rs` for size).

use super::*;

impl LiveBars {
    /// Applies one price-card update; returns the data events it causes (possibly none).
    pub fn push(&mut self, tick: &CardTick) -> Vec<DataEvent> {
        let mut out = Vec::new();
        if self.cfg.method == BarMethod::Trades {
            self.stats.wrong_source += 1;
            if tick.phase.is_some() {
                self.stats.last_phase = tick.phase;
            }
            return out;
        }
        self.started = true;
        self.stats.ticks += 1;
        if tick.phase.is_some() {
            self.stats.last_phase = tick.phase;
        }
        let Some(span) = self.bar_of(tick.t_ms) else {
            self.stats.off_session += 1;
            return out;
        };
        let cur_span = self.cur.as_ref().map(|(_, s)| *s);
        let late = self.final_at(span) <= self.closed_through
            || cur_span.is_some_and(|c| span.open < c.open || (span == c && self.cur_final));
        if late {
            self.stats.late += 1;
            return out;
        }

        // Sanitized fields.
        let bad = &mut self.stats.invalid_fields;
        let last = price(tick.last, bad);
        let day_open = price(tick.day_open, bad);
        let day_high = price(tick.day_high, bad);
        let day_low = price(tick.day_low, bad);
        let cum = counter(
            match self.cfg.volume {
                VolumeUnit::Value => tick.cum_volume,
                VolumeUnit::Quantity => tick.cum_quantity,
            },
            bad,
        );

        // Day state: the counter and the day fields restart each trading day.
        let day = self.cal.local_day(tick.t_ms);
        if self.day != Some(day) {
            self.day = Some(day);
            self.cum_last = self.seam_cum.take();
            self.day_open = None;
            self.day_high = None;
            self.day_low = None;
        }
        self.seam_cum = None;

        let mut inc = 0.0;
        if let Some(x) = cum {
            match self.cum_last {
                None if tick.initial => self.stats.volume_rebases += 1,
                None => inc = x,
                Some(p) if x >= p => inc = x - p,
                Some(_) => self.stats.volume_regressions += 1,
            }
            self.cum_last = Some(x);
        }
        if last.is_some() {
            self.known_last = last;
        }
        if day_open.is_some() {
            self.day_open = day_open;
        }
        // A rise of the day high / fall of the day low seen in an update (not a snapshot:
        // after a reconnect the extreme may belong to an earlier bar).
        let high_rise = day_high.filter(|&h| !tick.initial && self.day_high.is_some_and(|p| h > p));
        let low_fall = day_low.filter(|&l| !tick.initial && self.day_low.is_some_and(|p| l < p));
        if day_high.is_some() {
            self.day_high = day_high;
        }
        if day_low.is_some() {
            self.day_low = day_low;
        }

        let card = self.interval == Interval::Day
            && matches!(self.cfg.method, BarMethod::CardDaily | BarMethod::Mixed);
        let reflect = !card && self.cfg.method == BarMethod::Mixed;
        let same = cur_span == Some(span);
        let traded = (!tick.initial && last.is_some()) || inc > 0.0;

        let next = if same {
            let Some((old, _)) = self.cur.as_ref() else {
                return out;
            };
            let mut b = old.clone();
            if card {
                self.card_update(&mut b, last);
            } else {
                if let Some(p) = last {
                    b.c = p;
                    b.h = b.h.max(p);
                    b.l = b.l.min(p);
                }
                b.v += inc;
            }
            Some(b)
        } else if traded || (card && self.cum_last.is_some_and(|c| c > 0.0)) {
            let Some(p) = last.or(self.known_last) else {
                self.stats.no_price += 1;
                return out;
            };
            let mut b = Bar {
                t: span.open,
                o: p,
                h: p,
                l: p,
                c: p,
                v: inc,
            };
            if card {
                b.o = self.day_open.unwrap_or(p);
                b.h = b.o.max(p);
                b.l = b.o.min(p);
                b.v = 0.0;
                self.card_update(&mut b, last);
            }
            Some(b)
        } else {
            None
        };
        let Some(mut b) = next else {
            return out;
        };
        if reflect {
            if let Some(h) = high_rise.filter(|&h| h > b.h) {
                b.h = h;
                self.stats.extremes_reflected += 1;
            }
            if let Some(l) = low_fall.filter(|&l| l < b.l) {
                b.l = l;
                self.stats.extremes_reflected += 1;
            }
        }

        if same {
            let Some((old, _)) = self.cur.as_mut() else {
                return out;
            };
            let fields = bar_patch_diff(old, &b);
            *old = b;
            if !fields.is_empty() {
                let t = span.open;
                self.emit(DataChange::Patch { t, fields }, &mut out);
            }
        } else {
            if let Some((prev, _)) = &self.cur {
                self.last_final_t = Some(prev.t);
            }
            self.cur = Some((b.clone(), span));
            self.cur_final = false;
            self.stats.bars_opened += 1;
            self.emit(DataChange::Bar { bar: b }, &mut out);
        }
        out
    }

    /// Applies one trade (rule 8); returns the data events it causes (possibly none).
    pub fn push_trade(&mut self, tr: &TradeTick) -> Vec<DataEvent> {
        let mut out = Vec::new();
        if self.cfg.method != BarMethod::Trades {
            self.stats.wrong_source += 1;
            return out;
        }
        self.started = true;
        self.stats.trades += 1;
        if tr.special && !self.cfg.special_trades.joins(self.interval) {
            self.stats.special_excluded += 1;
            return out;
        }
        let Some(span) = self.bar_of(tr.t_ms) else {
            self.stats.off_session += 1;
            return out;
        };
        let cur_span = self.cur.as_ref().map(|(_, s)| *s);
        let late = self.final_at(span) <= self.closed_through
            || cur_span.is_some_and(|c| span.open < c.open || (span == c && self.cur_final));
        if late {
            self.stats.late += 1;
            return out;
        }
        let bad = &mut self.stats.invalid_fields;
        let Some(p) = price(Some(tr.price), bad) else {
            return out;
        };
        let qty = match tr.quantity {
            None => {
                self.stats.no_quantity += 1;
                0.0
            }
            Some(q) => counter(Some(q), bad).unwrap_or(0.0),
        };
        let inc = match self.cfg.volume {
            VolumeUnit::Value => p * qty,
            VolumeUnit::Quantity => qty,
        };
        if tr.special && self.cfg.special_trades == SpecialTrades::DayOnly {
            // (RP4 M27) A special (reported) trade adds to the bar's volume only: REST's
            // daily bar holds its quantity but not its price (one daily low 3.6 % off when
            // its price was taken, 0 otherwise; `M27/RESULTS.md`).
            if cur_span == Some(span) {
                let Some((old, _)) = self.cur.as_mut() else {
                    return out;
                };
                let mut b = old.clone();
                b.v += inc;
                let fields = bar_patch_diff(old, &b);
                *old = b;
                self.cur_qty += qty;
                if !fields.is_empty() {
                    let t = span.open;
                    self.emit(DataChange::Patch { t, fields }, &mut out);
                }
            } else {
                let (v0, q0) = match self.pending_special {
                    Some((t, v, q)) if t == span.open => (v, q),
                    _ => (0.0, 0.0),
                };
                self.pending_special = Some((span.open, v0 + inc, q0 + qty));
            }
            return out;
        }
        self.known_last = Some(p);
        if cur_span == Some(span) {
            let Some((old, _)) = self.cur.as_mut() else {
                return out;
            };
            let mut b = old.clone();
            b.c = p;
            b.h = b.h.max(p);
            b.l = b.l.min(p);
            b.v += inc;
            let fields = bar_patch_diff(old, &b);
            *old = b;
            self.cur_qty += qty;
            if !fields.is_empty() {
                let t = span.open;
                self.emit(DataChange::Patch { t, fields }, &mut out);
            }
        } else {
            if let Some((prev, _)) = &self.cur {
                self.last_final_t = Some(prev.t);
            }
            let (pv, pq) = match self.pending_special.take() {
                Some((t, v, q)) if t == span.open => (v, q),
                _ => (0.0, 0.0),
            };
            let b = Bar {
                t: span.open,
                o: p,
                h: p,
                l: p,
                c: p,
                v: pv + inc,
            };
            self.cur = Some((b.clone(), span));
            self.cur_final = false;
            self.cur_qty = pq + qty;
            self.stats.bars_opened += 1;
            self.emit(DataChange::Bar { bar: b }, &mut out);
        }
        out
    }

    /// `CardDaily` state of day bar `b` from the merged card (monotone `h`, `l`, `v`).
    fn card_update(&self, b: &mut Bar, last: Option<f64>) {
        if let Some(o) = self.day_open {
            b.o = o;
        }
        if let Some(p) = last {
            b.c = p;
        }
        let (mut h, mut l) = (b.h.max(b.o).max(b.c), b.l.min(b.o).min(b.c));
        if let Some(x) = self.day_high {
            h = h.max(x);
        }
        if let Some(x) = self.day_low {
            l = l.min(x);
        }
        b.h = h;
        b.l = l;
        if let Some(c) = self.cum_last {
            b.v = b.v.max(c);
        }
    }
}
