//! The ledger index: close-time ordered counts per ledger key and over all keys, so
//! point-in-time ledger reads do not scan the rows.

use super::*;

/// Close-time ordered counts of ledger rows (one key, or all keys).
#[derive(Debug, Clone, Default)]
pub(super) struct KeyIndex {
    /// Close times (`recorded_at`), non-decreasing.
    t: Vec<i64>,
    /// `realized[i]` = realized rows among the first `i` (length `t.len() + 1`).
    realized: Vec<u32>,
    /// Retracted rows: `(recorded_at, retracted_at, realized)`.
    retracted: Vec<(i64, i64, bool)>,
}

impl KeyIndex {
    fn insert(&mut self, t: i64, realized: bool) {
        if self.realized.is_empty() {
            self.realized.push(0);
        }
        let pos = self.t.partition_point(|x| *x <= t);
        self.t.insert(pos, t);
        if pos + 1 == self.t.len() {
            let last = self.realized[pos];
            self.realized.push(last + u32::from(realized));
        } else {
            // Out-of-order close (bars fed out of global time order): rebuild the tail.
            self.realized.insert(pos + 1, 0);
            self.realized[pos + 1] = self.realized[pos] + u32::from(realized);
            // Every later prefix gains this row.
            for r in &mut self.realized[pos + 2..] {
                *r += u32::from(realized);
            }
        }
    }

    /// Same semantics as [`LedgerRecord::counts`] summed over the rows.
    pub(super) fn stat(&self, before_t: i64) -> LedgerStat {
        let k = self.t.partition_point(|x| *x < before_t);
        let mut n = u32::try_from(k).unwrap_or(u32::MAX);
        let mut realized = self.realized.get(k).copied().unwrap_or(0);
        for (rec, ret, real) in &self.retracted {
            if *rec < before_t && *ret < before_t {
                n -= 1;
                realized -= u32::from(*real);
            }
        }
        LedgerStat { n, realized }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct LedgerIndex {
    pub(super) all: KeyIndex,
    pub(super) by_key: BTreeMap<String, KeyIndex>,
}

impl LedgerIndex {
    fn key(r: &LedgerRecord) -> &str {
        r.entry.key.as_deref().unwrap_or("")
    }

    pub(super) fn insert(&mut self, r: &LedgerRecord) {
        let real = r.entry.outcome == Outcome::Confirmed;
        self.all.insert(r.entry.recorded_at, real);
        self.by_key
            .entry(Self::key(r).to_owned())
            .or_default()
            .insert(r.entry.recorded_at, real);
    }

    pub(super) fn retract(&mut self, r: &LedgerRecord, at: i64) {
        let row = (
            r.entry.recorded_at,
            at,
            r.entry.outcome == Outcome::Confirmed,
        );
        self.all.retracted.push(row);
        self.by_key
            .entry(Self::key(r).to_owned())
            .or_default()
            .retracted
            .push(row);
    }
}

pub(super) fn stat<'a>(rows: impl Iterator<Item = &'a LedgerRecord>, before_t: i64) -> LedgerStat {
    let mut st = LedgerStat::default();
    for r in rows.filter(|r| r.counts(before_t)) {
        st.n += 1;
        if r.entry.outcome == Outcome::Confirmed {
            st.realized += 1;
        }
    }
    st
}
