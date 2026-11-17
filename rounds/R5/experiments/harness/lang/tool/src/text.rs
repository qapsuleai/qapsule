//! Text helpers shared by the scans and the scorer: Turkish-aware lower-casing,
//! sentence splitting, number / time / date / relative-day extraction. Pure functions.

use std::sync::OnceLock;

use regex::Regex;

use crate::lang::Lang;

/// Lower-case with the Turkish dotted/dotless i rule (`I` → `ı`, `İ` → `i`) for both
/// languages (an English text has no `İ`; `I` in English becomes `ı`, which no English
/// pattern relies on — patterns are written in lower case without `i`-sensitive words
/// where it matters, and English text is lower-cased with the plain rule instead).
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

/// Removes every occurrence of the instrument symbol (its digits are not data, e.g.
/// `SYN03`) and of the redacted fallback subject.
#[must_use]
pub fn strip_symbol(s: &str, symbol: &str) -> String {
    if symbol.is_empty() {
        return s.to_owned();
    }
    s.replace(symbol, " ")
}

/// Splits into sentences: a break after `.`, `;`, `!`, `?` followed by whitespace or
/// the end (a decimal point is never followed by whitespace). Empty pieces dropped.
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

/// A sentence normalised for the repetition check: lower case, letters/digits only,
/// single spaces.
#[must_use]
pub fn norm_sentence(s: &str, lang: Lang) -> String {
    let l = lower(s, lang);
    let mapped: String = l
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("valid regex"))
}

static TIME_RE: OnceLock<Regex> = OnceLock::new();
static NUM_RE: OnceLock<Regex> = OnceLock::new();

/// Clock times `HH:MM` in the text (as `(h, m)`).
#[must_use]
pub fn times(s: &str) -> Vec<(u32, u32)> {
    re(&TIME_RE, r"\b(\d{1,2}):(\d{2})\b")
        .captures_iter(s)
        .filter_map(|c| Some((c[1].parse().ok()?, c[2].parse().ok()?)))
        .collect()
}

/// Numbers in the text by the language's convention (TR: `.` thousands, `,` decimal;
/// EN: `,` thousands, `.` decimal). Clock times are removed first. `%` is ignored.
#[must_use]
pub fn numbers(s: &str, lang: Lang) -> Vec<f64> {
    let no_time = re(&TIME_RE, r"\b(\d{1,2}):(\d{2})\b").replace_all(s, " ");
    re(&NUM_RE, r"\d+(?:[.,]\d+)*")
        .find_iter(&no_time)
        .filter_map(|m| {
            let raw = m.as_str();
            let canon = match lang {
                Lang::Tr => raw.replace('.', "").replace(',', "."),
                Lang::En => raw.replace(',', ""),
            };
            canon.parse::<f64>().ok()
        })
        .collect()
}

/// Whether `xs` holds a number close to `v`: within 0,6 % of `|v|` or 0,011 absolute
/// (prices are written with two decimals; the product rounds by category).
#[must_use]
pub fn has_close(xs: &[f64], v: f64) -> bool {
    if !v.is_finite() {
        return false;
    }
    let tol = (0.006 * v.abs()).max(0.011);
    xs.iter().any(|x| (x - v).abs() <= tol)
}

const TR_MONTHS: [&str; 12] = [
    "ocak", "şubat", "mart", "nisan", "mayıs", "haziran", "temmuz", "ağustos", "eylül", "ekim",
    "kasım", "aralık",
];
const EN_MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

static TR_DATE: OnceLock<Regex> = OnceLock::new();
static EN_DATE: OnceLock<Regex> = OnceLock::new();
static EN_DATE2: OnceLock<Regex> = OnceLock::new();

/// Calendar dates written as "15 Mayıs(’ta) [2025]" / "15 May [2025]" / "May 15[, 2025]":
/// `(day, month 1..=12, year if written)`.
#[must_use]
pub fn dates(s: &str, lang: Lang) -> Vec<(u32, u32, Option<i32>)> {
    let l = lower(s, lang);
    let mut out = Vec::new();
    match lang {
        Lang::Tr => {
            let pat = format!(
                r"\b(\d{{1,2}}) ({})\w*(?:'\w+)?(?: (\d{{4}}))?",
                TR_MONTHS.join("|")
            );
            for c in re(&TR_DATE, &pat).captures_iter(&l) {
                let m = TR_MONTHS.iter().position(|x| *x == &c[2]).unwrap_or(0);
                out.push((
                    c[1].parse().unwrap_or(0),
                    u32::try_from(m + 1).unwrap_or(0),
                    c.get(3).and_then(|y| y.as_str().parse().ok()),
                ));
            }
        }
        Lang::En => {
            let pat = format!(
                r"\b(\d{{1,2}}) ({})\b(?:,? (\d{{4}}))?",
                EN_MONTHS.join("|")
            );
            for c in re(&EN_DATE, &pat).captures_iter(&l) {
                let m = EN_MONTHS.iter().position(|x| *x == &c[2]).unwrap_or(0);
                out.push((
                    c[1].parse().unwrap_or(0),
                    u32::try_from(m + 1).unwrap_or(0),
                    c.get(3).and_then(|y| y.as_str().parse().ok()),
                ));
            }
            let pat2 = format!(
                r"\b({}) (\d{{1,2}})\b(?:,? (\d{{4}}))?",
                EN_MONTHS.join("|")
            );
            for c in re(&EN_DATE2, &pat2).captures_iter(&l) {
                let m = EN_MONTHS.iter().position(|x| *x == &c[1]).unwrap_or(0);
                out.push((
                    c[2].parse().unwrap_or(0),
                    u32::try_from(m + 1).unwrap_or(0),
                    c.get(3).and_then(|y| y.as_str().parse().ok()),
                ));
            }
        }
    }
    out
}

/// Unit of a relative time expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelUnit {
    /// Trading days (bugün = 0, dün = 1, "N işlem günü önce").
    TradingDay,
    /// Bars ("N bar önce").
    Bar,
}

static TR_REL: OnceLock<Regex> = OnceLock::new();
static EN_REL: OnceLock<Regex> = OnceLock::new();

/// Relative day expressions: "bugün"/"today" → 0, "dün"/"yesterday" → 1 (trading days),
/// "N işlem günü önce"/"N trading days ago", "N bar önce"/"N bars ago". Weekday names,
/// calendar-day counts and hour counts are not parsed (not checked).
#[must_use]
pub fn relative_days(s: &str, lang: Lang) -> Vec<(RelUnit, u64)> {
    let l = lower(s, lang);
    let mut out = Vec::new();
    let r = match lang {
        Lang::Tr => re(
            &TR_REL,
            r"\b(?:(bugün)|(dün)|(\d+) işlem günü önce|(\d+) bar önce)",
        ),
        Lang::En => re(
            &EN_REL,
            r"\b(?:(today)|(yesterday)|(\d+) trading days? ago|(\d+) bars? ago)",
        ),
    };
    for c in r.captures_iter(&l) {
        if c.get(1).is_some() {
            out.push((RelUnit::TradingDay, 0));
        } else if c.get(2).is_some() {
            out.push((RelUnit::TradingDay, 1));
        } else if let Some(n) = c.get(3) {
            out.push((RelUnit::TradingDay, n.as_str().parse().unwrap_or(0)));
        } else if let Some(n) = c.get(4) {
            out.push((RelUnit::Bar, n.as_str().parse().unwrap_or(0)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_by_language() {
        assert_eq!(
            numbers("kapanış 1.234,56; RSI 70", Lang::Tr),
            vec![1234.56, 70.0]
        );
        assert_eq!(
            numbers("close 1,234.56; RSI 70.", Lang::En),
            vec![1234.56, 70.0]
        );
        assert_eq!(
            numbers("74'te 36 tuttu, saat 17:00", Lang::Tr),
            vec![74.0, 36.0]
        );
        assert_eq!(times("dün 17:00'de"), vec![(17, 0)]);
        assert!(has_close(&[148.9], 148.904));
        assert!(!has_close(&[149.9], 148.904));
    }

    #[test]
    fn sentences_and_norm() {
        let s = sentences("Fiyat 2,5 kat. Hâlâ geçerli. Close 2.5 times; still holds");
        assert_eq!(
            s,
            vec![
                "Fiyat 2,5 kat.",
                "Hâlâ geçerli.",
                "Close 2.5 times;",
                "still holds"
            ]
        );
        assert_eq!(norm_sentence("Hâlâ Geçerli!", Lang::Tr), "hâlâ geçerli");
        assert_eq!(lower("IŞIK İzmir", Lang::Tr), "ışık izmir");
    }

    #[test]
    fn dates_and_relative() {
        assert_eq!(
            dates("Kapandı: 15 Mayıs'ta gerçekleşti", Lang::Tr),
            vec![(15, 5, None)]
        );
        assert_eq!(
            dates("on 15 September 2025", Lang::En),
            vec![(15, 9, Some(2025))]
        );
        assert_eq!(
            relative_days("3 işlem günü önce ve dün; 6 bar önce", Lang::Tr),
            vec![
                (RelUnit::TradingDay, 3),
                (RelUnit::TradingDay, 1),
                (RelUnit::Bar, 6)
            ]
        );
        assert_eq!(
            relative_days("Today, 2 bars ago", Lang::En),
            vec![(RelUnit::TradingDay, 0), (RelUnit::Bar, 2)]
        );
    }
}
