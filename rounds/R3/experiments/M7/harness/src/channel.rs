//! The protocol channel: opaque byte messages, one direction per instance, with seeded
//! delay/jitter/reorder/duplication and disconnect windows (messages sent while
//! disconnected are held and delivered after reconnection, possibly twice). Counts
//! bytes: raw, deflate per message (no shared context) and deflate of the whole
//! direction's stream (messages joined with `\n`).

use std::io::Write;

use flate2::write::DeflateEncoder;
use flate2::Compression;

use crate::rng::Rng;
use crate::spec::{ms, ChannelSpec, VTime, Window};

/// Byte counters of one direction.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChanStats {
    /// Messages sent by the adapter.
    pub msgs: u64,
    /// Raw bytes sent.
    pub raw: u64,
    /// Sum of per-message deflate sizes.
    pub deflate_msg: u64,
    /// Deflate size of the whole stream (set by [`Channel::finish`]).
    pub deflate_stream: u64,
    /// Deliveries (including duplicates).
    pub delivered: u64,
}

/// Deflate size of bytes (default level).
#[must_use]
pub fn deflate_len(bytes: &[u8]) -> u64 {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
    e.write_all(bytes).expect("in-memory write");
    e.finish().expect("in-memory finish").len() as u64
}

/// One direction of the channel.
#[derive(Debug)]
pub struct Channel {
    spec: ChannelSpec,
    disconnects: Vec<Window>,
    rng: Rng,
    last: VTime,
    stream: Vec<u8>,
    /// Counters.
    pub stats: ChanStats,
}

impl Channel {
    /// A channel direction.
    #[must_use]
    pub fn new(spec: ChannelSpec, disconnects: Vec<Window>, rng: Rng) -> Self {
        Self {
            spec,
            disconnects,
            rng,
            last: -1,
            stream: Vec::new(),
            stats: ChanStats::default(),
        }
    }

    /// Sends a message at `now`; returns its deliveries `(time, bytes)`.
    pub fn send(&mut self, now: VTime, msg: Vec<u8>) -> Vec<(VTime, Vec<u8>)> {
        self.stats.msgs += 1;
        self.stats.raw += msg.len() as u64;
        self.stats.deflate_msg += deflate_len(&msg);
        self.stream.extend_from_slice(&msg);
        self.stream.push(b'\n');
        let mut out = Vec::new();
        let d = self.spec.delay.sample(&mut self.rng);
        if let Some(w) = self.disconnects.iter().find(|w| w.contains(now)) {
            let t = (w.end() + d).max(self.last + 1);
            self.last = t;
            if self.rng.chance(self.spec.dup_on_reconnect_p) {
                out.push((t + 1 + ms(self.rng.range_f64(0.0, 50.0)), msg.clone()));
            }
            out.push((t, msg));
        } else {
            let t = if self.rng.chance(self.spec.reorder_p) {
                now + d + ms(self.rng.range_f64(0.0, self.spec.reorder_ms))
            } else {
                let t = (now + d).max(self.last + 1);
                self.last = t;
                t
            };
            if self.rng.chance(self.spec.dup_p) {
                out.push((
                    t + 1 + ms(self.rng.range_f64(0.0, self.spec.dup_ms)),
                    msg.clone(),
                ));
            }
            out.push((t, msg));
        }
        self.stats.delivered += out.len() as u64;
        out
    }

    /// Computes the stream deflate size.
    pub fn finish(&mut self) {
        self.stats.deflate_stream = if self.stream.is_empty() {
            0
        } else {
            deflate_len(&self.stream)
        };
    }
}
