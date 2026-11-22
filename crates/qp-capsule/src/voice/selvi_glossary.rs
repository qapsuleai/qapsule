//! The `selvi` glossary (R5/M17, K49): every technical word the voice puts on a screen
//! opens a short explanation, in Turkish and English, from this fixed table.
//!
//! Deterministic: an entry is a pure function of the term, the bar period, the story's
//! kind and direction (the track-record rule) and whether the reader may see digits (a
//! redacted reader's entries carry no digit, K33). No entry uses a product-internal word
//! (no "bar", "sinyal", "teyit", "aile", "olgu"…) and none suggests an action.
//!
//! | Term (TR / EN) | Opens on |
//! |---|---|
//! | `RSI` / `RSI` | RSI extremes and RSI–price divergences |
//! | `bant` / `band` | the narrowest-band (squeeze) capsule |
//! | `hacim` / `volume` | the volume capsule, and a volume note in the detail |
//! | `kurum` / `institutions` | the institutional net-flow capsule |
//! | `benzer` / `past cases` | every track-record line: what "a similar case" is and what counted as a hit |

use qp_schema::capsule::{Direction, MeaningKind};

use super::selvi::{GlossCtx, Span};
use crate::outcome::base_params;
use crate::realize::elapsed::Period;
use crate::realize::num::{fmt, Cat};
use crate::Lang;
use qp_facts::{BandSqueeze, RsiExtreme};

/// A glossary term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    /// RSI.
    Rsi,
    /// The price band (Bollinger).
    Band,
    /// Volume.
    Volume,
    /// Institutions (the broker net-flow feed).
    Institution,
    /// "Benzer" / "past cases": the track record's population and hit rule.
    Ledger,
}

/// The terms a detector's sentences use.
#[must_use]
pub fn terms_of(detector: &str) -> &'static [Term] {
    match detector {
        "extreme.rsi" | "reversal.rsi_divergence" => &[Term::Rsi],
        "squeeze.bandwidth" => &[Term::Band],
        "flow.volume_surge" => &[Term::Volume],
        "flow.institutional" => &[Term::Institution],
        _ => &[],
    }
}

/// Whether `term` is written in the Turkish `text` (lower-cased, Turkish i rule).
#[must_use]
pub fn appears(term: Term, text: &str, lang: Lang) -> bool {
    let l: String = text
        .chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .collect();
    let pats: &[&str] = match (term, lang) {
        (Term::Rsi, _) => &["rsı", "rsi"],
        (Term::Band, Lang::Tr) => &["bant"],
        (Term::Band, Lang::En) => &["band"],
        (Term::Volume, Lang::Tr) => &["hacim"],
        (Term::Volume, Lang::En) => &["volume"],
        (Term::Institution, Lang::Tr) => &["kurum"],
        (Term::Institution, Lang::En) => &["institution"],
        (Term::Ledger, Lang::Tr) => &["benzer"],
        (Term::Ledger, Lang::En) => &["past cases"],
    };
    pats.iter().any(|p| l.contains(p))
}

fn unit_adj(period: Period) -> (&'static str, &'static str) {
    let one = Span::one(period);
    match one.tr.as_str() {
        "gün" => ("günlük", "daily"),
        "saat" => ("saatlik", "hourly"),
        "dakika" => ("dakikalık", "per-minute"),
        "hafta" => ("haftalık", "weekly"),
        _ => ("aylık", "monthly"),
    }
}

#[allow(clippy::too_many_lines)]
/// One entry: `(term, explanation)` in `lang`.
#[must_use]
pub fn entry(term: Term, cx: &GlossCtx, lang: Lang) -> (String, String) {
    let red = cx.redacted;
    let tr = lang == Lang::Tr;
    match term {
        Term::Rsi => {
            let span = Span::of(cx.period, u32::try_from(RsiExtreme::RSI).unwrap_or(14));
            let (up, lo) = (
                fmt(RsiExtreme::UPPER, Cat::Osc, lang),
                fmt(RsiExtreme::LOWER, Cat::Osc, lang),
            );
            let text = match (tr, red) {
                (true, false) => format!(
                    "Göreceli güç endeksi: son {} kazançları kayıplarla tartar, 0 ile 100 arası. {up} üstü hızlı yükseliş, {lo} altı hızlı düşüş sayılır.",
                    span.trading_loc_ki().tr
                ),
                (false, false) => format!(
                    "Relative strength index: weighs gains against losses over the last {}, 0 to 100. Above {up} counts as a fast rise, below {lo} as a fast fall.",
                    span.trading().en
                ),
                (true, true) => "Göreceli güç endeksi: son günlerdeki kazançları kayıplarla tartar. Üst çizginin üstü hızlı yükseliş, alt çizginin altı hızlı düşüş sayılır.".to_owned(),
                (false, true) => "Relative strength index: weighs recent gains against recent losses. Above its upper line counts as a fast rise, below its lower line as a fast fall.".to_owned(),
            };
            ("RSI".to_owned(), text)
        }
        Term::Band => {
            let span = Span::of(cx.period, u32::try_from(BandSqueeze::PERIOD).unwrap_or(20));
            let a = span.adj();
            let a = (a.tr, a.en);
            let text = match (tr, red) {
                (true, false) => format!(
                    "Bollinger bandı: {} ortalama fiyatın iki yanına, fiyatın olağan salınımına göre çizilen alt ve üst sınır. Daraldıkça fiyat durgunlaşmış demektir.",
                    a.0
                ),
                (false, false) => format!(
                    "Bollinger band: lower and upper lines around the {} average price, set by its usual swing. A narrow band means the price has gone quiet.",
                    a.1
                ),
                (true, true) => "Bollinger bandı: ortalama fiyatın iki yanına, fiyatın olağan salınımına göre çizilen alt ve üst sınır. Daraldıkça fiyat durgunlaşmış demektir.".to_owned(),
                (false, true) => "Bollinger band: lower and upper lines around the average price, set by its usual swing. A narrow band means the price has gone quiet.".to_owned(),
            };
            (if tr { "bant" } else { "band" }.to_owned(), text)
        }
        Term::Volume => {
            let one = Span::one(cx.period);
            let text = if tr {
                format!("Bir {} içinde el değiştiren hisse adedi.", one.tr)
            } else {
                format!("The number of shares that changed hands in one {}.", one.en)
            };
            (if tr { "hacim" } else { "volume" }.to_owned(), text)
        }
        Term::Institution => {
            let text = if tr {
                "Hisse işlemlerine aracılık eden aracı kurumlar. Burada o gün en çok net işlem yapan kurum sayılır, bu veri lisanslıdır.".to_owned()
            } else {
                "Brokerage firms that carry out trades. Counted here: the broker with the largest net trade that day. This data is licensed.".to_owned()
            };
            (if tr { "kurum" } else { "institutions" }.to_owned(), text)
        }
        Term::Ledger => {
            let p = base_params(cx.kind);
            let neutral = cx.dir == Direction::Neutral;
            let k = if neutral { p.k_neutral } else { p.k };
            let kk = fmt(k, Cat::Osc, lang);
            let (adj_tr, adj_en) = unit_adj(cx.period);
            let head_tr = "Bütün hisselerde görülmüş aynı tür ve yönde durumlar.";
            let head_en = "Earlier cases of the same kind and direction on any stock.";
            let text = match (tr, red, cx.dir) {
                // Directional: the price reached k average swings its way before k the
                // other way (the barrier rule, `crate::outcome::Barrier`).
                (true, false, Direction::Up | Direction::Down) => format!(
                    "{head_tr} Tutan: fiyatın önce ters yöne dönmeden {adj_tr} ortalama salınımının {kk} katı {}.",
                    if cx.dir == Direction::Up { "yükselmesi" } else { "düşmesi" }
                ),
                (false, false, Direction::Up | Direction::Down) => format!(
                    "{head_en} A hit: the price {} {kk}× its average {adj_en} swing before going that far the other way.",
                    if cx.dir == Direction::Up { "rising" } else { "falling" }
                ),
                (true, true, Direction::Up | Direction::Down) => format!(
                    "{head_tr} Tutan: fiyatın önce ters yöne dönmeden olağan {adj_tr} salınımını aşacak kadar {}.",
                    if cx.dir == Direction::Up { "yükselmesi" } else { "düşmesi" }
                ),
                (false, true, Direction::Up | Direction::Down) => format!(
                    "{head_en} A hit: the price {} beyond its usual {adj_en} swing before going that far the other way.",
                    if cx.dir == Direction::Up { "rising" } else { "falling" }
                ),
                // No direction: a move of k long-run average swings either way in time.
                (true, false, Direction::Neutral) => format!(
                    "{head_tr} Tutan: fiyatın süre içinde uzun dönem {adj_tr} ortalama salınımının {kk} katı bir yöne gitmesi."
                ),
                (false, false, Direction::Neutral) => format!(
                    "{head_en} A hit: the price moving {kk}× its long-run average {adj_en} swing either way in time."
                ),
                (true, true, Direction::Neutral) => format!(
                    "{head_tr} Tutan: fiyatın süre içinde olağan {adj_tr} salınımının birkaç katı bir yöne gitmesi."
                ),
                (false, true, Direction::Neutral) => format!(
                    "{head_en} A hit: the price moving several times its usual {adj_en} swing either way in time."
                ),
            };
            (if tr { "benzer" } else { "past cases" }.to_owned(), text)
        }
    }
}

/// What happened, in plain words without technical terms (fallback headline and detail).
#[must_use]
pub fn plain_event(kind: MeaningKind, dir: Direction, lang: Lang) -> &'static str {
    use Direction::{Down, Neutral, Up};
    use MeaningKind::{Breakout, Continuation, Extreme, Flow, Reversal, Squeeze};
    let (tr, en) = match (kind, dir) {
        (Breakout, Up) => (
            "fiyat yakın dönemin en yükseğini aştı",
            "the price passed its recent high",
        ),
        (Breakout, Down) => (
            "fiyat yakın dönemin en düşüğünün altına indi",
            "the price fell below its recent low",
        ),
        (Breakout, Neutral) => (
            "fiyat yakın dönemin aralığından çıktı",
            "the price left its recent range",
        ),
        (Reversal, Up) => ("düşüş yukarı dönüyor olabilir", "a fall may be turning up"),
        (Reversal, Down) => (
            "yükseliş aşağı dönüyor olabilir",
            "a rise may be turning down",
        ),
        (Reversal, Neutral) => ("yön dönüyor olabilir", "the direction may be turning"),
        (Continuation, Up) => ("yükseliş düzeni", "an upward trend"),
        (Continuation, Down) => ("düşüş düzeni", "a downward trend"),
        (Continuation, Neutral) => ("yatay düzen", "a sideways trend"),
        (Squeeze, _) => (
            "fiyat dar bir aralığa sıkıştı",
            "the price got stuck in a narrow range",
        ),
        (Extreme, Up) => (
            "fiyat kısa sürede çok yükseldi",
            "the price rose a lot in a short time",
        ),
        (Extreme, Down) => (
            "fiyat kısa sürede çok düştü",
            "the price fell a lot in a short time",
        ),
        (Extreme, Neutral) => (
            "fiyat kısa sürede çok oynadı",
            "the price moved a lot in a short time",
        ),
        (Flow, Up) => ("alışılmadık para girişi", "money flowed in more than usual"),
        (Flow, Down) => (
            "alışılmadık para çıkışı",
            "money flowed out more than usual",
        ),
        (Flow, Neutral) => ("alışılmadık para hareketi", "money moved more than usual"),
    };
    match lang {
        Lang::Tr => tr,
        Lang::En => en,
    }
}
