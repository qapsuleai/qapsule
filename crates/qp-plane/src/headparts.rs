//! (R4/H-FP) Headline pieces for a face design's headline template (M8, SPEC E5;
//! `docs/CONCEPTS.md` "Başlık yapı taşları").
//!
//! The authority fills [`qp_grammar::HeadlineParts`] where it writes the reader's
//! sentence, by the same rule: the reader's story (projected for a reader with a
//! projection), the facts it may see, the same "now" (the last final bar of the story's
//! series), the same validity (K42/A28) and the same redaction (K33: no digits). The
//! texts are `qp_capsule::realize::parts` in the reader's language. The shell's replica
//! fills them for its reader from the capsule item the authority answered
//! ([`crate::Session::item_headline_parts`]).
//!
//! "Now" for the elapsed phrases is the open time of the series' last bar, final or not
//! (the sentence's "now" is the last *final* bar; on a daily series that is yesterday's
//! while today's bar runs, which would turn "dün" into "bugün"). Validity is as of the
//! sentence's bar.
//!
//! The elapsed phrases follow the bar period (the series' `interval`) and an exchange
//! calendar: the series instrument's calendar when it names a built-in one, else the
//! session's ([`crate::Session::set_headline_calendar`]: a replay host sets the synthetic
//! market's frozen calendar for synthetic data), else the built-in `XIST`.

use qp_calendar::{builtin, ExchangeCalendar};
use qp_capsule::realize::elapsed::Period;
use qp_capsule::realize::parts::{parts, Parts};
use qp_capsule::Lang;
use qp_grammar::depth::GLYPH_MIN_LEDGER_N;
use qp_grammar::HeadlineParts;
use qp_schema::capsule::{Story, Validity};
use qp_schema::protocol::{CapsuleVoice, GlossEntry, Language, ReplaySourceKind};
use qp_schema::{Bar, Calendar, Series};

/// The calendar a replay of `kind` builds its bars on: the synthetic market's frozen
/// calendar, or the built-in `XIST` for local (BIST) data. A replay host and a shell's
/// replica give it to [`crate::Session::set_headline_calendar`].
#[must_use]
pub fn replay_calendar(kind: ReplaySourceKind) -> Option<Calendar> {
    match kind {
        ReplaySourceKind::Synthetic => Some(qp_source::synthetic_calendar()),
        ReplaySourceKind::Local => builtin("XIST"),
    }
}

/// The calendar of bar series `bs`'s elapsed phrases (see the module).
///
/// (R6) Built calendars are cached per thread: building one parses every holiday date,
/// and this runs for every capsule sentence of every reader (the live demo profile showed
/// it as the hottest path). The result is the same calendar, so outputs do not change.
fn calendar_of(bs: &Series, fallback: Option<&Calendar>) -> Option<ExchangeCalendar> {
    thread_local! {
        static BY_NAME: std::cell::RefCell<std::collections::BTreeMap<String, Option<ExchangeCalendar>>> =
            const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
        static BY_VALUE: std::cell::RefCell<Vec<(Calendar, Option<ExchangeCalendar>)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    let by_name = |id: &str| -> Option<Option<ExchangeCalendar>> {
        BY_NAME.with(|c| {
            if let Some(v) = c.borrow().get(id) {
                return Some(v.clone());
            }
            let cal = builtin(id)?;
            let built = ExchangeCalendar::new(&cal).ok();
            c.borrow_mut().insert(id.to_owned(), built.clone());
            Some(built)
        })
    };
    if let Some(id) = bs.instrument.as_ref().and_then(|i| i.calendar.as_deref()) {
        if let Some(v) = by_name(id) {
            return v;
        }
    }
    if let Some(cal) = fallback {
        return BY_VALUE.with(|c| {
            if let Some((_, v)) = c.borrow().iter().find(|(k, _)| k == cal) {
                return v.clone();
            }
            let built = ExchangeCalendar::new(cal).ok();
            let mut c = c.borrow_mut();
            if c.len() >= 8 {
                c.remove(0);
            }
            c.push((cal.clone(), built.clone()));
            built
        });
    }
    by_name("XIST").flatten()
}

/// The headline pieces of `story` as of bar `now` of `bars` (bar series `bs`; `at` =
/// `(bs, bars, now)`) for a reader in `lang`; `None` when the series has no known
/// interval or the calendar is invalid.
pub fn story_parts(
    story: &Story,
    at: (&Series, &[Bar], u64),
    validity: Validity,
    redacted: bool,
    lang: Language,
    fallback: Option<&Calendar>,
) -> Option<HeadlineParts> {
    let (bs, bars, now) = at;
    // The elapsed phrases' "now" is the series' last bar, final or not (the bar the
    // reader's day is in: a daily capsule born on yesterday's bar reads "dün" while
    // today's bar runs), never before the sentence's bar `now`. Authority and replica
    // hold the same bars, so both say the same.
    let now_t = bars.get(usize::try_from(now).ok()?)?.t.max(bars.last()?.t);
    let period = Period::parse(bs.interval.as_deref()?)?;
    let cal = calendar_of(bs, fallback)?;
    let p = parts(story, now_t, validity, &cal, period, redacted);
    Some(from_parts(&p, lang))
}

/// (R5/m17-int, `docs/LANGUAGE.md`) The surfaces of `story` in voice `voice` for a reader
/// in `lang`: the voice speaks from the sentence's input
/// ([`crate::sentences::with_sentence_input`]) and the same headline pieces as
/// [`story_parts`] (same "now", calendar, period, validity, redaction), so the open
/// capsule's detail is built from the face's pieces and cannot contradict it (A33).
/// `None` under `r4`, without a known interval, a valid calendar or the lead fact.
#[allow(clippy::too_many_arguments)]
pub fn story_voice(
    voice: &str,
    story: &Story,
    facts: &[&qp_schema::Fact],
    at: (&Series, &[Bar], u64),
    validity: Validity,
    redacted: bool,
    lang: Language,
    fallback: Option<&Calendar>,
    clock: Option<Option<&str>>,
    memo: &mut qp_capsule::sentence::StudyMemo,
) -> Option<CapsuleVoice> {
    let (bs, bars, now) = at;
    let now_t = bars.get(usize::try_from(now).ok()?)?.t.max(bars.last()?.t);
    let period = Period::parse(bs.interval.as_deref()?)?;
    let calendar = calendar_of(bs, fallback)?;
    let p = parts(story, now_t, validity, &calendar, period, redacted);
    // (RP4/T7) The row's time before the headline, when the connection asked for it
    // (`clock`: `Some(display zone)`); never for a redacted reader (K33).
    let stamps = clock
        .filter(|_| !redacted)
        .and_then(|tz| story_event_t(story).map(|ev| (ev, tz)))
        .map(|(ev, tz)| {
            let zone = tz
                .and_then(qp_calendar::Zone::get)
                .unwrap_or_else(|| calendar.zone());
            [Lang::Tr, Lang::En]
                .map(|l| qp_capsule::realize::stamp::stamp(zone, period, ev, now_t, l))
        });
    let [tr, en] =
        crate::sentences::with_sentence_input(story, facts, bars, now, redacted, memo, |s| {
            qp_voice::speak(
                voice,
                &qp_voice::Input {
                    sentence: s,
                    parts: &p,
                    calendar: &calendar,
                    period,
                    stamp: stamps.as_ref().map(|[t, e]| [t.as_str(), e.as_str()]),
                },
            )
        })??;
    let s = match lang {
        Language::Tr => tr,
        Language::En => en,
    };
    Some(CapsuleVoice {
        voice: voice.to_owned(),
        face: qp_voice::text_face(&s),
        headline: s.headline,
        detail: s.detail,
        ledger: s.ledger,
        glossary: s
            .glossary
            .into_iter()
            .map(|g| GlossEntry {
                term: g.term,
                text: g.text,
            })
            .collect(),
    })
}

fn lang_of(lang: Language) -> Lang {
    match lang {
        Language::Tr => Lang::Tr,
        Language::En => Lang::En,
    }
}

/// (RP4/T7) The time a story's row is about: its birth while open, its close once closed
/// (the moment the headline's "when" names: the birth phrase, or the closing chip).
#[must_use]
pub(crate) fn story_event_t(story: &Story) -> Option<i64> {
    if story.stage.is_closed() {
        story.history.last().map(|h| h.t)
    } else {
        story.history.first().map(|h| h.t)
    }
}

/// (RP4/T7) The stamp of an event at bar time `event_t` on bar series `bs` (`bars`: its
/// bars; "now" is the last one's time, the authority's data time) for a reader in `lang`,
/// in display zone `display_tz` (`None` or unknown: the exchange calendar's zone, K14).
/// `None` without a known interval or a valid calendar.
pub fn event_stamp(
    event_t: i64,
    bs: &Series,
    bars: &[Bar],
    lang: Language,
    fallback: Option<&Calendar>,
    display_tz: Option<&str>,
) -> Option<String> {
    let now_t = bars.last()?.t;
    let period = Period::parse(bs.interval.as_deref()?)?;
    let cal = calendar_of(bs, fallback)?;
    let zone = display_tz
        .and_then(qp_calendar::Zone::get)
        .unwrap_or_else(|| cal.zone());
    Some(qp_capsule::realize::stamp::stamp(
        zone,
        period,
        event_t,
        now_t,
        lang_of(lang),
    ))
}

/// The grammar's plain pieces from the realizer's.
#[must_use]
pub(crate) fn from_parts(p: &Parts, lang: Language) -> HeadlineParts {
    let l = match lang {
        Language::Tr => Lang::Tr,
        Language::En => Lang::En,
    };
    HeadlineParts {
        lang,
        instrument: p.instrument.clone(),
        event: p.event(l).to_owned(),
        direction_word: p.direction_word(l).to_owned(),
        when: p.when(l),
        moved: p.moved(l),
        validity_phrase: p.validity_phrase(l).to_owned(),
        ledger_short: p.ledger_short(l),
        stage: p.stage,
        validity: p.validity,
        redacted: p.redacted,
        ledger_n: (!p.redacted).then_some(p.ledger.n),
        few_cases: p.ledger.n < GLYPH_MIN_LEDGER_N,
    }
}
