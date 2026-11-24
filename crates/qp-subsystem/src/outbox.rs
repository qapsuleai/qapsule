//! The outbox: the only way a subsystem writes.

use qp_schema::protocol::{ChangeOp, OutboundLine};

/// An engine entry a subsystem asks for (author `qp`; §15 order when written).
#[derive(Debug, Clone, PartialEq)]
pub struct EngineEntry {
    /// The entry's changes.
    pub changes: Vec<ChangeOp>,
    /// The entry's summary.
    pub summary: String,
}

/// What subsystems asked for during a phase; the authority applies it at the phase's
/// emit step, in request order.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Outbox {
    entries: Vec<EngineEntry>,
    notes: Vec<OutboundLine>,
    wake: Option<i64>,
}

impl Outbox {
    /// Asks for an engine entry.
    pub fn entry(&mut self, changes: Vec<ChangeOp>, summary: String) {
        self.entries.push(EngineEntry { changes, summary });
    }
    /// Sends a notification.
    pub fn notify(&mut self, line: OutboundLine) {
        self.notes.push(line);
    }
    /// Asks to be woken (a clock at or after `t`; the earliest request wins).
    pub fn wake_at(&mut self, t: i64) {
        self.wake = Some(self.wake.map_or(t, |w| w.min(t)));
    }
    /// Whether nothing was asked for.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.notes.is_empty() && self.wake.is_none()
    }
    /// The requests, emptying the outbox: entries, notifications, wake.
    pub fn drain(&mut self) -> (Vec<EngineEntry>, Vec<OutboundLine>, Option<i64>) {
        let o = std::mem::take(self);
        (o.entries, o.notes, o.wake)
    }
}
