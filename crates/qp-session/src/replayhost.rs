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

/// Builds the replay feeds asked for by `r`; local without data falls back to the
/// synthetic market with a note.
/// Keeps each feed's last `window` coarse bars before `from_t` (and the replayed part);
/// finer bars before the first kept coarse bar go too (they are used only inside replayed
/// days). A feed with no bar before `from_t` is left as it is (the replay reports it).
fn window_feeds(feeds: &mut [ReplayFeed], from_t: i64, window: usize) {
    for f in feeds {
        let first = f.bars.partition_point(|b| b.t < from_t);
        let cut = first.saturating_sub(window.max(1));
        if cut == 0 {
            continue;
        }
        f.bars.drain(..cut);
        let t0 = f.bars[0].t;
        let lo = f.fine.partition_point(|b| b.t < t0);
        f.fine.drain(..lo);
    }
}

fn build_feeds(r: &ReplayInit, local_dir: &Path) -> Result<(Vec<ReplayFeed>, SourceInfo), String> {
    let n = r.instruments.unwrap_or(DEFAULT_INSTRUMENTS).max(1) as usize;
    let interval = r.interval.clone().unwrap_or_else(|| "1d".to_owned());
    let feed = r.feed.as_deref();
    if let Some(f) = feed {
        qp_source::validate_id("feed", f).map_err(|e| format!("init.replay.feed: {e}"))?;
    }
    if let Some(f) = &r.fine {
        interval_ms(f).ok_or_else(|| format!("init.replay.fine: unknown interval `{f}`"))?;
    }
    let mut note = None;
    if r.source == ReplaySourceKind::Local {
        let period = local_period(&interval)
            .ok_or_else(|| format!("init.replay.interval: no local files for `{interval}`"))?;
        let fine_period = match &r.fine {
            Some(f) => Some(
                local_period(f)
                    .ok_or_else(|| format!("init.replay.fine: no local files for `{f}`"))?,
            ),
            None => None,
        };
        let feeds =
            qp_source::local_replay_feeds(local_dir, period, &interval, fine_period, n, feed);
        if !feeds.is_empty() {
            let info = SourceInfo {
                kind: ReplaySourceKind::Local,
                note: None,
                interval,
                fine: r.fine.clone(),
            };
            return Ok((feeds, info));
        }
        note = Some("no local data on this machine: the synthetic market is shown".to_owned());
    }
    if interval != "1d" {
        return Err(format!(
            "init.replay.interval: the synthetic market has `1d` only, not `{interval}`"
        ));
    }
    let fine = match r.fine.as_deref() {
        None => false,
        Some("1h" | "60m") => true,
        Some(f) => {
            return Err(format!(
                "init.replay.fine: the synthetic market has `1h` only, not `{f}`"
            ))
        }
    };
    let days = r.days.unwrap_or(DEFAULT_DAYS).max(2) as usize;
    let feeds = qp_source::synthetic_replay_feeds(n, days, r.seed.unwrap_or(1), fine, feed);
    let info = SourceInfo {
        kind: ReplaySourceKind::Synthetic,
        note,
        interval: "1d".to_owned(),
        fine: fine.then(|| "1h".to_owned()),
    };
    Ok((feeds, info))
}

/// Expands a session init with `init.replay` into the init the authority starts from:
/// `init.data` from the replay's initial bars, bound series and capsule series if asked,
/// the clock at the replay's start; `replay` removed.
fn expand_init(
    mut init: SessionInit,
    root: &Path,
    r: &ReplayInit,
    replay: &Replay,
) -> Result<SessionInit, String> {
    if init.data.is_some() {
        return Err("give either init.data or init.replay, not both".to_owned());
    }
    let mut doc = match init.document {
        DocumentRef::Inline(d) => *d,
        DocumentRef::Path(p) => read_json(&confine(root, &p)?)?,
    };
    let mut replay_series = Vec::new();
    for (key, initial) in replay.feeds() {
        let bound = doc
            .series
            .iter()
            .find(|s| s.binding().is_some_and(|b| b.feed_key() == *key))
            .map(|s| s.id.clone());
        let id = match bound {
            Some(id) => id,
            None if r.bind_all => {
                let id = key.symbol.to_lowercase();
                if doc.series.iter().any(|s| s.id == id) {
                    return Err(format!(
                        "init.replay.bind_all: series id `{id}` is taken in the document"
                    ));
                }
                let n = u32::try_from(initial.len()).unwrap_or(u32::MAX);
                doc.series.push(Series {
                    id: id.clone(),
                    instrument: Some(Instrument {
                        symbol: key.symbol.clone(),
                        exchange: None,
                        calendar: None,
                    }),
                    interval: Some(key.interval.clone()),
                    source: SeriesSource::Bind {
                        symbol: key.symbol.clone(),
                        interval: key.interval.clone(),
                        window: Window::last(n),
                        feed: key.feed.clone(),
                        anchor_t: None,
                    },
                    data: None,
                });
                id
            }
            None => continue,
        };
        replay_series.push(id);
    }
    if r.capsules_all {
        let c = init.capsules.get_or_insert_with(Default::default);
        for id in replay_series {
            if !c.series.contains(&id) {
                c.series.push(id);
            }
        }
    }
    let entitlements = if r.entitlements.is_empty() {
        init.principals
            .iter()
            .map(|p| (p.id.clone(), vec!["*".to_owned()]))
            .collect()
    } else {
        r.entitlements.clone()
    };
    init.data = Some(InitData {
        feeds: replay
            .feeds()
            .map(|(k, b)| InitFeed {
                feed: k.clone(),
                bars: Some(b.to_vec()),
                path: None,
                synthetic: None,
            })
            .collect(),
        entitlements,
        pending: Vec::new(),
        settled: None,
    });
    init.document = DocumentRef::Inline(Box::new(doc));
    init.clock = replay.start_t();
    init.replay = None;
    Ok(init)
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
    recorded: Vec<DataEvent>,
    /// (R5/M19) Per-principal feed rules the host set (kept over a backward seek).
    principal_rules: std::collections::BTreeMap<String, Option<crate::CoinFlipRule>>,
    /// (R6/T3) The market status the host gave (kept over a backward seek).
    market: Option<qp_schema::market::MarketStatus>,
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
    /// Starts an authority on the replay of `init.replay`; local data is read from
    /// [`qp_source::local_bars_dir`].
    ///
    /// # Errors
    /// No `init.replay`, an invalid replay, or the session's own errors.
    pub fn new(init: SessionInit, root: &Path) -> Result<Self, String> {
        Self::with_local_dir(init, root, &qp_source::local_bars_dir())
    }

    /// [`ReplayHost::new`] with local data read from `local_dir`.
    ///
    /// # Errors
    /// As [`ReplayHost::new`].
    pub fn with_local_dir(
        init: SessionInit,
        root: &Path,
        local_dir: &Path,
    ) -> Result<Self, String> {
        Self::with_window(init, root, local_dir, None)
    }

    /// (R4/T6) [`ReplayHost::with_local_dir`] with a history window: with `init.replay.from_t`
    /// set, each feed keeps only its last `window` bars before the first replayed bar (the
    /// initial resolution; `from_t` alone takes every earlier bar). Without `from_t` the
    /// window changes nothing (`init.replay.history` already counts the bars). For a host
    /// whose readers must open fast (`qp demo`): the document — capsules, ledger — grows
    /// with the history.
    ///
    /// # Errors
    /// As [`ReplayHost::new`].
    pub fn with_window(
        init: SessionInit,
        root: &Path,
        local_dir: &Path,
        window: Option<usize>,
    ) -> Result<Self, String> {
        let r = init
            .replay
            .clone()
            .ok_or("a replay host needs init.replay")?;
        let (mut feeds, info) = build_feeds(&r, local_dir)?;
        if let (Some(w), Some(from)) = (window, r.from_t) {
            window_feeds(&mut feeds, from, w);
        }
        let speed = r.speed.unwrap_or(DEFAULT_SPEED);
        if !(speed > 0.0 && speed <= MAX_SPEED) {
            return Err(format!(
                "init.replay.speed: {speed} is not in (0, {MAX_SPEED}]"
            ));
        }
        let opts = ReplayOptions {
            history: r.history.unwrap_or(DEFAULT_HISTORY) as usize,
            from_t: r.from_t,
            to_t: r.to_t,
            path: r.path.unwrap_or(true),
            fine_ms: info.fine.as_deref().and_then(interval_ms),
            calendar: match info.kind {
                ReplaySourceKind::Local => ReplayCalendar::Xist,
                ReplaySourceKind::Synthetic => ReplayCalendar::Synthetic,
            },
        };
        let replay = Replay::new(feeds, &opts).map_err(|e| format!("init.replay: {e}"))?;
        let init = expand_init(init, root, &r, &replay)?;
        let mut session = Session::new(init.clone(), root)?;
        // (R4/H-FP) The headline pieces' elapsed phrases on the replay's own calendar.
        session.set_headline_calendar(crate::headparts::replay_calendar(info.kind));
        // (RP3/M15, §22) Viewers are told once per step, not after every event of it.
        session.set_viewer_batch(true);
        Ok(Self {
            init,
            root: root.to_owned(),
            session,
            replay,
            info,
            speed,
            paused: !r.running,
            anchor: None,
            resets: 0,
            controls: Vec::new(),
            notices: Vec::new(),
            record: false,
            recorded: Vec::new(),
            principal_rules: std::collections::BTreeMap::new(),
            market: None,
        })
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
    #[must_use]
    pub fn recorded_events(&self) -> &[DataEvent] {
        &self.recorded
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
        ReplayState {
            t: c.t,
            step: c.step,
            steps: self.replay.total_steps(),
            events: self.replay.given_events(),
            from_t: self.replay.first_t(),
            to_t: self.replay.last_t(),
            paused: self.paused,
            done: self.replay.is_done(),
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

    /// One raw inbound message on `conn` (the in-process transport): a `replay` message
    /// of a connection that said `hello` is answered here, everything else goes to the
    /// authority unchanged (which also reports a malformed `replay` message).
    pub fn handle_raw(&mut self, conn: &str, line: &str) -> Vec<OutboundLine> {
        match self.as_replay(conn, line) {
            Some((id, control)) => self.control_msg(conn, id, control),
            None => self.session.handle_raw(conn, line),
        }
    }

    /// One line of the line transport (`{"conn":…,"msg":…}`), as
    /// [`Session::handle_frame`]; framing errors are the authority's.
    pub fn handle_frame(&mut self, line: &str) -> Vec<OutboundLine> {
        let v: Option<serde_json::Value> = serde_json::from_str(line).ok();
        let framed = v.as_ref().and_then(|v| {
            let o = v.as_object()?;
            if o.keys().any(|k| k != "conn" && k != "msg") {
                return None;
            }
            let conn = o.get("conn")?.as_str()?;
            let text = match o.get("msg")? {
                serde_json::Value::String(s) => s.clone(),
                m => m.to_string(),
            };
            Some((conn.to_owned(), text))
        });
        match framed {
            Some((conn, text)) => self.handle_raw(&conn, &text),
            None => self.session.handle_frame(line),
        }
    }

    /// A data event on the host's path, outside the replay (a transcript's `data` step).
    ///
    /// # Errors
    /// The authority rejected it.
    pub fn data_event(&mut self, ev: &DataEvent) -> Result<Vec<OutboundLine>, crate::DataError> {
        let mut lines = self.session.data_event(ev)?;
        lines.extend(self.session.viewer_flush());
        Ok(lines)
    }

    /// (RP3/M15, §22) What a host serves a viewer for feed `key` when it subscribes to it:
    /// the feed's initial bars and the data events applied since (while recording,
    /// [`ReplayHost::set_record_events`]). `None`: no such feed.
    #[must_use]
    pub fn feed_data(&self, key: &qp_schema::data::FeedKey) -> Option<(InitFeed, Vec<DataEvent>)> {
        let f = self
            .init
            .data
            .as_ref()?
            .feeds
            .iter()
            .find(|f| &f.feed == key)?
            .clone();
        let evs = self
            .recorded
            .iter()
            .filter(|e| &e.feed == key)
            .cloned()
            .collect();
        Some((f, evs))
    }

    fn as_replay(&self, conn: &str, line: &str) -> Option<(String, ReplayControl)> {
        if line.len() as u64 > self.session.limits().max_message_bytes
            || !line.contains("\"replay\"")
        {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        if v.get("type").and_then(serde_json::Value::as_str) != Some("replay") {
            return None;
        }
        self.session.conns.iter().find(|c| c.id == conn)?;
        match serde_json::from_value::<ClientMsg>(v).ok()? {
            ClientMsg::Replay { id, control } => Some((id, control)),
            _ => None,
        }
    }

    fn error(conn: &str, id: &str, e: ProtocolError) -> Vec<OutboundLine> {
        vec![OutboundLine {
            conn: conn.to_owned(),
            msg: ServerMsg::Error {
                id: Some(id.to_owned()),
                error: e,
            },
        }]
    }

    /// A `replay` message of connection `conn` (it said `hello`).
    fn control_msg(&mut self, conn: &str, id: String, control: ReplayControl) -> Vec<OutboundLine> {
        let Some((principal, role)) = self
            .session
            .conns
            .iter()
            .find(|c| c.id == conn)
            .map(|c| (c.principal.clone(), c.role))
        else {
            return Vec::new();
        };
        let status = matches!(control, ReplayControl::Status);
        let allowed = if status {
            permissions(role).read
        } else {
            matches!(role, Role::Host | Role::Owner)
        };
        if !allowed {
            let e = if status {
                E::new(
                    ErrorCode::Forbidden,
                    "this role may not read the replay status",
                )
            } else {
                E::new(
                    ErrorCode::Forbidden,
                    "only the roles `host` and `owner` control the replay",
                )
                .hint("Ask for `{\"kind\":\"status\"}`; the timeline is shared by every reader.")
            };
            return Self::error(conn, &id, e.at("/control").build());
        }
        let (state, lines) = match self.host_control(&control) {
            Ok(x) => x,
            Err(e) => return Self::error(conn, &id, e),
        };
        let seq = if state.reset {
            self.session.log().len() as u64
        } else {
            self.session
                .seq_to(&principal, self.session.log().len() as u64)
        };
        let mut out = vec![OutboundLine {
            conn: conn.to_owned(),
            msg: ServerMsg::Result {
                id,
                seq,
                now: self.session.clock(),
                data: QueryResult::Replay { replay: state },
                resolved: BTreeMap::new(),
            },
        }];
        out.extend(lines);
        out
    }

    /// Applies a control on the host's own authority (no connection, no permission
    /// check): the state after it and the notifications its steps caused. State-changing
    /// controls are recorded ([`ReplayHost::controls`]).
    ///
    /// # Errors
    /// An invalid control (`invalid`, with the JSON pointer of the field).
    pub fn host_control(
        &mut self,
        control: &ReplayControl,
    ) -> Result<(ReplayState, Vec<OutboundLine>), ProtocolError> {
        let at = self.replay.cursor().step;
        let mut lines = Vec::new();
        let mut reset = false;
        match control {
            ReplayControl::Status => return Ok((self.state(), lines)),
            ReplayControl::Pause => {
                self.paused = true;
                self.anchor = None;
            }
            ReplayControl::Resume => {
                self.paused = false;
                self.anchor = None;
            }
            ReplayControl::Speed { steps_per_s } => {
                let s = *steps_per_s;
                if !(s > 0.0 && s <= MAX_SPEED) {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("speed {s} is not in (0, {MAX_SPEED}] steps per second"),
                    )
                    .at("/control/steps_per_s")
                    .expected([format!("0 < steps_per_s <= {MAX_SPEED}")])
                    .build());
                }
                self.speed = s;
                self.anchor = None;
            }
            ReplayControl::Step { n } => {
                let n = n.unwrap_or(1);
                if n == 0 || n > MAX_STEP_N {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("`n` is {n}; take 1 to {MAX_STEP_N} steps"),
                    )
                    .at("/control/n")
                    .expected([format!("1..={MAX_STEP_N}")])
                    .build());
                }
                lines = self.take(u64::from(n));
                self.anchor = None;
            }
            ReplayControl::Seek { t, date, step } => {
                let target = match (t, date, step) {
                    (Some(t), None, None) => self.replay.steps_until(*t),
                    (None, Some(d), None) => {
                        let start = qp_source::date_start(d).map_err(|e| {
                            E::new(ErrorCode::Invalid, format!("`date`: {e}"))
                                .at("/control/date")
                                .expected(["YYYY-MM-DD"])
                                .build()
                        })?;
                        self.replay.steps_until(start - 1)
                    }
                    (None, None, Some(s)) => *s,
                    _ => {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            "`seek` takes exactly one of `t`, `date`, `step`",
                        )
                        .at("/control")
                        .expected(["t", "date", "step"])
                        .build())
                    }
                };
                let target = target.min(self.replay.total_steps());
                if target < at {
                    self.rebuild()
                        .map_err(|e| E::new(ErrorCode::Invalid, e).build())?;
                    let _ = self.take(target);
                    reset = true;
                } else {
                    lines = self.take(target - at);
                }
                self.anchor = None;
            }
        }
        self.controls.push((at, control.clone()));
        let mut state = self.state();
        state.reset = reset;
        Ok((state, lines))
    }

    /// A new authority from the init (`epoch + 1`), the replay back at its start.
    fn rebuild(&mut self) -> Result<(), String> {
        self.resets += 1;
        let mut init = self.init.clone();
        init.epoch += self.resets;
        self.session = Session::new(init, &self.root)?;
        self.session
            .set_headline_calendar(crate::headparts::replay_calendar(self.info.kind));
        self.session.set_viewer_batch(true);
        for (p, r) in &self.principal_rules {
            self.session.set_principal_coin_flip_rule(p, *r);
        }
        if let Some(m) = &self.market {
            // No viewer yet on the new authority: nothing to tell.
            let _ = self.session.set_market_status(m.clone());
        }
        self.replay.reset();
        self.recorded.clear();
        Ok(())
    }

    /// Applies one replay step to the authority: the clock, then the events in order.
    fn apply(&mut self, st: &ReplayStep) -> Vec<OutboundLine> {
        let mut lines = match self.session.host_clock(st.t) {
            Ok(l) => l,
            Err(e) => {
                self.notices.push(format!("clock {}: {}", st.t, e.message));
                Vec::new()
            }
        };
        if self.record {
            self.recorded.extend(st.events.iter().cloned());
        }
        for ev in &st.events {
            match self.session.host_data_event(ev) {
                Ok(o) => {
                    for n in &o.notices {
                        self.notices.push(n.frame().to_string());
                    }
                    lines.extend(o.lines);
                }
                Err(e) => self.notices.push(format!(
                    "event {}/{} seq {}: {e}",
                    ev.feed.symbol, ev.feed.interval, ev.seq
                )),
            }
        }
        lines
    }

    /// Takes up to `n` steps; returns the notifications they caused.
    fn take(&mut self, n: u64) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for _ in 0..n {
            let Some(st) = self.replay.step() else { break };
            lines.extend(self.apply(&st));
        }
        // (RP3/M15, §22) Viewers once, after the steps (what they show now).
        if n > 0 {
            lines.extend(self.session.viewer_flush());
        }
        lines
    }

    /// Takes one step and returns it with the notifications it caused (`None` at the
    /// end): for a host that also feeds a replica from the same steps.
    pub fn take_step(&mut self) -> Option<(ReplayStep, Vec<OutboundLine>)> {
        let st = self.replay.step()?;
        let mut lines = self.apply(&st);
        lines.extend(self.session.viewer_flush());
        Some((st, lines))
    }

    /// The wall clock moved to `wall_ms` (any monotone millisecond clock): while running,
    /// takes the steps that are due at the current speed (at most
    /// [`MAX_STEPS_PER_PUMP`]) and returns their notifications. The first call after a
    /// start, resume, speed change, step or seek only sets the anchor.
    pub fn pump(&mut self, wall_ms: u64) -> Vec<OutboundLine> {
        if self.paused || self.replay.is_done() {
            self.anchor = None;
            return Vec::new();
        }
        let step = self.replay.cursor().step;
        let Some((w0, s0)) = self.anchor else {
            self.anchor = Some((wall_ms, step));
            return Vec::new();
        };
        let elapsed = wall_ms.saturating_sub(w0);
        // Steps due since the anchor; `as` after `floor` of a bounded positive value.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let due = s0 + ((elapsed as f64) * self.speed / 1000.0).floor() as u64;
        let n = due.saturating_sub(step).min(MAX_STEPS_PER_PUMP);
        if due.saturating_sub(step) > MAX_STEPS_PER_PUMP {
            // Fell behind: catch up from here instead of bursting.
            self.anchor = Some((wall_ms, step + n));
        }
        self.take(n)
    }

    /// Wall milliseconds from `wall_ms` until the next step is due (`None`: paused,
    /// done, or no anchor yet — call [`ReplayHost::pump`] now).
    #[must_use]
    pub fn next_due_in(&self, wall_ms: u64) -> Option<u64> {
        if self.paused || self.replay.is_done() {
            return None;
        }
        let (w0, s0) = self.anchor?;
        let next = self.replay.cursor().step + 1 - s0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let at = w0 + ((next as f64) * 1000.0 / self.speed).ceil() as u64;
        Some(at.saturating_sub(wall_ms))
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
