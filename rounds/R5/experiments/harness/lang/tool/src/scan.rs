//! Deterministic scans (SPEC §2). Every scan's lists and limits are constants here and
//! are quoted verbatim in SPEC §2 with their reasons; changing one is a SPEC change
//! (dated appendix entry, applied to every language).
//!
//! Patterns are lower-case regexes matched against the lower-cased text (Turkish i rule
//! for TR) with Unicode word boundaries (`\b`); Turkish suffixes are allowed only where a
//! pattern lists them, so a short stem never matches a different word ("bar" ≠ "barış",
//! "olgu" ≠ "olgun", "al" ≠ "alım").

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::key::Key;
use crate::lang::{GlossEntry, Lang, Surfaces};
use crate::text::{
    dates, has_close, lower, norm_sentence, numbers, relative_days, sentences, strip_symbol, times,
    RelUnit,
};

/// §2.1 Vague phrases, always forbidden.
pub const VAGUE_HARD_TR: &[&str] = &[
    r"önemli bir eşi\w*",
    r"önemli eşi\w*",
    r"önemli (bir )?seviye\w*",
    r"kritik (bir )?(seviye|eşi|nokta)\w*",
    r"kilit (bir )?(seviye|eşi|nokta)\w*",
    r"güçlü (bir )?sinyal\w*",
    r"dikkat çekici",
    r"kayda değer",
    r"anlamlı bir (hareket|değişim|seviye)\w*",
    r"belirgin bir (hareket|değişim|seviye)\w*",
    r"teknik görünüm\w*",
    r"muhtemel\w*",
    r"büyük (ihtimal|olasılık)\w*",
    r"olasılıkla",
    r"%\d+ daha\w*",
    r"ortalamadan %?\d+ (daha )?(iyi|kötü|fazla)\w*",
];
/// §2.1 Vague phrases, always forbidden (EN).
pub const VAGUE_HARD_EN: &[&str] = &[
    r"(a )?key level\w*",
    r"important level\w*",
    r"critical (level|point|threshold)\w*",
    r"crucial (level|point)\w*",
    r"significant (level|move)\w*",
    r"major level\w*",
    r"strong signal\w*",
    r"notable",
    r"noteworthy",
    r"meaningful move\w*",
    r"technical picture",
    r"likely",
    r"probabl[ey]",
    r"possibly",
    r"\d+ ?% (more|less|better|worse)",
];
/// §2.1 Soft vague words: forbidden in a sentence that carries no number (an entitled
/// reader only; a redacted reader may not see numbers).
pub const VAGUE_SOFT_TR: &[&str] = &[
    r"olağandışı",
    r"büyük",
    r"önemli",
    r"kritik",
    r"güçlü",
    r"sert",
    r"aşırı yüksek",
];
/// §2.1 (EN).
pub const VAGUE_SOFT_EN: &[&str] = &[
    r"unusual(ly)?",
    r"heavy",
    r"big",
    r"significant(ly)?",
    r"strong(ly)?",
    r"sharp(ly)?",
    r"key",
];

/// §2.2 Advice words (TR), incl. the conjugations an advice sentence uses.
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
/// §2.2 Advice words (EN).
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

/// §2.4 Product-internal terms: always forbidden on every surface (glossary included).
pub const INTERNAL_TR: &[&str] = &[
    r"bar(ı|ın|a|da|dan|lar|ları|ların|larda|lardan|lık|lik)?",
    r"teyit\w*",
    r"aile(si|nin|den|ler|leri|lerden|ye|yi)?",
    r"dedektör\w*",
    r"olgu(su|lar|ları|yu|nun|dan|ya)?",
    r"bulgu(su|lar|ları|yu|nun|dan|ya)?",
    r"kademe\w*",
    r"sinyal\w*",
    r"aday(ı|lar|ları)?",
    r"kesinleş\w*",
    r"skor\w*",
];
/// §2.4 (EN).
pub const INTERNAL_EN: &[&str] = &[
    r"bars?",
    r"confirm(ed|s|ation|ations)?",
    r"famil(y|ies)",
    r"detectors?",
    r"facts?",
    r"findings?",
    r"tiers?",
    r"signals?",
    r"candidates?",
    r"scores?",
];

/// §2.4 Technical terms: allowed only when this screen's glossary explains them
/// (an entry whose `term`, lower-cased, matches the same pattern).
pub const TECH_TR: &[&str] = &[
    r"rs[iı]",
    r"hareketli ortalama\w*",
    r"bollinger",
    r"bant\w*",
    r"hacim\w*",
    r"kırılım\w*",
    r"aşırı alım\w*",
    r"aşırı satım\w*",
    r"sıkışma\w*",
    r"uyumsuzluk\w*",
    r"kurumsal",
    r"direnç\w*",
    r"destek\w*",
    r"oynaklık\w*",
    r"momentum\w*",
    r"karne\w*",
    r"atr",
];
/// §2.4 (EN).
pub const TECH_EN: &[&str] = &[
    r"rsi",
    r"moving averages?",
    r"bollinger",
    r"bands?",
    r"volume",
    r"breakouts?",
    r"overbought",
    r"oversold",
    r"squeeze\w*",
    r"divergence\w*",
    r"institutional",
    r"resistance",
    r"support",
    r"volatility",
    r"momentum",
    r"track record",
    r"atr",
];

/// §2.5 Length limits (characters).
pub const MAX_HEADLINE: usize = 80;
/// §2.5
pub const MAX_DETAIL: usize = 200;
/// §2.5
pub const MAX_DETAIL_SENTENCES: usize = 3;
/// §2.5
pub const MAX_LEDGER: usize = 70;
/// §2.5
pub const MAX_GLOSS: usize = 160;

/// §2.7 Ledger phrases that state no expectation.
pub const LEDGER_BANNED_TR: &[&str] =
    &[r"beklendiği gibi", r"beklenen yönde", r"beklendiği şekilde"];
/// §2.7 (EN).
pub const LEDGER_BANNED_EN: &[&str] = &[
    r"as expected",
    r"the expected way",
    r"in the expected direction",
];
/// §2.7 Words that say which way the expectation went (one must appear when the
/// ledger states counts).
pub const LEDGER_DIR_TR: &str = r"yüksel\w*|art\w*|çık\w*|yukarı|düş\w*|geril\w*|in(di|er|mesi)\w*|aşağı|sürdü|sürme\w*|devam\w*|dışına|dışında|kırıl\w*|dön\w*";
/// §2.7 (EN).
pub const LEDGER_DIR_EN: &str = r"ris\w*|rose|up|higher|climb\w*|fall\w*|fell|down|lower|drop\w*|continu\w*|kept|outside|broke|break\w*|turn\w*|revers\w*";

/// §2.8 Validity lexicon (TR): class → patterns.
pub const VALIDITY_TR: &[(&str, &str)] = &[
    (
        "valid",
        r"hâlâ geçerli|halen geçerli|hâlâ sürüyor|sürüyor|devam ediyor|geçerliliğini koruyor",
    ),
    ("unknown", r"(hâlâ )?geçerli olup olmadığı bilinmiyor"),
    (
        "no_longer",
        r"artık geçerli değil|geçerliliğini yitirdi|artık sürmüyor",
    ),
    ("closed:realized", r"gerçekleşti"),
    (
        "closed:failed",
        r"tutmadı|gerçekleşmedi|geçersiz (oldu|kaldı)",
    ),
    ("closed:expired", r"süresi doldu"),
    ("closed:retracted", r"geri çekildi"),
];
/// §2.8 (EN).
pub const VALIDITY_EN: &[(&str, &str)] = &[
    ("valid", r"still holds|still valid|still in place"),
    ("unknown", r"whether it still holds is unknown"),
    (
        "no_longer",
        r"no longer (holds|valid|in place)|is no longer valid",
    ),
    ("closed:realized", r"played out|came true"),
    (
        "closed:failed",
        r"did not play out|didn't play out|not played out|did not come true|was invalidated|invalidated",
    ),
    ("closed:expired", r"ran out of time|expired"),
    ("closed:retracted", r"withdrawn|retracted"),
];
/// §2.8 A sentence about the track record is not read for validity / direction (its
/// "gerçekleşti / played out" is about past cases).
pub const LEDGER_SENTENCE_TR: &str = r"geçmişte|benzerler\w*|karne\w*|seferin|örnek|vaka";
/// §2.8 (EN).
pub const LEDGER_SENTENCE_EN: &str = r"historically|in the past|past:|record|cases|of \d+";
/// §2.8 Direction lexicon (TR / EN): up, down.
pub const DIR_UP_TR: &str = r"yukarı|yüksel\w*|üstüne|üstünde|tırman\w*|arttı";
/// §2.8
pub const DIR_DOWN_TR: &str = r"aşağı|düş\w*|altına|altında|geriledi|azaldı";
/// §2.8
pub const DIR_UP_EN: &str = r"\bup\b|upside|upward|above|rose|rising|climb\w*|higher";
/// §2.8
pub const DIR_DOWN_EN: &str = r"\bdown\b|downside|downward|below|fell|falling|slid\w*|lower";

/// One scan finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Scan name.
    pub scan: String,
    /// Situation id.
    pub situation: String,
    /// Language (`tr`, `en`, or `tr+en`).
    pub lang: String,
    /// Surface (`headline`, `detail`, `ledger`, `glossary`, `screen`).
    pub surface: String,
    /// What was found.
    pub detail: String,
}

fn compile(
    cell: &'static OnceLock<Vec<(String, Regex)>>,
    pats: &[&str],
) -> &'static [(String, Regex)] {
    cell.get_or_init(|| {
        pats.iter()
            .map(|p| {
                (
                    (*p).to_owned(),
                    Regex::new(&format!(r"\b(?:{p})\b")).expect("regex"),
                )
            })
            .collect()
    })
}

macro_rules! list {
    ($name:ident, $tr:expr, $en:expr) => {
        fn $name(lang: Lang) -> &'static [(String, Regex)] {
            static TR: OnceLock<Vec<(String, Regex)>> = OnceLock::new();
            static EN: OnceLock<Vec<(String, Regex)>> = OnceLock::new();
            match lang {
                Lang::Tr => compile(&TR, $tr),
                Lang::En => compile(&EN, $en),
            }
        }
    };
}

list!(vague_hard, VAGUE_HARD_TR, VAGUE_HARD_EN);
list!(vague_soft, VAGUE_SOFT_TR, VAGUE_SOFT_EN);
list!(advice, ADVICE_TR, ADVICE_EN);
list!(internal, INTERNAL_TR, INTERNAL_EN);
list!(tech, TECH_TR, TECH_EN);
list!(ledger_banned, LEDGER_BANNED_TR, LEDGER_BANNED_EN);

fn one(cell: &'static OnceLock<Regex>, p: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(p).expect("regex"))
}

fn ledger_dir(lang: Lang) -> &'static Regex {
    static TR: OnceLock<Regex> = OnceLock::new();
    static EN: OnceLock<Regex> = OnceLock::new();
    match lang {
        Lang::Tr => one(&TR, &format!(r"\b(?:{LEDGER_DIR_TR})\b")),
        Lang::En => one(&EN, &format!(r"\b(?:{LEDGER_DIR_EN})\b")),
    }
}

fn ledger_sentence(lang: Lang) -> &'static Regex {
    static TR: OnceLock<Regex> = OnceLock::new();
    static EN: OnceLock<Regex> = OnceLock::new();
    match lang {
        Lang::Tr => one(&TR, LEDGER_SENTENCE_TR),
        Lang::En => one(&EN, LEDGER_SENTENCE_EN),
    }
}

fn validity_lex(lang: Lang) -> &'static [(String, Regex)] {
    static TR: OnceLock<Vec<(String, Regex)>> = OnceLock::new();
    static EN: OnceLock<Vec<(String, Regex)>> = OnceLock::new();
    let (cell, src) = match lang {
        Lang::Tr => (&TR, VALIDITY_TR),
        Lang::En => (&EN, VALIDITY_EN),
    };
    cell.get_or_init(|| {
        src.iter()
            .map(|(c, p)| {
                (
                    (*c).to_owned(),
                    Regex::new(&format!(r"\b(?:{p})\b")).expect("regex"),
                )
            })
            .collect()
    })
}

fn dir_lex(lang: Lang) -> (&'static Regex, &'static Regex) {
    static TU: OnceLock<Regex> = OnceLock::new();
    static TD: OnceLock<Regex> = OnceLock::new();
    static EU: OnceLock<Regex> = OnceLock::new();
    static ED: OnceLock<Regex> = OnceLock::new();
    match lang {
        Lang::Tr => (
            one(&TU, &format!(r"\b(?:{DIR_UP_TR})")),
            one(&TD, &format!(r"\b(?:{DIR_DOWN_TR})")),
        ),
        Lang::En => (one(&EU, DIR_UP_EN), one(&ED, DIR_DOWN_EN)),
    }
}

fn hits(list: &[(String, Regex)], text: &str) -> Vec<String> {
    list.iter()
        .flat_map(|(_, r)| r.find_iter(text).map(|m| m.as_str().to_owned()))
        .collect()
}

/// Validity classes a text expresses, track-record sentences skipped (§2.8). A class
/// whose phrase is contained in a longer matched phrase is dropped ("did not play out"
/// is not also "played out"; "geçersiz oldu" is not "no_longer").
#[must_use]
pub fn validity_classes(text: &str, lang: Lang) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for sent in sentences(text) {
        let l = lower(&sent, lang);
        if ledger_sentence(lang).is_match(&l) {
            continue;
        }
        let mut spans: Vec<(usize, usize, String)> = Vec::new();
        for (c, r) in validity_lex(lang) {
            for m in r.find_iter(&l) {
                spans.push((m.start(), m.end(), c.clone()));
            }
        }
        for (i, (s, e, c)) in spans.iter().enumerate() {
            let inside = spans
                .iter()
                .enumerate()
                .any(|(j, (s2, e2, _))| j != i && s2 <= s && e <= e2 && (e2 - s2) > (e - s));
            if !inside && !out.contains(c) {
                out.push(c.clone());
            }
        }
    }
    out
}

/// Directions a text expresses (`up`, `down`), track-record sentences skipped.
#[must_use]
pub fn directions(text: &str, lang: Lang) -> Vec<&'static str> {
    let (u, d) = dir_lex(lang);
    let mut out = Vec::new();
    for sent in sentences(text) {
        let l = lower(&sent, lang);
        if ledger_sentence(lang).is_match(&l) {
            continue;
        }
        if u.is_match(&l) && !out.contains(&"up") {
            out.push("up");
        }
        if d.is_match(&l) && !out.contains(&"down") {
            out.push("down");
        }
    }
    out
}

/// Everything the scans need about one situation in one language.
pub struct Item<'a> {
    /// The key.
    pub key: &'a Key,
    /// Surfaces by language.
    pub by_lang: BTreeMap<Lang, &'a Surfaces>,
    /// A second rendering (determinism check), by language.
    pub again: BTreeMap<Lang, &'a Surfaces>,
}

fn f(scan: &str, sid: &str, lang: &str, surface: &str, detail: impl Into<String>) -> Finding {
    Finding {
        scan: scan.to_owned(),
        situation: sid.to_owned(),
        lang: lang.to_owned(),
        surface: surface.to_owned(),
        detail: detail.into(),
    }
}

fn surfaces_of(s: &Surfaces) -> Vec<(&'static str, String)> {
    let mut v = vec![
        ("headline", s.headline.clone()),
        ("detail", s.detail.clone()),
        ("ledger", s.ledger.clone()),
    ];
    for g in &s.glossary {
        v.push(("glossary", format!("{}: {}", g.term, g.text)));
    }
    v
}

fn glossed(gl: &[GlossEntry], pat: &Regex, lang: Lang) -> bool {
    gl.iter().any(|g| pat.is_match(&lower(&g.term, lang)))
}

/// Every scan over one situation (SPEC §2).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn scan(it: &Item<'_>) -> Vec<Finding> {
    let k = it.key;
    let sid = k.id.as_str();
    let mut out = Vec::new();
    for (&lang, s) in &it.by_lang {
        let ln = lang.name();
        let surf = surfaces_of(s);
        // §2.0 non-empty
        for (name, t) in &surf[..3] {
            if t.trim().is_empty() {
                out.push(f("empty", sid, ln, name, "boş"));
            }
        }
        for (name, t) in &surf {
            let l = lower(&strip_symbol(t, &k.instrument), lang);
            // §2.1 vague
            for h in hits(vague_hard(lang), &l) {
                out.push(f("vague", sid, ln, name, h));
            }
            if !k.redacted {
                for sent in sentences(&l) {
                    if sent.bytes().any(|b| b.is_ascii_digit()) {
                        continue;
                    }
                    for h in hits(vague_soft(lang), &sent) {
                        out.push(f("vague", sid, ln, name, format!("{h} (sayısız cümlede)")));
                    }
                }
            }
            // §2.2 advice
            for h in hits(advice(lang), &l) {
                out.push(f("advice", sid, ln, name, h));
            }
            // §2.4 internal terms
            for h in hits(internal(lang), &l) {
                out.push(f("internal", sid, ln, name, h));
            }
            // §2.4 technical term without a glossary entry
            if *name != "glossary" {
                for (p, r) in tech(lang) {
                    if let Some(m) = r.find(&l) {
                        if !glossed(&s.glossary, r, lang) {
                            out.push(f(
                                "unglossed",
                                sid,
                                ln,
                                name,
                                format!("{} ({p})", m.as_str()),
                            ));
                        }
                    }
                }
            }
            // §2.9 K33: no digit for a redacted reader (instrument symbol stripped)
            if k.redacted
                && strip_symbol(t, &k.instrument)
                    .bytes()
                    .any(|b| b.is_ascii_digit())
            {
                out.push(f("redacted_digit", sid, ln, name, t.clone()));
            }
        }
        // §2.5 lengths
        let c = |t: &str| t.chars().count();
        if c(&s.headline) > MAX_HEADLINE {
            out.push(f(
                "length",
                sid,
                ln,
                "headline",
                format!("{} > {MAX_HEADLINE}", c(&s.headline)),
            ));
        }
        if c(&s.detail) > MAX_DETAIL {
            out.push(f(
                "length",
                sid,
                ln,
                "detail",
                format!("{} > {MAX_DETAIL}", c(&s.detail)),
            ));
        }
        let ns = sentences(&s.detail).len();
        if ns > MAX_DETAIL_SENTENCES {
            out.push(f(
                "length",
                sid,
                ln,
                "detail",
                format!("{ns} cümle > {MAX_DETAIL_SENTENCES}"),
            ));
        }
        if c(&s.ledger) > MAX_LEDGER {
            out.push(f(
                "length",
                sid,
                ln,
                "ledger",
                format!("{} > {MAX_LEDGER}", c(&s.ledger)),
            ));
        }
        for g in &s.glossary {
            if c(&g.text) > MAX_GLOSS {
                out.push(f(
                    "length",
                    sid,
                    ln,
                    "glossary",
                    format!("{}: {} > {MAX_GLOSS}", g.term, c(&g.text)),
                ));
            }
        }
        // §2.3 repetition on one screen (face = headline + ledger; open = all)
        let mut seen: BTreeMap<String, &str> = BTreeMap::new();
        for (name, t) in &surf {
            for sent in sentences(t) {
                let n = norm_sentence(&sent, lang);
                if n.split_whitespace().count() < 2 {
                    continue;
                }
                if let Some(prev) = seen.get(&n) {
                    out.push(f(
                        "repeat",
                        sid,
                        ln,
                        "screen",
                        format!("{prev} = {name}: {sent}"),
                    ));
                } else {
                    seen.insert(n, name);
                }
            }
        }
        // §2.6 threshold named and stated (screen = headline + detail)
        let screen = strip_symbol(&format!("{} {}", s.headline, s.detail), &k.instrument);
        let low = lower(&screen, lang);
        let named = Regex::new(k.name_pattern(lang)).is_ok_and(|r| r.is_match(&low));
        let nums = numbers(&screen, lang);
        let valued = k.threshold.values.iter().any(|v| has_close(&nums, *v));
        let param = k.threshold.params_any.is_empty()
            || k.threshold.params_any.iter().any(|v| has_close(&nums, *v));
        if !named {
            out.push(f(
                "threshold",
                sid,
                ln,
                "screen",
                format!("eşiğin adı yok ({})", k.threshold.id),
            ));
        }
        if !k.redacted && !valued {
            out.push(f(
                "threshold",
                sid,
                ln,
                "screen",
                format!("eşiğin değeri yok ({})", k.threshold.id),
            ));
        }
        if !k.redacted && !param {
            out.push(f(
                "threshold",
                sid,
                ln,
                "screen",
                format!("dedektör parametresi yok ({})", k.threshold.id),
            ));
        }
        // §2.7 the ledger states the expectation itself
        let ll = lower(&s.ledger, lang);
        let dl = lower(&s.detail, lang);
        for h in hits(ledger_banned(lang), &ll)
            .into_iter()
            .chain(hits(ledger_banned(lang), &dl))
        {
            out.push(f("ledger_expect", sid, ln, "ledger", format!("yasak: {h}")));
        }
        if !k.redacted && k.ledger_n > 0 {
            let ln_nums = numbers(&strip_symbol(&s.ledger, &k.instrument), lang);
            let has = |v: u32| has_close(&ln_nums, f64::from(v));
            if !(has(k.ledger_n) && has(k.ledger_realized)) {
                out.push(f(
                    "ledger_expect",
                    sid,
                    ln,
                    "ledger",
                    "sayılar (n, tutan) yok",
                ));
            }
            if !has(k.horizon_bars) {
                out.push(f(
                    "ledger_expect",
                    sid,
                    ln,
                    "ledger",
                    format!("beklentinin süresi ({}) yok", k.horizon_bars),
                ));
            }
            if !ledger_dir(lang).is_match(&ll) {
                out.push(f(
                    "ledger_expect",
                    sid,
                    ln,
                    "ledger",
                    "beklentinin yönü yok",
                ));
            }
        }
        // §2.8 contradiction: each of headline / detail against the key, and each other
        let want = k.validity_class();
        let mut per: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for (name, t) in [("headline", &s.headline), ("detail", &s.detail)] {
            let cls = validity_classes(t, lang);
            for c in &cls {
                if *c != want {
                    out.push(f(
                        "contradiction",
                        sid,
                        ln,
                        name,
                        format!("geçerlilik {c} ≠ {want}"),
                    ));
                }
            }
            per.insert(name, cls);
            if k.direction != "neutral" && name == "headline" {
                let ds = directions(t, lang);
                let opp = if k.direction == "up" { "down" } else { "up" };
                if ds.contains(&opp) && !ds.contains(&k.direction.as_str()) {
                    out.push(f(
                        "contradiction",
                        sid,
                        ln,
                        name,
                        format!("yön {opp} ≠ {}", k.direction),
                    ));
                }
            }
            for (unit, n) in relative_days(&strip_symbol(t, &k.instrument), lang) {
                let ok = match unit {
                    RelUnit::TradingDay => k.day_offsets.contains(&i64::try_from(n).unwrap_or(-1)),
                    RelUnit::Bar => k.bar_offsets.contains(&n),
                };
                if !ok {
                    out.push(f(
                        "contradiction",
                        sid,
                        ln,
                        name,
                        format!("zaman {n} ({unit:?}) olayların hiçbiriyle uyuşmuyor"),
                    ));
                }
            }
            for (d, m, y) in dates(t, lang) {
                let ok = k
                    .dates
                    .iter()
                    .any(|(kd, km, ky)| *kd == d && *km == m && y.is_none_or(|y| y == *ky));
                if !ok {
                    out.push(f(
                        "contradiction",
                        sid,
                        ln,
                        name,
                        format!("tarih {d}.{m} olaylarla uyuşmuyor"),
                    ));
                }
            }
        }
        let (h, d) = (&per["headline"], &per["detail"]);
        if !h.is_empty() && !d.is_empty() && h.iter().all(|c| !d.contains(c)) {
            out.push(f(
                "contradiction",
                sid,
                ln,
                "screen",
                format!("başlık {h:?} ↔ ayrıntı {d:?}"),
            ));
        }
        // §2.10 determinism
        if let Some(a) = it.again.get(&lang) {
            if *a != *s {
                out.push(f("determinism", sid, ln, "screen", "iki üretim farklı"));
            }
        }
    }
    // §2.9 TR ↔ EN equivalence
    if let (Some(tr), Some(en)) = (it.by_lang.get(&Lang::Tr), it.by_lang.get(&Lang::En)) {
        for ((name, a), (_, b)) in surfaces_of(tr)
            .iter()
            .take(3)
            .zip(surfaces_of(en).iter().take(3))
        {
            let mut na = numbers(&strip_symbol(a, &k.instrument), Lang::Tr);
            let mut nb = numbers(&strip_symbol(b, &k.instrument), Lang::En);
            na.sort_by(f64::total_cmp);
            nb.sort_by(f64::total_cmp);
            if na != nb {
                out.push(f(
                    "equivalence",
                    sid,
                    "tr+en",
                    name,
                    format!("sayılar {na:?} ↔ {nb:?}"),
                ));
            }
            let (ta, tb) = (times(a), times(b));
            if ta != tb {
                out.push(f(
                    "equivalence",
                    sid,
                    "tr+en",
                    name,
                    format!("saatler {ta:?} ↔ {tb:?}"),
                ));
            }
            let mut da = dates(a, Lang::Tr);
            let mut db = dates(b, Lang::En);
            da.sort_unstable();
            db.sort_unstable();
            if da != db {
                out.push(f(
                    "equivalence",
                    sid,
                    "tr+en",
                    name,
                    format!("tarihler {da:?} ↔ {db:?}"),
                ));
            }
            let mut va = validity_classes(a, Lang::Tr);
            let mut vb = validity_classes(b, Lang::En);
            va.sort();
            vb.sort();
            if va != vb {
                out.push(f(
                    "equivalence",
                    sid,
                    "tr+en",
                    name,
                    format!("geçerlilik {va:?} ↔ {vb:?}"),
                ));
            }
        }
        if tr.glossary.len() != en.glossary.len() {
            out.push(f(
                "equivalence",
                sid,
                "tr+en",
                "glossary",
                format!(
                    "sözlük girdisi {} ↔ {}",
                    tr.glossary.len(),
                    en.glossary.len()
                ),
            ));
        }
    }
    out
}

/// Scan names in report order.
pub const SCANS: &[&str] = &[
    "vague",
    "advice",
    "repeat",
    "internal",
    "unglossed",
    "length",
    "threshold",
    "ledger_expect",
    "contradiction",
    "equivalence",
    "redacted_digit",
    "determinism",
    "empty",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn h(list: fn(Lang) -> &'static [(String, Regex)], lang: Lang, t: &str) -> Vec<String> {
        hits(list(lang), &lower(t, lang))
    }

    #[test]
    fn word_boundaries_avoid_false_friends() {
        assert!(h(internal, Lang::Tr, "barış içinde olgun bulgur").is_empty());
        assert_eq!(
            h(internal, Lang::Tr, "3 bar önce, barda; teyit edildi"),
            vec!["bar", "barda", "teyit"]
        );
        assert!(h(advice, Lang::Tr, "büyük alım geldi; satış sürdü; alt sınır").is_empty());
        assert_eq!(
            h(advice, Lang::Tr, "Şimdi al, fırsatı kaçırma"),
            vec!["al", "fırsatı", "kaçırma"]
        );
        assert!(h(advice, Lang::En, "heavy buying and selling").is_empty());
        assert_eq!(
            h(advice, Lang::En, "Buy now: a rare opportunity"),
            vec!["buy", "opportunity"]
        );
        assert!(h(internal, Lang::En, "it fell back; the barrier").is_empty());
    }

    #[test]
    fn vague_lists() {
        assert_eq!(
            h(vague_hard, Lang::Tr, "SYN03 önemli bir eşiğin üstüne çıktı"),
            vec!["önemli bir eşiğin"]
        );
        assert_eq!(
            h(vague_hard, Lang::En, "broke above a key level"),
            vec!["a key level"]
        );
        assert!(h(
            vague_hard,
            Lang::Tr,
            "son 20 günün en yükseğinin (148,90) üstüne çıktı"
        )
        .is_empty());
    }

    #[test]
    fn validity_lexicon() {
        assert_eq!(
            validity_classes("SYN06 zayıfladı. Artık geçerli değil.", Lang::Tr),
            vec!["no_longer"]
        );
        assert_eq!(
            validity_classes("Kapandı: 10 Şubat'ta tutmadı.", Lang::Tr),
            vec!["closed:failed"]
        );
        assert_eq!(
            validity_classes("Closed: did not play out on 10 February.", Lang::En),
            vec!["closed:failed"]
        );
        assert_eq!(
            validity_classes(
                "Still holds. Historically, this played out in 3 of 9 cases.",
                Lang::En
            ),
            vec!["valid"]
        );
        assert_eq!(
            directions("RSI fell back below 70.", Lang::En),
            vec!["down"]
        );
    }
}
