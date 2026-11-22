//! The connection thread and its sink ([`spawn_connection`], [`HubSink`]; RG1: split out of `livehost.rs`).

use super::*;

/// The connection's sink: the raw log (K60) and a copy to the hub.
pub struct HubSink {
    raw: RawSink,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
    source: FeedSource,
}

impl HubSink {
    /// A sink writing `raw` and sending to the hub (the market endpoint).
    pub fn new(raw: RawSink, to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>) -> Self {
        Self {
            raw,
            to_hub,
            source: FeedSource::Market,
        }
    }

    /// (RP4) A sink for the trade endpoint: its records go to the hub as
    /// [`LiveIn::Trade`] (`qp demo --live --trades`).
    pub fn for_trades(raw: RawSink, to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>) -> Self {
        Self {
            raw,
            to_hub,
            source: FeedSource::Trade,
        }
    }

    fn wrap(&self, rec: RawRecord) -> LiveIn {
        match self.source {
            FeedSource::Market => LiveIn::Record(rec),
            FeedSource::Trade => LiveIn::Trade(rec),
        }
    }

    /// A local note (a deliberate drop) into the log and to the hub as a lost connection.
    pub fn local_drop(&self, note: &str) {
        let rec = RawRecord {
            t_us: now_us(),
            kind: RecordKind::ConnectionLost,
            topic: String::new(),
            payload: note.as_bytes().to_vec(),
        };
        self.raw.put_record(&rec);
        (self.to_hub)(self.wrap(rec));
    }

    /// Ends the raw log (the trade sanitizing note, if any) and flushes it.
    pub fn flush(&self) {
        self.raw.finish();
    }
}

impl LiveSink for HubSink {
    fn on_message(&self, m: &LiveMessage<'_>) {
        // The log and the hub get the same kept payload: a trade without buyer / seller.
        let kept = self.raw.take_message(m);
        (self.to_hub)(self.wrap(RawRecord::message(
            m.received_at_us,
            m.topic,
            kept.into_owned(),
        )));
    }

    fn on_event(&self, ep: &qp_live::EndpointKey, ev: &LiveEvent) {
        self.raw.on_event(ep, ev);
        let kind = match ev {
            LiveEvent::Connected { session } => {
                Some((RecordKind::ConnectionOpened, format!("session {session}")))
            }
            LiveEvent::Disconnected { .. } => {
                Some((RecordKind::ConnectionLost, "disconnected".to_owned()))
            }
            LiveEvent::Subscribed { .. } | LiveEvent::Connecting { .. } => None,
            other => {
                (self.to_hub)(LiveIn::Note(other.to_string()));
                None
            }
        };
        if let Some((kind, note)) = kind {
            (self.to_hub)(self.wrap(RawRecord {
                t_us: now_us(),
                kind,
                topic: String::new(),
                payload: note.into_bytes(),
            }));
        }
    }
}

/// A command to the connection thread.
#[derive(Debug)]
pub enum ConnCtl {
    /// Close (DISCONNECT), wait this long, open again.
    Drop(Duration),
    /// Close and end the thread (answer when closed).
    Stop(Sender<()>),
}

/// What the connection thread needs to open the connection.
pub struct ConnSpec {
    pub cfg: ConnConfig,
    pub auth: Arc<qp_live::AuthClient>,
    pub topics: Vec<String>,
    pub lock_dir: std::path::PathBuf,
    /// (RP4) The trade endpoint (`/trade`, its own lock) instead of the market endpoint.
    pub trade: bool,
}

impl ConnSpec {
    fn endpoint(&self) -> &qp_live::Endpoint {
        if self.trade {
            self.cfg.trade_endpoint()
        } else {
            self.cfg.market_endpoint()
        }
    }
}

/// Starts the connection thread: opens the one connection, serves `drop` and `stop`.
/// Returns its control channel.
///
/// # Errors
/// The first open failed (lock held elsewhere, halt marker, …).
pub fn spawn_connection(
    spec: ConnSpec,
    sink: Arc<HubSink>,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
) -> Result<Sender<ConnCtl>, String> {
    let open = |spec: &ConnSpec, sink: &Arc<HubSink>| {
        let mut opts = qp_live::ConnectOptions::new(spec.topics.clone());
        opts.classifier = Some(crate::live::classifier());
        opts.lock_dir = spec.lock_dir.clone();
        LiveConnection::open(
            &spec.cfg,
            spec.endpoint(),
            spec.auth.clone(),
            opts,
            sink.clone() as Arc<dyn LiveSink>,
        )
        .map_err(|e| e.to_string())
    };
    let first = open(&spec, &sink)?;
    let (tx, rx): (Sender<ConnCtl>, Receiver<ConnCtl>) = mpsc::channel();
    std::thread::spawn(move || {
        let mut conn = Some(first);
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(ConnCtl::Drop(wait)) => {
                    let Some(c) = conn.take() else { continue };
                    sink.local_drop("local drop");
                    let _ = c.close();
                    // Interruptible wait: a stop during it ends the thread.
                    match rx.recv_timeout(wait) {
                        Ok(ConnCtl::Stop(done)) => {
                            sink.flush();
                            let _ = done.send(());
                            return;
                        }
                        Ok(ConnCtl::Drop(_)) | Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    match open(&spec, &sink) {
                        Ok(c) => conn = Some(c),
                        Err(e) => to_hub(LiveIn::Halted(format!("reopen failed: {e}"))),
                    }
                }
                Ok(ConnCtl::Stop(done)) => {
                    if let Some(c) = conn.take() {
                        let _ = c.close();
                    }
                    sink.flush();
                    let _ = done.send(());
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(c) = &conn {
                        if let qp_live::ConnState::Halted(r) = c.state() {
                            to_hub(LiveIn::Halted(r.to_string()));
                            if let Some(c) = conn.take() {
                                let _ = c.close();
                            }
                        }
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    if let Some(c) = conn.take() {
                        let _ = c.close();
                    }
                    sink.flush();
                    return;
                }
            }
        }
    });
    Ok(tx)
}
