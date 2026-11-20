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

// ---------------------------------------------------------------------------
// Numbers and words
// ---------------------------------------------------------------------------

fn n(v: f64, cat: Cat, lang: Lang) -> String {
    fmt(v, cat, lang)
}

pub(super) fn words_tr(x: u32) -> String {
    const ONES: [&str; 10] = [
        "", "bir", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz",
    ];
    const TENS: [&str; 10] = [
        "", "on", "yirmi", "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan",
    ];
    if x == 0 {
        return "sıfır".into();
    }
    if x >= 1000 {
        return x.to_string();
    }
    let (h, t, o) = (
        (x / 100) as usize,
        ((x / 10) % 10) as usize,
        (x % 10) as usize,
    );
    let mut out: Vec<&str> = Vec::new();
    if h == 1 {
        out.push("yüz");
    } else if h > 1 {
        out.push(ONES[h]);
        out.push("yüz");
    }
    if t > 0 {
        out.push(TENS[t]);
    }
    if o > 0 {
        out.push(ONES[o]);
    }
    out.join(" ")
}

pub(super) fn words_en(x: u32) -> String {
    const SMALL: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    if x >= 1000 {
        return x.to_string();
    }
    if x < 20 {
        return SMALL[x as usize].into();
    }
    let (h, rest) = (x / 100, x % 100);
    let mut out: Vec<String> = Vec::new();
    if h > 0 {
        out.push(format!("{} hundred", SMALL[h as usize]));
    }
    if rest > 0 {
        if rest < 20 {
            out.push(SMALL[rest as usize].into());
        } else if rest % 10 == 0 {
            out.push(TENS[(rest / 10) as usize].into());
        } else {
            out.push(format!(
                "{}-{}",
                TENS[(rest / 10) as usize],
                SMALL[(rest % 10) as usize]
            ));
        }
    }
    out.join(" ")
}

/// A detector parameter (public rule, not licensed data): digits, or words for a
/// redacted reader.
fn param(x: usize, lang: Lang, red: bool) -> String {
    let x = u32::try_from(x).unwrap_or(u32::MAX);
    match (red, lang) {
        (false, _) => x.to_string(),
        (true, Lang::Tr) => words_tr(x),
        (true, Lang::En) => words_en(x),
    }
}

/// A multiplier rule ("2,5 katı", "two and a half times"): digits, or words when
/// redacted.
fn mult(x: f64, lang: Lang, red: bool) -> String {
    if !red {
        return n(x, Cat::Ratio, lang);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = x.trunc().max(0.0) as u32;
    let half = (x - x.trunc() - 0.5).abs() < 1e-9;
    match lang {
        Lang::Tr if half => format!("{} buçuk", words_tr(whole)),
        Lang::En if half => format!("{} and a half", words_en(whole)),
        Lang::Tr => words_tr(whole),
        Lang::En => words_en(whole),
    }
}

/// Percentage distance of `x` from `thr`, with the sign dropped (the sentence says which
/// side): TR `%0,9`, EN `0.9%`; below 0,05 % "less than 0,1 %".
fn pct(x: f64, thr: f64, lang: Lang) -> String {
    let p = if thr.abs() > 0.0 {
        ((x - thr) / thr * 100.0).abs()
    } else {
        0.0
    };
    let s = n(p, Cat::Osc, lang);
    if s == "0" {
        return match lang {
            Lang::Tr => "%0,1'den az".into(),
            Lang::En => "less than 0.1%".into(),
        };
    }
    match lang {
        Lang::Tr => format!("%{s}"),
        Lang::En => format!("{s}%"),
    }
}

/// TR ablative of a number word ("68,2'den", "70'ten").
fn abl(w: &str) -> String {
    format!("{}n", Word::number(w).loc())
}

fn dat(w: &str) -> String {
    Word::number(w).dat()
}

/// "20 günün" / "20 saatin" (TR genitive of a counted period).
fn tr_gen(k: &str, u: Unit) -> String {
    let noun = match u {
        Unit::Day => "günün",
        Unit::Hour => "saatin",
        Unit::Week => "haftanın",
        Unit::Month => "ayın",
        Unit::Slice(_) => "dilimin",
    };
    format!("{k} {noun}")
}

/// "20 günlük" / "20 saatlik".
fn tr_adj(k: &str, u: Unit) -> String {
    format!("{k} {}", u.words().tr_adj)
}

/// "20-day" / "20-hour" (redacted: "twenty-day").
fn en_adj(k: &str, u: Unit) -> String {
    format!("{k}-{}", u.words().en)
}

/// The horizon span: TR "10 işlem günü", EN "10 trading days".
fn span(k: u32, u: Unit, lang: Lang, red: bool) -> String {
    let ks = match (red, lang) {
        (false, _) => k.to_string(),
        (true, Lang::Tr) => words_tr(k),
        (true, Lang::En) => words_en(k),
    };
    match (lang, u) {
        (Lang::Tr, Unit::Day) => format!("{ks} işlem günü"),
        (Lang::Tr, Unit::Hour) => format!("{ks} işlem saati"),
        (Lang::Tr, Unit::Week) => format!("{ks} hafta"),
        (Lang::Tr, Unit::Month) => format!("{ks} ay"),
        (Lang::Tr, Unit::Slice(m)) => format!("{ks} adet {m} dakikalık dilim"),
        (Lang::En, Unit::Day) => format!("{ks} trading day{}", plural_s(k)),
        (Lang::En, Unit::Hour) => format!("{ks} trading hour{}", plural_s(k)),
        (Lang::En, Unit::Week) => format!("{ks} week{}", plural_s(k)),
        (Lang::En, Unit::Month) => format!("{ks} month{}", plural_s(k)),
        (Lang::En, Unit::Slice(m)) => format!("{ks} {m}-minute period{}", plural_s(k)),
    }
}

/// TR locative of the span: "10 işlem gününde".
fn span_loc_tr(k: u32, u: Unit, red: bool) -> String {
    let ks = if red { words_tr(k) } else { k.to_string() };
    match u {
        Unit::Day => format!("{ks} işlem gününde"),
        Unit::Hour => format!("{ks} işlem saatinde"),
        Unit::Week => format!("{ks} haftada"),
        Unit::Month => format!("{ks} ayda"),
        Unit::Slice(m) => format!("{ks} adet {m} dakikalık dilimde"),
    }
}

fn plural_s(k: u32) -> &'static str {
    if k == 1 {
        ""
    } else {
        "s"
    }
}

/// "O gün" / "That day".
fn that(u: Unit, lang: Lang) -> &'static str {
    match (lang, u) {
        (Lang::Tr, Unit::Day) => "O gün",
        (Lang::Tr, Unit::Hour) => "O saat",
        (Lang::Tr, Unit::Week) => "O hafta",
        (Lang::Tr, Unit::Month) => "O ay",
        (Lang::Tr, Unit::Slice(_)) => "O dilimde",
        (Lang::En, Unit::Day) => "That day",
        (Lang::En, Unit::Hour) => "That hour",
        (Lang::En, Unit::Week) => "That week",
        (Lang::En, Unit::Month) => "That month",
        (Lang::En, Unit::Slice(_)) => "In that period",
    }
}

// ---------------------------------------------------------------------------
// Detector parameters (the detectors' own constants)
// ---------------------------------------------------------------------------

use qp_facts::{BandSqueeze, InstitutionalFlow, MaStack, RangeBreakout, VolumeSurge};

// ---------------------------------------------------------------------------
// Headline: "Ne değişti?" + when + validity
// ---------------------------------------------------------------------------

const HEAD_LEVELS: usize = 6;

/// Levels, richest first: (core tier, long validity, with when). The long validity
/// ("hâlâ geçerli") is kept before the short one ("sürüyor"), which can read as "the
/// buying goes on"; dropping "when" is the last resort (never needed on the harness
/// situations).
const HEAD_PLAN: [(u8, bool, bool); HEAD_LEVELS] = [
    (0, true, true),
    (1, true, true),
    (2, true, true),
    (3, true, true),
    (3, false, true),
    (3, false, false),
];

fn headline(f: &Frame<'_>, lang: Lang, level: usize) -> String {
    let (tier, long_v, with_when) = HEAD_PLAN[level.min(HEAD_LEVELS - 1)];
    let core = core(f, lang, tier);
    if f.validity == Validity::Closed {
        return format!("{core} · {}", closed_chip(f, lang, with_when));
    }
    let v = validity_chip(f, lang, long_v);
    if with_when {
        format!("{core} · {}, {v}", f.when(lang))
    } else {
        format!("{core} · {v}")
    }
}

/// How and when a closed claim ended: TR "bugün gerçekleşti", EN "played out today"
/// (the last move is the closing transition).
fn closed_chip(f: &Frame<'_>, lang: Lang, with_when: bool) -> String {
    let e = validity_chip(f, lang, false);
    if !with_when {
        return e;
    }
    let mv = f.moved(lang);
    match lang {
        Lang::Tr => format!("{mv} {e}"),
        Lang::En => format!("{e} {mv}"),
    }
}

fn validity_chip(f: &Frame<'_>, lang: Lang, long: bool) -> String {
    let s = match (f.validity, f.ending, lang, long) {
        (Validity::Valid, _, Lang::Tr, true) => "hâlâ geçerli",
        (Validity::Valid, _, Lang::Tr, false) => "sürüyor",
        (Validity::Valid, _, Lang::En, _) => "still holds",
        (Validity::NoLonger, _, Lang::Tr, true) => "artık geçerli değil",
        (Validity::NoLonger, _, Lang::Tr, false) => "artık sürmüyor",
        (Validity::NoLonger, _, Lang::En, _) => "no longer holds",
        (Validity::Closed, Some(Ending::Realized), Lang::Tr, true) => "kapandı: gerçekleşti",
        (Validity::Closed, Some(Ending::Realized), Lang::Tr, false) => "gerçekleşti",
        (Validity::Closed, Some(Ending::Realized), Lang::En, true) => "closed: played out",
        (Validity::Closed, Some(Ending::Realized), Lang::En, false) => "played out",
        (Validity::Closed, Some(Ending::Failed), Lang::Tr, true) => "kapandı: tutmadı",
        (Validity::Closed, Some(Ending::Failed), Lang::Tr, false) => "tutmadı",
        (Validity::Closed, Some(Ending::Failed), Lang::En, true) => "closed: did not play out",
        (Validity::Closed, Some(Ending::Failed), Lang::En, false) => "did not play out",
        (Validity::Closed, Some(Ending::Expired), Lang::Tr, true) => "kapandı: süresi doldu",
        (Validity::Closed, Some(Ending::Expired), Lang::Tr, false) => "süresi doldu",
        (Validity::Closed, Some(Ending::Expired), Lang::En, true) => "closed: ran out of time",
        (Validity::Closed, Some(Ending::Expired), Lang::En, false) => "ran out of time",
        (Validity::Closed, Some(Ending::Retracted), Lang::Tr, _) => "geri çekildi",
        (Validity::Closed, Some(Ending::Retracted), Lang::En, _) => "withdrawn",
        (Validity::Closed, None, Lang::Tr, _) => "kapandı",
        (Validity::Closed, None, Lang::En, _) => "closed",
    };
    s.to_owned()
}

/// The plain change with the threshold by name (and value for an entitled reader).
/// `tier` 0 = full, 1 = short, 2 = tiny, 3 = tiniest (where it differs from tiny); the
/// numbers of tiers 1–3 are the same in both languages.
#[allow(clippy::too_many_lines)]
fn core(f: &Frame<'_>, lang: Lang, tier: u8) -> String {
    let i = f.inst;
    let u = f.unit;
    let red = f.red;
    let up = f.up();
    match f.det {
        "breakout.range" => {
            let k = param(RangeBreakout::WINDOW, lang, red);
            let g = tr_gen(&k, u);
            let a = en_adj(&k, u);
            let lvl = f
                .v("level")
                .map(|x| format!(" ({})", n(x, Cat::Price, lang)))
                .unwrap_or_default();
            match (lang, up, tier) {
                (Lang::Tr, true, 0) => format!("{i} son {g} en yüksek fiyatını{lvl} aştı"),
                (Lang::Tr, true, _) => format!("{i} {g} zirvesini{lvl} aştı"),
                (Lang::Tr, false, 0) => format!("{i} son {g} en düşük fiyatının{lvl} altına indi"),
                (Lang::Tr, false, 1) => format!("{i} {g} en düşüğünün{lvl} altına indi"),
                (Lang::Tr, false, _) => format!("{i} {g} en düşüğünü{lvl} kırdı"),
                (Lang::En, true, 0) => format!("{i} rose above its {a} high{lvl}"),
                (Lang::En, true, _) => format!("{i} broke its {a} high{lvl}"),
                (Lang::En, false, 0) => format!("{i} fell below its {a} low{lvl}"),
                (Lang::En, false, _) => format!("{i} broke its {a} low{lvl}"),
            }
        }
        "extreme.rsi" => {
            let lvl =
                f.ev.get("level")
                    .copied()
                    .unwrap_or(if up { 70.0 } else { 30.0 });
            let l = if red {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let w = lvl.round().max(0.0) as u32;
                match lang {
                    Lang::Tr => words_tr(w),
                    Lang::En => words_en(w),
                }
            } else {
                n(lvl, Cat::Osc, lang)
            };
            match (lang, up, tier) {
                (Lang::Tr, true, 0) => {
                    format!("{i} kısa sürede çok yükseldi: RSI {} aştı", tr_acc(&l, red))
                }
                (Lang::Tr, true, _) => {
                    format!("{i} hızla yükseldi: RSI {} aştı", tr_acc(&l, red))
                }
                (Lang::Tr, false, 0) => format!(
                    "{i} kısa sürede çok düştü: RSI {} altına indi",
                    tr_gen_w(&l, red)
                ),
                (Lang::Tr, false, _) => {
                    format!("{i} hızla düştü: RSI {} altına indi", tr_gen_w(&l, red))
                }
                (Lang::En, true, 0) => {
                    format!("{i} rose a lot in a short time: RSI went above {l}")
                }
                (Lang::En, true, _) => format!("{i} rose fast: RSI went above {l}"),
                (Lang::En, false, 0) => {
                    format!("{i} fell a lot in a short time: RSI went below {l}")
                }
                (Lang::En, false, _) => format!("{i} fell fast: RSI went below {l}"),
            }
        }
        "reversal.rsi_divergence" => match (lang, up, tier) {
            (Lang::Tr, true, 0) => {
                format!("{i}: düşüş yukarı dönebilir, fiyat yeni dip yaptı ama RSI yapmadı")
            }
            (Lang::Tr, true, 1) => {
                format!("{i} yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı")
            }
            (Lang::Tr, true, _) => format!("{i} yukarı dönebilir: RSI yeni dip yapmadı"),
            (Lang::Tr, false, 0) => {
                format!("{i}: yükseliş aşağı dönebilir, fiyat yeni tepe yaptı ama RSI yapmadı")
            }
            (Lang::Tr, false, 1) => {
                format!("{i} aşağı dönebilir: fiyat yeni tepe yaptı, RSI yapmadı")
            }
            (Lang::Tr, false, _) => format!("{i} aşağı dönebilir: RSI yeni tepe yapmadı"),
            (Lang::En, true, 0) => {
                format!("{i}'s fall could turn up: a new price low that RSI did not match")
            }
            (Lang::En, true, 1) => format!("{i} could turn up: new price low, not matched by RSI"),
            (Lang::En, true, _) => format!("{i} could turn up: RSI made no new low"),
            (Lang::En, false, 0) => {
                format!("{i}'s rise could turn down: a new price high that RSI did not match")
            }
            (Lang::En, false, 1) => {
                format!("{i} could turn down: new price high, not matched by RSI")
            }
            (Lang::En, false, _) => format!("{i} could turn down: RSI made no new high"),
        },
        "continuation.ma_stack" => {
            // (R5/m17-int, S1) The event verb first and plain: the trend goes on
            // ("düşüşünü sürdürdü"), the averages after it as where the price stands —
            // never "altına indi", which readers took for a threshold crossing.
            let a = param(MaStack::FAST, lang, red);
            let b = param(MaStack::SLOW, lang, red);
            match (lang, up, tier) {
                (Lang::Tr, true, 0) => format!(
                    "{i} yükselişini sürdürdü: fiyat {a} ve {} ortalamaların üstünde",
                    tr_adj(&b, u)
                ),
                (Lang::Tr, true, 1) => {
                    format!(
                        "{i} yükselişini sürdürdü, {} ortalamanın üstünde",
                        tr_adj(&a, u)
                    )
                }
                (Lang::Tr, true, 2) => {
                    format!("{i} yükselişte, {} ortalamanın üstünde", tr_adj(&a, u))
                }
                (Lang::Tr, true, _) => {
                    format!("{i} yükselişte: {} ortalama üstü", tr_adj(&a, u))
                }
                (Lang::Tr, false, 0) => format!(
                    "{i} düşüşünü sürdürdü: fiyat {a} ve {} ortalamaların altında",
                    tr_adj(&b, u)
                ),
                (Lang::Tr, false, 1) => {
                    format!(
                        "{i} düşüşünü sürdürdü, {} ortalamanın altında",
                        tr_adj(&a, u)
                    )
                }
                (Lang::Tr, false, 2) => {
                    format!("{i} düşüşte, {} ortalamanın altında", tr_adj(&a, u))
                }
                (Lang::Tr, false, _) => {
                    format!("{i} düşüşte: {} ortalama altı", tr_adj(&a, u))
                }
                (Lang::En, true, 0) => format!(
                    "{i} kept rising: price stayed above its {a}- and {} averages",
                    en_adj(&b, u)
                ),
                (Lang::En, true, 1) => {
                    format!("{i} kept rising, above its {} average", en_adj(&a, u))
                }
                (Lang::En, true, 2) => {
                    format!("{i} kept rising above its {} average", en_adj(&a, u))
                }
                (Lang::En, true, _) => {
                    format!("{i} kept rising above {} average", en_adj(&a, u))
                }
                (Lang::En, false, 0) => format!(
                    "{i} kept falling: price stayed below its {a}- and {} averages",
                    en_adj(&b, u)
                ),
                (Lang::En, false, 1) => {
                    format!("{i} kept falling, below its {} average", en_adj(&a, u))
                }
                (Lang::En, false, 2) => {
                    format!("{i} kept falling below its {} average", en_adj(&a, u))
                }
                (Lang::En, false, _) => {
                    format!("{i} kept falling below {} average", en_adj(&a, u))
                }
            }
        }
        "squeeze.bandwidth" => {
            let a = param(BandSqueeze::PERIOD, lang, red);
            let b = param(BandSqueeze::LOOKBACK, lang, red);
            let w = u.words();
            match (lang, tier) {
                (Lang::Tr, 0) => format!(
                    "{i} dar bir aralığa sıkıştı: {} bant son {} en darı",
                    tr_adj(&a, u),
                    tr_gen(&b, u)
                ),
                (Lang::Tr, 1) => {
                    format!(
                        "{i} sıkıştı: {} bant {} en darı",
                        tr_adj(&a, u),
                        tr_gen(&b, u)
                    )
                }
                (Lang::Tr, _) => {
                    format!("{i}: {} bant {} en darı", tr_adj(&a, u), tr_gen(&b, u))
                }
                (Lang::En, 0) => format!(
                    "{i} narrowed to a tight range: {} band narrowest in {b} {}",
                    en_adj(&a, u),
                    w.en_pl
                ),
                (Lang::En, 1) => format!(
                    "{i} went quiet: {} band narrowest in {b} {}",
                    en_adj(&a, u),
                    w.en_pl
                ),
                (Lang::En, _) => format!("{i}: {} band at a {} low", en_adj(&a, u), en_adj(&b, u)),
            }
        }
        "flow.volume_surge" => {
            let a = param(VolumeSurge::WINDOW, lang, red);
            let m = mult(VolumeSurge::MULT, lang, red);
            let x = times_short(&m, red);
            let (tr_px, en_px, en_tiny) = match f.dir {
                Direction::Up => ("fiyat yükseldi", "as the price rose", "price rose"),
                Direction::Down => ("fiyat düştü", "as the price fell", "price fell"),
                Direction::Neutral => ("fiyat yerinde kaldı", "as the price held", "price held"),
            };
            match (lang, tier) {
                (Lang::Tr, 0) => format!(
                    "{i}: hacim {} ortalamanın {m} katını aştı, {tr_px}",
                    tr_adj(&a, u)
                ),
                (Lang::Tr, 1) => format!("{i}: hacim ortalamanın {m} katını aştı, {tr_px}"),
                (Lang::Tr, _) => format!("{i}: hacim ortalamayı {m} kat aştı, {tr_px}"),
                (Lang::En, 0) => format!(
                    "{i}: volume topped {m} times its {} average {en_px}",
                    en_adj(&a, u)
                ),
                (Lang::En, 1) => format!("{i}: volume topped {m} times its average {en_px}"),
                (Lang::En, _) => format!("{i}: volume over {x} average, {en_tiny}"),
            }
        }
        "flow.institutional" => {
            let a = param(InstitutionalFlow::WINDOW, lang, red);
            let m = mult(InstitutionalFlow::MULT, lang, red);
            let x = times_short(&m, red);
            let (tr_s, en_s) = if up {
                ("alım", "buying")
            } else {
                ("satım", "selling")
            };
            match (lang, tier) {
                (Lang::Tr, 0) => format!(
                    "{i}: kurumsal net {tr_s} {} ortalamanın {m} katını aştı",
                    tr_adj(&a, u)
                ),
                (Lang::Tr, 1) => format!("{i}: kurumsal net {tr_s} ortalamanın {m} katını aştı"),
                (Lang::Tr, 3) => format!(
                    "{i}: kurumlar olağanın {m} katı {}",
                    if up { "aldı" } else { "sattı" }
                ),
                (Lang::Tr, _) => format!("{i}: kurumsal {tr_s} ortalamayı {m} kat aştı"),
                (Lang::En, 0) => format!(
                    "{i}: institutional net {en_s} topped {m} times its {} average",
                    en_adj(&a, u)
                ),
                (Lang::En, 1) => {
                    format!("{i}: institutional net {en_s} topped {m} times its average")
                }
                (Lang::En, _) if red => format!(
                    "{i}: institutions {} {x}",
                    if up { "bought" } else { "sold" }
                ),
                (Lang::En, _) => format!(
                    "{i}: institutions {} over {x} average",
                    if up { "bought" } else { "sold" }
                ),
            }
        }
        _ => match lang {
            Lang::Tr => format!("{i}: {}", crate::realize::tr::noun_state(f.kind, f.dir)),
            Lang::En => format!("{i}: {}", crate::realize::en::noun_bare(f.kind, f.dir)),
        },
    }
}

/// EN compact multiple: "2.5x"; for a redacted reader words ("twice the usual",
/// "two and a half times its").
fn times_short(m: &str, red: bool) -> String {
    if red && m == "two" {
        "twice the usual".to_owned()
    } else if red {
        format!("{m} times its")
    } else {
        format!("{m}x")
    }
}

/// TR accusative of a number or number word ("70'i", "yetmişi").
fn tr_acc(w: &str, red: bool) -> String {
    if red {
        Word::noun(w).acc()
    } else {
        Word::number(w).acc()
    }
}

/// TR genitive of a number or number word ("30'un", "otuzun").
fn tr_gen_w(w: &str, red: bool) -> String {
    if red {
        Word::noun(w).gen()
    } else {
        Word::number(w).gen()
    }
}

// ---------------------------------------------------------------------------
// Detail: "Neden?" + now + what changed since
// ---------------------------------------------------------------------------

/// Detail compositions, richest first: (now, time left, change sentence).
const DETAIL_MASKS: [(bool, bool, bool); 8] = [
    (true, true, true),
    (true, false, true),
    (false, true, true),
    (false, false, true),
    (true, true, false),
    (true, false, false),
    (false, true, false),
    (false, false, false),
];

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

fn lead_concepts(f: &Frame<'_>) -> Vec<Concept> {
    det_concepts(f.det, f.up())
}

fn det_concepts(det: &str, up: bool) -> Vec<Concept> {
    match det {
        "breakout.range" => vec![Concept::HighLow(up)],
        "extreme.rsi" => vec![Concept::Rsi],
        "reversal.rsi_divergence" => vec![Concept::Rsi, Concept::Swing(up)],
        "continuation.ma_stack" => vec![Concept::Average],
        "squeeze.bandwidth" => vec![Concept::Band],
        "flow.volume_surge" => vec![Concept::Volume],
        "flow.institutional" => vec![Concept::Institutional(up)],
        _ => Vec::new(),
    }
}

/// The numbers behind the claim, each against its reference.
#[allow(clippy::too_many_lines)]
fn why(f: &Frame<'_>, lang: Lang) -> String {
    if f.red {
        return match (f.det, lang) {
            ("flow.institutional", Lang::Tr) => {
                "Tutarlar lisanslı veridir: sunucu hesapladı ve doğruladı, kurumsal akış veri \
                 yetkisiyle görünür."
                    .into()
            }
            ("flow.institutional", Lang::En) => {
                "The amounts are licensed, computed and verified by the server, shown with \
                 institutional flow access."
                    .into()
            }
            (_, Lang::Tr) => {
                "Değerler lisanslı veridir: sunucu hesapladı ve doğruladı, veri yetkisiyle görünür."
                    .into()
            }
            (_, Lang::En) => {
                "The values are licensed, computed and verified by the server, shown with data \
                 access."
                    .into()
            }
        };
    }
    let s = why_numbers(f, lang);
    if f.server {
        // Server-computed claim (K38): say who verified it, inside the same sentence.
        let mark = match lang {
            Lang::Tr => " (sunucu doğruladı)",
            Lang::En => " (verified by the server)",
        };
        if let Some(body) = s.strip_suffix('.') {
            return format!("{body}{mark}.");
        }
    }
    s
}

#[allow(clippy::too_many_lines)]
fn why_numbers(f: &Frame<'_>, lang: Lang) -> String {
    let u = f.unit;
    let up = f.up();
    let p = |x: f64| n(x, Cat::Price, lang);
    let o = |x: f64| n(x, Cat::Osc, lang);
    match f.det {
        "breakout.range" => match (f.v("close"), f.v("level")) {
            (Some(c), Some(l)) => match lang {
                Lang::Tr => format!(
                    "{} kapanış {} oldu, eşiğin {} {}.",
                    anchor(f, lang),
                    p(c),
                    pct(c, l, lang),
                    if up { "üstünde" } else { "altında" }
                ),
                Lang::En => format!(
                    "{} it closed at {}, {} {}.",
                    anchor(f, lang),
                    p(c),
                    pct(c, l, lang),
                    if up {
                        "above the high"
                    } else {
                        "below the low"
                    }
                ),
            },
            _ => generic_why(lang),
        },
        "extreme.rsi" => match (f.v("prev"), f.v("value")) {
            (Some(a), Some(b)) => match lang {
                Lang::Tr => format!(
                    "RSI {} {} {} {}.",
                    if f.validity == Validity::Closed {
                        f.when(lang)
                    } else {
                        that(u, lang).to_lowercase()
                    },
                    abl(&o(a)),
                    dat(&o(b)),
                    if up { "çıktı" } else { "indi" }
                ),
                Lang::En => format!("{} RSI went from {} to {}.", anchor(f, lang), o(a), o(b)),
            },
            _ => generic_why(lang),
        },
        "reversal.rsi_divergence" => {
            match (f.v("p1_price"), f.v("p2_price"), f.v("p1_rsi"), f.v("p2_rsi")) {
                (Some(p1), Some(p2), Some(r1), Some(r2)) => match (lang, up) {
                    (Lang::Tr, true) => format!(
                        "Fiyat önceki dibin ({}) altına, {} indi, RSI ise {} {} çıktı.",
                        p(p1),
                        dat(&p(p2)),
                        abl(&o(r1)),
                        dat(&o(r2))
                    ),
                    (Lang::Tr, false) => format!(
                        "Fiyat önceki tepenin ({}) üstüne, {} çıktı, RSI ise {} {} indi.",
                        p(p1),
                        dat(&p(p2)),
                        abl(&o(r1)),
                        dat(&o(r2))
                    ),
                    (Lang::En, true) => format!(
                        "The price fell below the previous low ({}) to {}, while RSI rose from {} to {}.",
                        p(p1),
                        p(p2),
                        o(r1),
                        o(r2)
                    ),
                    (Lang::En, false) => format!(
                        "The price rose above the previous high ({}) to {}, while RSI fell from {} to {}.",
                        p(p1),
                        p(p2),
                        o(r1),
                        o(r2)
                    ),
                },
                _ => generic_why(lang),
            }
        }
        "continuation.ma_stack" => match (f.v("close"), f.v("sma20"), f.v("sma50")) {
            (Some(c), Some(a), Some(b)) => {
                let fa = param(MaStack::FAST, lang, false);
                let fb = param(MaStack::SLOW, lang, false);
                match lang {
                    Lang::Tr => format!(
                        "Kapanış ({}) {} ortalamanın ({}), o da {} ({}) {}.",
                        p(c),
                        tr_adj(&fa, u),
                        p(a),
                        Word::noun(&tr_adj(&fb, u)).gen(),
                        p(b),
                        if up { "üstündeydi" } else { "altındaydı" }
                    ),
                    Lang::En => format!(
                        "The close ({}) was {} the {} average ({}), itself {} the {} ({}).",
                        p(c),
                        if up { "above" } else { "below" },
                        en_adj(&fa, u),
                        p(a),
                        if up { "above" } else { "below" },
                        en_adj(&fb, u),
                        p(b)
                    ),
                }
            }
            _ => generic_why(lang),
        },
        "squeeze.bandwidth" => match (f.v("lower"), f.v("upper"), f.v("middle")) {
            (Some(lo), Some(hi), Some(mid)) => {
                let w = if mid.abs() > 0.0 {
                    (hi - lo) / mid * 100.0
                } else {
                    0.0
                };
                match lang {
                    Lang::Tr => format!(
                        "Bant {} ile {} arasında (genişliği fiyatın %{} kadarı).",
                        p(lo),
                        p(hi),
                        o(w)
                    ),
                    Lang::En => format!(
                        "The band runs from {} to {} ({}% of the price wide).",
                        p(lo),
                        p(hi),
                        o(w)
                    ),
                }
            }
            _ => generic_why(lang),
        },
        "flow.volume_surge" => match (f.v("volume"), f.v("vol_avg"), f.v("ratio")) {
            (Some(v), Some(a), Some(r)) => {
                let k = param(VolumeSurge::WINDOW, lang, false);
                match lang {
                    Lang::Tr => format!(
                        "Hacim {}, son {} ortalaması {}: {} katı.",
                        n(v, Cat::Count, lang),
                        tr_gen(&k, u),
                        n(a, Cat::Count, lang),
                        o(r)
                    ),
                    Lang::En => format!(
                        "Volume was {} against a {} average of {}: {} times.",
                        n(v, Cat::Count, lang),
                        en_adj(&k, u),
                        n(a, Cat::Count, lang),
                        o(r)
                    ),
                }
            }
            _ => generic_why(lang),
        },
        "flow.institutional" => match (f.v("net"), f.v("avg5")) {
            (Some(net), Some(avg)) => {
                let k = param(InstitutionalFlow::WINDOW, lang, false);
                let r = if avg.abs() > 0.0 {
                    Some(net.abs() / avg.abs())
                } else {
                    None
                };
                let tail = r.map(|r| format!(": {}", o(r))).unwrap_or_default();
                match lang {
                    Lang::Tr => format!(
                        "Net {} {}, son {} ortalaması {}{}{}.",
                        if up { "alım" } else { "satım" },
                        n(net, Cat::Count, lang),
                        tr_gen(&k, u),
                        n(avg, Cat::Count, lang),
                        tail,
                        if r.is_some() { " katı" } else { "" }
                    ),
                    Lang::En => format!(
                        "Net {} was {} against a {} average of {}{}{}.",
                        if up { "buying" } else { "selling" },
                        n(net, Cat::Count, lang),
                        en_adj(&k, u),
                        n(avg, Cat::Count, lang),
                        tail,
                        if r.is_some() { " times" } else { "" }
                    ),
                }
            }
            _ => generic_why(lang),
        },
        _ => generic_why(lang),
    }
}

fn generic_why(lang: Lang) -> String {
    match lang {
        Lang::Tr => "Dayandığı değerler kanıt ekranında.".into(),
        Lang::En => "The values behind it are on the evidence screen.".into(),
    }
}

/// Where things stand now against the threshold (open stories with a current value),
/// without a final period. `None` when there is nothing to say or the side disagrees
/// with the validity (never contradict the face).
fn now(f: &Frame<'_>, lang: Lang) -> Option<String> {
    if f.fresh {
        return None;
    }
    let cur = f.current?;
    let up = f.up();
    let holds = f.validity == Validity::Valid;
    match f.det {
        "breakout.range" => {
            let l = f.v("level")?;
            let above = cur > l;
            if above != (up == holds) {
                return None;
            }
            let side = |a: bool, lang: Lang| match (lang, a) {
                (Lang::Tr, true) => "üstünde",
                (Lang::Tr, false) => "altında",
                (Lang::En, true) => "above",
                (Lang::En, false) => "below",
            };
            Some(match lang {
                Lang::Tr => format!(
                    "şimdi {} (eşiğin {} {})",
                    n(cur, Cat::Price, lang),
                    pct(cur, l, lang),
                    side(above, lang)
                ),
                Lang::En => format!(
                    "now {} ({} {} it)",
                    n(cur, Cat::Price, lang),
                    pct(cur, l, lang),
                    side(above, lang)
                ),
            })
        }
        "extreme.rsi" => {
            let l = f.ev.get("level").copied()?;
            let above = cur > l;
            if above != (up == holds) {
                return None;
            }
            let ls = n(l, Cat::Osc, lang);
            Some(match lang {
                Lang::Tr => format!(
                    "şimdi RSI {} ({} {})",
                    n(cur, Cat::Osc, lang),
                    Word::number(&ls).gen(),
                    if above { "üstünde" } else { "altında" }
                ),
                Lang::En => format!(
                    "RSI is now {} ({} {ls})",
                    n(cur, Cat::Osc, lang),
                    if above { "above" } else { "below" }
                ),
            })
        }
        "continuation.ma_stack" if holds => Some(match lang {
            Lang::Tr => format!("şimdi kapanış {}", n(cur, Cat::Price, lang)),
            Lang::En => format!("the close is now {}", n(cur, Cat::Price, lang)),
        }),
        _ => None,
    }
}

/// How long until the outcome is known, without a final period.
fn remaining(f: &Frame<'_>, lang: Lang) -> Option<String> {
    let r = f.remaining?;
    if r == 0 {
        let w = f.unit.words();
        return Some(match lang {
            Lang::Tr => format!("sonuç bu {} belli olacak", w.tr),
            Lang::En => format!("the outcome is due this {}", w.en),
        });
    }
    let s = span(r, f.unit, lang, f.red);
    Some(match lang {
        Lang::Tr => format!("sonuç en geç {s} içinde belli olacak"),
        Lang::En => format!("the outcome is due within {s}"),
    })
}

/// What "played out" means for this claim, as a past verb phrase.
fn dir_past(kind: MeaningKind, dir: Direction, lang: Lang) -> &'static str {
    use Direction::{Down, Neutral, Up};
    use MeaningKind::{Breakout, Continuation, Extreme, Flow, Reversal, Squeeze};
    match (kind, dir, lang) {
        (_, Neutral, Lang::Tr) | (Squeeze, _, Lang::Tr) => "dar aralıktan çıktı",
        (_, Neutral, Lang::En) | (Squeeze, _, Lang::En) => "broke out",
        (Extreme, Up, Lang::Tr) => "daha da yükseldi",
        (Extreme, Down, Lang::Tr) => "daha da düştü",
        (Extreme, Up, Lang::En) => "rose further",
        (Extreme, Down, Lang::En) => "fell further",
        (Reversal, Up, Lang::Tr) => "yukarı döndü",
        (Reversal, Down, Lang::Tr) => "aşağı döndü",
        (Reversal, Up, Lang::En) => "turned up",
        (Reversal, Down, Lang::En) => "turned down",
        (Continuation, Up, Lang::Tr) => "yükselişini sürdürdü",
        (Continuation, Down, Lang::Tr) => "düşüşünü sürdürdü",
        (Continuation, Up, Lang::En) => "kept rising",
        (Continuation, Down, Lang::En) => "kept falling",
        (Breakout | Flow, Up, Lang::Tr) => "yükseldi",
        (Breakout | Flow, Down, Lang::Tr) => "düştü",
        (Breakout | Flow, Up, Lang::En) => "rose",
        (Breakout | Flow, Down, Lang::En) => "fell",
    }
}

/// How a closed story ended. The face says when it closed ([`closed_chip`]); this
/// sentence says what the price did, and when the claim began unless the "why" sentence
/// already anchors it in time.
fn closing(f: &Frame<'_>, lang: Lang) -> String {
    let s = span(f.horizon, f.unit, lang, f.red);
    let neutral = f.dir == Direction::Neutral || f.kind == MeaningKind::Squeeze;
    let dp = dir_past(f.kind, f.dir, lang);
    let what = match (f.ending, lang) {
        (Some(Ending::Retracted), Lang::Tr) => "dayandığı veri sonradan düzeltildi".to_owned(),
        (Some(Ending::Retracted), Lang::En) => "the data behind it was later corrected".to_owned(),
        (Some(Ending::Realized), Lang::Tr) => format!("fiyat {s} dolmadan {dp}"),
        (Some(Ending::Realized), Lang::En) => format!("the price {dp} within {s}"),
        (Some(Ending::Failed), Lang::Tr) if neutral => {
            format!("fiyat {s} boyunca dar aralıkta kaldı")
        }
        (Some(Ending::Failed), Lang::En) if neutral => {
            format!("the price stayed in a narrow range for {s}")
        }
        (Some(Ending::Failed), Lang::Tr) => {
            format!("fiyat {s} dolmadan ters yöne aynı ölçüde gitti")
        }
        (Some(Ending::Failed), Lang::En) => {
            format!("the price moved as far the other way within {s}")
        }
        (Some(Ending::Expired), Lang::Tr) => {
            format!("{s} doldu, fiyat iki yöne de yeterince gitmedi")
        }
        (Some(Ending::Expired), Lang::En) => {
            format!("{s} passed with too small a move either way")
        }
        (None, Lang::Tr) => "iddia kapandı".to_owned(),
        (None, Lang::En) => "the claim closed".to_owned(),
    };
    if anchored(f) {
        return format!("{}.", capitalize(&what));
    }
    let w = f.when(lang);
    match lang {
        Lang::Tr => format!("{} başladı, {what}.", capitalize(&w)),
        Lang::En => format!("It began {w}, and {what}."),
    }
}

/// Whether the "why" sentence names the event's time itself.
fn anchored(f: &Frame<'_>) -> bool {
    !f.red && matches!(f.det, "breakout.range" | "extreme.rsi")
}

/// The time anchor of the "why" sentence: "O gün" for an open claim (the face shows
/// when it began), the birth phrase for a closed one (the face shows when it closed).
fn anchor(f: &Frame<'_>, lang: Lang) -> String {
    if f.validity == Validity::Closed {
        capitalize(&f.when(lang))
    } else {
        that(f.unit, lang).to_owned()
    }
}

/// The last stage change (strengthened / weakened / second kind of evidence) and what
/// brought it, with when.
fn change(f: &Frame<'_>, lang: Lang) -> Option<(String, Vec<Concept>)> {
    let mv = f.moved(lang);
    let (clause, concepts) = match f.latest.as_ref() {
        Some(l) => {
            let (c, k) = latest_clause(f, l, lang);
            (Some(c), k)
        }
        None => (None, Vec::new()),
    };
    let tail = clause.map(|c| format!(": {c}")).unwrap_or_default();
    let s = match (f.stage, lang) {
        (Stage::Strengthened, Lang::Tr) => format!("{} güçlendi{tail}.", capitalize(&mv)),
        (Stage::Strengthened, Lang::En) => format!("Strengthened {mv}{tail}."),
        (Stage::Confirmed, Lang::Tr) => format!(
            "{} başka bir ölçü de aynı yönü gösterdi{tail}.",
            capitalize(&mv)
        ),
        (Stage::Confirmed, Lang::En) => {
            format!(
                "{}, a second measure pointed the same way{tail}.",
                capitalize(&mv)
            )
        }
        (Stage::Weakened, Lang::Tr) => format!("{} zayıfladı{tail}.", capitalize(&mv)),
        (Stage::Weakened, Lang::En) => format!("Weakened {mv}{tail}."),
        _ => return None,
    };
    Some((s, concepts))
}

/// What the latest fact said, in plain words (numbers only when its data is public and
/// the reader is not redacted).
#[allow(clippy::too_many_lines)]
fn latest_clause(f: &Frame<'_>, l: &Latest<'_>, lang: Lang) -> (String, Vec<Concept>) {
    let u = f.unit;
    let red = !l.open;
    let up = l.dir == Direction::Up;
    let ev = |k: &str| {
        if red {
            None
        } else {
            l.ev.get(k).copied().filter(|x| x.is_finite())
        }
    };
    let exit = l.edge == Edge::Exit || l.code.ends_with("_end");
    // The same detector again without numbers: say "again" so the sentence does not
    // read as the headline once more.
    let again = l.det == f.det;
    let (tr_again, en_again) = if again {
        ("yeniden ", "again ")
    } else {
        ("", "")
    };
    let concepts = det_concepts(l.det, up);
    let s = match l.det {
        "breakout.range" => {
            let k = param(RangeBreakout::WINDOW, lang, red);
            match (lang, up) {
                (Lang::Tr, true) => format!("fiyat {} en yüksek fiyatını geçti", tr_gen(&k, u)),
                (Lang::Tr, false) => format!("fiyat {} en düşük fiyatını kırdı", tr_gen(&k, u)),
                (Lang::En, true) => format!("the price broke its {} high", en_adj(&k, u)),
                (Lang::En, false) => format!("the price broke its {} low", en_adj(&k, u)),
            }
        }
        "extreme.rsi" => {
            let lvl =
                l.ev.get("level")
                    .copied()
                    .unwrap_or(if up { 70.0 } else { 30.0 });
            let ls = if red {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let w = lvl.round().max(0.0) as u32;
                match lang {
                    Lang::Tr => words_tr(w),
                    Lang::En => words_en(w),
                }
            } else {
                n(lvl, Cat::Osc, lang)
            };
            let val = ev("value").map(|v| n(v, Cat::Osc, lang));
            match (lang, exit, val) {
                (Lang::Tr, false, Some(v)) => format!("RSI {v} oldu"),
                (Lang::Tr, false, None) => format!(
                    "RSI {} {}",
                    tr_gen_w(&ls, red),
                    if up {
                        "üstüne çıktı"
                    } else {
                        "altına indi"
                    }
                ),
                (Lang::Tr, true, Some(v)) => format!(
                    "RSI {v} ile {} {}",
                    Word::number(&ls).gen(),
                    if up {
                        "altına indi"
                    } else {
                        "üstüne çıktı"
                    }
                ),
                (Lang::Tr, true, None) => format!(
                    "RSI {} {}",
                    tr_gen_w(&ls, red),
                    if up {
                        "altına indi"
                    } else {
                        "üstüne çıktı"
                    }
                ),
                (Lang::En, false, Some(v)) => format!("RSI reached {v}"),
                (Lang::En, false, None) => {
                    format!("RSI went {} {ls}", if up { "above" } else { "below" })
                }
                (Lang::En, true, Some(v)) => format!(
                    "RSI moved back {} {ls} at {v}",
                    if up { "below" } else { "above" }
                ),
                (Lang::En, true, None) => {
                    format!("RSI moved back {} {ls}", if up { "below" } else { "above" })
                }
            }
        }
        "reversal.rsi_divergence" => match (lang, up) {
            (Lang::Tr, true) => "RSI fiyatın yeni dibine eşlik etmedi".into(),
            (Lang::Tr, false) => "RSI fiyatın yeni tepesine eşlik etmedi".into(),
            (Lang::En, true) => "RSI did not match the price's new low".into(),
            (Lang::En, false) => "RSI did not match the price's new high".into(),
        },
        "continuation.ma_stack" => match (lang, exit, up) {
            (Lang::Tr, false, true) => "ortalamalar yükseliş sırasına girdi".into(),
            (Lang::Tr, false, false) => "ortalamalar düşüş sırasına girdi".into(),
            (Lang::Tr, true, _) => "ortalamaların sırası bozuldu".into(),
            (Lang::En, false, true) => "the averages lined up for a rise".into(),
            (Lang::En, false, false) => "the averages lined up for a fall".into(),
            (Lang::En, true, _) => "the averages fell out of order".into(),
        },
        "squeeze.bandwidth" => {
            let a = param(BandSqueeze::PERIOD, lang, red);
            let b = param(BandSqueeze::LOOKBACK, lang, red);
            match (lang, exit) {
                (Lang::Tr, false) => {
                    format!(
                        "{} bant son {} en darına indi",
                        tr_adj(&a, u),
                        tr_gen(&b, u)
                    )
                }
                (Lang::Tr, true) => "bant yeniden genişledi".into(),
                (Lang::En, false) => format!(
                    "the {} band hit its narrowest in {b} {}",
                    en_adj(&a, u),
                    u.words().en_pl
                ),
                (Lang::En, true) => "the band widened again".into(),
            }
        }
        "flow.volume_surge" => match (lang, ev("ratio")) {
            (Lang::Tr, Some(r)) => {
                format!("hacim ortalamanın {} katına çıktı", n(r, Cat::Osc, lang))
            }
            (Lang::En, Some(r)) => {
                format!("volume reached {} times its average", n(r, Cat::Osc, lang))
            }
            (Lang::Tr, None) => format!(
                "hacim {tr_again}ortalamanın {} katını aştı",
                mult(VolumeSurge::MULT, lang, red)
            ),
            (Lang::En, None) => format!(
                "volume {en_again}topped {} times its average",
                mult(VolumeSurge::MULT, lang, red)
            ),
        },
        "flow.institutional" => {
            let r = match (ev("net"), ev("avg5")) {
                (Some(a), Some(b)) if b.abs() > 0.0 => Some(a.abs() / b.abs()),
                _ => None,
            };
            let (ts, es) = if up {
                ("alım", "buying")
            } else {
                ("satım", "selling")
            };
            match (lang, r) {
                (Lang::Tr, Some(r)) => format!(
                    "kurumsal net {ts} ortalamanın {} katına çıktı",
                    n(r, Cat::Osc, lang)
                ),
                (Lang::En, Some(r)) => format!(
                    "institutional net {es} reached {} times its average",
                    n(r, Cat::Osc, lang)
                ),
                (Lang::Tr, None) => format!(
                    "kurumsal net {ts} {tr_again}ortalamanın {} katını aştı",
                    mult(InstitutionalFlow::MULT, lang, red)
                ),
                (Lang::En, None) => format!(
                    "institutional net {es} {en_again}topped {} times its average",
                    mult(InstitutionalFlow::MULT, lang, red)
                ),
            }
        }
        _ => match lang {
            Lang::Tr => "yeni bir işaret geldi".into(),
            Lang::En => "new evidence arrived".into(),
        },
    };
    (s, concepts)
}

// ---------------------------------------------------------------------------
// Ledger: "Ne kadar emin? Daha önce oldu mu?" — the expectation itself, counted
// ---------------------------------------------------------------------------

const LEDGER_LEVELS: usize = 3;

fn ledger(f: &Frame<'_>, lang: Lang, level: usize) -> String {
    let l = f.ledger;
    let dp = dir_past(f.kind, f.dir, lang);
    if l.n == 0 {
        return match lang {
            Lang::Tr => "Henüz kapanmış benzer durum yok, ne sıklıkla tuttuğu bilinmiyor.".into(),
            Lang::En => "No similar closed cases yet, so no hit rate.".into(),
        };
    }
    let thin = l.n < LEDGER_MIN_N;
    if f.red {
        let s = span(f.horizon, f.unit, lang, true);
        if thin {
            return match lang {
                Lang::Tr => {
                    "Az örnek: benzer durum henüz az, sayılar veri yetkisiyle görünür.".into()
                }
                Lang::En => "Few similar cases yet; the counts need data access.".into(),
            };
        }
        let (tr_q, en_q) = if 10 * l.realized >= 6 * l.n {
            ("Benzer durumların çoğunda", "In most similar cases")
        } else if 10 * l.realized >= 4 * l.n {
            (
                "Benzer durumların yarısına yakınında",
                "In about half of similar cases",
            )
        } else {
            ("Benzer durumların azında", "In few similar cases")
        };
        return match (lang, level) {
            (Lang::Tr, 0) => format!("{tr_q} fiyat {s} içinde {dp}."),
            (Lang::Tr, _) => format!("{tr_q} {} {dp}.", span_loc_tr(f.horizon, f.unit, true)),
            (Lang::En, 0) => format!("{en_q} the price {dp} within {s}."),
            (Lang::En, _) => format!("{en_q} it {dp} within {s}."),
        };
    }
    let (nn, rr) = (l.n.to_string(), l.realized.to_string());
    let s = span(f.horizon, f.unit, lang, false);
    let sl = span_loc_tr(f.horizon, f.unit, false);
    let r_in = Word::number(&rr).poss3_loc();
    match (lang, thin, level) {
        (Lang::Tr, false, 0) => format!("Benzer {nn} durumun {r_in} fiyat {s} içinde {dp}."),
        (Lang::Tr, false, 1) => format!("Benzer {nn} durumun {r_in} {sl} {dp}."),
        (Lang::Tr, false, _) => format!("{nn} benzerin {r_in} {sl} {dp}."),
        (Lang::Tr, true, 0) => format!("Az örnek: {nn} benzerin {r_in} fiyat {s} içinde {dp}."),
        (Lang::Tr, true, 1) => format!("Az örnek: {nn} benzerin {r_in} {sl} {dp}."),
        (Lang::Tr, true, _) => format!("Az örnek: {nn}'de {rr}, {sl} {dp}."),
        (Lang::En, false, 0) => format!("In {rr} of {nn} similar cases the price {dp} within {s}."),
        (Lang::En, false, 1) => format!("In {rr} of {nn} similar cases it {dp} within {s}."),
        (Lang::En, false, _) => format!("{rr} of {nn} similar cases {dp} within {s}."),
        (Lang::En, true, 0) => format!("Few cases: the price {dp} in {rr} of {nn} within {s}."),
        (Lang::En, true, 1) => format!("Few cases: it {dp} in {rr} of {nn} within {s}."),
        (Lang::En, true, _) => format!("Few cases: {rr} of {nn} {dp} within {s}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_words() {
        assert_eq!(words_tr(5), "beş");
        assert_eq!(words_tr(20), "yirmi");
        assert_eq!(words_tr(120), "yüz yirmi");
        assert_eq!(words_tr(15), "on beş");
        assert_eq!(words_en(5), "five");
        assert_eq!(words_en(120), "one hundred twenty");
        assert_eq!(words_en(25), "twenty-five");
        assert_eq!(mult(2.5, Lang::Tr, true), "iki buçuk");
        assert_eq!(mult(2.0, Lang::En, true), "two");
        assert_eq!(mult(2.5, Lang::Tr, false), "2,5");
    }

    #[test]
    fn sentences_are_counted_like_the_reader() {
        assert_eq!(sentence_count("A 1,5. B; C"), 3);
        assert_eq!(sentence_count("Fiyat 148,90."), 1);
        assert_eq!(sentence_count(""), 0);
    }

    #[test]
    fn percentages() {
        assert_eq!(pct(150.2, 148.9, Lang::Tr), "%0,9");
        assert_eq!(pct(150.2, 148.9, Lang::En), "0.9%");
        assert_eq!(pct(148.91, 148.9, Lang::Tr), "%0,1'den az");
    }
}

/// End to end over the engine (synthetic market with licensed flow): every story event,
/// entitled and redacted, keeps the language's own rules. The M17 harness
/// (`rounds/R5/experiments/harness/lang`) scans the same surfaces with its 13 scans.
#[cfg(test)]
mod engine_tests {
    use super::*;
    use crate::engine::EngineConfig;
    use crate::pipeline::simulate_engine;
    use crate::realize::parts::parts;
    use crate::sentence::{closed_override, condition_now, sentence_lead, validity_now, Reader};
    use crate::InstrumentInput;
    use qp_facts::{default_detectors, FlowPoint};
    use qp_schema::Fact;

    fn inputs() -> Vec<InstrumentInput> {
        qp_source::synthetic_market(8, 320, 3)
            .into_iter()
            .enumerate()
            .map(|(i, inst)| {
                let flow = qp_source::synthetic_flow(&inst.bars, 11 + i as u64)
                    .into_iter()
                    .map(|r| FlowPoint {
                        t: r.t,
                        values: r.values,
                    })
                    .collect();
                InstrumentInput {
                    series: format!("{}.1day", inst.symbol),
                    symbol: inst.symbol,
                    bars: inst.bars,
                    flow: Some(flow),
                }
            })
            .collect()
    }

    const INTERNAL: [&str; 9] = [
        " bar",
        "teyit",
        "aile",
        "sinyal",
        "olgu",
        "dedektör",
        "signal",
        "confirm",
        "detector",
    ];

    #[test]
    fn every_event_keeps_the_rules() {
        let ins = inputs();
        let (engine, _) = simulate_engine(&ins, &default_detectors, EngineConfig::r3());
        let cal = qp_calendar::ExchangeCalendar::new(&qp_source::synthetic_calendar()).unwrap();
        let mut seen = 0;
        let mut redacted_seen = 0;
        for e in engine.events() {
            let rec = engine
                .open_stories()
                .iter()
                .chain(engine.closed_stories())
                .find(|s| s.id == e.capsule)
                .unwrap();
            let input = ins.iter().find(|i| i.series == e.story.series).unwrap();
            let now = e.transition.bar;
            let bars = &input.bars[..=usize::try_from(now).unwrap()];
            let facts: Vec<Fact> = rec.facts.iter().filter(|f| f.bar <= now).cloned().collect();
            let lead = &facts[0];
            let latest = facts.last().unwrap();
            let (sh, cur) = condition_now(lead, &facts, bars);
            let sh = closed_override(&e.story, sh);
            let validity = validity_now(&e.story, sh);
            let m = lead.meaning.as_ref().unwrap();
            let lead_view = sentence_lead(lead).unwrap_or_else(|| lead.clone());
            for reader in [Reader::entitled(), Reader::unentitled()] {
                let redacted = !reader.may_see(&m.license, &m.instrument);
                let inp = SentenceInput {
                    story: &e.story,
                    lead: &lead_view,
                    latest: if latest.id == lead.id {
                        &lead_view
                    } else {
                        latest
                    },
                    now_bar: now,
                    now_t: bars[bars.len() - 1].t,
                    still_holds: sh,
                    current: if redacted { None } else { cur },
                    redacted,
                };
                let p = parts(&e.story, inp.now_t, validity, &cal, Period::Daily, redacted);
                let both = speak_both(&inp, &p);
                assert_eq!(both, speak_both(&inp, &p), "deterministic");
                let [tr, en] = &both;
                for v in [tr, en] {
                    assert!(len(&v.headline) <= MAX_HEADLINE, "{}", v.headline);
                    assert!(len(&v.detail) <= MAX_DETAIL, "{}", v.detail);
                    assert!(
                        sentence_count(&v.detail) <= MAX_DETAIL_SENTENCES,
                        "{}",
                        v.detail
                    );
                    assert!(len(&v.ledger) <= MAX_LEDGER, "{}", v.ledger);
                    let all = format!(
                        "{} {} {} {}",
                        v.headline,
                        v.detail,
                        v.ledger,
                        v.glossary
                            .iter()
                            .map(|g| format!("{} {}", g.term, g.text))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .to_lowercase();
                    for w in INTERNAL {
                        assert!(!all.contains(w), "{w}: {all}");
                    }
                    if redacted {
                        let digits = all.replace(&e.story.instrument.to_lowercase(), "");
                        assert!(!digits.bytes().any(|b| b.is_ascii_digit()), "{all}");
                    }
                }
                assert_eq!(tr.glossary.len(), en.glossary.len());
                for (a, b) in [
                    (&tr.headline, &en.headline),
                    (&tr.detail, &en.detail),
                    (&tr.ledger, &en.ledger),
                ] {
                    assert_eq!(digit_runs(a), digit_runs(b), "{a} | {b}");
                }
                seen += 1;
                redacted_seen += usize::from(redacted);
            }
        }
        assert!(seen > 200, "{seen}");
        assert!(redacted_seen > 10, "{redacted_seen}");
    }

    #[test]
    fn examples() {
        // One readable example per surface, pinned (breakout up, entitled, born today).
        let ins = inputs();
        let (engine, _) = simulate_engine(&ins, &default_detectors, EngineConfig::r3());
        let cal = qp_calendar::ExchangeCalendar::new(&qp_source::synthetic_calendar()).unwrap();
        let e = engine
            .events()
            .iter()
            .find(|e| {
                e.story.history.len() == 1
                    && e.story.direction == Direction::Up
                    && e.story.kind == MeaningKind::Breakout
            })
            .unwrap();
        let rec = engine
            .open_stories()
            .iter()
            .chain(engine.closed_stories())
            .find(|s| s.id == e.capsule)
            .unwrap();
        let input = ins.iter().find(|i| i.series == e.story.series).unwrap();
        let now = e.transition.bar;
        let bars = &input.bars[..=usize::try_from(now).unwrap()];
        let lead = &rec.facts[0];
        let (sh, cur) = condition_now(lead, std::slice::from_ref(lead), bars);
        let validity = validity_now(&e.story, sh);
        let inp = SentenceInput {
            story: &e.story,
            lead,
            latest: lead,
            now_bar: now,
            now_t: bars[bars.len() - 1].t,
            still_holds: sh,
            current: cur,
            redacted: false,
        };
        let p = parts(&e.story, inp.now_t, validity, &cal, Period::Daily, false);
        let tr = speak(&inp, &p, Lang::Tr);
        let en = speak(&inp, &p, Lang::En);
        assert!(
            tr.headline.contains("son 20 günün en yüksek fiyatını ("),
            "{}",
            tr.headline
        );
        assert!(
            tr.headline.ends_with("· bugün, hâlâ geçerli"),
            "{}",
            tr.headline
        );
        assert!(
            en.headline.contains("rose above its 20-day high ("),
            "{}",
            en.headline
        );
        assert!(tr.detail.starts_with("O gün kapanış "), "{}", tr.detail);
        assert!(tr.detail.contains("eşiğin %"), "{}", tr.detail);
        assert!(
            tr.detail
                .contains("sonuç en geç 10 işlem günü içinde belli olacak")
                || tr
                    .detail
                    .contains("Sonuç en geç 10 işlem günü içinde belli olacak"),
            "{}",
            tr.detail
        );
        assert_eq!(tr.glossary[0].term, "en yüksek fiyat");
        assert_eq!(en.glossary.last().unwrap().term, "similar cases");
    }
}
