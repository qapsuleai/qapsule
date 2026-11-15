//! R3/M4 candidate **"kinddir"** — theme identity by meaning kind + direction.
//!
//! Premise: on one instrument (and series), facts of the same meaning kind and the same
//! direction tell the same story; a different kind is a different story. The details that
//! make this premise usable (all deterministic, no state outside the open stories):
//!
//! 1. **State exit** (`Edge::Exit`): joins the open story of the same kind and direction
//!    that holds an `enter` of the same detector (the most recently active one) → the
//!    engine records `weakened / state_ended`. No such story → `Ignore` (a finished state
//!    gives birth to nothing). No gap limit: an exit always belongs to its enter.
//! 2. **Same kind, same direction** (point events and state enters): join the open story
//!    of that kind and direction whose last own-kind fact is at most [`KindDir::gap_bars`]
//!    bars before this one (most recent first). Older than that → a **new** story: a second
//!    breakout after a long quiet stretch is a second episode, not the same one.
//! 3. **Opposite direction, same kind**: a separate story (`<kind>.<other direction>`).
//!    The premise treats "breakout up" and "breakout down" as two claims; the open story of
//!    the other direction is not touched (see NOTES: this is the premise's price).
//! 4. **Neutral direction**: a neutral fact only meets neutral stories of its kind (a
//!    neutral fact never opposes and never claims a direction it does not have).
//! 5. **Flow as evidence (K30)** — the one exception to the strict key. A directional
//!    `flow` fact (volume surge, institutional flow) that arrives at most
//!    [`KindDir::confirm_bars`] bars after the last own-kind fact of an open directional
//!    *price-meaning* story (breakout, reversal, continuation) joins that story instead of
//!    starting its own: same direction → `confirmed` (new family) / `strengthened`;
//!    opposite direction → `weakened / opposite_fact` (when
//!    [`KindDir::flow_opposite_weakens`]). Without this, a strict kind + direction key can
//!    never let volume confirm a breakout, and since every price detector is in the price
//!    family no price story could ever reach `confirmed`. Flow facts with no recent price
//!    story fall back to rule 2 (their own `flow.<dir>` story).
//!
//! Theme key: `<kind>.<direction>` (e.g. `breakout.up`); the engine adds instrument and
//! birth bar to the capsule id.

use qp_capsule::{OpenStory, ThemeDecision, ThemeRule};
use qp_schema::capsule::{Direction, Edge, MeaningKind};
use qp_schema::Fact;

/// Default join tolerance for same kind + direction, in bars.
pub const GAP_BARS: u64 = 10;
/// Default window in which a flow fact confirms a price-meaning story, in bars.
pub const CONFIRM_BARS: u64 = 1;

/// The "kind + direction" theme rule (see the crate docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindDir {
    /// A same kind + direction fact joins only if the story's last own-kind fact is at
    /// most this many bars earlier.
    pub gap_bars: u64,
    /// A flow fact attaches to a price-meaning story whose last own-kind fact is at most
    /// this many bars earlier.
    pub confirm_bars: u64,
    /// Whether flow facts may attach to price-meaning stories at all (rule 5). `false`
    /// gives the strict premise.
    pub flow_confirms: bool,
    /// Whether an opposite-direction flow fact weakens a recent price-meaning story
    /// (`true`) or starts its own flow story (`false`).
    pub flow_opposite_weakens: bool,
}

impl Default for KindDir {
    fn default() -> Self {
        Self {
            gap_bars: GAP_BARS,
            confirm_bars: CONFIRM_BARS,
            flow_confirms: true,
            flow_opposite_weakens: true,
        }
    }
}

/// Entry point for the harness (`harness-sim`, name `kinddir`).
#[must_use]
pub fn theme() -> Box<dyn ThemeRule> {
    Box::new(KindDir::default())
}

fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

fn opposite(a: Direction, b: Direction) -> bool {
    matches!(
        (a, b),
        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up)
    )
}

/// Kinds a flow fact may confirm or weaken: directional claims about price.
fn flow_attachable(k: MeaningKind) -> bool {
    matches!(
        k,
        MeaningKind::Breakout | MeaningKind::Reversal | MeaningKind::Continuation
    )
}

/// Bar of the story's latest fact of the story's own kind (flow facts that joined a
/// price story do not keep it alive). Falls back to the birth bar.
fn last_own_bar(s: &OpenStory) -> u64 {
    s.facts
        .iter()
        .filter(|f| f.meaning.as_ref().is_some_and(|m| m.kind == s.story.kind))
        .map(|f| f.bar)
        .max()
        .unwrap_or(s.story.born_bar)
}

/// Most recently active story among `cands` (latest own-kind fact; ties → the older
/// story, i.e. the earlier one in the list).
fn most_recent<'a>(cands: impl Iterator<Item = &'a &'a OpenStory>) -> Option<&'a OpenStory> {
    let mut best: Option<(&OpenStory, u64)> = None;
    for s in cands {
        let b = last_own_bar(s);
        if best.is_none_or(|(_, bb)| b > bb) {
            best = Some((s, b));
        }
    }
    best.map(|(s, _)| s)
}

impl ThemeRule for KindDir {
    fn name(&self) -> &'static str {
        "kinddir"
    }

    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let Some(m) = fact.meaning.as_ref() else {
            return ThemeDecision::Ignore;
        };
        let mine = || {
            open.iter()
                .filter(|s| s.story.series == fact.series && s.story.instrument == m.instrument)
        };

        // 1. State exit → the story holding its enter.
        if m.edge == Edge::Exit {
            let own = mine().filter(|s| {
                s.story.kind == m.kind
                    && s.story.direction == m.direction
                    && s.facts.iter().any(|g| {
                        g.meaning
                            .as_ref()
                            .is_some_and(|gm| gm.edge == Edge::Enter && gm.detector == m.detector)
                    })
            });
            return most_recent(own)
                .map_or(ThemeDecision::Ignore, |s| ThemeDecision::Join(s.id.clone()));
        }

        // 5. Directional flow → a recent price-meaning story (confirm / weaken).
        if self.flow_confirms && m.kind == MeaningKind::Flow && m.direction != Direction::Neutral {
            let recent = |s: &&&OpenStory| {
                flow_attachable(s.story.kind)
                    && last_own_bar(s).saturating_add(self.confirm_bars) >= fact.bar
            };
            let same = most_recent(
                mine()
                    .filter(recent)
                    .filter(|s| s.story.direction == m.direction),
            );
            let opp = if self.flow_opposite_weakens {
                most_recent(
                    mine()
                        .filter(recent)
                        .filter(|s| opposite(s.story.direction, m.direction)),
                )
            } else {
                None
            };
            // The most recently active one; on a tie, confirmation wins over weakening.
            let pick = match (same, opp) {
                (Some(a), Some(b)) => Some(if last_own_bar(b) > last_own_bar(a) {
                    b
                } else {
                    a
                }),
                (a, b) => a.or(b),
            };
            if let Some(s) = pick {
                return ThemeDecision::Join(s.id.clone());
            }
        }

        // 2–4. Same kind and direction, within the gap.
        let hit = most_recent(mine().filter(|s| {
            s.story.kind == m.kind
                && s.story.direction == m.direction
                && last_own_bar(s).saturating_add(self.gap_bars) >= fact.bar
        }));
        match hit {
            Some(s) => ThemeDecision::Join(s.id.clone()),
            None => ThemeDecision::New {
                theme: format!("{}.{}", m.kind.name(), dir_name(m.direction)),
            },
        }
    }
}

#[cfg(test)]
mod tests;
