//! Turkish morphology for the template grammar.
//!
//! Suffixes are chosen from the **pronounced** form of a word (a number is read aloud:
//! `148,20` → "… yirmi", `70` → "yetmiş"; a ticker is read as a word: `THYAO` → "thyao")
//! and attached to the **written** form, with an apostrophe after numbers and proper
//! nouns (`70'in`, `148,20'nin`, `THYAO'da`, `ASELS'te`).
//!
//! Rules implemented:
//! - two-way harmony A = a/e (last vowel back → a, front → e);
//! - four-way harmony I = ı/i/u/ü (a,ı → ı; o,u → u; e,i → i; ö,ü → ü);
//! - consonant assimilation of D (d/t) after a voiceless final ("fıstıkçı şahap":
//!   f s t k ç ş h p): `çıktı`, `ASELS'te`, `71,4'te`;
//! - buffer consonants after a vowel: genitive `n` (`yirmi'nin`), dative/accusative `y`,
//!   possessive `s`; and `n` after a 3rd-person possessive noun (`ortalamasına`);
//! - final-stop softening p/ç/t/k → b/c/d/ğ before a vowel-initial suffix on written
//!   polysyllabic common nouns (`eşik` → `eşiğin`, `direnç` → `direnci`), never after an
//!   apostrophe;
//! - aorist (-Ar for monosyllabic stems except the -Ir list, -Ir for polysyllabic, -r after
//!   a vowel), conditional = aorist + sA (`çıkarsa`, `inerse`, `kapanırsa`), past -DI,
//!   copula -DIr (`bardır`, `gündür`).

/// A Turkish word (or number, or ticker) ready to take suffixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// Written form.
    pub written: String,
    /// Pronounced form (lower case); only its end matters.
    pub spoken: String,
    /// Suffixes are separated by an apostrophe (numbers, proper nouns).
    pub apostrophe: bool,
    /// Already carries a 3rd-person possessive (`ortalaması`, `seviyesi`): buffer `n`.
    pub possessed: bool,
}

const VOWELS: [char; 8] = ['a', 'ı', 'o', 'u', 'e', 'i', 'ö', 'ü'];
const VOICELESS: [char; 8] = ['f', 's', 't', 'k', 'ç', 'ş', 'h', 'p'];
/// Polysyllabic nouns ending in p/ç/t/k that do not soften.
const NO_SOFTEN: [&str; 6] = ["fiyat", "saat", "hukuk", "evrak", "devlet", "millet"];

fn is_vowel(c: char) -> bool {
    VOWELS.contains(&c)
}

fn is_back(v: char) -> bool {
    matches!(v, 'a' | 'ı' | 'o' | 'u')
}

/// Lower case with Turkish dotted/dotless I. ASCII `I` in a ticker is read as `i`
/// (tickers are written without Turkish letters: `BIMAS` = BİMAŞ).
fn lower_tr(s: &str, ascii_i_dotted: bool) -> String {
    s.chars()
        .map(|c| match c {
            'I' if ascii_i_dotted => 'i',
            'I' => 'ı',
            'İ' => 'i',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

fn last_vowel(s: &str) -> char {
    s.chars().rev().find(|c| is_vowel(*c)).unwrap_or('e')
}

fn last_char(s: &str) -> char {
    s.chars().last().unwrap_or('e')
}

/// Four-way harmony vowel after `v`.
#[must_use]
pub fn i4(v: char) -> char {
    match v {
        'a' | 'ı' => 'ı',
        'o' | 'u' => 'u',
        'ö' | 'ü' => 'ü',
        _ => 'i',
    }
}

/// Two-way harmony vowel after `v`.
#[must_use]
pub fn a2(v: char) -> char {
    if is_back(v) {
        'a'
    } else {
        'e'
    }
}

fn syllables(s: &str) -> usize {
    s.chars().filter(|c| is_vowel(*c)).count()
}

fn soften(w: &str) -> String {
    if syllables(w) < 2 || NO_SOFTEN.iter().any(|x| w.ends_with(x)) {
        return w.to_owned();
    }
    let mut cs: Vec<char> = w.chars().collect();
    let n = cs.len();
    let prev = if n >= 2 { cs[n - 2] } else { ' ' };
    let last = cs[n - 1];
    cs[n - 1] = match last {
        'p' => 'b',
        'ç' => 'c',
        't' => 'd',
        'k' if prev == 'n' => 'g',
        'k' => 'ğ',
        c => c,
    };
    cs.into_iter().collect()
}

impl Word {
    /// A common noun (no apostrophe), e.g. `eşik`, `uzun ortalama`.
    #[must_use]
    pub fn noun(w: &str) -> Self {
        Self {
            written: w.to_owned(),
            spoken: lower_tr(w, false),
            apostrophe: false,
            possessed: false,
        }
    }

    /// A noun that already carries a 3rd-person possessive (`son günlerin ortalaması`).
    #[must_use]
    pub fn possessed(w: &str) -> Self {
        Self {
            possessed: true,
            ..Self::noun(w)
        }
    }

    /// A proper noun / ticker read as a word (`THYAO`, `ASELS`). A ticker without any
    /// vowel is read letter by letter; consonant letter names end in `e` (`be`, `ce`…).
    #[must_use]
    pub fn proper(w: &str) -> Self {
        let mut spoken = lower_tr(w, true);
        if syllables(&spoken) == 0 {
            spoken.push('e');
        }
        Self {
            written: w.to_owned(),
            spoken,
            apostrophe: true,
            possessed: false,
        }
    }

    /// A number as written (`148,20`, `2.340.000`), read aloud for its suffixes.
    #[must_use]
    pub fn number(written: &str) -> Self {
        Self {
            written: written.to_owned(),
            spoken: number_last_word(written).to_owned(),
            apostrophe: true,
            possessed: false,
        }
    }

    fn ends_vowel(&self) -> bool {
        is_vowel(last_char(&self.spoken))
    }

    fn voiceless(&self) -> bool {
        VOICELESS.contains(&last_char(&self.spoken))
    }

    fn lv(&self) -> char {
        last_vowel(&self.spoken)
    }

    fn join(&self, suffix: &str) -> String {
        if self.apostrophe {
            format!("{}'{suffix}", self.written)
        } else if suffix.starts_with(is_vowel) && !self.ends_vowel() {
            format!("{}{suffix}", soften(&self.written))
        } else {
            format!("{}{suffix}", self.written)
        }
    }

    /// Genitive: `70'in`, `148,20'nin`, `eşiğin`, `ortalamasının`.
    #[must_use]
    pub fn gen(&self) -> String {
        let i = i4(self.lv());
        if self.ends_vowel() {
            self.join(&format!("n{i}n"))
        } else {
            self.join(&format!("{i}n"))
        }
    }

    /// Dative: `70'e`, `THYAO'ya`, `ortalamasına`.
    #[must_use]
    pub fn dat(&self) -> String {
        let a = a2(self.lv());
        if self.possessed {
            self.join(&format!("n{a}"))
        } else if self.ends_vowel() {
            self.join(&format!("y{a}"))
        } else {
            self.join(&a.to_string())
        }
    }

    /// Locative: `THYAO'da`, `ASELS'te`, `148,20'de`, `ortalamasında`.
    #[must_use]
    pub fn loc(&self) -> String {
        let a = a2(self.lv());
        if self.possessed {
            self.join(&format!("nd{a}"))
        } else if self.voiceless() {
            self.join(&format!("t{a}"))
        } else {
            self.join(&format!("d{a}"))
        }
    }

    /// Locative + `ki` (adjective: "the resistance at 148,20" = `148,20'deki direnç`).
    #[must_use]
    pub fn loc_ki(&self) -> String {
        format!("{}ki", self.loc())
    }

    /// Accusative: `direnci`, `desteği`, `THYAO'yu`.
    #[must_use]
    pub fn acc(&self) -> String {
        let i = i4(self.lv());
        if self.possessed {
            self.join(&format!("n{i}"))
        } else if self.ends_vowel() {
            self.join(&format!("y{i}"))
        } else {
            self.join(&i.to_string())
        }
    }

    /// 3rd-person possessive + locative: `87'sinde`, `40'ında`, `100'ünde`
    /// ("in 87 of them").
    #[must_use]
    pub fn poss3_loc(&self) -> String {
        let i = i4(self.lv());
        let a = a2(i);
        if self.ends_vowel() {
            self.join(&format!("s{i}nd{a}"))
        } else {
            self.join(&format!("{i}nd{a}"))
        }
    }

    /// Copula -DIr ("for … / it is"): `5 bar` → `5 bardır`, `gün` → `gündür`.
    #[must_use]
    pub fn copula(&self) -> String {
        let i = i4(self.lv());
        let d = if self.voiceless() { 't' } else { 'd' };
        format!("{}{d}{i}r", self.written)
    }
}

/// Monosyllabic verb stems whose aorist is -Ir rather than -Ar.
const AORIST_IR: [&str; 13] = [
    "al", "bil", "bul", "dur", "gel", "gör", "kal", "ol", "öl", "san", "var", "ver", "vur",
];

/// Aorist of a verb stem: `çık` → `çıkar`, `in` → `iner`, `kal` → `kalır`,
/// `kapan` → `kapanır`, `yüksel` → `yükselir`, `de` → `der`.
#[must_use]
pub fn aorist(stem: &str) -> String {
    let st = lower_tr(stem, false);
    let lv = last_vowel(&st);
    if is_vowel(last_char(&st)) {
        return format!("{stem}r");
    }
    if syllables(&st) <= 1 && !AORIST_IR.contains(&st.as_str()) {
        format!("{stem}{}r", a2(lv))
    } else {
        format!("{stem}{}r", i4(lv))
    }
}

/// Conditional on the aorist: `çıkarsa`, `inerse`, `kapanırsa`, `olursa`.
#[must_use]
pub fn conditional(stem: &str) -> String {
    let ao = aorist(stem);
    let lv = last_vowel(&lower_tr(&ao, false));
    format!("{ao}s{}", a2(lv))
}

/// Simple past -DI: `kır` → `kırdı`, `çık` → `çıktı`, `in` → `indi`, `geç` → `geçti`,
/// `dön` → `döndü`, `ol` → `oldu`, `zayıfla` → `zayıfladı`.
#[must_use]
pub fn past(stem: &str) -> String {
    let st = lower_tr(stem, false);
    let d = if VOICELESS.contains(&last_char(&st)) {
        't'
    } else {
        'd'
    };
    format!("{stem}{d}{}", i4(last_vowel(&st)))
}

fn digit_word(d: u32) -> &'static str {
    match d {
        1 => "bir",
        2 => "iki",
        3 => "üç",
        4 => "dört",
        5 => "beş",
        6 => "altı",
        7 => "yedi",
        8 => "sekiz",
        9 => "dokuz",
        _ => "sıfır",
    }
}

fn tens_word(d: u32) -> &'static str {
    match d {
        1 => "on",
        2 => "yirmi",
        3 => "otuz",
        4 => "kırk",
        5 => "elli",
        6 => "altmış",
        7 => "yetmiş",
        8 => "seksen",
        _ => "doksan",
    }
}

/// Last spoken word of an integer given as a digit string (any length; leading zeros
/// ignored): `70` → `yetmiş`, `340000` → `bin`, `0` → `sıfır`, `100` → `yüz`.
fn integer_last_word(digits: &str) -> &'static str {
    let ds: Vec<u32> = digits
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let at = |i: usize| ds.get(i).copied().unwrap_or(0);
    if ds.iter().all(|d| *d == 0) {
        return "sıfır";
    }
    // Groups of three from the right: units, thousands, millions, billions, …
    let mut g = 0;
    while g * 3 < ds.len() {
        let (u, t, h) = (at(g * 3), at(g * 3 + 1), at(g * 3 + 2));
        if u + t + h > 0 {
            if g > 0 {
                return match g {
                    1 => "bin",
                    2 => "milyon",
                    3 => "milyar",
                    _ => "trilyon",
                };
            }
            if u > 0 {
                return digit_word(u);
            }
            if t > 0 {
                return tens_word(t);
            }
            return "yüz";
        }
        g += 1;
    }
    "sıfır"
}

/// Last spoken word of a Turkish-formatted number (`.` thousands, `,` decimals):
/// `148,20` → `yirmi` ("yüz kırk sekiz virgül yirmi"), `0,06` → `altı`,
/// `71,4` → `dört`, `2.340.000` → `bin`.
#[must_use]
pub fn number_last_word(written: &str) -> &'static str {
    match written.split_once(',') {
        Some((_, frac)) if frac.chars().any(|c| c.is_ascii_digit() && c != '0') => {
            integer_last_word(frac)
        }
        Some((int, _)) => integer_last_word(int),
        None => integer_last_word(written),
    }
}

/// Upper-cases the first letter (Turkish-aware for `i` → `İ`).
#[must_use]
pub fn capitalize(s: &str) -> String {
    let mut cs = s.chars();
    match cs.next() {
        Some('i') => format!("İ{}", cs.as_str()),
        Some('ı') => format!("I{}", cs.as_str()),
        Some(c) => format!("{}{}", c.to_uppercase(), cs.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(Word::number("70").gen(), "70'in");
        assert_eq!(Word::number("30").gen(), "30'un");
        assert_eq!(Word::number("148,20").gen(), "148,20'nin");
        assert_eq!(Word::number("0,06").gen(), "0,06'nın");
        assert_eq!(Word::number("71,4").loc(), "71,4'te");
        assert_eq!(Word::number("2.340.000").gen(), "2.340.000'in");
        assert_eq!(Word::number("1.000.000").gen(), "1.000.000'un");
        assert_eq!(Word::number("100").gen(), "100'ün");
        assert_eq!(Word::number("87").poss3_loc(), "87'sinde");
        assert_eq!(Word::number("62").poss3_loc(), "62'sinde");
        assert_eq!(Word::number("40").poss3_loc(), "40'ında");
        assert_eq!(Word::number("30").poss3_loc(), "30'unda");
        assert_eq!(Word::number("3").poss3_loc(), "3'ünde");
        assert_eq!(Word::number("6").poss3_loc(), "6'sında");
        assert_eq!(Word::number("148,20").loc_ki(), "148,20'deki");
        assert_eq!(Word::number("143,5").loc_ki(), "143,5'teki");
        assert_eq!(Word::number("55").dat(), "55'e");
        assert_eq!(Word::number("60").dat(), "60'a");
        assert_eq!(Word::number("0").gen(), "0'ın");
    }

    #[test]
    fn tickers() {
        assert_eq!(Word::proper("THYAO").loc(), "THYAO'da");
        assert_eq!(Word::proper("THYAO").gen(), "THYAO'nun");
        assert_eq!(Word::proper("ASELS").loc(), "ASELS'te");
        assert_eq!(Word::proper("GARAN").loc(), "GARAN'da");
        assert_eq!(Word::proper("AKBNK").loc(), "AKBNK'ta");
        assert_eq!(Word::proper("EREGL").loc(), "EREGL'de");
        assert_eq!(Word::proper("KCHOL").loc(), "KCHOL'da");
        assert_eq!(Word::proper("SISE").loc(), "SISE'de");
        assert_eq!(Word::proper("BIMAS").loc(), "BIMAS'ta");
        assert_eq!(Word::proper("PGSUS").loc(), "PGSUS'ta");
        assert_eq!(Word::proper("TUPRS").loc(), "TUPRS'ta");
        assert_eq!(Word::proper("FROTO").loc(), "FROTO'da");
    }

    #[test]
    fn nouns() {
        assert_eq!(Word::noun("eşik").gen(), "eşiğin");
        assert_eq!(Word::noun("direnç").acc(), "direnci");
        assert_eq!(Word::noun("destek").acc(), "desteği");
        assert_eq!(Word::noun("uzun ortalama").gen(), "uzun ortalamanın");
        assert_eq!(
            Word::possessed("son günlerin ortalaması").gen(),
            "son günlerin ortalamasının"
        );
        assert_eq!(
            Word::possessed("son günlerin ortalaması").dat(),
            "son günlerin ortalamasına"
        );
        assert_eq!(
            Word::possessed("kırılım seviyesi").gen(),
            "kırılım seviyesinin"
        );
        assert_eq!(Word::noun("fiyat").gen(), "fiyatın");
        assert_eq!(Word::noun("5 bar").copula(), "5 bardır");
        assert_eq!(Word::noun("birkaç gün").copula(), "birkaç gündür");
        assert_eq!(Word::noun("bir süre").copula(), "bir süredir");
        assert_eq!(
            Word::noun("yaklaşık bir hafta").copula(),
            "yaklaşık bir haftadır"
        );
    }

    #[test]
    fn verbs() {
        assert_eq!(past("kır"), "kırdı");
        assert_eq!(past("çık"), "çıktı");
        assert_eq!(past("in"), "indi");
        assert_eq!(past("geç"), "geçti");
        assert_eq!(past("dön"), "döndü");
        assert_eq!(past("ol"), "oldu");
        assert_eq!(past("aş"), "aştı");
        assert_eq!(past("zayıfla"), "zayıfladı");
        assert_eq!(past("gerçekleş"), "gerçekleşti");
        assert_eq!(conditional("çık"), "çıkarsa");
        assert_eq!(conditional("in"), "inerse");
        assert_eq!(conditional("kapan"), "kapanırsa");
        assert_eq!(conditional("kal"), "kalırsa");
        assert_eq!(conditional("ol"), "olursa");
        assert_eq!(conditional("düş"), "düşerse");
        assert_eq!(conditional("yüksel"), "yükselirse");
        assert_eq!(aorist("gel"), "gelir");
    }

    #[test]
    fn caps() {
        assert_eq!(capitalize("bugün"), "Bugün");
        assert_eq!(capitalize("iki"), "İki");
    }
}
