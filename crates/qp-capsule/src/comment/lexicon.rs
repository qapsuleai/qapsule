//! Word lists of the comment checks (advice words, direction words) and their scans.

use super::*;

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
