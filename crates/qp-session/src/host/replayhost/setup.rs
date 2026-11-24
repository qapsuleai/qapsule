//! Building a replay host: feeds, the expanded init, the window and the live variant.

use super::*;

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

#[allow(clippy::too_many_lines)]
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
        session.set_headline_calendar(qp_plane::headparts::replay_calendar(info.kind));
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
            recorded: Recorded::default(),
            principal_rules: std::collections::BTreeMap::new(),
            live: None,
            market: None,
        })
    }

    /// (R6/T7) A live host: `feeds` are the initial bars (history, e.g. REST) and nothing
    /// is replayed; the host gives the data events itself ([`ReplayHost::live_event`]) and
    /// the clock ([`Session::host_clock`]). `init.replay` (as [`demo_init`] makes it) says
    /// which feeds are bound and run by the capsule engine; its replay fields are ignored.
    /// Events are recorded (viewers' `bind`, replicas' `start`). The authority resolves
    /// from the host's store, so a feed made stale ([`ReplayHost::live_mark_stale`]) is
    /// resolved again on the repaired bars ([`ReplayHost::live_refresh`]). Replay controls
    /// other than `status` are refused.
    ///
    /// # Errors
    /// No `init.replay`, no feeds, invalid bars, or the session's own errors.
    pub fn live(
        mut init: SessionInit,
        root: &Path,
        feeds: Vec<ReplayFeed>,
    ) -> Result<Self, String> {
        let mut r = init
            .replay
            .clone()
            .ok_or("a live host needs init.replay (bind_all, capsules_all)")?;
        r.source = ReplaySourceKind::Local;
        init.replay = Some(r.clone());
        let history = feeds.iter().map(|f| f.bars.len()).max().unwrap_or(0);
        let opts = ReplayOptions {
            history,
            from_t: None,
            to_t: None,
            path: false,
            fine_ms: None,
            calendar: ReplayCalendar::Xist,
        };
        let replay = Replay::new(feeds, &opts).map_err(|e| format!("live: {e}"))?;
        let init = expand_init(init, root, &r, &replay)?;
        let data = init.data.clone().ok_or("live: no initial data")?;
        let provider =
            qp_source::MemoryProvider::from_init(&data, root).map_err(|e| format!("live: {e}"))?;
        let shared = std::sync::Arc::new(std::sync::Mutex::new(provider));
        let mut session_init = init.clone();
        session_init.data = None;
        let mut session =
            Session::with_provider(session_init, root, Box::new(SharedProvider(shared.clone())))?;
        session.set_headline_calendar(qp_plane::headparts::replay_calendar(
            ReplaySourceKind::Local,
        ));
        session.set_viewer_batch(true);
        Ok(Self {
            init,
            root: root.to_owned(),
            session,
            replay,
            info: SourceInfo {
                kind: ReplaySourceKind::Local,
                note: None,
                interval: r.interval.clone().unwrap_or_else(|| "1d".to_owned()),
                fine: None,
            },
            speed: DEFAULT_SPEED,
            paused: false,
            anchor: None,
            resets: 0,
            controls: Vec::new(),
            notices: Vec::new(),
            record: true,
            recorded: Recorded::default(),
            principal_rules: std::collections::BTreeMap::new(),
            live: Some(shared),
            market: None,
        })
    }
}
