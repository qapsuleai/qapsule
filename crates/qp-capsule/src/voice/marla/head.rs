//! Marla's headline: "Ne değişti?" + when + validity.

use super::*;

// ---------------------------------------------------------------------------
// Headline: "Ne değişti?" + when + validity
// ---------------------------------------------------------------------------

pub(super) const HEAD_LEVELS: usize = 6;

/// Levels, richest first: (core tier, long validity, with when). The long validity
/// ("hâlâ geçerli") is kept before the short one ("sürüyor"), which can read as "the
/// buying goes on"; dropping "when" is the last resort (never needed on the harness
/// situations).
pub(super) const HEAD_PLAN: [(u8, bool, bool); HEAD_LEVELS] = [
    (0, true, true),
    (1, true, true),
    (2, true, true),
    (3, true, true),
    (3, false, true),
    (3, false, false),
];

pub(super) fn headline(f: &Frame<'_>, lang: Lang, level: usize) -> String {
    let (tier, long_v, with_when) = HEAD_PLAN[level.min(HEAD_LEVELS - 1)];
    let core = core(f, lang, tier);
    if f.validity == Validity::Closed {
        return format!("{core} · {}", closed_chip(f, lang, with_when));
    }
    let v = validity_chip(f, lang, long_v);
    if with_when {
        format!("{core} · {}, {v}", f.when(lang))
    } else {
        format!("{core} · {v}")
    }
}

/// How and when a closed claim ended: TR "bugün gerçekleşti", EN "played out today"
/// (the last move is the closing transition).
fn closed_chip(f: &Frame<'_>, lang: Lang, with_when: bool) -> String {
    let e = validity_chip(f, lang, false);
    if !with_when {
        return e;
    }
    let mv = f.moved(lang);
    match lang {
        Lang::Tr => format!("{mv} {e}"),
        Lang::En => format!("{e} {mv}"),
    }
}

fn validity_chip(f: &Frame<'_>, lang: Lang, long: bool) -> String {
    let s = match (f.validity, f.ending, lang, long) {
        (Validity::Valid, _, Lang::Tr, true) => "hâlâ geçerli",
        (Validity::Valid, _, Lang::Tr, false) => "sürüyor",
        (Validity::Valid, _, Lang::En, _) => "still holds",
        (Validity::NoLonger, _, Lang::Tr, true) => "artık geçerli değil",
        (Validity::NoLonger, _, Lang::Tr, false) => "artık sürmüyor",
        (Validity::NoLonger, _, Lang::En, _) => "no longer holds",
        (Validity::Closed, Some(Ending::Realized), Lang::Tr, true) => "kapandı: gerçekleşti",
        (Validity::Closed, Some(Ending::Realized), Lang::Tr, false) => "gerçekleşti",
        (Validity::Closed, Some(Ending::Realized), Lang::En, true) => "closed: played out",
        (Validity::Closed, Some(Ending::Realized), Lang::En, false) => "played out",
        (Validity::Closed, Some(Ending::Failed), Lang::Tr, true) => "kapandı: tutmadı",
        (Validity::Closed, Some(Ending::Failed), Lang::Tr, false) => "tutmadı",
        (Validity::Closed, Some(Ending::Failed), Lang::En, true) => "closed: did not play out",
        (Validity::Closed, Some(Ending::Failed), Lang::En, false) => "did not play out",
        (Validity::Closed, Some(Ending::Expired), Lang::Tr, true) => "kapandı: süresi doldu",
        (Validity::Closed, Some(Ending::Expired), Lang::Tr, false) => "süresi doldu",
        (Validity::Closed, Some(Ending::Expired), Lang::En, true) => "closed: ran out of time",
        (Validity::Closed, Some(Ending::Expired), Lang::En, false) => "ran out of time",
        (Validity::Closed, Some(Ending::Retracted), Lang::Tr, _) => "geri çekildi",
        (Validity::Closed, Some(Ending::Retracted), Lang::En, _) => "withdrawn",
        (Validity::Closed, None, Lang::Tr, _) => "kapandı",
        (Validity::Closed, None, Lang::En, _) => "closed",
    };
    s.to_owned()
}

/// The plain change with the threshold by name (and value for an entitled reader).
/// `tier` 0 = full, 1 = short, 2 = tiny, 3 = tiniest (where it differs from tiny); the
/// numbers of tiers 1–3 are the same in both languages.
#[allow(clippy::too_many_lines)]
fn core(f: &Frame<'_>, lang: Lang, tier: u8) -> String {
    let i = f.inst;
    let u = f.unit;
    let red = f.red;
    let up = f.up();
    match f.det {
        "breakout.range" => {
            let k = param(RangeBreakout::WINDOW, lang, red);
            let g = tr_gen(&k, u);
            let a = en_adj(&k, u);
            let lvl = f
                .v("level")
                .map(|x| format!(" ({})", n(x, Cat::Price, lang)))
                .unwrap_or_default();
            match (lang, up, tier) {
                (Lang::Tr, true, 0) => format!("{i} son {g} en yüksek fiyatını{lvl} aştı"),
                (Lang::Tr, true, _) => format!("{i} {g} zirvesini{lvl} aştı"),
                (Lang::Tr, false, 0) => format!("{i} son {g} en düşük fiyatının{lvl} altına indi"),
                (Lang::Tr, false, 1) => format!("{i} {g} en düşüğünün{lvl} altına indi"),
                (Lang::Tr, false, _) => format!("{i} {g} en düşüğünü{lvl} kırdı"),
                (Lang::En, true, 0) => format!("{i} rose above its {a} high{lvl}"),
                (Lang::En, true, _) => format!("{i} broke its {a} high{lvl}"),
                (Lang::En, false, 0) => format!("{i} fell below its {a} low{lvl}"),
                (Lang::En, false, _) => format!("{i} broke its {a} low{lvl}"),
            }
        }
        "extreme.rsi" => {
            let lvl =
                f.ev.get("level")
                    .copied()
                    .unwrap_or(if up { 70.0 } else { 30.0 });
            let l = if red {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let w = lvl.round().max(0.0) as u32;
                match lang {
                    Lang::Tr => words_tr(w),
                    Lang::En => words_en(w),
                }
            } else {
                n(lvl, Cat::Osc, lang)
            };
            match (lang, up, tier) {
                (Lang::Tr, true, 0) => {
                    format!("{i} kısa sürede çok yükseldi: RSI {} aştı", tr_acc(&l, red))
                }
                (Lang::Tr, true, _) => {
                    format!("{i} hızla yükseldi: RSI {} aştı", tr_acc(&l, red))
                }
                (Lang::Tr, false, 0) => format!(
                    "{i} kısa sürede çok düştü: RSI {} altına indi",
                    tr_gen_w(&l, red)
                ),
                (Lang::Tr, false, _) => {
                    format!("{i} hızla düştü: RSI {} altına indi", tr_gen_w(&l, red))
                }
                (Lang::En, true, 0) => {
                    format!("{i} rose a lot in a short time: RSI went above {l}")
                }
                (Lang::En, true, _) => format!("{i} rose fast: RSI went above {l}"),
                (Lang::En, false, 0) => {
                    format!("{i} fell a lot in a short time: RSI went below {l}")
                }
                (Lang::En, false, _) => format!("{i} fell fast: RSI went below {l}"),
            }
        }
        "reversal.rsi_divergence" => match (lang, up, tier) {
            (Lang::Tr, true, 0) => {
                format!("{i}: düşüş yukarı dönebilir, fiyat yeni dip yaptı ama RSI yapmadı")
            }
            (Lang::Tr, true, 1) => {
                format!("{i} yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı")
            }
            (Lang::Tr, true, _) => format!("{i} yukarı dönebilir: RSI yeni dip yapmadı"),
            (Lang::Tr, false, 0) => {
                format!("{i}: yükseliş aşağı dönebilir, fiyat yeni tepe yaptı ama RSI yapmadı")
            }
            (Lang::Tr, false, 1) => {
                format!("{i} aşağı dönebilir: fiyat yeni tepe yaptı, RSI yapmadı")
            }
            (Lang::Tr, false, _) => format!("{i} aşağı dönebilir: RSI yeni tepe yapmadı"),
            (Lang::En, true, 0) => {
                format!("{i}'s fall could turn up: a new price low that RSI did not match")
            }
            (Lang::En, true, 1) => format!("{i} could turn up: new price low, not matched by RSI"),
            (Lang::En, true, _) => format!("{i} could turn up: RSI made no new low"),
            (Lang::En, false, 0) => {
                format!("{i}'s rise could turn down: a new price high that RSI did not match")
            }
            (Lang::En, false, 1) => {
                format!("{i} could turn down: new price high, not matched by RSI")
            }
            (Lang::En, false, _) => format!("{i} could turn down: RSI made no new high"),
        },
        "continuation.ma_stack" => {
            // (R5/m17-int, S1) The event verb first and plain: the trend goes on
            // ("düşüşünü sürdürdü"), the averages after it as where the price stands —
            // never "altına indi", which readers took for a threshold crossing.
            let a = param(MaStack::FAST, lang, red);
            let b = param(MaStack::SLOW, lang, red);
            match (lang, up, tier) {
                (Lang::Tr, true, 0) => format!(
                    "{i} yükselişini sürdürdü: fiyat {a} ve {} ortalamaların üstünde",
                    tr_adj(&b, u)
                ),
                (Lang::Tr, true, 1) => {
                    format!(
                        "{i} yükselişini sürdürdü, {} ortalamanın üstünde",
                        tr_adj(&a, u)
                    )
                }
                (Lang::Tr, true, 2) => {
                    format!("{i} yükselişte, {} ortalamanın üstünde", tr_adj(&a, u))
                }
                (Lang::Tr, true, _) => {
                    format!("{i} yükselişte: {} ortalama üstü", tr_adj(&a, u))
                }
                (Lang::Tr, false, 0) => format!(
                    "{i} düşüşünü sürdürdü: fiyat {a} ve {} ortalamaların altında",
                    tr_adj(&b, u)
                ),
                (Lang::Tr, false, 1) => {
                    format!(
                        "{i} düşüşünü sürdürdü, {} ortalamanın altında",
                        tr_adj(&a, u)
                    )
                }
                (Lang::Tr, false, 2) => {
                    format!("{i} düşüşte, {} ortalamanın altında", tr_adj(&a, u))
                }
                (Lang::Tr, false, _) => {
                    format!("{i} düşüşte: {} ortalama altı", tr_adj(&a, u))
                }
                (Lang::En, true, 0) => format!(
                    "{i} kept rising: price stayed above its {a}- and {} averages",
                    en_adj(&b, u)
                ),
                (Lang::En, true, 1) => {
                    format!("{i} kept rising, above its {} average", en_adj(&a, u))
                }
                (Lang::En, true, 2) => {
                    format!("{i} kept rising above its {} average", en_adj(&a, u))
                }
                (Lang::En, true, _) => {
                    format!("{i} kept rising above {} average", en_adj(&a, u))
                }
                (Lang::En, false, 0) => format!(
                    "{i} kept falling: price stayed below its {a}- and {} averages",
                    en_adj(&b, u)
                ),
                (Lang::En, false, 1) => {
                    format!("{i} kept falling, below its {} average", en_adj(&a, u))
                }
                (Lang::En, false, 2) => {
                    format!("{i} kept falling below its {} average", en_adj(&a, u))
                }
                (Lang::En, false, _) => {
                    format!("{i} kept falling below {} average", en_adj(&a, u))
                }
            }
        }
        "squeeze.bandwidth" => {
            let a = param(BandSqueeze::PERIOD, lang, red);
            let b = param(BandSqueeze::LOOKBACK, lang, red);
            let w = u.words();
            match (lang, tier) {
                (Lang::Tr, 0) => format!(
                    "{i} dar bir aralığa sıkıştı: {} bant son {} en darı",
                    tr_adj(&a, u),
                    tr_gen(&b, u)
                ),
                (Lang::Tr, 1) => {
                    format!(
                        "{i} sıkıştı: {} bant {} en darı",
                        tr_adj(&a, u),
                        tr_gen(&b, u)
                    )
                }
                (Lang::Tr, _) => {
                    format!("{i}: {} bant {} en darı", tr_adj(&a, u), tr_gen(&b, u))
                }
                (Lang::En, 0) => format!(
                    "{i} narrowed to a tight range: {} band narrowest in {b} {}",
                    en_adj(&a, u),
                    w.en_pl
                ),
                (Lang::En, 1) => format!(
                    "{i} went quiet: {} band narrowest in {b} {}",
                    en_adj(&a, u),
                    w.en_pl
                ),
                (Lang::En, _) => format!("{i}: {} band at a {} low", en_adj(&a, u), en_adj(&b, u)),
            }
        }
        "flow.volume_surge" => {
            let a = param(VolumeSurge::WINDOW, lang, red);
            let m = mult(VolumeSurge::MULT, lang, red);
            let x = times_short(&m, red);
            let (tr_px, en_px, en_tiny) = match f.dir {
                Direction::Up => ("fiyat yükseldi", "as the price rose", "price rose"),
                Direction::Down => ("fiyat düştü", "as the price fell", "price fell"),
                Direction::Neutral => ("fiyat yerinde kaldı", "as the price held", "price held"),
            };
            match (lang, tier) {
                (Lang::Tr, 0) => format!(
                    "{i}: hacim {} ortalamanın {m} katını aştı, {tr_px}",
                    tr_adj(&a, u)
                ),
                (Lang::Tr, 1) => format!("{i}: hacim ortalamanın {m} katını aştı, {tr_px}"),
                (Lang::Tr, _) => format!("{i}: hacim ortalamayı {m} kat aştı, {tr_px}"),
                (Lang::En, 0) => format!(
                    "{i}: volume topped {m} times its {} average {en_px}",
                    en_adj(&a, u)
                ),
                (Lang::En, 1) => format!("{i}: volume topped {m} times its average {en_px}"),
                (Lang::En, _) => format!("{i}: volume over {x} average, {en_tiny}"),
            }
        }
        "flow.institutional" => {
            let a = param(InstitutionalFlow::WINDOW, lang, red);
            let m = mult(InstitutionalFlow::MULT, lang, red);
            let x = times_short(&m, red);
            let (tr_s, en_s) = if up {
                ("alım", "buying")
            } else {
                ("satım", "selling")
            };
            match (lang, tier) {
                (Lang::Tr, 0) => format!(
                    "{i}: kurumsal net {tr_s} {} ortalamanın {m} katını aştı",
                    tr_adj(&a, u)
                ),
                (Lang::Tr, 1) => format!("{i}: kurumsal net {tr_s} ortalamanın {m} katını aştı"),
                (Lang::Tr, 3) => format!(
                    "{i}: kurumlar olağanın {m} katı {}",
                    if up { "aldı" } else { "sattı" }
                ),
                (Lang::Tr, _) => format!("{i}: kurumsal {tr_s} ortalamayı {m} kat aştı"),
                (Lang::En, 0) => format!(
                    "{i}: institutional net {en_s} topped {m} times its {} average",
                    en_adj(&a, u)
                ),
                (Lang::En, 1) => {
                    format!("{i}: institutional net {en_s} topped {m} times its average")
                }
                (Lang::En, _) if red => format!(
                    "{i}: institutions {} {x}",
                    if up { "bought" } else { "sold" }
                ),
                (Lang::En, _) => format!(
                    "{i}: institutions {} over {x} average",
                    if up { "bought" } else { "sold" }
                ),
            }
        }
        _ => match lang {
            Lang::Tr => format!("{i}: {}", crate::realize::tr::noun_state(f.kind, f.dir)),
            Lang::En => format!("{i}: {}", crate::realize::en::noun_bare(f.kind, f.dir)),
        },
    }
}

/// EN compact multiple: "2.5x"; for a redacted reader words ("twice the usual",
/// "two and a half times its").
fn times_short(m: &str, red: bool) -> String {
    if red && m == "two" {
        "twice the usual".to_owned()
    } else if red {
        format!("{m} times its")
    } else {
        format!("{m}x")
    }
}

/// TR accusative of a number or number word ("70'i", "yetmişi").
fn tr_acc(w: &str, red: bool) -> String {
    if red {
        Word::noun(w).acc()
    } else {
        Word::number(w).acc()
    }
}

/// TR genitive of a number or number word ("30'un", "otuzun").
pub(super) fn tr_gen_w(w: &str, red: bool) -> String {
    if red {
        Word::noun(w).gen()
    } else {
        Word::number(w).gen()
    }
}
