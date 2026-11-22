//! The viewer stream (RP3/M15, §22): trust marks, faces and viewer updates.

use super::*;

/// (RP3/M15, K48, §22) The trust mark of a capsule on a viewer's screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustMark {
    /// The authority's settled claim, shown as "verified by the server": a feed capsule
    /// (the device did not check it; it holds none of its data).
    Server,
    /// Checked on the device's own data (the open capsule, after its check passed).
    Device,
}

/// (RP3/M15, §22) One feed row of a viewer: the capsule as listed for the reader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerItem {
    /// The capsule as the `capsules` query lists it for the reader.
    pub capsule: CapsuleItem,
    /// When the reader could first know its latest transition (the open time of the bar
    /// after the transition bar); absent: the transition is on the newest bar (now).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_at: Option<TimestampMs>,
    /// The authority sends a face scene for it when it is visible (the reader may see the
    /// series' data); else the device shows the sentence.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub face: bool,
}

/// (RP3/M15, §22) A ready face: the scene the authority compiled for the reader (its
/// face design, look, language and data), at the face frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerFace {
    /// Capsule id.
    pub capsule: String,
    /// `view_digest` of the scene (§19.6).
    pub digest: String,
    /// The scene (`SceneDesc`).
    pub scene: Box<crate::scene::SceneDesc>,
    /// Always `server` from the authority.
    pub trust: TrustMark,
}

/// (RP3/M15, §22) The open capsule's slice: what the device needs to draw and check it
/// on its own data — the document's items about its instrument (its series and the
/// studies on them, their facts, findings, capsules, views and intents, their ledger
/// rows, the reader's audience) — and the feeds the host now serves this device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerOpen {
    /// Capsule id.
    pub capsule: String,
    /// Its finding.
    pub finding: String,
    /// Instrument symbol.
    pub instrument: String,
    /// Bar series.
    pub series: String,
    /// The slice (a document; `document_digest` in `digest`).
    pub document: Box<Document>,
    /// `document_digest` of the slice.
    pub digest: String,
    /// Bound feeds the slice rests on (the device's data subscriptions while it is open).
    pub feeds: Vec<FeedKey>,
}

/// (RP3/M15, §22) What changed for a viewer (`viewer` event and `visible` answer).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerUpdate {
    /// The feed, top to bottom (when it changed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<Vec<ViewerItem>>,
    /// Faces of visible capsules that are new on screen or changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<ViewerFace>,
    /// The open capsule's slice (when it opened or its slice changed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<ViewerOpen>,
    /// The open capsule was closed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    /// The reader's audience (when it changed; `null` in JSON is not sent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Audience>,
    /// The device's data subscriptions now: the feeds the host serves this connection
    /// (empty on the feed screen).
    pub data: Vec<FeedKey>,
    /// (R5/M19, K53) With `feed`: the `capsules` answer's `coin_flip_hidden` (absent:
    /// none).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub coin_flip_hidden: u32,
    /// (R6/T3, §24) The market status (when it changed, and in the first event once the
    /// host gave one). Absent while the host gave none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<crate::market::MarketStatus>,
}

impl ViewerUpdate {
    /// Nothing changed (only the data subscriptions, repeated).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.feed.is_none()
            && self.faces.is_empty()
            && self.open.is_none()
            && !self.closed
            && self.audience.is_none()
            && self.market.is_none()
    }
}
