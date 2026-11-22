//! Numbers in a comment's text: how they are written, masked (dates, times, ids) and
//! matched against engine values.

use super::*;

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

#[allow(clippy::expect_used)]
pub(super) fn re(cell: &'static OnceLock<Regex>, p: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(p).expect("comment regex"))
}

const TR_MONTHS: &str = r"(?:(?:ocak|şubat|mart|nisan|mayıs|haziran|temmuz|ağustos|eylül|ekim|kasım|aralık)\p{L}*(?:['’]\p{L}+)?|(?:oca|şub|mar|nis|may|haz|tem|ağu|eyl|eki|kas|ara)\b\.?)";
/// EN months after a day: "may" only with a year ("12 may rise" is not a date).
const EN_MONTHS_DM: &str = r"(?:(?:january|february|march|april|june|july|august|september|october|november|december)\b|(?:jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)\b\.?|may\s+\d{4}\b)";
const EN_MONTHS: &str = r"(?:(?:january|february|march|april|may|june|july|august|september|october|november|december)\b|(?:jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)\b\.?)";

#[allow(clippy::expect_used)]
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

const NUMBER_WORDS: &str = "sıfır|sifir|bir|iki|üç|uc|dört|dort|beş|bes|altı|alti|yedi|sekiz|dokuz|on|yirmi|otuz|kırk|kirk|elli|altmış|altmis|yetmiş|yetmis|seksen|doksan|yüz|yuz|bin|milyon|milyar|yarım|yarim|çeyrek|ceyrek|zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety|hundred|thousand|million|billion|half|quarter|dozen";

/// Both lowercasings (TR dotted/dotless I and EN), with combining marks dropped, so a
/// word matches however its I was written.
fn folded(text: &str) -> [String; 2] {
    let strip = |s: String| {
        s.chars()
            .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
            .collect()
    };
    [strip(lower(text, Lang::Tr)), strip(lower(text, Lang::En))]
}

/// (R5B audit B1) A decimal written in words ("beş virgül yirmi üç", "on beş buçuk",
/// "five point two three", "… dot …", "… comma …"): the number scan cannot read it, so
/// every writer is refused it. `_lang` is kept for the signature; both lowercasings are
/// tried.
#[must_use]
pub fn spelled_decimal(text: &str, _lang: Lang) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    let w = NUMBER_WORDS;
    let p = format!(
        r"\b(?:(?:{w})[\s-]+)*(?:{w})[\s-]+(?:(?:virgül|virgul|nokta|point|dot|comma)[\s-]+(?:{w})|buçuk|bucuk|and a half)\b"
    );
    let r = re(&R, &p);
    folded(text)
        .iter()
        .find_map(|t| r.find(t).map(|m| m.as_str().to_owned()))
}

/// (R5B security review) Two or more number words in a row ("beş yirmi üç", "five
/// twenty three"): a restricted reader never receives such a text (fail-closed; a
/// writer may still use them where numbers in words are harmless).
#[must_use]
pub fn number_word_run(text: &str) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    let w = NUMBER_WORDS;
    let r = re(&R, &format!(r"\b(?:{w})(?:[\s-]+(?:{w}))+\b"));
    folded(text)
        .iter()
        .find_map(|t| r.find(t).map(|m| m.as_str().to_owned()))
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
