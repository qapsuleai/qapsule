//! (R5B, `docs/PROTOCOL.md` §25.5) Watches: structured intents on story transitions,
//! matched deterministically by the engine.
//!
//! - `add_watch` → `put_intent` with `watch` (state `active`); `remove_watch` →
//!   `put_intent` with `active: false`, state `removed`.
//! - After the capsule engine's look (one engine entry per watched series and look),
//!   every active watch is tested against the look's story transitions in a fixed order
//!   (transitions in the engine's order, watches in document order). The engine entry
//!   itself carries the watches' new counts and states (`put_intent`): `matched` once
//!   `max_matches` is reached, `expired` once the clock reaches `expires_t`. Same input,
//!   same entries, same events.
//! - Each match is an `intent_matched` event on the `intents` topic, to the watch's
//!   person (and the agents working for them), projected: a public-view reader gets the
//!   public world's matches, a reader with a projection none about a capsule it may not
//!   see.

use qp_capsule::StoryEvent;
use qp_schema::capsule::Stage;
use qp_schema::protocol::{ChangeOp, ErrorCode, Event, Op, OutboundLine, Seq, ServerMsg, Topic};
use qp_schema::{Document, Intent, IntentState, Watch, WatchMatch};

use crate::error::{invalid, ptr, PResult, E};
use crate::expand::Ctx;
use crate::session::{out, Session};

/// Active watches per person at most (`welcome.limits.max_watches`).
pub const MAX_WATCHES: u32 = 32;

/// One match of a watch.
#[derive(Debug, Clone)]
pub(crate) struct Matched {
    pub intent: String,
    pub person: String,
    pub capsule: String,
    pub stage: Stage,
    pub bar: u64,
    pub t: i64,
}

/// Whether a watch is on (active, state `active`).
fn on(i: &Intent) -> bool {
    i.active && i.watch.is_some() && i.state == Some(IntentState::Active)
}

fn negative(s: Stage) -> bool {
    matches!(s, Stage::Weakened | Stage::Invalidated | Stage::Retracted)
}

fn holds(doc: &Document, person: &str, instrument: &str) -> bool {
    doc.audiences
        .get(person)
        .and_then(|a| a.positions.get(instrument))
        .is_some_and(|q| *q != 0.0)
}

/// Whether event `e` matches `m` for `person` in `doc`.
fn matches(doc: &Document, person: &str, m: &WatchMatch, e: &StoryEvent) -> bool {
    let st = &e.story;
    let stage = e.transition.stage;
    m.instrument.as_ref().is_none_or(|i| i == &st.instrument)
        && (!m.holdings || holds(doc, person, &st.instrument))
        && m.meaning.as_ref().is_none_or(|k| k.contains(&st.kind))
        && m.direction.is_none_or(|d| d == st.direction)
        && m.stages.as_ref().is_none_or(|s| s.contains(&stage))
        && (!m.negative || negative(stage))
}

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
    let may =
        |who: &str, ctx: &Ctx<'_>| who == ctx.speaker || who == person_of_speaker || ctx.is_owner;
    match op {
        Op::AddWatch {
            id,
            for_,
            text,
            match_,
            expires_t,
            max_matches,
        } => {
            let person = for_.clone().unwrap_or_else(|| person_of_speaker.clone());
            if !may(&person, ctx) {
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
            let iid = ctx.new_id(doc, "intent", id.as_ref(), &ptr(p, "id"))?;
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
            let Some(i) = doc
                .intents
                .iter()
                .find(|i| &i.id == intent && i.watch.is_some())
            else {
                return Err(E::new(ErrorCode::NotFound, format!("no watch `{intent}`"))
                    .at(ip)
                    .dym(
                        intent,
                        doc.intents
                            .iter()
                            .filter(|i| i.watch.is_some())
                            .map(|i| i.id.as_str()),
                    )
                    .build());
            };
            let person = i.watch.as_ref().map(|w| w.for_.clone()).unwrap_or_default();
            if !may(&person, ctx) {
                return Err(E::new(
                    ErrorCode::Forbidden,
                    format!("the watch `{intent}` is `{person}`'s"),
                )
                .at(ip)
                .build());
            }
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

impl Session {
    /// The watches' changes and matches for one engine look's `events` (in the world the
    /// fields hold).
    pub(crate) fn match_watches(&self, events: &[StoryEvent]) -> (Vec<ChangeOp>, Vec<Matched>) {
        let doc = &self.doc;
        let mut state: Vec<(usize, Intent, bool)> = Vec::new();
        for (k, i) in doc.intents.iter().enumerate() {
            if on(i) {
                state.push((k, i.clone(), false));
            }
        }
        if state.is_empty() {
            return (Vec::new(), Vec::new());
        }
        // Expiry first: a watch past its time matches nothing more.
        for (_, i, changed) in &mut state {
            let w = i.watch.as_ref().expect("on");
            if w.expires_t.is_some_and(|t| self.clock >= t) {
                i.active = false;
                i.state = Some(IntentState::Expired);
                *changed = true;
            }
        }
        let mut found = Vec::new();
        for e in events {
            for (_, i, changed) in &mut state {
                if !i.active {
                    continue;
                }
                let w = i.watch.as_ref().expect("on").clone();
                if !matches(doc, &w.for_, &w.match_, e) {
                    continue;
                }
                i.matches += 1;
                *changed = true;
                found.push(Matched {
                    intent: i.id.clone(),
                    person: w.for_.clone(),
                    capsule: e.capsule.clone(),
                    stage: e.transition.stage,
                    bar: e.transition.bar,
                    t: e.transition.t,
                });
                if w.max_matches.is_some_and(|m| i.matches >= m) {
                    i.active = false;
                    i.state = Some(IntentState::Matched);
                }
            }
        }
        let changes = state
            .into_iter()
            .filter(|(_, _, c)| *c)
            .map(|(_, i, _)| ChangeOp::PutIntent {
                intent: i,
                index: None,
            })
            .collect();
        (changes, found)
    }

    /// `intent_matched` events: `full` (the authority's matches, entry `seq`) to the
    /// readers of its world, `public` (entry `pub_seq`) to the public-view readers; each
    /// to the watch's person and the agents working for them, on `intents`.
    pub(crate) fn watch_lines(
        &self,
        full: &[Matched],
        seq: Seq,
        public: &[Matched],
        pub_seq: Option<Seq>,
    ) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for c in &self.conns {
            let subs: Vec<&str> = c
                .subs
                .iter()
                .filter(|s| matches!(s.topic, Topic::Intents {}))
                .map(|s| s.id.as_str())
                .collect();
            if subs.is_empty() {
                continue;
            }
            let pv = self.other.is_some() && self.public_view(&c.principal);
            let (items, at) = if pv {
                match pub_seq {
                    Some(p) => (public, p),
                    None => continue,
                }
            } else {
                (full, seq)
            };
            let person = self
                .plane
                .delegates
                .get(&c.principal)
                .cloned()
                .unwrap_or_else(|| c.principal.clone());
            let proj = if pv {
                None
            } else {
                self.reader_proj(&c.principal)
            };
            for m in items {
                if m.person != person {
                    continue;
                }
                if proj.as_ref().is_some_and(|p| p.hidden.contains(&m.capsule)) {
                    continue;
                }
                for sub in &subs {
                    lines.push(out(
                        &c.id,
                        ServerMsg::Notify {
                            sub: (*sub).to_owned(),
                            event: Event::IntentMatched {
                                intent: m.intent.clone(),
                                capsule: m.capsule.clone(),
                                stage: m.stage,
                                bar: m.bar,
                                t: m.t,
                                seq: at,
                            },
                        },
                    ));
                }
            }
        }
        lines
    }
}
