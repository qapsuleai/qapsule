//! `selvi` — a capsule voice (R5/M17): **"konuşur gibi tek cümle"**.
//!
//! The headline is the one sentence a person would say to a friend: *who* did *what*,
//! against *which concrete line* (its name and value), *when* — "SYN03 dün önceki 20
//! günün en yükseği olan 148,90'ı aştı." Validity follows as a short second part
//! ("Hâlâ geçerli."). The detail keeps the same voice (no internal words: no bar count,
//! no "teyit", no "aile"), the track-record line says the expectation itself as a natural
//! frequency ("Geçmişte 74 benzerinin 36'sında fiyat 10 işlem günü içinde yükseldi."),
//! and every technical word on the screen opens a glossary entry
//! ([`super::selvi_glossary`]).
//!
//! # Entry point
//!
//! [`speak`] takes a [`VoiceInput`] — the product's [`SentenceInput`] (story, lead and
//! latest fact, now, `still_holds`, `current`, `redacted`), the face's building blocks
//! [`Parts`] (`crate::realize::parts::parts`: when / moved / validity / ending / ledger,
//! the same values the glyph and the face carry, K42), the exchange calendar and the bar
//! period — and returns a [`Voice`]: the four surfaces in Turkish **and** English
//! ([`Surfaces`]: `headline`, `detail`, `ledger`, `glossary`, plus a short `trust` chip).
//! Both languages are written together on purpose: whenever a surface has a shorter
//! fallback form (length limits below), the same form is chosen in both languages, so the
//! two always carry the same numbers, dates, times and validity (TR↔EN equivalence).
//!
//! Pure and deterministic: no clock, no randomness, no network, no language model.
//!
//! # Rules
//!
//! - **Concrete (K49):** every detector names its own line with its value and its
//!   parameter ("önceki 20 günün en yükseği, 148,90"; "RSI 70 çizgisi"; "20 günlük
//!   ortalama 48,10"; "son 120 günün en dar bandı, 45,10–47,30"; "20 günlük ortalamanın
//!   2,5 katı hacim"; "5 günlük ortalamanın 2 katı kurum net alımı").
//! - **Time** is the calendar elapsed phrase ([`crate::realize::elapsed`]), never a bar
//!   count; spans are trading days / trading hours ("10 işlem günü içinde").
//! - **Validity** comes only from [`Parts::validity`] and [`Parts::ending`]: the headline's
//!   second part says it; the detail never says a different validity (A33).
//! - **Ledger:** counts as a natural frequency with the horizon and the direction that
//!   counted ("… 36'sında fiyat 10 işlem günü içinde yükseldi"); "Az örnek" below
//!   [`LEDGER_MIN_N`]; "Henüz benzeri yok" with the rule that will count when empty.
//! - **Licence (K33/K34):** a redacted reader gets no digit anywhere (the values are not
//!   even read), but still gets when, validity and a qualitative record, plus which
//!   entitlement opens the figures and who verified the claim.
//! - **Observation, not advice:** no sentence tells anyone to do anything.
//! - **Limits** (glance, K43): headline ≤ [`MAX_HEADLINE`] characters, detail ≤
//!   [`MAX_DETAIL`] characters and ≤ [`MAX_DETAIL_SENTENCES`] sentences, ledger ≤
//!   [`MAX_LEDGER`], glossary entry ≤ [`MAX_GLOSS`]. A surface that would be longer falls
//!   back to its next shorter form (in both languages at once).

use qp_calendar::ExchangeCalendar;
use qp_facts::{BandSqueeze, InstitutionalFlow, MaStack, RangeBreakout, RsiExtreme, VolumeSurge};
use qp_schema::capsule::{Direction, Edge, LicenseClass, MeaningKind, Story, Trust, Validity};
use qp_schema::Fact;

use super::selvi_glossary::{self, Term};
use crate::realize::elapsed::{elapsed, phrase, phrase_vague, Period};
use crate::realize::morph::{capitalize, number_last_word, Word};
use crate::realize::num::{fmt, Cat};
use crate::realize::parts::{Ending, Parts};
use crate::realize::LEDGER_MIN_N;
use crate::{Lang, SentenceInput};

/// Headline limit (characters).
pub const MAX_HEADLINE: usize = 80;
/// Detail limit (characters).
pub const MAX_DETAIL: usize = 200;
/// Detail limit (sentences).
pub const MAX_DETAIL_SENTENCES: usize = 3;
/// Ledger line limit (characters).
pub const MAX_LEDGER: usize = 70;
/// Glossary entry limit (characters of the explanation).
pub const MAX_GLOSS: usize = 160;

/// Everything the voice reads (see the module docs).
#[derive(Debug, Clone, Copy)]
pub struct VoiceInput<'a> {
    /// The product's sentence input.
    pub sentence: &'a SentenceInput<'a>,
    /// The face's building blocks for the same story, now and reader.
    pub parts: &'a Parts,
    /// The exchange calendar of the elapsed phrases.
    pub calendar: &'a ExchangeCalendar,
    /// The bar period.
    pub period: Period,
}

/// One glossary entry: a term on the screen and its explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gloss {
    /// The term as written on the screen ("RSI", "bant").
    pub term: String,
    /// Its explanation.
    pub text: String,
}

/// The surfaces in one language.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Surfaces {
    /// The face's headline.
    pub headline: String,
    /// The open capsule's detail (≤ 3 sentences).
    pub detail: String,
    /// The track-record line (face and open capsule).
    pub ledger: String,
    /// Glossary entries for the terms on this screen.
    pub glossary: Vec<Gloss>,
    /// Who checked the claim ("Bu cihazda sınandı" / "Sunucu doğruladı"): a face chip.
    pub trust: String,
}

/// The surfaces in both languages.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Voice {
    /// Turkish.
    pub tr: Surfaces,
    /// English.
    pub en: Surfaces,
}

impl Voice {
    /// The surfaces in `lang`.
    #[must_use]
    pub fn get(&self, lang: Lang) -> &Surfaces {
        match lang {
            Lang::Tr => &self.tr,
            Lang::En => &self.en,
        }
    }
}

/// Writes the four surfaces (see the module docs). The single entry point.
#[must_use]
pub fn speak(inp: &VoiceInput<'_>) -> Voice {
    speak_in(inp, MAX_HEADLINE)
}

/// (RP4/T7) [`speak`] with the headline kept within `room` characters (a row stamp goes
/// before it, `crate::voice::Input::stamp`).
#[must_use]
pub fn speak_in(inp: &VoiceInput<'_>, room: usize) -> Voice {
    let mut cx = Cx::new(inp);
    cx.room = room.min(MAX_HEADLINE);
    let headline = cx.headline();
    let detail = cx.detail();
    let ledger = cx.ledger();
    let trust = cx.trust();
    let terms = cx.terms();
    let gloss = |lang: Lang| -> Vec<Gloss> {
        terms
            .iter()
            .map(|t| {
                let (term, text) = selvi_glossary::entry(*t, &cx.gloss_ctx(), lang);
                Gloss { term, text }
            })
            .collect()
    };
    Voice {
        tr: Surfaces {
            headline: headline.tr,
            detail: detail.tr,
            ledger: ledger.tr,
            glossary: gloss(Lang::Tr),
            trust: trust.tr,
        },
        en: Surfaces {
            headline: headline.en,
            detail: detail.en,
            ledger: ledger.en,
            glossary: gloss(Lang::En),
            trust: trust.en,
        },
    }
}

// ---------------------------------------------------------------------------
// Bilingual strings
// ---------------------------------------------------------------------------

/// A text in both languages, built together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Bi {
    pub(super) tr: String,
    pub(super) en: String,
}

fn bi(tr: impl Into<String>, en: impl Into<String>) -> Bi {
    Bi {
        tr: tr.into(),
        en: en.into(),
    }
}

/// English capitalization (no Turkish dotted capital I).
fn cap_en(s: &str) -> String {
    let mut cs = s.chars();
    match cs.next() {
        Some(c) => format!("{}{}", c.to_ascii_uppercase(), cs.as_str()),
        None => String::new(),
    }
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// Sentences the way the measurement splits them (a break after `.`, `;`, `!`, `?`
/// followed by whitespace or the end).
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

impl Bi {
    fn fits(&self, max: usize) -> bool {
        chars(&self.tr) <= max && chars(&self.en) <= max
    }

    /// Joins non-empty parts with a space.
    fn join(parts: &[Bi]) -> Bi {
        let j = |f: fn(&Bi) -> &str| {
            parts
                .iter()
                .map(f)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        };
        bi(j(|b| &b.tr), j(|b| &b.en))
    }

    fn cap(&self) -> Bi {
        bi(capitalize(&self.tr), cap_en(&self.en))
    }
}

/// The first candidate that fits `ok` in both languages; else the last one.
fn first_fit(cands: Vec<Bi>, ok: impl Fn(&Bi) -> bool) -> Bi {
    let mut last = Bi::default();
    for c in cands {
        if ok(&c) {
            return c;
        }
        last = c;
    }
    last
}

// ---------------------------------------------------------------------------
// Units of time spans (period-aware; trading days, trading hours…)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Day,
    Hour,
    Minute,
    Week,
    Month,
}

fn unit_of(period: Period, n: u32) -> (Unit, u32) {
    match period {
        Period::Daily => (Unit::Day, n),
        Period::Intraday(m) => {
            let t = n.saturating_mul(m.max(1));
            if t % 60 == 0 {
                (Unit::Hour, t / 60)
            } else {
                (Unit::Minute, t)
            }
        }
        Period::Weekly => (Unit::Week, n),
        Period::Monthly => (Unit::Month, n),
    }
}

/// Period words for `n` bars (see the module docs).
#[derive(Debug, Clone, Copy)]
pub(super) struct Span {
    unit: Unit,
    n: u32,
}

impl Span {
    pub(super) fn of(period: Period, n: u32) -> Self {
        let (unit, n) = unit_of(period, n);
        Self { unit, n }
    }

    /// "20 günlük" / "20-day".
    pub(super) fn adj(self) -> Bi {
        let (tr, en) = match self.unit {
            Unit::Day => ("günlük", "day"),
            Unit::Hour => ("saatlik", "hour"),
            Unit::Minute => ("dakikalık", "minute"),
            Unit::Week => ("haftalık", "week"),
            Unit::Month => ("aylık", "month"),
        };
        bi(format!("{} {tr}", self.n), format!("{}-{en}", self.n))
    }

    /// "10 işlem günü" / "10 trading days".
    pub(super) fn trading(self) -> Bi {
        let (tr, one, many) = match self.unit {
            Unit::Day => ("işlem günü", "trading day", "trading days"),
            Unit::Hour => ("işlem saati", "trading hour", "trading hours"),
            Unit::Minute => ("işlem dakikası", "trading minute", "trading minutes"),
            Unit::Week => ("hafta", "week", "weeks"),
            Unit::Month => ("ay", "month", "months"),
        };
        let en = if self.n == 1 { one } else { many };
        bi(format!("{} {tr}", self.n), format!("{} {en}", self.n))
    }

    /// "20 işlem gününün" (TR genitive; EN: "20-day").
    fn trading_gen(self) -> Bi {
        let tr = match self.unit {
            Unit::Day => "işlem gününün",
            Unit::Hour => "işlem saatinin",
            Unit::Minute => "işlem dakikasının",
            Unit::Week => "haftanın",
            Unit::Month => "ayın",
        };
        bi(format!("{} {tr}", self.n), self.adj().en)
    }

    /// "20 günün" (TR genitive) / "20 days" (EN plural noun).
    fn short_gen(self) -> Bi {
        let (tr, en) = match self.unit {
            Unit::Day => ("günün", "days"),
            Unit::Hour => ("saatin", "hours"),
            Unit::Minute => ("dakikanın", "minutes"),
            Unit::Week => ("haftanın", "weeks"),
            Unit::Month => ("ayın", "months"),
        };
        bi(format!("{} {tr}", self.n), format!("{} {en}", self.n))
    }

    /// "14 işlem günündeki" (TR) / "14 trading days" (EN).
    pub(super) fn trading_loc_ki(self) -> Bi {
        let tr = match self.unit {
            Unit::Day => "işlem günündeki",
            Unit::Hour => "işlem saatindeki",
            Unit::Minute => "işlem dakikasındaki",
            Unit::Week => "haftadaki",
            Unit::Month => "aydaki",
        };
        bi(format!("{} {tr}", self.n), self.trading().en)
    }

    /// One unit: "gün" / "day".
    pub(super) fn one(period: Period) -> Bi {
        match Span::of(period, 1).unit {
            Unit::Day => bi("gün", "day"),
            Unit::Hour => bi("saat", "hour"),
            Unit::Minute => bi("dakika", "minute"),
            Unit::Week => bi("hafta", "week"),
            Unit::Month => bi("ay", "month"),
        }
    }
}

fn to_u32(x: usize) -> u32 {
    u32::try_from(x).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// The context
// ---------------------------------------------------------------------------

/// What the glossary needs to know (no text from the voice itself).
#[derive(Debug, Clone, Copy)]
pub struct GlossCtx {
    /// The bar period.
    pub period: Period,
    /// Story kind.
    pub kind: MeaningKind,
    /// Story direction.
    pub dir: Direction,
    /// No digits.
    pub redacted: bool,
}

struct Cx<'a> {
    inp: &'a SentenceInput<'a>,
    parts: &'a Parts,
    cal: &'a ExchangeCalendar,
    period: Period,
    story: &'a Story,
    lead: &'a Fact,
    latest: &'a Fact,
    det: &'a str,
    dir: Direction,
    red: bool,
    inst: String,
    /// Headline length limit ([`MAX_HEADLINE`]; less when a row stamp goes before it).
    room: usize,
}

/// Which way a detector's line was crossed / what the story's direction is.
fn up(d: Direction) -> bool {
    d == Direction::Up
}

fn opposite(a: Direction, b: Direction) -> bool {
    matches!(
        (a, b),
        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up)
    )
}

/// The ticker as a Turkish word: trailing digits are read as a number (`SYN03` →
/// "…üç" → `SYN03'te`), otherwise as a word (`THYAO'da`).
fn inst_word(sym: &str) -> Word {
    let digits: String = sym
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if digits.is_empty() {
        Word::proper(sym)
    } else {
        Word {
            written: sym.to_owned(),
            spoken: number_last_word(&digits).to_owned(),
            apostrophe: true,
            possessed: false,
        }
    }
}

/// A number in both languages, plus its Turkish word for suffixes.
#[derive(Debug, Clone)]
struct Num {
    tr: String,
    en: String,
}

impl Num {
    fn new(v: f64, cat: Cat) -> Self {
        Self {
            tr: fmt(v, cat, Lang::Tr),
            en: fmt(v, cat, Lang::En),
        }
    }
    fn w(&self) -> Word {
        Word::number(&self.tr)
    }
}

impl<'a> Cx<'a> {
    fn new(inp: &'a VoiceInput<'a>) -> Self {
        let s = inp.sentence;
        let det = s.lead.meaning.as_ref().map_or("", |m| m.detector.as_str());
        Self {
            inp: s,
            parts: inp.parts,
            cal: inp.calendar,
            period: inp.period,
            story: s.story,
            lead: s.lead,
            latest: s.latest,
            det,
            dir: s.story.direction,
            red: s.redacted || inp.parts.redacted,
            inst: s.story.instrument.clone(),
            room: MAX_HEADLINE,
        }
    }

    fn gloss_ctx(&self) -> GlossCtx {
        GlossCtx {
            period: self.period,
            kind: self.story.kind,
            dir: self.dir,
            redacted: self.red,
        }
    }

    /// An evidence value of `f`; never for a redacted reader.
    fn ev(&self, f: &Fact, k: &str) -> Option<f64> {
        if self.red {
            return None;
        }
        f.evidence.get(k).copied().filter(|v| v.is_finite())
    }

    fn num(&self, f: &Fact, k: &str, cat: Cat) -> Option<Num> {
        self.ev(f, k).map(|v| Num::new(v, cat))
    }

    /// The current value of the condition subject (never redacted).
    fn current(&self, cat: Cat) -> Option<Num> {
        if self.red {
            return None;
        }
        self.inp
            .current
            .filter(|v| v.is_finite())
            .map(|v| Num::new(v, cat))
    }

    fn span(&self, n: usize) -> Span {
        Span::of(self.period, to_u32(n))
    }

    fn span32(&self, n: u32) -> Span {
        Span::of(self.period, n)
    }

    /// Birth (the face's `when`).
    fn when(&self) -> Bi {
        bi(self.parts.when(Lang::Tr), self.parts.when(Lang::En))
    }

    /// Last transition (the face's `moved`).
    fn moved(&self) -> Bi {
        bi(self.parts.moved(Lang::Tr), self.parts.moved(Lang::En))
    }

    /// Elapsed phrase of a bar time.
    fn ago(&self, t: i64) -> Bi {
        let e = elapsed(self.cal, self.period, t, self.inp.now_t);
        if self.red {
            bi(phrase_vague(&e, Lang::Tr), phrase_vague(&e, Lang::En))
        } else {
            bi(phrase(&e, Lang::Tr), phrase(&e, Lang::En))
        }
    }

    fn inst_loc(&self) -> String {
        inst_word(&self.inst).loc()
    }

    fn latest_is_new(&self) -> bool {
        self.latest.id != self.lead.id
    }

    fn latest_is_exit(&self) -> bool {
        self.latest_is_new()
            && self
                .latest
                .meaning
                .as_ref()
                .is_some_and(|m| m.edge == Edge::Exit)
    }

    /// Bars left until the outcome is due (≥ 1).
    fn remaining(&self) -> u32 {
        let age = self.inp.now_bar.saturating_sub(self.story.born_bar);
        let left = u64::from(self.story.horizon_bars).saturating_sub(age);
        u32::try_from(left.max(1)).unwrap_or(u32::MAX)
    }

    // -----------------------------------------------------------------------
    // Trust chip and glossary terms
    // -----------------------------------------------------------------------

    fn trust(&self) -> Bi {
        let server = self.story.trust == Trust::Server
            || self.lead.meaning.as_ref().is_some_and(|m| m.server);
        if server {
            bi("Sunucu doğruladı", "Verified by the server")
        } else {
            bi("Bu cihazda sınandı", "Checked on this device")
        }
    }

    /// Glossary terms on this screen: the lead's, the latest fact's (when the detail
    /// tells it), then the track record's.
    fn terms(&self) -> Vec<Term> {
        let mut out: Vec<Term> = Vec::new();
        let mut add = |t: Term| {
            if !out.contains(&t) {
                out.push(t);
            }
        };
        for t in selvi_glossary::terms_of(self.det) {
            add(*t);
        }
        let detail = self.detail();
        if self.latest_is_new() {
            if let Some(m) = self.latest.meaning.as_ref() {
                for t in selvi_glossary::terms_of(&m.detector) {
                    if selvi_glossary::appears(*t, &detail.tr, Lang::Tr) {
                        add(*t);
                    }
                }
            }
        }
        add(Term::Ledger);
        out
    }
}

/// Whether a bilingual text carries a digit (the instrument symbol removed).
fn has_digit(b: &Bi, inst: &str) -> bool {
    let strip = |s: &str| s.replace(inst, " ");
    strip(&b.tr).bytes().any(|c| c.is_ascii_digit())
        || strip(&b.en).bytes().any(|c| c.is_ascii_digit())
}

mod clause;
mod detail;
mod headline;
mod holds;
mod ledger;

#[cfg(test)]
mod tests;
