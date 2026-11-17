//! Voice `tunca` (R5/M17): **a very short headline + scannable layers**.
//!
//! # Principle
//!
//! The headline says in a few words *what happened*, naming the threshold (and its value
//! when it fits); everything concrete then comes in short, fixed-order pieces that are
//! read by glancing, each meaningful on its own (label – value):
//!
//! | Surface | Shape (fixed order) |
//! |---------|---------------------|
//! | headline | `{title} · {when} · {validity}` — title + at most two small extras (K43) |
//! | detail | `Eşik: …. Zaman: … · …. Durum: … · ….` — three layers, one sentence each |
//! | ledger | `Karne: {r of n} · {horizon} içinde {expected move}` — the expectation itself (K49) |
//! | glossary | the screen's terms: detector concept, track record, validity word |
//!
//! Every piece comes from one engine value: the threshold from the lead fact's evidence
//! and the detector's own constants, times from the headline pieces' elapsed phrases
//! ([`crate::realize::elapsed`], calendar-based, never a bar count), validity from the
//! same value as the item and the glyph (K42/A28), the ledger from the story's ledger at
//! birth and its outcome horizon. The detail is built from the headline's own pieces, so
//! it cannot contradict the face (A33).
//!
//! A redacted reader (licensed fact, unentitled; K33/K34) gets the same layers without a
//! single digit (A32): the threshold by name and its public rule in words, vague times,
//! a qualitative ledger and a sentence saying which licence opens the figures.
//!
//! Observation, not advice: no sentence tells anyone to buy, sell or wait.
//!
//! # Entry point
//!
//! [`speak`] — one call per capsule, language and reader:
//!
//! ```ignore
//! let parts = qp_capsule::realize::parts::parts(&story, now_t, validity, &cal, period, redacted);
//! let v = qp_capsule::voice::tunca::speak(
//!     &VoiceInput { sentence: &sentence_input, parts: &parts, period },
//!     Lang::Tr,
//! );
//! // v.headline (face), v.detail (open capsule), v.ledger (face + open), v.glossary (tap)
//! ```
//!
//! Pure and deterministic: no clock, no randomness, no network, no language model.

use qp_schema::capsule::{Direction, EvidenceFamily, MeaningKind, Stage, Validity};

use crate::realize::elapsed::Period;
use crate::realize::morph::Word;
use crate::realize::num::{fmt, Cat};
use crate::realize::parts::{Ending, Parts};
use crate::realize::{Plan, Subject, LEDGER_MIN_N};
use crate::{Lang, SentenceInput};

use super::tunca_glossary::{glossary, Concept};

/// Longest headline the voice writes (characters; R5 harness SPEC §2.5).
pub const MAX_HEADLINE: usize = 80;
/// Longest detail (characters).
pub const MAX_DETAIL: usize = 200;
/// Longest ledger line (characters).
pub const MAX_LEDGER: usize = 70;

/// What the voice reads: the product's sentence input, the headline pieces of the same
/// story at the same "now", and the bar period (for calendar-based spans).
#[derive(Debug, Clone, Copy)]
pub struct VoiceInput<'a> {
    /// The sentence input (story, lead and latest fact, now, current value, redaction).
    pub sentence: &'a SentenceInput<'a>,
    /// The headline pieces (elapsed phrases, validity, ending, ledger, redaction).
    pub parts: &'a Parts,
    /// Bar period of the story's series.
    pub period: Period,
}

/// One glossary entry ("dokununca açılan" term).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gloss {
    /// The term as written on the screen.
    pub term: String,
    /// Its explanation.
    pub text: String,
}

/// The four surfaces of one capsule in one language for one reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    /// Başlık: the closed capsule's face line.
    pub headline: String,
    /// Ayrıntı: the open capsule's layers.
    pub detail: String,
    /// Karne: the track-record line.
    pub ledger: String,
    /// Sözlük: the screen's terms.
    pub glossary: Vec<Gloss>,
}

/// The voice of one capsule (see the module docs).
#[must_use]
pub fn speak(inp: &VoiceInput<'_>, lang: Lang) -> Voice {
    // Both languages are fitted together, so a piece dropped for length in one is
    // dropped in the other too (TR and EN carry the same numbers, times and validity).
    let both = [Ctx::new(inp, Lang::Tr), Ctx::new(inp, Lang::En)];
    let title = both.iter().map(Ctx::title_index).max().unwrap_or(0);
    let drops = both.iter().map(Ctx::drops_needed).max().unwrap_or(0);
    let c = &both[usize::from(lang == Lang::En)];
    Voice {
        headline: c.headline(title),
        detail: c.detail(drops),
        ledger: c.ledger(),
        glossary: glossary(&c.gloss_ctx()),
    }
}

// ---------------------------------------------------------------------------
// Spans and numbers
// ---------------------------------------------------------------------------

/// Calendar unit of a span of bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Trading days.
    Day,
    /// Trading hours.
    Hour,
    /// Minutes.
    Minute,
    /// Weeks.
    Week,
    /// Months.
    Month,
}

/// `bars` bars of `period` as a calendar span (unit, count).
#[must_use]
pub fn span(period: Period, bars: u64) -> (Unit, u64) {
    match period {
        Period::Daily => (Unit::Day, bars),
        Period::Intraday(m) => {
            let total = bars * u64::from(m);
            if total.is_multiple_of(60) {
                (Unit::Hour, total / 60)
            } else {
                (Unit::Minute, total)
            }
        }
        Period::Weekly => (Unit::Week, bars),
        Period::Monthly => (Unit::Month, bars),
    }
}

const TR_ONES: [&str; 10] = [
    "sıfır", "bir", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz",
];
const TR_TENS: [&str; 10] = [
    "", "on", "yirmi", "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan",
];
const EN_ONES: [&str; 20] = [
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
const EN_TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

/// A whole number in words (0–999; larger: "çok" / "many"). Used where a redacted reader
/// sees a public detector constant (K33: no digit on the screen).
#[must_use]
pub fn words(n: u64, lang: Lang) -> String {
    let idx = |x: u64| usize::try_from(x).unwrap_or(0);
    match lang {
        Lang::Tr => {
            if n >= 1000 {
                return "çok".into();
            }
            if n < 10 {
                return TR_ONES[idx(n)].into();
            }
            let (h, t, o) = (n / 100, (n / 10) % 10, n % 10);
            let mut out: Vec<&str> = Vec::new();
            if h > 1 {
                out.push(TR_ONES[idx(h)]);
            }
            if h > 0 {
                out.push("yüz");
            }
            if t > 0 {
                out.push(TR_TENS[idx(t)]);
            }
            if o > 0 {
                out.push(TR_ONES[idx(o)]);
            }
            out.join(" ")
        }
        Lang::En => {
            if n >= 1000 {
                return "many".into();
            }
            let below100 = |x: u64| -> String {
                if x < 20 {
                    EN_ONES[idx(x)].to_owned()
                } else if x.is_multiple_of(10) {
                    EN_TENS[idx(x / 10)].to_owned()
                } else {
                    format!("{}-{}", EN_TENS[idx(x / 10)], EN_ONES[idx(x % 10)])
                }
            };
            if n < 100 {
                return below100(n);
            }
            let (h, r) = (n / 100, n % 100);
            if r == 0 {
                format!("{} hundred", EN_ONES[idx(h)])
            } else {
                format!("{} hundred {}", EN_ONES[idx(h)], below100(r))
            }
        }
    }
}

/// How spans are written for this reader.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Spans {
    pub(crate) period: Period,
    pub(crate) lang: Lang,
    pub(crate) red: bool,
}

impl Spans {
    fn n(&self, n: u64) -> String {
        if self.red {
            words(n, self.lang)
        } else {
            n.to_string()
        }
    }

    /// `10 işlem günü` / `10 trading days`.
    pub(crate) fn count(&self, bars: u64) -> String {
        let (u, k) = span(self.period, bars);
        let n = self.n(k);
        match self.lang {
            Lang::Tr => match u {
                Unit::Day => format!("{n} işlem günü"),
                Unit::Hour => format!("{n} işlem saati"),
                Unit::Minute => format!("{n} dakika"),
                Unit::Week => format!("{n} hafta"),
                Unit::Month => format!("{n} ay"),
            },
            Lang::En => {
                let (one, many) = match u {
                    Unit::Day => ("trading day", "trading days"),
                    Unit::Hour => ("trading hour", "trading hours"),
                    Unit::Minute => ("minute", "minutes"),
                    Unit::Week => ("week", "weeks"),
                    Unit::Month => ("month", "months"),
                };
                format!("{n} {}", if k == 1 { one } else { many })
            }
        }
    }

    /// `20 günlük` / `20-day`.
    pub(crate) fn adj(&self, bars: u64) -> String {
        let (u, k) = span(self.period, bars);
        let n = self.n(k);
        match self.lang {
            Lang::Tr => match u {
                Unit::Day => format!("{n} günlük"),
                Unit::Hour => format!("{n} saatlik"),
                Unit::Minute => format!("{n} dakikalık"),
                Unit::Week => format!("{n} haftalık"),
                Unit::Month => format!("{n} aylık"),
            },
            Lang::En => {
                let w = match u {
                    Unit::Day => "day",
                    Unit::Hour => "hour",
                    Unit::Minute => "minute",
                    Unit::Week => "week",
                    Unit::Month => "month",
                };
                format!("{n}-{w}")
            }
        }
    }

    /// `son 20 günün` / `the last 20 days` (short, for a title).
    pub(crate) fn last_short(&self, bars: u64) -> String {
        let (u, k) = span(self.period, bars);
        let n = self.n(k);
        match self.lang {
            Lang::Tr => match u {
                Unit::Day => format!("son {n} günün"),
                Unit::Hour => format!("son {n} saatin"),
                Unit::Minute => format!("son {n} dakikanın"),
                Unit::Week => format!("son {n} haftanın"),
                Unit::Month => format!("son {n} ayın"),
            },
            Lang::En => self.adj(bars),
        }
    }

    /// `o gün` / `that day`: the bar of the event, in calendar words.
    pub(crate) fn that_bar(&self) -> &'static str {
        match (self.period, self.lang) {
            (Period::Daily, Lang::Tr) => "o gün",
            (Period::Daily, Lang::En) => "that day",
            (Period::Intraday(60), Lang::Tr) => "o saat",
            (Period::Intraday(60), Lang::En) => "that hour",
            (Period::Intraday(_), Lang::Tr) => "o dönem",
            (Period::Intraday(_), Lang::En) => "that period",
            (Period::Weekly, Lang::Tr) => "o hafta",
            (Period::Weekly, Lang::En) => "that week",
            (Period::Monthly, Lang::Tr) => "o ay",
            (Period::Monthly, Lang::En) => "that month",
        }
    }

    /// `son 20 işlem gününün` / `the last 20 trading days`.
    pub(crate) fn last_gen(&self, bars: u64) -> String {
        let (u, k) = span(self.period, bars);
        let n = self.n(k);
        match self.lang {
            Lang::Tr => match u {
                Unit::Day => format!("son {n} işlem gününün"),
                Unit::Hour => format!("son {n} işlem saatinin"),
                Unit::Minute => format!("son {n} dakikanın"),
                Unit::Week => format!("son {n} haftanın"),
                Unit::Month => format!("son {n} ayın"),
            },
            Lang::En => format!("the last {}", self.count(bars)),
        }
    }
}

fn usize_u64(x: usize) -> u64 {
    u64::try_from(x).unwrap_or(u64::MAX)
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Which detector concept a story is about (the seven detectors; unknown → by kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Det {
    Range,
    Rsi,
    MaStack,
    Divergence,
    Band,
    Volume,
    Institutional,
}

impl Det {
    fn of(detector: &str, kind: MeaningKind) -> Self {
        match detector {
            "breakout.range" => Self::Range,
            "extreme.rsi" => Self::Rsi,
            "continuation.ma_stack" => Self::MaStack,
            "reversal.rsi_divergence" => Self::Divergence,
            "squeeze.bandwidth" => Self::Band,
            "flow.volume_surge" => Self::Volume,
            "flow.institutional" => Self::Institutional,
            _ => match kind {
                MeaningKind::Breakout => Self::Range,
                MeaningKind::Extreme => Self::Rsi,
                MeaningKind::Continuation => Self::MaStack,
                MeaningKind::Reversal => Self::Divergence,
                MeaningKind::Squeeze => Self::Band,
                MeaningKind::Flow => Self::Volume,
            },
        }
    }
}

struct Ctx<'a> {
    p: Plan<'a>,
    parts: &'a Parts,
    lang: Lang,
    red: bool,
    det: Det,
    sp: Spans,
    horizon: u32,
}

/// Chooses between Turkish and English text.
fn pick(lang: Lang, tr: impl Into<String>, en: impl Into<String>) -> String {
    match lang {
        Lang::Tr => tr.into(),
        Lang::En => en.into(),
    }
}

impl<'a> Ctx<'a> {
    fn new(inp: &VoiceInput<'a>, lang: Lang) -> Self {
        let p = Plan::new(inp.sentence);
        let red = inp.parts.redacted || inp.sentence.redacted;
        let det = Det::of(p.detector, p.kind);
        Self {
            det,
            parts: inp.parts,
            lang,
            red,
            sp: Spans {
                period: inp.period,
                lang,
                red,
            },
            horizon: inp.sentence.story.horizon_bars,
            p,
        }
    }

    fn tr(&self) -> bool {
        self.lang == Lang::Tr
    }

    fn num(&self, v: f64, cat: Cat) -> String {
        fmt(v, cat, self.lang)
    }

    fn val(&self, keys: &[&str], cat: Cat) -> Option<String> {
        self.p.val(keys).map(|v| self.num(v, cat))
    }

    fn up(&self) -> bool {
        self.p.dir == Direction::Up
    }

    fn down(&self) -> bool {
        self.p.dir == Direction::Down
    }

    /// Instrument with the Turkish locative (`SYN03'te`); English: the bare symbol.
    fn inst_loc(&self) -> String {
        if self.tr() {
            inst_word(self.p.inst).loc()
        } else {
            self.p.inst.to_owned()
        }
    }

    /// Volume ratio of the lead fact (2 decimals, so the stated value stays within the
    /// harness tolerance of the evidence).
    fn ratio(&self) -> Option<String> {
        self.p.val(&["ratio"]).map(|v| {
            let r = (v * 100.0).round() / 100.0;
            let s = format!("{r:.2}");
            let s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
            if self.tr() {
                s.replace('.', ",")
            } else {
                s
            }
        })
    }

    // -----------------------------------------------------------------------
    // Headline
    // -----------------------------------------------------------------------

    /// Title candidates, longest first (each names the threshold or the event).
    #[allow(clippy::too_many_lines)]
    fn titles(&self) -> Vec<String> {
        let i = self.p.inst;
        let il = self.inst_loc();
        let tr = self.tr();
        let mut out: Vec<String> = Vec::new();
        match self.det {
            Det::Range => {
                let w = self
                    .sp
                    .last_short(usize_u64(qp_facts::RangeBreakout::WINDOW));
                let base = match (tr, self.down()) {
                    (true, false) => format!("{i} {w} zirvesini aştı"),
                    (true, true) => format!("{i} {w} dibini kırdı"),
                    (false, false) => format!("{i} broke above its {w} high"),
                    (false, true) => format!("{i} broke below its {w} low"),
                };
                if let Some(l) = self.val(&["level"], Cat::Price) {
                    out.push(format!("{base} ({l})"));
                }
                out.push(base);
            }
            Det::Rsi => {
                let lvl = self.val(&["level"], Cat::Osc);
                let long = match (tr, self.down(), &lvl) {
                    (true, false, Some(l)) => {
                        format!("{i} hızlı yükseldi: RSI {} aştı", Word::number(l).acc())
                    }
                    (true, true, Some(l)) => {
                        format!("{i} hızlı düştü: RSI {} altına indi", Word::number(l).gen())
                    }
                    (false, false, Some(l)) => format!("{i} rose fast: RSI crossed above {l}"),
                    (false, true, Some(l)) => format!("{i} fell fast: RSI crossed below {l}"),
                    (true, false, None) => format!("{i} hızlı yükseldi: RSI üst çizgiyi aştı"),
                    (true, true, None) => format!("{i} hızlı düştü: RSI alt çizgiyi kırdı"),
                    (false, false, None) => format!("{i} rose fast: RSI crossed its upper line"),
                    (false, true, None) => format!("{i} fell fast: RSI crossed its lower line"),
                };
                out.push(long);
                out.push(match (tr, self.down()) {
                    (true, false) => format!("{i} hızlı yükseldi (RSI)"),
                    (true, true) => format!("{i} hızlı düştü (RSI)"),
                    (false, false) => format!("{i} rose fast (RSI)"),
                    (false, true) => format!("{i} fell fast (RSI)"),
                });
            }
            Det::MaStack => out.push(match (tr, self.down()) {
                (true, false) => format!("{i} yükseliş eğilimine girdi"),
                (true, true) => format!("{i} düşüş eğilimine girdi"),
                (false, false) => format!("{i} entered an uptrend"),
                (false, true) => format!("{i} entered a downtrend"),
            }),
            Det::Divergence => out.push(match (tr, self.down()) {
                (true, false) => format!("{il} dipten yukarı dönüş izi"),
                (true, true) => format!("{il} tepeden aşağı dönüş izi"),
                (false, false) => format!("{i}: a turn-up sign at a new low"),
                (false, true) => format!("{i}: a turn-down sign at a new high"),
            }),
            Det::Band => out.push(pick(
                self.lang,
                format!("{i} dar bir aralığa sıkıştı"),
                format!("{i} settled into a narrow range"),
            )),
            Det::Volume => {
                let r = self.ratio();
                let (tv, ev) = match self.p.dir {
                    Direction::Up => ("yükseldi", "rose"),
                    Direction::Down => ("düştü", "fell"),
                    Direction::Neutral => ("yatay kaldı", "held flat"),
                };
                if let Some(r) = &r {
                    out.push(pick(
                        self.lang,
                        format!("{i} ortalamanın {r} katı hacimle {tv}"),
                        format!("{i} {ev} on {r}× its average volume"),
                    ));
                    out.push(pick(
                        self.lang,
                        format!("{i} {r} kat hacimle {tv}"),
                        format!("{i} {ev} on {r}× volume"),
                    ));
                }
                out.push(pick(
                    self.lang,
                    format!("{i} yüksek hacimle {tv}"),
                    format!("{i} {ev} on high volume"),
                ));
            }
            Det::Institutional => {
                let mult = qp_facts::InstitutionalFlow::MULT;
                let (side_tr, side_en, verb_en) = if self.down() {
                    ("satış", "selling", "sold")
                } else {
                    ("alış", "buying", "bought")
                };
                // A public detector constant: in words for a redacted reader (K33).
                let (m_tr, m_en, acc_tr) = if self.red {
                    (
                        "iki".to_owned(),
                        "twice".to_owned(),
                        "iki katını".to_owned(),
                    )
                } else {
                    let m = self.num(mult, Cat::Ratio);
                    let acc = Word::possessed(&format!("{m} katı")).acc();
                    (m.clone(), format!("{m}×"), acc)
                };
                out.push(pick(
                    self.lang,
                    format!("{il} kurumsal net {side_tr} ortalamanın {acc_tr} aştı"),
                    format!("{i}: institutional net {side_en} topped {m_en} its average"),
                ));
                out.push(pick(
                    self.lang,
                    format!("{il} kurumsal net {side_tr} {m_tr} katı aştı"),
                    format!("{i}: institutions net {verb_en} over {m_en} usual"),
                ));
                out.push(pick(
                    self.lang,
                    format!("{il} kurumsal net {side_tr}"),
                    format!("{i}: institutional net {side_en}"),
                ));
            }
        }
        out
    }

    fn validity_chip(&self) -> String {
        self.parts.validity_phrase(self.lang).to_owned()
    }

    fn headline_with(&self, t: &str) -> String {
        format!(
            "{t} · {} · {}",
            self.parts.when(self.lang),
            self.validity_chip()
        )
    }

    /// Index of the longest title whose headline fits (`MAX_HEADLINE`).
    fn title_index(&self) -> usize {
        let titles = self.titles();
        titles
            .iter()
            .position(|t| self.headline_with(t).chars().count() <= MAX_HEADLINE)
            .unwrap_or(titles.len().saturating_sub(1))
    }

    /// The headline with title candidate `idx` (the larger of the two languages' fits).
    fn headline(&self, idx: usize) -> String {
        let titles = self.titles();
        let t = titles
            .get(idx.min(titles.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default();
        self.headline_with(&t)
    }

    // -----------------------------------------------------------------------
    // Detail: three layers
    // -----------------------------------------------------------------------

    /// Layer 1 — the threshold: its name, value(s) and the detector's parameter.
    #[allow(clippy::too_many_lines)]
    fn threshold_layer(&self) -> Vec<String> {
        let tr = self.tr();
        let mut chips: Vec<String> = Vec::new();
        match self.det {
            Det::Range => {
                let g = self.sp.last_gen(usize_u64(qp_facts::RangeBreakout::WINDOW));
                let name = match (tr, self.down()) {
                    (true, false) => format!("{g} en yükseği"),
                    (true, true) => format!("{g} en düşüğü"),
                    (false, false) => format!("the high of {g}"),
                    (false, true) => format!("the low of {g}"),
                };
                chips.push(match self.val(&["level"], Cat::Price) {
                    Some(l) => format!("{name}, {l}"),
                    None => name,
                });
            }
            Det::Rsi => {
                let w = self.sp.adj(usize_u64(qp_facts::RsiExtreme::RSI));
                chips.push(match (tr, self.val(&["level"], Cat::Osc)) {
                    (true, Some(l)) => format!("{l} çizgisi ({w} RSI)"),
                    (false, Some(l)) => format!("the {l} line ({w} RSI)"),
                    (true, None) => format!(
                        "{} çizgisi ({w} RSI)",
                        if self.down() { "alt" } else { "üst" }
                    ),
                    (false, None) => format!(
                        "the {} line ({w} RSI)",
                        if self.down() { "lower" } else { "upper" }
                    ),
                });
            }
            Det::MaStack => {
                let f = self.sp.adj(usize_u64(qp_facts::MaStack::FAST));
                let s = self.sp.adj(usize_u64(qp_facts::MaStack::SLOW));
                let (fv, sv) = (
                    self.val(&["sma20", "fast"], Cat::Price),
                    self.val(&["sma50", "slow"], Cat::Price),
                );
                let one = |w: &str, v: Option<String>| match (tr, v) {
                    (true, Some(v)) => format!("{w} ortalama {v}"),
                    (false, Some(v)) => format!("{w} average {v}"),
                    (true, None) => format!("{w} ortalama"),
                    (false, None) => format!("{w} average"),
                };
                chips.push(one(&f, fv));
                chips.push(one(&s, sv));
                chips.push(match (tr, self.down()) {
                    (true, false) => "fiyat ikisinin de üstüne çıktı".into(),
                    (true, true) => "fiyat ikisinin de altına indi".into(),
                    (false, false) => "price moved above both".into(),
                    (false, true) => "price moved below both".into(),
                });
            }
            Det::Divergence => {
                let (p1, r1, p2, r2) = (
                    self.val(&["p1_price"], Cat::Price),
                    self.val(&["p1_rsi"], Cat::Osc),
                    self.val(&["p2_price"], Cat::Price),
                    self.val(&["p2_rsi"], Cat::Osc),
                );
                let (prev, new) = match (tr, self.down()) {
                    (true, false) => ("önceki dip", "yeni dip"),
                    (true, true) => ("önceki tepe", "yeni tepe"),
                    (false, false) => ("previous low", "new low"),
                    (false, true) => ("previous high", "new high"),
                };
                let one = |name: &str, p: Option<String>, r: Option<String>| match (p, r) {
                    (Some(p), Some(r)) => format!("{name} {p} (RSI {r})"),
                    (Some(p), None) => format!("{name} {p}"),
                    _ => format!("{name} (RSI)"),
                };
                chips.push(one(prev, p1, r1));
                chips.push(one(new, p2, r2));
            }
            Det::Band => {
                let w = self.sp.adj(usize_u64(qp_facts::BandSqueeze::PERIOD));
                let lb = self.sp.count(usize_u64(qp_facts::BandSqueeze::LOOKBACK));
                let lb_tr = Word::noun(&lb).gen();
                chips.push(pick(
                    self.lang,
                    format!("{w} bant {lb_tr} en darı"),
                    format!("the {w} band at its narrowest in {lb}"),
                ));
                if let (Some(lo), Some(hi)) = (
                    self.val(&["lower"], Cat::Price),
                    self.val(&["upper"], Cat::Price),
                ) {
                    chips.push(pick(
                        self.lang,
                        format!("bant {lo}–{hi}"),
                        format!("band {lo}–{hi}"),
                    ));
                }
            }
            Det::Volume => {
                let w = self.sp.adj(usize_u64(qp_facts::VolumeSurge::WINDOW));
                let m = self.num(qp_facts::VolumeSurge::MULT, Cat::Ratio);
                chips.push(pick(
                    self.lang,
                    format!("{w} ortalama hacmin {m} katı"),
                    format!("{m}× the {w} average volume"),
                ));
                if let Some(r) = self.ratio() {
                    chips.push(pick(
                        self.lang,
                        format!("{} {r} katı", self.sp.that_bar()),
                        format!("{} {r}×", self.sp.that_bar()),
                    ));
                }
            }
            Det::Institutional => {
                let w = self.sp.adj(usize_u64(qp_facts::InstitutionalFlow::WINDOW));
                let mult = qp_facts::InstitutionalFlow::MULT;
                let m = if self.red {
                    // A public detector constant, in words (K33: no digit).
                    pick(self.lang, "iki", "twice")
                } else {
                    self.num(mult, Cat::Ratio)
                };
                let (side_tr, side_en) = if self.down() {
                    ("satışta", "selling")
                } else {
                    ("alışta", "buying")
                };
                chips.push(match (tr, self.red) {
                    (true, _) => format!("kurumsal net {side_tr} {w} ortalamanın {m} katı"),
                    (false, true) => {
                        format!("{m} the {w} average of institutional net {side_en}")
                    }
                    (false, false) => {
                        format!("{m}× the {w} average of institutional net {side_en}")
                    }
                });
                if let Some(a) = self.val(&["avg5"], Cat::Count) {
                    chips.push(pick(
                        self.lang,
                        format!("ortalama {a}"),
                        format!("average {a}"),
                    ));
                }
                if let Some(n) = self.val(&["net"], Cat::Count) {
                    chips.push(format!("{} {n}", self.sp.that_bar()));
                }
            }
        }
        chips
    }

    fn family_name(&self, f: Option<EvidenceFamily>) -> String {
        let (t, e) = match f {
            Some(EvidenceFamily::Price) => ("fiyat", "price"),
            Some(EvidenceFamily::Volume) => ("alım satım miktarı", "trading activity"),
            Some(EvidenceFamily::Flow) => ("para akışı", "money flow"),
            Some(EvidenceFamily::Derivative) => ("türev piyasası", "the derivatives market"),
            Some(EvidenceFamily::News) => ("haber akışı", "news flow"),
            None => ("başka bir veri", "other data"),
        };
        pick(self.lang, t, e)
    }

    /// Layer 2 — when: birth, the last change, the latest value.
    fn time_layer(&self) -> Vec<String> {
        let tr = self.tr();
        let born = self.parts.when(self.lang);
        let mut chips = vec![pick(
            self.lang,
            format!("{born} başladı"),
            format!("began {born}"),
        )];
        let moved = self.parts.moved(self.lang);
        let stage_chip = match self.p.stage {
            Stage::Born => None,
            Stage::Candidate => Some(pick(
                self.lang,
                "dönem kapanmadı, henüz kesin değil",
                "the period has not closed, not final yet",
            )),
            Stage::Strengthened => Some(pick(
                self.lang,
                format!("{moved} güçlendi"),
                format!("strengthened {moved}"),
            )),
            Stage::Weakened => Some(pick(
                self.lang,
                format!("{moved} zayıfladı"),
                format!("weakened {moved}"),
            )),
            Stage::Confirmed => {
                let fam = self.family_name(self.p.latest_family);
                Some(if tr {
                    format!("{moved} {fam} {} aynı yönü gösterdi", da(&fam))
                } else {
                    format!("{fam} pointed the same way {moved}")
                })
            }
            Stage::Realized | Stage::Invalidated | Stage::Expired | Stage::Retracted => Some(pick(
                self.lang,
                format!("{moved} kapandı"),
                format!("closed {moved}"),
            )),
        };
        chips.extend(stage_chip);
        chips
    }

    /// The expected move as a noun (ledger) — what "tuttu / played out" means.
    fn expected_noun(&self) -> String {
        let (t, e) = match (self.p.kind, self.p.dir) {
            (_, Direction::Neutral) | (MeaningKind::Squeeze, _) => {
                ("aralığın kırılması", "range break")
            }
            (MeaningKind::Extreme, Direction::Up) => ("yükselişin sürmesi", "further rise"),
            (MeaningKind::Extreme, Direction::Down) => ("düşüşün sürmesi", "further fall"),
            (_, Direction::Up) => ("yükseliş", "rise"),
            (_, Direction::Down) => ("düşüş", "fall"),
        };
        pick(self.lang, t, e)
    }

    /// Whether the story's claim is neutral (a move either way).
    fn neutral(&self) -> bool {
        self.p.dir == Direction::Neutral || self.p.kind == MeaningKind::Squeeze
    }

    /// Layer 3 — status: validity (closed: how and why), current value, trust.
    fn status_layer(&self) -> Vec<String> {
        let tr = self.tr();
        let h = self.sp.count(u64::from(self.horizon));
        let mut chips: Vec<String> = Vec::new();
        let v = self.parts.validity_phrase(self.lang);
        let main = match (self.parts.validity, self.parts.ending) {
            (Validity::Closed, Some(Ending::Realized)) => {
                let what = match (self.neutral(), self.p.kind, self.up()) {
                    (true, _, _) => pick(
                        self.lang,
                        "fiyat aralığın dışına çıktı",
                        "price broke out of the range",
                    ),
                    (false, MeaningKind::Extreme, true) => {
                        pick(self.lang, "yükseliş sürdü", "the rise went on")
                    }
                    (false, MeaningKind::Extreme, false) => {
                        pick(self.lang, "düşüş sürdü", "the fall went on")
                    }
                    (false, _, true) => pick(
                        self.lang,
                        format!("fiyat {h} içinde yükseldi"),
                        format!("price rose within {h}"),
                    ),
                    (false, _, false) => pick(
                        self.lang,
                        format!("fiyat {h} içinde düştü"),
                        format!("price fell within {h}"),
                    ),
                };
                chips.push(v.to_owned());
                what
            }
            (Validity::Closed, Some(Ending::Failed)) => {
                let what = match (self.neutral(), self.p.kind) {
                    (true, _) => pick(
                        self.lang,
                        format!("fiyat {h} aralıkta kaldı"),
                        format!("price stayed in range for {h}"),
                    ),
                    (false, MeaningKind::Extreme) => {
                        pick(self.lang, "fiyat geri döndü", "price snapped back")
                    }
                    _ => pick(
                        self.lang,
                        "fiyat ters yöne gitti",
                        "price went the other way",
                    ),
                };
                chips.push(v.to_owned());
                what
            }
            (Validity::Closed, Some(Ending::Expired)) => {
                chips.push(v.to_owned());
                pick(
                    self.lang,
                    format!("{h} içinde sonuç çıkmadı"),
                    format!("no outcome within {h}"),
                )
            }
            (Validity::Closed, Some(Ending::Retracted)) => {
                chips.push(v.to_owned());
                pick(
                    self.lang,
                    "dayandığı veri düzeltildi",
                    "the data behind it was corrected",
                )
            }
            _ => v.to_owned(),
        };
        // Closed: [validity, why]; open: [validity].
        chips.push(main);
        if !self.p.closed() {
            if let (Some(c), Some(cur)) = (self.p.cond.as_ref(), self.p.current()) {
                match c.subject {
                    Subject::Close => chips.push(pick(
                        self.lang,
                        format!("son kapanış {}", self.num(cur, Cat::Price)),
                        format!("last close {}", self.num(cur, Cat::Price)),
                    )),
                    Subject::Rsi => chips.push(pick(
                        self.lang,
                        format!("RSI şu an {}", self.num(cur, Cat::Osc)),
                        format!("RSI now {}", self.num(cur, Cat::Osc)),
                    )),
                    _ => {}
                }
            }
        }
        chips.push(if self.p.server {
            pick(self.lang, "sunucu doğruladı", "verified by the server")
        } else {
            pick(self.lang, "bu cihazda sınandı", "checked on this device")
        });
        if self.red {
            let lic = match self.p.entitlement {
                Some(e) if e.starts_with("flow") => {
                    pick(self.lang, "kurumsal akış lisansıyla", "a flow licence")
                }
                _ => pick(self.lang, "veri lisansıyla", "a data licence"),
            };
            chips.push(if tr {
                format!("rakamlar {lic} açılır")
            } else {
                format!("figures need {lic}")
            });
        }
        chips
    }

    /// Drop priority of an optional chip (0: never dropped; higher: dropped first).
    fn drop_priority(&self, layer: usize, idx: usize) -> u8 {
        match (layer, idx) {
            (0, 0) | (1, 0) | (2, 0) => 0,
            (0, _) => match self.det {
                Det::Volume => 3,
                Det::Institutional if idx == 1 => 2,
                Det::Institutional => 3,
                Det::MaStack if idx == 2 => 4,
                _ => 0,
            },
            (1, _) => 1,
            (2, 1) if self.has_current_chip() => 5,
            (2, 1) if self.p.closed() => 2,
            _ => 0,
        }
    }

    fn labels(&self) -> [&'static str; 3] {
        if self.tr() {
            ["Eşik", "Zaman", "Durum"]
        } else {
            ["Threshold", "Timeline", "Status"]
        }
    }

    /// The three layers, each chip with its drop priority.
    fn layers(&self) -> Vec<Vec<(String, u8)>> {
        [
            self.threshold_layer(),
            self.time_layer(),
            self.status_layer(),
        ]
        .into_iter()
        .enumerate()
        .map(|(l, chips)| {
            chips
                .into_iter()
                .enumerate()
                .map(|(i, c)| {
                    let pr = self.drop_priority(l, i);
                    (c, pr)
                })
                .collect()
        })
        .collect()
    }

    fn render(&self, ls: &[Vec<(String, u8)>]) -> String {
        self.labels()
            .iter()
            .zip(ls.iter())
            .map(|(l, chips)| {
                let cs: Vec<&str> = chips.iter().map(|(c, _)| c.as_str()).collect();
                format!("{l}: {}.", cs.join(" · "))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Drops the optional chip with the highest priority (ties: the later one); `false`
    /// when nothing optional is left.
    fn drop_one(ls: &mut [Vec<(String, u8)>]) -> bool {
        let worst = ls
            .iter()
            .enumerate()
            .flat_map(|(l, cs)| cs.iter().enumerate().map(move |(i, (_, p))| (*p, l, i)))
            .filter(|(p, _, _)| *p > 0)
            .max();
        match worst {
            Some((_, l, i)) => {
                ls[l].remove(i);
                true
            }
            None => false,
        }
    }

    /// How many optional chips must go for the detail to fit (`MAX_DETAIL`).
    fn drops_needed(&self) -> usize {
        let mut ls = self.layers();
        let mut n = 0;
        while self.render(&ls).chars().count() > MAX_DETAIL && Self::drop_one(&mut ls) {
            n += 1;
        }
        n
    }

    /// The detail with `drops` optional chips removed, highest priority first: the
    /// current value, raw counts of the threshold layer, the reason a closed claim
    /// closed, then the stage change. The threshold's name and value, the birth time,
    /// the validity and the trust mark always stay.
    fn detail(&self, drops: usize) -> String {
        let mut ls = self.layers();
        for _ in 0..drops {
            if !Self::drop_one(&mut ls) {
                break;
            }
        }
        self.render(&ls)
    }

    fn has_current_chip(&self) -> bool {
        !self.p.closed()
            && self.p.current().is_some()
            && self
                .p
                .cond
                .as_ref()
                .is_some_and(|c| matches!(c.subject, Subject::Close | Subject::Rsi))
    }

    // -----------------------------------------------------------------------
    // Ledger
    // -----------------------------------------------------------------------

    fn ledger(&self) -> String {
        let l = self.parts.ledger;
        let noun = self.expected_noun();
        let h = self.sp.count(u64::from(self.horizon));
        let expect = pick(
            self.lang,
            format!("{h} içinde {noun}"),
            format!("{noun} within {h}"),
        );
        let thin = l.n > 0 && l.n < LEDGER_MIN_N;
        let (label, value) = if l.n == 0 {
            (
                pick(self.lang, "Karne", "Record"),
                pick(self.lang, "henüz örnek yok", "no cases yet"),
            )
        } else if self.red {
            let label = if thin {
                pick(self.lang, "Az örnek", "Few cases")
            } else {
                pick(self.lang, "Karne", "Record")
            };
            let value = if thin {
                pick(self.lang, "oran henüz yok", "no rate yet")
            } else if 10 * l.realized >= 6 * l.n {
                pick(self.lang, "çoğunlukla tuttu", "mostly played out")
            } else if 10 * l.realized >= 4 * l.n {
                pick(self.lang, "yarı yarıya tuttu", "half played out")
            } else {
                pick(self.lang, "çoğunlukla tutmadı", "mostly not")
            };
            (label, value)
        } else {
            let label = if thin {
                pick(self.lang, "Az örnek", "Few cases")
            } else {
                pick(self.lang, "Karne", "Record")
            };
            let value = pick(
                self.lang,
                format!(
                    "{} {} tuttu",
                    Word::number(&l.n.to_string()).loc(),
                    l.realized
                ),
                format!("{} of {} played out", l.realized, l.n),
            );
            (label, value)
        };
        format!("{label}: {value} · {expect}")
    }

    fn gloss_ctx(&self) -> GlossCtx {
        GlossCtx {
            concept: match self.det {
                Det::Range if self.down() => Concept::RangeLow,
                Det::Range => Concept::RangeHigh,
                Det::Rsi | Det::Divergence => Concept::Rsi,
                Det::MaStack => Concept::Average,
                Det::Band => Concept::Band,
                Det::Volume => Concept::Volume,
                Det::Institutional => Concept::Institutional,
            },
            validity: self.parts.validity,
            ending: self.parts.ending,
            neutral: self.neutral(),
            sp: self.sp,
        }
    }
}

/// What the glossary needs to know about the screen.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlossCtx {
    pub(crate) concept: Concept,
    pub(crate) validity: Validity,
    pub(crate) ending: Option<Ending>,
    pub(crate) neutral: bool,
    pub(crate) sp: Spans,
}

/// An instrument symbol as a Turkish word: a trailing number is read aloud
/// (`SYN03` → "…üç" → `SYN03'te`), letters otherwise (`THYAO'da`).
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
            spoken: crate::realize::morph::number_last_word(&digits).to_owned(),
            apostrophe: true,
            possessed: false,
        }
    }
}

/// Turkish `de/da` clitic after a phrase (vowel harmony with its last vowel).
fn da(w: &str) -> &'static str {
    let lv = w
        .chars()
        .rev()
        .find(|c| "aıoueiöü".contains(*c))
        .unwrap_or('e');
    if "aıou".contains(lv) {
        "da"
    } else {
        "de"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use qp_calendar::{builtin, days_from_civil, ExchangeCalendar};
    use qp_schema::capsule::{
        Edge, FactMeaning, LedgerStat, License, LicenseClass, MeaningCondition, Story, TimeKind,
        Transition, Trust,
    };
    use qp_schema::{Author, CompareOp, Fact, Provenance};

    use crate::realize::parts::parts;

    /// UTC ms of 07:00 UTC on a 2026 day (10:00 Istanbul).
    fn ist(m: u32, d: u32) -> i64 {
        (days_from_civil(2026, m, d) * 86_400 + 7 * 3600) * 1000
    }

    struct Case {
        story: Story,
        lead: Fact,
        now_t: i64,
        current: Option<f64>,
        redacted: bool,
    }

    fn case(
        det: &str,
        kind: MeaningKind,
        dir: Direction,
        ev: &[(&str, f64)],
        stage: Stage,
        ledger: LedgerStat,
        redacted: bool,
    ) -> Case {
        let inst = "SYN03";
        let licensed = det == "flow.institutional";
        let cond = (det == "breakout.range").then(|| MeaningCondition {
            subject: "close".into(),
            op: CompareOp::Gt,
            key: "level".into(),
            within_bars: Some(3),
        });
        let lead = Fact {
            id: format!("fx.{inst}.{det}.0"),
            kind: det.into(),
            series: format!("{inst}.1day"),
            bar: 100,
            t: ist(5, 4),
            text: String::new(),
            evidence: ev
                .iter()
                .map(|(k, v)| ((*k).to_owned(), *v))
                .collect::<BTreeMap<_, _>>(),
            evidence_ref: None,
            meaning: Some(FactMeaning {
                kind,
                direction: dir,
                family: EvidenceFamily::Price,
                time: TimeKind::Event,
                edge: Edge::Point,
                detector: det.into(),
                detector_version: 1,
                instrument: inst.into(),
                strength: 0.5,
                license: License {
                    class: if licensed {
                        LicenseClass::Licensed
                    } else {
                        LicenseClass::Public
                    },
                    entitlement: licensed.then(|| "flow/*".to_owned()),
                },
                geometry: Vec::new(),
                condition: cond.clone(),
                server: licensed,
            }),
            provenance: Provenance {
                author: Author::Engine {
                    name: "test".into(),
                    version: "0".into(),
                },
                at: 0,
                based_on: Vec::new(),
                approved_by: None,
            },
        };
        let tr = |stage, bar, t| Transition {
            stage,
            bar,
            t,
            facts: Vec::new(),
            reason: String::new(),
        };
        let mut history = vec![tr(Stage::Born, 100, ist(5, 4))];
        if stage != Stage::Born {
            history.push(tr(stage, 102, ist(5, 6)));
        }
        let story = Story {
            instrument: inst.into(),
            theme: "t".into(),
            kind,
            direction: dir,
            time: TimeKind::Event,
            stage,
            families: vec![EvidenceFamily::Price],
            strength: 0.5,
            importance: 1.0,
            tier: 1,
            series: format!("{inst}.1day"),
            born_bar: 100,
            last_bar: 102,
            horizon_bars: 10,
            ledger_key: "k".into(),
            ledger_at_birth: ledger,
            trust: if licensed {
                Trust::Server
            } else {
                Trust::Replica
            },
            history,
            condition: cond,
            resolution: None,
        };
        Case {
            story,
            lead,
            now_t: ist(5, 6),
            current: (!redacted && det == "breakout.range").then_some(151.2),
            redacted,
        }
    }

    fn voices(c: &Case) -> [Voice; 2] {
        let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
        let validity = if c.story.stage.is_closed() {
            Validity::Closed
        } else {
            Validity::Valid
        };
        let p = parts(&c.story, c.now_t, validity, &cal, Period::Daily, c.redacted);
        let si = SentenceInput {
            story: &c.story,
            lead: &c.lead,
            latest: &c.lead,
            now_bar: 102,
            now_t: c.now_t,
            still_holds: c.story.condition.as_ref().map(|_| true),
            current: c.current,
            redacted: c.redacted,
        };
        let inp = VoiceInput {
            sentence: &si,
            parts: &p,
            period: Period::Daily,
        };
        [speak(&inp, Lang::Tr), speak(&inp, Lang::En)]
    }

    fn breakout(stage: Stage, ledger: LedgerStat) -> Case {
        case(
            "breakout.range",
            MeaningKind::Breakout,
            Direction::Up,
            &[("level", 148.9), ("close", 149.5)],
            stage,
            ledger,
            false,
        )
    }

    #[test]
    fn breakout_names_its_threshold_and_layers() {
        let [tr, en] = voices(&breakout(
            Stage::Born,
            LedgerStat {
                n: 24,
                realized: 15,
            },
        ));
        assert_eq!(
            tr.headline,
            "SYN03 son 20 günün zirvesini aştı (148,90) · 2 işlem günü önce · hâlâ geçerli"
        );
        assert_eq!(
            en.headline,
            "SYN03 broke above its 20-day high (148.90) · 2 trading days ago · still holds"
        );
        assert_eq!(
            tr.detail,
            "Eşik: son 20 işlem gününün en yükseği, 148,90. Zaman: 2 işlem günü önce başladı. \
             Durum: hâlâ geçerli · son kapanış 151,20 · bu cihazda sınandı."
        );
        assert_eq!(
            tr.ledger,
            "Karne: 24'te 15 tuttu · 10 işlem günü içinde yükseliş"
        );
        assert_eq!(
            en.ledger,
            "Record: 15 of 24 played out · rise within 10 trading days"
        );
        for v in [&tr, &en] {
            assert!(v.headline.chars().count() <= MAX_HEADLINE);
            assert!(v.detail.chars().count() <= MAX_DETAIL);
            assert!(v.ledger.chars().count() <= MAX_LEDGER);
            assert_eq!(v.glossary.len(), 3);
        }
    }

    #[test]
    fn closed_and_thin() {
        let [tr, en] = voices(&breakout(
            Stage::Invalidated,
            LedgerStat { n: 4, realized: 1 },
        ));
        assert!(tr.headline.ends_with("· tutmadı"), "{}", tr.headline);
        assert!(
            en.headline.ends_with("· did not play out"),
            "{}",
            en.headline
        );
        assert!(tr.detail.contains("Durum: tutmadı · fiyat ters yöne gitti"));
        assert!(tr.detail.contains("bugün kapandı"));
        assert_eq!(
            tr.ledger,
            "Az örnek: 4'te 1 tuttu · 10 işlem günü içinde yükseliş"
        );
        assert_eq!(tr.glossary[2].term, "Tutmadı");
        assert_eq!(en.glossary[2].term, "Did not play out");
    }

    #[test]
    fn redacted_reader_sees_no_digit_but_the_licence() {
        let c = case(
            "flow.institutional",
            MeaningKind::Flow,
            Direction::Up,
            &[("net", 2_340_000.0), ("avg5", 780_000.0)],
            Stage::Strengthened,
            LedgerStat {
                n: 30,
                realized: 20,
            },
            true,
        );
        let [tr, en] = voices(&c);
        for v in [&tr, &en] {
            let mut all = vec![v.headline.clone(), v.detail.clone(), v.ledger.clone()];
            all.extend(v.glossary.iter().map(|g| format!("{}: {}", g.term, g.text)));
            for t in all {
                assert!(
                    !t.replace("SYN03", " ").bytes().any(|b| b.is_ascii_digit()),
                    "{t}"
                );
            }
        }
        assert!(tr
            .detail
            .contains("rakamlar kurumsal akış lisansıyla açılır"));
        assert!(tr.detail.contains("beş günlük ortalamanın iki katı"));
        assert!(en.detail.contains("figures need a flow licence"));
        assert_eq!(
            tr.ledger,
            "Karne: çoğunlukla tuttu · on işlem günü içinde yükseliş"
        );
    }

    #[test]
    fn deterministic_and_symmetric() {
        let c = breakout(Stage::Strengthened, LedgerStat::default());
        assert_eq!(voices(&c), voices(&c));
        let [tr, en] = voices(&c);
        assert_eq!(tr.glossary.len(), en.glossary.len());
        assert_eq!(
            en.ledger,
            "Record: no cases yet · rise within 10 trading days"
        );
    }

    #[test]
    fn instrument_locative_reads_the_number() {
        assert_eq!(inst_word("SYN03").loc(), "SYN03'te");
        assert_eq!(inst_word("SYN26").loc(), "SYN26'da");
        assert_eq!(inst_word("THYAO").loc(), "THYAO'da");
    }

    #[test]
    fn number_words() {
        assert_eq!(words(5, Lang::Tr), "beş");
        assert_eq!(words(10, Lang::Tr), "on");
        assert_eq!(words(20, Lang::Tr), "yirmi");
        assert_eq!(words(120, Lang::Tr), "yüz yirmi");
        assert_eq!(words(215, Lang::Tr), "iki yüz on beş");
        assert_eq!(words(5, Lang::En), "five");
        assert_eq!(words(15, Lang::En), "fifteen");
        assert_eq!(words(120, Lang::En), "one hundred twenty");
        assert_eq!(words(42, Lang::En), "forty-two");
    }

    #[test]
    fn spans_follow_the_calendar() {
        let d = Spans {
            period: Period::Daily,
            lang: Lang::Tr,
            red: false,
        };
        assert_eq!(d.count(10), "10 işlem günü");
        assert_eq!(d.adj(20), "20 günlük");
        assert_eq!(d.last_gen(20), "son 20 işlem gününün");
        let h = Spans {
            period: Period::Intraday(60),
            lang: Lang::En,
            red: false,
        };
        assert_eq!(h.count(10), "10 trading hours");
        assert_eq!(h.adj(20), "20-hour");
        let q = Spans {
            period: Period::Intraday(15),
            lang: Lang::En,
            red: true,
        };
        assert_eq!(q.count(8), "two trading hours");
        assert_eq!(q.count(3), "forty-five minutes");
    }
}
