//! One broker connection: the WebSocket + MQTT server side (split out of `mock.rs` for
//! size).

use super::*;

#[allow(clippy::unwrap_used, clippy::too_many_lines)]
pub(super) fn serve_connection(
    s: &BrokerShared,
    stream: TcpStream,
    rx: Receiver<BrokerCmd>,
    mut gauge: Gauge,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut chosen: Option<String> = None;
    let mut req_path = String::new();
    let callback = |req: &Request, mut resp: Response| {
        req_path = req.uri().path().to_string();
        if let Some(p) = req.headers().get("Sec-WebSocket-Protocol") {
            let first = p
                .to_str()
                .unwrap_or("")
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if let Ok(v) = HeaderValue::from_str(&first) {
                resp.headers_mut().insert("Sec-WebSocket-Protocol", v);
            }
            chosen = Some(first);
        }
        Ok(resp)
    };
    let mut ws = match tungstenite::accept_hdr(stream, callback) {
        Ok(ws) => ws,
        Err(_) => return,
    };
    gauge.set_path(&req_path);
    let path_key = req_path.trim_matches('/').to_string();
    let _ = ws
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(20)));
    let mut dec = Decoder::new();
    let mut subscribed: BTreeSet<String> = BTreeSet::new();
    let send = |ws: &mut tungstenite::WebSocket<TcpStream>, p: &Packet| {
        ws.send(Message::Binary(p.encode().into())).is_ok()
    };
    loop {
        if s.stop.load(Ordering::SeqCst) {
            gauge.close();
            let _ = ws.get_ref().shutdown(Shutdown::Both);
            return;
        }
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                BrokerCmd::Publish {
                    topic,
                    payload,
                    only_subscribed,
                } => {
                    if !only_subscribed || subscribed.contains(&topic) {
                        send(
                            &mut ws,
                            &Packet::Publish {
                                topic,
                                payload,
                                qos: 0,
                                retain: false,
                                dup: false,
                                packet_id: None,
                            },
                        );
                    }
                }
                BrokerCmd::Drop(p) if p.as_ref().is_none_or(|p| *p == path_key) => {
                    gauge.close();
                    let _ = ws.get_ref().shutdown(Shutdown::Both);
                    return;
                }
                BrokerCmd::Drop(_) => {}
            }
        }
        match ws.read() {
            Ok(Message::Binary(b)) => {
                dec.push(&b);
                while let Ok(Some(p)) = dec.next_packet() {
                    match p {
                        Packet::Connect(c) => {
                            let code = s.script.lock().unwrap().connack_codes.pop_front();
                            let code = code.unwrap_or(0);
                            {
                                let mut st = s.stats.lock().unwrap();
                                st.connects.push(ConnectRecord {
                                    ws_subprotocol: chosen.clone(),
                                    protocol_name: c.protocol_name,
                                    protocol_level: c.protocol_level,
                                    client_id: c.client_id,
                                    username: c.username,
                                    password: c
                                        .password
                                        .map(|p| String::from_utf8_lossy(&p).into_owned()),
                                    keepalive: c.keepalive,
                                    clean_session: c.clean_session,
                                });
                                if code == 0 {
                                    st.sessions_accepted += 1;
                                }
                            }
                            if code != 0 {
                                gauge.close();
                                send(
                                    &mut ws,
                                    &Packet::ConnAck {
                                        session_present: false,
                                        code,
                                    },
                                );
                                let _ = ws.get_ref().shutdown(Shutdown::Both);
                                return;
                            }
                            send(
                                &mut ws,
                                &Packet::ConnAck {
                                    session_present: false,
                                    code: 0,
                                },
                            );
                        }
                        Packet::Subscribe { packet_id, topics } => {
                            let names: Vec<String> =
                                topics.iter().map(|(t, _)| t.clone()).collect();
                            {
                                let mut st = s.stats.lock().unwrap();
                                st.subscribe_packets.push(names.clone());
                                st.by_path
                                    .entry(path_key.clone())
                                    .or_default()
                                    .subscribed_topics
                                    .extend(names.iter().cloned());
                            }
                            let (codes, initial) = {
                                let sc = s.script.lock().unwrap();
                                let codes: Vec<u8> = names
                                    .iter()
                                    .map(|t| sc.suback_codes.get(t).copied().unwrap_or(0))
                                    .collect();
                                let mut initial = Vec::new();
                                for (t, c) in names.iter().zip(&codes) {
                                    if *c != crate::mqtt::SUBACK_FAILURE {
                                        for m in sc.initial.get(t).into_iter().flatten() {
                                            initial.push((t.clone(), m.clone()));
                                        }
                                    }
                                }
                                (codes, initial)
                            };
                            for (t, c) in names.iter().zip(&codes) {
                                if *c != crate::mqtt::SUBACK_FAILURE {
                                    subscribed.insert(t.clone());
                                }
                            }
                            send(&mut ws, &Packet::SubAck { packet_id, codes });
                            for (topic, payload) in initial {
                                send(
                                    &mut ws,
                                    &Packet::Publish {
                                        topic,
                                        payload,
                                        qos: 0,
                                        retain: false,
                                        dup: false,
                                        packet_id: None,
                                    },
                                );
                            }
                        }
                        Packet::Unsubscribe { packet_id, topics } => {
                            for t in &topics {
                                subscribed.remove(t);
                            }
                            s.stats.lock().unwrap().unsubscribe_packets.push(topics);
                            send(&mut ws, &Packet::UnsubAck(packet_id));
                        }
                        Packet::PingReq => {
                            s.stats.lock().unwrap().pings += 1;
                            if !s.script.lock().unwrap().ignore_pings {
                                send(&mut ws, &Packet::PingResp);
                            }
                        }
                        Packet::Disconnect => {
                            s.stats.lock().unwrap().disconnects += 1;
                            gauge.close();
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => {
                gauge.close();
                // tungstenite queued the close reply; flush it, then drop.
                let _ = ws.flush();
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => {
                gauge.close();
                return;
            }
        }
    }
}
