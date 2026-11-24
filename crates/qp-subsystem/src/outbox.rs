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
///
/// Order (audit-s30 B3): depth first. Each requested entry is written and settled before
/// the next one; an entry a subsystem asks for while one of them settles is written
/// before the next requested one (`w1`, `w1-child`, `w2`). The authority stops writing
/// requested entries [`MAX_EMIT_DEPTH`] levels deep: deeper requests are dropped, so a
/// subsystem that answers its own entries cannot loop.
///
/// Two requests are taken at the phase's own line, not at the emit step: changes to the
/// entry the phase is about ([`Outbox::amend`]; [`crate::Moment::Look`]) and
/// notifications sent at that line ([`Outbox::say`]; [`crate::Moment::Written`]).
/// Asked for at any other moment, they are dropped at the emit step.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Outbox {
    entries: Vec<EngineEntry>,
    notes: Vec<OutboundLine>,
    wake: Option<i64>,
    changes: Vec<ChangeOp>,
    lines: Vec<OutboundLine>,
}

/// How deep requested engine entries are written (an entry asked for while a requested
/// entry settles is one level deeper).
pub const MAX_EMIT_DEPTH: u32 = 8;

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
    /// Adds changes to the engine entry the phase is about ([`crate::Moment::Look`]).
    pub fn amend(&mut self, changes: impl IntoIterator<Item = ChangeOp>) {
        self.changes.extend(changes);
    }
    /// Sends a notification at the phase's own line ([`crate::Moment::Written`]).
    pub fn say(&mut self, line: OutboundLine) {
        self.lines.push(line);
    }
    /// The changes asked for with [`Outbox::amend`], emptying them.
    pub fn take_changes(&mut self) -> Vec<ChangeOp> {
        std::mem::take(&mut self.changes)
    }
    /// The notifications asked for with [`Outbox::say`], emptying them.
    pub fn take_lines(&mut self) -> Vec<OutboundLine> {
        std::mem::take(&mut self.lines)
    }
    /// Whether nothing was asked for.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
            && self.notes.is_empty()
            && self.wake.is_none()
            && self.changes.is_empty()
            && self.lines.is_empty()
    }
    /// The requests, emptying the outbox: entries, notifications, wake (changes and
    /// line notifications not taken at their phase's line are dropped).
    pub fn drain(&mut self) -> (Vec<EngineEntry>, Vec<OutboundLine>, Option<i64>) {
        let o = std::mem::take(self);
        (o.entries, o.notes, o.wake)
    }
}
