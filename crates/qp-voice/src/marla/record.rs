//! Marla's ledger line: "Ne kadar emin? Daha önce oldu mu?" — the expectation, counted.

use super::*;

// ---------------------------------------------------------------------------
// Ledger: "Ne kadar emin? Daha önce oldu mu?" — the expectation itself, counted
// ---------------------------------------------------------------------------

pub(super) const LEDGER_LEVELS: usize = 3;

pub(super) fn ledger(f: &Frame<'_>, lang: Lang, level: usize) -> String {
    let l = f.ledger;
    let dp = dir_past(f.kind, f.dir, lang);
    if l.n == 0 {
        return match lang {
            Lang::Tr => "Henüz kapanmış benzer durum yok, ne sıklıkla tuttuğu bilinmiyor.".into(),
            Lang::En => "No similar closed cases yet, so no hit rate.".into(),
        };
    }
    let thin = l.n < LEDGER_MIN_N;
    if f.red {
        let s = span(f.horizon, f.unit, lang, true);
        if thin {
            return match lang {
                Lang::Tr => {
                    "Az örnek: benzer durum henüz az, sayılar veri yetkisiyle görünür.".into()
                }
                Lang::En => "Few similar cases yet; the counts need data access.".into(),
            };
        }
        let (tr_q, en_q) = if 10 * l.realized >= 6 * l.n {
            ("Benzer durumların çoğunda", "In most similar cases")
        } else if 10 * l.realized >= 4 * l.n {
            (
                "Benzer durumların yarısına yakınında",
                "In about half of similar cases",
            )
        } else {
            ("Benzer durumların azında", "In few similar cases")
        };
        return match (lang, level) {
            (Lang::Tr, 0) => format!("{tr_q} fiyat {s} içinde {dp}."),
            (Lang::Tr, _) => format!("{tr_q} {} {dp}.", span_loc_tr(f.horizon, f.unit, true)),
            (Lang::En, 0) => format!("{en_q} the price {dp} within {s}."),
            (Lang::En, _) => format!("{en_q} it {dp} within {s}."),
        };
    }
    let (nn, rr) = (l.n.to_string(), l.realized.to_string());
    let s = span(f.horizon, f.unit, lang, false);
    let sl = span_loc_tr(f.horizon, f.unit, false);
    let r_in = Word::number(&rr).poss3_loc();
    match (lang, thin, level) {
        (Lang::Tr, false, 0) => format!("Benzer {nn} durumun {r_in} fiyat {s} içinde {dp}."),
        (Lang::Tr, false, 1) => format!("Benzer {nn} durumun {r_in} {sl} {dp}."),
        (Lang::Tr, false, _) => format!("{nn} benzerin {r_in} {sl} {dp}."),
        (Lang::Tr, true, 0) => format!("Az örnek: {nn} benzerin {r_in} fiyat {s} içinde {dp}."),
        (Lang::Tr, true, 1) => format!("Az örnek: {nn} benzerin {r_in} {sl} {dp}."),
        (Lang::Tr, true, _) => format!("Az örnek: {nn}'de {rr}, {sl} {dp}."),
        (Lang::En, false, 0) => format!("In {rr} of {nn} similar cases the price {dp} within {s}."),
        (Lang::En, false, 1) => format!("In {rr} of {nn} similar cases it {dp} within {s}."),
        (Lang::En, false, _) => format!("{rr} of {nn} similar cases {dp} within {s}."),
        (Lang::En, true, 0) => format!("Few cases: the price {dp} in {rr} of {nn} within {s}."),
        (Lang::En, true, 1) => format!("Few cases: it {dp} in {rr} of {nn} within {s}."),
        (Lang::En, true, _) => format!("Few cases: {rr} of {nn} {dp} within {s}."),
    }
}
