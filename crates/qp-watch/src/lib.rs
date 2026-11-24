//! Watches (`izleme niyeti`; R5B, `docs/PROTOCOL.md` §25.5): the first subsystem of the
//! authority (RF1 S3.1). Structured intents on story transitions, matched
//! deterministically by the engine.
//!
//! - `add_watch` → `put_intent` with `watch` (state `active`); `remove_watch` →
//!   `put_intent` with `active: false`, state `removed`. Policy: anyone who may read, never
//!   a proposal, an agent directly, another person's requests private ([`qp_subsystem::OpHandler`]).
//! - After the capsule engine's look (one engine entry per watched series and look),
//!   every active watch is tested against the look's story transitions in a fixed order
//!   (transitions in the engine's order, watches in document order). The engine entry
//!   itself carries the watches' new counts and states (`put_intent`): `matched` once
//!   `max_matches` is reached, `expired` once the clock reaches `expires_t`. Same input,
//!   same entries, same events ([`qp_subsystem::Phase::AfterData`], `Moment::Look`).
//! - Each match is an `intent_matched` event on the `intents` topic, to the watch's
//!   person (and the agents working for them), projected: a public-view reader gets the
//!   public world's matches, a reader with a projection none about a capsule it may not
//!   see (`Moment::Written`).
//!
//! The privacy of watch items in a document (who may read another person's watch) stays
//! with the authority's projection: it holds for any document, with or without this
//! subsystem. Knows no authority: only the contract (`qp-subsystem`).

mod matching;
mod ops;
mod subsystem;

pub use subsystem::{Watch, WATCH};

#[cfg(test)]
mod params_binding {
    // RG1/K3: the constant reads `qp_schema::params` and keeps its old literal.
    #[test]
    fn max_watches_binds_registry() {
        assert_eq!(qp_ops::watch::MAX_WATCHES, 32);
    }
}
