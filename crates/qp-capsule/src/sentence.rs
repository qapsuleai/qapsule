//! Sentences from the engine (K26, `docs/CAPSULE.md` §9.5).
//!
//! [`sentences_at`] writes a story's TR and EN sentences **as of bar `now`** from engine
//! data only:
//!
//! - the story is taken as it was after its last transition at or before `now` (the
//!   engine's event log), the facts as those that joined at or before `now`;
//! - [`condition_now`] computes `still_holds` and `current` for the lead fact's condition
//!   from `bars[..=now]` with `qp-compute` studies (deterministic; nothing after `now`);
//! - `redacted` is set when the lead fact is licensed and the reader's entitlements do
//!   not cover it (K33): the realizer then writes no digit at all.
//!
//! # Condition subjects
//!
//! | Subject | Value at `now` |
//! |---------|----------------|
//! | `rsi`, `value` | RSI(14) of close ([`qp_facts::RsiExtreme::RSI`]; the same study the detector uses) |
//! | `close`, `price` | close |
//! | `close_dev` | `|close − middle|` of Bollinger(20, 2) |
//! | `bandwidth`, `bw` | Bollinger(20, 2) bandwidth |
//! | anything else (`net`: licensed flow, not in the bars) | unknown → `None` |
//!
//! The threshold is the lead fact's evidence value of the condition key, except for keys
//! that name a moving study, which are recomputed at `now`: `sma20`, `sma50`, `middle`,
//! `half_width` (`upper − middle`). The sentence still prints the lead fact's evidence
//! value as the threshold (the fact's frozen evidence), while `still_holds` compares the
//! current subject with the current threshold.
//!
//! # State facts
//!
//! - If an `exit` fact of the lead's detector joined the story at or before `now`, the
//!   state ended: `still_holds = false` (the detector said so), `current` = the subject
//!   now.
//! - (R4, K42/A28: [`split_condition`]) A state whose condition has `within_bars` is not a
//!   holding condition (state condition) but the state's
//!   expected resolution (`squeeze.bandwidth`: "a close outside the band within 20 bars").
//!   Its truth is not whether the state holds, so the sentence uses the lead fact without
//!   that condition: `still_holds` = no exit yet and the story still open, `current =
//!   None`.
//! - A story closed as `invalidated`, `expired` or `retracted` is never "holding".

use qp_compute::{Bollinger, Rsi, Sma};
use qp_schema::capsule::{
    Edge, FactMeaning, License, LicenseClass, MeaningCondition, Stage, Story, TimeKind, Validity,
};
use qp_schema::{Bar, CompareOp, Fact};

use crate::engine::Engine;
use crate::{Lang, Realizer, SentenceInput};

/// Who reads the sentence: the data entitlements they hold (`docs/PROTOCOL.md` §18.3
/// form: `*`, `SYMBOL`, `feed/SYMBOL`, `feed/*`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reader {
    /// Entitlement entries.
    pub entitlements: Vec<String>,
}

impl Reader {
    /// A reader with every entitlement (`*`).
    #[must_use]
    pub fn entitled() -> Self {
        Self {
            entitlements: vec!["*".to_owned()],
        }
    }

    /// A reader with no entitlement (public data only).
    #[must_use]
    pub fn unentitled() -> Self {
        Self::default()
    }

    /// Whether the reader may see the numbers of a fact with `license` about
    /// `instrument`. Public data: always. Licensed data: the licence's entitlement
    /// (`feed/*` or `feed/SYMBOL`; `*` = the fact's instrument) must be covered by one of
    /// the reader's entries, with the §18.3 matching rule.
    #[must_use]
    pub fn may_see(&self, license: &License, instrument: &str) -> bool {
        if license.class == LicenseClass::Public {
            return true;
        }
        let Some(need) = license.entitlement.as_deref() else {
            return self.entitlements.iter().any(|e| e == "*");
        };
        let (feed, symbol) = match need.split_once('/') {
            Some((f, "*")) => (Some(f), instrument),
            Some((f, s)) => (Some(f), s),
            None => (None, need),
        };
        self.entitlements.iter().any(|e| covers(e, feed, symbol))
    }
}

/// §18.3: `*` covers everything; `SYMBOL` that symbol on any feed; `feed/SYMBOL` and
/// `feed/*` only that named feed.
fn covers(entry: &str, feed: Option<&str>, symbol: &str) -> bool {
    match entry.split_once('/') {
        _ if entry == "*" => true,
        None => entry == symbol,
        Some((f, s)) => feed == Some(f) && (s == "*" || s == symbol),
    }
}

/// What [`sentences_at`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentences {
    /// Capsule id.
    pub capsule: String,
    /// Bar the sentences are written for.
    pub now_bar: u64,
    /// Stage of the story as of `now`.
    pub stage: Stage,
    /// Whether the lead fact's condition / state still holds at `now` (K26).
    pub still_holds: Option<bool>,
    /// Current value of the condition subject (`None` when unknown or redacted).
    pub current: Option<f64>,
    /// No digits for this reader (K33).
    pub redacted: bool,
    /// (R4, K42/A28) Validity at `now`: the same value the glyph carries
    /// ([`validity_now`]).
    pub validity: Validity,
    /// Turkish sentence.
    pub tr: String,
    /// English sentence.
    pub en: String,
}

/// Study values at the last bar of one bars slice, each computed at most once. A caller
/// that writes many sentences over the same series and `now` in one pass (the authority
/// rewriting every capsule's sentence) shares one memo; the values are the ones
/// [`condition_now`] computes, bit for bit.
///
/// Entries are keyed by the slice's address and length: a memo is valid only while every
/// slice it has seen stays unchanged (one pass over borrowed bars). Drop it before the
/// bars change.
#[derive(Debug, Default)]
pub struct StudyMemo {
    at: Vec<((usize, usize), Studies)>,
}

#[derive(Debug, Default, Clone, Copy)]
#[allow(clippy::option_option)]
struct Studies {
    rsi: Option<Option<f64>>,
    sma20: Option<Option<f64>>,
    sma50: Option<Option<f64>>,
    bollinger: Option<Option<(f64, f64, f64)>>,
}

impl StudyMemo {
    fn slot(&mut self, bars: &[Bar]) -> &mut Studies {
        let key = (bars.as_ptr() as usize, bars.len());
        let i = if let Some(i) = self.at.iter().rposition(|(k, _)| *k == key) {
            i
        } else {
            self.at.push((key, Studies::default()));
            self.at.len() - 1
        };
        &mut self.at[i].1
    }

    fn rsi(&mut self, bars: &[Bar]) -> Option<f64> {
        *self
            .slot(bars)
            .rsi
            .get_or_insert_with(|| rsi_now(bars, qp_facts::RsiExtreme::RSI))
    }

    fn sma(&mut self, bars: &[Bar], period: usize) -> Option<f64> {
        let s = self.slot(bars);
        let v = match period {
            20 => &mut s.sma20,
            50 => &mut s.sma50,
            _ => return sma_now(bars, period),
        };
        *v.get_or_insert_with(|| sma_now(bars, period))
    }

    fn bollinger(&mut self, bars: &[Bar]) -> Option<(f64, f64, f64)> {
        *self
            .slot(bars)
            .bollinger
            .get_or_insert_with(|| bollinger_now(bars))
    }
}

/// Value of a condition subject at the last bar of `bars`.
fn subject_now(subject: &str, bars: &[Bar], memo: &mut StudyMemo) -> Option<f64> {
    let last = bars.last()?;
    match subject {
        "close" | "price" => Some(last.c),
        "rsi" | "value" => memo.rsi(bars),
        "close_dev" => memo.bollinger(bars).map(|(m, _, _)| (last.c - m).abs()),
        "bandwidth" | "bw" => memo.bollinger(bars).map(|(_, _, bw)| bw),
        _ => None,
    }
    .filter(|v: &f64| v.is_finite())
}

/// Threshold of a condition key at the last bar of `bars`: moving studies recomputed,
/// everything else from the lead fact's evidence.
fn threshold_now(key: &str, lead: &Fact, bars: &[Bar], memo: &mut StudyMemo) -> Option<f64> {
    match key {
        "sma20" => memo.sma(bars, 20),
        "sma50" => memo.sma(bars, 50),
        "middle" => memo.bollinger(bars).map(|(m, _, _)| m),
        "half_width" => memo.bollinger(bars).map(|(m, u, _)| u - m),
        k => lead.evidence.get(k).copied(),
    }
    .filter(|v: &f64| v.is_finite())
}

fn rsi_now(bars: &[Bar], period: usize) -> Option<f64> {
    let (last, head) = bars.split_last()?;
    let mut r = Rsi::new(period).ok()?;
    for b in head {
        r.commit1(b.c);
    }
    r.peek1(last.c)
}

fn sma_now(bars: &[Bar], period: usize) -> Option<f64> {
    let (last, head) = bars.split_last()?;
    let mut s = Sma::new(period).ok()?;
    for b in head {
        s.commit1(b.c);
    }
    s.peek1(last.c)
}

/// Bollinger(20, 2) at the last bar: `(middle, upper, bandwidth)`.
fn bollinger_now(bars: &[Bar]) -> Option<(f64, f64, f64)> {
    let (last, head) = bars.split_last()?;
    let mut s = Bollinger::new(20, 2.0).ok()?;
    for b in head {
        s.commit1(b.c);
    }
    s.peek1(last.c).map(|v| (v.middle, v.upper, v.bandwidth))
}

fn compare(op: CompareOp, a: f64, b: f64) -> bool {
    match op {
        CompareOp::Lt | CompareOp::CrossBelow => a < b,
        CompareOp::Le => a <= b,
        CompareOp::Gt | CompareOp::CrossAbove => a > b,
        CompareOp::Ge => a >= b,
    }
}

/// `(still_holds, current)` of the lead fact's condition at the last bar of `bars`
/// (`bars` = the story's series up to and including `now`; nothing later is read).
/// `facts` are the story's facts that joined at or before `now` (an `exit` of the lead's
/// detector ends a state). See the module docs.
#[must_use]
pub fn condition_now(lead: &Fact, facts: &[Fact], bars: &[Bar]) -> (Option<bool>, Option<f64>) {
    let facts: Vec<&Fact> = facts.iter().collect();
    condition_now_memo(lead, &facts, bars, &mut StudyMemo::default())
}

/// [`condition_now`] over borrowed facts, with the study values of `bars` taken from (and
/// kept in) `memo` (see [`StudyMemo`]). Same result, bit for bit.
#[must_use]
pub fn condition_now_memo(
    lead: &Fact,
    facts: &[&Fact],
    bars: &[Bar],
    memo: &mut StudyMemo,
) -> (Option<bool>, Option<f64>) {
    let Some(m) = lead.meaning.as_ref() else {
        return (None, None);
    };
    let exited = m.time == TimeKind::State
        && facts.iter().any(|f| {
            f.meaning
                .as_ref()
                .is_some_and(|fm| fm.edge == Edge::Exit && fm.detector == m.detector)
        });
    // The state condition decides; an expectation keeps its own condition (R3 rule).
    let (state, resolution) = split_condition(m);
    let tracked = if m.time == TimeKind::Expectation {
        resolution
    } else {
        state
    };
    let Some(c) = tracked else {
        return if m.time == TimeKind::State {
            (Some(!exited), None)
        } else {
            (None, None)
        };
    };
    let cur = subject_now(&c.subject, bars, memo);
    if exited {
        return (Some(false), cur);
    }
    let thr = threshold_now(&c.key, lead, bars, memo);
    let holds = match (cur, thr) {
        (Some(a), Some(b)) => Some(compare(c.op, a, b)),
        _ => None,
    };
    (holds, cur)
}

/// Sentences of story `capsule` as of bar `now` for `reader` (see the module docs).
/// `bars` is the story's bar series (anything after `now` is ignored). `None` when the
/// capsule is unknown, was not born yet at `now`, or `bars` does not reach `now`.
#[must_use]
pub fn sentences_at(
    engine: &Engine,
    realizer: &dyn Realizer,
    capsule: &str,
    bars: &[Bar],
    now: u64,
    reader: &Reader,
) -> Option<Sentences> {
    let now_i = usize::try_from(now).ok()?;
    let bars = bars.get(..=now_i)?;
    let rec = engine
        .open_stories()
        .iter()
        .chain(engine.closed_stories())
        .find(|s| s.id == capsule)?;
    let story = engine
        .events()
        .iter()
        .rfind(|e| e.capsule == capsule && e.transition.bar <= now)
        .map(|e| &e.story)?;
    let facts: Vec<Fact> = rec.facts.iter().filter(|f| f.bar <= now).cloned().collect();
    let lead = facts.first()?;
    let latest = facts.last()?;
    let (still_holds, current) = condition_now(lead, &facts, bars);
    let still_holds = closed_override(story, still_holds);
    let validity = validity_now(story, still_holds);
    let m = lead.meaning.as_ref()?;
    let redacted = !reader.may_see(&m.license, &m.instrument);
    // A state's expected resolution is not its state condition (module docs).
    let lead_view = sentence_lead(lead);
    let lead_ref = lead_view.as_ref().unwrap_or(lead);
    let current = if redacted { None } else { current };
    let inp = SentenceInput {
        story,
        lead: lead_ref,
        latest: if latest.id == lead.id {
            lead_ref
        } else {
            latest
        },
        now_bar: now,
        now_t: bars[now_i].t,
        still_holds,
        current,
        redacted,
    };
    Some(Sentences {
        capsule: capsule.to_owned(),
        now_bar: now,
        stage: story.stage,
        still_holds,
        current,
        redacted,
        validity,
        tr: realizer.sentence(&inp, Lang::Tr),
        en: realizer.sentence(&inp, Lang::En),
    })
}

/// (R4, K42/A28) The two roles of a fact's condition: `(state condition, expected
/// resolution)`.
///
/// - an expectation's condition is what resolves it: `(None, Some(c))`;
/// - a state's condition bounded by `within_bars` is the state's expected resolution
///   (`squeeze.bandwidth`: "a close outside the band within 20 bars"): `(None, Some(c))`;
/// - every other condition (an event's "the close is still beyond the level", a state's
///   holding condition) is a state condition: `(Some(c), None)`.
#[must_use]
pub fn split_condition(m: &FactMeaning) -> (Option<&MeaningCondition>, Option<&MeaningCondition>) {
    match m.condition.as_ref() {
        None => (None, None),
        Some(c)
            if m.time == TimeKind::Expectation
                || (m.time == TimeKind::State && c.within_bars.is_some()) =>
        {
            (None, Some(c))
        }
        Some(c) => (Some(c), None),
    }
}

/// The lead fact as the realizer reads it: a state whose condition is its expected
/// resolution is written without that condition (it does not say whether the state
/// holds). `None`: the lead as is.
#[must_use]
pub fn sentence_lead(lead: &Fact) -> Option<Fact> {
    let m = lead.meaning.as_ref()?;
    if m.time == TimeKind::State && split_condition(m).1.is_some() {
        let mut l = lead.clone();
        if let Some(lm) = l.meaning.as_mut() {
            lm.condition = None;
        }
        Some(l)
    } else {
        None
    }
}

/// A state story closed as `invalidated`, `expired` or `retracted` is never holding.
#[must_use]
pub fn closed_override(story: &Story, still_holds: Option<bool>) -> Option<bool> {
    if matches!(
        story.stage,
        Stage::Invalidated | Stage::Expired | Stage::Retracted
    ) && story.time == TimeKind::State
    {
        Some(false)
    } else {
        still_holds
    }
}

/// (R4, K42/A28) Validity of a story today from its stage and whether its state
/// condition still holds (`still_holds` as [`condition_now`] gives it, after
/// [`closed_override`]): closed stage → `closed`; open and the state condition broke →
/// `no_longer`; otherwise `valid`. The sentence's "artık geçerli değil / no longer holds"
/// tail is said exactly when this is not `valid`.
#[must_use]
pub fn validity_now(story: &Story, still_holds: Option<bool>) -> Validity {
    if story.stage.is_closed() {
        Validity::Closed
    } else if still_holds == Some(false) {
        Validity::NoLonger
    } else {
        Validity::Valid
    }
}
