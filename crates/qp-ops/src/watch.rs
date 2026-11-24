//! (R5B, `docs/PROTOCOL.md` §25.5) The pure part of watches every op shares: the privacy
//! rule of who may see a watch, its limit and its state. The `add_watch` /
//! `remove_watch` expansion and matching a watch against story transitions are the watch
//! subsystem's (`qp-watch`, RF1 S3.1).

use qp_schema::{Document, Intent, IntentState, Watch};

use crate::expand::Ctx;

/// Active watches per person at most (`welcome.limits.max_watches`).
pub const MAX_WATCHES: u32 = qp_schema::params::system::WATCH_MAX_PER_PERSON_DEFAULT;

/// Whether a watch is on (active, state `active`).
pub fn on(i: &Intent) -> bool {
    i.active && i.watch.is_some() && i.state == Some(IntentState::Active)
}

/// (Privacy, §25.5) The one rule for who may see (and so touch, or learn the existence
/// of) a watch: an `owner`, its person, or an agent working for its person.
pub fn watch_visible(w: &Watch, ctx: &Ctx<'_>) -> bool {
    ctx.is_owner || w.for_ == ctx.speaker || w.for_ == person_of(ctx.plane, ctx.speaker)
}

/// Whether intent `i` is visible to the speaker of `ctx` (an intent without a watch is).
pub(crate) fn intent_visible(i: &Intent, ctx: &Ctx<'_>) -> bool {
    i.watch.as_ref().is_none_or(|w| watch_visible(w, ctx))
}

/// Every item id of `doc` the speaker of `ctx` may learn of (another person's watches
/// left out), for `did_you_mean`.
pub(crate) fn visible_ids<'d>(doc: &'d Document, ctx: &Ctx<'_>) -> Vec<&'d str> {
    let hidden: std::collections::BTreeSet<&str> = doc
        .intents
        .iter()
        .filter(|i| !intent_visible(i, ctx))
        .map(|i| i.id.as_str())
        .collect();
    crate::apply::all_ids(doc)
        .into_iter()
        .filter(|x| !hidden.contains(x))
        .collect()
}

/// The reserved prefix of the authority's watch ids.
pub(crate) const WATCH_PREFIX: &str = "watch-";

/// The person principal `p` reads for (an agent: its `on_behalf`).
pub fn person_of<'a>(plane: &'a qp_plane::plane::DataPlane, p: &'a str) -> &'a str {
    plane.delegates.get(p).map_or(p, String::as_str)
}
