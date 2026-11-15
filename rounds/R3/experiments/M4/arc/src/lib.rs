//! R3/M4 candidate theme rule **"arc"** (yön yayı, direction arc).
//!
//! Premise: on one instrument (and series), consecutive meanings in the same direction —
//! a breakout, its volume surge, the MA stack that follows, the next breakouts of the
//! trend, an institutional buy — are chapters of **one** story arc. The arc is the unit
//! the reader follows ("THYAO broke up, volume confirmed, the trend is going on"), not
//! the detector that happened to fire.
//!
//! The rule is **stateless**: everything it needs is in the open stories it is given
//! (their facts, `last_bar`, direction). That keeps it a pure function of its input,
//! which matters because the engine also runs it on scratch copies
//! ([`qp_capsule::Engine::candidates`]).
//!
//! # Definitions
//!
//! - **Effective direction** of a story: its own direction when directional; for a
//!   neutral story (a squeeze *prelude*) the direction of the first directional fact that
//!   joined it, else neutral.
//! - **Sealed** story: an arc that has ended its chapter list even though the engine still
//!   holds it open — it contains a reversal fact against its effective direction, or the
//!   exit of a continuation state (the trend it rode is over). Nothing new joins a sealed
//!   arc except the exit of a state it holds.
//! - **Strong / weak** chapter: flow and extreme are *weak* (they colour a move but do not
//!   make or end one on their own); breakout, continuation, reversal, squeeze are strong.
//! - **Current strong arc**: among the open, unsealed stories with a directional
//!   effective direction that hold a strong chapter, the one with the latest `last_bar`
//!   (ties: born later), if `fact.bar - last_bar <= gap`. **Weak arc**: the same among
//!   stories holding only weak chapters.
//!
//! # Decision (default parameters, [`ArcConfig::default`])
//!
//! 1. `exit` fact: join the most recent open story that holds an `enter` of the same
//!    detector (engine: `weakened / state_ended`), else ignore (an exit with no story
//!    says nothing on its own and the engine would drop it anyway).
//! 2. Neutral fact (squeeze enter, a doji volume surge): join the current arc (strong,
//!    else weak) — a pause inside a move; else an open unsealed prelude within `gap`;
//!    else start a new prelude `arc.neutral`.
//! 3. Directional fact, direction `d`, with a current strong arc:
//!    - arc in `d` → join (any kind: the arc spans kinds);
//!    - arc in `-d`: an opposite **reversal** joins it (`weakened / opposite_fact`) and
//!      seals it — the reversal is the arc's last chapter; an opposite **weak** fact goes
//!      to a *counter* side story `side.<kind>.<d>` and the arc stays current (one counter
//!      bar does not end a move, and the fact keeps its own glyph); any other opposite fact
//!      (breakout, continuation enter) **breaks** the arc: new arc `arc.<d>`, which is now
//!      the most recent and therefore current.
//! 4. Directional fact, no current strong arc: a strong fact resolves an open unsealed
//!    prelude whose last activity is within `prelude_gap` (the squeeze breaks in `d`);
//!    else a weak arc in `d` takes it; else a new arc `arc.<d>`.
//!
//! # Kind
//!
//! The engine fixes `story.kind` at birth, so an arc's kind is the kind of its **opening
//! chapter** (usually a breakout — the engine processes `breakout.*` first on a bar);
//! later chapters of other kinds are kept in the facts, history and families, not in the
//! glyph shape. See `NOTES.md`.

use qp_capsule::{OpenStory, ThemeDecision, ThemeRule};
use qp_schema::capsule::{Direction, Edge, MeaningKind};
use qp_schema::Fact;

/// Parameters of the arc rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcConfig {
    /// Most bars between an arc's last chapter and a new fact for it to continue.
    pub gap: u64,
    /// Most bars between a prelude's last activity and the directional fact resolving it.
    pub prelude_gap: u64,
    /// Whether a directional fact may resolve an open neutral prelude.
    pub prelude: bool,
    /// What an opposite weak fact (flow, extreme) does to the current arc.
    pub weak_opposite: WeakOpposite,
    /// Whether an opposite reversal joins and seals the arc (else it breaks it).
    pub reversal_seals: bool,
    /// Whether extreme facts in the arc's direction join it (else they go to a
    /// kind-and-direction side story like the reference).
    pub extreme_joins: bool,
}

/// Treatment of an opposite weak fact (flow, extreme) while an arc is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeakOpposite {
    /// Joins the arc (engine: `weakened / opposite_fact`); the arc goes on.
    Join,
    /// Starts (or joins) a counter story of its own; the arc stays current. (A "break"
    /// treatment would act the same: a story of weak chapters never becomes the current
    /// strong arc, so the old arc stays current either way.)
    Counter,
}

impl Default for ArcConfig {
    fn default() -> Self {
        Self {
            gap: 5,
            prelude_gap: 10,
            prelude: true,
            weak_opposite: WeakOpposite::Counter,
            reversal_seals: true,
            extreme_joins: true,
        }
    }
}

/// Weak kinds: they colour a move but do not make or end one on their own.
fn weak(k: MeaningKind) -> bool {
    matches!(k, MeaningKind::Flow | MeaningKind::Extreme)
}

/// Whether the story holds at least one non-weak fact (it can carry an arc).
fn has_strong(s: &OpenStory) -> bool {
    s.facts
        .iter()
        .filter_map(|f| f.meaning.as_ref())
        .any(|m| !weak(m.kind))
}

/// The direction-arc theme rule.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArcTheme {
    /// Parameters.
    pub cfg: ArcConfig,
}

/// Entry point for the harness (`harness-sim`, name `arc`).
#[must_use]
pub fn theme() -> Box<dyn qp_capsule::ThemeRule> {
    Box::new(ArcTheme::default())
}

/// The rule with explicit parameters (variants for notes and tests).
#[must_use]
pub fn theme_with(cfg: ArcConfig) -> Box<dyn qp_capsule::ThemeRule> {
    Box::new(ArcTheme { cfg })
}

fn opposite(a: Direction, b: Direction) -> bool {
    matches!(
        (a, b),
        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up)
    )
}

fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

/// Effective direction: the story's, or for a neutral prelude the first directional
/// non-exit fact that joined it.
#[must_use]
pub fn effective_direction(s: &OpenStory) -> Direction {
    if s.story.direction != Direction::Neutral {
        return s.story.direction;
    }
    s.facts
        .iter()
        .filter_map(|f| f.meaning.as_ref())
        .find(|m| m.edge != Edge::Exit && m.direction != Direction::Neutral)
        .map_or(Direction::Neutral, |m| m.direction)
}

/// Whether the arc has ended its chapter list (opposite reversal, or the exit of a
/// continuation state it holds).
#[must_use]
pub fn sealed(s: &OpenStory) -> bool {
    let d = effective_direction(s);
    s.facts.iter().filter_map(|f| f.meaning.as_ref()).any(|m| {
        (m.kind == MeaningKind::Reversal && opposite(d, m.direction))
            || (m.kind == MeaningKind::Continuation && m.edge == Edge::Exit)
    })
}

fn within(fact_bar: u64, last_bar: u64, gap: u64) -> bool {
    fact_bar >= last_bar && fact_bar - last_bar <= gap
}

impl ArcTheme {
    /// Most recently active open story satisfying `pred` (ties: born later).
    fn latest<'a>(
        open: &[&'a OpenStory],
        series: &str,
        pred: impl Fn(&OpenStory) -> bool,
    ) -> Option<&'a OpenStory> {
        let mut best: Option<&'a OpenStory> = None;
        for s in open.iter().copied() {
            if s.story.series != series || !pred(s) {
                continue;
            }
            if best.is_none_or(|b| s.story.last_bar >= b.story.last_bar) {
                best = Some(s);
            }
        }
        best
    }

    /// The current strong arc: the latest active unsealed directional story holding a
    /// strong (non-weak) chapter, within `gap`.
    fn strong_arc<'a>(&self, fact: &Fact, open: &[&'a OpenStory]) -> Option<&'a OpenStory> {
        Self::latest(open, &fact.series, |s| {
            effective_direction(s) != Direction::Neutral && !sealed(s) && has_strong(s)
        })
        .filter(|s| within(fact.bar, s.story.last_bar, self.cfg.gap))
    }

    /// The current weak arc: the latest active directional story holding only weak
    /// chapters (flow, extreme), within `gap`.
    fn weak_arc<'a>(&self, fact: &Fact, open: &[&'a OpenStory]) -> Option<&'a OpenStory> {
        Self::latest(open, &fact.series, |s| {
            effective_direction(s) != Direction::Neutral && !has_strong(s)
        })
        .filter(|s| within(fact.bar, s.story.last_bar, self.cfg.gap))
    }

    /// A side story for a weak fact kept out of the arc: the latest open story of the
    /// same kind and direction holding only weak facts, within `gap`; else a new one.
    fn side(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let Some(m) = fact.meaning.as_ref() else {
            return ThemeDecision::Ignore;
        };
        let hit = Self::latest(open, &fact.series, |s| {
            !has_strong(s) && s.story.kind == m.kind && s.story.direction == m.direction
        })
        .filter(|s| within(fact.bar, s.story.last_bar, self.cfg.gap));
        match hit {
            Some(s) => ThemeDecision::Join(s.id.clone()),
            None => ThemeDecision::New {
                theme: format!("side.{}.{}", m.kind.name(), dir_name(m.direction)),
            },
        }
    }

    fn prelude<'a>(fact: &Fact, open: &[&'a OpenStory], gap: u64) -> Option<&'a OpenStory> {
        let s = Self::latest(open, &fact.series, |s| {
            effective_direction(s) == Direction::Neutral && !sealed(s)
        })?;
        within(fact.bar, s.story.last_bar, gap).then_some(s)
    }
}

fn new_arc(d: Direction) -> ThemeDecision {
    ThemeDecision::New {
        theme: format!("arc.{}", dir_name(d)),
    }
}

impl ThemeRule for ArcTheme {
    fn name(&self) -> &'static str {
        "arc"
    }

    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let Some(m) = fact.meaning.as_ref() else {
            return ThemeDecision::Ignore;
        };

        // 1. A state's end goes to the story holding its start.
        if m.edge == Edge::Exit {
            let own = Self::latest(open, &fact.series, |s| {
                s.facts.iter().any(|g| {
                    g.meaning
                        .as_ref()
                        .is_some_and(|gm| gm.edge == Edge::Enter && gm.detector == m.detector)
                })
            });
            return own.map_or(ThemeDecision::Ignore, |s| ThemeDecision::Join(s.id.clone()));
        }

        if m.kind == MeaningKind::Extreme && !self.cfg.extreme_joins {
            return self.side(fact, open);
        }
        let strong = self.strong_arc(fact, open);

        // 2. Neutral: a pause inside the current arc, or a prelude.
        if m.direction == Direction::Neutral {
            if let Some(s) = strong.or_else(|| self.weak_arc(fact, open)) {
                return ThemeDecision::Join(s.id.clone());
            }
            if let Some(s) = Self::prelude(fact, open, self.cfg.gap) {
                return ThemeDecision::Join(s.id.clone());
            }
            return new_arc(Direction::Neutral);
        }

        // 3. Directional, against the current strong arc.
        if let Some(s) = strong {
            if effective_direction(s) == m.direction {
                return ThemeDecision::Join(s.id.clone());
            }
            if m.kind == MeaningKind::Reversal && self.cfg.reversal_seals {
                return ThemeDecision::Join(s.id.clone());
            }
            if weak(m.kind) {
                match self.cfg.weak_opposite {
                    WeakOpposite::Join => return ThemeDecision::Join(s.id.clone()),
                    WeakOpposite::Counter => return self.side(fact, open),
                }
            }
            return new_arc(m.direction);
        }
        // No strong arc: a strong fact may resolve a squeeze prelude.
        if self.cfg.prelude && !weak(m.kind) {
            if let Some(s) = Self::prelude(fact, open, self.cfg.prelude_gap) {
                return ThemeDecision::Join(s.id.clone());
            }
        }
        // A weak arc (flow / extreme only) in the same direction takes the fact.
        if let Some(s) = self.weak_arc(fact, open) {
            if effective_direction(s) == m.direction {
                return ThemeDecision::Join(s.id.clone());
            }
        }
        new_arc(m.direction)
    }
}
