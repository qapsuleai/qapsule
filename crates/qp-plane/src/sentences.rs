//! The R3 sentence, its input and a story's validity, as the reader sees them.

use qp_capsule::realize::Template;
use qp_capsule::sentence::{condition_now_memo, StudyMemo};
use qp_capsule::{Lang, Realizer, SentenceInput};
use qp_schema::capsule::Story;
use qp_schema::{Bar, Fact};

/// The sentence pair of a story for one reader as of bar `now` of `bars` (K26), from the
/// story and its facts only (no engine needed): `qp_capsule::sentence` rules with the
/// chosen realizer. `redacted`: no digit at all (K33).
///
/// (R3/perf) Study values come from `memo` (one pass over unchanged bars), and the
/// realizer's output from `cache` (capsule id, memo) when it already wrote this exact
/// input. `only`: a caller that reads one language gets that one written (the other is
/// empty unless the cache had both; nothing is cached then).
#[allow(clippy::too_many_arguments)]
pub fn sentence_pair_memo(
    story: &Story,
    facts: &[&Fact],
    bars: &[Bar],
    now: u64,
    redacted: bool,
    memo: &mut StudyMemo,
    cache: Option<(&str, &std::cell::RefCell<crate::capidx::SentenceMemo>)>,
    only: Option<Lang>,
) -> Option<(String, String)> {
    let now_i = usize::try_from(now).ok()?;
    let bars = bars.get(..=now_i)?;
    let facts: Vec<&Fact> = facts.iter().copied().filter(|f| f.bar <= now).collect();
    let lead = *facts.first()?;
    let latest = *facts.last()?;
    let (still_holds, current) = condition_now_memo(lead, &facts, bars, memo);
    let still_holds = qp_capsule::sentence::closed_override(story, still_holds);
    lead.meaning.as_ref()?;
    let current = if redacted { None } else { current };
    let key = crate::capidx::SentenceKeyRef {
        story,
        lead,
        latest,
        now_bar: now,
        now_t: bars[now_i].t,
        still_holds,
        current: current.map(f64::to_bits),
        redacted,
    };
    if let Some((id, c)) = cache {
        if let Some(v) = c.borrow().get(id, &key) {
            return Some(v);
        }
    }
    // (K42/A28) A state's expected resolution is not its state condition.
    let lead_view = qp_capsule::sentence::sentence_lead(lead);
    let lead_ref = lead_view.as_ref().unwrap_or(lead);
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
    if let Some(l) = only {
        let t = Template.sentence(&inp, l);
        return Some(match l {
            Lang::Tr => (t, String::new()),
            Lang::En => (String::new(), t),
        });
    }
    let v = (
        Template.sentence(&inp, Lang::Tr),
        Template.sentence(&inp, Lang::En),
    );
    if let Some((id, c)) = cache {
        c.borrow_mut().put(id, &key, v.clone());
    }
    Some(v)
}

/// (R5/m17-int) `f` over the sentence input of a story for one reader as of bar `now` of
/// `bars`: the same input [`sentence_pair_memo`] gives the R3 sentence (lead and latest
/// fact, still-holds, the condition's current value, redaction). A capsule voice
/// (`qp_capsule::voice`) speaks from it. `None` as [`sentence_pair_memo`].
pub(crate) fn with_sentence_input<R>(
    story: &Story,
    facts: &[&Fact],
    bars: &[Bar],
    now: u64,
    redacted: bool,
    memo: &mut StudyMemo,
    f: impl FnOnce(&SentenceInput<'_>) -> R,
) -> Option<R> {
    let now_i = usize::try_from(now).ok()?;
    let bars = bars.get(..=now_i)?;
    let facts: Vec<&Fact> = facts.iter().copied().filter(|f| f.bar <= now).collect();
    let lead = *facts.first()?;
    let latest = *facts.last()?;
    let (still_holds, current) = condition_now_memo(lead, &facts, bars, memo);
    let still_holds = qp_capsule::sentence::closed_override(story, still_holds);
    lead.meaning.as_ref()?;
    let current = if redacted { None } else { current };
    let lead_view = qp_capsule::sentence::sentence_lead(lead);
    let lead_ref = lead_view.as_ref().unwrap_or(lead);
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
    Some(f(&inp))
}

/// (R4, K42/A28) Validity of `story` as of bar `now` over `facts` (the reader's) and
/// `bars`: the same `still_holds` the sentence reads ([`sentence_pair_memo`]), so the
/// item's `validity`, the glyph and the sentence agree.
pub fn story_validity(
    story: &Story,
    facts: &[&Fact],
    bars: &[Bar],
    now: u64,
    memo: &mut StudyMemo,
) -> qp_schema::capsule::Validity {
    let still = usize::try_from(now)
        .ok()
        .and_then(|i| bars.get(..=i))
        .and_then(|bars| {
            let facts: Vec<&Fact> = facts.iter().copied().filter(|f| f.bar <= now).collect();
            let lead = *facts.first()?;
            Some(condition_now_memo(lead, &facts, bars, memo).0)
        })
        .flatten();
    let still = qp_capsule::sentence::closed_override(story, still);
    qp_capsule::sentence::validity_now(story, still)
}

/// Fact ids of a story in join order (transitions' facts, first occurrence).
pub fn story_fact_ids(story: &Story) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in &story.history {
        for f in &t.facts {
            if !out.contains(f) {
                out.push(f.clone());
            }
        }
    }
    out
}
