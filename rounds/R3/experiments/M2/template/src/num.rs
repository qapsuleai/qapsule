//! Number formatting. Every number printed comes from an engine slot and is rounded with
//! the same rule the harness checks (`round(v, k) = (v·10^k).round() / 10^k`), so a token
//! with `k` decimals always equals `round(v, k)`.

use qp_capsule::Lang;

/// How a value is printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cat {
    /// Prices and price-derived levels: always 2 decimals (`148,20`).
    Price,
    /// Oscillators (RSI): up to 1 decimal, trailing zero dropped (`70`, `71,4`).
    Osc,
    /// Ratios (band width, thresholds): up to 4 decimals, trailing zeros dropped.
    Ratio,
    /// Counts and flows (volume, net flow): integer, grouped (`2.340.000`).
    Count,
}

/// The harness rounding rule.
#[must_use]
pub fn round_to(x: f64, d: i32) -> f64 {
    let p = 10f64.powi(d);
    let r = (x * p).round() / p;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

fn group(int: &str, sep: char) -> String {
    let cs: Vec<char> = int.chars().collect();
    let mut out = String::new();
    for (i, c) in cs.iter().enumerate() {
        if i > 0 && (cs.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(*c);
    }
    out
}

/// Formats `|v|` (sign is carried by words: "net satış") in `lang`.
#[must_use]
pub fn fmt(v: f64, cat: Cat, lang: Lang) -> String {
    let (dec, trim) = match cat {
        Cat::Price => (2, false),
        Cat::Osc => (1, true),
        Cat::Ratio => (4, true),
        Cat::Count => (0, false),
    };
    let r = round_to(v.abs(), dec);
    let mut s = format!("{r:.prec$}", prec = usize::try_from(dec).unwrap_or(0));
    if trim && s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_owned(), Some(f.to_owned())),
        None => (s, None),
    };
    let (thou, mark) = match lang {
        Lang::Tr => ('.', ','),
        Lang::En => (',', '.'),
    };
    let int = if int.len() > 3 {
        group(&int, thou)
    } else {
        int
    };
    match frac {
        Some(f) => format!("{int}{mark}{f}"),
        None => int,
    }
}

/// An integer count (bars, ledger), plain digits.
#[must_use]
pub fn int(n: u64) -> String {
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt(148.2, Cat::Price, Lang::Tr), "148,20");
        assert_eq!(fmt(148.2, Cat::Price, Lang::En), "148.20");
        assert_eq!(fmt(1250.5, Cat::Price, Lang::Tr), "1.250,50");
        assert_eq!(fmt(1250.5, Cat::Price, Lang::En), "1,250.50");
        assert_eq!(fmt(70.0, Cat::Osc, Lang::Tr), "70");
        assert_eq!(fmt(71.4, Cat::Osc, Lang::Tr), "71,4");
        assert_eq!(fmt(0.06, Cat::Ratio, Lang::Tr), "0,06");
        assert_eq!(fmt(0.0582, Cat::Ratio, Lang::En), "0.0582");
        assert_eq!(fmt(-2_340_000.0, Cat::Count, Lang::Tr), "2.340.000");
        assert_eq!(fmt(780_000.0, Cat::Count, Lang::En), "780,000");
        assert_eq!(fmt(55.0, Cat::Osc, Lang::En), "55");
    }
}
