//! Broker notices (`MessengerMessage` on `mx/messenger`, `mx/user/<sub>/messenger`).
//!
//! A **multi-connect** notice means a second connection with the same identity
//! took over the endpoint: the connector must close and must not retry
//! (R6 GOAL, protection rule 3). It is recognised in both shapes seen in the
//! sources: `code == "DISCONNECT"` with `subject == "MULTICONNECT"`
//! (`docs/matriks/CONNECTIVITY.md` §2.4 item 2) and `code == "MULTICONNECT"`
//! (code list in `proto/Messenger.proto`). Comparison is ASCII
//! case-insensitive and ignores surrounding whitespace, to err on the side of
//! stopping.

use prost::Message as _;

use crate::messages::MessengerMessage;
use crate::topic::parse_topic;

/// The fields of a `MessengerMessage`, as sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeInfo {
    /// `code` (required): ALERT, DISCONNECT, MULTICONNECT, METAUPDATE, …
    pub code: String,
    /// `subject`, if sent.
    pub subject: Option<String>,
    /// `from`, if sent.
    pub from: Option<String>,
    /// `timestamp`, if sent (unit not documented).
    pub timestamp: Option<i64>,
    /// `contentType`, if sent (proto default `text/plain` is *not* filled in).
    pub content_type: Option<String>,
    /// `content`, if sent.
    pub content: Option<String>,
}

/// Classification of a broker notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerNotice {
    /// Another connection with the same identity took over: stop, do not retry.
    MultiConnect(NoticeInfo),
    /// The broker is disconnecting us for another reason.
    Disconnect(NoticeInfo),
    /// A user alert (`ALERT`).
    Alert(NoticeInfo),
    /// Symbol metadata changed (`METAUPDATE`).
    MetaUpdate(NoticeInfo),
    /// Any other code.
    Other(NoticeInfo),
    /// A messenger topic whose payload did not decode. The caller decides;
    /// a cautious connector treats this like a disconnect.
    Malformed {
        /// Decode error text.
        reason: String,
    },
}

impl BrokerNotice {
    /// Whether this is a multi-connect notice (stop, do not retry).
    pub fn is_multi_connect(&self) -> bool {
        matches!(self, BrokerNotice::MultiConnect(_))
    }
}

fn eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b)
}

/// Classifies a decoded `MessengerMessage`.
pub fn classify_message(m: MessengerMessage) -> BrokerNotice {
    let info = NoticeInfo {
        code: m.code,
        subject: m.subject,
        from: m.from,
        timestamp: m.timestamp,
        content_type: m.content_type,
        content: m.content,
    };
    let subject_multi = info
        .subject
        .as_deref()
        .is_some_and(|s| eq(s, "MULTICONNECT"));
    if eq(&info.code, "MULTICONNECT") || (eq(&info.code, "DISCONNECT") && subject_multi) {
        BrokerNotice::MultiConnect(info)
    } else if eq(&info.code, "DISCONNECT") {
        BrokerNotice::Disconnect(info)
    } else if eq(&info.code, "ALERT") {
        BrokerNotice::Alert(info)
    } else if eq(&info.code, "METAUPDATE") {
        BrokerNotice::MetaUpdate(info)
    } else {
        BrokerNotice::Other(info)
    }
}

/// Classifies a `(topic, payload)` pair. `None` if the topic is not a
/// messenger topic (`mx/messenger`, `mx/user/<sub>/messenger`, or a user
/// mirror of `mx/messenger`).
pub fn classify_notice(topic: &str, payload: &[u8]) -> Option<BrokerNotice> {
    if !parse_topic(topic).is_messenger() {
        return None;
    }
    Some(match MessengerMessage::decode(payload) {
        Ok(m) => classify_message(m),
        Err(e) => BrokerNotice::Malformed {
            reason: e.to_string(),
        },
    })
}
