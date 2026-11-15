//! M2 lexicons and the deterministic time-correctness classifier (SPEC "M2" 1, 3).
//!
//! The classifier looks for surface markers only; it does not parse. Known limits (also in
//! the README):
//! - TR past = a word ending in `-dı/-di/-du/-dü/-tı/-ti/-tu/-tü` (+ common person/plural
//!   endings), `-mış` family, or `-dığ-` participles; a short exception list (`şimdi`,
//!   `kendi`, …) avoids the commonest false hits; unknown words ending that way still count.
//! - TR present = `-yor` (not `-yordu`), `-makta/-mekte`, `-dır` family, or the words
//!   `şu an(da)`, `şimdi`, `hâlâ/hala/halen`. A bare nominal predicate ("RSI 70'in
//!   üstünde.") has **no** present marker and is counted as missing one.
//! - TR conditional = a word ending in `-sa/-se` (with an exception list: `hisse`, `borsa`,
//!   `kısa`, …), or `eğer`, `takdirde`, `halinde`, `koşuluyla`.
//! - TR "no longer" = `artık` together with a negation (`değil…`, `-mıyor`, `-maz`, `-madı`…).
//! - EN past = a word ending in `-ed` (exception list: `need`, `speed`, `hundred`, `based`…)
//!   or an irregular past form (list); present = `is/are/remains/stays/holds/…`, `still`,
//!   `now`, `currently`; conditional = `if`, `unless`, `provided`, `in case`; "no longer" =
//!   `no longer`, `not … anymore`, `no more`.
//! - Time reference = `önce`/`ago`, `bugün`/`today`, `dün`/`yesterday`, `this bar`, … .
//!   Exact = a number (digits or a number word) followed by a bar/day/session word and
//!   `önce`/`ago` equal to the true bars-ago; for 0 bars ago `bugün/bu bar/today/this bar`,
//!   for 1 also `dün/önceki bar/yesterday/previous bar`.
//! - Number words: TR without `bir` (also the indefinite article), EN without `one` / `a`;
//!   TR `altı`/`on` are counted only as whole words (they also mean "under"/"on").

use qp_capsule::Lang;

/// Lower-case with Turkish dotted/dotless I handled for TR.
#[must_use]
pub fn lower(s: &str, lang: Lang) -> String {
    if lang == Lang::Tr {
        s.chars()
            .map(|c| match c {
                'I' => 'ı',
                'İ' => 'i',
                _ => c,
            })
            .collect::<String>()
            .to_lowercase()
    } else {
        s.to_lowercase()
    }
}

/// Word tokens (letters only, lower-cased); digits and punctuation split words.
#[must_use]
pub fn words(s: &str, lang: Lang) -> Vec<String> {
    lower(s, lang)
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Tokens keeping numbers: maximal runs of letters, or of digits with inner `.`/`,`.
#[must_use]
pub fn tokens(s: &str, lang: Lang) -> Vec<String> {
    let l = lower(s, lang);
    let cs: Vec<char> = l.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_alphabetic() {
            let st = i;
            while i < cs.len() && cs[i].is_alphabetic() {
                i += 1;
            }
            out.push(cs[st..i].iter().collect());
        } else if c.is_ascii_digit() {
            let st = i;
            while i < cs.len()
                && (cs[i].is_ascii_digit()
                    || ((cs[i] == '.' || cs[i] == ',')
                        && i + 1 < cs.len()
                        && cs[i + 1].is_ascii_digit()))
            {
                i += 1;
            }
            out.push(cs[st..i].iter().collect());
        } else {
            i += 1;
        }
    }
    out
}

/// TR number words (value). `bir` is excluded (indefinite article).
pub const TR_NUMBERS: [(&str, u64); 25] = [
    ("sıfır", 0),
    ("iki", 2),
    ("üç", 3),
    ("dört", 4),
    ("beş", 5),
    ("altı", 6),
    ("yedi", 7),
    ("sekiz", 8),
    ("dokuz", 9),
    ("on", 10),
    ("yirmi", 20),
    ("otuz", 30),
    ("kırk", 40),
    ("elli", 50),
    ("altmış", 60),
    ("yetmiş", 70),
    ("seksen", 80),
    ("doksan", 90),
    ("yüz", 100),
    ("bin", 1000),
    ("milyon", 1_000_000),
    ("milyar", 1_000_000_000),
    ("yarım", 0),
    ("çeyrek", 0),
    ("yüzde", 0),
];

/// EN number words (value). `one` / `a` are excluded (pronoun / article).
pub const EN_NUMBERS: [(&str, u64); 33] = [
    ("zero", 0),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
    ("hundred", 100),
    ("thousand", 1000),
    ("million", 1_000_000),
    ("billion", 1_000_000_000),
    ("half", 0),
    ("quarter", 0),
    ("dozen", 12),
    ("twice", 2),
    ("percent", 0),
    ("double", 2),
];

fn number_table(lang: Lang) -> &'static [(&'static str, u64)] {
    match lang {
        Lang::Tr => &TR_NUMBERS,
        Lang::En => &EN_NUMBERS,
    }
}

/// TR stems of four or more letters that also match with suffixes (`yetmişin`,
/// `otuzdan`); shorter stems (`iki`, `üç`, `beş`, `on`, `bin`, `yüz`, `altı`) match only
/// as whole words because their prefixes are common words (`altında`, `yüzünden`, `bina`).
fn tr_prefix_ok(stem: &str) -> bool {
    stem.chars().count() >= 4 && stem != "altı"
}

/// Number words in `s` (in order). EN: whole words; TR: whole words, and long stems with
/// suffixes ([`tr_prefix_ok`]).
#[must_use]
pub fn number_words(s: &str, lang: Lang) -> Vec<String> {
    let t = number_table(lang);
    words(s, lang)
        .into_iter()
        .filter(|w| {
            t.iter()
                .any(|(n, _)| n == w || (lang == Lang::Tr && tr_prefix_ok(n) && w.starts_with(n)))
        })
        .collect()
}

/// Whether `s` contains an ASCII digit.
#[must_use]
pub fn has_digit(s: &str) -> bool {
    s.bytes().any(|b| b.is_ascii_digit())
}

const TR_PAST_ENDS: [&str; 8] = ["dı", "di", "du", "dü", "tı", "ti", "tu", "tü"];
const TR_PAST_TAILS: [&str; 10] = ["", "lar", "ler", "k", "m", "n", "nız", "niz", "ğı", "ği"];
const TR_NOT_PAST: [&str; 12] = [
    "şimdi", "kendi", "gerçi", "hedefi", "kedi", "ödendi", "ardı", "adı", "yedi", "idi", "edi",
    "di",
];
const TR_COND_EXCEPT: [&str; 12] = [
    "hisse", "borsa", "kasa", "masa", "kısa", "pasa", "tasa", "yasa", "bursa", "ise", "bile",
    "kese",
];
const EN_NOT_PAST: [&str; 16] = [
    "need", "speed", "feed", "seed", "bed", "red", "shed", "indeed", "embed", "exceed", "proceed",
    "succeed", "breed", "based", "hundred", "led",
];
const EN_IRREGULAR_PAST: [&str; 22] = [
    "was", "were", "had", "did", "broke", "rose", "fell", "began", "became", "went", "came",
    "held", "ran", "got", "made", "took", "left", "saw", "grew", "sank", "swung", "struck",
];

fn tr_past_word(w: &str) -> bool {
    if w.chars().count() < 3 || TR_NOT_PAST.contains(&w) {
        return false;
    }
    if w.contains("mış") || w.contains("miş") || w.contains("muş") || w.contains("müş") {
        return true;
    }
    for e in TR_PAST_ENDS {
        for t in TR_PAST_TAILS {
            if w.ends_with(&format!("{e}{t}"))
                && w.chars().count() > e.chars().count() + t.chars().count() + 1
            {
                return true;
            }
        }
    }
    ["dığ", "diğ", "duğ", "düğ", "tığ", "tiğ", "tuğ", "tüğ"]
        .iter()
        .any(|p| w.contains(p))
}

fn tr_present_word(w: &str) -> bool {
    if w.contains("yor") && !["yordu", "yordı", "yormuş"].iter().any(|p| w.contains(p)) {
        return true;
    }
    if w.contains("makta") || w.contains("mekte") {
        return true;
    }
    w.chars().count() > 4
        && ["dır", "dir", "dur", "dür", "tır", "tir", "tur", "tür"]
            .iter()
            .any(|e| {
                w.ends_with(e) || w.ends_with(&format!("{e}lar")) || w.ends_with(&format!("{e}ler"))
            })
}

fn tr_cond_word(w: &str) -> bool {
    if TR_COND_EXCEPT.contains(&w) || w.chars().count() < 4 {
        return false;
    }
    w.ends_with("sa") || w.ends_with("se")
}

fn en_past_word(w: &str) -> bool {
    (w.len() >= 4 && w.ends_with("ed") && !EN_NOT_PAST.contains(&w))
        || EN_IRREGULAR_PAST.contains(&w)
}

fn phrase(ws: &[String], p: &[&str]) -> bool {
    ws.windows(p.len())
        .any(|win| win.iter().zip(p).all(|(a, b)| a == b))
}

/// Surface features of a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Features {
    /// A past-tense marker.
    pub past: bool,
    /// A present-tense marker.
    pub present: bool,
    /// A conditional marker.
    pub conditional: bool,
    /// A "no longer" construction.
    pub no_longer: bool,
    /// Any time reference.
    pub time_ref: bool,
    /// A time reference equal to the true bars-ago.
    pub time_ref_exact: bool,
}

fn value_of(tok: &str, lang: Lang) -> Option<u64> {
    if tok.bytes().all(|b| b.is_ascii_digit()) {
        return tok.parse().ok();
    }
    if lang == Lang::Tr && tok == "bir" {
        return Some(1);
    }
    if lang == Lang::En && (tok == "one" || tok == "a") {
        return Some(1);
    }
    number_table(lang)
        .iter()
        .find(|(n, _)| *n == tok)
        .map(|(_, v)| *v)
        .filter(|v| *v > 0)
}

/// Classifies `s` (true bars-ago `bars_ago` for the exactness check).
#[must_use]
pub fn features(s: &str, lang: Lang, bars_ago: Option<u64>) -> Features {
    let ws = words(s, lang);
    let toks = tokens(s, lang);
    let has = |w: &str| ws.iter().any(|x| x == w);
    let mut f = Features::default();
    let unit_tr = |w: &str| w.starts_with("bar") || w.starts_with("gün") || w.starts_with("seans");
    let unit_en =
        |w: &str| w.starts_with("bar") || w.starts_with("day") || w.starts_with("session");
    match lang {
        Lang::Tr => {
            f.past = ws.iter().any(|w| tr_past_word(w));
            f.present = ws.iter().any(|w| tr_present_word(w))
                || has("şimdi")
                || has("hâlâ")
                || has("hala")
                || has("halen")
                || phrase(&ws, &["şu", "an"])
                || phrase(&ws, &["şu", "anda"]);
            f.conditional = ws.iter().any(|w| tr_cond_word(w))
                || has("eğer")
                || has("takdirde")
                || has("halinde")
                || has("koşuluyla");
            let neg = ws.iter().any(|w| {
                w.starts_with("değil")
                    || [
                        "mıyor", "miyor", "muyor", "müyor", "maz", "mez", "madı", "medi",
                    ]
                    .iter()
                    .any(|p| w.contains(p))
            });
            f.no_longer = has("artık") && neg;
            f.time_ref = has("önce")
                || has("evvel")
                || has("bugün")
                || has("dün")
                || has("geçen")
                || phrase(&ws, &["bu", "bar"])
                || phrase(&ws, &["bu", "barda"])
                || phrase(&ws, &["son", "barda"]);
            if let Some(n) = bars_ago {
                let mut exact = toks
                    .windows(3)
                    .any(|w| value_of(&w[0], lang) == Some(n) && unit_tr(&w[1]) && w[2] == "önce");
                if n == 0 {
                    exact |= has("bugün")
                        || phrase(&ws, &["bu", "bar"])
                        || phrase(&ws, &["bu", "barda"])
                        || phrase(&ws, &["son", "barda"]);
                }
                if n == 1 {
                    exact |= has("dün")
                        || phrase(&ws, &["önceki", "bar"])
                        || phrase(&ws, &["önceki", "barda"]);
                }
                f.time_ref_exact = exact;
            }
        }
        Lang::En => {
            f.past = ws.iter().any(|w| en_past_word(w));
            f.present = [
                "is",
                "are",
                "remains",
                "remain",
                "stays",
                "stay",
                "holds",
                "sits",
                "trades",
                "still",
                "now",
                "currently",
            ]
            .iter()
            .any(|w| has(w));
            f.conditional =
                has("if") || has("unless") || has("provided") || phrase(&ws, &["in", "case"]);
            f.no_longer = phrase(&ws, &["no", "longer"])
                || phrase(&ws, &["no", "more"])
                || (has("anymore") && (has("not") || has("t")))
                || phrase(&ws, &["any", "more"]) && has("not");
            f.time_ref = has("ago")
                || has("today")
                || has("yesterday")
                || has("earlier")
                || has("recently")
                || phrase(&ws, &["this", "bar"])
                || phrase(&ws, &["last", "bar"])
                || phrase(&ws, &["previous", "bar"])
                || phrase(&ws, &["latest", "bar"]);
            if let Some(n) = bars_ago {
                let mut exact = toks
                    .windows(3)
                    .any(|w| value_of(&w[0], lang) == Some(n) && unit_en(&w[1]) && w[2] == "ago");
                if n == 0 {
                    exact |= has("today")
                        || phrase(&ws, &["this", "bar"])
                        || phrase(&ws, &["latest", "bar"]);
                }
                if n == 1 {
                    exact |= has("yesterday")
                        || phrase(&ws, &["previous", "bar"])
                        || phrase(&ws, &["last", "bar"]);
                }
                f.time_ref_exact = exact;
            }
        }
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_k26_sentence() {
        let s = "RSI 3 bar önce 70'in üstüne çıktı; şu an 55, artık 70'in üstünde değil.";
        let f = features(s, Lang::Tr, Some(3));
        assert!(f.past && f.present && f.no_longer && f.time_ref && f.time_ref_exact);
        assert!(!f.conditional);
    }

    #[test]
    fn tr_markers() {
        assert!(
            features(
                "THYAO son 20 barın en yükseğini yukarı kırdı.",
                Lang::Tr,
                None
            )
            .past
        );
        assert!(!features("RSI şimdi kendi seviyesinde.", Lang::Tr, None).past);
        assert!(features("RSI 70'in üstünde kalıyor.", Lang::Tr, None).present);
        assert!(!features("RSI 70'in üstünde kalıyordu.", Lang::Tr, None).present);
        assert!(
            features(
                "148 üzerinde iki kapanış olursa kırılım teyit olur.",
                Lang::Tr,
                None
            )
            .conditional
        );
        assert!(!features("Hisse borsada kısa süre yükseldi.", Lang::Tr, None).conditional);
        assert!(features("Bugün kırıldı.", Lang::Tr, Some(0)).time_ref_exact);
        assert!(features("beş bar önce kırdı", Lang::Tr, Some(5)).time_ref_exact);
        assert!(!features("beş bar önce kırdı", Lang::Tr, Some(30)).time_ref_exact);
    }

    #[test]
    fn en_markers() {
        let s = "RSI crossed above 70 three bars ago; it is now 55 and no longer above 70.";
        let f = features(s, Lang::En, Some(3));
        assert!(f.past && f.present && f.no_longer && f.time_ref_exact);
        assert!(
            features(
                "If it closes above 148, the breakout is confirmed.",
                Lang::En,
                None
            )
            .conditional
        );
        assert!(!features("We need a hundred more.", Lang::En, None).past);
        assert!(features("It broke out yesterday.", Lang::En, Some(1)).time_ref_exact);
    }

    #[test]
    fn numbers_and_words() {
        assert_eq!(
            tokens("70'in 1.250,5 ve 3", Lang::Tr),
            vec!["70", "in", "1.250,5", "ve", "3"]
        );
        assert_eq!(
            number_words("bir hisse beş bar önce", Lang::Tr),
            vec!["beş"]
        );
        assert_eq!(
            number_words("yetmişin altında, yüzünden", Lang::Tr),
            vec!["yetmişin"]
        );
        assert_eq!(number_words("one of seventy", Lang::En), vec!["seventy"]);
        assert!(has_digit("a1") && !has_digit("abc"));
        assert_eq!(lower("IŞIK İzmir", Lang::Tr), "ışık izmir");
    }
}
