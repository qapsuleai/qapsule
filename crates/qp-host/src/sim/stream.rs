//! The simulation run: the output stream and its summary ([`Stream`], [`run`]; RG1: split out of `sim.rs`).

use super::*;

/// The event stream of a run: the data events the authority took, capsule births and
/// state changes, market status changes and the authority's messages, one JSON line each,
/// with a running SHA-256 (the digest compares two runs without printing data).
pub(crate) struct Stream {
    w: Option<BufWriter<std::fs::File>>,
    hash: Sha256,
    sent: usize,
    capsules: BTreeMap<String, (String, u32)>,
    market: Option<Value>,
    cal: ExchangeCalendar,
    iv: Interval,
    /// Capsules are compared again after a bar event or once the clock reaches this.
    next_check: i64,
    /// Lines written.
    pub lines: u64,
    /// Data events.
    pub events: u64,
    /// Capsules born.
    pub born: u64,
    /// Capsule state changes.
    pub changed: u64,
    /// Market status changes.
    pub market_changes: u64,
    /// Authority messages.
    pub messages: u64,
}

impl Stream {
    /// A stream (to `out` if given) after `host`'s initial state.
    ///
    /// # Errors
    /// The file cannot be created.
    pub fn new(
        out: Option<&Path>,
        host: &ReplayHost,
        cal: ExchangeCalendar,
        iv: Interval,
    ) -> Result<Self, String> {
        let w = match out {
            Some(p) => Some(BufWriter::new(
                std::fs::File::create(p).map_err(|e| format!("{}: {e}", p.display()))?,
            )),
            None => None,
        };
        let capsules = host
            .session()
            .document()
            .capsules
            .iter()
            .map(|c| (c.id.clone(), (format!("{:?}", c.state), c.revision)))
            .collect();
        Ok(Self {
            w,
            hash: Sha256::new(),
            sent: host.recorded_len(),
            capsules,
            market: None,
            cal,
            iv,
            next_check: i64::MIN,
            lines: 0,
            events: 0,
            born: 0,
            changed: 0,
            market_changes: 0,
            messages: 0,
        })
    }

    fn put(&mut self, v: &Value) -> Result<(), String> {
        let s = v.to_string();
        self.hash.update(s.as_bytes());
        self.hash.update(b"\n");
        self.lines += 1;
        if let Some(w) = self.w.as_mut() {
            writeln!(w, "{s}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Takes what one input caused: new data events, the authority's messages, the
    /// market status and capsule changes (compared after a bar event, and when the
    /// authority's clock passes the end of the bar it was in at the last comparison).
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn after(
        &mut self,
        host: &ReplayHost,
        rt: &LiveRuntime,
        lines: &[qp_schema::protocol::OutboundLine],
    ) -> Result<(), String> {
        let total = host.recorded_len();
        let mut bar_event = false;
        if total > self.sent {
            let new: Vec<Value> = host
                .recorded_since(self.sent)
                .iter()
                .map(|e| {
                    bar_event |= matches!(e.change, DataChange::Bar { .. });
                    json!({"ev": e})
                })
                .collect();
            self.sent = total;
            for v in new {
                self.events += 1;
                self.put(&v)?;
            }
        }
        for l in lines {
            self.messages += 1;
            self.put(&json!({"conn": l.conn, "msg": l.msg}))?;
        }
        if let Some(m) = rt.market() {
            let v = json!({"open": m.open, "holiday": m.holiday, "stale": m.stale,
                           "phases": m.phases.len()});
            if self.market.as_ref() != Some(&v) {
                self.market_changes += 1;
                self.market = Some(v.clone());
                self.put(&json!({"t": m.exchange_time, "market": v}))?;
            }
        }
        let t = host.session().clock();
        if bar_event || t >= self.next_check {
            self.capsule_changes(host)?;
            self.next_check = self
                .cal
                .bar_at(self.iv, t)
                .map_or(t + 60_000, |s| s.close.max(t + 1));
        }
        Ok(())
    }

    fn capsule_changes(&mut self, host: &ReplayHost) -> Result<(), String> {
        let doc = host.session().document();
        let t = host.session().clock();
        let mut out = Vec::new();
        for c in &doc.capsules {
            let st = format!("{:?}", c.state);
            match self.capsules.get(&c.id) {
                Some((s, r)) if *s == st && *r == c.revision => continue,
                Some((s, _)) => {
                    if *s != st {
                        self.changed += 1;
                    }
                }
                None => self.born += 1,
            }
            out.push(json!({"t": t, "capsule": c.id, "state": st.to_lowercase(),
                            "rev": c.revision}));
            self.capsules.insert(c.id.clone(), (st, c.revision));
        }
        for v in out {
            self.put(&v)?;
        }
        Ok(())
    }

    /// Flushes and returns the digest (hex).
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn finish(mut self) -> Result<String, String> {
        if let Some(w) = self.w.as_mut() {
            w.flush().map_err(|e| e.to_string())?;
        }
        Ok(self
            .hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
}

/// Checks that `out` is under `root/fixtures/local/` (real data stays there).
///
/// # Errors
/// Anywhere else.
pub(crate) fn check_out(out: &Path, root: &Path) -> Result<PathBuf, String> {
    let abs = |p: &Path| {
        if p.is_absolute() {
            p.to_owned()
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        }
    };
    let local = abs(&root.join("fixtures/local"));
    let local = local.canonicalize().map_err(|e| {
        format!(
            "--out: {} must exist ({e}); real data is written only there",
            local.display()
        )
    })?;
    let out = abs(out);
    let parent = out
        .parent()
        .ok_or("--out: no directory")?
        .canonicalize()
        .map_err(|e| format!("--out: {e}"))?;
    let name = out.file_name().ok_or("--out: no file name")?;
    if !parent.starts_with(&local) {
        return Err(format!(
            "--out: real data is written only under {} (got {})",
            local.display(),
            parent.display()
        ));
    }
    Ok(parent.join(name))
}

/// Result counts of a run (printed; no values).
#[derive(Debug, Clone)]
pub(crate) struct RunSummary {
    /// The summary as JSON.
    pub json: Value,
    /// The stream digest.
    pub digest: String,
}

/// Runs a simulation to the end, in this thread: every input goes straight to the
/// runtime (`LiveRuntime::on_in`), then the viewers' flush, as the demo hub does.
///
/// # Errors
/// Loading, the host, or the output file.
pub(crate) fn run(
    spec: &SimSpec,
    speed: Speed,
    out: Option<&Path>,
    pace: &dyn PaceClock,
) -> Result<RunSummary, String> {
    let t0 = Instant::now();
    let input = load(spec)?;
    let load_ms = t0.elapsed().as_millis();
    let decode = TradeDecodeConfig::default();
    let (mut host, mut rt) = runtime(spec, &input, Arc::new(|_| {}), decode)?;
    let iv = Interval::parse(&spec.interval).map_err(|e| e.to_string())?;
    let mut stream = Stream::new(out, &host, input.cal.clone(), iv)?;
    let mut pacer = Pacer::new(speed, pace, input.cal.clone());
    let t1 = Instant::now();
    let mut err = None;
    let ds = drive(&input, iv, &decode, &mut pacer, &mut |i| {
        let clock = matches!(i, LiveIn::Clock(_));
        let mut lines = rt.on_in(&mut host, i);
        if clock {
            lines.extend(host.live_flush());
        }
        match stream.after(&host, &rt, &lines) {
            Ok(()) => true,
            Err(e) => {
                err = Some(e);
                false
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    let run_ms = t1.elapsed().as_millis();
    let rt_stats = rt.stats(&host);
    let (events, born, changed, market_changes, lines) = (
        stream.events,
        stream.born,
        stream.changed,
        stream.market_changes,
        stream.lines,
    );
    let digest = stream.finish()?;
    let json = json!({
        "symbols": spec.symbols.len(),
        "symbols_with_history": input.hist.len(),
        "history_from_rest": input.hist_from.rest.len(),
        "history_from_trades": input.hist_from.trades.len(),
        "symbols_without_history": input.missing.len(),
        "days": ds.days,
        "trading_days": (spec.first_day..=spec.last_day).filter(|&d| input.cal.is_trading_day(d)).count(),
        "interval": spec.interval,
        "speed": speed.label(),
        "cache": {"files": input.load.files, "trades": input.load.trades,
                  "bad_rows": input.load.bad_rows, "duplicates": input.load.duplicates},
        "trade_records": ds.trades,
        "clock_moves": ds.clock_moves,
        "events": events,
        "capsules_born": born,
        "capsule_state_changes": changed,
        "market_changes": market_changes,
        "stream_lines": lines,
        "finalized_bars": rt_stats["finalized_bars"],
        "events_rejected": rt_stats["events_rejected"],
        "feed_trades": rt_stats["feed"]["trades"],
        "trade_repairs": rt.trade_repairs.0,
        "trade_repair_errors": rt.trade_repairs.1,
        "load_ms": load_ms,
        "run_ms": run_ms,
        "notices": host.notices().len(),
    });
    Ok(RunSummary { json, digest })
}
