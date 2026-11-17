//! Glossary of the `tunca` voice: the terms a screen shows, each opened by a tap.
//!
//! Deterministic, Turkish and English with the same entries in the same order. A screen
//! gets three entries in a fixed order (the same layering as the detail):
//!
//! 1. the detector's concept (the threshold's name: "son 20 günün zirvesi", "RSI", "Bant"…),
//! 2. the track record ("Karne" / "Record": what *tuttu / played out* means),
//! 3. the validity word on the face ("Hâlâ geçerli", "Tutmadı"…).
//!
//! Spans follow the bar period (`son 20 işlem günü`, `the last 20 trading hours`). For a
//! redacted reader (K33) every number is written in words, so no digit reaches the screen.

use qp_schema::capsule::Validity;

use super::tunca::{Gloss, GlossCtx};
use crate::realize::morph::capitalize;
use crate::realize::parts::Ending;
use crate::Lang;

/// The detector concept a screen explains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Concept {
    /// The range high (`breakout.range`, up).
    RangeHigh,
    /// The range low (`breakout.range`, down).
    RangeLow,
    /// RSI (`extreme.rsi`, `reversal.rsi_divergence`).
    Rsi,
    /// The moving averages (`continuation.ma_stack`).
    Average,
    /// The band (`squeeze.bandwidth`).
    Band,
    /// Volume (`flow.volume_surge`).
    Volume,
    /// Institutional net flow (`flow.institutional`).
    Institutional,
}

fn g(term: impl Into<String>, text: impl Into<String>) -> Gloss {
    Gloss {
        term: term.into(),
        text: text.into(),
    }
}

fn u(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A number as digits, or in words for a redacted reader.
fn n(c: &GlossCtx, x: u64) -> String {
    if c.sp.red {
        super::tunca::words(x, c.sp.lang)
    } else {
        x.to_string()
    }
}

fn concept(c: &GlossCtx) -> Gloss {
    let sp = &c.sp;
    let tr = sp.lang == Lang::Tr;
    match c.concept {
        Concept::RangeHigh | Concept::RangeLow => {
            let w = u(qp_facts::RangeBreakout::WINDOW);
            let high = c.concept == Concept::RangeHigh;
            match (tr, high) {
                (true, true) => g(
                    format!("{} zirvesi", capitalize(&sp.last_short(w))),
                    format!(
                        "{} en yüksek fiyatı. Fiyatın onu aşması, yakın dönemin üst sınırını geçmesi demektir.",
                        sp.last_gen(w)
                    ),
                ),
                (true, false) => g(
                    format!("{} dibi", capitalize(&sp.last_short(w))),
                    format!(
                        "{} en düşük fiyatı. Fiyatın onun altına inmesi, yakın dönemin alt sınırını geçmesi demektir.",
                        sp.last_gen(w)
                    ),
                ),
                (false, true) => g(
                    format!("{} high", sp.adj(w)),
                    format!(
                        "The highest price of {}. Moving above it means price left its recent upper limit.",
                        sp.last_gen(w)
                    ),
                ),
                (false, false) => g(
                    format!("{} low", sp.adj(w)),
                    format!(
                        "The lowest price of {}. Moving below it means price left its recent lower limit.",
                        sp.last_gen(w)
                    ),
                ),
            }
        }
        Concept::Rsi => {
            let w = u(qp_facts::RsiExtreme::RSI);
            let (hi, lo, top) = (n(c, 70), n(c, 30), n(c, 100));
            let zero = n(c, 0);
            if tr {
                g(
                    "RSI",
                    format!(
                        "Göreceli güç endeksi: {} yükseliş ve düşüşlerini {zero} ile {top} arasında özetler. {hi} üstü hızlı yükseliş, {lo} altı hızlı düşüş sayılır.",
                        sp.last_gen(w)
                    ),
                )
            } else {
                g(
                    "RSI",
                    format!(
                        "Relative Strength Index: gains vs losses of {} on a {zero} to {top} scale. Above {hi} means a fast rise, below {lo} a fast fall.",
                        sp.last_gen(w)
                    ),
                )
            }
        }
        Concept::Average => {
            let (f, s) = (u(qp_facts::MaStack::FAST), u(qp_facts::MaStack::SLOW));
            if tr {
                g(
                    "Ortalama",
                    format!(
                        "{} ortalama, {} kapanış fiyatlarının ortalamasıdır ({} de öyle). Fiyat ikisinin de üstündeyse eğilim yukarı sayılır.",
                        sp.adj(f),
                        sp.last_gen(f),
                        sp.adj(s)
                    ),
                )
            } else {
                g(
                    "Average",
                    format!(
                        "The {} average is the mean close of {} (the {} likewise). Price above both counts as an uptrend.",
                        sp.adj(f),
                        sp.last_gen(f),
                        sp.adj(s)
                    ),
                )
            }
        }
        Concept::Band => {
            let w = u(qp_facts::BandSqueeze::PERIOD);
            if tr {
                g(
                    "Bant",
                    format!(
                        "{} ortalamanın iki yanına, fiyatın olağan oynaklığına göre çizilen üst ve alt sınır. Bant daralınca fiyat dar bir aralıkta gider.",
                        sp.adj(w)
                    ),
                )
            } else {
                g(
                    "Band",
                    format!(
                        "Upper and lower lines around the {} average, set by the price's usual swings. A narrow band means price moves in a tight range.",
                        sp.adj(w)
                    ),
                )
            }
        }
        Concept::Volume => {
            if tr {
                g(
                    "Hacim",
                    "Bir dönemde el değiştiren hisse adedi. Ortalamasının katlarına çıkması, o dönem olağandan çok alım satım olduğunu gösterir.",
                )
            } else {
                g(
                    "Volume",
                    "The number of shares traded in a period. A multiple of its average means far more trading than usual.",
                )
            }
        }
        Concept::Institutional => {
            if tr {
                g(
                    "Kurumsal",
                    "Kurumsal net alış ya da satış: kurumsal yatırımcıların aldığı ile sattığı hisse arasındaki fark. Lisanslı veridir, rakamları lisansla görülür.",
                )
            } else {
                g(
                    "Institutional",
                    "Institutional net buying or selling: shares bought minus shares sold by institutional investors. Licensed data, its figures need a licence.",
                )
            }
        }
    }
}

fn record(c: &GlossCtx) -> Gloss {
    let tr = c.sp.lang == Lang::Tr;
    match (tr, c.neutral) {
        (true, false) => g(
            "Karne",
            "Geçmişteki benzer durumların sayımı: fiyat süre içinde, olağan oynaklığına göre ölçülen mesafeyi yazılan yönde gittiyse tuttu sayılır.",
        ),
        (true, true) => g(
            "Karne",
            "Geçmişteki benzer durumların sayımı: fiyat süre içinde, olağan oynaklığına göre ölçülen mesafeyi herhangi bir yönde gittiyse tuttu sayılır.",
        ),
        (false, false) => g(
            "Record",
            "A count of similar past cases: one played out if price moved a set distance, scaled to its usual swings, the stated way within the period.",
        ),
        (false, true) => g(
            "Record",
            "A count of similar past cases: one played out if price moved a set distance, scaled to its usual swings, either way within the period.",
        ),
    }
}

fn validity(c: &GlossCtx) -> Gloss {
    let tr = c.sp.lang == Lang::Tr;
    let (t, e) = match (c.validity, c.ending) {
        (Validity::Valid, _) => (
            g(
                "Hâlâ geçerli",
                "İddia henüz kapanmadı ve bugüne kadar onu bozan bir şey olmadı.",
            ),
            g(
                "Still holds",
                "The claim is still open and nothing has broken it so far.",
            ),
        ),
        (Validity::NoLonger, _) => (
            g(
                "Artık geçerli değil",
                "İddianın dayandığı koşul bugün sağlanmıyor. İddia henüz kapanmadı, sonucu açık.",
            ),
            g(
                "No longer holds",
                "The condition behind the claim is not met today. The claim is not closed yet, its outcome is open.",
            ),
        ),
        (Validity::Closed, Some(Ending::Realized)) if c.neutral => (
            g(
                "Gerçekleşti",
                "Fiyat süre dolmadan ölçülen mesafeyi herhangi bir yönde gitti ve iddia kapandı.",
            ),
            g(
                "Played out",
                "Before the period ended, price moved the set distance either way and the claim closed.",
            ),
        ),
        (Validity::Closed, Some(Ending::Realized)) => (
            g(
                "Gerçekleşti",
                "Fiyat süre dolmadan ölçülen mesafeyi yazılan yönde gitti ve iddia kapandı.",
            ),
            g(
                "Played out",
                "Before the period ended, price moved the set distance the stated way and the claim closed.",
            ),
        ),
        (Validity::Closed, Some(Ending::Failed)) if c.neutral => (
            g(
                "Tutmadı",
                "Fiyat süre boyunca dar aralıkta kaldı ve iddia kapandı.",
            ),
            g(
                "Did not play out",
                "Price stayed in a tight range for the whole period and the claim closed.",
            ),
        ),
        (Validity::Closed, Some(Ending::Failed)) => (
            g(
                "Tutmadı",
                "Fiyat süre dolmadan aynı mesafeyi ters yönde gitti ve iddia kapandı.",
            ),
            g(
                "Did not play out",
                "Before the period ended, price moved the same distance the other way and the claim closed.",
            ),
        ),
        (Validity::Closed, Some(Ending::Expired)) => (
            g(
                "Süresi doldu",
                "Süre içinde fiyat ölçülen mesafeye iki yönde de ulaşmadı. İddia sonuçsuz kapandı.",
            ),
            g(
                "Ran out of time",
                "Within the period price reached the set distance neither way. The claim closed without an outcome.",
            ),
        ),
        (Validity::Closed, Some(Ending::Retracted)) => (
            g(
                "Geri çekildi",
                "Dayandığı veri sonradan düzeltildi. Karneye sayılmaz.",
            ),
            g(
                "Withdrawn",
                "The data behind it was corrected later. It does not count in the record.",
            ),
        ),
        (Validity::Closed, None) => (
            g("Kapandı", "İddia kapandı, artık izlenmiyor."),
            g("Closed", "The claim is closed and no longer followed."),
        ),
    };
    if tr {
        t
    } else {
        e
    }
}

/// The glossary of one screen (three entries, fixed order).
#[must_use]
pub(crate) fn glossary(c: &GlossCtx) -> Vec<Gloss> {
    vec![concept(c), record(c), validity(c)]
}
