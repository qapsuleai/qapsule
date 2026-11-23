//! Numbers and dates in the reader's language: Turkish writes a decimal comma and groups
//! thousands with a dot ("2.925,95", "%1,28"), English the reverse ("2,925.95", "1.28%");
//! large numbers compact in one unit per axis ("1,2 Mn" / "1.2M") so they stay inside the
//! gutter; month abbreviations (OV-6: "TR/EN ay adları çekirdekte"). Integer civil dates.

use qp_schema::protocol::Language;

use qp_look::detmath::pow10_int;

/// Magnitudes from here on are written compact (a volume axis, not a price).
pub(super) const COMPACT_FROM: f64 = 100_000.0;

const MONTHS_TR: [&str; 12] = [
    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
];
const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// Compact units: size, Turkish suffix (bin, milyon, milyar, trilyon), English suffix.
const UNITS: [(f64, &str, &str); 4] = [
    (1e12, " Tn", "T"),
    (1e9, " Mr", "B"),
    (1e6, " Mn", "M"),
    (1e3, " B", "K"),
];
/// Most decimals a label shows.
const MAX_DECIMALS: usize = 6;
/// Decimals of a price (BIST ticks) unless its step needs more.
const PRICE_DECIMALS: usize = 2;

/// Milliseconds in a day.
pub(super) const DAY_MS: i64 = 86_400_000;

/// The month abbreviation of month `m` (1–12).
pub(super) fn month(m: i64, lang: Language) -> &'static str {
    let names = if lang == Language::En {
        &MONTHS_EN
    } else {
        &MONTHS_TR
    };
    names[usize::try_from(m - 1).unwrap_or(0).min(11)]
}

/// (year, month, day) of a time in ms (Howard Hinnant's `civil_from_days`).
pub(super) fn civil(t: i64) -> (i64, i64, i64) {
    let z = t.div_euclid(DAY_MS) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// "09:30".
pub(super) fn hm(t: i64) -> String {
    let min = t.rem_euclid(DAY_MS) / 60_000;
    format!("{:02}:{:02}", min / 60, min % 60)
}

/// "3 Tem" / "3 Jul".
pub(super) fn day_month(t: i64, lang: Language) -> String {
    let (_, m, d) = civil(t);
    format!("{d} {}", month(m, lang))
}

/// The date pill: "3 Tem 2025", intraday "3 Tem 09:30".
pub(super) fn pill_date(t: i64, intraday: bool, lang: Language) -> String {
    let tail = if intraday {
        hm(t)
    } else {
        civil(t).0.to_string()
    };
    format!("{} {tail}", day_month(t, lang))
}

/// Fewest decimals that write `step` exactly (at most [`MAX_DECIMALS`]).
fn decimals(step: f64) -> usize {
    (0..MAX_DECIMALS)
        .find(|d| {
            #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
            let m = step * pow10_int(*d as i32);
            (m - m.round()).abs() < 1e-6 * m.abs().max(1.0)
        })
        .unwrap_or(MAX_DECIMALS)
}

/// `v` with `dec` decimals, grouped, in `lang`.
fn fixed(v: f64, dec: usize, lang: Language) -> String {
    let s = format!("{:.*}", dec, v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((s.as_str(), ""));
    let (group, point) = if lang == Language::En {
        (',', '.')
    } else {
        ('.', ',')
    };
    let mut out = String::new();
    if v < 0.0 && s.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
        out.push('-');
    }
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(group);
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push(point);
        out.push_str(frac);
    }
    out
}

/// A signed change: "+1,23"; a percentage: "+%1,28" (Turkish puts `%` first) / "+1.28%".
pub(super) fn signed(v: f64, dec: usize, pct: bool, lang: Language) -> String {
    let sign = if v < 0.0 { "-" } else { "+" };
    let n = fixed(v.abs(), dec, lang);
    match (pct, lang) {
        (false, _) => format!("{sign}{n}"),
        (true, Language::En) => format!("{sign}{n}%"),
        (true, _) => format!("{sign}%{n}"),
    }
}

/// One axis's or one tag's number format: fixed decimals, or compact in one unit.
#[derive(Debug, Clone, Copy)]
pub(super) struct NumFmt {
    lang: Language,
    dec: usize,
    unit: Option<(f64, &'static str)>,
}

impl NumFmt {
    /// For values up to `mag` in magnitude, `step` apart (`None`: a single value).
    pub(super) fn new(mag: f64, step: Option<f64>, lang: Language) -> Self {
        let unit = UNITS
            .iter()
            .find(|u| mag >= COMPACT_FROM && mag >= u.0)
            .map(|u| (u.0, if lang == Language::En { u.2 } else { u.1 }));
        let dec = match (unit, step) {
            (Some((size, _)), Some(s)) => decimals(s / size).min(2),
            (Some(_), None) => 1,
            (None, Some(s)) => decimals(s).max(PRICE_DECIMALS),
            (None, None) => PRICE_DECIMALS,
        };
        NumFmt { lang, dec, unit }
    }

    /// `v` in this format.
    pub(super) fn fmt(&self, v: f64) -> String {
        match self.unit {
            Some(_) if v == 0.0 => "0".to_owned(),
            Some((size, sfx)) => format!("{}{sfx}", fixed(v / size, self.dec, self.lang)),
            None => fixed(v, self.dec, self.lang),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_dates_by_language() {
        let (tr, en) = (Language::Tr, Language::En);
        assert_eq!(NumFmt::new(3000.0, None, tr).fmt(2925.95), "2.925,95");
        assert_eq!(NumFmt::new(3000.0, None, en).fmt(-2925.95), "-2,925.95");
        assert_eq!(NumFmt::new(2.5e6, Some(5e5), tr).fmt(1.5e6), "1,5 Mn");
        assert_eq!(NumFmt::new(2.5e6, Some(1e6), en).fmt(2e6), "2M");
        assert_eq!(NumFmt::new(100.0, Some(0.025), tr).fmt(0.075), "0,075");
        assert_eq!(signed(-0.0128 * 100.0, 2, true, tr), "-%1,28");
        assert_eq!(signed(22.43, 2, false, en), "+22.43");
        // 2025-07-03 00:00 UTC.
        assert_eq!(civil(1_751_500_800_000), (2025, 7, 3));
        assert_eq!(pill_date(1_751_500_800_000, false, tr), "3 Tem 2025");
        assert_eq!(pill_date(1_751_535_000_000, true, en), "3 Jul 09:30");
        assert_eq!(civil(0), (1970, 1, 1));
    }
}
