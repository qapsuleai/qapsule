//! `marla` — a capsule language (R5/M17 alternative): **the order of the reader's five
//! questions, and context for every number**.
//!
//! The five questions of `NORTH_STAR.md` are the skeleton. Each surface answers one or two
//! of them, in a fixed order, and never repeats another:
//!
//! | Surface | Question | What it says |
//! |---------|----------|--------------|
//! | başlık (headline) | **Ne değişti?** (+ ne zaman, hâlâ geçerli mi) | the plain change and the threshold by name and value: "SYN03 son 20 günün en yüksek fiyatını (148,90) aştı · 3 işlem günü önce · hâlâ geçerli" |
//! | ayrıntı (detail) | **Neden?** (+ şimdi ne durumda) | the numbers behind it, each against a reference the reader can hold: "O gün 150,20 ile kapandı, eşiğin %0,9 üstünde. Şimdi 151,00 (eşiğin %1,4 üstünde), sonuç en geç 7 işlem günü içinde belli olacak." |
//! | karne (ledger) | **Ne kadar emin? Daha önce oldu mu?** | the expectation itself, counted: "Benzer 74 durumun 36'sında fiyat 10 işlem günü içinde yükseldi." |
//! | sözlük (glossary) | — | every term on the screen, opened on touch, incl. what "tuttu" means and the coin-flip base rate |
//!
//! "Beni ilgilendiriyor mu?" needs the reader's holdings; the language does not guess it.
//!
//! Every number is said against a reference: the threshold (absolute value and the
//! percentage distance), the period it is the extreme of ("son 20 günün"), the average it
//! is a multiple of ("20 günlük ortalamanın 2,5 katı"), the time left until the outcome is
//! known ("en geç 7 işlem günü içinde"). Time is an elapsed phrase from the exchange
//! calendar ([`crate::realize::elapsed`]), never a bar count.
//!
//! # Entry point
//!
//! [`speak`] (one language) / [`speak_both`] (Turkish and English together): a pure
//! function of the product's own data —
//!
//! - [`SentenceInput`]: the story (kind, direction, stage, horizon, ledger at birth,
//!   history), the lead fact (detector, evidence, licence), the latest fact, now, the
//!   current value of the condition subject and whether the reader is redacted;
//! - [`Parts`]: the face's headline building blocks (when, last move, validity, ending,
//!   ledger, redacted). Validity, "when" and the ending come **only** from here, so the
//!   detail is built from the same pieces as the face and cannot contradict it (A33).
//!
//! No clock, no randomness, no model. The length limits (headline 80, detail 200 and three
//! sentences, ledger 70 characters) are met by choosing, for both languages together, the
//! richest variant that fits; both languages always carry the same numbers.
//!
//! # Redacted reader (K33/K34, A32)
//!
//! A reader without the data licence gets the same skeleton without a single digit: the
//! threshold is named and the detector's public rule is said in words ("beş günlük
//! ortalamanın iki katı"), time is a vague elapsed phrase, the ledger is qualitative, and
//! the detail says which licence opens the numbers and that the server (not this device)
//! computed and verified them.
//!
//! # Integration
//!
//! The face headline template can call [`speak`] with the authority's `Parts`; the open
//! capsule shows `detail`, the face's ledger row shows `ledger`, and `glossary` is the
//! screen's touch-to-open list. Existing transcripts and goldens do not use this module.

use std::collections::BTreeMap;

use qp_schema::capsule::Validity;
use qp_schema::capsule::{Direction, Edge, LedgerStat, LicenseClass, MeaningKind, Stage, Trust};
use qp_schema::Fact;

use super::marla_glossary::{entry, Concept, Gloss, UnitWords};
use crate::realize::elapsed::{Elapsed, Period};
use crate::realize::morph::{capitalize, Word};
use crate::realize::num::{fmt, Cat};
use crate::realize::parts::{Ending, Parts};
use crate::realize::LEDGER_MIN_N;
use crate::{Lang, SentenceInput};

/// Headline length limit (characters).
pub const MAX_HEADLINE: usize = 80;
/// Detail length limit (characters).
pub const MAX_DETAIL: usize = 200;
/// Detail sentence limit.
pub const MAX_DETAIL_SENTENCES: usize = 3;
/// Ledger sentence length limit (characters).
pub const MAX_LEDGER: usize = 70;

/// The four surfaces of one capsule in one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    /// Başlık: what changed, with the threshold, when and whether it still holds.
    pub headline: String,
    /// Ayrıntı: why, and where things stand now.
    pub detail: String,
    /// Karne: the expectation and how often it played out before.
    pub ledger: String,
    /// Sözlük: the terms on this screen.
    pub glossary: Vec<Gloss>,
}

/// The surfaces in `lang` (see the module docs).
#[must_use]
pub fn speak(inp: &SentenceInput<'_>, parts: &Parts, lang: Lang) -> Voice {
    let [tr, en] = speak_both(inp, parts);
    match lang {
        Lang::Tr => tr,
        Lang::En => en,
    }
}

/// The surfaces in Turkish and English (`[tr, en]`), length variants chosen together so
/// both carry the same numbers.
#[must_use]
pub fn speak_both(inp: &SentenceInput<'_>, parts: &Parts) -> [Voice; 2] {
    speak_both_in(inp, parts, MAX_HEADLINE)
}

/// (RP4/T7) [`speak_both`] with the headline kept within `room` characters (a row stamp
/// goes before it, `crate::voice::Input::stamp`).
#[must_use]
pub fn speak_both_in(inp: &SentenceInput<'_>, parts: &Parts, room: usize) -> [Voice; 2] {
    let room = room.min(MAX_HEADLINE);
    let f = Frame::new(inp, parts);
    let langs = [Lang::Tr, Lang::En];

    // Headline: the richest level that fits, per language when both carry the same
    // numbers and both (or neither) say when; otherwise one level for both.
    let fit = |l: Lang| {
        (0..HEAD_LEVELS)
            .find(|&lv| len(&headline(&f, l, lv)) <= room)
            .unwrap_or(HEAD_LEVELS - 1)
    };
    let (lt, le) = (fit(Lang::Tr), fit(Lang::En));
    let same = HEAD_PLAN[lt].2 == HEAD_PLAN[le].2
        && digit_runs(&headline(&f, Lang::Tr, lt)) == digit_runs(&headline(&f, Lang::En, le));
    let hl = |l: Lang| -> usize {
        if same {
            match l {
                Lang::Tr => lt,
                Lang::En => le,
            }
        } else {
            lt.max(le)
        }
    };

    // Detail: richest composition that fits in both languages.
    let dm = DETAIL_MASKS
        .iter()
        .copied()
        .find(|&m| {
            langs.iter().all(|&l| {
                let (d, _) = detail(&f, l, m);
                len(&d) <= MAX_DETAIL && sentence_count(&d) <= MAX_DETAIL_SENTENCES
            })
        })
        .unwrap_or(DETAIL_MASKS[DETAIL_MASKS.len() - 1]);

    // Ledger: richest variant that fits in both languages.
    let ll = (0..LEDGER_LEVELS)
        .find(|&lv| langs.iter().all(|&l| len(&ledger(&f, l, lv)) <= MAX_LEDGER))
        .unwrap_or(LEDGER_LEVELS - 1);

    langs.map(|l| {
        let (d, concepts) = detail(&f, l, dm);
        Voice {
            headline: headline(&f, l, hl(l)),
            detail: d,
            ledger: ledger(&f, l, ll),
            glossary: concepts
                .into_iter()
                .map(|c| entry(c, f.unit.words(), l))
                .collect(),
        }
    })
}

fn len(s: &str) -> usize {
    s.chars().count()
}

/// The digit runs of a text, sorted ("1.234,56" and "1,234.56" give the same runs):
/// two texts with equal runs carry the same numbers, clock times and dates.
fn digit_runs(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s
        .split(|c: char| !c.is_ascii_digit())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    v.sort();
    v
}

/// Sentences as a reader splits them: after `.`, `;`, `!`, `?` followed by a space or
/// the end (a decimal mark is never followed by a space).
fn sentence_count(s: &str) -> usize {
    let cs: Vec<char> = s.chars().collect();
    let mut n = 0;
    let mut open = false;
    for (i, &c) in cs.iter().enumerate() {
        if !c.is_whitespace() {
            open = true;
        }
        if matches!(c, '.' | ';' | '!' | '?') && cs.get(i + 1).is_none_or(|x| x.is_whitespace()) {
            if open {
                n += 1;
            }
            open = false;
        }
    }
    n + usize::from(open)
}

// ---------------------------------------------------------------------------
// Frame: language-independent reading of the inputs
// ---------------------------------------------------------------------------

/// The period's time unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Day,
    Hour,
    Week,
    Month,
    /// Intraday bars of this many minutes (not 60).
    Slice(u32),
}

impl Unit {
    fn of(series: &str) -> Self {
        let suffix = series.rsplit('.').next().unwrap_or("");
        match Period::parse(suffix) {
            Some(Period::Intraday(60)) => Self::Hour,
            Some(Period::Intraday(m)) => Self::Slice(m),
            Some(Period::Weekly) => Self::Week,
            Some(Period::Monthly) => Self::Month,
            Some(Period::Daily) | None => Self::Day,
        }
    }

    fn words(self) -> UnitWords {
        match self {
            Self::Day => UnitWords {
                tr: "gün",
                tr_loc: "günde",
                tr_adj: "günlük",
                tr_abl: "günden",
                en: "day",
                en_pl: "days",
            },
            Self::Hour => UnitWords {
                tr: "saat",
                tr_loc: "saatte",
                tr_adj: "saatlik",
                tr_abl: "saatten",
                en: "hour",
                en_pl: "hours",
            },
            Self::Week => UnitWords {
                tr: "hafta",
                tr_loc: "haftada",
                tr_adj: "haftalık",
                tr_abl: "haftadan",
                en: "week",
                en_pl: "weeks",
            },
            Self::Month => UnitWords {
                tr: "ay",
                tr_loc: "ayda",
                tr_adj: "aylık",
                tr_abl: "aydan",
                en: "month",
                en_pl: "months",
            },
            Self::Slice(_) => UnitWords {
                tr: "dilim",
                tr_loc: "dilimde",
                tr_adj: "dilimlik",
                tr_abl: "dilimden",
                en: "period",
                en_pl: "periods",
            },
        }
    }
}

/// The latest fact when it is not the lead.
struct Latest<'a> {
    det: &'a str,
    code: &'a str,
    dir: Direction,
    edge: Edge,
    ev: &'a BTreeMap<String, f64>,
    /// Its numbers may be printed (public data and the lead is not redacted).
    open: bool,
}

struct Frame<'a> {
    inst: &'a str,
    kind: MeaningKind,
    dir: Direction,
    det: &'a str,
    ev: &'a BTreeMap<String, f64>,
    unit: Unit,
    red: bool,
    validity: Validity,
    ending: Option<Ending>,
    born: Elapsed,
    last: Elapsed,
    stage: Stage,
    current: Option<f64>,
    /// Now is the lead fact's own bar (the "now" value is the event's value).
    fresh: bool,
    latest: Option<Latest<'a>>,
    horizon: u32,
    /// Bars left until the horizon (open stories only).
    remaining: Option<u32>,
    ledger: LedgerStat,
    server: bool,
}

fn code_of(f: &Fact) -> &str {
    f.kind.rsplit('.').next().unwrap_or("")
}

impl<'a> Frame<'a> {
    fn new(inp: &SentenceInput<'a>, p: &Parts) -> Self {
        let s = inp.story;
        let m = inp.lead.meaning.as_ref();
        let red = p.redacted || inp.redacted;
        let latest = (inp.latest.id != inp.lead.id)
            .then(|| {
                inp.latest.meaning.as_ref().map(|lm| Latest {
                    det: lm.detector.as_str(),
                    code: code_of(inp.latest),
                    dir: lm.direction,
                    edge: lm.edge,
                    ev: &inp.latest.evidence,
                    open: !red
                        && (lm.license.class == LicenseClass::Public
                            || m.is_some_and(|m| {
                                m.license.class == LicenseClass::Licensed
                                    && m.license.entitlement == lm.license.entitlement
                            })),
                })
            })
            .flatten();
        let born_bar = s.history.first().map_or(s.born_bar, |h| h.bar);
        let remaining = (!s.stage.is_closed()).then(|| {
            let used = inp.now_bar.saturating_sub(born_bar);
            u32::try_from(u64::from(s.horizon_bars).saturating_sub(used)).unwrap_or(0)
        });
        Self {
            inst: &s.instrument,
            kind: s.kind,
            dir: s.direction,
            det: m.map_or("", |m| m.detector.as_str()),
            ev: &inp.lead.evidence,
            unit: Unit::of(&s.series),
            red,
            validity: p.validity,
            ending: p.ending,
            born: p.born,
            last: p.last,
            stage: p.stage,
            current: if red {
                None
            } else {
                inp.current.filter(|v| v.is_finite())
            },
            fresh: inp.now_bar == inp.lead.bar,
            latest,
            horizon: s.horizon_bars,
            remaining,
            ledger: p.ledger,
            server: s.trust == Trust::Server || m.is_some_and(|m| m.server),
        }
    }

    /// A lead evidence value (never for a redacted reader).
    fn v(&self, k: &str) -> Option<f64> {
        if self.red {
            return None;
        }
        self.ev.get(k).copied().filter(|x| x.is_finite())
    }

    fn up(&self) -> bool {
        self.dir == Direction::Up
    }

    fn when(&self, lang: Lang) -> String {
        el(&self.born, lang, self.red)
    }

    fn moved(&self, lang: Lang) -> String {
        el(&self.last, lang, self.red)
    }
}

fn el(e: &Elapsed, lang: Lang, red: bool) -> String {
    if red {
        crate::realize::elapsed::phrase_vague(e, lang)
    } else {
        crate::realize::elapsed::phrase(e, lang)
    }
}

mod head;
mod record;
mod since;
mod why;
mod words;

use head::{headline, tr_gen_w, HEAD_LEVELS, HEAD_PLAN};
use record::{ledger, LEDGER_LEVELS};
use since::{anchor, change, closing};
use why::{det_concepts, dir_past, lead_concepts, now, remaining, why, DETAIL_MASKS};
use words::{abl, dat, en_adj, mult, n, param, pct, span, span_loc_tr, that, tr_adj, tr_gen};
pub(super) use words::{words_en, words_tr};

// ---------------------------------------------------------------------------
// Detector parameters (the detectors' own constants)
// ---------------------------------------------------------------------------

use qp_facts::{BandSqueeze, InstitutionalFlow, MaStack, RangeBreakout, VolumeSurge};

fn detail(f: &Frame<'_>, lang: Lang, mask: (bool, bool, bool)) -> (String, Vec<Concept>) {
    let (with_now, with_rem, with_change) = mask;
    let mut concepts: Vec<Concept> = lead_concepts(f);
    let mut out: Vec<String> = Vec::new();
    out.push(why(f, lang));
    if f.validity == Validity::Closed {
        out.push(closing(f, lang));
    } else {
        // The change sentence already says where an exit left the lead's own measure.
        let said = with_change
            && f.latest.as_ref().is_some_and(|l| {
                l.det == f.det && (l.edge == Edge::Exit || l.code.ends_with("_end"))
            });
        let now = if with_now && !said {
            now(f, lang)
        } else {
            None
        };
        let rem = if with_rem { remaining(f, lang) } else { None };
        match (now, rem) {
            (Some(a), Some(b)) => out.push(format!("{}, {b}.", capitalize(&a))),
            (Some(a), None) => out.push(format!("{}.", capitalize(&a))),
            (None, Some(b)) => out.push(format!("{}.", capitalize(&b))),
            (None, None) => {}
        }
        if with_change {
            if let Some((s, c)) = change(f, lang) {
                out.push(s);
                for x in c {
                    if !concepts.contains(&x) {
                        concepts.push(x);
                    }
                }
            }
        }
    }
    if f.validity == Validity::NoLonger {
        concepts.push(Concept::NoLonger);
    }
    concepts.push(Concept::Similar);
    (out.join(" "), concepts)
}

#[cfg(test)]
mod tests;

/// End to end over the engine (synthetic market with licensed flow): every story event,
/// entitled and redacted, keeps the language's own rules. The M17 harness
/// (`rounds/R5/experiments/harness/lang`) scans the same surfaces with its 13 scans.
#[cfg(test)]
mod engine_tests;
