//! Sentence realizer chosen in R3/M2: **template grammar** ([`Template`]; TR + EN).
//!
//! Provenance: productized from the measured experiment crate
//! `rounds/R3/experiments/M2/template/` (commit `85b5b5d`, branch `r3/m2a-template`;
//! choice and reasons in `rounds/R3/experiments/M2/RESULTS.md`). Outputs are byte-identical
//! to the measured crate on the M2 harness situations
//! (`crates/qp-capsule/tests/fixtures/m2_inputs.json`), except for two documented
//! sentence fixes from the experiments (`rounds/R3/experiments/S/S-A.md`):
//!
//! - **flow condition** (M2b finding): a flow state/expectation on net institutional flow
//!   is said as "net buying / net selling stronger than usual", never as "net below the
//!   average" (which reads as weak selling);
//! - **extreme outcome** (M5 RESULTS): with the barrier outcome rule a *realized* extreme
//!   means "the stretch continued", an *invalidated* one "the stretch did not continue";
//!   the stage remark says exactly that.
//!
//! Original description of the measured realizer follows.
//!
//! A sentence is a pure function of [`SentenceInput`](crate::SentenceInput): no LLM, no clock, no randomness.
//! Two steps:
//!
//! 1. [`Plan::new`] reads the input into a language-independent frame: the **time class**
//!    (event / state that holds / state that ended / expectation, from `story.time`,
//!    `still_holds` and the stage), the bars-ago of the lead fact and of the last
//!    transition, the condition (subject, side, threshold key), the current value, the
//!    stage remark, the validity tail, the ledger and the trust/licence marks. Every
//!    number the frame can print comes from an engine slot; in a redacted frame the
//!    accessors return no numbers at all.
//! 2. [`tr`] / [`en`] render the frame with a fixed template family per class. Turkish
//!    suffixes come from real morphology ([`morph`]); numbers are formatted with the
//!    harness rounding rule ([`num`]).
//!
//! Redacted facts (licensed, unentitled; K33) use a separate **no-digit family**: bars-ago
//! becomes "bugün / dün / birkaç bar önce / yaklaşık bir hafta önce…", thresholds become
//! names ("aşırı alım eşiği", "kırılım seviyesi"), the ledger becomes qualitative
//! ("çoğunlukla beklendiği gibi sonuçlandı"). A last guard replaces any sentence that
//! would still carry a digit (it never fires on the tested inputs; see tests).

pub mod elapsed;
pub mod en;
pub mod morph;
pub mod num;
pub mod parts;
mod plan;
pub mod stamp;
mod template;
pub mod tr;

pub use plan::{
    key_cat, Ago, Class, Cond, LedgerView, Plan, Remark, Subject, Tendency, Vague, Validity,
    LEDGER_MIN_N,
};
pub use template::{render, Template};

#[cfg(test)]
mod tests;
