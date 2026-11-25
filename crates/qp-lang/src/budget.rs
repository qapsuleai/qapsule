//! The per-owner live budget inside one document (PR §8.2; checked in the `Check` phase,
//! PS §2.3): instances, the sum of `live_per_bar`, and program bytes in the log, each at
//! most its `program.live.doc.*` limit (`E_BUDGET`, `expected` = the limit).

use qp_schema::protocol::ProgramError;

use crate::limits::{Limit, Limits};
use crate::message::{Lang, Msg};
use crate::reason::Reason;

/// An owner's live use in one document, after the change being checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LiveUsage {
    /// Program instances.
    pub instances: u64,
    /// Σ `live_per_bar` of the instances.
    pub cost_per_bar: u64,
    /// Bytes of program changes in the log.
    pub log_bytes: u64,
}

/// One `E_BUDGET` error per exceeded limit (none when within budget).
#[must_use]
pub fn check_live_budget(usage: &LiveUsage, limits: &Limits, lang: Lang) -> Vec<ProgramError> {
    [
        (Limit::LiveDocMaxInstances, usage.instances),
        (Limit::LiveDocMaxCostPerBar, usage.cost_per_bar),
        (Limit::LiveDocMaxLogBytes, usage.log_bytes),
    ]
    .into_iter()
    .filter(|(l, used)| *used > limits.get(*l))
    .map(|(l, used)| {
        let lim = limits.get(l);
        let mut e = Reason::Budget.error(None, Msg::Budget(l.key(), used, lim).render(lang));
        e.expected = vec![lim.to_string()];
        e
    })
    .collect()
}
