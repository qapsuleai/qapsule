//! Sözlük of the `marla` capsule language (R5/M17): the terms a `marla` screen may show,
//! each with a short explanation that opens on touch. Deterministic, Turkish and English,
//! one entry per [`Concept`]; the same concept gives one entry in both languages (the
//! entry counts are equal by construction).
//!
//! Rules every text here keeps (the M17 harness scans them like any other surface):
//!
//! - no digit (a redacted reader sees the glossary too, K33): numbers are words;
//! - no internal product term (bar, aile, teyit, olgu, sinyal…; K43, A33);
//! - no advice, no vague importance word ("önemli", "güçlü", "key", "strong"…);
//! - at most 160 characters per explanation.
//!
//! The time unit follows the bar period ([`UnitWords`]): a daily capsule says "gün", an
//! hourly one "saat".

use crate::Lang;

/// A term a screen can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Concept {
    /// RSI (göreceli güç endeksi).
    Rsi,
    /// The period's highest (`true`) or lowest (`false`) price (range breakout).
    HighLow(bool),
    /// A swing low (`true`: "dip") or swing high (`false`: "tepe") (divergence).
    Swing(bool),
    /// N-period moving average (trend order).
    Average,
    /// Bollinger band (squeeze).
    Band,
    /// Traded volume.
    Volume,
    /// Institutional net buying (`true`) or selling (`false`): licensed, server-side.
    Institutional(bool),
    /// "Artık geçerli değil": the condition broke, the outcome is still open.
    NoLonger,
    /// "Benzer durum": what the track-record sentence counts and what "tuttu" means.
    Similar,
}

/// The period's unit words used inside explanations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitWords {
    /// Turkish noun ("gün", "saat").
    pub tr: &'static str,
    /// Turkish locative ("günde", "saatte").
    pub tr_loc: &'static str,
    /// Turkish adjective suffix form ("günlük", "saatlik").
    pub tr_adj: &'static str,
    /// Turkish ablative plural ("günden", "saatten").
    pub tr_abl: &'static str,
    /// English noun, singular ("day", "hour").
    pub en: &'static str,
    /// English noun, plural ("days", "hours").
    pub en_pl: &'static str,
}

/// One glossary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gloss {
    /// The term as the screen writes it.
    pub term: String,
    /// Its explanation.
    pub text: String,
}

/// The entry of `c` in `lang` for a capsule whose period unit is `u`.
#[must_use]
pub fn entry(c: Concept, u: UnitWords, lang: Lang) -> Gloss {
    let (term, text) = match lang {
        Lang::Tr => tr(c, u),
        Lang::En => en(c, u),
    };
    Gloss { term, text }
}

#[allow(clippy::too_many_lines)]
fn tr(c: Concept, u: UnitWords) -> (String, String) {
    match c {
        Concept::Rsi => (
            "RSI".into(),
            "Göreceli güç endeksi: son yükselişlerin düşüşlere göre ağırlığını sıfırdan yüze \
             ölçer. Yetmiş üstü hızlı yükselmiş, otuz altı hızlı düşmüş sayılır."
                .into(),
        ),
        Concept::HighLow(true) => (
            "en yüksek fiyat".into(),
            format!(
                "Sayılan dönemde görülen en yüksek fiyat; içinde bulunulan {} hariç. Onu aşmak, \
                 fiyatın o dönemin bütün fiyatlarının üstüne çıkması demektir.",
                u.tr
            ),
        ),
        Concept::HighLow(false) => (
            "en düşük fiyat".into(),
            format!(
                "Sayılan dönemde görülen en düşük fiyat; içinde bulunulan {} hariç. Onun altına \
                 inmek, fiyatın o dönemin bütün fiyatlarının altına düşmesi demektir.",
                u.tr
            ),
        ),
        Concept::Swing(true) => (
            "dip".into(),
            format!(
                "Fiyatın, iki önceki ve iki sonraki {} daha aşağıda kaldığı nokta. Yeni dip \
                 öncekinin altındadır.",
                u.tr_abl
            ),
        ),
        Concept::Swing(false) => (
            "tepe".into(),
            format!(
                "Fiyatın, iki önceki ve iki sonraki {} daha yukarıda kaldığı nokta. Yeni tepe \
                 öncekinin üstündedir.",
                u.tr_abl
            ),
        ),
        Concept::Average => (
            "ortalama".into(),
            format!(
                "Son belirtilen sayıda kapanış fiyatının ortalaması, her {} yeniden hesaplanır. \
                 Fiyat kısa ortalamanın, kısa da uzunun üstündeyse yükseliş düzeni sayılır.",
                u.tr
            ),
        ),
        Concept::Band => (
            "bant".into(),
            format!(
                "Fiyatın yirmi {} ortalamasının iki yanında, son oynaklığa göre açılıp daralan \
                 aralık (Bollinger bandı). Dar bant fiyatın sakinleştiğini gösterir.",
                u.tr_adj
            ),
        ),
        Concept::Volume => (
            "hacim".into(),
            format!(
                "Bir {} el değiştiren hisse adedi. Ortalamasının birkaç katı, olağandan çok daha \
                 fazla işlem yapıldığı anlamına gelir.",
                u.tr_loc
            ),
        ),
        Concept::Institutional(buy) => (
            if buy {
                "kurumsal net alım"
            } else {
                "kurumsal net satım"
            }
            .into(),
            format!(
                "Aracı kurumların alışları ile satışları arasındaki fark, {} yönünde. Lisanslı \
                 veridir: sunucuda hesaplanır, bu cihaz kendisi sınayamaz.",
                if buy { "alış" } else { "satış" }
            ),
        ),
        Concept::NoLonger => (
            "artık geçerli değil".into(),
            "İddiayı doğuran koşul bugün tutmuyor, ama süresi dolmadı; sonucu henüz belli değil."
                .into(),
        ),
        Concept::Similar => (
            "benzer durum".into(),
            "Aynı tür ve yönde, bundan önce kapanmış iddialar. Fiyat süresinde olağan oynaklığı \
             kadar o yöne gittiyse tuttu sayılır; rastgele bir gün de yarı yarıya tutar."
                .into(),
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn en(c: Concept, u: UnitWords) -> (String, String) {
    match c {
        Concept::Rsi => (
            "RSI".into(),
            "Relative Strength Index: weighs recent gains against recent losses from zero to one \
             hundred. Above seventy counts as a fast rise, below thirty as a fast fall."
                .into(),
        ),
        Concept::HighLow(true) => (
            "high".into(),
            format!(
                "The highest price of the counted period, not counting the current {}. Breaking \
                 it means the price went above every price of that period.",
                u.en
            ),
        ),
        Concept::HighLow(false) => (
            "low".into(),
            format!(
                "The lowest price of the counted period, not counting the current {}. Breaking \
                 it means the price went below every price of that period.",
                u.en
            ),
        ),
        Concept::Swing(true) => (
            "low".into(),
            format!(
                "A point where the price sat below the two {} before and the two after it. A new \
                 low is below the previous one.",
                u.en_pl
            ),
        ),
        Concept::Swing(false) => (
            "high".into(),
            format!(
                "A point where the price sat above the two {} before and the two after it. A new \
                 high is above the previous one.",
                u.en_pl
            ),
        ),
        Concept::Average => (
            "average".into(),
            format!(
                "The mean of the last so many closing prices, recomputed every {}. Price above \
                 the short average, and the short above the long, counts as an uptrend.",
                u.en
            ),
        ),
        Concept::Band => (
            "band".into(),
            format!(
                "A range around the price's twenty-{} average that widens and narrows with recent \
                 volatility (Bollinger band). A narrow band means the price went quiet.",
                u.en
            ),
        ),
        Concept::Volume => (
            "volume".into(),
            format!(
                "The number of shares traded in one {}. Several times its average means far more \
                 trading than usual.",
                u.en
            ),
        ),
        Concept::Institutional(buy) => (
            if buy {
                "institutional net buying"
            } else {
                "institutional net selling"
            }
            .into(),
            format!(
                "Brokers' {}. Licensed data: computed on the server, so this device cannot check \
                 it itself.",
                if buy {
                    "purchases minus their sales"
                } else {
                    "sales minus their purchases"
                }
            ),
        ),
        Concept::NoLonger => (
            "no longer holds".into(),
            "The condition behind the claim does not hold today, but its time is not up, so the \
             outcome is still open."
                .into(),
        ),
        Concept::Similar => (
            "similar cases".into(),
            "Earlier claims of the same kind and direction, closed before this one. A case \
             counts if the price moved its usual swing that way in time; a random day: half."
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: UnitWords = UnitWords {
        tr: "gün",
        tr_loc: "günde",
        tr_adj: "günlük",
        tr_abl: "günden",
        en: "day",
        en_pl: "days",
    };

    #[test]
    fn every_entry_is_short_and_digit_free() {
        let all = [
            Concept::Rsi,
            Concept::HighLow(true),
            Concept::HighLow(false),
            Concept::Swing(true),
            Concept::Swing(false),
            Concept::Average,
            Concept::Band,
            Concept::Volume,
            Concept::Institutional(true),
            Concept::Institutional(false),
            Concept::NoLonger,
            Concept::Similar,
        ];
        for c in all {
            for l in [Lang::Tr, Lang::En] {
                let g = entry(c, DAY, l);
                assert!(g.text.chars().count() <= 160, "{c:?} {l:?} {}", g.text);
                assert!(!g.text.bytes().any(|b| b.is_ascii_digit()), "{}", g.text);
                assert!(!g.term.bytes().any(|b| b.is_ascii_digit()), "{}", g.term);
            }
        }
    }
}
