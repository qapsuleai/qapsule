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
pub(crate) fn story_parts(
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
/// ([`crate::capsule::with_sentence_input`]) and the same headline pieces as
/// [`story_parts`] (same "now", calendar, period, validity, redaction), so the open
/// capsule's detail is built from the face's pieces and cannot contradict it (A33).
/// `None` under `r4`, without a known interval, a valid calendar or the lead fact.
#[allow(clippy::too_many_arguments)]
pub(crate) fn story_voice(
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
        crate::capsule::with_sentence_input(story, facts, bars, now, redacted, memo, |s| {
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
pub(crate) fn event_stamp(
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use qp_grammar::HeadlineParts;
    use qp_schema::capsule::Validity;
    use qp_schema::protocol::{Language, ReplayInit, ReplaySourceKind};

    use crate::{demo_init, ReplayHost};

    /// An example headline template (tests only; no face design in the registry).
    fn example(hp: &HeadlineParts) -> String {
        let (sep, few) = match hp.lang {
            Language::Tr => (" · ", "az örnek"),
            Language::En => (" · ", "few cases"),
        };
        let tail = if hp.validity == Validity::Valid {
            hp.when.clone()
        } else {
            hp.validity_phrase.clone()
        };
        let mut s = format!("{} {}{sep}{tail}", hp.instrument, hp.event);
        if hp.few_cases {
            s.push_str(sep);
            s.push_str(few);
        }
        s
    }

    /// The demo world (synthetic, as `qp demo`), small; `visitor` reads without data.
    fn host() -> ReplayHost {
        let mut ent = BTreeMap::new();
        for p in ["host", "arkin"] {
            ent.insert(p.to_owned(), vec!["*".to_owned()]);
        }
        ent.insert("visitor".to_owned(), Vec::new());
        let mut init = demo_init(ReplayInit {
            source: ReplaySourceKind::Synthetic,
            instruments: Some(8),
            days: Some(260),
            history: Some(120),
            fine: Some("1h".to_owned()),
            entitlements: ent,
            ..ReplayInit::default()
        });
        init.principals.push(
            serde_json::from_value(serde_json::json!(
                {"id": "visitor", "author": {"kind": "human", "name": "visitor"}, "role": "owner"}))
            .expect("visitor"),
        );
        let none = std::env::temp_dir().join("qp-hfp-no-local-data");
        ReplayHost::with_local_dir(init, std::path::Path::new("."), &none).expect("demo host")
    }

    /// Every capsule's pieces for a reader: `(finding, TR, EN)`.
    fn all_parts(h: &mut ReplayHost, reader: &str) -> Vec<(String, HeadlineParts, HeadlineParts)> {
        let s = h.session_mut();
        let mut out = Vec::new();
        let findings: Vec<String> = s
            .doc
            .capsules
            .iter()
            .filter(|c| c.story.is_some())
            .map(|c| c.finding.clone())
            .collect();
        for f in findings {
            let mut both = Vec::new();
            for lang in [Language::Tr, Language::En] {
                let hp = s.as_reader(reader, lang, |s| {
                    let c = s.doc.capsules.iter().find(|c| c.finding == f)?;
                    s.reader_parts(
                        &s.doc,
                        c,
                        s.proj.as_deref(),
                        lang,
                        &mut qp_capsule::sentence::StudyMemo::default(),
                    )
                });
                both.extend(hp);
            }
            if let [tr, en] = both.as_slice() {
                out.push((f, tr.clone(), en.clone()));
            }
        }
        out
    }

    /// (R4/H-FP) The authority fills the headline pieces by the sentence's language and
    /// audience rule: TR and EN texts, an example template within 80 characters, no
    /// internal term, no digit for a reader without data entitlement; deterministic.
    #[test]
    fn reader_parts_by_language_and_audience() {
        let mut h = host();
        let mut seen = 0;
        let mut redacted = 0;
        let mut kinds = std::collections::BTreeSet::new();
        for step in 0..60 {
            if step % 20 == 0 {
                let a = all_parts(&mut h, "arkin");
                let v = all_parts(&mut h, "visitor");
                for (f, tr, en) in &a {
                    seen += 1;
                    assert_eq!((tr.lang, en.lang), (Language::Tr, Language::En));
                    assert!(!tr.redacted && tr.ledger_n.is_some(), "{f}");
                    assert_eq!(tr.few_cases, tr.ledger_n.unwrap() < 20);
                    kinds.insert(format!("{:?}/{:?}", tr.stage, tr.validity));
                    for hp in [tr, en] {
                        let line = example(hp);
                        assert!(line.chars().count() <= 80, "{line}");
                        let low = line.to_lowercase();
                        for bad in ["bar", "aile", "teyit", "family"] {
                            assert!(!low.contains(bad), "{line}");
                        }
                    }
                    assert_ne!(tr.event, en.event);
                }
                for (f, tr, en) in &v {
                    if tr.redacted {
                        redacted += 1;
                        assert_eq!(tr.ledger_n, None, "{f}");
                        for hp in [tr, en] {
                            let line = example(hp);
                            for t in [
                                &line,
                                &hp.when,
                                &hp.moved,
                                &hp.ledger_short,
                                &hp.validity_phrase,
                            ] {
                                let digits = t.chars().filter(char::is_ascii_digit).count();
                                // The instrument label (`SYN03`) is the only digit source.
                                let label =
                                    hp.instrument.chars().filter(char::is_ascii_digit).count();
                                let allowed = if std::ptr::eq(t, &line) { label } else { 0 };
                                assert_eq!(digits, allowed, "{f}: {t}");
                            }
                        }
                    }
                }
                // Deterministic.
                assert_eq!(a, all_parts(&mut h, "arkin"));
            }
            assert!(h.take_step().is_some());
        }
        assert!(seen > 0, "no capsule with a story");
        assert!(redacted > 0, "no redacted reader's pieces");
        assert!(kinds.len() > 1, "{kinds:?}");
    }
}
