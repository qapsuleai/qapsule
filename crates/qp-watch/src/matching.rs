//! Matching watches against the capsule engine's story transitions (moved from
//! `qp-authority` `capsule::watch`, RF1 S3.1), and their `intent_matched` events.

use qp_capsule::StoryEvent;
use qp_ops::watch::{on, person_of};
use qp_schema::capsule::Stage;
use qp_schema::protocol::{ChangeOp, Event, OutboundLine, Seq, ServerMsg, Topic};
use qp_schema::{Document, Intent, IntentState, WatchMatch};
use qp_subsystem::SessionView;

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

/// The watches' changes and matches for one engine look's `events` in `doc` at clock
/// `clock` (the world the look runs in).
pub(crate) fn match_watches(
    doc: &Document,
    clock: i64,
    events: &[StoryEvent],
) -> (Vec<ChangeOp>, Vec<Matched>) {
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
        let Some(w) = i.watch.as_ref() else { continue };
        if w.expires_t.is_some_and(|t| clock >= t) {
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
            let Some(w) = i.watch.clone() else { continue };
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
    view: &dyn SessionView,
    full: &[Matched],
    seq: Seq,
    public: &[Matched],
    pub_seq: Option<Seq>,
) -> Vec<OutboundLine> {
    let mut lines = Vec::new();
    for cid in view.conn_ids() {
        let subs: Vec<&str> = view
            .subs(cid)
            .into_iter()
            .filter(|(_, t)| matches!(t, Topic::Intents {}))
            .map(|(id, _)| id)
            .collect();
        if subs.is_empty() {
            continue;
        }
        let Some(c) = view.conn(cid) else { continue };
        let pv = view.public_view(c.principal);
        let (items, at) = if pv {
            match pub_seq {
                Some(p) => (public, p),
                None => continue,
            }
        } else {
            (full, seq)
        };
        let person = person_of(view.plane(), c.principal);
        let proj = if pv {
            None
        } else {
            view.reader_proj(c.principal)
        };
        for m in items {
            if m.person != person {
                continue;
            }
            if proj.as_ref().is_some_and(|p| p.hidden.contains(&m.capsule)) {
                continue;
            }
            for sub in &subs {
                lines.push(OutboundLine {
                    conn: cid.to_owned(),
                    msg: ServerMsg::Notify {
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
                });
            }
        }
    }
    lines
}
