//! `nobra`'s answers: the question, answer and detail texts of each ask, by language.

use super::*;

fn family_name(f: EvidenceFamily, l: Language) -> &'static str {
    match (f, l) {
        (EvidenceFamily::Price, Language::Tr) => "fiyat",
        (EvidenceFamily::Volume, Language::Tr) => "hacim",
        (EvidenceFamily::Flow, Language::Tr) => "para akışı",
        (EvidenceFamily::Derivative, Language::Tr) => "türev piyasa",
        (EvidenceFamily::News, Language::Tr) => "haber ve temel veri",
        (EvidenceFamily::Price, Language::En) => "price",
        (EvidenceFamily::Volume, Language::En) => "volume",
        (EvidenceFamily::Flow, Language::En) => "money flow",
        (EvidenceFamily::Derivative, Language::En) => "derivatives",
        (EvidenceFamily::News, Language::En) => "news and fundamentals",
    }
}

fn count_word(n: u8, l: Language) -> &'static str {
    match (n, l) {
        (0 | 1, Language::Tr) => "bir",
        (2, Language::Tr) => "iki",
        (3, Language::Tr) => "üç",
        (4, Language::Tr) => "dört",
        (_, Language::Tr) => "beş",
        (0 | 1, Language::En) => "one",
        (2, Language::En) => "two",
        (3, Language::En) => "three",
        (4, Language::En) => "four",
        (_, Language::En) => "five",
    }
}

/// "fiyat ve hacim verisi"; without the family names (a feed item), "iki bağımsız kanıt
/// ailesi" in words (no digit).
fn families_of(x: &Facts, l: Language) -> String {
    if x.families.is_empty() {
        let w = count_word(x.confirmations, l);
        return match (x.confirmations > 1, l) {
            (true, Language::Tr) => format!("{w} bağımsız kanıt ailesi"),
            (false, Language::Tr) => "tek bir kanıt ailesi".to_owned(),
            (true, Language::En) => format!("{w} independent evidence families"),
            (false, Language::En) => "a single evidence family".to_owned(),
        };
    }
    match l {
        Language::Tr => format!("{} verisi", families(&x.families, l)),
        Language::En => format!("{} data", families(&x.families, l)),
    }
}

/// "fiyat", "fiyat ve hacim", "fiyat, hacim ve para akışı".
fn families(fs: &[EvidenceFamily], l: Language) -> String {
    let names: Vec<&str> = fs.iter().map(|f| family_name(*f, l)).collect();
    let and = match l {
        Language::Tr => " ve ",
        Language::En => " and ",
    };
    match names.as_slice() {
        [] => family_name(EvidenceFamily::Price, l).to_owned(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{}{and}{last}", init.join(", ")),
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

pub(super) fn question(key: &str, l: Language) -> &'static str {
    match (key, l) {
        ("what", Language::Tr) => "Ne oldu?",
        ("who", Language::Tr) => "Kim doğruladı?",
        ("before", Language::Tr) => "Daha önce kaç kez?",
        ("why", Language::Tr) => "Neden rakam göremiyorum?",
        ("basis", Language::Tr) => "Neye dayanıyor?",
        ("what", Language::En) => "What happened?",
        ("who", Language::En) => "Who verified it?",
        ("before", Language::En) => "How often before?",
        ("why", Language::En) => "Why can't I see numbers?",
        ("basis", Language::En) => "What does it rest on?",
        _ => "",
    }
}

fn what_answer(x: &Facts, l: Language) -> String {
    let tr = l == Language::Tr;
    let s = x.stage;
    if s.is_closed() {
        return match (s, tr) {
            (Stage::Realized, true) => "Kapandı: beklenen oldu.",
            (Stage::Invalidated, true) => "Kapandı: geçersiz oldu, beklenen olmadı.",
            (Stage::Expired, true) => "Kapandı: süresi doldu, beklenen olmadı.",
            (_, true) => "Geri çekildi: dayandığı veri düzeltildi.",
            (Stage::Realized, false) => "Closed: it played out as expected.",
            (Stage::Invalidated, false) => "Closed: invalidated, it did not play out.",
            (Stage::Expired, false) => "Closed: its time ran out without playing out.",
            (_, false) => "Withdrawn: the data it rested on was corrected.",
        }
        .to_owned();
    }
    if x.validity == Some(Validity::NoLonger) {
        return if tr {
            "Artık geçerli değil; beklenen sonuç henüz belli değil."
        } else {
            "No longer holds; the expected outcome is still open."
        }
        .to_owned();
    }
    match (s, tr) {
        (Stage::Strengthened, true) => "Hâlâ geçerli; yeni bir olguyla güçlendi.",
        (Stage::Weakened, true) => "Hâlâ geçerli, ama karşı bir olguyla zayıfladı.",
        (Stage::Confirmed, true) => "Hâlâ geçerli; ikinci bir kanıt ailesi teyit etti.",
        (_, true) => "Hâlâ geçerli.",
        (Stage::Strengthened, false) => "Still holds; a new fact strengthened it.",
        (Stage::Weakened, false) => "Still holds, but a fact against it weakened it.",
        (Stage::Confirmed, false) => "Still holds; a second evidence family confirmed it.",
        (_, false) => "Still holds.",
    }
    .to_owned()
}

fn who_answer(check: Option<Check>, l: Language) -> &'static str {
    match (check, l) {
        (None, Language::Tr) => "Sunucu doğruladı.",
        (Some(Check::Checking), Language::Tr) => "Sunucu doğruladı; bu cihazda sınanıyor.",
        (Some(Check::Device), Language::Tr) => "Sunucu doğruladı ve bu cihazda da sınandı.",
        (Some(Check::Differs), Language::Tr) => {
            "Kaynaklar farklı: bu cihazın verisi başka sonuç veriyor."
        }
        (None, Language::En) => "Verified by the server.",
        (Some(Check::Checking), Language::En) => "Verified by the server; checking on this device.",
        (Some(Check::Device), Language::En) => {
            "Verified by the server and checked on this device too."
        }
        (Some(Check::Differs), Language::En) => {
            "Sources differ: this device's data gives another result."
        }
    }
}

fn who_detail(x: &Facts, check: Option<Check>, l: Language) -> String {
    let tr = l == Language::Tr;
    let base = match (x.trust == Trust::Server || x.licensed, tr) {
        (true, true) => "Bu iddia lisanslı veriden sunucuda hesaplandı. Bu cihaz o veriyi almadığı için yeniden hesaplayamaz; gördüğün, sunucunun kesinleşmiş iddiası.",
        (true, false) => "This claim was computed on the server from licensed data. This device does not receive that data and cannot recompute it; what you see is the server's settled claim.",
        (false, true) => "Akıştaki her kapsül sunucunun kesinleşmiş iddiasıdır. Açtığında bu cihaz aynı kuralı yalnız bu hissenin kendi verisinde yeniden sınar.",
        (false, false) => "Every capsule in the feed is the server's settled claim. When you open one, this device re-runs the same rule on its own data of this instrument only.",
    };
    let more = match (check, tr) {
        (None, _) => "",
        (Some(Check::Checking), true) => " Bu cihazın verisi henüz o barı kapsamıyor.",
        (Some(Check::Device), true) => " Bu cihaz kendi verisinde aynı sonucu buldu.",
        (Some(Check::Differs), true) => {
            " Bu cihaz kendi verisinde başka sonuç buldu; iki kaynaktan biri farklı."
        }
        (Some(Check::Checking), false) => " This device's data does not cover that bar yet.",
        (Some(Check::Device), false) => " This device found the same result on its own data.",
        (Some(Check::Differs), false) => {
            " This device found another result on its own data; one of the two sources differs."
        }
    };
    format!("{base}{more}")
}

fn ledger_answer(x: &Facts, l: Language) -> String {
    if let Some(v) = &x.voice {
        return v.ledger.clone();
    }
    let st = x.ledger;
    match (st.n, x.withheld, l) {
        (0, _, Language::Tr) => "Henüz karne yok.".to_owned(),
        (0, _, Language::En) => "No track record yet.".to_owned(),
        (_, true, Language::Tr) => "Karnesi var; ayrıntıya dokun.".to_owned(),
        (_, true, Language::En) => "It has a track record; tap for the detail.".to_owned(),
        (_, false, _) => crate::depth::ledger_text(&st),
    }
}

fn before_detail(x: &Facts, l: Language) -> String {
    let tr = l == Language::Tr;
    let st = x.ledger;
    let past = if tr {
        " Karne geleceği söylemez; geçmişteki sıklığı gösterir ve veri yetkisinden bağımsızdır."
    } else {
        " A track record does not tell the future; it shows a past frequency and does not depend on the data entitlement."
    };
    if st.n == 0 {
        return if tr {
            format!("Bu türden, bu kapsülden önce kapanmış bir hikâye yok.{past}")
        } else {
            format!("No story of this kind closed before this capsule.{past}")
        };
    }
    let few = st.n < GLYPH_MIN_LEDGER_N;
    if x.withheld {
        // No digit for this reader (K33): the counting rule, not the counts.
        return match (few, tr) {
            (true, true) => format!("Aynı tür, yön ve kuralla doğup bu kapsülden önce kapanan hikâyelerden sayılır; az örnek var, oran henüz güvenilir değil.{past}"),
            (false, true) => format!("Aynı tür, yön ve kuralla doğup bu kapsülden önce kapanan hikâyelerden sayılır.{past}"),
            (true, false) => format!("Counted over stories of the same kind, direction and rule that closed before this capsule; there are few samples, the rate is not reliable yet.{past}"),
            (false, false) => format!("Counted over stories of the same kind, direction and rule that closed before this capsule.{past}"),
        };
    }
    let (n, r) = (st.n, st.realized);
    let head = if tr {
        format!("Aynı tür, yön ve kuralla doğup bu kapsülden önce kapanan {n} hikâyenin {r} tanesinde beklenen oldu.")
    } else {
        format!("Of {n} stories of the same kind, direction and rule that closed before this capsule, {r} played out as expected.")
    };
    let few = match (few, tr) {
        (true, true) => " Az örnek: bu oran henüz güvenilir değil.",
        (true, false) => " Few samples: this rate is not reliable yet.",
        _ => "",
    };
    format!("{head}{few}{past}")
}

fn basis_answer(x: &Facts, l: Language) -> String {
    let fams = families_of(x, l);
    let tr = l == Language::Tr;
    let mut s = if tr {
        format!("{}ne dayanıyor.", capitalised(&fams))
    } else {
        format!("It rests on {fams}.")
    };
    if x.no_data {
        s.push_str(if tr {
            " Grafiği ve değerleri sende gizli."
        } else {
            " Its chart and values are withheld from you."
        });
    } else if x.withheld {
        s.push_str(if tr {
            " İşaretlerin değerleri sende gizli."
        } else {
            " Its marks' values are withheld from you."
        });
    } else if x.licensed {
        s.push_str(if tr {
            " Lisanslı veri: sunucuda hesaplandı."
        } else {
            " Licensed data: computed on the server."
        });
    }
    s
}

fn basis_detail(x: &Facts, l: Language) -> String {
    let tr = l == Language::Tr;
    let has = |k: usize| x.geometry[k];
    let mut parts: Vec<&str> = Vec::new();
    if x.no_data {
        return if tr {
            "Grafik ve kanıt işaretleri bu hissenin verisini gerektirir; veri yetkin olmadığı için çizilmedi."
        } else {
            "The chart and the evidence marks need this instrument's data; without the entitlement they are not drawn."
        }
        .to_owned();
    }
    if x.withheld {
        parts.push(if tr {
            "İşaretler, değerleri sende gizli olduğu için çizilmedi."
        } else {
            "The marks are not drawn: their values are withheld from you."
        });
    } else {
        if has(0) {
            parts.push(if tr {
                "Vurgulu çizgi, iddianın dayandığı seviye."
            } else {
                "The highlighted line is the level the claim rests on."
            });
        }
        if has(1) {
            parts.push(if tr {
                "Gölgeli bant, iddianın dayandığı aralık."
            } else {
                "The shaded band is the range the claim rests on."
            });
        }
        if has(2) {
            parts.push(if tr {
                "Noktalar, iddianın dayandığı dönüş noktaları."
            } else {
                "The points are the turns the claim rests on."
            });
        }
    }
    parts.push(if tr {
        "Grafiğe geçmek için grafiğe dokun."
    } else {
        "Tap the chart to open it."
    });
    parts.join(" ")
}

fn why_answer(l: Language) -> &'static str {
    match l {
        Language::Tr => "Kanıtın değerleri veri yetkisi gerektiriyor.",
        Language::En => "The evidence's values need a data entitlement.",
    }
}

fn why_detail(x: &Facts, l: Language) -> String {
    let fams = families_of(x, l);
    let i = &x.instrument;
    match (x.no_data, x.entitlement.as_deref(), l) {
        (true, _, Language::Tr) => format!("Kanıt var ve {fams}ne dayanıyor. Rakamları ve grafiği görmek için {i} verisinin yetkisi gerekiyor. Yorum ve karne herkese açık."),
        (false, Some(e), Language::Tr) => format!("Kanıt var ve {fams}ne dayanıyor. Değerlerini görmek için gereken yetki: {e}. Yorum ve karne herkese açık."),
        (false, None, Language::Tr) => format!("Kanıt var ve {fams}ne dayanıyor; değerleri bu okuyucuya açık değil. Yorum ve karne herkese açık."),
        (true, _, Language::En) => format!("The evidence exists and rests on {fams}. Seeing its numbers and chart needs the data entitlement for {i}. The comment and the track record are open to everyone."),
        (false, Some(e), Language::En) => format!("The evidence exists and rests on {fams}. The entitlement needed to see its values: {e}. The comment and the track record are open to everyone."),
        (false, None, Language::En) => format!("The evidence exists and rests on {fams}; its values are not open to this reader. The comment and the track record are open to everyone."),
    }
}

/// The text of one part of one question, from the provider's data (`None`: the question
/// does not apply, e.g. `why` for a reader who sees the values).
pub(super) fn text(
    x: &Facts,
    key: &str,
    part: &str,
    l: Language,
    check: Option<Check>,
) -> Option<String> {
    if key == "why" && !x.withheld {
        return None;
    }
    Some(match (key, part) {
        // A closed card says it opens ("+"), the open one that it closes ("−").
        (_, "q") => format!("+ {}", question(key, l)),
        (_, "qo") => format!("\u{2212} {}", question(key, l)),
        ("what", "a") => what_answer(x, l),
        ("what", "d") => match &x.voice {
            Some(v) => v.detail.clone(),
            None => x.claim.clone(),
        },
        ("who", "a") => who_answer(check, l).to_owned(),
        ("who", "d") => who_detail(x, check, l),
        ("before", "a") => ledger_answer(x, l),
        ("before", "d") => before_detail(x, l),
        ("why", "a") => why_answer(l).to_owned(),
        ("why", "d") => why_detail(x, l),
        ("basis", "a") => basis_answer(x, l),
        ("basis", "d") => basis_detail(x, l),
        _ => return None,
    })
}
