//! Raw message log: a binary, appendable record of everything the live
//! connector received, for replay through the same decoders.
//!
//! Raw logs contain real market data: they live only under `fixtures/local/`
//! and are never committed.
//!
//! # Format (version 1, all integers little-endian)
//!
//! ```text
//! file    = header record*
//! header  = magic "QPMXRAW\0" (8 bytes) | version u16 (= 1) | reserved u16 (= 0)
//! record  = body_len u32 | body | crc32 u32
//! body    = t_us i64 | kind u8 | topic_len u16 | topic (UTF-8) | payload_len u32 | payload
//! ```
//!
//! - `t_us`: arrival time on the host, Unix epoch microseconds (UTC). The
//!   writer does not read a clock; the caller passes the time.
//! - `kind`: see [`RecordKind`].
//! - `crc32`: CRC-32 (IEEE) of `body`.
//! - `body_len` = 15 + topic_len + payload_len.
//!
//! A writer that dies mid-record leaves a partial tail. [`RawReader`] reports
//! it as [`RawError::Truncated`] (clean EOF between records is the normal end)
//! and every record before it is intact; a damaged record is
//! [`RawError::Corrupt`]. Appending to an existing log: open the file in append
//! mode and use [`RawWriter::resume`] (no new header); truncate a partial tail
//! to [`RawReader::offset`] of the last good record first.

use std::io::{self, Read, Write};

/// File magic.
pub const MAGIC: [u8; 8] = *b"QPMXRAW\0";
/// Current format version.
pub const VERSION: u16 = 1;
/// Header length in bytes.
pub const HEADER_LEN: usize = 12;
/// Fixed part of a record body (`t_us` + `kind` + `topic_len` + `payload_len`).
const BODY_FIXED: usize = 8 + 1 + 2 + 4;
/// Largest payload accepted (writer and reader), as a guard against corrupt lengths.
pub const MAX_PAYLOAD: usize = 64 * 1024 * 1024;

/// What a record describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RecordKind {
    /// A received MQTT message: `topic` and `payload` as received.
    Message = 1,
    /// Connection opened. `topic`: endpoint label (never an address); payload empty.
    ConnectionOpened = 2,
    /// Connection lost or closed. Payload: optional UTF-8 reason.
    ConnectionLost = 3,
    /// Subscription acknowledged for `topic`.
    Subscribed = 4,
    /// Subscription rejected for `topic`. Payload: optional reason / return code bytes.
    SubscribeRejected = 5,
    /// Free-form note (UTF-8 payload), e.g. a run marker.
    Note = 6,
}

impl RecordKind {
    /// The kind for a byte code.
    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => RecordKind::Message,
            2 => RecordKind::ConnectionOpened,
            3 => RecordKind::ConnectionLost,
            4 => RecordKind::Subscribed,
            5 => RecordKind::SubscribeRejected,
            6 => RecordKind::Note,
            _ => return None,
        })
    }

    /// The byte code.
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// One record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRecord {
    /// Arrival time, Unix epoch microseconds (UTC).
    pub t_us: i64,
    /// Record kind.
    pub kind: RecordKind,
    /// Topic (may be empty for kinds that have none).
    pub topic: String,
    /// Payload bytes.
    pub payload: Vec<u8>,
}

impl RawRecord {
    /// A received message record.
    pub fn message(t_us: i64, topic: impl Into<String>, payload: impl Into<Vec<u8>>) -> Self {
        RawRecord {
            t_us,
            kind: RecordKind::Message,
            topic: topic.into(),
            payload: payload.into(),
        }
    }
}

/// Raw log error.
#[derive(Debug)]
pub enum RawError {
    /// I/O error from the underlying reader or writer.
    Io(io::Error),
    /// Not a raw log (bad magic).
    BadMagic,
    /// Unsupported format version.
    UnsupportedVersion(u16),
    /// The log ends inside a record (partially written tail).
    Truncated {
        /// Byte offset where the incomplete record starts.
        offset: u64,
    },
    /// A record failed validation (CRC, lengths, kind or UTF-8).
    Corrupt {
        /// Byte offset of the record.
        offset: u64,
        /// What was wrong.
        reason: &'static str,
    },
    /// The writer was given a topic or payload that does not fit the format.
    TooLarge,
}

impl std::fmt::Display for RawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawError::Io(e) => write!(f, "i/o: {e}"),
            RawError::BadMagic => write!(f, "not a qapsule matriks raw log (bad magic)"),
            RawError::UnsupportedVersion(v) => write!(f, "unsupported raw log version {v}"),
            RawError::Truncated { offset } => write!(f, "truncated record at byte {offset}"),
            RawError::Corrupt { offset, reason } => {
                write!(f, "corrupt record at byte {offset}: {reason}")
            }
            RawError::TooLarge => write!(f, "topic or payload too large for the raw log"),
        }
    }
}

impl std::error::Error for RawError {}

impl From<io::Error> for RawError {
    fn from(e: io::Error) -> Self {
        RawError::Io(e)
    }
}

/// CRC-32 (IEEE 802.3, reflected, poly 0xEDB88320).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Appends records to a raw log.
#[derive(Debug)]
pub struct RawWriter<W: Write> {
    inner: W,
    buf: Vec<u8>,
}

impl<W: Write> RawWriter<W> {
    /// Starts a new log: writes the header.
    pub fn new(mut inner: W) -> Result<Self, RawError> {
        let mut header = [0u8; HEADER_LEN];
        header[..8].copy_from_slice(&MAGIC);
        header[8..10].copy_from_slice(&VERSION.to_le_bytes());
        inner.write_all(&header)?;
        Ok(RawWriter {
            inner,
            buf: Vec::new(),
        })
    }

    /// Continues an existing log (header already present): writes no header.
    pub fn resume(inner: W) -> Self {
        RawWriter {
            inner,
            buf: Vec::new(),
        }
    }

    /// Writes one record with a single `write_all`.
    pub fn write(&mut self, rec: &RawRecord) -> Result<(), RawError> {
        self.write_parts(rec.t_us, rec.kind, &rec.topic, &rec.payload)
    }

    /// Writes one record from its parts.
    pub fn write_parts(
        &mut self,
        t_us: i64,
        kind: RecordKind,
        topic: &str,
        payload: &[u8],
    ) -> Result<(), RawError> {
        let topic_len = u16::try_from(topic.len()).map_err(|_| RawError::TooLarge)?;
        if payload.len() > MAX_PAYLOAD {
            return Err(RawError::TooLarge);
        }
        let body_len = BODY_FIXED + topic.len() + payload.len();
        self.buf.clear();
        self.buf.reserve(4 + body_len + 4);
        self.buf.extend_from_slice(&(body_len as u32).to_le_bytes());
        self.buf.extend_from_slice(&t_us.to_le_bytes());
        self.buf.push(kind.code());
        self.buf.extend_from_slice(&topic_len.to_le_bytes());
        self.buf.extend_from_slice(topic.as_bytes());
        self.buf
            .extend_from_slice(&(payload.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(payload);
        let crc = crc32(&self.buf[4..]);
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.inner.write_all(&self.buf)?;
        Ok(())
    }

    /// Flushes the underlying writer.
    pub fn flush(&mut self) -> Result<(), RawError> {
        self.inner.flush()?;
        Ok(())
    }

    /// Returns the underlying writer.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

/// Reads records from a raw log.
#[derive(Debug)]
pub struct RawReader<R: Read> {
    inner: R,
    offset: u64,
    done: bool,
}

/// Reads up to `buf.len()` bytes; returns how many were read before EOF.
fn read_full<R: Read>(r: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

impl<R: Read> RawReader<R> {
    /// Opens a log: reads and checks the header.
    pub fn new(mut inner: R) -> Result<Self, RawError> {
        let mut header = [0u8; HEADER_LEN];
        let n = read_full(&mut inner, &mut header)?;
        if n < 8 || header[..8] != MAGIC {
            return Err(RawError::BadMagic);
        }
        if n < HEADER_LEN {
            return Err(RawError::Truncated { offset: 0 });
        }
        let version = u16::from_le_bytes([header[8], header[9]]);
        if version != VERSION {
            return Err(RawError::UnsupportedVersion(version));
        }
        Ok(RawReader {
            inner,
            offset: HEADER_LEN as u64,
            done: false,
        })
    }

    /// Byte offset of the next record (= end of the last good record).
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Next record; `Ok(None)` at a clean end. After an error, always `Ok(None)`.
    pub fn next_record(&mut self) -> Result<Option<RawRecord>, RawError> {
        if self.done {
            return Ok(None);
        }
        let r = self.read_one();
        if !matches!(r, Ok(Some(_))) {
            self.done = true;
        }
        r
    }

    #[allow(clippy::expect_used)]
    fn read_one(&mut self) -> Result<Option<RawRecord>, RawError> {
        let at = self.offset;
        let corrupt = |reason| RawError::Corrupt { offset: at, reason };
        let mut len_buf = [0u8; 4];
        match read_full(&mut self.inner, &mut len_buf)? {
            0 => return Ok(None),
            4 => {}
            _ => return Err(RawError::Truncated { offset: at }),
        }
        let body_len = u32::from_le_bytes(len_buf) as usize;
        if !(BODY_FIXED..=BODY_FIXED + u16::MAX as usize + MAX_PAYLOAD).contains(&body_len) {
            return Err(corrupt("record length out of range"));
        }
        let mut rest = vec![0u8; body_len + 4];
        if read_full(&mut self.inner, &mut rest)? < rest.len() {
            return Err(RawError::Truncated { offset: at });
        }
        let (body, crc_bytes) = rest.split_at(body_len);
        let crc = u32::from_le_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);
        if crc32(body) != crc {
            return Err(corrupt("crc mismatch"));
        }
        let t_us = i64::from_le_bytes(body[0..8].try_into().expect("8 bytes"));
        let kind = RecordKind::from_code(body[8]).ok_or(corrupt("unknown record kind"))?;
        let topic_len = u16::from_le_bytes([body[9], body[10]]) as usize;
        let topic_end = 11 + topic_len;
        if topic_end + 4 > body_len {
            return Err(corrupt("topic length exceeds record"));
        }
        let topic = std::str::from_utf8(&body[11..topic_end])
            .map_err(|_| corrupt("topic is not utf-8"))?
            .to_string();
        let payload_len =
            u32::from_le_bytes(body[topic_end..topic_end + 4].try_into().expect("4 bytes"))
                as usize;
        if topic_end + 4 + payload_len != body_len {
            return Err(corrupt("payload length mismatch"));
        }
        let payload = body[topic_end + 4..].to_vec();
        self.offset += 4 + body_len as u64 + 4;
        Ok(Some(RawRecord {
            t_us,
            kind,
            topic,
            payload,
        }))
    }

    /// Reads every record. Returns the intact records and, if the log did not
    /// end cleanly, the error that stopped reading (e.g. a truncated tail).
    pub fn read_all(mut self) -> (Vec<RawRecord>, Option<RawError>) {
        let mut out = Vec::new();
        loop {
            match self.next_record() {
                Ok(Some(r)) => out.push(r),
                Ok(None) => return (out, None),
                Err(e) => return (out, Some(e)),
            }
        }
    }
}

impl<R: Read> Iterator for RawReader<R> {
    type Item = Result<RawRecord, RawError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }
}
