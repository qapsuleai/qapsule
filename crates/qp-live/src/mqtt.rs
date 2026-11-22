//! Minimal MQTT 3.1 / 3.1.1 codec (both directions, so the mock broker can share it).
//!
//! Packets: CONNECT, CONNACK, PUBLISH, PUBACK, SUBSCRIBE, SUBACK, UNSUBSCRIBE, UNSUBACK,
//! PINGREQ, PINGRESP, DISCONNECT. QoS 0 is what this crate subscribes with; a QoS 1
//! PUBLISH is decoded (with its packet id) so it can be acknowledged.

use std::fmt;

use crate::config::MqttProtocol;

/// SUBACK return code meaning "refused".
pub const SUBACK_FAILURE: u8 = 0x80;

/// Upper bound for one packet (16 MiB).
pub const MAX_PACKET: usize = 16 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct Connect {
    pub protocol_name: String,
    pub protocol_level: u8,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<Vec<u8>>,
    pub keepalive: u16,
    pub clean_session: bool,
}

impl Connect {
    pub fn new(protocol: MqttProtocol, client_id: &str, keepalive: u16) -> Self {
        Self {
            protocol_name: protocol.name().to_string(),
            protocol_level: protocol.level(),
            client_id: client_id.to_string(),
            username: None,
            password: None,
            keepalive,
            clean_session: true,
        }
    }
}

impl fmt::Debug for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connect")
            .field("protocol_name", &self.protocol_name)
            .field("protocol_level", &self.protocol_level)
            .field("client_id", &self.client_id)
            .field("username", &self.username.as_ref().map(|_| "[redacted]"))
            .field("password", &self.password.as_ref().map(|_| "[redacted]"))
            .field("keepalive", &self.keepalive)
            .field("clean_session", &self.clean_session)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Connect(Connect),
    ConnAck {
        session_present: bool,
        code: u8,
    },
    Publish {
        topic: String,
        payload: Vec<u8>,
        qos: u8,
        retain: bool,
        dup: bool,
        packet_id: Option<u16>,
    },
    PubAck(u16),
    Subscribe {
        packet_id: u16,
        topics: Vec<(String, u8)>,
    },
    SubAck {
        packet_id: u16,
        codes: Vec<u8>,
    },
    Unsubscribe {
        packet_id: u16,
        topics: Vec<String>,
    },
    UnsubAck(u16),
    PingReq,
    PingResp,
    Disconnect,
    /// A packet type this codec does not handle (type nibble); skipped.
    Unknown(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Malformed(&'static str),
    TooLarge(usize),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Malformed(w) => write!(f, "malformed MQTT packet: {w}"),
            DecodeError::TooLarge(n) => write!(f, "MQTT packet too large: {n} bytes"),
        }
    }
}

impl std::error::Error for DecodeError {}

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    put_u16(out, b.len().min(u16::MAX as usize) as u16);
    out.extend_from_slice(&b[..b.len().min(u16::MAX as usize)]);
}

fn put_len(out: &mut Vec<u8>, mut len: usize) {
    loop {
        let mut byte = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if len == 0 {
            break;
        }
    }
}

fn frame(first: u8, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 5);
    out.push(first);
    put_len(&mut out, body.len());
    out.extend_from_slice(body);
    out
}

impl Packet {
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        match self {
            Packet::Connect(c) => {
                put_bytes(&mut b, c.protocol_name.as_bytes());
                b.push(c.protocol_level);
                let mut flags = 0u8;
                if c.clean_session {
                    flags |= 0x02;
                }
                if c.username.is_some() {
                    flags |= 0x80;
                }
                if c.password.is_some() {
                    flags |= 0x40;
                }
                b.push(flags);
                put_u16(&mut b, c.keepalive);
                put_bytes(&mut b, c.client_id.as_bytes());
                if let Some(u) = &c.username {
                    put_bytes(&mut b, u.as_bytes());
                }
                if let Some(p) = &c.password {
                    put_bytes(&mut b, p);
                }
                frame(0x10, &b)
            }
            Packet::ConnAck {
                session_present,
                code,
            } => frame(0x20, &[u8::from(*session_present), *code]),
            Packet::Publish {
                topic,
                payload,
                qos,
                retain,
                dup,
                packet_id,
            } => {
                put_bytes(&mut b, topic.as_bytes());
                if *qos > 0 {
                    put_u16(&mut b, packet_id.unwrap_or(1));
                }
                b.extend_from_slice(payload);
                let first = 0x30 | (u8::from(*dup) << 3) | ((qos & 0x03) << 1) | u8::from(*retain);
                frame(first, &b)
            }
            Packet::PubAck(id) => frame(0x40, &id.to_be_bytes()),
            Packet::Subscribe { packet_id, topics } => {
                put_u16(&mut b, *packet_id);
                for (t, q) in topics {
                    put_bytes(&mut b, t.as_bytes());
                    b.push(*q);
                }
                frame(0x82, &b)
            }
            Packet::SubAck { packet_id, codes } => {
                put_u16(&mut b, *packet_id);
                b.extend_from_slice(codes);
                frame(0x90, &b)
            }
            Packet::Unsubscribe { packet_id, topics } => {
                put_u16(&mut b, *packet_id);
                for t in topics {
                    put_bytes(&mut b, t.as_bytes());
                }
                frame(0xA2, &b)
            }
            Packet::UnsubAck(id) => frame(0xB0, &id.to_be_bytes()),
            Packet::PingReq => vec![0xC0, 0],
            Packet::PingResp => vec![0xD0, 0],
            Packet::Disconnect => vec![0xE0, 0],
            Packet::Unknown(t) => vec![t << 4, 0],
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Result<u8, DecodeError> {
        let v = *self
            .b
            .get(self.pos)
            .ok_or(DecodeError::Malformed("truncated"))?;
        self.pos += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_be_bytes([self.u8()?, self.u8()?]))
    }

    fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let n = self.u16()? as usize;
        let end = self.pos + n;
        let s = self
            .b
            .get(self.pos..end)
            .ok_or(DecodeError::Malformed("truncated string"))?;
        self.pos = end;
        Ok(s)
    }

    fn string(&mut self) -> Result<String, DecodeError> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| DecodeError::Malformed("utf-8"))
    }

    fn rest(&mut self) -> &'a [u8] {
        let r = &self.b[self.pos.min(self.b.len())..];
        self.pos = self.b.len();
        r
    }

    fn done(&self) -> bool {
        self.pos >= self.b.len()
    }
}

#[allow(clippy::too_many_lines)]
fn decode_body(first: u8, body: &[u8]) -> Result<Packet, DecodeError> {
    let mut r = Reader { b: body, pos: 0 };
    let kind = first >> 4;
    Ok(match kind {
        1 => {
            let protocol_name = r.string()?;
            let protocol_level = r.u8()?;
            let flags = r.u8()?;
            let keepalive = r.u16()?;
            let client_id = r.string()?;
            if flags & 0x04 != 0 {
                // Will topic + message: not used here, skipped.
                r.bytes()?;
                r.bytes()?;
            }
            let username = if flags & 0x80 != 0 {
                Some(r.string()?)
            } else {
                None
            };
            let password = if flags & 0x40 != 0 {
                Some(r.bytes()?.to_vec())
            } else {
                None
            };
            Packet::Connect(Connect {
                protocol_name,
                protocol_level,
                client_id,
                username,
                password,
                keepalive,
                clean_session: flags & 0x02 != 0,
            })
        }
        2 => {
            let ack = r.u8()?;
            Packet::ConnAck {
                session_present: ack & 1 != 0,
                code: r.u8()?,
            }
        }
        3 => {
            let qos = (first >> 1) & 0x03;
            let topic = r.string()?;
            let packet_id = if qos > 0 { Some(r.u16()?) } else { None };
            Packet::Publish {
                topic,
                payload: r.rest().to_vec(),
                qos,
                retain: first & 1 != 0,
                dup: first & 0x08 != 0,
                packet_id,
            }
        }
        4 => Packet::PubAck(r.u16()?),
        8 => {
            let packet_id = r.u16()?;
            let mut topics = Vec::new();
            while !r.done() {
                let t = r.string()?;
                topics.push((t, r.u8()?));
            }
            Packet::Subscribe { packet_id, topics }
        }
        9 => {
            let packet_id = r.u16()?;
            Packet::SubAck {
                packet_id,
                codes: r.rest().to_vec(),
            }
        }
        10 => {
            let packet_id = r.u16()?;
            let mut topics = Vec::new();
            while !r.done() {
                topics.push(r.string()?);
            }
            Packet::Unsubscribe { packet_id, topics }
        }
        11 => Packet::UnsubAck(r.u16()?),
        12 => Packet::PingReq,
        13 => Packet::PingResp,
        14 => Packet::Disconnect,
        other => Packet::Unknown(other),
    })
}

/// Stream decoder: WebSocket messages may carry partial or several MQTT packets.
#[derive(Default)]
pub struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Next complete packet, if any.
    pub fn next_packet(&mut self) -> Result<Option<Packet>, DecodeError> {
        if self.buf.len() < 2 {
            return Ok(None);
        }
        let mut len = 0usize;
        let mut mult = 1usize;
        let mut i = 1;
        loop {
            let Some(&byte) = self.buf.get(i) else {
                return Ok(None);
            };
            len += (byte & 0x7F) as usize * mult;
            i += 1;
            if byte & 0x80 == 0 {
                break;
            }
            if i > 4 {
                return Err(DecodeError::Malformed("remaining length"));
            }
            mult *= 128;
        }
        if len > MAX_PACKET {
            return Err(DecodeError::TooLarge(len));
        }
        if self.buf.len() < i + len {
            return Ok(None);
        }
        let first = self.buf[0];
        let packet = decode_body(first, &self.buf[i..i + len]);
        self.buf.drain(..i + len);
        packet.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(p: Packet) {
        let mut d = Decoder::new();
        d.push(&p.encode());
        assert_eq!(d.next_packet().unwrap(), Some(p));
        assert_eq!(d.next_packet().unwrap(), None);
    }

    #[test]
    fn all_packets_roundtrip() {
        let mut c = Connect::new(MqttProtocol::V31, "qp-1234abcd-42", 30);
        c.username = Some("JWT".into());
        c.password = Some(b"pw".to_vec());
        roundtrip(Packet::Connect(c));
        roundtrip(Packet::Connect(Connect::new(MqttProtocol::V311, "x", 0)));
        roundtrip(Packet::ConnAck {
            session_present: false,
            code: 5,
        });
        roundtrip(Packet::Publish {
            topic: "mx/symbol/ABC".into(),
            payload: vec![0, 1, 2, 255],
            qos: 0,
            retain: false,
            dup: false,
            packet_id: None,
        });
        roundtrip(Packet::Publish {
            topic: "t".into(),
            payload: vec![9; 300],
            qos: 1,
            retain: true,
            dup: true,
            packet_id: Some(7),
        });
        roundtrip(Packet::PubAck(7));
        roundtrip(Packet::Subscribe {
            packet_id: 3,
            topics: vec![("a/b".into(), 0), ("c".into(), 0)],
        });
        roundtrip(Packet::SubAck {
            packet_id: 3,
            codes: vec![0, SUBACK_FAILURE],
        });
        roundtrip(Packet::Unsubscribe {
            packet_id: 4,
            topics: vec!["a/b".into()],
        });
        roundtrip(Packet::UnsubAck(4));
        roundtrip(Packet::PingReq);
        roundtrip(Packet::PingResp);
        roundtrip(Packet::Disconnect);
    }

    #[test]
    fn connect_wire_bytes_match_mqtt31() {
        let mut c = Connect::new(MqttProtocol::V31, "id", 30);
        c.username = Some("JWT".into());
        c.password = Some(b"p".to_vec());
        let bytes = Packet::Connect(c).encode();
        assert_eq!(bytes[0], 0x10);
        assert_eq!(&bytes[2..10], b"\x00\x06MQIsdp");
        assert_eq!(bytes[10], 3);
        assert_eq!(bytes[11], 0xC2); // user + password + clean session
        assert_eq!(&bytes[12..14], &[0, 30]);
    }

    #[test]
    fn decoder_handles_split_and_joined_input() {
        let a = Packet::PingResp.encode();
        let big = Packet::Publish {
            topic: "x".into(),
            payload: vec![1; 20_000],
            qos: 0,
            retain: false,
            dup: false,
            packet_id: None,
        };
        let b = big.encode();
        let mut all = a.clone();
        all.extend_from_slice(&b);
        let mut d = Decoder::new();
        for chunk in all.chunks(7) {
            d.push(chunk);
        }
        assert_eq!(d.next_packet().unwrap(), Some(Packet::PingResp));
        assert_eq!(d.next_packet().unwrap(), Some(big));
        let mut d = Decoder::new();
        d.push(&b[..3]);
        assert_eq!(d.next_packet().unwrap(), None);
    }

    #[test]
    fn rejects_bad_length() {
        let mut d = Decoder::new();
        d.push(&[0x30, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]);
        assert!(d.next_packet().is_err());
    }
}
