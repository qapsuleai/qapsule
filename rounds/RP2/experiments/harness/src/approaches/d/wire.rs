//! D's protocol messages (JSON, canonical per Ö4). Market data never travels here (K23):
//! only claims, their evidence envelopes (ranges, digests, values) and data marks
//! (digests of settled bars).

use std::collections::BTreeMap;

use qp_schema::data::data_digest_u64;
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

use crate::detector::ClaimKey;

/// Engine identity stamped into every envelope (the replica runs the same code; a
/// different engine would make value equality meaningless).
pub const ENGINE: &str = "qp-compute/0.0.0";

/// Digest algorithm name (`qp_schema::data::data_digest`).
pub const ALG: &str = "fnv1a64";

/// Bars per data-mark chunk (the lower level of the two-level digest tree, E).
/// Chunks are aligned on absolute bar time, so both sides agree on chunk boundaries
/// whatever their anchors.
pub const CHUNK: i64 = 32;

/// Maturity on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mat {
    /// On the authority's live bar, or on a closed bar not yet confirmed against the
    /// provider's settled bars.
    Provisional,
    /// Every bar the claim rests on is confirmed against the provider's settled bars.
    Final,
}

/// The evidence envelope: `qp_schema::data::EvidenceRef` (series, from_t, to_t, bars,
/// alg, digest) plus the fields D needs for live consistency (anchor_t, rev, epoch,
/// engine, optional prefix digest). One per binding the claim rests on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    /// Bar series (binding id).
    pub series: String,
    /// Compute start: time of the authority's bar 0 (PROTOCOL §18.3).
    pub anchor_t: i64,
    /// Evidence window start (inclusive): the claim bar minus `period` bars.
    pub from_t: i64,
    /// Evidence window end (inclusive): the claim's bar.
    pub to_t: i64,
    /// Bars in the window on the authority.
    pub bars: u64,
    /// Digest algorithm.
    pub alg: String,
    /// Window digest (only when final; live bars are not compared).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Digest of `anchor_t..=to_t` (full commitment; only the `prefix` variant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Authority's source sequence number for the series when stamped (data position /
    /// live revision).
    pub rev: u64,
    /// Data epoch of the series on the authority (+1 per history change: correction,
    /// rescale, refresh that changed closed bars).
    pub epoch: u32,
    /// Engine identity.
    pub engine: String,
}

/// Why a claim was retracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// The predicate no longer holds on the authority's data (live bar, close,
    /// settled confirmation or correction).
    NoLongerHolds,
}

/// One operation of an authority batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Session start: engine and anchors (also in every envelope).
    Hello {
        /// Engine identity.
        engine: String,
        /// `(series, anchor_t)`.
        anchors: Vec<(String, i64)>,
    },
    /// A claim holds on the authority's data (new, or provisional again).
    Fact {
        /// Authority-assigned id (stable per claim key).
        id: u64,
        /// The claim by meaning.
        key: ClaimKey,
        /// Maturity.
        m: Mat,
        /// Evidence values (`prev`, `value`, `level`; pair: `a`, `b`, `level`).
        v: BTreeMap<String, f64>,
        /// Envelopes.
        ev: Vec<Envelope>,
    },
    /// The claim is final (confirmed against settled bars), or its final envelope
    /// changed (correction / rescale: new digest, new epoch).
    FactFinal {
        /// Id.
        id: u64,
        /// The claim by meaning (repeated so a reordered `fact_final` stands alone).
        key: ClaimKey,
        /// Evidence values on the settled data.
        v: BTreeMap<String, f64>,
        /// Envelopes with window digests.
        ev: Vec<Envelope>,
    },
    /// The claim no longer holds.
    FactRetracted {
        /// Id.
        id: u64,
        /// Reason.
        why: Why,
    },
    /// Data mark (E): digests of settled bars of one series, so the replica can find
    /// exactly which of its bars differ and refetch only those from its own source.
    DataMark {
        /// Series.
        series: String,
        /// Epoch.
        epoch: u32,
        /// `(bar t, digest of that bar)` for newly settled or corrected bars.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bars: Vec<(i64, String)>,
        /// `(chunk index, digest of its CHUNK bars)` for complete settled chunks.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        chunks: Vec<(i64, String)>,
    },
}

/// Authority → replica message: a batch with a per-authority sequence number (the
/// replica applies an op on a claim only if the batch is newer than the last op it
/// applied on that claim: reordering and duplicates are harmless).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Batch {
    /// Sequence number.
    pub s: u64,
    /// Operations (at most one per claim id).
    pub ops: Vec<Op>,
}

/// Replica → authority report (informational; never changes the document).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Report {
    /// The replica's data does not confirm a final claim even after refetching from its
    /// own source.
    DataDispute {
        /// Claim id.
        id: u64,
        /// `predicate` (fails locally), `digest` (window differs), `values`.
        what: String,
    },
}

/// Hex form of a digest.
#[must_use]
pub fn hex(d: u64) -> String {
    format!("{d:016x}")
}

/// Digest of bars as a number.
#[must_use]
pub fn digest(bars: &[Bar]) -> u64 {
    data_digest_u64(bars)
}

/// Chunk index of a bar time.
#[must_use]
pub fn chunk_of(t: i64, interval_ms: i64) -> i64 {
    t.div_euclid(CHUNK * interval_ms)
}

/// `[first, last]` bar times of a chunk.
#[must_use]
pub fn chunk_span(c: i64, interval_ms: i64) -> (i64, i64) {
    let w = CHUNK * interval_ms;
    (c * w, c * w + w - interval_ms)
}
