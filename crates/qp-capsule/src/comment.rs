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
        v.push(r"\b\d{4}-\d{1,2}-\d{1,2}\b".to_owned());
        v.push(r"\b\d{1,2}[./]\d{1,2}[./]\d{4}\b".to_owned());
        // clock time
        v.push(r"\b\d{1,2}:\d{2}(?::\d{2})?\b".to_owned());
    };
    let build = |lang: Lang| {
        let mut v = Vec::new();
        common(&mut v);
        match lang {
            Lang::Tr => {
                v.push(format!(r"\b\d{{1,2}}\.?\s+{TR_MONTHS}(?:\s+\d{{4}}\b)?"));
                v.push(format!(r"\b{TR_MONTHS}\s+\d{{4}}\b"));
                v.push(r"\b(?:19|20)\d{2}(?:['’]\p{L}+|\s+yıl\p{L}*)".to_owned());
            }
            Lang::En => {
                v.push(format!(
                    r"\b\d{{1,2}}(?:st|nd|rd|th)?\s+(?:of\s+)?{EN_MONTHS_DM}(?:,?\s+\d{{4}}\b)?"
                ));
                v.push(format!(
                    r"\b{EN_MONTHS}\s+\d{{1,2}}(?:st|nd|rd|th)?\b(?:,?\s+\d{{4}}\b)?"
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
        r"(?P<s1>[+\-−])?(?P<p1>%\s?)?(?P<s2>[+\-−])?(?P<num>\d+(?:[.,]\d+)*)(?P<p2>\s?%)?(?P<mul>x\b|×|\s?(?:kat(?:ı|ın|ına|ını|ında|ından)?|times)\b)?",
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
    let mut t = lower(&strip_symbols(text, symbols), lang);
    for m in masks(lang) {
        t = m.replace_all(&t, " ").into_owned();
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
        let (int, frac) = split_num(num, lang);
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

/// Advice words in a text (lower-cased, symbols removed), in list order.
#[must_use]
pub fn advice_hits(text: &str, lang: Lang, symbols: &[String]) -> Vec<String> {
    let l = lower(&strip_symbols(text, symbols), lang);
    advice_list(lang)
        .iter()
        .flat_map(|r| r.find_iter(&l).map(|m| m.as_str().to_owned()))
        .collect()
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
fn num_of_value(v: f64) -> Option<Num> {
    let s = format!("{}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let decimals = u8::try_from(frac.len()).ok()?;
    let scaled = if frac.len() <= 18 {
        format!("{int}{frac}").parse::<u128>().ok()
    } else {
        None
    };
    Some(Num {
        raw: s.clone(),
        negative: None,
        scaled,
        decimals,
        value: v.abs(),
        kind: NumKind::Plain,
    })
}

fn other(lang: Lang) -> Lang {
    match lang {
        Lang::Tr => Lang::En,
        Lang::En => Lang::Tr,
    }
}

/// Validates a comment draft against the reader-projected evidence (PLAN §2.1).
///
/// # Errors
///
/// Every reason the draft is rejected, in a fixed order: citations, external quotes,
/// then sentence by sentence (advice, markers, numbers, per-sentence citation).
pub fn validate(d: &CommentDraft, ev: &Evidence, s: Strictness) -> Result<(), Vec<Reason>> {
    let lang = lang_of(d.lang);
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
            let leaked = num_of_value(v)
                .is_some_and(|n| ev.withheld.iter().any(|&w| number_matches(&n, w, None)));
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
        for h in advice_hits(sent, lang, &ev.symbols) {
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
        let nums = numbers(sent, lang, &ev.symbols);
        for n in &nums {
            let visible = ev
                .values
                .iter()
                .any(|cv| number_matches(n, cv.value, Some(cv.decimals)));
            if visible {
                continue;
            }
            if ev.withheld.iter().any(|&w| number_matches(n, w, None)) {
                out.push(reason(
                    Check::License,
                    "licensed_value",
                    Some(n.raw.clone()),
                    Some(i),
                    "This number is licensed data the reader may not see; remove it.",
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

    if out.is_empty() {
        Ok(())
    } else {
        Err(out)
    }
}
