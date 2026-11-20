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
    // Headline
    // -----------------------------------------------------------------------

    fn validity_part(&self) -> Bi {
        match (self.parts.validity, self.parts.ending) {
            (Validity::Valid, _) => bi("Hâlâ geçerli.", "Still holds."),
            (Validity::NoLonger, _) => bi("Artık geçerli değil.", "No longer holds."),
            (Validity::Closed, Some(Ending::Realized)) => {
                bi("Kapandı: gerçekleşti.", "Closed: played out.")
            }
            (Validity::Closed, Some(Ending::Failed)) => {
                bi("Kapandı: tutmadı.", "Closed: did not play out.")
            }
            (Validity::Closed, Some(Ending::Expired)) => {
                bi("Kapandı: süresi doldu.", "Closed: ran out of time.")
            }
            (Validity::Closed, Some(Ending::Retracted)) => {
                bi("Kapandı: geri çekildi.", "Closed: withdrawn.")
            }
            (Validity::Closed, None) => bi("Kapandı.", "Closed."),
        }
    }

    /// The short form of the validity part: a closed capsule's ending alone ("Tutmadı.").
    fn validity_short(&self) -> Bi {
        match (self.parts.validity, self.parts.ending) {
            (Validity::Closed, Some(Ending::Realized)) => bi("Gerçekleşti.", "Played out."),
            (Validity::Closed, Some(Ending::Failed)) => bi("Tutmadı.", "Did not play out."),
            (Validity::Closed, Some(Ending::Expired)) => bi("Süresi doldu.", "Ran out of time."),
            (Validity::Closed, Some(Ending::Retracted)) => bi("Geri çekildi.", "Withdrawn."),
            _ => self.validity_part(),
        }
    }

    fn headline(&self) -> Bi {
        let (v, vs) = (self.validity_part(), self.validity_short());
        let mains = self.headline_mains();
        let mut cands: Vec<Bi> = Vec::new();
        for m in &mains {
            cands.push(Bi::join(&[m.clone(), v.clone()]));
            if vs != v {
                cands.push(Bi::join(&[m.clone(), vs.clone()]));
            }
        }
        first_fit(cands, |b| b.fits(self.room))
    }

    /// Main sentences, most preferred first (each ends with a full stop).
    fn headline_mains(&self) -> Vec<Bi> {
        let i = self.inst.as_str();
        let w = self.when();
        let mut out: Vec<Option<Bi>> = match self.det {
            "breakout.range" => self.hl_breakout(i, &w),
            "extreme.rsi" => self.hl_extreme(i, &w),
            "continuation.ma_stack" => self.hl_ma(i, &w),
            "reversal.rsi_divergence" => self.hl_divergence(i, &w),
            "squeeze.bandwidth" => self.hl_squeeze(i, &w),
            "flow.volume_surge" => self.hl_volume(i, &w),
            "flow.institutional" => self.hl_institutional(i, &w),
            _ => Vec::new(),
        };
        out.push(Some(self.hl_generic(i, &w)));
        out.into_iter()
            .flatten()
            .filter(|b| !self.red || !has_digit(b, &self.inst))
            .collect()
    }

    fn hl_generic(&self, i: &str, w: &Bi) -> Bi {
        let k = self.story.kind;
        bi(
            format!(
                "{i} {}: {}.",
                w.tr,
                selvi_glossary::plain_event(k, self.dir, Lang::Tr)
            ),
            format!(
                "{i}: {}, {}.",
                selvi_glossary::plain_event(k, self.dir, Lang::En),
                w.en
            ),
        )
    }

    fn hl_breakout(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let span = self.span(RangeBreakout::WINDOW);
        let (u, a) = (span.short_gen(), span.adj());
        let l = self.num(self.lead, "level", Cat::Price);
        if up(self.dir) {
            vec![
                l.as_ref().map(|l| {
                    bi(
                        format!(
                            "{i} {} son {} en yükseği olan {} aştı.",
                            w.tr,
                            u.tr,
                            l.w().acc()
                        ),
                        format!(
                            "{i} broke above its prior {} high of {} {}.",
                            a.en, l.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!("{i} {} son {} en yükseğini aştı.", w.tr, u.tr),
                    format!("{i} broke above its prior {} high {}.", a.en, w.en),
                )),
                Some(bi(
                    format!("{i} {} son zirvesini aştı.", w.tr),
                    format!("{i} broke above its recent high {}.", w.en),
                )),
            ]
        } else {
            vec![
                l.as_ref().map(|l| {
                    bi(
                        format!(
                            "{i} {} son {} en düşüğü olan {} altına indi.",
                            w.tr,
                            u.tr,
                            l.w().gen()
                        ),
                        format!(
                            "{i} broke below its prior {} low of {} {}.",
                            a.en, l.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!("{i} {} son {} en düşüğünün altına indi.", w.tr, u.tr),
                    format!("{i} broke below its prior {} low {}.", a.en, w.en),
                )),
                Some(bi(
                    format!("{i} {} son en düşüğünün altına indi.", w.tr),
                    format!("{i} broke below its recent low {}.", w.en),
                )),
            ]
        }
    }

    fn rsi_line(&self) -> Option<Num> {
        let fallback = if up(self.dir) {
            RsiExtreme::UPPER
        } else {
            RsiExtreme::LOWER
        };
        if self.red {
            return None;
        }
        Some(Num::new(
            self.ev(self.lead, "level").unwrap_or(fallback),
            Cat::Osc,
        ))
    }

    fn hl_extreme(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let l = self.rsi_line();
        let il = self.inst_loc();
        let Some(l) = l else {
            return Vec::new();
        };
        if up(self.dir) {
            vec![
                Some(bi(
                    format!(
                        "{i} kısa sürede çok yükseldi: RSI {} {} üstüne çıktı.",
                        w.tr,
                        l.w().gen()
                    ),
                    format!("{i} rose fast: its RSI went above {} {}.", l.en, w.en),
                )),
                Some(bi(
                    format!("{il} RSI {} {} üstüne çıktı.", w.tr, l.w().gen()),
                    format!("{i}'s RSI went above {} {}.", l.en, w.en),
                )),
                Some(bi(
                    format!("{il} RSI {} {} aştı.", w.tr, l.w().acc()),
                    format!("{i}'s RSI topped {} {}.", l.en, w.en),
                )),
            ]
        } else {
            vec![
                Some(bi(
                    format!(
                        "{i} kısa sürede çok düştü: RSI {} {} altına indi.",
                        w.tr,
                        l.w().gen()
                    ),
                    format!("{i} fell fast: its RSI went below {} {}.", l.en, w.en),
                )),
                Some(bi(
                    format!("{il} RSI {} {} altına indi.", w.tr, l.w().gen()),
                    format!("{i}'s RSI went below {} {}.", l.en, w.en),
                )),
            ]
        }
    }

    fn hl_ma(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        // (R5/m17-int, S1) The event first and plain: the price is in a fall / kept falling
        // ("düşüşte" / "kept falling"), the average after it as where the price stands —
        // never "altına yerleşti", which readers took for a threshold crossing.
        let a = self.span(MaStack::FAST).adj();
        let s20 = self.num(self.lead, "sma20", Cat::Price);
        let (tr_state, tr_side, en_verb) = if up(self.dir) {
            ("yükselişte", "üstünde", "kept rising above")
        } else {
            ("düşüşte", "altında", "kept falling below")
        };
        vec![
            s20.as_ref().map(|s| {
                bi(
                    format!(
                        "{i} {} {tr_state}: {} ortalama ({}) {tr_side}.",
                        w.tr, a.tr, s.tr
                    ),
                    format!("{i} {en_verb} {} average ({}) {}.", a.en, s.en, w.en),
                )
            }),
            Some(bi(
                format!("{i} {} {tr_state}: {} ortalama {tr_side}.", w.tr, a.tr),
                format!("{i} {en_verb} {} average {}.", a.en, w.en),
            )),
            Some(bi(
                format!("{i} {} {tr_state}, ortalamanın {tr_side}.", w.tr),
                format!("{i} {en_verb} its average {}.", w.en),
            )),
        ]
    }

    fn hl_divergence(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let p2 = self.num(self.lead, "p2_price", Cat::Price);
        if up(self.dir) {
            vec![
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha derin bir dip yaptı ama RSI yukarıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a lower low at {} {}, but its RSI stayed higher.",
                            p.en, w.en
                        ),
                    )
                }),
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha derin dip yaptı, RSI yukarıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a lower low at {} {}, but RSI stayed higher.",
                            p.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!(
                        "{i} {} daha derin bir dip yaptı ama RSI yukarıda kaldı.",
                        w.tr
                    ),
                    format!("{i} made a lower low {}, but its RSI stayed higher.", w.en),
                )),
                Some(bi(
                    format!("{i} {} dip yaptı, RSI yukarıda kaldı.", w.tr),
                    format!("{i} made a low {}, its RSI stayed higher.", w.en),
                )),
            ]
        } else {
            vec![
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha yüksek bir tepe yaptı ama RSI aşağıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a higher high at {} {}, but its RSI stayed lower.",
                            p.en, w.en
                        ),
                    )
                }),
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha yüksek tepe yaptı, RSI aşağıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a higher high at {} {}, but RSI stayed lower.",
                            p.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!(
                        "{i} {} daha yüksek bir tepe yaptı ama RSI aşağıda kaldı.",
                        w.tr
                    ),
                    format!("{i} made a higher high {}, but its RSI stayed lower.", w.en),
                )),
                Some(bi(
                    format!("{i} {} tepe yaptı, RSI aşağıda kaldı.", w.tr),
                    format!("{i} made a high {}, its RSI stayed lower.", w.en),
                )),
            ]
        }
    }

    fn hl_squeeze(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let u = self.span(BandSqueeze::LOOKBACK).short_gen();
        let lo = self.num(self.lead, "lower", Cat::Price);
        let hi = self.num(self.lead, "upper", Cat::Price);
        vec![
            lo.as_ref().zip(hi.as_ref()).map(|(lo, hi)| {
                bi(
                    format!(
                        "{i} {} {} en dar aralığına sıkıştı: bant {}–{}.",
                        w.tr, u.tr, lo.tr, hi.tr
                    ),
                    format!(
                        "{i} hit its tightest band in {} {}: {}–{}.",
                        u.en, w.en, lo.en, hi.en
                    ),
                )
            }),
            Some(bi(
                format!("{i} {} {} en dar bant aralığına sıkıştı.", w.tr, u.tr),
                format!("{i} hit its tightest band in {} {}.", u.en, w.en),
            )),
            Some(bi(
                format!("{i} {} en dar bant aralığına sıkıştı.", w.tr),
                format!("{i} hit its tightest band {}.", w.en),
            )),
        ]
    }

    fn hl_volume(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let a = self.span(VolumeSurge::WINDOW).adj();
        let r = self.num(self.lead, "ratio", Cat::Osc);
        let (tr_v, en_v) = match self.dir {
            Direction::Up => ("yükseldi", "rose"),
            Direction::Down => ("düştü", "fell"),
            Direction::Neutral => ("yatay kapandı", "closed flat"),
        };
        vec![
            r.as_ref().map(|r| {
                bi(
                    format!(
                        "{i} {} {} ortalamanın {} katı hacimle {tr_v}.",
                        w.tr, a.tr, r.tr
                    ),
                    format!(
                        "{i} {en_v} on {} times its {} average volume {}.",
                        r.en, a.en, w.en
                    ),
                )
            }),
            r.as_ref().map(|r| {
                bi(
                    format!("{i} {} ortalamanın {} katı hacimle {tr_v}.", w.tr, r.tr),
                    format!("{i} {en_v} on {} times its average volume {}.", r.en, w.en),
                )
            }),
            Some(bi(
                format!("{i} {} çok yüksek hacimle {tr_v}.", w.tr),
                format!("{i} {en_v} on very high volume {}.", w.en),
            )),
            Some(bi(
                format!("{i} {} yüksek hacimle {tr_v}.", w.tr),
                format!("{i} {en_v} on high volume {}.", w.en),
            )),
        ]
    }

    fn hl_institutional(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let a = self.span(InstitutionalFlow::WINDOW).adj();
        let il = self.inst_loc();
        let (tr_v, en_v) = if up(self.dir) {
            ("aldı", "bought")
        } else {
            ("sattı", "sold")
        };
        let m = (!self.red).then(|| Num::new(InstitutionalFlow::MULT, Cat::Osc));
        vec![
            m.as_ref().map(|m| {
                bi(
                    format!(
                        "Kurumlar {} {il} {} ortalamanın {} katından fazla net {tr_v}.",
                        w.tr, a.tr, m.tr
                    ),
                    format!(
                        "Institutions net-{en_v} {i} at over {}× their {} average {}.",
                        m.en, a.en, w.en
                    ),
                )
            }),
            m.as_ref().map(|m| {
                bi(
                    format!(
                        "Kurumlar {} {il} ortalamanın {} katından fazla net {tr_v}.",
                        w.tr, m.tr
                    ),
                    format!(
                        "Institutions net-{en_v} {i} at over {}× their average {}.",
                        m.en, w.en
                    ),
                )
            }),
            Some(bi(
                format!("Kurumlar {} {il} her zamankinden çok net {tr_v}.", w.tr),
                format!("Institutions net-{en_v} {i} far more than usual {}.", w.en),
            )),
            Some(bi(
                format!("Kurumlar {} {il} yoğun net {tr_v}.", w.tr),
                format!("Institutions net-{en_v} {i} heavily {}.", w.en),
            )),
        ]
    }

    // -----------------------------------------------------------------------
    // Detail
    // -----------------------------------------------------------------------

    fn detail(&self) -> Bi {
        let s1s = self.d_threshold();
        let s2 = self.d_latest();
        let s3s = self.d_state();
        let s3 = s3s.first().cloned().unwrap_or_default();
        let st = self.d_trust();
        let mut cands: Vec<Bi> = Vec::new();
        let opt = |b: &Option<Bi>| b.clone().unwrap_or_default();
        // Most complete first. What changed since birth (S2) is kept before the long
        // forms: the line and the state fall back to their compact forms first; then
        // the trust note goes (never the redacted reader's), then S2.
        for with2 in [true, false] {
            for s1 in &s1s {
                for s3v in &s3s {
                    for with_t in [true, false] {
                        if self.red && !with_t {
                            continue;
                        }
                        let parts = [
                            s1.clone(),
                            if with2 { opt(&s2) } else { Bi::default() },
                            s3v.clone(),
                            if with_t { opt(&st) } else { Bi::default() },
                        ];
                        cands.push(Bi::join(&parts));
                    }
                }
            }
        }
        let ok = |b: &Bi| {
            b.fits(MAX_DETAIL)
                && sentence_count(&b.tr) <= MAX_DETAIL_SENTENCES
                && sentence_count(&b.en) <= MAX_DETAIL_SENTENCES
        };
        if let Some(b) = cands.iter().find(|b| ok(b)) {
            return b.clone();
        }
        let s3 = &s3;
        // Last resort: the threshold and the state, the threshold in its short form.
        first_fit(
            vec![
                Bi::join(&[self.d_threshold_short(), s3.clone()]),
                s3.clone(),
            ],
            ok,
        )
    }

    /// The trust note for the open capsule: the redacted reader always learns which
    /// entitlement opens the figures and who verified the claim; an entitled reader of a
    /// server fact learns that the server verified it.
    fn d_trust(&self) -> Option<Bi> {
        let server = self.story.trust == Trust::Server
            || self.lead.meaning.as_ref().is_some_and(|m| m.server);
        if self.red {
            let (tr_feed, en_feed) = self.feed_name();
            let verified_tr = if server {
                ", bunu sunucu doğruladı"
            } else {
                ""
            };
            let verified_en = if server {
                ", and the server verified it"
            } else {
                ""
            };
            return Some(bi(
                format!("Rakamlar {tr_feed} verisi yetkisiyle açılır{verified_tr}."),
                format!("The figures open with a {en_feed} data entitlement{verified_en}."),
            ));
        }
        server.then(|| bi("Bunu sunucu doğruladı.", "The server verified this."))
    }

    fn feed_name(&self) -> (&'static str, &'static str) {
        let ent = self
            .lead
            .meaning
            .as_ref()
            .and_then(|m| m.license.entitlement.as_deref())
            .unwrap_or("");
        if ent.starts_with("flow") {
            ("kurum akışı", "broker-flow")
        } else {
            ("lisanslı", "licensed")
        }
    }

    /// S1: the line, with its name, value and parameter — the full sentence, then its
    /// compact form (both carry the name, the value and the parameter).
    fn d_threshold(&self) -> Vec<Bi> {
        let w = self.when();
        let got = match self.det {
            "breakout.range" => self.d1_breakout(&w),
            "extreme.rsi" => self.d1_extreme(&w),
            "continuation.ma_stack" => self.d1_ma(&w),
            "reversal.rsi_divergence" => self.d1_divergence(),
            "squeeze.bandwidth" => self.d1_squeeze(&w),
            "flow.volume_surge" => self.d1_volume(&w),
            "flow.institutional" => self.d1_institutional(&w),
            _ => None,
        };
        let mut out: Vec<Bi> = got.into_iter().collect();
        out.extend(self.d1_compact());
        if out.is_empty() {
            out.push(self.d_threshold_short());
        }
        out
    }

    /// The compact form of S1 (numbers only; `None` for a redacted reader).
    fn d1_compact(&self) -> Option<Bi> {
        let f = self.lead;
        Some(match self.det {
            "breakout.range" => {
                let s = self.span(RangeBreakout::WINDOW);
                let l = self.num(f, "level", Cat::Price)?;
                let c = self.num(f, "close", Cat::Price)?;
                let (tr, en) = if up(self.dir) {
                    ("en yükseği", "high")
                } else {
                    ("en düşüğü", "low")
                };
                bi(
                    format!(
                        "Önceki {} {tr} {}, kapanış {}.",
                        s.trading_gen().tr,
                        l.tr,
                        c.tr
                    ),
                    format!("Prior {} {en} {}, close {}.", s.adj().en, l.en, c.en),
                )
            }
            "extreme.rsi" => {
                let v = self.num(f, "value", Cat::Osc)?;
                let l = self.rsi_line()?;
                bi(
                    format!("RSI {} oldu, çizgi {}.", v.tr, l.tr),
                    format!("RSI reached {}, the line is {}.", v.en, l.en),
                )
            }
            "continuation.ma_stack" => {
                let a = self.num(f, "sma20", Cat::Price)?;
                let b = self.num(f, "sma50", Cat::Price)?;
                let (af, asl) = (
                    self.span(MaStack::FAST).adj(),
                    self.span(MaStack::SLOW).adj(),
                );
                bi(
                    format!(
                        "{} ortalama {}, {} ortalama {} idi.",
                        capitalize(&af.tr),
                        a.tr,
                        asl.tr,
                        b.tr
                    ),
                    format!(
                        "The {} average was {} and the {} {}.",
                        af.en, a.en, asl.en, b.en
                    ),
                )
            }
            "reversal.rsi_divergence" => {
                let p1 = self.num(f, "p1_price", Cat::Price)?;
                let p2 = self.num(f, "p2_price", Cat::Price)?;
                let r1 = self.num(f, "p1_rsi", Cat::Osc)?;
                let r2 = self.num(f, "p2_rsi", Cat::Osc)?;
                let (tr, en) = if up(self.dir) {
                    ("dip", "low")
                } else {
                    ("tepe", "high")
                };
                bi(
                    format!(
                        "Önceki {tr} {} (RSI {}), yeni {tr} {} (RSI {}).",
                        p1.tr, r1.tr, p2.tr, r2.tr
                    ),
                    format!(
                        "Previous {en} {} (RSI {}), new {en} {} (RSI {}).",
                        p1.en, r1.en, p2.en, r2.en
                    ),
                )
            }
            "squeeze.bandwidth" => {
                let lo = self.num(f, "lower", Cat::Price)?;
                let hi = self.num(f, "upper", Cat::Price)?;
                let u = self.span(BandSqueeze::LOOKBACK).short_gen();
                bi(
                    format!("Bant {}–{}, son {} en darı.", lo.tr, hi.tr, u.tr),
                    format!("Band {}–{}, the narrowest in {}.", lo.en, hi.en, u.en),
                )
            }
            "flow.volume_surge" => {
                let r = self.num(f, "ratio", Cat::Osc)?;
                let m = Num::new(VolumeSurge::MULT, Cat::Osc);
                let a = self.span(VolumeSurge::WINDOW).adj();
                bi(
                    format!(
                        "Hacim {} ortalamanın {} katıydı, eşik {} kat.",
                        a.tr, r.tr, m.tr
                    ),
                    format!(
                        "Volume was {} times the {} average, the mark {}×.",
                        r.en, a.en, m.en
                    ),
                )
            }
            "flow.institutional" => {
                let net = self.num(f, "net", Cat::Count)?;
                let avg = self.num(f, "avg5", Cat::Count)?;
                let m = Num::new(InstitutionalFlow::MULT, Cat::Osc);
                let a = self.span(InstitutionalFlow::WINDOW).adj();
                let (tr, en) = if up(self.dir) {
                    ("alımı", "buying")
                } else {
                    ("satışı", "selling")
                };
                bi(
                    format!("En çok işlem yapan kurumun net {tr} {} TL, {} ortalama {} TL, eşik {} kat.", net.tr, a.tr, avg.tr, m.tr),
                    format!("Top institution net {en} TRY {}, {} average TRY {}, mark {}×.", net.en, a.en, avg.en, m.en),
                )
            }
            _ => return None,
        })
    }

    /// The shortest statement of the line (no number when redacted).
    fn d_threshold_short(&self) -> Bi {
        let side_up = up(self.dir);
        let red = self.red;
        let b = |tr: String, en: String| bi(tr, en);
        match self.det {
            "breakout.range" if !red => {
                let s = self.span(RangeBreakout::WINDOW);
                let l = self.num(self.lead, "level", Cat::Price);
                let lv = l.map(|l| (l.tr, l.en)).unwrap_or_default();
                if side_up {
                    b(
                        format!("Çizgi önceki {} en yükseği, {}.", s.trading_gen().tr, lv.0),
                        format!("The line is the prior {} high, {}.", s.adj().en, lv.1),
                    )
                } else {
                    b(
                        format!("Çizgi önceki {} en düşüğü, {}.", s.trading_gen().tr, lv.0),
                        format!("The line is the prior {} low, {}.", s.adj().en, lv.1),
                    )
                }
            }
            "flow.institutional" if red => {
                let (tr, en) = if side_up {
                    ("alımı", "buying")
                } else {
                    ("satışı", "selling")
                };
                b(
                    format!("En çok net işlem yapan kurumun net {tr} son günlerdeki olağan düzeyinin çok üstündeydi."),
                    format!("The top institution's net {en} was far beyond its recent usual level."),
                )
            }
            _ => {
                let e = selvi_glossary::plain_event(self.story.kind, self.dir, Lang::Tr);
                let ee = selvi_glossary::plain_event(self.story.kind, self.dir, Lang::En);
                b(format!("Olan: {e}."), format!("What happened: {ee}."))
            }
        }
    }

    fn d1_breakout(&self, w: &Bi) -> Option<Bi> {
        let s = self.span(RangeBreakout::WINDOW);
        let l = self.num(self.lead, "level", Cat::Price)?;
        let c = self.num(self.lead, "close", Cat::Price)?;
        let wc = w.cap();
        Some(if up(self.dir) {
            bi(
                format!(
                    "{} kapanış {} oldu, önceki {} en yükseği {} idi.",
                    wc.tr,
                    c.tr,
                    s.trading_gen().tr,
                    l.tr
                ),
                format!(
                    "{}, it closed at {}, above the prior {} high of {}.",
                    wc.en,
                    c.en,
                    s.adj().en,
                    l.en
                ),
            )
        } else {
            bi(
                format!(
                    "{} kapanış {} oldu, önceki {} en düşüğü {} idi.",
                    wc.tr,
                    c.tr,
                    s.trading_gen().tr,
                    l.tr
                ),
                format!(
                    "{}, it closed at {}, below the prior {} low of {}.",
                    wc.en,
                    c.en,
                    s.adj().en,
                    l.en
                ),
            )
        })
    }

    fn d1_extreme(&self, w: &Bi) -> Option<Bi> {
        let v = self.num(self.lead, "value", Cat::Osc)?;
        let l = self.rsi_line()?;
        let wc = w.cap();
        Some(if up(self.dir) {
            bi(
                format!("{} RSI {} oldu ve {} çizgisini geçti.", wc.tr, v.tr, l.tr),
                format!("{}, RSI reached {}, past the {} line.", wc.en, v.en, l.en),
            )
        } else {
            bi(
                format!(
                    "{} RSI {} oldu ve {} çizgisinin altına indi.",
                    wc.tr, v.tr, l.tr
                ),
                format!("{}, RSI fell to {}, below the {} line.", wc.en, v.en, l.en),
            )
        })
    }

    fn d1_ma(&self, w: &Bi) -> Option<Bi> {
        let c = self.num(self.lead, "close", Cat::Price)?;
        let f = self.num(self.lead, "sma20", Cat::Price)?;
        let s = self.num(self.lead, "sma50", Cat::Price)?;
        let af = self.span(MaStack::FAST).adj();
        let asl = self.span(MaStack::SLOW).adj();
        let (tr_d, en_d) = if up(self.dir) {
            ("yukarı", "upward")
        } else {
            ("aşağı", "downward")
        };
        let wc = w.cap();
        Some(bi(
            format!(
                "{} fiyat {}, {} ortalama {}, {} ortalama {} idi: sıra {tr_d} doğru.",
                wc.tr, c.tr, af.tr, f.tr, asl.tr, s.tr
            ),
            format!(
                "{}, the price was {}, with the {} average at {} and the {} at {}, lined up {en_d}.",
                wc.en, c.en, af.en, f.en, asl.en, s.en
            ),
        ))
    }

    fn d1_divergence(&self) -> Option<Bi> {
        let p1 = self.num(self.lead, "p1_price", Cat::Price)?;
        let p2 = self.num(self.lead, "p2_price", Cat::Price)?;
        let r1 = self.num(self.lead, "p1_rsi", Cat::Osc)?;
        let r2 = self.num(self.lead, "p2_rsi", Cat::Osc)?;
        Some(if up(self.dir) {
            bi(
                format!(
                    "Önceki dipte fiyat {} ve RSI {} idi, yeni dipte fiyat {} ve RSI {}: düşüş hız kesiyor.",
                    p1.tr, r1.tr, p2.tr, r2.tr
                ),
                format!(
                    "At the previous low the price was {} with RSI at {}, at the new low {} with RSI at {}: the fall is losing pace.",
                    p1.en, r1.en, p2.en, r2.en
                ),
            )
        } else {
            bi(
                format!(
                    "Önceki tepede fiyat {} ve RSI {} idi, yeni tepede fiyat {} ve RSI {}: yükseliş hız kesiyor.",
                    p1.tr, r1.tr, p2.tr, r2.tr
                ),
                format!(
                    "At the previous high the price was {} with RSI at {}, at the new high {} with RSI at {}: the rise is losing pace.",
                    p1.en, r1.en, p2.en, r2.en
                ),
            )
        })
    }

    fn d1_squeeze(&self, w: &Bi) -> Option<Bi> {
        let lo = self.num(self.lead, "lower", Cat::Price)?;
        let hi = self.num(self.lead, "upper", Cat::Price)?;
        let u = self.span(BandSqueeze::LOOKBACK).short_gen();
        let wc = w.cap();
        Some(bi(
            format!(
                "{} bant {} ile {} arasına daraldı, bu son {} en darı.",
                wc.tr, lo.tr, hi.tr, u.tr
            ),
            format!(
                "{}, the band narrowed to {}–{}, its narrowest in {}.",
                wc.en, lo.en, hi.en, u.en
            ),
        ))
    }

    fn d1_volume(&self, w: &Bi) -> Option<Bi> {
        let v = self.num(self.lead, "volume", Cat::Count)?;
        let a = self.num(self.lead, "vol_avg", Cat::Count)?;
        let m = Num::new(VolumeSurge::MULT, Cat::Osc);
        let ad = self.span(VolumeSurge::WINDOW).adj();
        let wc = w.cap();
        Some(bi(
            format!(
                "{} {} adet hisse el değiştirdi, {} ortalama {} adet, eşik bunun {} katı.",
                wc.tr, v.tr, ad.tr, a.tr, m.tr
            ),
            format!(
                "{}, {} shares changed hands against a {} average of {}, and the mark is {} times that.",
                wc.en, v.en, ad.en, a.en, m.en
            ),
        ))
    }

    fn d1_institutional(&self, w: &Bi) -> Option<Bi> {
        let net = self.num(self.lead, "net", Cat::Count)?;
        let avg = self.num(self.lead, "avg5", Cat::Count)?;
        let m = Num::new(InstitutionalFlow::MULT, Cat::Osc);
        let s = self.span(InstitutionalFlow::WINDOW);
        let (tr, en) = if up(self.dir) {
            ("alımı", "buying")
        } else {
            ("satışı", "selling")
        };
        let wc = w.cap();
        Some(bi(
            format!(
                "{} en çok net işlem yapan kurumun net {tr} {} TL oldu, son {} ortalaması {} TL, eşik bunun {} katı.",
                wc.tr, net.tr, s.short_gen().tr, avg.tr, m.tr
            ),
            format!(
                "{}, the top institution's net {en} was TRY {} against a {} average of TRY {}, and the mark is {} times that.",
                wc.en, net.en, s.adj().en, avg.en, m.en
            ),
        ))
    }

    /// S2: the latest fact that joined, if it is not the lead (and not the exit the state
    /// sentence already tells).
    fn d_latest(&self) -> Option<Bi> {
        if !self.latest_is_new()
            || (self.parts.validity == Validity::NoLonger && self.latest_is_exit())
        {
            return None;
        }
        if self.red {
            return None;
        }
        let m = self.latest.meaning.as_ref()?;
        let against = opposite(self.dir, m.direction);
        let also = !against && m.edge != Edge::Exit;
        let c = self.clause(self.latest, also)?;
        let t = self.ago(self.latest.t);
        let tc = t.cap();
        Some(if against {
            bi(
                format!("{} bunun tersine {}.", tc.tr, c.tr),
                format!("Against it, {} {}.", c.en, t.en),
            )
        } else {
            bi(
                format!("{} {}.", tc.tr, c.tr),
                format!("{} {}.", cap_en(&c.en), t.en),
            )
        })
    }

    /// A clause about fact `f` (no time, no full stop). `also`: it adds to the story.
    fn clause(&self, f: &Fact, also: bool) -> Option<Bi> {
        let m = f.meaning.as_ref()?;
        let u = up(m.direction);
        let exit = m.edge == Edge::Exit;
        // A licensed fact's figures are shown only when the reader holds the same
        // entitlement as for the lead (entitled to the lead's licensed figures).
        let licensed = m.license.class == LicenseClass::Licensed;
        let lead_lic = self.lead.meaning.as_ref().map(|lm| &lm.license);
        let figures = !self.red
            && (!licensed
                || lead_lic.is_some_and(|ll| {
                    ll.class == LicenseClass::Licensed && ll.entitlement == m.license.entitlement
                }));
        let da = |w: &str| -> String {
            if !also {
                return String::new();
            }
            // "RSI" is read "ar-es-ay" here: "RSI da".
            let last = if w == "rsi" {
                'a'
            } else {
                w.chars()
                    .rev()
                    .find(|c| "aıoueiöü".contains(*c))
                    .unwrap_or('e')
            };
            let a = crate::realize::morph::a2(last);
            format!(" d{a}")
        };
        let ae = if also { " also" } else { "" };
        Some(match m.detector.as_str() {
            "breakout.range" => {
                let s = self.span(RangeBreakout::WINDOW);
                let l = self.num(f, "level", Cat::Price)?;
                if u {
                    bi(
                        format!(
                            "fiyat{} önceki {} en yükseğini ({}) aştı",
                            da("fiyat"),
                            s.short_gen().tr,
                            l.tr
                        ),
                        format!(
                            "the price{ae} broke its prior {} high ({})",
                            s.adj().en,
                            l.en
                        ),
                    )
                } else {
                    bi(
                        format!(
                            "fiyat{} önceki {} en düşüğünün ({}) altına indi",
                            da("fiyat"),
                            s.short_gen().tr,
                            l.tr
                        ),
                        format!(
                            "the price{ae} broke below its prior {} low ({})",
                            s.adj().en,
                            l.en
                        ),
                    )
                }
            }
            "extreme.rsi" => {
                let l = Num::new(
                    self.ev(f, "level").unwrap_or(if u {
                        RsiExtreme::UPPER
                    } else {
                        RsiExtreme::LOWER
                    }),
                    Cat::Osc,
                );
                match (exit, u) {
                    (false, true) => bi(
                        format!("RSI{} {} üstüne çıktı", da("rsi"), l.w().gen()),
                        format!("RSI{ae} went above {}", l.en),
                    ),
                    (false, false) => bi(
                        format!("RSI{} {} altına indi", da("rsi"), l.w().gen()),
                        format!("RSI{ae} went below {}", l.en),
                    ),
                    (true, true) => bi(
                        format!("RSI {} altına döndü", l.w().gen()),
                        format!("RSI went back below {}", l.en),
                    ),
                    (true, false) => bi(
                        format!("RSI {} üstüne döndü", l.w().gen()),
                        format!("RSI went back above {}", l.en),
                    ),
                }
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                match (exit, u) {
                    (false, _) => {
                        let s = self.num(f, "sma20", Cat::Price)?;
                        let (ts, es) = if u {
                            ("üstüne", "above")
                        } else {
                            ("altına", "below")
                        };
                        bi(
                            format!(
                                "fiyat{} {} ortalamanın ({}) {ts} yerleşti",
                                da("fiyat"),
                                a.tr,
                                s.tr
                            ),
                            format!("the price{ae} settled {es} its {} average ({})", a.en, s.en),
                        )
                    }
                    (true, true) => bi(
                        format!("fiyat {} ortalamanın altına indi", a.tr),
                        format!("the price fell back below its {} average", a.en),
                    ),
                    (true, false) => bi(
                        format!("fiyat {} ortalamanın üstüne çıktı", a.tr),
                        format!("the price rose back above its {} average", a.en),
                    ),
                }
            }
            "reversal.rsi_divergence" => {
                let again = if also { " yine" } else { "" };
                let again_en = if also { " again" } else { "" };
                bi(
                    format!("fiyatla RSI{again} ters yönlere gitti"),
                    format!("price and RSI moved apart{again_en}"),
                )
            }
            "squeeze.bandwidth" => {
                let u120 = self.span(BandSqueeze::LOOKBACK).short_gen();
                if exit {
                    let lo = self.num(f, "lower", Cat::Price)?;
                    let hi = self.num(f, "upper", Cat::Price)?;
                    bi(
                        format!("bant yeniden genişledi ({}–{})", lo.tr, hi.tr),
                        format!("the band widened again ({}–{})", lo.en, hi.en),
                    )
                } else {
                    bi(
                        format!("bant{} son {} en darına indi", da("bant"), u120.tr),
                        format!("the band{ae} hit its narrowest in {}", u120.en),
                    )
                }
            }
            "flow.volume_surge" => {
                let r = self.num(f, "ratio", Cat::Osc)?;
                let a = self.span(VolumeSurge::WINDOW).adj();
                bi(
                    format!(
                        "hacim{} {} ortalamanın {} katına çıktı",
                        da("hacim"),
                        a.tr,
                        r.tr
                    ),
                    format!("volume{ae} reached {} times its {} average", r.en, a.en),
                )
            }
            "flow.institutional" if !figures => {
                let (tv, ev) = if u {
                    ("aldı", "bought")
                } else {
                    ("sattı", "sold")
                };
                bi(
                    format!("kurumlar{} her zamankinden çok net {tv}", da("kurumlar")),
                    format!("institutions{ae} net-{ev} far more than usual"),
                )
            }
            "flow.institutional" => {
                let (tv, ev) = if u {
                    ("aldı", "bought")
                } else {
                    ("sattı", "sold")
                };
                let a = self.span(InstitutionalFlow::WINDOW).adj();
                let net = self.ev(f, "net")?;
                let avg = self.ev(f, "avg5").filter(|v| *v > 0.0)?;
                let r = Num::new(net.abs() / avg, Cat::Osc);
                bi(
                    format!(
                        "kurumlar{} {} ortalamanın {} katı net {tv}",
                        da("kurumlar"),
                        a.tr,
                        r.tr
                    ),
                    format!(
                        "institutions{ae} net-{ev} {} times their {} average",
                        r.en, a.en
                    ),
                )
            }
            _ => return None,
        })
    }

    /// S3: does it still hold, and when is the outcome due — or how it closed. The
    /// full form first, then the compact one.
    fn d_state(&self) -> Vec<Bi> {
        match self.parts.validity {
            Validity::Closed => vec![self.d_closed()],
            Validity::NoLonger => vec![self.d_no_longer()],
            Validity::Valid => {
                let mut v = vec![self.d_holds()];
                v.extend(self.d_holds_compact());
                v
            }
        }
    }

    /// The holding condition alone (no "when is the result due").
    fn d_holds_compact(&self) -> Option<Bi> {
        let side_up = up(self.dir);
        let (ts, es) = if side_up {
            ("üstünde", "above")
        } else {
            ("altında", "below")
        };
        Some(match self.det {
            "breakout.range" => {
                let l = self.num(self.lead, "level", Cat::Price)?;
                bi(
                    format!("Kapanış {} {ts} kaldıkça geçerli.", l.w().gen()),
                    format!("It holds while the close stays {es} {}.", l.en),
                )
            }
            "extreme.rsi" => {
                let l = self.rsi_line()?;
                bi(
                    format!("RSI {} {ts} kaldıkça geçerli.", l.w().gen()),
                    format!("It holds while RSI stays {es} {}.", l.en),
                )
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                bi(
                    format!("Kapanış {} ortalamanın {ts} kaldıkça geçerli.", a.tr),
                    format!("It holds while the close stays {es} the {} average.", a.en),
                )
            }
            "squeeze.bandwidth" => bi(
                "Bant yeniden açılana kadar geçerli.",
                "It holds until the band widens again.",
            ),
            _ => return None,
        })
    }

    fn due(&self) -> Bi {
        if self.red {
            return bi(
                "sonuç önümüzdeki günlerde belli olur",
                "the result is due in the coming days",
            );
        }
        let r = self.span32(self.remaining()).trading();
        bi(
            format!("sonuç en geç {} içinde belli olur", r.tr),
            format!("the result is due within {}", r.en),
        )
    }

    fn d_holds(&self) -> Bi {
        let due = self.due();
        let side_up = up(self.dir);
        let plain = || {
            bi(
                format!("{}.", capitalize(&due.tr)),
                format!("{}.", cap_en(&due.en)),
            )
        };
        match self.det {
            "breakout.range" => {
                let Some(l) = self.num(self.lead, "level", Cat::Price) else {
                    return plain();
                };
                let (ts, es) = if side_up {
                    ("üstünde", "above")
                } else {
                    ("altında", "below")
                };
                bi(
                    format!("Kapanış {} {ts} kaldıkça geçerli, {}.", l.w().gen(), due.tr),
                    format!(
                        "It holds while the close stays {es} {}, and {}.",
                        l.en, due.en
                    ),
                )
            }
            "extreme.rsi" => {
                let Some(l) = self.rsi_line() else {
                    return plain();
                };
                let (ts, es) = if side_up {
                    ("üstünde", "above")
                } else {
                    ("altında", "below")
                };
                let v0 = self.num(self.lead, "value", Cat::Osc).map(|v| v.tr);
                match self
                    .current(Cat::Osc)
                    .filter(|c| Some(&c.tr) != v0.as_ref())
                {
                    Some(c) => bi(
                        format!(
                            "RSI {} {ts} kaldıkça geçerli, şu an {} ve {}.",
                            l.w().gen(),
                            c.tr,
                            due.tr
                        ),
                        format!(
                            "It holds while RSI stays {es} {}, now at {}, and {}.",
                            l.en, c.en, due.en
                        ),
                    ),
                    None => bi(
                        format!("RSI {} {ts} kaldıkça geçerli, {}.", l.w().gen(), due.tr),
                        format!("It holds while RSI stays {es} {}, and {}.", l.en, due.en),
                    ),
                }
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                let (ts, es) = if side_up {
                    ("üstünde", "above")
                } else {
                    ("altında", "below")
                };
                match self.current(Cat::Price) {
                    Some(c) => bi(
                        format!("Kapanış {} ortalamanın {ts} kaldıkça geçerli, son kapanış {} ve {}.", a.tr, c.tr, due.tr),
                        format!("It holds while the close stays {es} the {} average, last at {}, and {}.", a.en, c.en, due.en),
                    ),
                    None => bi(
                        format!("Kapanış {} ortalamanın {ts} kaldıkça geçerli, {}.", a.tr, due.tr),
                        format!("It holds while the close stays {es} the {} average, and {}.", a.en, due.en),
                    ),
                }
            }
            "squeeze.bandwidth" => bi(
                format!("Bant yeniden açılana kadar geçerli, {}.", due.tr),
                format!("It holds until the band widens again, and {}.", due.en),
            ),
            _ => plain(),
        }
    }

    fn d_no_longer(&self) -> Bi {
        let side_up = up(self.dir);
        if self.latest_is_exit() {
            if let Some(c) = self.clause(self.latest, false) {
                let t = self.ago(self.latest.t);
                return bi(
                    format!("{} {}.", t.cap().tr, c.tr),
                    format!("{} {}.", cap_en(&c.en), t.en),
                );
            }
        }
        let (ts, es) = if side_up {
            ("altına", "below")
        } else {
            ("üstüne", "above")
        };
        match self.det {
            "breakout.range" => {
                let l = self.num(self.lead, "level", Cat::Price);
                match (l, self.current(Cat::Price)) {
                    (Some(l), Some(c)) => bi(
                        format!("Son kapanış {} ile {} {ts} döndü.", c.tr, l.w().gen()),
                        format!("The latest close, {}, is back {es} {}.", c.en, l.en),
                    ),
                    (Some(l), None) => bi(
                        format!("Son kapanış {} {ts} döndü.", l.w().gen()),
                        format!("The latest close is back {es} {}.", l.en),
                    ),
                    _ => bi(
                        "Son kapanış çizginin öbür yanına döndü.",
                        "The latest close is back across the line.",
                    ),
                }
            }
            "extreme.rsi" => {
                let l = self.rsi_line();
                match (l, self.current(Cat::Osc)) {
                    (Some(l), Some(c)) => bi(
                        format!("RSI şimdi {} ile {} {ts} döndü.", c.tr, l.w().gen()),
                        format!("RSI is now back {es} {}, at {}.", l.en, c.en),
                    ),
                    (Some(l), None) => bi(
                        format!("RSI {} {ts} döndü.", l.w().gen()),
                        format!("RSI is back {es} {}.", l.en),
                    ),
                    _ => bi(
                        "RSI çizginin öbür yanına döndü.",
                        "RSI is back across its line.",
                    ),
                }
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                match self.current(Cat::Price) {
                    Some(c) => bi(
                        format!("Son kapanış {} ile {} ortalamanın {ts} geçti.", c.tr, a.tr),
                        format!(
                            "The latest close, {}, crossed back {es} the {} average.",
                            c.en, a.en
                        ),
                    ),
                    None => bi(
                        format!("Son kapanış {} ortalamanın {ts} geçti.", a.tr),
                        format!("The latest close crossed back {es} the {} average.", a.en),
                    ),
                }
            }
            "squeeze.bandwidth" => bi("Bant yeniden genişledi.", "The band widened again."),
            _ => bi(
                "Dayandığı koşul bozuldu.",
                "The condition it rests on broke.",
            ),
        }
    }

    fn d_closed(&self) -> Bi {
        let mv = self.moved();
        let mc = mv.cap();
        let neutral = self.dir == Direction::Neutral;
        let (tr_past, en_past) = match self.dir {
            Direction::Up => ("yükseldi", "rose"),
            Direction::Down => ("düştü", "fell"),
            Direction::Neutral => ("bir yöne yeterince açıldı", "moved far enough one way"),
        };
        let h = self.span32(self.story.horizon_bars).trading();
        let (h_tr, h_en) = if self.red {
            ("beklenen süre".to_owned(), "the set time".to_owned())
        } else {
            (h.tr.clone(), h.en.clone())
        };
        match self.parts.ending {
            Some(Ending::Realized) if neutral => bi(
                format!("{} fiyat {tr_past} ve gerçekleşti.", mc.tr),
                format!("The price {en_past} {}, so it played out.", mv.en),
            ),
            Some(Ending::Realized) => bi(
                format!("{} fiyat yeterince {tr_past} ve gerçekleşti.", mc.tr),
                format!(
                    "The price {en_past} far enough {}, so it played out.",
                    mv.en
                ),
            ),
            Some(Ending::Failed) if neutral => bi(
                format!(
                    "Fiyat {h_tr} içinde yeterince hareket etmedi, {} tutmadı.",
                    mv.tr
                ),
                format!(
                    "The price did not move far enough within {h_en}, so it did not play out {}.",
                    mv.en
                ),
            ),
            Some(Ending::Failed) => bi(
                format!("{} fiyat ters yöne yeterince gitti ve tutmadı.", mc.tr),
                format!(
                    "The price went far enough the other way {}, so it did not play out.",
                    mv.en
                ),
            ),
            Some(Ending::Expired) => bi(
                format!(
                    "{} içinde yeterli hareket olmadı, {} süresi doldu.",
                    capitalize(&h_tr),
                    mv.tr
                ),
                format!(
                    "No large enough move came within {h_en}, so it ran out of time {}.",
                    mv.en
                ),
            ),
            Some(Ending::Retracted) => bi(
                format!(
                    "Dayandığı veri {} düzeltildi, bu yüzden geri çekildi ve sayılmadı.",
                    mv.tr
                ),
                format!(
                    "The data behind it was corrected {}, so it was withdrawn and not counted.",
                    mv.en
                ),
            ),
            None => bi(
                format!("{} kapandı.", mc.tr),
                format!("It closed {}.", mv.en),
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Ledger
    // -----------------------------------------------------------------------

    fn ledger(&self) -> Bi {
        let l = self.story.ledger_at_birth;
        let (tr_past, en_past, tr_neg, tr_if, en_if) = match self.dir {
            Direction::Up => ("yükseldi", "rose", "yükselmedi", "yükselirse", "rises"),
            Direction::Down => ("düştü", "fell", "düşmedi", "düşerse", "falls"),
            Direction::Neutral => (
                "bir yöne kırıldı",
                "broke out",
                "bir yöne kırılmadı",
                "bir yöne kırılırsa",
                "breaks out",
            ),
        };
        let ok = |b: &Bi| b.fits(MAX_LEDGER);
        if self.red {
            let b = if l.n == 0 {
                bi(
                    format!("Henüz benzeri yok, fiyat {tr_if} tutmuş sayılacak."),
                    format!("No past cases yet, it counts if the price {en_if}."),
                )
            } else if l.n < LEDGER_MIN_N {
                bi(
                    "Az örnek: geçmişte yalnız birkaç benzeri var.",
                    "Few past cases: only a handful so far.",
                )
            } else {
                let (tr_q, en_q) = if 10 * l.realized >= 6 * l.n {
                    ("çoğunda", "most")
                } else if 10 * l.realized >= 4 * l.n {
                    ("yaklaşık yarısında", "about half of the")
                } else {
                    ("azında", "few")
                };
                bi(
                    format!("Geçmişteki benzerlerinin {tr_q} fiyat sonradan {tr_past}."),
                    format!("In {en_q} past cases the price {en_past} afterwards."),
                )
            };
            return first_fit(vec![b, bi("Benzerleri var.", "Past cases exist.")], ok);
        }
        let h = self.span32(self.story.horizon_bars).trading();
        if l.n == 0 {
            return first_fit(
                vec![
                    bi(
                        format!(
                            "Henüz benzeri yok, fiyat {} içinde {tr_if} tutmuş sayılacak.",
                            h.tr
                        ),
                        format!(
                            "No past cases yet, it counts if the price {en_if} within {}.",
                            h.en
                        ),
                    ),
                    bi(
                        format!(
                            "Henüz benzeri yok, {} içinde {tr_if} tutmuş sayılacak.",
                            h.tr
                        ),
                        format!(
                            "No past cases yet, it counts if it {en_if} within {}.",
                            h.en
                        ),
                    ),
                    bi(
                        format!("Henüz benzeri yok, {} içinde {tr_if} tutacak.", h.tr),
                        format!("No past cases yet; counts if it {en_if} within {}.", h.en),
                    ),
                    bi("Henüz benzeri yok.", "No past cases yet."),
                ],
                ok,
            );
        }
        let n = Word::number(&l.n.to_string());
        let r = Word::number(&l.realized.to_string());
        let (tr_hit, tr_verb) = if l.realized == 0 {
            ("hiçbirinde (0)".to_owned(), tr_neg)
        } else {
            (r.poss3_loc(), tr_past)
        };
        let thin = l.n < LEDGER_MIN_N;
        let (tr_head, tr_head_short) = if thin {
            ("Az örnek: ", "Az örnek: ")
        } else {
            ("Geçmişte ", "")
        };
        let en_full = if thin {
            format!(
                "Few past cases: the price {en_past} within {} in {} of {}.",
                h.en, l.realized, l.n
            )
        } else {
            format!(
                "The price {en_past} within {} in {} of {} past cases.",
                h.en, l.realized, l.n
            )
        };
        let en_short = if thin {
            format!(
                "Few past cases: it {en_past} within {} in {} of {}.",
                h.en, l.realized, l.n
            )
        } else {
            format!(
                "It {en_past} within {} in {} of {} past cases.",
                h.en, l.realized, l.n
            )
        };
        first_fit(
            vec![
                bi(
                    format!(
                        "{tr_head}{} benzerinin {tr_hit} fiyat {} içinde {tr_verb}.",
                        n.written, h.tr
                    ),
                    en_full,
                ),
                bi(
                    format!(
                        "{tr_head}{} benzerinin {tr_hit} {} içinde {tr_verb}.",
                        n.written, h.tr
                    ),
                    en_short.clone(),
                ),
                bi(
                    format!(
                        "{tr_head_short}{} benzerinin {tr_hit} {} içinde {tr_verb}.",
                        n.written, h.tr
                    ),
                    en_short.clone(),
                ),
                bi(
                    format!(
                        "{tr_head_short}{} benzerinde {} içinde {} kez {tr_past}.",
                        n.written, h.tr, l.realized
                    ),
                    en_short,
                ),
            ],
            ok,
        )
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

#[cfg(test)]
mod tests;
