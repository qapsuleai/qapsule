//! Line transport framing, host configuration and conformance transcripts.

use super::*;

// ---------------------------------------------------------------------------
// Line transport framing
// ---------------------------------------------------------------------------

/// A line of the line-based transport (`qp serve`), client to authority: one JSON
/// object per line. `conn` names the logical connection, so several clients can share
/// one stream. The first message on a connection must be `hello`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InboundLine {
    /// Logical connection id.
    pub conn: String,
    /// The message.
    pub msg: ClientMsg,
}

/// A line of the line-based transport, authority to client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutboundLine {
    /// Logical connection id.
    pub conn: String,
    /// The message.
    pub msg: ServerMsg,
}

// ---------------------------------------------------------------------------
// Host configuration and conformance transcripts
// ---------------------------------------------------------------------------

/// A document given inline or as a path (relative to the repository root).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DocumentRef {
    /// Path to a `Document` JSON file.
    Path(String),
    /// The document itself.
    Inline(Box<Document>),
}

/// How an authority starts (host configuration; not a protocol message).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionInit {
    /// The initial document.
    pub document: DocumentRef,
    /// Known principals.
    pub principals: Vec<Principal>,
    /// Initial time, epoch ms.
    pub clock: TimestampMs,
    /// Authority epoch.
    #[serde(default)]
    pub epoch: u64,
    /// Proposals expire this long after creation; never if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_ttl_ms: Option<u64>,
    /// (RP2) The host's data plane for bound series: feeds and entitlements, served by
    /// a test source provider (`docs/PROTOCOL.md` §18.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<InitData>,
    /// (R3, §20) The capsule engine the authority runs: on which bar series, with which
    /// licensed flow. Absent: no engine (documents and transcripts before R3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsules: Option<CapsuleInit>,
    /// (R4, §21) A replay source: the authority runs under a replay host that serves
    /// `init.data` from the replay's initial bars, gives the clock and the data events,
    /// and answers `replay` messages. Absent: no replay (`replay` → `unsupported`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay: Option<ReplayInit>,
    /// (R5/M19, K53) The host's feed rule: stories whose ledger at birth is near a coin
    /// flip leave the feed below the `all` sensitivity (`docs/CAPSULE.md` §9.10). Absent:
    /// no rule (no story is marked; the protocol's default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_rule: Option<FeedRuleInit>,
    /// (R5B, §25.3) The agent policy: `"r5b"` applies the agent write rule (no facts,
    /// findings, bar data, undo, capsule, ledger or story changes) to every command of an
    /// agent principal, on any connection. Absent: the rule holds only on connections the
    /// host marked as agent connections. An unknown name is refused at session start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_policy: Option<String>,
    /// (R5B, §25.4) The comment validator's strictness (`cites`, `cites_numbers`,
    /// `per_sentence`). Absent: `cites_numbers`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment_strictness: Option<crate::comment::Strictness>,
}

/// (R5/M19, K53, K42) The host's feed rule (host configuration, not a protocol message).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeedRuleInit {
    /// Rule name (`qp_capsule::feedrule::FEED_RULES`: `rate55`, `none`).
    pub rule: String,
    /// (K42) A negative stage of a story about the reader's position or active intent is
    /// never hidden by the rule (default `true`, M19's recommendation).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub exempt_negative: bool,
}

fn yes() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_true(b: &bool) -> bool {
    *b
}

/// The replay source of a session (R4, §21; host configuration, not a protocol
/// message). Paths never come from here: local data is read from the host's own
/// directory (`qp_source::local_bars_dir`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayInit {
    /// `synthetic` (default) or `local`; `local` without data falls back to the
    /// synthetic market and says so (`ReplayState::note`).
    #[serde(default)]
    pub source: ReplaySourceKind,
    /// Number of instruments (default 30).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruments: Option<u32>,
    /// Synthetic market: trading days (default 260).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
    /// Synthetic market: seed (default 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Feed interval (default `1d`; the synthetic market has `1d` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Finer interval that moves a bar inside its day (e.g. `1h`); absent: none (the
    /// labelled OHLC path, or whole bars with `path: false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fine: Option<String>,
    /// Bars of each feed resolved before the first replayed bar (default 120).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<u32>,
    /// First replayed bar: the first with `t >= from_t` (overrides `history`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// Last replayed bar: `t <= to_t`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
    /// Without fine data: the OHLC path (default true) or one whole bar per day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<bool>,
    /// Named feed of the replay's feed keys (default: none, the host's default feed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
    /// Initial speed, steps per wall-clock second (default 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    /// Start running with the wall clock (default false: paused).
    #[serde(default, skip_serializing_if = "is_false")]
    pub running: bool,
    /// Principal id -> data entitlements on the replay's feeds (as `InitData`);
    /// empty: every principal is entitled to every feed (`*`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub entitlements: BTreeMap<String, Vec<String>>,
    /// Add a bound series (id: the symbol in lower case, window: the history) for every
    /// replay feed no series of the document binds.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bind_all: bool,
    /// Run the capsule engine (§20) on every bound series of a replay feed (added to
    /// `capsules.series`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub capsules_all: bool,
}

/// The authority's capsule engine (R3, §20; host configuration).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleInit {
    /// Bar series (bound or inline, with an instrument symbol) the detectors and the
    /// capsule engine run on, at every bar that becomes final.
    pub series: Vec<String>,
    /// Licensed flow feeds the authority alone reads (server facts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flow: Vec<FlowInit>,
    /// (RP4, `docs/CAPSULE.md` §9.9) The named capsule constant record the engine runs
    /// with: its outcome rule and its detector set (`r3`, `rp4`). Absent: `r3` (the
    /// protocol default). An unknown name is refused at session start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

/// A licensed flow feed of the test host (R3, §20): `qp_source::synthetic_flow` over the
/// bars of a watched series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlowInit {
    /// The watched bar series the flow follows (one record per bar).
    pub series: String,
    /// Seed of the synthetic flow.
    pub seed: u64,
    /// Feed name; entitlement `feed/SYMBOL` (default `flow`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
}

/// Test data plane of a session init (RP2): the feeds a test source provider serves and
/// who is entitled to which symbols.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitData {
    /// Feeds, each with its initial bars.
    #[serde(default)]
    pub feeds: Vec<InitFeed>,
    /// Principal id -> entitlements: `"SYMBOL"` (that symbol on every feed),
    /// `"feed/SYMBOL"` (that symbol on that named feed only), `"feed/*"`, or `"*"` (all).
    /// Principals not listed have no entitlement. The authority's own engine work uses the
    /// entitlement of the first principal whose role is `host` (§18.3).
    #[serde(default)]
    pub entitlements: BTreeMap<String, Vec<String>>,
    /// Feeds that answer `pending` (never resolved) instead of data, for tests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<FeedKey>,
    /// (R3, §19.9) The test provider's settled source: answers the authority's settled
    /// requests synchronously with the provider's bars, these bars replacing theirs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<InitSettled>,
}

/// The test settled source of a session init (R3, §19.9).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitSettled {
    /// Per feed, settled bars that differ from what the feed delivered (by `t`).
    #[serde(default)]
    pub feeds: Vec<InitSettledFeed>,
    /// Verify every closed bar of a bound series against the settled source (the
    /// authority's settled-bar hygiene); off: only disputes ask for settled bars.
    #[serde(default)]
    pub verify_closed: bool,
}

/// Settled bars of one feed (R3, §19.9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitSettledFeed {
    /// The feed.
    pub feed: FeedKey,
    /// The settled bars (replacing the provider's bars with the same `t`).
    pub bars: Vec<Bar>,
}

/// One test feed: give exactly one of `bars`, `path`, `synthetic`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitFeed {
    /// The feed.
    pub feed: FeedKey,
    /// Bars inline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bars: Option<Vec<Bar>>,
    /// A bar series JSON file (a `Series` with bars), relative to the repository root and
    /// confined to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Deterministic synthetic bars (`qp_source::synthetic_bars(n, seed)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<SyntheticFeed>,
}

/// Parameters of a synthetic test feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SyntheticFeed {
    /// Number of bars.
    pub n: u32,
    /// Seed.
    pub seed: u64,
}

/// A session init given inline or as a path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum InitRef {
    /// Path to a `SessionInit` JSON file.
    Path(String),
    /// The init itself.
    Inline(Box<SessionInit>),
}

/// A raw (possibly invalid) line sent on a connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawLine {
    /// Logical connection id.
    pub conn: String,
    /// The text of the message, sent as is.
    pub line: String,
}

/// An expected outbound message: `msg` is a pattern matched as a subset
/// (see `docs/PROTOCOL.md` §15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// Connection the message goes to.
    pub conn: String,
    /// Pattern of the message.
    pub msg: serde_json::Value,
}

/// One step of a transcript: send one message, expect exactly these outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// A well-formed message to send.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send: Option<InboundLine>,
    /// A raw line to send instead (for parse and schema errors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_raw: Option<RawLine>,
    /// (RP2) A data event the host delivers instead (data plane, not a protocol message;
    /// §18.4). Outputs are the notifications it causes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<DataEvent>,
    /// Expected outputs, complete and in order.
    pub expect: Vec<Expect>,
    /// Optional comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Checks after the last step.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FinalCheck {
    /// Pattern the final document must match (subset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<serde_json::Value>,
    /// Exact number of log entries expected, if given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_len: Option<u64>,
}

/// A conformance transcript: language- and transport-independent
/// "input messages -> expected output messages" scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transcript {
    /// Short name (file stem).
    pub name: String,
    /// What it shows.
    pub description: String,
    /// Message kinds and features it covers.
    #[serde(default)]
    pub covers: Vec<String>,
    /// How the authority starts.
    pub init: InitRef,
    /// Steps, in order.
    pub steps: Vec<Step>,
    /// Final checks. Replay byte-equality is always checked.
    #[serde(default, rename = "final")]
    pub final_check: FinalCheck,
}
