//! The data plane (RP2, K23; `docs/PROTOCOL.md` §18, `docs/CONCEPTS.md` "Veri düzlemi").
//!
//! Market data does not flow through the protocol. A document carries a **data binding**
//! (instrument, interval, window) instead of bars; each principal's binding is resolved
//! with their own data entitlement by a host-supplied source provider, and data then
//! arrives as field-level [`DataEvent`]s from the source in its own format.
//!
//! The types here are the vocabulary shared by the document (the binding inside
//! `SeriesSource::Bind`, the frozen [`EvidenceRef`] on facts), the protocol (binding
//! state, evidence checks, errors) and the host/test side (data events in transcripts).
//! Data events are **not** protocol messages: they never appear on a connection and never
//! enter the log.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Bar, TimestampMs};

/// Which bars a binding asks for.
///
/// Wire form: exactly one of `{"last": n}`, `{"from_t": t}`, `{"from_t": a, "to_t": b}`
/// (inclusive). Bar numbers of a bound series count from the first bar of the window as
/// resolved by the authority (its *anchor*, announced in the `binding` query); bars that
/// arrive later are appended, the window does not slide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Window {
    /// The last `n` bars at resolution time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<u32>,
    /// Bars with `t >= from_t` (epoch ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// With `from_t`: bars with `t <= to_t`; the window is closed (no live bars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
}

impl Window {
    /// The last `n` bars.
    #[must_use]
    pub fn last(n: u32) -> Self {
        Self {
            last: Some(n),
            from_t: None,
            to_t: None,
        }
    }

    /// Checks the one-of rule and ordering; `max_bars` bounds `last`.
    ///
    /// # Errors
    /// A message naming the offending field.
    pub fn validate(&self, max_bars: u32) -> Result<(), String> {
        match (self.last, self.from_t, self.to_t) {
            (Some(0), None, None) => Err("`last` must be at least 1".to_owned()),
            (Some(n), None, None) if n > max_bars => Err(format!(
                "`last` is {n}, which exceeds the maximum of {max_bars} bars per binding"
            )),
            (Some(_), None, None) | (None, Some(_), None) => Ok(()),
            (None, Some(a), Some(b)) if a <= b => Ok(()),
            (None, Some(_), Some(_)) => Err("`from_t` must not be after `to_t`".to_owned()),
            _ => Err(
                "a window is exactly one of {\"last\": n}, {\"from_t\": t}, {\"from_t\": a, \"to_t\": b}"
                    .to_owned(),
            ),
        }
    }

    /// True if live bars may extend the window (no `to_t`).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.to_t.is_none()
    }
}

/// A data binding: what a bound series' data is, without the data (K23).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataBinding {
    /// Exchange symbol, e.g. `THYAO` (`^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$`).
    pub symbol: String,
    /// Bar interval: `1m`…`1440m`, `1h`…`24h`, `1d`, `1w`, `1M`.
    pub interval: String,
    /// Which bars.
    pub window: Window,
    /// Named feed of the host (e.g. `matriks`, `delayed`); the host's default if absent
    /// (same id pattern as the symbol).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
    /// Time of bar 0 as fixed by the authority on first resolution (§18.3). Written by the
    /// authority (an engine entry), never by clients; once set, every resolution (the
    /// authority's after a restart, each replica's) starts at this bar, so bar numbers and
    /// the calculation start are the same everywhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_t: Option<TimestampMs>,
}

impl DataBinding {
    /// The feed this binding reads.
    #[must_use]
    pub fn feed_key(&self) -> FeedKey {
        FeedKey {
            symbol: self.symbol.clone(),
            interval: self.interval.clone(),
            feed: self.feed.clone(),
        }
    }
}

/// One data stream: a symbol and interval on a named feed. Series bound to the same
/// feed share its data.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct FeedKey {
    /// Exchange symbol.
    pub symbol: String,
    /// Bar interval.
    pub interval: String,
    /// Named feed; the host's default if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
}

/// Field-level values of a bar; absent fields are unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BarPatch {
    /// Open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub o: Option<f64>,
    /// High.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f64>,
    /// Low.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l: Option<f64>,
    /// Close.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c: Option<f64>,
    /// Volume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v: Option<f64>,
}

impl BarPatch {
    /// True if no field is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.o.is_none()
            && self.h.is_none()
            && self.l.is_none()
            && self.c.is_none()
            && self.v.is_none()
    }

    /// Applies the set fields to `bar`.
    pub fn apply_to(&self, bar: &mut Bar) {
        if let Some(x) = self.o {
            bar.o = x;
        }
        if let Some(x) = self.h {
            bar.h = x;
        }
        if let Some(x) = self.l {
            bar.l = x;
        }
        if let Some(x) = self.c {
            bar.c = x;
        }
        if let Some(x) = self.v {
            bar.v = x;
        }
    }
}

/// What a data event does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataChange {
    /// A new bar after the last one (strictly later `t`).
    Bar {
        /// The bar.
        bar: Bar,
    },
    /// Only the changed fields of an existing bar, by its time: on the last bar a live
    /// update; on an earlier bar a correction.
    Patch {
        /// Time of the bar to change.
        t: TimestampMs,
        /// Changed fields (at least one).
        fields: BarPatch,
    },
}

/// One step of a feed, as the source delivers it (field-level, K23). Not a protocol
/// message: hosts feed it to the engine directly; transcripts inject it with a `data`
/// step (`docs/PROTOCOL.md` §15, §18).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEvent {
    /// The feed.
    pub feed: FeedKey,
    /// The source's sequence number for this feed: 1, 2, 3… without gaps. A gap or a
    /// repeat is detected by the receiver (market-data practice: sequence + gap fill).
    pub seq: u64,
    /// What changes.
    pub change: DataChange,
}

/// Where one side (authority or replica) stands on a feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataPosition {
    /// The feed.
    pub feed: FeedKey,
    /// Number of bars held.
    pub bars: u64,
    /// Time of the last bar, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_t: Option<TimestampMs>,
    /// Source sequence number of the last applied event (0 = only the initial bars).
    pub seq: u64,
    /// [`data_digest`] of all bars held.
    pub digest: String,
}

/// State of a binding for one principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BindingState {
    /// Bars are available.
    Ready,
    /// Resolution was requested; no answer from the provider yet.
    Pending,
    /// The principal has no entitlement to this feed (data licence).
    NoEntitlement,
    /// The source has no data for this binding (unknown symbol, empty window).
    NoData,
    /// Data was held but the feed is no longer trustworthy: a `seq` gap was detected, or
    /// the window reached its size limit (§18.4, §18.7). Values are withheld until the
    /// host re-resolves the binding.
    Stale,
}

/// Name of the digest algorithm of [`data_digest`].
pub const DATA_DIGEST_ALG: &str = "fnv1a64";

/// Start state of [`data_digest_u64`] (the FNV-1a 64 offset basis).
pub const DATA_DIGEST_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

fn fnv(mut h: u64, v: u64) -> u64 {
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Content digest of bars (`docs/PROTOCOL.md` §18.5): FNV-1a 64 from the standard offset
/// basis over, for each bar in order, the 8 little-endian bytes of `t` (as `i64`) and of
/// the IEEE-754 bits of `o`, `h`, `l`, `c`, `v` (`-0.0` digested as `0.0`); written as 16
/// lowercase hex digits.
/// Deterministic on every target. Not cryptographic: it detects corrections, not forgery.
#[must_use]
pub fn data_digest(bars: &[Bar]) -> String {
    format!("{:016x}", data_digest_u64(bars))
}

/// [`data_digest`] as a number.
#[must_use]
pub fn data_digest_u64(bars: &[Bar]) -> u64 {
    bars.iter().fold(DATA_DIGEST_OFFSET, data_digest_extend)
}

/// One bar step of [`data_digest_u64`]: running digests (append, prefix) start at
/// [`DATA_DIGEST_OFFSET`] and fold this over the bars in order.
#[must_use]
pub fn data_digest_extend(h: u64, b: &Bar) -> u64 {
    let mut h = fnv(h, u64::from_le_bytes(b.t.to_le_bytes()));
    for v in [b.o, b.h, b.l, b.c, b.v] {
        // -0.0 and 0.0 are the same price/volume: digest them alike.
        let v = if v == 0.0 { 0.0 } else { v };
        h = fnv(h, v.to_bits());
    }
    h
}

/// Frozen evidence: the data a fact rests on, by reference and content digest (K23).
/// Stamped by the authority when a fact is added; `verify_evidence` re-checks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    /// The bar series the evidence lies on (the fact's series, or the bar series a study
    /// series is computed from).
    pub series: String,
    /// The binding of that series at stamping time (absent for inline data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<DataBinding>,
    /// Time of the first bar of the range (inclusive).
    pub from_t: TimestampMs,
    /// Time of the last bar of the range (inclusive): the fact's bar.
    pub to_t: TimestampMs,
    /// Number of bars in the range.
    pub bars: u64,
    /// Digest algorithm (`fnv1a64`).
    pub alg: String,
    /// [`data_digest`] of the bars in the range.
    pub digest: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            t,
            o: c,
            h: c,
            l: c,
            c,
            v: 1.0,
        }
    }

    #[test]
    fn digest_is_pinned_and_sensitive() {
        let a = vec![bar(1, 10.0), bar(2, 11.0)];
        let d = data_digest(&a);
        assert_eq!(d.len(), 16);
        assert_eq!(data_digest(&[]), format!("{DATA_DIGEST_OFFSET:016x}"));
        let mut b = a.clone();
        b[0].c = 10.000_000_000_000_002;
        assert_ne!(d, data_digest(&b));
        assert_eq!(d, data_digest(&a.clone()));
        let mut z = a.clone();
        z[0].v = -0.0;
        let mut z2 = a.clone();
        z2[0].v = 0.0;
        assert_eq!(data_digest(&z), data_digest(&z2));
    }

    /// Vectors cross-checked with an independent Python FNV-1a 64 (RP2 T2 audit).
    #[test]
    fn digest_vectors_are_pinned() {
        let b1 = Bar {
            t: 1_700_000_000_000,
            o: 100.0,
            h: 101.5,
            l: 99.25,
            c: 100.75,
            v: 12_345.0,
        };
        let b2 = Bar {
            t: 1_700_000_060_000,
            o: 100.75,
            h: 102.0,
            l: 100.5,
            c: 101.0,
            v: 0.0,
        };
        let b3 = Bar {
            t: -1,
            o: 1e-300,
            h: 1e300,
            l: -5.5,
            c: 0.1,
            v: 3.0,
        };
        let got: Vec<String> = [vec![], vec![b1.clone()], vec![b1, b2], vec![b3]]
            .iter()
            .map(|v| data_digest(v))
            .collect();
        assert_eq!(
            got,
            [
                "cbf29ce484222325",
                "9f8403dfb08a2464",
                "c870db4da9b49625",
                "029b32219aabf4f9"
            ]
        );
    }

    #[test]
    fn window_rules() {
        assert!(Window::last(10).validate(100).is_ok());
        assert!(Window::last(0).validate(100).is_err());
        assert!(Window::last(101).validate(100).is_err());
        let w = Window {
            last: Some(3),
            from_t: Some(1),
            to_t: None,
        };
        assert!(w.validate(100).is_err());
        let w = Window {
            last: None,
            from_t: Some(5),
            to_t: Some(4),
        };
        assert!(w.validate(100).is_err());
        let w = Window {
            last: None,
            from_t: None,
            to_t: Some(4),
        };
        assert!(w.validate(100).is_err());
    }

    #[test]
    fn event_wire_form() {
        let e: DataEvent = serde_json::from_str(
            r#"{"feed":{"symbol":"THYAO","interval":"1d"},"seq":3,"change":{"kind":"patch","t":5,"fields":{"c":101.5}}}"#,
        )
        .unwrap();
        assert_eq!(e.seq, 3);
        assert!(matches!(e.change, DataChange::Patch { t: 5, .. }));
        assert!(serde_json::from_str::<DataEvent>(
            r#"{"feed":{"symbol":"THYAO","interval":"1d"},"seq":3,"change":{"kind":"patch","t":5,"fields":{"close":1}}}"#
        )
        .is_err());
    }
}
