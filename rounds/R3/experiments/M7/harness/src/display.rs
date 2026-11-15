//! What the replica shows the user, keyed by meaning. The harness reads the replica's
//! [`DisplayState`] after every virtual instant and derives the display log from
//! successive states; the metrics are computed from that log (plus the replica's
//! visible data and the channel bytes), never from an approach's own account.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::detector::{ClaimKey, EvidenceRange};

/// How a claim is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Label {
    /// Asserted, marked "provisional — bar not closed yet".
    Provisional,
    /// Asserted as final.
    Final,
    /// Not asserted: only a "sources differ" marker (the replica's data contradicts the
    /// authority's claim).
    Disputed,
    /// Not asserted: "the server found something, data on its way".
    Pending,
    /// Asserted on the replica's own data (its values), with a note "your data source
    /// differs from the server's".
    SourceDiffers,
}

impl Label {
    /// True if the label asserts the claim to the user (Provisional, Final,
    /// SourceDiffers). Disputed and Pending assert nothing.
    #[must_use]
    pub fn asserts(self) -> bool {
        matches!(self, Self::Provisional | Self::Final | Self::SourceDiffers)
    }

    /// Short name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Final => "final",
            Self::Disputed => "disputed",
            Self::Pending => "pending",
            Self::SourceDiffers => "source_differs",
        }
    }
}

/// One shown item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayItem {
    /// Identity by meaning.
    pub key: ClaimKey,
    /// Label.
    pub label: Label,
    /// Values shown next to the claim (e.g. `value`, `prev`, `level`).
    pub shown_values: BTreeMap<String, f64>,
    /// Evidence bars the claim points at.
    pub evidence: Vec<EvidenceRange>,
}

/// Everything the replica shows (claims not in the map are not shown).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DisplayState {
    /// Items by key.
    pub items: BTreeMap<ClaimKey, DisplayItem>,
}

impl DisplayState {
    /// Shows or replaces an item.
    pub fn put(&mut self, item: DisplayItem) {
        self.items.insert(item.key.clone(), item);
    }

    /// Removes an item.
    pub fn remove(&mut self, key: &ClaimKey) -> Option<DisplayItem> {
        self.items.remove(key)
    }
}

/// Formats values compactly and deterministically (shortest round-trip form).
#[must_use]
pub fn fmt_values(v: &BTreeMap<String, f64>) -> String {
    let mut s = String::new();
    for (i, (k, x)) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(k);
        s.push('=');
        s.push_str(&format!("{x}"));
    }
    s
}
