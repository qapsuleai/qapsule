//! (R5B/V) The agent comment validator (`rounds/R5B/PLAN.md` §2.1, GOAL principle 2).
//!
//! Pure and deterministic: no clock, network, randomness or language model. The same
//! draft, evidence and strictness always give the same result (and the same reasons in
//! the same order).
//!
//! Checks:
//!
//! 1. **Citations** (every strictness): `cites` is not empty; every id is known to the
//!    reader (`license`/`not_visible` when it exists but the reader may not see it,
//!    `cites`/`unknown_id` otherwise); every `[n]` marker points into `cites`; every
//!    external quote names its source.
//! 2. **Numbers** ([`Strictness::CitesNumbers`], [`Strictness::PerSentence`]): every
//!    number in the text is one of the evidence values or an external quote value, by the
//!    rounding rule of [`number_matches`]; every external value appears in its quote.
//! 3. **Advice** (every strictness): the R5 advice list (`ADVICE_TR`, `ADVICE_EN`; R5
//!    `lang/SPEC.md` §2.2) with the Turkish suffix rule.
//! 4. **Licence** (every strictness): a number that matches a withheld value and no
//!    visible value is `licensed_value`, even when an external quote carries it; an
//!    external value that matches a withheld value is `licensed_external`.
//! 5. **Per sentence** ([`Strictness::PerSentence`]): a sentence with a number carries a
//!    valid `[n]` marker.
//! 6. **Direction** ([`Strictness::CitesNumbers`], [`Strictness::PerSentence`]): when
//!    every cited directional item ([`Evidence::directions`]) points the same way and the
//!    text uses only words of the opposite direction (`DIR_*`), `wrong_direction`. Both
//!    directions or none pass (false rejections kept low).
//!
//! The advice scan always uses both lists (TR and EN); `lang` decides only how numbers,
//! dates and direction words are read.
//!
//! What is not a number: instrument symbols ([`Evidence::symbols`], removed first),
//! citation markers `[1]`, clock times (`14:30`), dates (`12 Kas`, `12 Nov`, `Nov 12`,
//! `Kasım 2026`, `2026-11-21`, `21.11.2026`) and a year written as a year (`2026'da`,
//! `2026 yılında`, `in 2026`). A bare `2026` is a number.
//!
//! Reading a number: `1.234,5` / `1,234.5` (both separators: the last one is the
//! decimal mark); one separator repeated (`1.234.567`) is a thousands separator; a
//! single separator is the decimal mark of the language (TR `,`, EN `.`), and the other
//! one is a thousands separator only when exactly three digits follow and at most three
//! (not starting with `0`) precede it: TR `1.234` = 1234, EN `1.234` = 1.234, TR `12.5`
//! = 12.5, EN `1,234` = 1234, EN `12,5` = 12.5.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

pub use qp_schema::comment::{Check, CommentDraft, ExternalQuote, Reason, Strictness};
use qp_schema::protocol::Language;

use crate::Lang;

/// The evidence the authority gathered for one draft, projected to the reader.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    /// Ids that exist and the reader may see.
    pub known_ids: BTreeSet<String>,
    /// Ids that exist but the reader may not see (licence).
    pub hidden_ids: BTreeSet<String>,
    /// Numbers of the cited items, visible to the reader.
    pub values: Vec<CitedValue>,
    /// Numbers in the cited items' or the instrument's scope the reader may not see.
    pub withheld: Vec<f64>,
    /// Instrument symbols (removed before the number scan, e.g. `SYN03`).
    pub symbols: Vec<String>,
    /// Directions of the cited items (facts / capsules); neutral ones are ignored.
    #[serde(default)]
    pub directions: Vec<CitedDirection>,
}

/// A direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Up.
    Up,
    /// Down.
    Down,
    /// No direction.
    Neutral,
}

/// The direction of one cited item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitedDirection {
    /// The cited item.
    pub id: String,
    /// Its direction.
    pub direction: Direction,
}

/// One number of a cited item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CitedValue {
    /// The cited item.
    pub id: String,
    /// What the number is (e.g. `close`, `rsi`).
    pub key: String,
    /// The value.
    pub value: f64,
    /// Decimals the engine shows it with.
    pub decimals: u8,
}

/// Advice words (TR), R5 `lang/tool/src/scan.rs` `ADVICE_TR` verbatim.
pub const ADVICE_TR: &[&str] = &[
    r"al",
    r"alın",
    r"alınız",
    r"alınmalı\w*",
    r"almalı\w*",
    r"alabilirsin\w*",
    r"alınabilir",
    r"sat",
    r"satın",
    r"satınız",
    r"satılmalı\w*",
    r"satmalı\w*",
    r"satabilirsin\w*",
    r"satılabilir",
    r"fırsat\w*",
    r"tavsiye\w*",
    r"öneririz",
    r"öneriyoruz",
    r"önerimiz\w*",
    r"hedef fiyat\w*",
    r"kaçırma\w*",
    r"pozisyon aç\w*",
    r"zarar kes\w*",
    r"stop loss",
    r"kâr al\w*",
    r"kar al\w*",
];

/// Advice words (EN), R5 `lang/tool/src/scan.rs` `ADVICE_EN` verbatim.
pub const ADVICE_EN: &[&str] = &[
    r"buy",
    r"sell",
    r"opportunit(y|ies)",
    r"should",
    r"recommend\w*",
    r"advice",
    r"advis\w*",
    r"target price\w*",
    r"don't miss",
    r"do not miss",
    r"stop[- ]loss",
    r"take profit",
    r"go (long|short)",
];

/// The validator's language for a protocol language.
#[must_use]
pub fn lang_of(l: Language) -> Lang {
    match l {
        Language::Tr => Lang::Tr,
        Language::En => Lang::En,
    }
}

/// Lower case; Turkish maps `I` → `ı` and `İ` → `i`.
#[must_use]
pub fn lower(s: &str, lang: Lang) -> String {
    match lang {
        Lang::Tr => s
            .chars()
            .flat_map(|c| match c {
                'I' => vec!['ı'],
                'İ' => vec!['i'],
                c => c.to_lowercase().collect(),
            })
            .collect(),
        Lang::En => s.to_lowercase(),
    }
}

/// Splits into sentences: a break after `.`, `;`, `!`, `?` followed by whitespace or
/// the end (a decimal mark is never followed by whitespace). Empty pieces dropped.
#[must_use]
pub fn sentences(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end = matches!(c, '.' | ';' | '!' | '?')
            && chars.get(i + 1).is_none_or(|n| n.is_whitespace());
        if end {
            let t = cur.trim().to_owned();
            if !t.is_empty() {
                out.push(t);
            }
            cur.clear();
        }
    }
    let t = cur.trim().to_owned();
    if !t.is_empty() {
        out.push(t);
    }
    out
}

/// How a number is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumKind {
    /// A plain number.
    Plain,
    /// A percentage (`%12,5`, `12.5%`).
    Percent,
    /// A multiple (`2,5 kat`, `2.5x`).
    Multiple,
}

/// A number as written in a text.
#[derive(Debug, Clone, PartialEq)]
pub struct Num {
    /// The token as written (lower-cased).
    pub raw: String,
    /// `Some(true)` for an explicit minus, `Some(false)` for an explicit plus.
    pub negative: Option<bool>,
    /// The digits without separators (value × 10^decimals); `None` when too long.
    pub scaled: Option<u128>,
    /// Digits after the decimal mark.
    pub decimals: u8,
    /// The (signed) value.
    pub value: f64,
    /// Plain, percent or multiple.
    pub kind: NumKind,
}

fn re(cell: &'static OnceLock<Regex>, p: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(p).expect("comment regex"))
}

const TR_MONTHS: &str = r"(?:(?:ocak|şubat|mart|nisan|mayıs|haziran|temmuz|ağustos|eylül|ekim|kasım|aralık)\p{L}*(?:['’]\p{L}+)?|(?:oca|şub|mar|nis|may|haz|tem|ağu|eyl|eki|kas|ara)\b\.?)";
/// EN months after a day: "may" only with a year ("12 may rise" is not a date).
const EN_MONTHS_DM: &str = r"(?:(?:january|february|march|april|june|july|august|september|october|november|december)\b|(?:jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)\b\.?|may\s+\d{4}\b)";
const EN_MONTHS: &str = r"(?:(?:january|february|march|april|may|june|july|august|september|october|november|december)\b|(?:jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)\b\.?)";

fn masks(lang: Lang) -> &'static [Regex] {
    static TR: OnceLock<Vec<Regex>> = OnceLock::new();
    static EN: OnceLock<Vec<Regex>> = OnceLock::new();
    let common = |v: &mut Vec<String>| {
        // citation markers
        v.push(r"\[\d+(?:\s*[,;]\s*\d+)*\]".to_owned());
        // ISO date, numeric day.month.year / month/day/year
        // (R5B audit B1/B4) ranges checked: a "date" or "time" outside them is a number
        v.push(r"\b\d{4}-(?:0?[1-9]|1[0-2])-(?:0?[1-9]|[12]\d|3[01])\b".to_owned());
        // day.month or (EN) month/day: both parts a day number, the year 19xx/20xx
        v.push(
            r"\b(?:0?[1-9]|[12]\d|3[01])[./](?:0?[1-9]|[12]\d|3[01])[./](?:19|20)\d{2}\b"
                .to_owned(),
        );
        // clock time
        v.push(r"\b(?:[01]?\d|2[0-3]):[0-5]\d(?::[0-5]\d)?\b".to_owned());
    };
    let build = |lang: Lang| {
        let mut v = Vec::new();
        common(&mut v);
        match lang {
            Lang::Tr => {
                v.push(format!(
                    r"\b(?:0?[1-9]|[12]\d|3[01])\.?\s+{TR_MONTHS}(?:\s+\d{{4}}\b)?"
                ));
                v.push(format!(r"\b{TR_MONTHS}\s+\d{{4}}\b"));
                v.push(r"\b(?:19|20)\d{2}(?:['’]\p{L}+|\s+yıl\p{L}*)".to_owned());
            }
            Lang::En => {
                v.push(format!(
                    r"\b(?:0?[1-9]|[12]\d|3[01])(?:st|nd|rd|th)?\s+(?:of\s+)?{EN_MONTHS_DM}(?:,?\s+\d{{4}}\b)?"
                ));
                v.push(format!(
                    r"\b{EN_MONTHS}\s+(?:0?[1-9]|[12]\d|3[01])(?:st|nd|rd|th)?\b(?:,?\s+\d{{4}}\b)?"
                ));
                v.push(format!(r"\b{EN_MONTHS}\s+\d{{4}}\b"));
                v.push(r"\b(?:in|since|during|year)\s+(?:19|20)\d{2}\b".to_owned());
                v.push(r"\b(?:19|20)\d{2}['’]s\b".to_owned());
            }
        }
        v.iter()
            .map(|p| Regex::new(p).expect("comment mask regex"))
            .collect::<Vec<_>>()
    };
    match lang {
        Lang::Tr => TR.get_or_init(|| build(Lang::Tr)),
        Lang::En => EN.get_or_init(|| build(Lang::En)),
    }
}

fn num_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(
        &R,
        r"(?P<s1>[+\-−])?(?P<p1>%\s?|yüzde\s+)?(?P<s2>[+\-−])?(?P<num>\d+(?:[.,]\d+)*)(?P<p2>\s?%|\s+(?:puan\p{L}*|percent|per\s+cent|pct)\b)?(?P<mul>x\b|×|\s?(?:kat(?:ı|ın|ına|ını|ında|ından)?|times)\b)?",
    )
}

fn marker_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"\[(\d+(?:\s*[,;]\s*\d+)*)\]")
}

/// Removes every instrument symbol (longest first) from `s`.
#[must_use]
pub fn strip_symbols(s: &str, symbols: &[String]) -> String {
    let mut syms: Vec<&String> = symbols.iter().filter(|x| !x.is_empty()).collect();
    syms.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    let mut out = s.to_owned();
    for x in syms {
        out = out.replace(x.as_str(), " ");
    }
    out
}

/// Splits a digit run with separators into (integer digits, fraction digits).
fn split_num(s: &str, lang: Lang) -> (String, String) {
    let digits = |x: &str| x.chars().filter(char::is_ascii_digit).collect::<String>();
    let has_dot = s.contains('.');
    let has_comma = s.contains(',');
    match (has_dot, has_comma) {
        (false, false) => (s.to_owned(), String::new()),
        (true, true) => {
            let last = s.rfind(['.', ',']).unwrap_or(s.len());
            (digits(&s[..last]), s[last + 1..].to_owned())
        }
        _ => {
            let sep = if has_dot { '.' } else { ',' };
            if s.matches(sep).count() > 1 {
                return (digits(s), String::new());
            }
            let (a, b) = s.split_once(sep).unwrap_or((s, ""));
            let dec = match lang {
                Lang::Tr => ',',
                Lang::En => '.',
            };
            let thousands = sep != dec && b.len() == 3 && a.len() <= 3 && !a.starts_with('0');
            if thousands {
                (format!("{a}{b}"), String::new())
            } else {
                (a.to_owned(), b.to_owned())
            }
        }
    }
}

/// The numbers of a text (symbols, markers, dates, times and years removed).
#[must_use]
pub fn numbers(text: &str, lang: Lang, symbols: &[String]) -> Vec<Num> {
    numbers_with(text, lang, symbols, true)
}

fn numbers_with(text: &str, lang: Lang, symbols: &[String], masked: bool) -> Vec<Num> {
    let mut t = lower(&strip_symbols(text, symbols), lang);
    if masked {
        for m in masks(lang) {
            t = m.replace_all(&t, " ").into_owned();
        }
    }
    let mut out = Vec::new();
    for c in num_re().captures_iter(&t) {
        let whole = c.get(0).map_or(0, |m| m.start());
        let prev_alnum = t[..whole]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric);
        let sign_of = |m: regex::Match<'_>| m.as_str() != "+";
        let negative = match (c.name("s1"), c.name("s2")) {
            (_, Some(s)) => Some(sign_of(s)),
            (Some(s), None) if !prev_alnum => Some(sign_of(s)),
            _ => None,
        };
        let num = c.name("num").map_or("", |m| m.as_str());
        let (int, mut frac) = split_num(num, lang);
        // trailing zeros carry no value; dropping them keeps long but exact numbers
        // comparable (a number still too long is unsupported, never passed: fail-closed)
        if frac.len() > 18 {
            frac = frac.trim_end_matches('0').to_owned();
        }
        let kind = if c.name("p1").is_some() || c.name("p2").is_some() {
            NumKind::Percent
        } else if c.name("mul").is_some() {
            NumKind::Multiple
        } else {
            NumKind::Plain
        };
        let decimals = u8::try_from(frac.len()).unwrap_or(u8::MAX);
        let scaled = if frac.len() <= 18 {
            format!("{int}{frac}").parse::<u128>().ok()
        } else {
            None
        };
        let mag = format!("{int}.{}", if frac.is_empty() { "0" } else { &frac })
            .parse::<f64>()
            .unwrap_or(f64::NAN);
        let value = if negative == Some(true) { -mag } else { mag };
        let whole_m = c.get(0).map_or("", |m| m.as_str());
        let hyphen = c
            .name("s1")
            .filter(|_| prev_alnum && c.name("s2").is_none());
        let raw = hyphen
            .map_or(whole_m, |h| &whole_m[h.len()..])
            .trim()
            .to_owned();
        out.push(Num {
            raw,
            negative,
            scaled,
            decimals,
            value,
            kind,
        });
    }
    out
}

/// (R5B audit B1) Every number a reader could take from the text, read **without** the
/// date and time masks: each digit group, and each pair of adjacent groups joined as a
/// decimal (`5:23`, `5.23.2026`, `5/23` → 5.23 and 5,23; `23.2026` too). Symbols and
/// citation markers are removed first. Used where failing closed matters more than a
/// false alarm: the licence check and what a restricted reader may receive.
#[must_use]
pub fn raw_numbers(text: &str, lang: Lang, symbols: &[String]) -> Vec<Num> {
    static RUN: OnceLock<Regex> = OnceLock::new();
    let t = unmarked(text, lang, symbols);
    // the spans a reader could read a number from that `numbers` does not read as one:
    // what the date/time masks removed, and digit runs joined by `:` `/` or by two or
    // more `.`/`,` (an ordinary `5,87` is read whole by `numbers` and checked there)
    let mut spans: Vec<String> = masks(lang)
        .iter()
        .flat_map(|m| m.find_iter(&t).map(|x| x.as_str().to_owned()))
        .collect();
    spans.extend(
        re(&RUN, r"\d+(?:[:/]\d+)+(?:[.,:/]\d+)*|\d+(?:[.,]\d+){2,}")
            .find_iter(&t)
            .map(|x| x.as_str().to_owned()),
    );
    let mut out = Vec::new();
    for sp in spans {
        let groups: Vec<&str> = sp
            .split(|c: char| !c.is_ascii_digit())
            .filter(|g| !g.is_empty())
            .collect();
        for w in groups.windows(2) {
            for (sep, l) in [('.', Lang::En), (',', Lang::Tr)] {
                out.extend(numbers_unmasked(&format!("{}{sep}{}", w[0], w[1]), l));
            }
        }
        for g in groups {
            out.extend(numbers_unmasked(g, lang));
        }
    }
    out
}

/// (R5B audit B1) The numbers of a text with **no** date or time mask (symbols and
/// citation markers removed): `(5:23)` reads as 5 and 23, `5.23.2026` as one long
/// number. What a restricted reader may receive is decided on these.
#[must_use]
pub fn unmasked_numbers(text: &str, lang: Lang, symbols: &[String]) -> Vec<Num> {
    numbers_unmasked(&unmarked(text, lang, symbols), lang)
}

fn unmarked(text: &str, lang: Lang, symbols: &[String]) -> String {
    static MARK: OnceLock<Regex> = OnceLock::new();
    let t = lower(&strip_symbols(text, symbols), lang);
    re(&MARK, r"\[\d+(?:\s*[,;]\s*\d+)*\]")
        .replace_all(&t, " ")
        .into_owned()
}

fn numbers_unmasked(t: &str, lang: Lang) -> Vec<Num> {
    // `numbers` with no mask: the masks are the only difference.
    numbers_with(t, lang, &[], false)
}

/// (R5B audit B1) A decimal written in words ("beş virgül yirmi üç", "five point two
/// three"): the number scan cannot read it, so it is treated as a number nobody checked.
#[must_use]
pub fn spelled_decimal(text: &str, lang: Lang) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    const W: &str = "sıfır|bir|iki|üç|dört|beş|altı|yedi|sekiz|dokuz|on|yirmi|otuz|kırk|elli|altmış|yetmiş|seksen|doksan|yüz|bin|milyon|zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety|hundred|thousand|million";
    let p = format!(r"\b(?:(?:{W})\s+)*(?:{W})\s+(?:virgül|nokta|point)\s+(?:{W})\b");
    re(&R, &p)
        .find(&lower(text, lang))
        .map(|m| m.as_str().to_owned())
}

/// `|v|` rounded half away from zero to `d` decimals, as an integer × 10^d. A tiny
/// relative nudge makes a decimal half (`1.005`, stored as `1.00499…`) round up.
fn round_scaled(v: f64, d: u8) -> Option<u128> {
    let x = v.abs() * 10f64.powi(i32::from(d));
    if !x.is_finite() || x >= 1e30 {
        return None;
    }
    let r = (x + 0.5 + 1e-12 * x.max(1.0)).floor();
    // `r` is finite, non-negative and below 1e30 < u128::MAX.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some(r as u128)
}

fn pow10(n: u8) -> Option<u128> {
    10u128.checked_pow(u32::from(n))
}

/// Whether the written number `n` is the source value `v` (shown with `shown` decimals,
/// if known): `v` rounded (half away from zero) to the decimals `n` is written with
/// equals `n`; when `n` has more decimals than the source shows, `v` rounded to the
/// shown decimals and padded with zeros also counts (`44,320` for a shown `44,32`). A
/// percentage also matches the ratio `v × 100`. An explicit sign must agree with `v`'s
/// sign; an unsigned number matches `|v|` (the direction is in the words).
#[must_use]
pub fn number_matches(n: &Num, v: f64, shown: Option<u8>) -> bool {
    let Some(t) = n.scaled else {
        return false;
    };
    if !v.is_finite() {
        return false;
    }
    match n.negative {
        Some(true) if v >= 0.0 => return false,
        Some(false) if v < 0.0 => return false,
        _ => {}
    }
    let mut cands = vec![(v, shown)];
    if n.kind == NumKind::Percent {
        cands.push((v * 100.0, shown.map(|k| k.saturating_sub(2))));
    }
    cands.into_iter().any(|(c, k)| {
        if round_scaled(c, n.decimals) == Some(t) {
            return true;
        }
        match k {
            Some(k) if n.decimals > k => {
                let pad = pow10(n.decimals - k);
                match (round_scaled(c, k), pad) {
                    (Some(r), Some(p)) => r.checked_mul(p) == Some(t),
                    _ => false,
                }
            }
            _ => false,
        }
    })
}

/// Whether the written number `n` matches the withheld value `w` in any form: `w`,
/// `w × 100` or `w / 100`, sign and percent sign ignored (licence check; how a number is
/// written must not let it through). A number without exact digits (too long) is
/// compared as `f64`, with a relative tolerance of 1e-9.
#[must_use]
pub fn withheld_matches(n: &Num, w: f64) -> bool {
    if !w.is_finite() {
        return false;
    }
    let cands = [w, w * 100.0, w / 100.0];
    if n.scaled.is_none() {
        let x = n.value.abs();
        return cands
            .iter()
            .any(|c| (x - c.abs()).abs() <= 1e-9 * c.abs().max(1.0));
    }
    let plain = Num {
        negative: None,
        kind: NumKind::Plain,
        ..n.clone()
    };
    cands.iter().any(|&c| number_matches(&plain, c, None))
}

/// The citation markers of a sentence (`[1]`, `[1, 2]`), as written (1-based).
#[must_use]
pub fn markers(sentence: &str) -> Vec<usize> {
    marker_re()
        .captures_iter(sentence)
        .flat_map(|c| {
            c.get(1)
                .map_or("", |m| m.as_str())
                .split([',', ';'])
                .filter_map(|x| x.trim().parse::<usize>().ok())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn advice_list(lang: Lang) -> &'static [Regex] {
    static TR: OnceLock<Vec<Regex>> = OnceLock::new();
    static EN: OnceLock<Vec<Regex>> = OnceLock::new();
    let build = |pats: &[&str]| {
        pats.iter()
            .map(|p| Regex::new(&format!(r"\b(?:{p})\b")).expect("advice regex"))
            .collect::<Vec<_>>()
    };
    match lang {
        Lang::Tr => TR.get_or_init(|| build(ADVICE_TR)),
        Lang::En => EN.get_or_init(|| build(ADVICE_EN)),
    }
}

/// Advice words in a text (symbols removed): the TR list on the Turkish lower case, then
/// the EN list on the English lower case, whatever the comment's language.
#[must_use]
pub fn advice_hits(text: &str, symbols: &[String]) -> Vec<String> {
    let t = strip_symbols(text, symbols);
    [Lang::Tr, Lang::En]
        .into_iter()
        .flat_map(|lang| {
            let l = lower(&t, lang);
            advice_list(lang)
                .iter()
                .flat_map(|r| r.find_iter(&l).map(|m| m.as_str().to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Up words (TR). (M30 fix) "güçlendi" / "zayıfladı" are not direction words: they name
/// a story's stage (strengthened, weakened), in either direction ("aşağı yönlü kırılım
/// güçlendi"). Turkish suffix rule: only the listed endings ("artık" is not "art-",
/// "düşük" / "düşünce" are not "düş-", "gerilim" is not "gerile-").
pub const DIR_UP_TR: &[&str] = &[
    r"yüksel\w*",
    r"art(tı|tık|tılar|ıyor\w*|ış\w*|an|ar|acak\w*|abilir\w*|mış\w*|ma|ması\w*|arak)",
    r"çık(tı|tılar|ıyor\w*|ış\w*|an|ar|acak\w*|mış\w*|ması\w*|arak)",
    r"yukarı\w*",
    r"tırman\w*",
];
/// Down words (TR).
pub const DIR_DOWN_TR: &[&str] = &[
    r"düş(tü|tüler|üyor\w*|üş\w*|en|er|ecek\w*|müş\w*|me|mesi\w*|erek)",
    r"gerile\w*",
    r"geriliyor\w*",
    r"in(di|diler|iyor\w*|iş\w*|en|er|ecek\w*|miş\w*|mesi\w*|erek)",
    r"aşağı\w*",
    r"kayb\w*",
    r"kayıp\w*",
];
/// Up words (EN).
pub const DIR_UP_EN: &[&str] = &[
    r"ris(e|es|ing|en)",
    r"rose",
    r"up",
    r"upward\w*",
    r"higher",
    r"climb\w*",
    r"gain\w*",
];
/// Down words (EN).
pub const DIR_DOWN_EN: &[&str] = &[
    r"fall(s|ing|en)?",
    r"fell",
    r"down",
    r"downward\w*",
    r"lower",
    r"drop(s|ped|ping)?",
    r"declin\w*",
    r"los(e|es|ing|t|s)",
];

fn dir_lists(lang: Lang) -> &'static (Vec<Regex>, Vec<Regex>) {
    static TR: OnceLock<(Vec<Regex>, Vec<Regex>)> = OnceLock::new();
    static EN: OnceLock<(Vec<Regex>, Vec<Regex>)> = OnceLock::new();
    let build = |pats: &[&str]| {
        pats.iter()
            .map(|p| Regex::new(&format!(r"\b(?:{p})\b")).expect("direction regex"))
            .collect::<Vec<_>>()
    };
    match lang {
        Lang::Tr => TR.get_or_init(|| (build(DIR_UP_TR), build(DIR_DOWN_TR))),
        Lang::En => EN.get_or_init(|| (build(DIR_UP_EN), build(DIR_DOWN_EN))),
    }
}

/// Direction words of a text in its language (symbols removed): (up words, down words).
#[must_use]
pub fn direction_words(text: &str, lang: Lang, symbols: &[String]) -> (Vec<String>, Vec<String>) {
    let mut l = lower(&strip_symbols(text, symbols), lang);
    if lang == Lang::Tr {
        // (R5B, M28 usability) "para çıkışı" (money outflow) is a flow noun, not a rise.
        static FLOW: OnceLock<Regex> = OnceLock::new();
        l = re(
            &FLOW,
            r"\b(?:para|fon|sermaye|net|yabancı|nakit)\s+çıkış\w*",
        )
        .replace_all(&l, " ")
        .into_owned();
    }
    let (up, down) = dir_lists(lang);
    let find = |rs: &[Regex]| {
        rs.iter()
            .flat_map(|r| r.find_iter(&l).map(|m| m.as_str().to_owned()))
            .collect::<Vec<_>>()
    };
    (find(up), find(down))
}

fn reason(
    check: Check,
    code: &str,
    token: Option<String>,
    sentence: Option<usize>,
    hint: &str,
) -> Reason {
    Reason {
        check,
        code: code.to_owned(),
        token,
        sentence,
        hint: hint.to_owned(),
    }
}

/// The written form of an `f64` as a [`Num`] (unsigned), for comparing an external
/// value with withheld values at the precision it is given with.
fn num_of_value(v: f64) -> Num {
    let s = format!("{}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let decimals = u8::try_from(frac.len()).unwrap_or(u8::MAX);
    let scaled = if frac.len() <= 18 {
        format!("{int}{frac}").parse::<u128>().ok()
    } else {
        None
    };
    Num {
        raw: s.clone(),
        negative: None,
        scaled,
        decimals,
        value: v.abs(),
        kind: NumKind::Plain,
    }
}

fn other(lang: Lang) -> Lang {
    match lang {
        Lang::Tr => Lang::En,
        Lang::En => Lang::Tr,
    }
}

/// (R5B, M28 usability) What is removed from the text before the number and direction
/// scans: the instrument symbols and every id the draft may name (known, hidden or
/// cited) — "cap.SYN02.breakout.up.151" carries neither the number 151 nor the word
/// "up". Only id-shaped tokens (a letter and a separator, 5+ chars) are removed, so a
/// short or numeric id never masks a real number.
fn text_ids(d: &CommentDraft, ev: &Evidence) -> Vec<String> {
    let id_like = |x: &&String| {
        x.chars().count() >= 5
            && x.chars().any(char::is_alphabetic)
            && x.chars().any(|c| matches!(c, '.' | '-' | '_' | ':'))
    };
    let mut v: Vec<String> = ev.symbols.clone();
    v.extend(
        ev.known_ids
            .iter()
            .chain(&ev.hidden_ids)
            .chain(&d.cites)
            .filter(id_like)
            .cloned(),
    );
    v.sort();
    v.dedup();
    v
}

/// Validates a comment draft against the reader-projected evidence (PLAN §2.1).
///
/// # Errors
///
/// Every reason the draft is rejected, in a fixed order: citations, external quotes,
/// then sentence by sentence (advice, markers, numbers, per-sentence citation).
pub fn validate(d: &CommentDraft, ev: &Evidence, s: Strictness) -> Result<(), Vec<Reason>> {
    let lang = lang_of(d.lang);
    let strip = text_ids(d, ev);
    let numbers_on = matches!(s, Strictness::CitesNumbers | Strictness::PerSentence);
    let mut out = Vec::new();

    // 1. citations
    if d.cites.is_empty() {
        out.push(reason(
            Check::Cites,
            "no_cites",
            None,
            None,
            "Cite at least one fact, finding or capsule id the comment rests on.",
        ));
    }
    for id in &d.cites {
        if ev.known_ids.contains(id) {
            continue;
        }
        if ev.hidden_ids.contains(id) {
            out.push(reason(
                Check::License,
                "not_visible",
                Some(id.clone()),
                None,
                "This id cannot be cited for this reader; cite only ids the reader can see.",
            ));
        } else {
            out.push(reason(
                Check::Cites,
                "unknown_id",
                Some(id.clone()),
                None,
                "This id does not exist; cite ids returned by the Qapsule tools.",
            ));
        }
    }

    // external quotes
    for q in &d.external {
        if q.source.trim().is_empty() {
            out.push(reason(
                Check::Cites,
                "external_no_source",
                Some(q.quote.clone()),
                None,
                "Name the external source of every quote.",
            ));
        }
        let quote_nums: Vec<Num> = numbers(&q.quote, lang, &ev.symbols)
            .into_iter()
            .chain(numbers(&q.quote, other(lang), &ev.symbols))
            .collect();
        for &v in &q.values {
            let as_num = num_of_value(v);
            let leaked = ev.withheld.iter().any(|&w| withheld_matches(&as_num, w));
            if leaked {
                out.push(reason(
                    Check::License,
                    "licensed_external",
                    Some(format!("{v}")),
                    None,
                    "This value is licensed data the reader may not see; it cannot be passed as an external value.",
                ));
            } else if numbers_on && !quote_nums.iter().any(|t| number_matches(t, v, None)) {
                out.push(reason(
                    Check::Number,
                    "external_value_not_in_quote",
                    Some(format!("{v}")),
                    None,
                    "Every external value must appear in its quote text.",
                ));
            }
        }
    }

    // per sentence
    let ext_values: Vec<f64> = d.external.iter().flat_map(|q| q.values.clone()).collect();
    for (i, sent) in sentences(&d.text).iter().enumerate() {
        for h in advice_hits(sent, &ev.symbols) {
            out.push(reason(
                Check::Advice,
                "advice_word",
                Some(h),
                Some(i),
                "Describe what happened; do not advise (no buy/sell, opportunity, target price, should).",
            ));
        }
        let mut valid_markers = 0usize;
        for m in markers(sent) {
            if m == 0 || m > d.cites.len() {
                out.push(reason(
                    Check::Cites,
                    "bad_marker",
                    Some(format!("[{m}]")),
                    Some(i),
                    "A marker [n] must point to the n-th id in cites (1-based).",
                ));
            } else {
                valid_markers += 1;
            }
        }
        let nums = numbers(sent, lang, &strip);
        for n in &nums {
            let visible = ev
                .values
                .iter()
                .any(|cv| number_matches(n, cv.value, Some(cv.decimals)));
            if visible {
                continue;
            }
            if ev.withheld.iter().any(|&w| withheld_matches(n, w)) {
                out.push(reason(
                    Check::License,
                    "licensed_value",
                    Some(n.raw.clone()),
                    Some(i),
                    "This number is licensed data the reader may not see; remove it.",
                ));
            } else if n.scaled.is_none() {
                // fail-closed: a number that cannot be read exactly never passes
                out.push(reason(
                    Check::Number,
                    "unsupported_number",
                    Some(n.raw.clone()),
                    Some(i),
                    "This number is too long to check; write it at the precision of its source.",
                ));
            } else if numbers_on && !ext_values.iter().any(|&v| number_matches(n, v, None)) {
                out.push(reason(
                    Check::Number,
                    "unsupported_number",
                    Some(n.raw.clone()),
                    Some(i),
                    "Every number must be a value of a cited item or of a named external quote, written at its precision; derived numbers are not allowed.",
                ));
            }
        }
        if s == Strictness::PerSentence && !nums.is_empty() && valid_markers == 0 {
            out.push(reason(
                Check::Sentence,
                "uncited_sentence",
                None,
                Some(i),
                "A sentence with a number must carry a citation marker such as [1].",
            ));
        }
    }

    // 5b. (R5B audit B1) licence, fail-closed: a licensed value must not get through hidden
    //     in a "date" or "time", split across digit groups, or written in words.
    //     For a writer with withheld numbers in scope, a hiding form whose value the
    //     reader does not see is refused whether it is licensed or made up: the writer
    //     cannot tell the two apart (both are told `unsupported_number`, §25.4).
    if !ev.withheld.is_empty() && !out.iter().any(|r| r.code == "licensed_value") {
        let hidden = sentences(&d.text).iter().enumerate().find_map(|(i, sent)| {
            raw_numbers(sent, lang, &strip)
                .into_iter()
                .find(|n| {
                    !ev.values
                        .iter()
                        .any(|cv| number_matches(n, cv.value, Some(cv.decimals)))
                })
                .map(|n| (i, n))
        });
        if let Some((i, n)) = hidden {
            let licensed = ev.withheld.iter().any(|&w| withheld_matches(&n, w));
            out.push(if licensed {
                reason(
                    Check::License,
                    "licensed_value",
                    Some(n.raw),
                    Some(i),
                    "This number is licensed data the reader may not see; remove it.",
                )
            } else {
                reason(
                    Check::Number,
                    "unsupported_number",
                    Some(n.raw),
                    Some(i),
                    "A date or time here must not carry numbers the reader cannot see; leave it out.",
                )
            });
        }
    }
    // (every writer alike, so it tells nothing about what is withheld)
    if let Some(w) = spelled_decimal(&d.text, lang) {
        out.push(reason(
            Check::Number,
            "spelled_number",
            Some(w),
            None,
            "Write numbers in digits so they can be checked against the evidence.",
        ));
    }

    // 6. direction
    if numbers_on {
        let dirs: Vec<Direction> = ev
            .directions
            .iter()
            .filter(|c| c.direction != Direction::Neutral && d.cites.contains(&c.id))
            .map(|c| c.direction)
            .collect();
        let want = dirs
            .first()
            .copied()
            .filter(|w| dirs.iter().all(|x| x == w));
        if let Some(want) = want {
            let (up, down) = direction_words(&d.text, lang, &strip);
            let opposite = match want {
                Direction::Up if up.is_empty() => down.first(),
                Direction::Down if down.is_empty() => up.first(),
                _ => None,
            };
            if let Some(word) = opposite {
                let sentence = sentences(&d.text).iter().position(|s| {
                    let (u, w) = direction_words(s, lang, &strip);
                    u.contains(word) || w.contains(word)
                });
                out.push(reason(
                    Check::Direction,
                    "wrong_direction",
                    Some(word.clone()),
                    sentence,
                    "The text states the opposite direction of the cited items; describe the direction they show.",
                ));
            }
        }
    }

    if out.is_empty() {
        Ok(())
    } else {
        Err(out)
    }
}
