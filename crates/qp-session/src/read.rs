//! Reading and presentation: queries, semantic reading, preview, explanation and
//! interaction (the presenter is `qp_ops::present`).

pub(crate) mod explain;
pub(crate) mod interact;
pub(crate) mod preview;
pub(crate) mod query;
pub(crate) mod reading;

#[cfg(test)]
mod tests;
