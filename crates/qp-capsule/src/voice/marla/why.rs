//! Marla's detail: "Neden?" — why, with the numbers, and where things stand now.

use super::*;

// ---------------------------------------------------------------------------
// Detail: "Neden?" + now + what changed since
// ---------------------------------------------------------------------------

/// Detail compositions, richest first: (now, time left, change sentence).
pub(super) const DETAIL_MASKS: [(bool, bool, bool); 8] = [
    (true, true, true),
    (true, false, true),
    (false, true, true),
    (false, false, true),
    (true, true, false),
    (true, false, false),
    (false, true, false),
    (false, false, false),
];

pub(super) fn lead_concepts(f: &Frame<'_>) -> Vec<Concept> {
    det_concepts(f.det, f.up())
}

pub(super) fn det_concepts(det: &str, up: bool) -> Vec<Concept> {
    match det {
        "breakout.range" => vec![Concept::HighLow(up)],
        "extreme.rsi" => vec![Concept::Rsi],
        "reversal.rsi_divergence" => vec![Concept::Rsi, Concept::Swing(up)],
        "continuation.ma_stack" => vec![Concept::Average],
        "squeeze.bandwidth" => vec![Concept::Band],
        "flow.volume_surge" => vec![Concept::Volume],
        "flow.institutional" => vec![Concept::Institutional(up)],
        _ => Vec::new(),
    }
}

/// The numbers behind the claim, each against its reference.
#[allow(clippy::too_many_lines)]
pub(super) fn why(f: &Frame<'_>, lang: Lang) -> String {
    if f.red {
        return match (f.det, lang) {
            ("flow.institutional", Lang::Tr) => {
                "Tutarlar lisanslı veridir: sunucu hesapladı ve doğruladı, kurumsal akış veri \
                 yetkisiyle görünür."
                    .into()
            }
            ("flow.institutional", Lang::En) => {
                "The amounts are licensed, computed and verified by the server, shown with \
                 institutional flow access."
                    .into()
            }
            (_, Lang::Tr) => {
                "Değerler lisanslı veridir: sunucu hesapladı ve doğruladı, veri yetkisiyle görünür."
                    .into()
            }
            (_, Lang::En) => {
                "The values are licensed, computed and verified by the server, shown with data \
                 access."
                    .into()
            }
        };
    }
    let s = why_numbers(f, lang);
    if f.server {
        // Server-computed claim (K38): say who verified it, inside the same sentence.
        let mark = match lang {
            Lang::Tr => " (sunucu doğruladı)",
            Lang::En => " (verified by the server)",
        };
        if let Some(body) = s.strip_suffix('.') {
            return format!("{body}{mark}.");
        }
    }
    s
}

#[allow(clippy::too_many_lines)]
fn why_numbers(f: &Frame<'_>, lang: Lang) -> String {
    let u = f.unit;
    let up = f.up();
    let p = |x: f64| n(x, Cat::Price, lang);
    let o = |x: f64| n(x, Cat::Osc, lang);
    match f.det {
        "breakout.range" => match (f.v("close"), f.v("level")) {
            (Some(c), Some(l)) => match lang {
                Lang::Tr => format!(
                    "{} kapanış {} oldu, eşiğin {} {}.",
                    anchor(f, lang),
                    p(c),
                    pct(c, l, lang),
                    if up { "üstünde" } else { "altında" }
                ),
                Lang::En => format!(
                    "{} it closed at {}, {} {}.",
                    anchor(f, lang),
                    p(c),
                    pct(c, l, lang),
                    if up {
                        "above the high"
                    } else {
                        "below the low"
                    }
                ),
            },
            _ => generic_why(lang),
        },
        "extreme.rsi" => match (f.v("prev"), f.v("value")) {
            (Some(a), Some(b)) => match lang {
                Lang::Tr => format!(
                    "RSI {} {} {} {}.",
                    if f.validity == Validity::Closed {
                        f.when(lang)
                    } else {
                        that(u, lang).to_lowercase()
                    },
                    abl(&o(a)),
                    dat(&o(b)),
                    if up { "çıktı" } else { "indi" }
                ),
                Lang::En => format!("{} RSI went from {} to {}.", anchor(f, lang), o(a), o(b)),
            },
            _ => generic_why(lang),
        },
        "reversal.rsi_divergence" => {
            match (f.v("p1_price"), f.v("p2_price"), f.v("p1_rsi"), f.v("p2_rsi")) {
                (Some(p1), Some(p2), Some(r1), Some(r2)) => match (lang, up) {
                    (Lang::Tr, true) => format!(
                        "Fiyat önceki dibin ({}) altına, {} indi, RSI ise {} {} çıktı.",
                        p(p1),
                        dat(&p(p2)),
                        abl(&o(r1)),
                        dat(&o(r2))
                    ),
                    (Lang::Tr, false) => format!(
                        "Fiyat önceki tepenin ({}) üstüne, {} çıktı, RSI ise {} {} indi.",
                        p(p1),
                        dat(&p(p2)),
                        abl(&o(r1)),
                        dat(&o(r2))
                    ),
                    (Lang::En, true) => format!(
                        "The price fell below the previous low ({}) to {}, while RSI rose from {} to {}.",
                        p(p1),
                        p(p2),
                        o(r1),
                        o(r2)
                    ),
                    (Lang::En, false) => format!(
                        "The price rose above the previous high ({}) to {}, while RSI fell from {} to {}.",
                        p(p1),
                        p(p2),
                        o(r1),
                        o(r2)
                    ),
                },
                _ => generic_why(lang),
            }
        }
        "continuation.ma_stack" => match (f.v("close"), f.v("sma20"), f.v("sma50")) {
            (Some(c), Some(a), Some(b)) => {
                let fa = param(MaStack::FAST, lang, false);
                let fb = param(MaStack::SLOW, lang, false);
                match lang {
                    Lang::Tr => format!(
                        "Kapanış ({}) {} ortalamanın ({}), o da {} ({}) {}.",
                        p(c),
                        tr_adj(&fa, u),
                        p(a),
                        Word::noun(&tr_adj(&fb, u)).gen(),
                        p(b),
                        if up { "üstündeydi" } else { "altındaydı" }
                    ),
                    Lang::En => format!(
                        "The close ({}) was {} the {} average ({}), itself {} the {} ({}).",
                        p(c),
                        if up { "above" } else { "below" },
                        en_adj(&fa, u),
                        p(a),
                        if up { "above" } else { "below" },
                        en_adj(&fb, u),
                        p(b)
                    ),
                }
            }
            _ => generic_why(lang),
        },
        "squeeze.bandwidth" => match (f.v("lower"), f.v("upper"), f.v("middle")) {
            (Some(lo), Some(hi), Some(mid)) => {
                let w = if mid.abs() > 0.0 {
                    (hi - lo) / mid * 100.0
                } else {
                    0.0
                };
                match lang {
                    Lang::Tr => format!(
                        "Bant {} ile {} arasında (genişliği fiyatın %{} kadarı).",
                        p(lo),
                        p(hi),
                        o(w)
                    ),
                    Lang::En => format!(
                        "The band runs from {} to {} ({}% of the price wide).",
                        p(lo),
                        p(hi),
                        o(w)
                    ),
                }
            }
            _ => generic_why(lang),
        },
        "flow.volume_surge" => match (f.v("volume"), f.v("vol_avg"), f.v("ratio")) {
            (Some(v), Some(a), Some(r)) => {
                let k = param(VolumeSurge::WINDOW, lang, false);
                match lang {
                    Lang::Tr => format!(
                        "Hacim {}, son {} ortalaması {}: {} katı.",
                        n(v, Cat::Count, lang),
                        tr_gen(&k, u),
                        n(a, Cat::Count, lang),
                        o(r)
                    ),
                    Lang::En => format!(
                        "Volume was {} against a {} average of {}: {} times.",
                        n(v, Cat::Count, lang),
                        en_adj(&k, u),
                        n(a, Cat::Count, lang),
                        o(r)
                    ),
                }
            }
            _ => generic_why(lang),
        },
        "flow.institutional" => match (f.v("net"), f.v("avg5")) {
            (Some(net), Some(avg)) => {
                let k = param(InstitutionalFlow::WINDOW, lang, false);
                let r = if avg.abs() > 0.0 {
                    Some(net.abs() / avg.abs())
                } else {
                    None
                };
                let tail = r.map(|r| format!(": {}", o(r))).unwrap_or_default();
                match lang {
                    Lang::Tr => format!(
                        "Net {} {}, son {} ortalaması {}{}{}.",
                        if up { "alım" } else { "satım" },
                        n(net, Cat::Count, lang),
                        tr_gen(&k, u),
                        n(avg, Cat::Count, lang),
                        tail,
                        if r.is_some() { " katı" } else { "" }
                    ),
                    Lang::En => format!(
                        "Net {} was {} against a {} average of {}{}{}.",
                        if up { "buying" } else { "selling" },
                        n(net, Cat::Count, lang),
                        en_adj(&k, u),
                        n(avg, Cat::Count, lang),
                        tail,
                        if r.is_some() { " times" } else { "" }
                    ),
                }
            }
            _ => generic_why(lang),
        },
        _ => generic_why(lang),
    }
}

fn generic_why(lang: Lang) -> String {
    match lang {
        Lang::Tr => "Dayandığı değerler kanıt ekranında.".into(),
        Lang::En => "The values behind it are on the evidence screen.".into(),
    }
}

/// Where things stand now against the threshold (open stories with a current value),
/// without a final period. `None` when there is nothing to say or the side disagrees
/// with the validity (never contradict the face).
pub(super) fn now(f: &Frame<'_>, lang: Lang) -> Option<String> {
    if f.fresh {
        return None;
    }
    let cur = f.current?;
    let up = f.up();
    let holds = f.validity == Validity::Valid;
    match f.det {
        "breakout.range" => {
            let l = f.v("level")?;
            let above = cur > l;
            if above != (up == holds) {
                return None;
            }
            let side = |a: bool, lang: Lang| match (lang, a) {
                (Lang::Tr, true) => "üstünde",
                (Lang::Tr, false) => "altında",
                (Lang::En, true) => "above",
                (Lang::En, false) => "below",
            };
            Some(match lang {
                Lang::Tr => format!(
                    "şimdi {} (eşiğin {} {})",
                    n(cur, Cat::Price, lang),
                    pct(cur, l, lang),
                    side(above, lang)
                ),
                Lang::En => format!(
                    "now {} ({} {} it)",
                    n(cur, Cat::Price, lang),
                    pct(cur, l, lang),
                    side(above, lang)
                ),
            })
        }
        "extreme.rsi" => {
            let l = f.ev.get("level").copied()?;
            let above = cur > l;
            if above != (up == holds) {
                return None;
            }
            let ls = n(l, Cat::Osc, lang);
            Some(match lang {
                Lang::Tr => format!(
                    "şimdi RSI {} ({} {})",
                    n(cur, Cat::Osc, lang),
                    Word::number(&ls).gen(),
                    if above { "üstünde" } else { "altında" }
                ),
                Lang::En => format!(
                    "RSI is now {} ({} {ls})",
                    n(cur, Cat::Osc, lang),
                    if above { "above" } else { "below" }
                ),
            })
        }
        "continuation.ma_stack" if holds => Some(match lang {
            Lang::Tr => format!("şimdi kapanış {}", n(cur, Cat::Price, lang)),
            Lang::En => format!("the close is now {}", n(cur, Cat::Price, lang)),
        }),
        _ => None,
    }
}

/// How long until the outcome is known, without a final period.
pub(super) fn remaining(f: &Frame<'_>, lang: Lang) -> Option<String> {
    let r = f.remaining?;
    if r == 0 {
        let w = f.unit.words();
        return Some(match lang {
            Lang::Tr => format!("sonuç bu {} belli olacak", w.tr),
            Lang::En => format!("the outcome is due this {}", w.en),
        });
    }
    let s = span(r, f.unit, lang, f.red);
    Some(match lang {
        Lang::Tr => format!("sonuç en geç {s} içinde belli olacak"),
        Lang::En => format!("the outcome is due within {s}"),
    })
}

/// What "played out" means for this claim, as a past verb phrase.
pub(super) fn dir_past(kind: MeaningKind, dir: Direction, lang: Lang) -> &'static str {
    use Direction::{Down, Neutral, Up};
    use MeaningKind::{Breakout, Continuation, Extreme, Flow, Reversal, Squeeze};
    match (kind, dir, lang) {
        (_, Neutral, Lang::Tr) | (Squeeze, _, Lang::Tr) => "dar aralıktan çıktı",
        (_, Neutral, Lang::En) | (Squeeze, _, Lang::En) => "broke out",
        (Extreme, Up, Lang::Tr) => "daha da yükseldi",
        (Extreme, Down, Lang::Tr) => "daha da düştü",
        (Extreme, Up, Lang::En) => "rose further",
        (Extreme, Down, Lang::En) => "fell further",
        (Reversal, Up, Lang::Tr) => "yukarı döndü",
        (Reversal, Down, Lang::Tr) => "aşağı döndü",
        (Reversal, Up, Lang::En) => "turned up",
        (Reversal, Down, Lang::En) => "turned down",
        (Continuation, Up, Lang::Tr) => "yükselişini sürdürdü",
        (Continuation, Down, Lang::Tr) => "düşüşünü sürdürdü",
        (Continuation, Up, Lang::En) => "kept rising",
        (Continuation, Down, Lang::En) => "kept falling",
        (Breakout | Flow, Up, Lang::Tr) => "yükseldi",
        (Breakout | Flow, Down, Lang::Tr) => "düştü",
        (Breakout | Flow, Up, Lang::En) => "rose",
        (Breakout | Flow, Down, Lang::En) => "fell",
    }
}
