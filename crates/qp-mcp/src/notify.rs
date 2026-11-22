//! Notification delivery: the endpoint's bounded [`Inbox`], the [`NotifyMode`] and what
//! [`crate::McpServer::deliver`] hands the client ([`Delivery`]).

use serde_json::Value;

use crate::Queued;

/// URI of the resource holding an endpoint's pending notifications.
pub const NOTIFY_URI: &str = "qapsule://notifications";
/// Most notifications an endpoint keeps unread (the oldest are dropped and counted).
pub const INBOX_CAP: usize = 10_000;

/// An endpoint's unread notifications (bounded).
#[derive(Debug, Default)]
pub struct Inbox {
    items: std::collections::VecDeque<Queued>,
    dropped: u64,
}

impl Inbox {
    /// Adds notifications (dropping the oldest beyond [`INBOX_CAP`]).
    pub fn extend(&mut self, q: impl IntoIterator<Item = Queued>) {
        for x in q {
            if self.items.len() == INBOX_CAP {
                self.items.pop_front();
                self.dropped += 1;
            }
            self.items.push_back(x);
        }
    }

    /// Takes every unread notification, oldest first.
    pub fn drain(&mut self) -> Vec<Queued> {
        self.items.drain(..).collect()
    }

    /// Unread count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// No unread notification.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The newest unread notification's hub order.
    #[must_use]
    pub fn last_order(&self) -> Option<u64> {
        self.items.back().map(|q| q.order)
    }

    /// Notifications dropped unread so far.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

/// How an endpoint delivers notifications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotifyMode {
    /// RD1 behaviour (in-process adapters, the conformance runner): every pending
    /// notification goes out as `notifications/message` before the next response.
    #[default]
    Inline,
    /// (R5B, `qp mcp serve`) By the MCP revision the client speaks: a legacy client
    /// (`initialize` with 2025-11-25/2025-06-18) gets `notifications/message` on its
    /// stream ([`McpServer::deliver`]); a modern client (2026-07-28) gets
    /// `notifications/resources/updated` for [`NOTIFY_URI`] on its `subscriptions/listen`
    /// stream and reads the resource (or calls the `notifications` tool), and
    /// `notifications/message` only inline for a request that gave a `logLevel`.
    Spec,
}

/// What [`McpServer::deliver`] has for the client.
#[derive(Debug, Default)]
pub struct Delivery {
    /// `notifications/message` (legacy clients), oldest first.
    pub messages: Vec<Value>,
    /// `notifications/resources/updated` for the open `subscriptions/listen`, if new
    /// notifications arrived since the last signal.
    pub updated: Option<Value>,
}
