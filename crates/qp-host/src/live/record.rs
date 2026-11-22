//! `qp live record`: the raw-log sink and the recording connection ([`RawSink`], [`record`]; RG1: split out of `live.rs`).

use super::*;

/// A sink that appends every message and event to a raw log and counts. Trade payloads are
/// written without buyer and seller ([`keepable_payload_on`]).
pub(crate) struct RawSink {
    w: Mutex<RawWriter<BufWriter<File>>>,
    pub messages: AtomicU64,
    pub bytes: AtomicU64,
    pub events: Mutex<Vec<String>>,
    pub errors: AtomicU64,
    /// Trade messages (sanitized).
    pub trade_messages: AtomicU64,
    /// Trade messages that carried a buyer / a seller field on the wire (dropped).
    pub level2_buyer: AtomicU64,
    pub level2_seller: AtomicU64,
    /// Trade payloads that did not decode (kept empty).
    pub unreadable_trades: AtomicU64,
    finished: std::sync::atomic::AtomicBool,
    /// Sink of the trade endpoint: every payload is sanitized as a trade.
    trade_endpoint: bool,
}

impl RawSink {
    pub fn create(path: &Path) -> Result<Self, String> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let f = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let w = RawWriter::new(BufWriter::new(f)).map_err(|e| e.to_string())?;
        Ok(Self {
            w: Mutex::new(w),
            messages: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            events: Mutex::new(Vec::new()),
            errors: AtomicU64::new(0),
            trade_messages: AtomicU64::new(0),
            level2_buyer: AtomicU64::new(0),
            level2_seller: AtomicU64::new(0),
            unreadable_trades: AtomicU64::new(0),
            finished: std::sync::atomic::AtomicBool::new(false),
            trade_endpoint: false,
        })
    }

    /// The sink of a trade-endpoint connection: every received payload goes through the
    /// trade sanitizer, whatever its topic.
    #[must_use]
    pub fn for_trade_endpoint(mut self) -> Self {
        self.trade_endpoint = true;
        self
    }

    /// Writes one received message (trade payloads sanitized, counted) and returns the
    /// payload as kept, for the hub.
    pub fn take_message<'a>(&self, m: &LiveMessage<'a>) -> std::borrow::Cow<'a, [u8]> {
        self.messages.fetch_add(1, Ordering::Relaxed);
        self.bytes
            .fetch_add(m.payload.len() as u64, Ordering::Relaxed);
        let (kept, l2) = keepable_payload_on(m.topic, m.payload, self.trade_endpoint);
        if self.trade_endpoint || is_trade_topic(m.topic) {
            self.trade_messages.fetch_add(1, Ordering::Relaxed);
            match l2 {
                Some((b, s)) => {
                    self.level2_buyer.fetch_add(u64::from(b), Ordering::Relaxed);
                    self.level2_seller
                        .fetch_add(u64::from(s), Ordering::Relaxed);
                }
                None => {
                    self.unreadable_trades.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.put(&RawRecord::message(
            m.received_at_us,
            m.topic,
            kept.as_ref().to_vec(),
        ));
        kept
    }

    /// The trade sanitizing counts (counts only).
    #[must_use]
    pub fn trade_note(&self) -> String {
        format!(
            "{TRADE_SANITIZE_NOTE} messages={} buyer_present={} seller_present={} unreadable={}",
            self.trade_messages.load(Ordering::Relaxed),
            self.level2_buyer.load(Ordering::Relaxed),
            self.level2_seller.load(Ordering::Relaxed),
            self.unreadable_trades.load(Ordering::Relaxed)
        )
    }

    /// Ends the log: the trade sanitizing note (once, if trades were seen), then a flush.
    pub fn finish(&self) {
        if self.trade_messages.load(Ordering::Relaxed) > 0
            && !self.finished.swap(true, Ordering::Relaxed)
        {
            self.put(&RawRecord {
                t_us: now_us(),
                kind: RecordKind::Note,
                topic: String::new(),
                payload: self.trade_note().into_bytes(),
            });
        }
        self.flush();
    }

    fn put(&self, rec: &RawRecord) {
        let mut w = self
            .w
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if w.write(rec).is_err() {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Appends one record (a local note, e.g. a deliberate drop).
    pub fn put_record(&self, rec: &RawRecord) {
        self.put(rec);
    }

    pub fn flush(&self) {
        let mut w = self
            .w
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = w.flush();
    }
}

impl LiveSink for RawSink {
    fn on_message(&self, m: &LiveMessage<'_>) {
        let _ = self.take_message(m);
    }

    fn on_event(&self, _ep: &qp_live::EndpointKey, ev: &LiveEvent) {
        let t = now_us();
        let (kind, topic, note) = match ev {
            LiveEvent::Connected { session } => (
                RecordKind::ConnectionOpened,
                String::new(),
                format!("session {session}"),
            ),
            LiveEvent::Disconnected { .. } => {
                (RecordKind::ConnectionLost, String::new(), ev.to_string())
            }
            LiveEvent::Subscribed { topic } => {
                (RecordKind::Subscribed, topic.clone(), String::new())
            }
            LiveEvent::SubscribeRejected { topic, code } => (
                RecordKind::SubscribeRejected,
                topic.clone(),
                format!("0x{code:02x}"),
            ),
            _ => (RecordKind::Note, String::new(), ev.to_string()),
        };
        self.put(&RawRecord {
            t_us: t,
            kind,
            topic,
            payload: note.into_bytes(),
        });
        if !matches!(ev, LiveEvent::Subscribed { .. }) {
            let mut e = self
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            e.push(format!("{} {ev}", utc_stamp(t)));
        }
    }
}

/// The topics of `qp live record`: the market endpoint's (clock, notices, price cards) or,
/// with `--trades`, the trade endpoint's (level 1 trade topics; `@lvl2` refused).
///
/// # Errors
/// A bad symbol, or a refused level.
pub(crate) fn record_topics(a: &RecordArgs, symbols: &[String]) -> Result<Vec<String>, String> {
    let mut topics = Vec::new();
    if a.trades {
        if a.messenger {
            topics.push("mx/messenger".to_owned());
        }
        for s in symbols {
            let mut streams = vec![qp_matriks::TradeStream::Regular];
            if a.tradeex {
                streams.push(qp_matriks::TradeStream::Special);
            }
            for st in streams {
                let t = qp_matriks::trade_topic_at(st, s, a.level.as_deref())
                    .map_err(|e| format!("--level / --symbols: {e} ({s})"))?;
                topics.push(t);
            }
        }
        if let Some(t) = topics.iter().find(|t| qp_live::is_level2_trade_topic(t)) {
            return Err(format!("refused level-2 trade topic {t}"));
        }
        return Ok(topics);
    }
    topics.push("mx/timestamp".to_owned());
    if a.messenger {
        topics.push("mx/messenger".to_owned());
    }
    let level = a.level.as_deref().unwrap_or("@lvl2");
    topics.extend(symbols.iter().map(|s| format!("mx/symbol/{s}{level}")));
    Ok(topics)
}

pub(super) fn record(a: &RecordArgs) -> Result<(), String> {
    let symbols = parse_symbols(&a.symbols)?;
    let topics = record_topics(a, &symbols)?;
    let _identity = identity_lock()?;
    let cfg = LiveConfig::from_env().map_err(|e| e.to_string())?;
    if cfg.hardware_id_warning() {
        eprintln!("live: warning: MATRIKS_HARDWARE_ID does not start with `qapsule-`");
    }
    let root = repo_root();
    let start_us = now_us();
    let out = a.out.clone().unwrap_or_else(|| {
        root.join("fixtures/local/live/raw").join(format!(
            "{}{}.qpraw",
            utc_stamp(start_us),
            if a.trades { "-trade" } else { "" }
        ))
    });
    // A relative `--out` is taken from the current directory; `..` is refused so the
    // prefix check below cannot be walked around.
    let out = if out.is_relative() {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(out)
    } else {
        out
    };
    if out
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
        || !out.starts_with(root.join("fixtures/local"))
    {
        return Err("live: the raw log must stay under fixtures/local/ (K23)".to_owned());
    }
    let endpoint = if a.trades {
        cfg.trade_endpoint()
    } else {
        cfg.market_endpoint()
    };
    let sink = if a.trades {
        Arc::new(RawSink::create(&out)?.for_trade_endpoint())
    } else {
        Arc::new(RawSink::create(&out)?)
    };
    let auth = Arc::new(AuthClient::new(&cfg, AuthPolicy::default()));
    let mut opts = ConnectOptions::new(topics.clone());
    opts.classifier = Some(classifier());
    opts.lock_dir = qp_live::lock::default_lock_dir();
    println!(
        "live: mode {:?}, endpoint {}, {} symbols, {} topics, {} s",
        cfg.mode(),
        endpoint.key(),
        symbols.len(),
        topics.len(),
        a.seconds
    );
    let t0 = Instant::now();
    let conn = LiveConnection::open(
        &cfg,
        endpoint,
        auth,
        opts,
        sink.clone() as Arc<dyn LiveSink>,
    )
    .map_err(|e| e.to_string())?;
    let deadline = t0 + Duration::from_secs(a.seconds);
    let mut next = t0 + Duration::from_secs(a.every.max(1));
    let mut seen_events = 0usize;
    let halted = loop {
        std::thread::sleep(Duration::from_millis(200));
        {
            let ev = sink
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for e in &ev[seen_events..] {
                println!("live: {e}");
            }
            seen_events = ev.len();
        }
        let st = conn.state();
        if let ConnState::Halted(r) = &st {
            break Some(r.to_string());
        }
        let now = Instant::now();
        if now >= next {
            println!(
                "live: {:>4} s  state {}  messages {}",
                t0.elapsed().as_secs(),
                state_name(&st),
                sink.messages.load(Ordering::Relaxed)
            );
            next = now + Duration::from_secs(a.every.max(1));
        }
        if now >= deadline {
            break None;
        }
    };
    let close = conn.close();
    sink.finish();
    let ev = sink
        .events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for e in &ev[seen_events..] {
        println!("live: {e}");
    }
    println!(
        "live: done after {} s: {} messages, {} payload bytes, {} write errors; raw log {}",
        t0.elapsed().as_secs(),
        sink.messages.load(Ordering::Relaxed),
        sink.bytes.load(Ordering::Relaxed),
        sink.errors.load(Ordering::Relaxed),
        out.strip_prefix(&root).unwrap_or(&out).display()
    );
    if a.trades {
        println!(
            "live: trades: {} messages; level-2 fields on the wire (dropped, not kept): \
             buyer {}, seller {}; unreadable {}",
            sink.trade_messages.load(Ordering::Relaxed),
            sink.level2_buyer.load(Ordering::Relaxed),
            sink.level2_seller.load(Ordering::Relaxed),
            sink.unreadable_trades.load(Ordering::Relaxed)
        );
    }
    if let Some(r) = halted {
        return Err(format!(
            "live: HALTED ({r}); not retrying. Tell Arkin before any further live attempt."
        ));
    }
    close.map_err(|e| e.to_string())
}

fn state_name(s: &ConnState) -> String {
    match s {
        ConnState::Starting => "starting".into(),
        ConnState::Connecting { attempt } => format!("connecting#{attempt}"),
        ConnState::Connected { session } => format!("connected#{session}"),
        ConnState::BackingOff { failures } => format!("backoff({failures})"),
        ConnState::Halted(r) => format!("halted({r})"),
        ConnState::Closed => "closed".into(),
    }
}

/// Reads every record of a raw log (a truncated last record ends the read).
pub(crate) fn read_all(path: &Path) -> Result<(Vec<RawRecord>, Option<String>), String> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut r = RawReader::new(BufReader::new(f)).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    loop {
        match r.next_record() {
            Ok(Some(rec)) => out.push(rec),
            Ok(None) => return Ok((out, None)),
            Err(e) => return Ok((out, Some(e.to_string()))),
        }
    }
}
