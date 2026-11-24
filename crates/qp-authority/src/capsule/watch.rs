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
use qp_ops::watch::{on, person_of};
use qp_schema::capsule::Stage;
use qp_schema::protocol::{ChangeOp, Event, OutboundLine, Seq, ServerMsg, Topic};
use qp_schema::{Document, Intent, IntentState, WatchMatch};

use crate::session::{out, Session};

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

impl Session {
    /// (Privacy, §25.5) Whether reader `reader` (a projection's principal; empty: the
    /// host) may receive intent `i`: an intent without a watch, the reader's own watch
    /// (an agent: its person's), or any watch for an `owner`.
    pub(crate) fn intent_for_reader(&self, i: &Intent, reader: &str) -> bool {
        let Some(w) = &i.watch else { return true };
        reader.is_empty()
            || w.for_ == person_of(&self.plane, reader)
            || self
                .principal(reader)
                .is_some_and(|p| p.role == qp_schema::protocol::Role::Owner)
    }

    #[allow(clippy::expect_used)]
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

#[cfg(test)]
mod params_binding {
    // RG1/K3: the constant reads `qp_schema::params` and keeps its old literal.
    #[test]
    fn max_watches_binds_registry() {
        assert_eq!(qp_ops::watch::MAX_WATCHES, 32);
    }
}
