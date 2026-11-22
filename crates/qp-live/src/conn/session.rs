//! One WebSocket + MQTT session of the worker and its wire I/O (split out of `conn.rs`
//! for size).

use super::*;

impl Worker {
    pub(super) fn session(&mut self, token: &AccessToken) -> SessionEnd {
        let mut ws = match open_websocket(
            &self.url,
            &self.settings.ws_subprotocol,
            self.settings.connect_timeout,
        ) {
            Ok(ws) => ws,
            Err(e) => {
                return SessionEnd::Lost {
                    reason: DisconnectReason::ConnectFailed(Masked::new(&self.redactor, &e)),
                    stable: false,
                }
            }
        };
        let mut connect = Connect::new(
            self.settings.protocol,
            &self.client_id,
            self.settings.keepalive_secs,
        );
        connect.username = Some("JWT".to_string());
        connect.password = Some(token.expose().as_bytes().to_vec());
        if let Err(r) = self.send(&mut ws, &Packet::Connect(connect)) {
            self.close_ws(&mut ws);
            return SessionEnd::Lost {
                reason: r,
                stable: false,
            };
        }
        let mut dec = Decoder::new();
        let deadline = Instant::now() + self.settings.connect_timeout;
        let code = loop {
            if Instant::now() >= deadline {
                self.close_ws(&mut ws);
                return SessionEnd::Lost {
                    reason: DisconnectReason::ConnackTimeout,
                    stable: false,
                };
            }
            match self.poll_read(&mut ws, &mut dec) {
                Err(reason) => {
                    self.close_ws(&mut ws);
                    return SessionEnd::Lost {
                        reason,
                        stable: false,
                    };
                }
                Ok(ps) => {
                    if let Some(code) = ps.iter().find_map(|p| match p {
                        Packet::ConnAck { code, .. } => Some(*code),
                        _ => None,
                    }) {
                        break code;
                    }
                }
            }
        };
        match code {
            0 => {}
            4 | 5 => {
                self.close_ws(&mut ws);
                return SessionEnd::AuthRefused(code);
            }
            1 | 2 => {
                self.close_ws(&mut ws);
                return SessionEnd::Halt(HaltReason::ProtocolRejected { code });
            }
            other => {
                self.close_ws(&mut ws);
                return SessionEnd::Lost {
                    reason: DisconnectReason::ConnackRefused(other),
                    stable: false,
                };
            }
        }
        self.auth_retry_used = false;
        self.session += 1;
        let session = self.session;
        self.shared.sessions.fetch_add(1, Ordering::Relaxed);
        let _ = ws
            .get_ref()
            .tcp()
            .set_read_timeout(Some(self.opts.poll_interval));
        self.shared.set(ConnState::Connected { session });
        self.event(LiveEvent::Connected { session });

        let connected_at = Instant::now();
        let stable_after = self.opts.backoff.stable_after;
        let keepalive = Duration::from_secs(u64::from(self.settings.keepalive_secs.max(1)));
        let ping_every = keepalive * 3 / 4;
        let mut queue: VecDeque<Vec<String>> = self.batches(self.topics.iter().cloned()).into();
        let mut inflight: Option<(u16, Vec<String>)> = None;
        let mut next_pid: u16 = 0;
        let mut pid = || {
            next_pid = next_pid.wrapping_add(1).max(1);
            next_pid
        };
        let mut last_sent = Instant::now();
        let mut ping_sent: Option<Instant> = None;

        loop {
            let lost = |reason| SessionEnd::Lost {
                reason,
                stable: connected_at.elapsed() >= stable_after,
            };
            // Commands.
            loop {
                match self.rx.try_recv() {
                    Ok(Command::Stop) | Err(TryRecvError::Disconnected) => {
                        self.close_ws(&mut ws);
                        return SessionEnd::Stopped;
                    }
                    Ok(Command::Subscribe(t)) => {
                        let new = self.add_topics(t);
                        queue.extend(self.batches(new));
                    }
                    Ok(Command::Unsubscribe(t)) => {
                        let gone = self.remove_topics(t);
                        if !gone.is_empty() {
                            let p = Packet::Unsubscribe {
                                packet_id: pid(),
                                topics: gone.clone(),
                            };
                            if let Err(r) = self.send(&mut ws, &p) {
                                self.close_ws(&mut ws);
                                return lost(r);
                            }
                            last_sent = Instant::now();
                            let topics = gone.iter().map(|t| self.mask(t)).collect();
                            self.event(LiveEvent::Unsubscribed { topics });
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            // One SUBSCRIBE in flight at a time.
            if inflight.is_none() {
                if let Some(batch) = queue.pop_front() {
                    let batch: Vec<String> = batch
                        .into_iter()
                        .filter(|t| self.topics.contains(t))
                        .collect();
                    if !batch.is_empty() {
                        let id = pid();
                        let p = Packet::Subscribe {
                            packet_id: id,
                            topics: batch.iter().map(|t| (t.clone(), 0)).collect(),
                        };
                        if let Err(r) = self.send(&mut ws, &p) {
                            self.close_ws(&mut ws);
                            return lost(r);
                        }
                        last_sent = Instant::now();
                        inflight = Some((id, batch));
                    }
                }
            }
            // Token renewal on an open connection (opt-in). The new token is obtained first
            // (rate limited by the auth client); only a token different from the current one
            // closes the connection, so a refused / rate-limited request never causes a
            // reconnect loop. Old connection closed before the new one is opened.
            if self.opts.renewal == TokenRenewal::ReconnectBeforeExpiry
                && token.remaining() <= self.auth.policy().refresh_before
            {
                if let Ok(fresh) = self.auth.token() {
                    if !fresh.same_value(token) {
                        self.close_ws(&mut ws);
                        return SessionEnd::Renew(fresh);
                    }
                }
            }
            // Keepalive.
            match ping_sent {
                Some(at) if at.elapsed() > keepalive => {
                    self.close_ws(&mut ws);
                    return lost(DisconnectReason::KeepaliveTimeout);
                }
                None if last_sent.elapsed() >= ping_every => {
                    if let Err(r) = self.send(&mut ws, &Packet::PingReq) {
                        self.close_ws(&mut ws);
                        return lost(r);
                    }
                    last_sent = Instant::now();
                    ping_sent = Some(last_sent);
                }
                _ => {}
            }
            // Read.
            let packets = match self.poll_read(&mut ws, &mut dec) {
                Ok(ps) => ps,
                Err(reason) => {
                    self.close_ws(&mut ws);
                    return lost(reason);
                }
            };
            for p in packets {
                match p {
                    Packet::Publish {
                        topic,
                        payload,
                        qos,
                        packet_id,
                        ..
                    } => {
                        if qos > 0 {
                            if let Some(id) = packet_id {
                                if let Err(r) = self.send(&mut ws, &Packet::PubAck(id)) {
                                    self.close_ws(&mut ws);
                                    return lost(r);
                                }
                            }
                        }
                        self.seq += 1;
                        self.shared.messages.fetch_add(1, Ordering::Relaxed);
                        self.sink.on_message(&LiveMessage {
                            endpoint: &self.key,
                            seq: self.seq,
                            session,
                            topic: &topic,
                            payload: &payload,
                            received_at_us: now_us(),
                        });
                        let notice = self
                            .opts
                            .classifier
                            .as_ref()
                            .and_then(|c| c(&topic, &payload));
                        match notice {
                            Some(Notice::MultiConnect) => {
                                self.close_ws(&mut ws);
                                return SessionEnd::Halt(HaltReason::MultiConnect);
                            }
                            Some(Notice::Other(text)) => self.event(LiveEvent::Notice {
                                topic: self.mask(&topic),
                                text: Masked::new(&self.redactor, &text),
                            }),
                            None => {}
                        }
                    }
                    Packet::SubAck { packet_id, codes } => {
                        let Some((id, batch)) = inflight.take() else {
                            continue;
                        };
                        if id != packet_id {
                            inflight = Some((id, batch));
                            continue;
                        }
                        for (i, t) in batch.iter().enumerate() {
                            let code = codes.get(i).copied().unwrap_or(SUBACK_FAILURE);
                            if code == SUBACK_FAILURE {
                                // Not re-subscribed on reconnect unless asked again.
                                self.topics.remove(t);
                                self.event(LiveEvent::SubscribeRejected {
                                    topic: self.mask(t),
                                    code,
                                });
                            } else {
                                self.event(LiveEvent::Subscribed {
                                    topic: self.mask(t),
                                });
                            }
                        }
                    }
                    Packet::PingResp => ping_sent = None,
                    Packet::Disconnect => {
                        self.close_ws(&mut ws);
                        return lost(DisconnectReason::ServerClosed);
                    }
                    _ => {}
                }
            }
        }
    }

    fn send(&self, ws: &mut Ws, p: &Packet) -> Result<(), DisconnectReason> {
        ws.send(Message::Binary(p.encode().into()))
            .map_err(|e| self.ws_error(e))
    }

    fn ws_error(&self, e: tungstenite::Error) -> DisconnectReason {
        use std::io::ErrorKind as K;
        match e {
            tungstenite::Error::ConnectionClosed
            | tungstenite::Error::AlreadyClosed
            | tungstenite::Error::Protocol(
                tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
            ) => DisconnectReason::ServerClosed,
            tungstenite::Error::Io(io)
                if matches!(
                    io.kind(),
                    K::UnexpectedEof | K::ConnectionReset | K::ConnectionAborted | K::BrokenPipe
                ) =>
            {
                DisconnectReason::ServerClosed
            }
            tungstenite::Error::Protocol(p) => {
                DisconnectReason::Protocol(Masked::new(&self.redactor, &p.to_string()))
            }
            other => DisconnectReason::Io(Masked::new(&self.redactor, &other.to_string())),
        }
    }

    /// One read with the socket timeout; returns the complete packets received (may be none).
    fn poll_read(&self, ws: &mut Ws, dec: &mut Decoder) -> Result<Vec<Packet>, DisconnectReason> {
        match ws.read() {
            Ok(Message::Binary(b)) => {
                dec.push(&b);
                let mut out = Vec::new();
                loop {
                    match dec.next_packet() {
                        Ok(Some(p)) => out.push(p),
                        Ok(None) => return Ok(out),
                        Err(e) => {
                            return Err(DisconnectReason::Protocol(Masked::new(
                                &self.redactor,
                                &e.to_string(),
                            )))
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => Err(DisconnectReason::ServerClosed),
            Ok(_) => Ok(Vec::new()),
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(Vec::new())
            }
            Err(e) => Err(self.ws_error(e)),
        }
    }

    /// MQTT DISCONNECT, WebSocket close handshake (bounded wait), TCP shutdown. After this
    /// returns the socket is gone, so a following connect never overlaps it.
    fn close_ws(&self, ws: &mut Ws) {
        let _ = ws.send(Message::Binary(Packet::Disconnect.encode().into()));
        let _ = ws.close(None);
        let _ = ws
            .get_ref()
            .tcp()
            .set_read_timeout(Some(Duration::from_millis(50)));
        let deadline = Instant::now() + self.opts.close_timeout;
        while Instant::now() < deadline {
            match ws.read() {
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    let _ = ws.flush();
                }
                Err(_) => break,
            }
        }
        ws.get_ref().shutdown();
    }
}
