//! Marla's detail, continued: how a closed story ended and what changed since birth.

use super::*;

/// How a closed story ended. The face says when it closed ([`closed_chip`]); this
/// sentence says what the price did, and when the claim began unless the "why" sentence
/// already anchors it in time.
pub(super) fn closing(f: &Frame<'_>, lang: Lang) -> String {
    let s = span(f.horizon, f.unit, lang, f.red);
    let neutral = f.dir == Direction::Neutral || f.kind == MeaningKind::Squeeze;
    let dp = dir_past(f.kind, f.dir, lang);
    let what = match (f.ending, lang) {
        (Some(Ending::Retracted), Lang::Tr) => "dayandığı veri sonradan düzeltildi".to_owned(),
        (Some(Ending::Retracted), Lang::En) => "the data behind it was later corrected".to_owned(),
        (Some(Ending::Realized), Lang::Tr) => format!("fiyat {s} dolmadan {dp}"),
        (Some(Ending::Realized), Lang::En) => format!("the price {dp} within {s}"),
        (Some(Ending::Failed), Lang::Tr) if neutral => {
            format!("fiyat {s} boyunca dar aralıkta kaldı")
        }
        (Some(Ending::Failed), Lang::En) if neutral => {
            format!("the price stayed in a narrow range for {s}")
        }
        (Some(Ending::Failed), Lang::Tr) => {
            format!("fiyat {s} dolmadan ters yöne aynı ölçüde gitti")
        }
        (Some(Ending::Failed), Lang::En) => {
            format!("the price moved as far the other way within {s}")
        }
        (Some(Ending::Expired), Lang::Tr) => {
            format!("{s} doldu, fiyat iki yöne de yeterince gitmedi")
        }
        (Some(Ending::Expired), Lang::En) => {
            format!("{s} passed with too small a move either way")
        }
        (None, Lang::Tr) => "iddia kapandı".to_owned(),
        (None, Lang::En) => "the claim closed".to_owned(),
    };
    if anchored(f) {
        return format!("{}.", capitalize(&what));
    }
    let w = f.when(lang);
    match lang {
        Lang::Tr => format!("{} başladı, {what}.", capitalize(&w)),
        Lang::En => format!("It began {w}, and {what}."),
    }
}

/// Whether the "why" sentence names the event's time itself.
fn anchored(f: &Frame<'_>) -> bool {
    !f.red && matches!(f.det, "breakout.range" | "extreme.rsi")
}

/// The time anchor of the "why" sentence: "O gün" for an open claim (the face shows
/// when it began), the birth phrase for a closed one (the face shows when it closed).
pub(super) fn anchor(f: &Frame<'_>, lang: Lang) -> String {
    if f.validity == Validity::Closed {
        capitalize(&f.when(lang))
    } else {
        that(f.unit, lang).to_owned()
    }
}

/// The last stage change (strengthened / weakened / second kind of evidence) and what
/// brought it, with when.
pub(super) fn change(f: &Frame<'_>, lang: Lang) -> Option<(String, Vec<Concept>)> {
    let mv = f.moved(lang);
    let (clause, concepts) = match f.latest.as_ref() {
        Some(l) => {
            let (c, k) = latest_clause(f, l, lang);
            (Some(c), k)
        }
        None => (None, Vec::new()),
    };
    let tail = clause.map(|c| format!(": {c}")).unwrap_or_default();
    let s = match (f.stage, lang) {
        (Stage::Strengthened, Lang::Tr) => format!("{} güçlendi{tail}.", capitalize(&mv)),
        (Stage::Strengthened, Lang::En) => format!("Strengthened {mv}{tail}."),
        (Stage::Confirmed, Lang::Tr) => format!(
            "{} başka bir ölçü de aynı yönü gösterdi{tail}.",
            capitalize(&mv)
        ),
        (Stage::Confirmed, Lang::En) => {
            format!(
                "{}, a second measure pointed the same way{tail}.",
                capitalize(&mv)
            )
        }
        (Stage::Weakened, Lang::Tr) => format!("{} zayıfladı{tail}.", capitalize(&mv)),
        (Stage::Weakened, Lang::En) => format!("Weakened {mv}{tail}."),
        _ => return None,
    };
    Some((s, concepts))
}

/// What the latest fact said, in plain words (numbers only when its data is public and
/// the reader is not redacted).
#[allow(clippy::too_many_lines)]
fn latest_clause(f: &Frame<'_>, l: &Latest<'_>, lang: Lang) -> (String, Vec<Concept>) {
    let u = f.unit;
    let red = !l.open;
    let up = l.dir == Direction::Up;
    let ev = |k: &str| {
        if red {
            None
        } else {
            l.ev.get(k).copied().filter(|x| x.is_finite())
        }
    };
    let exit = l.edge == Edge::Exit || l.code.ends_with("_end");
    // The same detector again without numbers: say "again" so the sentence does not
    // read as the headline once more.
    let again = l.det == f.det;
    let (tr_again, en_again) = if again {
        ("yeniden ", "again ")
    } else {
        ("", "")
    };
    let concepts = det_concepts(l.det, up);
    let s = match l.det {
        "breakout.range" => {
            let k = param(RangeBreakout::WINDOW, lang, red);
            match (lang, up) {
                (Lang::Tr, true) => format!("fiyat {} en yüksek fiyatını geçti", tr_gen(&k, u)),
                (Lang::Tr, false) => format!("fiyat {} en düşük fiyatını kırdı", tr_gen(&k, u)),
                (Lang::En, true) => format!("the price broke its {} high", en_adj(&k, u)),
                (Lang::En, false) => format!("the price broke its {} low", en_adj(&k, u)),
            }
        }
        "extreme.rsi" => {
            let lvl =
                l.ev.get("level")
                    .copied()
                    .unwrap_or(if up { 70.0 } else { 30.0 });
            let ls = if red {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let w = lvl.round().max(0.0) as u32;
                match lang {
                    Lang::Tr => words_tr(w),
                    Lang::En => words_en(w),
                }
            } else {
                n(lvl, Cat::Osc, lang)
            };
            let val = ev("value").map(|v| n(v, Cat::Osc, lang));
            match (lang, exit, val) {
                (Lang::Tr, false, Some(v)) => format!("RSI {v} oldu"),
                (Lang::Tr, false, None) => format!(
                    "RSI {} {}",
                    tr_gen_w(&ls, red),
                    if up {
                        "üstüne çıktı"
                    } else {
                        "altına indi"
                    }
                ),
                (Lang::Tr, true, Some(v)) => format!(
                    "RSI {v} ile {} {}",
                    Word::number(&ls).gen(),
                    if up {
                        "altına indi"
                    } else {
                        "üstüne çıktı"
                    }
                ),
                (Lang::Tr, true, None) => format!(
                    "RSI {} {}",
                    tr_gen_w(&ls, red),
                    if up {
                        "altına indi"
                    } else {
                        "üstüne çıktı"
                    }
                ),
                (Lang::En, false, Some(v)) => format!("RSI reached {v}"),
                (Lang::En, false, None) => {
                    format!("RSI went {} {ls}", if up { "above" } else { "below" })
                }
                (Lang::En, true, Some(v)) => format!(
                    "RSI moved back {} {ls} at {v}",
                    if up { "below" } else { "above" }
                ),
                (Lang::En, true, None) => {
                    format!("RSI moved back {} {ls}", if up { "below" } else { "above" })
                }
            }
        }
        "reversal.rsi_divergence" => match (lang, up) {
            (Lang::Tr, true) => "RSI fiyatın yeni dibine eşlik etmedi".into(),
            (Lang::Tr, false) => "RSI fiyatın yeni tepesine eşlik etmedi".into(),
            (Lang::En, true) => "RSI did not match the price's new low".into(),
            (Lang::En, false) => "RSI did not match the price's new high".into(),
        },
        "continuation.ma_stack" => match (lang, exit, up) {
            (Lang::Tr, false, true) => "ortalamalar yükseliş sırasına girdi".into(),
            (Lang::Tr, false, false) => "ortalamalar düşüş sırasına girdi".into(),
            (Lang::Tr, true, _) => "ortalamaların sırası bozuldu".into(),
            (Lang::En, false, true) => "the averages lined up for a rise".into(),
            (Lang::En, false, false) => "the averages lined up for a fall".into(),
            (Lang::En, true, _) => "the averages fell out of order".into(),
        },
        "squeeze.bandwidth" => {
            let a = param(BandSqueeze::PERIOD, lang, red);
            let b = param(BandSqueeze::LOOKBACK, lang, red);
            match (lang, exit) {
                (Lang::Tr, false) => {
                    format!(
                        "{} bant son {} en darına indi",
                        tr_adj(&a, u),
                        tr_gen(&b, u)
                    )
                }
                (Lang::Tr, true) => "bant yeniden genişledi".into(),
                (Lang::En, false) => format!(
                    "the {} band hit its narrowest in {b} {}",
                    en_adj(&a, u),
                    u.words().en_pl
                ),
                (Lang::En, true) => "the band widened again".into(),
            }
        }
        "flow.volume_surge" => match (lang, ev("ratio")) {
            (Lang::Tr, Some(r)) => {
                format!("hacim ortalamanın {} katına çıktı", n(r, Cat::Osc, lang))
            }
            (Lang::En, Some(r)) => {
                format!("volume reached {} times its average", n(r, Cat::Osc, lang))
            }
            (Lang::Tr, None) => format!(
                "hacim {tr_again}ortalamanın {} katını aştı",
                mult(VolumeSurge::MULT, lang, red)
            ),
            (Lang::En, None) => format!(
                "volume {en_again}topped {} times its average",
                mult(VolumeSurge::MULT, lang, red)
            ),
        },
        "flow.institutional" => {
            let r = match (ev("net"), ev("avg5")) {
                (Some(a), Some(b)) if b.abs() > 0.0 => Some(a.abs() / b.abs()),
                _ => None,
            };
            let (ts, es) = if up {
                ("alım", "buying")
            } else {
                ("satım", "selling")
            };
            match (lang, r) {
                (Lang::Tr, Some(r)) => format!(
                    "kurumsal net {ts} ortalamanın {} katına çıktı",
                    n(r, Cat::Osc, lang)
                ),
                (Lang::En, Some(r)) => format!(
                    "institutional net {es} reached {} times its average",
                    n(r, Cat::Osc, lang)
                ),
                (Lang::Tr, None) => format!(
                    "kurumsal net {ts} {tr_again}ortalamanın {} katını aştı",
                    mult(InstitutionalFlow::MULT, lang, red)
                ),
                (Lang::En, None) => format!(
                    "institutional net {es} {en_again}topped {} times its average",
                    mult(InstitutionalFlow::MULT, lang, red)
                ),
            }
        }
        _ => match lang {
            Lang::Tr => "yeni bir işaret geldi".into(),
            Lang::En => "new evidence arrived".into(),
        },
    };
    (s, concepts)
}
