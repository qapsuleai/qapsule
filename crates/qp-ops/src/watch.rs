//! (R5B, `docs/PROTOCOL.md` §25.5) The pure part of watches: the `add_watch` /
//! `remove_watch` expansion and the privacy rule of who may see a watch. Matching a watch
//! against story transitions stays with the authority (`qp-session` `capsule::watch`).

use qp_schema::protocol::{ChangeOp, ErrorCode, Op};
use qp_schema::{Document, Intent, IntentState, Watch, WatchMatch};

use crate::expand::Ctx;
use qp_plane::error::{invalid, ptr, PResult, E};

/// Active watches per person at most (`welcome.limits.max_watches`).
pub const MAX_WATCHES: u32 = qp_schema::params::system::WATCH_MAX_PER_PERSON_DEFAULT;

/// Whether a watch is on (active, state `active`).
pub fn on(i: &Intent) -> bool {
    i.active && i.watch.is_some() && i.state == Some(IntentState::Active)
}

#[allow(clippy::too_many_lines)]
/// `add_watch` and `remove_watch` (§25.5).
pub(crate) fn expand_watch(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let person_of_speaker = ctx
        .plane
        .delegates
        .get(ctx.speaker)
        .cloned()
        .unwrap_or_else(|| ctx.speaker.to_owned());
    match op {
        Op::AddWatch {
            for_,
            text,
            match_,
            expires_t,
            max_matches,
        } => {
            let person = for_.clone().unwrap_or_else(|| person_of_speaker.clone());
            let probe = Watch {
                for_: person.clone(),
                match_: WatchMatch::default(),
                expires_t: None,
                max_matches: None,
            };
            if !watch_visible(&probe, ctx) {
                return Err(E::new(
                    ErrorCode::Forbidden,
                    format!("a watch for `{person}` is theirs to leave"),
                )
                .at(ptr(p, "for"))
                .hint("Leave it for yourself (or the person you work for); omit `for`.")
                .build());
            }
            if text.trim().is_empty() {
                return Err(invalid(
                    "a watch needs its request in words",
                    ptr(p, "text"),
                ));
            }
            if *max_matches == Some(0) {
                return Err(E::new(ErrorCode::Invalid, "max_matches must be at least 1")
                    .at(ptr(p, "max_matches"))
                    .expected(["1.."])
                    .build());
            }
            let n = doc
                .intents
                .iter()
                .filter(|i| on(i) && i.watch.as_ref().is_some_and(|w| w.for_ == person))
                .count();
            if n >= MAX_WATCHES as usize {
                return Err(E::new(
                    ErrorCode::Limit,
                    format!("`{person}` has {n} active watches already"),
                )
                .at(p.to_owned())
                .expected([MAX_WATCHES.to_string()])
                .hint("Remove a watch (`remove_watch`) first.")
                .build());
            }
            let iid = ctx.new_id(doc, "watch", None, &ptr(p, "id"))?;
            Ok(vec![ChangeOp::PutIntent {
                index: None,
                intent: Intent {
                    id: iid,
                    text: text.clone(),
                    trigger: None,
                    active: true,
                    provenance: ctx.prov(Vec::new()),
                    watch: Some(Watch {
                        for_: person,
                        match_: match_.clone(),
                        expires_t: *expires_t,
                        max_matches: *max_matches,
                    }),
                    state: Some(IntentState::Active),
                    matches: 0,
                },
            }])
        }
        Op::RemoveWatch { intent } => {
            let ip = ptr(p, "intent");
            // (Privacy) Another person's watch reads as a missing one; suggestions come
            // from the speaker's own watches only.
            let mine = |i: &&Intent| i.watch.as_ref().is_some_and(|w| watch_visible(w, ctx));
            let Some(i) = doc.intents.iter().filter(mine).find(|i| &i.id == intent) else {
                return Err(E::new(ErrorCode::NotFound, format!("no watch `{intent}`"))
                    .at(ip)
                    .dym(
                        intent,
                        doc.intents.iter().filter(mine).map(|i| i.id.as_str()),
                    )
                    .build());
            };
            if !on(i) {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("the watch `{intent}` is not active"),
                )
                .at(ip)
                .build());
            }
            let mut ni = i.clone();
            ni.active = false;
            ni.state = Some(IntentState::Removed);
            Ok(vec![ChangeOp::PutIntent {
                intent: ni,
                index: None,
            }])
        }
        _ => unreachable!("only watch ops"),
    }
}

/// (Privacy, §25.5) The one rule for who may see (and so touch, or learn the existence
/// of) a watch: an `owner`, its person, or an agent working for its person.
pub(crate) fn watch_visible(w: &Watch, ctx: &Ctx<'_>) -> bool {
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
