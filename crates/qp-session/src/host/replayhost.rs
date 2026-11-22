//! Replay host (R4/T2, K44; `docs/PROTOCOL.md` §21): the host that runs an authority
//! ([`Session`]) on a replay source ([`qp_source::Replay`]).
//!
//! - It serves the authority's data plane from the replay's initial bars (`init.data`),
//!   and at every step gives the authority the virtual time ([`Session::host_clock`]) and
//!   the step's data events ([`Session::host_data_event`]) — the authority has no clock
//!   of its own (§11) and sees a live feed.
//! - It answers the `replay` protocol message (speed, pause, resume, step, seek, status)
//!   and hands every other message to the authority unchanged. A shell and an agent
//!   drive the same session with the same messages.
//! - The wall clock only decides *when* steps are taken ([`ReplayHost::pump`]); a step's
//!   content is a function of the position. Same data + the same controls at the same
//!   positions ([`ReplayHost::controls`]) → byte-identical message stream.
//! - Going back rebuilds the authority from its init and advances it silently: a new
//!   authority (`epoch + 1`, §8.4), every connection closed.

mod build;
mod control;
mod recorded;

use recorded::*;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::data::{DataEvent, Window};
use qp_schema::protocol::{
    ClientMsg, DocumentRef, ErrorCode, InitData, InitFeed, OutboundLine, ProtocolError,
    QueryResult, ReplayControl, ReplayDays, ReplayInit, ReplaySourceKind, ReplayState, Role,
    ServerMsg, SessionInit,
};
use qp_schema::{Instrument, Series, SeriesSource};
use qp_source::{Replay, ReplayCalendar, ReplayFeed, ReplayOptions, ReplayStep};

use crate::error::E;
use crate::session::{confine, permissions, read_json, Session};

/// Default number of instruments.
pub const DEFAULT_INSTRUMENTS: u32 = 30;
/// Default synthetic trading days.
pub const DEFAULT_DAYS: u32 = 260;
/// Default history (bars before the first replayed bar).
pub const DEFAULT_HISTORY: u32 = 120;
/// Default speed, steps per wall-clock second.
pub const DEFAULT_SPEED: f64 = 4.0;
/// Largest `step.n`.
pub const MAX_STEP_N: u32 = 100_000;
/// Largest speed.
pub const MAX_SPEED: f64 = 1000.0;
/// Most steps one [`ReplayHost::pump`] takes (a host that stalled catches up gradually).
pub const MAX_STEPS_PER_PUMP: u64 = 2_000;

/// Length of an interval in ms (`Nm`, `Nh`, `1d`).
#[must_use]
pub fn interval_ms(s: &str) -> Option<i64> {
    let split = s.find(|c: char| !c.is_ascii_digit())?;
    let (n, unit) = s.split_at(split);
    let n: i64 = n.parse().ok()?;
    match unit {
        "m" => Some(n * 60_000),
        "h" => Some(n * 3_600_000),
        "d" => Some(n * 86_400_000),
        _ => None,
    }
    .filter(|&ms| ms > 0)
}

/// The `fetch-bars.sh` period of an interval (`1d` → `1day`, `1h`/`60m` → `1hour`,
/// `5m` → `5min`, `1m` → `1min`).
#[must_use]
pub fn local_period(interval: &str) -> Option<&'static str> {
    match interval {
        "1d" => Some("1day"),
        "1h" | "60m" => Some("1hour"),
        "5m" => Some("5min"),
        "1m" => Some("1min"),
        _ => None,
    }
}

/// What the host built the replay from.
#[derive(Debug, Clone, PartialEq)]
struct SourceInfo {
    kind: ReplaySourceKind,
    note: Option<String>,
    interval: String,
    fine: Option<String>,
}

/// The replay host (see the module docs).
pub struct ReplayHost {
    /// The expanded init (no `replay`); a backward seek starts a new authority from it.
    init: SessionInit,
    root: PathBuf,
    session: Session,
    replay: Replay,
    info: SourceInfo,
    speed: f64,
    paused: bool,
    /// Wall-clock anchor of the running replay: (wall ms, step at that time).
    anchor: Option<(u64, u64)>,
    resets: u64,
    controls: Vec<(u64, ReplayControl)>,
    notices: Vec<String>,
    /// (R4/T5) Record the data events applied since the start (or the last rebuild), for
    /// a host that also feeds replicas ([`ReplayHost::recorded_events`]).
    record: bool,
    recorded: Recorded,
    /// (R5/M19) Per-principal feed rules the host set (kept over a backward seek).
    principal_rules: std::collections::BTreeMap<String, Option<crate::CoinFlipRule>>,
    /// (R6/T7) A live host ([`ReplayHost::live`]): the data provider the authority
    /// resolves from, shared with the host (initial bars + every recorded event).
    live: Option<std::sync::Arc<std::sync::Mutex<qp_source::MemoryProvider>>>,
    /// (R6/T3) The market status the host gave (kept over a backward seek).
    market: Option<qp_schema::market::MarketStatus>,
}

/// (R6/T7) The authority's provider of a live host: the host's own store (initial bars +
/// every data event it recorded), so that a re-resolution after a gap (§18.4) sees the
/// repaired feed. The host never gives it the deliberate gap that marks a feed stale.
struct SharedProvider(std::sync::Arc<std::sync::Mutex<qp_source::MemoryProvider>>);

impl qp_source::SourceProvider for SharedProvider {
    fn has_row(&self, principal: &str) -> Option<bool> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .has_row(principal)
    }

    fn entitled(&self, principal: &str, feed: &qp_schema::data::FeedKey) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entitled(principal, feed)
    }

    fn resolve(
        &self,
        principal: &str,
        binding: &qp_schema::data::DataBinding,
    ) -> qp_source::Resolution {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .resolve(principal, binding)
    }
}

impl std::fmt::Debug for ReplayHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayHost")
            .field("step", &self.replay.cursor().step)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

impl ReplayHost {
    /// (R6/T7) True for a live host ([`ReplayHost::live`]).
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// (R6/T7) Sets the note the replay state carries (a live host: "live, delayed…").
    pub fn set_note(&mut self, note: Option<String>) {
        self.info.note = note;
    }

    /// (R6/T7) A live data event: recorded, given to the host's store and to the
    /// authority ([`Session::host_data_event`]; gap / stale notices go to
    /// [`ReplayHost::notices`]). Viewers are not told until [`ReplayHost::live_flush`].
    ///
    /// # Errors
    /// Not a live host, or the store rejected the event (nothing recorded).
    pub fn live_event(&mut self, ev: &DataEvent) -> Result<Vec<OutboundLine>, String> {
        let Some(p) = &self.live else {
            return Err("not a live host".to_owned());
        };
        p.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .apply(ev)
            .map_err(|e| {
                format!(
                    "event {}/{} seq {}: {e}",
                    ev.feed.symbol, ev.feed.interval, ev.seq
                )
            })?;
        self.recorded.push(ev);
        match self.session.host_data_event(ev) {
            Ok(o) => {
                for n in &o.notices {
                    self.notices.push(n.frame().to_string());
                }
                Ok(o.lines)
            }
            Err(e) => {
                self.notices.push(format!(
                    "event {}/{} seq {}: {e}",
                    ev.feed.symbol, ev.feed.interval, ev.seq
                ));
                Ok(Vec::new())
            }
        }
    }

    /// (R6/T7) The viewers' notifications after a batch of live events.
    pub fn live_flush(&mut self) -> Vec<OutboundLine> {
        self.session.viewer_flush()
    }

    /// (R6/T7) The host's current bars and last `seq` of a live feed.
    #[must_use]
    pub fn live_feed(&self, key: &qp_schema::data::FeedKey) -> Option<(Vec<qp_schema::Bar>, u64)> {
        let p = self
            .live
            .as_ref()?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let f = p.feed(key)?;
        Some((f.bars().to_vec(), f.seq()))
    }

    /// (R6/T7, M23) Marks every live feed `stale` in the authority (the connection was
    /// lost): the §18.4 gap rule, used on purpose — each feed gets an event one `seq`
    /// past the next, which the authority does not apply; its bindings become `stale`
    /// and give no values (no capsule is judged on them) until
    /// [`ReplayHost::live_refresh`]. The event is not recorded and never reaches the
    /// host's store or a device: the number it skips is the next real event's. Returns
    /// the notifications (binding state changes).
    pub fn live_mark_stale(&mut self) -> Vec<OutboundLine> {
        let Some(p) = &self.live else {
            return Vec::new();
        };
        let marks: Vec<DataEvent> = {
            let p = p.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            self.init
                .data
                .iter()
                .flat_map(|d| d.feeds.iter())
                .filter_map(|f| {
                    let store = p.feed(&f.feed)?;
                    let last = store.bars().last()?;
                    Some(DataEvent {
                        feed: f.feed.clone(),
                        seq: store.seq() + 2,
                        change: qp_schema::data::DataChange::Patch {
                            t: last.t,
                            fields: qp_schema::data::BarPatch {
                                c: Some(last.c),
                                ..Default::default()
                            },
                        },
                    })
                })
                .collect()
        };
        let mut lines = Vec::new();
        for ev in &marks {
            match self.session.host_data_event(ev) {
                Ok(o) => lines.extend(o.lines),
                Err(e) => self.notices.push(format!("stale mark: {e}")),
            }
        }
        lines.extend(self.session.viewer_flush());
        lines
    }

    /// (R6/T7, M23) Resolves the stale feeds again from the host's store (after the
    /// repair events went in with [`ReplayHost::live_event`]); returns the
    /// notifications.
    pub fn live_refresh(&mut self) -> Vec<OutboundLine> {
        let mut lines = self.session.refresh_bindings();
        lines.extend(self.session.viewer_flush());
        lines
    }

    /// (R6/T7) How many bound series are not `ready` in the authority.
    #[must_use]
    pub fn live_not_ready(&self) -> usize {
        self.session
            .document()
            .series
            .iter()
            .filter(|s| s.binding().is_some())
            .filter(|s| {
                self.session
                    .binding_state(&s.id)
                    .is_some_and(|b| b != qp_schema::data::BindingState::Ready)
            })
            .count()
    }

    /// (R6/T3, §24) The host's market status input
    /// ([`Session::set_market_status`]): the `viewer` events it causes. Kept over a
    /// backward seek (the new authority gets it again).
    pub fn set_market_status(
        &mut self,
        status: qp_schema::market::MarketStatus,
    ) -> Vec<OutboundLine> {
        self.market = Some(status.clone());
        self.session.set_market_status(status)
    }

    /// (R5/M19, K53) Sets `principal`'s feed rule over the init's
    /// ([`Session::set_principal_coin_flip_rule`]; `None`: no rule); kept over a backward
    /// seek (a new authority). The presentation host uses it for a page's `?feed_rule=`.
    pub fn set_principal_coin_flip_rule(
        &mut self,
        principal: &str,
        rule: Option<crate::CoinFlipRule>,
    ) {
        self.principal_rules.insert(principal.to_owned(), rule);
        self.session.set_principal_coin_flip_rule(principal, rule);
    }

    /// (R5/M19) Drops `principal`'s own feed rule: the init's applies again.
    pub fn clear_principal_coin_flip_rule(&mut self, principal: &str) {
        self.principal_rules.remove(principal);
        self.session.clear_principal_coin_flip_rule(principal);
    }

    /// The authority.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The authority, mutable (host-side use: e.g. late data checks).
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// The replay.
    #[must_use]
    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    /// The init the authority started from (expanded: `init.data`, no `replay`).
    #[must_use]
    pub fn init(&self) -> &SessionInit {
        &self.init
    }

    /// Every state-changing control so far, with the step count it was applied at (a
    /// recording: the same controls at the same steps reproduce the run).
    #[must_use]
    pub fn controls(&self) -> &[(u64, ReplayControl)] {
        &self.controls
    }

    /// (R4/T5) Records every data event applied to the authority from now on (cleared by
    /// a backward seek, which starts a new authority): a host that feeds replicas sends
    /// them the events past what each has (`qp demo`). Off by default.
    pub fn set_record_events(&mut self, on: bool) {
        self.record = on;
    }

    /// (R4/T5) The data events applied since the start (or the last backward seek), in
    /// order, while recording ([`ReplayHost::set_record_events`]). With the initial bars
    /// (`init().data`) they are a replica's data at the current position.
    ///
    /// (RP4/T6) Kept compact ([`Recorded`]) and written out on demand: a host that sends
    /// only what is new uses [`ReplayHost::recorded_len`] and
    /// [`ReplayHost::recorded_since`].
    #[must_use]
    pub fn recorded_events(&self) -> Vec<DataEvent> {
        self.recorded.since(0)
    }

    /// (RP4/T6) How many data events are recorded.
    #[must_use]
    pub fn recorded_len(&self) -> usize {
        self.recorded.len()
    }

    /// (RP4/T6) The recorded data events from position `from` on (empty past the end).
    #[must_use]
    pub fn recorded_since(&self, from: usize) -> Vec<DataEvent> {
        self.recorded.since(from)
    }

    /// Host notices so far (gaps, stale bindings, rejected events: none expected).
    #[must_use]
    pub fn notices(&self) -> &[String] {
        &self.notices
    }

    /// The replay's state.
    #[must_use]
    pub fn state(&self) -> ReplayState {
        let c = self.replay.cursor();
        let (fine, ohlc, bar) = self.replay.intraday();
        let live = self.live.is_some();
        ReplayState {
            t: if live { self.session.clock() } else { c.t },
            step: c.step,
            steps: self.replay.total_steps(),
            events: self.replay.given_events(),
            from_t: self.replay.first_t(),
            to_t: self.replay.last_t(),
            paused: self.paused,
            done: !live && self.replay.is_done(),
            speed: self.speed,
            synthetic: self.info.kind == ReplaySourceKind::Synthetic,
            source: self.info.kind,
            note: self.info.note.clone(),
            instruments: u32::try_from(self.replay.feed_count()).unwrap_or(u32::MAX),
            interval: self.info.interval.clone(),
            fine: self.info.fine.clone(),
            days: ReplayDays { fine, ohlc, bar },
            epoch: self.init.epoch + self.resets,
            reset: false,
        }
    }
}

/// A demo session init (R4): a document with no series, owner `arkin`, host `host`,
/// agent `scout` (proposer) and viewer `guest`; the replay binds every feed and runs the
/// capsule engine on all of them.
#[must_use]
pub fn demo_init(replay: ReplayInit) -> SessionInit {
    let v = serde_json::json!({
        "document": {"schema_version": "0", "id": "doc-replay", "title": "Yeniden oynatma",
                     "display_tz": "Europe/Istanbul"},
        "principals": [
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"},
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"scout","author":{"kind":"agent","name":"scout"},"role":"proposer"},
            {"id":"guest","author":{"kind":"human","name":"guest"},"role":"viewer"}
        ],
        "clock": 0, "epoch": 0
    });
    let mut init: SessionInit = serde_json::from_value(v).expect("demo init");
    init.replay = Some(ReplayInit {
        bind_all: true,
        capsules_all: true,
        ..replay
    });
    init
}
