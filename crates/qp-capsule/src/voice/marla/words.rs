//! Marla's numbers and words: number words, parameters, spans and Turkish case suffixes.

use super::*;

// ---------------------------------------------------------------------------
// Numbers and words
// ---------------------------------------------------------------------------

pub(super) fn n(v: f64, cat: Cat, lang: Lang) -> String {
    fmt(v, cat, lang)
}

pub(in crate::voice) fn words_tr(x: u32) -> String {
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

pub(in crate::voice) fn words_en(x: u32) -> String {
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
pub(super) fn param(x: usize, lang: Lang, red: bool) -> String {
    let x = u32::try_from(x).unwrap_or(u32::MAX);
    match (red, lang) {
        (false, _) => x.to_string(),
        (true, Lang::Tr) => words_tr(x),
        (true, Lang::En) => words_en(x),
    }
}

/// A multiplier rule ("2,5 katı", "two and a half times"): digits, or words when
/// redacted.
pub(super) fn mult(x: f64, lang: Lang, red: bool) -> String {
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
pub(super) fn pct(x: f64, thr: f64, lang: Lang) -> String {
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
pub(super) fn abl(w: &str) -> String {
    format!("{}n", Word::number(w).loc())
}

pub(super) fn dat(w: &str) -> String {
    Word::number(w).dat()
}

/// "20 günün" / "20 saatin" (TR genitive of a counted period).
pub(super) fn tr_gen(k: &str, u: Unit) -> String {
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
pub(super) fn tr_adj(k: &str, u: Unit) -> String {
    format!("{k} {}", u.words().tr_adj)
}

/// "20-day" / "20-hour" (redacted: "twenty-day").
pub(super) fn en_adj(k: &str, u: Unit) -> String {
    format!("{k}-{}", u.words().en)
}

/// The horizon span: TR "10 işlem günü", EN "10 trading days".
pub(super) fn span(k: u32, u: Unit, lang: Lang, red: bool) -> String {
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
pub(super) fn span_loc_tr(k: u32, u: Unit, red: bool) -> String {
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
pub(super) fn that(u: Unit, lang: Lang) -> &'static str {
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
