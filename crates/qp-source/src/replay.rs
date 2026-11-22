//! Replay source ("Yeniden oynatma", R4/T2, K44; `docs/PROTOCOL.md` §21): historical bars
//! delivered like a live feed, as field-level [`DataEvent`]s on a virtual timeline.
//!
//! Pure and deterministic: no clock, no I/O, no randomness. The wall clock of a host only
//! decides *when* the next step is taken; what a step delivers is a function of the
//! position alone.
//!
//! - Every feed has its coarse bars (e.g. `1d`) and optionally finer bars over the same
//!   period (e.g. `1h`). The bars before the start are the initial resolution (the host's
//!   provider holds them, `seq` 0); every later bar is delivered as events:
//!   - **fine** (finer bars cover the day): a `bar` event at the bar's open
//!     `{o, h: o, l: o, c: o, v: 0}`, then after every fine bar a `patch` with the changed
//!     fields of the running aggregate (clamped into the final bar's range); the last
//!     fine step lands exactly on the final bar;
//!   - **ohlc** (no fine data; a labelled, deterministic intra-bar path): open, then the
//!     first extreme (low for a rising bar, high for a falling one), the second extreme,
//!     the close — spread over the session of [`ReplayOptions::calendar`] (real data:
//!     `XIST`; the synthetic market: its own calendar); the last step lands on the final bar;
//!   - **bar** (path off): one `bar` event with the whole bar.
//! - Feeds are merged on virtual time: a **step** is one distinct event time; its events
//!   are ordered by feed key (symbol, interval, feed) — ties never depend on input order.
//! - Position ([`ReplayCursor`]): virtual time, steps taken, events given per feed.
//!   [`Replay::step`], [`Replay::advance_to`] (every step at or before `t`),
//!   [`Replay::seek`] (going back rebuilds from the start and advances: deterministic).
//!
//! Synthetic market ([`synthetic_replay_feeds`]): `qp_source::synthetic_market_hourly`
//! (daily bars aggregated from its 60-minute bars, which are the fine bars) or
//! `synthetic_market` (daily only). Local real data ([`local_replay_feeds`]): the files of
//! `scripts/fetch-bars.sh` (K23; never committed).

use std::path::Path;

use qp_calendar::{builtin, ExchangeCalendar};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::{Bar, TimestampMs};

use crate::events::bar_patch_diff;
use crate::{load_local_bars, synthetic_market, synthetic_market_hourly, SourceError};

/// One feed of a replay: coarse bars and optional finer bars over the same period.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayFeed {
    /// The feed key (symbol, coarse interval, named feed).
    pub key: FeedKey,
    /// Coarse bars (strictly increasing `t`), history and replayed part.
    pub bars: Vec<Bar>,
    /// Finer bars (strictly increasing `t`); empty: none.
    pub fine: Vec<Bar>,
}

/// How the bars of a replayed day are built inside the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Intraday {
    /// From the finer bars.
    Fine,
    /// The labelled synthetic open → extreme → extreme → close path.
    Ohlc,
    /// One `bar` event with the whole bar.
    Bar,
}

impl Intraday {
    /// `fine`, `ohlc` or `bar`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Fine => "fine",
            Self::Ohlc => "ohlc",
            Self::Bar => "bar",
        }
    }
}

/// The calendar whose session spans the OHLC path uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplayCalendar {
    /// The built-in `XIST` calendar (real Borsa Istanbul holidays and half days).
    #[default]
    Xist,
    /// The synthetic market's frozen calendar ([`crate::synthetic_calendar`]).
    Synthetic,
}

/// Where a replay starts and stops and how a bar is built inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayOptions {
    /// Without `from_t`: the number of bars of each feed before the first replayed bar
    /// (the initial resolution).
    pub history: usize,
    /// First replayed bar of each feed: the first with `t >= from_t` (overrides
    /// `history`).
    pub from_t: Option<TimestampMs>,
    /// Last replayed bar: `t <= to_t`.
    pub to_t: Option<TimestampMs>,
    /// Build bars without fine data on the OHLC path (else one `bar` event).
    pub path: bool,
    /// Length of a fine bar in ms (a fine step happens at its end); `None`: one hour.
    pub fine_ms: Option<i64>,
    /// Calendar of the OHLC path's session spans (real data: `Xist`; the synthetic
    /// market: `Synthetic`).
    pub calendar: ReplayCalendar,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            history: 120,
            from_t: None,
            to_t: None,
            path: true,
            fine_ms: None,
            calendar: ReplayCalendar::Xist,
        }
    }
}

/// A position on the replay timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCursor {
    /// Virtual time: the time of the last step taken (the start time before any).
    pub t: TimestampMs,
    /// Steps taken.
    pub step: u64,
    /// Events given per feed (feed order of [`Replay::feeds`]).
    pub given: Vec<u64>,
}

/// One step: its virtual time and its events, in feed order.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayStep {
    /// Virtual time.
    pub t: TimestampMs,
    /// The events (each feed's `seq` continues from its initial resolution at 0).
    pub events: Vec<DataEvent>,
}

#[derive(Debug, Clone)]
struct Timeline {
    key: FeedKey,
    initial: Vec<Bar>,
    events: Vec<(TimestampMs, DataChange)>,
}

/// Replayed days by how they were built: `(fine, ohlc, bar)`.
pub type IntradayCounts = (u64, u64, u64);

/// The replay (see the module docs).
#[derive(Debug, Clone)]
pub struct Replay {
    feeds: Vec<Timeline>,
    times: Vec<TimestampMs>,
    start_t: TimestampMs,
    cursor: ReplayCursor,
    intraday: IntradayCounts,
}

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

fn xist() -> Option<ExchangeCalendar> {
    builtin("XIST").and_then(|c| ExchangeCalendar::new(&c).ok())
}

fn calendar_of(which: ReplayCalendar) -> Option<ExchangeCalendar> {
    match which {
        ReplayCalendar::Xist => xist(),
        ReplayCalendar::Synthetic => ExchangeCalendar::new(&crate::synthetic_calendar()).ok(),
    }
}

fn check_bars(what: &str, bars: &[Bar]) -> Result<(), SourceError> {
    qp_schema::validate_bars(bars).map_err(|e| SourceError::new(format!("{what}: {e}")))?;
    if bars.windows(2).any(|w| w[1].t <= w[0].t) {
        return Err(SourceError::new(format!(
            "{what}: bar times must be strictly increasing"
        )));
    }
    Ok(())
}

/// The state after the fine bars `fine[..=k]` of day bar `d`, clamped into `d`'s range
/// and kept monotone (`prev`: the state before).
fn fine_state(d: &Bar, prev: &Bar, f: &Bar, cum_v: f64) -> Bar {
    let c = f.c.clamp(d.l, d.h);
    let h = prev.h.max(f.h.min(d.h)).max(d.o.max(c)).min(d.h);
    let l = prev.l.min(f.l.max(d.l)).min(d.o.min(c)).max(d.l);
    Bar {
        t: d.t,
        o: d.o,
        h,
        l,
        c,
        v: cum_v.min(d.v).max(prev.v),
    }
}

/// The (time, state) steps of day bar `d` (the first is the open state, the last is `d`).
fn day_states(
    d: &Bar,
    next_t: TimestampMs,
    fine: &[Bar],
    opts: &ReplayOptions,
    cal: Option<&ExchangeCalendar>,
) -> (Intraday, Vec<(TimestampMs, Bar)>) {
    let open = Bar {
        t: d.t,
        o: d.o,
        h: d.o,
        l: d.o,
        c: d.o,
        v: 0.0,
    };
    let latest = next_t - 1;
    if !fine.is_empty() {
        let fine_ms = opts.fine_ms.unwrap_or(HOUR_MS).max(1);
        let mut out = vec![(d.t, open.clone())];
        let mut s = open;
        let mut cum = 0.0;
        let mut last_t = d.t;
        for (k, f) in fine.iter().enumerate() {
            cum += f.v;
            let at = (f.t + fine_ms).min(latest).max(last_t + 1);
            s = if k + 1 == fine.len() {
                d.clone()
            } else {
                fine_state(d, &s, f, cum)
            };
            out.push((at, s.clone()));
            last_t = at;
        }
        return (Intraday::Fine, out);
    }
    if !opts.path {
        return (Intraday::Bar, vec![(d.t, d.clone())]);
    }
    // Session span from the replay's calendar when `t` opens a session; else eight hours.
    let span = cal
        .and_then(|c| c.day_span(c.local_day(d.t)))
        .filter(|s| s.open == d.t && s.close > s.open)
        .map_or(8 * HOUR_MS, |s| s.close - s.open);
    let span = span.min((latest - d.t).max(3));
    let (first, second) = if d.c >= d.o { (d.l, d.h) } else { (d.h, d.l) };
    let s1 = Bar {
        t: d.t,
        o: d.o,
        h: d.o.max(first),
        l: d.o.min(first),
        c: first,
        v: d.v / 3.0,
    };
    let s2 = Bar {
        t: d.t,
        o: d.o,
        h: s1.h.max(second),
        l: s1.l.min(second),
        c: second,
        v: d.v * 2.0 / 3.0,
    };
    let at = |j: i64| d.t + span * j / 3;
    (
        Intraday::Ohlc,
        vec![
            (d.t, open),
            (at(1), s1),
            (at(2), s2),
            (at(3).max(at(2) + 1), d.clone()),
        ],
    )
}

impl Replay {
    #[allow(clippy::too_many_lines)]
    /// Builds the replay of `feeds` (any order; kept sorted by key).
    ///
    /// # Errors
    /// No feed, a duplicate feed key, bars that break the bar rules or are not strictly
    /// increasing, or a feed with no bar before the first replayed bar.
    pub fn new(mut feeds: Vec<ReplayFeed>, opts: &ReplayOptions) -> Result<Self, SourceError> {
        if feeds.is_empty() {
            return Err(SourceError::new("a replay needs at least one feed"));
        }
        feeds.sort_by(|a, b| a.key.cmp(&b.key));
        if feeds.windows(2).any(|w| w[0].key == w[1].key) {
            return Err(SourceError::new("duplicate feed in a replay"));
        }
        let cal = calendar_of(opts.calendar);
        let mut timelines = Vec::with_capacity(feeds.len());
        let mut intraday = (0, 0, 0);
        let mut start_t = i64::MIN;
        for f in feeds {
            let what = format!("feed {}/{}", f.key.symbol, f.key.interval);
            check_bars(&what, &f.bars)?;
            if !f.fine.is_empty() {
                check_bars(&format!("{what} (fine)"), &f.fine)?;
            }
            let first = match opts.from_t {
                Some(t) => f.bars.partition_point(|b| b.t < t),
                None => opts.history.min(f.bars.len()),
            };
            if first == 0 {
                return Err(SourceError::new(format!(
                    "{what}: no bar before the first replayed bar (the initial resolution needs one)"
                )));
            }
            let end = match opts.to_t {
                Some(t) => f.bars.partition_point(|b| b.t <= t),
                None => f.bars.len(),
            }
            .max(first);
            let mut events = Vec::new();
            for i in first..end {
                let d = &f.bars[i];
                let next_t = f.bars.get(i + 1).map_or(d.t + DAY_MS, |b| b.t);
                let lo = f.fine.partition_point(|b| b.t < d.t);
                let hi = f.fine.partition_point(|b| b.t < next_t);
                let (kind, states) = day_states(d, next_t, &f.fine[lo..hi], opts, cal.as_ref());
                match kind {
                    Intraday::Fine => intraday.0 += 1,
                    Intraday::Ohlc => intraday.1 += 1,
                    Intraday::Bar => intraday.2 += 1,
                }
                let mut prev: Option<Bar> = None;
                for (at, s) in states {
                    let change = match &prev {
                        None => DataChange::Bar { bar: s.clone() },
                        Some(p) => {
                            let fields = bar_patch_diff(p, &s);
                            if fields.is_empty() {
                                continue;
                            }
                            DataChange::Patch { t: d.t, fields }
                        }
                    };
                    events.push((at, change));
                    prev = Some(s);
                }
            }
            let initial = f.bars[..first].to_vec();
            start_t = start_t.max(initial.last().map_or(i64::MIN, |b| b.t));
            timelines.push(Timeline {
                key: f.key,
                initial,
                events,
            });
        }
        let mut times: Vec<TimestampMs> = timelines
            .iter()
            .flat_map(|tl| tl.events.iter().map(|e| e.0))
            .collect();
        times.sort_unstable();
        times.dedup();
        // Before the first step the virtual time is the latest initial bar (or just
        // before the first step if a feed's history reaches past it).
        if let Some(&t0) = times.first() {
            start_t = start_t.min(t0 - 1);
        }
        let n = timelines.len();
        Ok(Self {
            feeds: timelines,
            times,
            start_t,
            cursor: ReplayCursor {
                t: start_t,
                step: 0,
                given: vec![0; n],
            },
            intraday,
        })
    }

    /// The feeds in replay order (by key) with their initial bars.
    pub fn feeds(&self) -> impl Iterator<Item = (&FeedKey, &[Bar])> {
        self.feeds.iter().map(|f| (&f.key, f.initial.as_slice()))
    }

    /// Number of feeds.
    #[must_use]
    pub fn feed_count(&self) -> usize {
        self.feeds.len()
    }

    /// The position.
    #[must_use]
    pub fn cursor(&self) -> &ReplayCursor {
        &self.cursor
    }

    /// Virtual time before the first step.
    #[must_use]
    pub fn start_t(&self) -> TimestampMs {
        self.start_t
    }

    /// Time of the first step (`None`: nothing to replay).
    #[must_use]
    pub fn first_t(&self) -> Option<TimestampMs> {
        self.times.first().copied()
    }

    /// Time of the last step.
    #[must_use]
    pub fn last_t(&self) -> Option<TimestampMs> {
        self.times.last().copied()
    }

    /// Number of steps of the whole replay.
    #[must_use]
    pub fn total_steps(&self) -> u64 {
        self.times.len() as u64
    }

    /// Number of events of the whole replay.
    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.feeds.iter().map(|f| f.events.len() as u64).sum()
    }

    /// Events given so far.
    #[must_use]
    pub fn given_events(&self) -> u64 {
        self.cursor.given.iter().sum()
    }

    /// Replayed days by how they were built: `(fine, ohlc, bar)`.
    #[must_use]
    pub fn intraday(&self) -> IntradayCounts {
        self.intraday
    }

    /// True when every step was taken.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.cursor.step as usize >= self.times.len()
    }

    /// Time of the next step.
    #[must_use]
    pub fn peek_t(&self) -> Option<TimestampMs> {
        self.times.get(self.cursor.step as usize).copied()
    }

    /// Number of steps whose time is at or before `t` (the step count [`Replay::seek`]
    /// to `t` reaches).
    #[must_use]
    pub fn steps_until(&self, t: TimestampMs) -> u64 {
        self.times.partition_point(|&x| x <= t) as u64
    }

    /// Takes the next step (`None` at the end).
    pub fn step(&mut self) -> Option<ReplayStep> {
        let t = self.peek_t()?;
        let mut events = Vec::new();
        for (fi, f) in self.feeds.iter().enumerate() {
            let given = &mut self.cursor.given[fi];
            while let Some((at, change)) = f.events.get(*given as usize) {
                if *at != t {
                    break;
                }
                *given += 1;
                events.push(DataEvent {
                    feed: f.key.clone(),
                    seq: *given,
                    change: change.clone(),
                });
            }
        }
        self.cursor.step += 1;
        self.cursor.t = t;
        Some(ReplayStep { t, events })
    }

    /// Takes every step whose time is at or before `t`.
    pub fn advance_to(&mut self, t: TimestampMs) -> Vec<ReplayStep> {
        let mut out = Vec::new();
        while self.peek_t().is_some_and(|x| x <= t) {
            if let Some(s) = self.step() {
                out.push(s);
            }
        }
        out
    }

    /// Back to the start (no step taken).
    pub fn reset(&mut self) {
        self.cursor = ReplayCursor {
            t: self.start_t,
            step: 0,
            given: vec![0; self.feeds.len()],
        };
    }

    /// Goes to virtual time `t`: forward, the steps up to `t`; backward (`t` before the
    /// virtual time), back to the start, then forward to `t`. Returns whether it went
    /// back and the steps taken (all of them since the start when it went back).
    pub fn seek(&mut self, t: TimestampMs) -> (bool, Vec<ReplayStep>) {
        let back = t < self.cursor.t;
        if back {
            self.reset();
        }
        (back, self.advance_to(t))
    }
}

/// Start of local day `date` (`YYYY-MM-DD`) in the XIST time zone, UTC epoch ms.
///
/// # Errors
/// Not a valid date.
pub fn date_start(date: &str) -> Result<TimestampMs, SourceError> {
    let day = qp_calendar::parse_date(date).map_err(|e| SourceError::new(e.to_string()))?;
    let midnight = day * DAY_MS;
    let off = xist().map_or(0, |c| {
        i64::from(c.zone().offset_at(midnight.div_euclid(1000)))
    });
    Ok(midnight - off * 1000)
}

/// `n` synthetic instruments (`SYN01`, …) of `days` daily bars on feed `feed`: with `fine`,
/// `synthetic_market_hourly` (daily bars aggregated from its 60-minute bars, which become
/// the fine bars), else `synthetic_market` (daily only). Interval `1d`.
#[must_use]
pub fn synthetic_replay_feeds(
    n: usize,
    days: usize,
    seed: u64,
    fine: bool,
    feed: Option<&str>,
) -> Vec<ReplayFeed> {
    let key = |symbol: &str| FeedKey {
        symbol: symbol.to_owned(),
        interval: "1d".to_owned(),
        feed: feed.map(str::to_owned),
    };
    if !fine {
        return synthetic_market(n, days, seed)
            .into_iter()
            .map(|i| ReplayFeed {
                key: key(&i.symbol),
                bars: i.bars,
                fine: Vec::new(),
            })
            .collect();
    }
    let cal = calendar_of(ReplayCalendar::Synthetic);
    synthetic_market_hourly(n, days, seed)
        .into_iter()
        .map(|i| ReplayFeed {
            key: key(&i.symbol),
            bars: aggregate_days(&i.bars, cal.as_ref()),
            fine: i.bars,
        })
        .collect()
}

/// Daily bars from intraday bars: grouped by XIST local day (UTC day without the
/// calendar), `t` the first bar's, `o` first, `h` max, `l` min, `c` last, `v` sum.
#[must_use]
pub fn aggregate_days(bars: &[Bar], cal: Option<&ExchangeCalendar>) -> Vec<Bar> {
    let day = |t: i64| cal.map_or(t.div_euclid(DAY_MS), |c| c.local_day(t));
    let mut out: Vec<Bar> = Vec::new();
    let mut cur_day = None;
    for b in bars {
        let d = day(b.t);
        match out.last_mut() {
            Some(a) if cur_day == Some(d) => {
                a.h = a.h.max(b.h);
                a.l = a.l.min(b.l);
                a.c = b.c;
                a.v += b.v;
            }
            _ => {
                out.push(b.clone());
                cur_day = Some(d);
            }
        }
    }
    out
}

/// Symbols listed in a `symbols.txt` file (one per line, `#` comments), in file order.
/// Missing file: empty.
#[must_use]
pub fn read_symbols(path: &Path) -> Vec<String> {
    let Ok(text) = crate::read(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Local real data (`fixtures/local/bars`, K23) as replay feeds: the adjusted `period`
/// bars (e.g. `1day`) of up to `max` symbols — in the order of `symbols.txt` next to the
/// bars directory when it exists, else sorted — with the adjusted `fine_period` bars
/// (e.g. `1hour`) as fine bars when given and present. Feed interval `interval`. Missing
/// directory or files: empty (the caller falls back to the synthetic market).
#[must_use]
pub fn local_replay_feeds(
    dir: &Path,
    period: &str,
    interval: &str,
    fine_period: Option<&str>,
    max: usize,
    feed: Option<&str>,
) -> Vec<ReplayFeed> {
    let coarse = load_local_bars(dir, period);
    if coarse.is_empty() {
        return Vec::new();
    }
    let listed = dir.parent().map(|p| read_symbols(&p.join("symbols.txt")));
    let mut order: Vec<String> = listed.unwrap_or_default();
    order.retain(|s| coarse.iter().any(|(c, _)| c == s));
    for (s, _) in &coarse {
        if !order.contains(s) {
            order.push(s.clone());
        }
    }
    order.truncate(max);
    let mut fine = match fine_period {
        Some(p) => load_local_bars(dir, p),
        None => Vec::new(),
    };
    order
        .into_iter()
        .filter_map(|s| {
            let bars = coarse.iter().find(|(c, _)| *c == s)?.1.clone();
            let f = fine
                .iter()
                .position(|(c, _)| *c == s)
                .map(|i| fine.swap_remove(i).1)
                .unwrap_or_default();
            Some(ReplayFeed {
                key: FeedKey {
                    symbol: s,
                    interval: interval.to_owned(),
                    feed: feed.map(str::to_owned),
                },
                bars,
                fine: f,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FeedStore;

    fn run_all(r: &mut Replay) -> Vec<ReplayStep> {
        let mut v = Vec::new();
        while let Some(s) = r.step() {
            v.push(s);
        }
        v
    }

    /// Applying every event to a store holding the initial bars yields the source bars
    /// exactly, for the fine path, the OHLC path and whole bars.
    #[test]
    fn events_rebuild_the_bars_bit_for_bit() {
        for (fine, path) in [(true, true), (false, true), (false, false)] {
            let feeds = synthetic_replay_feeds(3, 40, 7, fine, None);
            let want: Vec<Vec<Bar>> = feeds.iter().map(|f| f.bars.clone()).collect();
            let opts = ReplayOptions {
                history: 10,
                path,
                ..ReplayOptions::default()
            };
            let mut r = Replay::new(feeds, &opts).unwrap();
            let mut stores: Vec<FeedStore> = r
                .feeds()
                .map(|(k, b)| FeedStore::with_bars(k.clone(), b.to_vec(), 0).unwrap())
                .collect();
            let steps = run_all(&mut r);
            let mut last = r.start_t();
            for s in &steps {
                assert!(s.t > last, "step times strictly increase");
                last = s.t;
                for ev in &s.events {
                    let i = stores.iter().position(|x| x.feed() == &ev.feed).unwrap();
                    stores[i].apply(ev).unwrap();
                }
            }
            for (st, w) in stores.iter().zip(&want) {
                assert_eq!(st.bars(), w.as_slice());
            }
            let (f, o, b) = r.intraday();
            assert_eq!(f + o + b, 3 * 30);
            match (fine, path) {
                (true, _) => assert_eq!(f, 90),
                (false, true) => assert_eq!(o, 90),
                (false, false) => assert_eq!(b, 90),
            }
            assert!(r.is_done());
            assert_eq!(r.given_events(), r.total_events());
        }
    }

    /// Events of one step are in feed key order; seeking back rebuilds the same stream.
    #[test]
    fn merge_order_and_seek_are_deterministic() {
        let mut feeds = synthetic_replay_feeds(4, 30, 3, true, None);
        feeds.reverse();
        let mut r = Replay::new(feeds, &ReplayOptions::default_with_history(5)).unwrap();
        let all = run_all(&mut r);
        for s in &all {
            assert!(s.events.windows(2).all(|w| w[0].feed <= w[1].feed));
        }
        let mid = all[all.len() / 2].t;
        let mut r2 = r.clone();
        r2.reset();
        let fwd = r2.advance_to(mid);
        let (back, again) = r2.seek(mid - 1);
        assert!(back);
        assert_eq!(again, fwd[..fwd.len() - 1]);
        let (back, rest) = r2.seek(i64::MAX);
        assert!(!back);
        assert_eq!(fwd.len() - 1 + rest.len(), all.len());
        assert_eq!(r2.cursor(), r.cursor());
    }

    impl ReplayOptions {
        fn default_with_history(history: usize) -> Self {
            Self {
                history,
                ..Self::default()
            }
        }
    }

    #[test]
    fn rejects_a_replay_without_history() {
        let feeds = synthetic_replay_feeds(1, 10, 1, false, None);
        let e = Replay::new(feeds, &ReplayOptions::default_with_history(0)).unwrap_err();
        assert!(e.to_string().contains("no bar before"));
    }

    #[test]
    fn date_start_is_istanbul_midnight() {
        // 2024-03-05 00:00 Europe/Istanbul (UTC+3) = 2024-03-04 21:00 UTC.
        assert_eq!(date_start("2024-03-05").unwrap(), 1_709_586_000_000);
        assert!(date_start("2024-13-01").is_err());
    }

    #[test]
    fn local_data_absent_is_empty() {
        let d = std::env::temp_dir().join("qp-replay-no-such-dir");
        assert!(local_replay_feeds(&d, "1day", "1d", Some("1hour"), 30, None).is_empty());
    }
}
