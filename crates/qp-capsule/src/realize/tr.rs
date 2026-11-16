//! Turkish templates.
//!
//! | Sınıf | Kalıp |
//! |---|---|
//! | Olay | `{özne} {ne zaman} {olay yüklemi, -DI}: {tür adı}.` |
//! | Süren durum | `{özne} {süre, -DIr} {eşik-İN} üstünde/altında; şu an {değer}. {tür adı} sürüyor.` |
//! | Biten durum | `{özne} {ne zaman} {eşik-İN} üstüne çıktı/altına indi[, {ne zaman} yeniden …]; [şu an {değer},] artık {eşik-İN} üstünde değil. {tür adı} sona erdi.` |
//! | Beklenti | `{özne} [{n} bar içinde] {eşik-İN} üstüne çıkarsa/altına inerse/üstünde kapanırsa {tür adı} teyit olur.` |
//!
//! Ardından: aşama notu, geçerlilik kuyruğu (olay/beklenti), karne, sunucu işareti.

use crate::Lang;
use qp_schema::capsule::{Direction, EvidenceFamily, MeaningKind};

use crate::realize::morph::{capitalize, past, Word};
use crate::realize::num::{fmt, int, Cat};
use crate::realize::{
    key_cat, Ago, Class, LedgerView, Plan, Remark, Subject, Tendency, Vague, Validity,
};

const L: Lang = Lang::Tr;

/// The kind's noun as the subject of "sürüyor / sona erdi / teyit olur".
#[must_use]
pub fn noun_state(kind: MeaningKind, dir: Direction) -> &'static str {
    use Direction::{Down, Neutral, Up};
    use MeaningKind::{Breakout, Continuation, Extreme, Flow, Reversal, Squeeze};
    match (kind, dir) {
        (Continuation, Up) => "yükseliş eğilimi",
        (Continuation, Down) => "düşüş eğilimi",
        (Continuation, Neutral) => "eğilim",
        (Reversal, Up) => "yukarı dönüş",
        (Reversal, Down) => "aşağı dönüş",
        (Reversal, Neutral) => "dönüş işareti",
        (Breakout, Up) => "yukarı kırılım",
        (Breakout, Down) => "aşağı kırılım",
        (Breakout, Neutral) => "kırılım",
        (Squeeze, _) => "sıkışma (yön belirsiz)",
        (Extreme, Up) => "aşırı alım",
        (Extreme, Down) => "aşırı satım",
        (Extreme, Neutral) => "aşırılık",
        (Flow, Up) => "para girişi",
        (Flow, Down) => "para çıkışı",
        (Flow, Neutral) => "olağandışı para akışı",
    }
}

/// The kind's noun after an event clause.
fn noun_event(kind: MeaningKind, dir: Direction) -> &'static str {
    match (kind, dir) {
        (MeaningKind::Continuation, Direction::Up) => "yükseliş eğiliminde devam sinyali",
        (MeaningKind::Continuation, Direction::Down) => "düşüş eğiliminde devam sinyali",
        (MeaningKind::Squeeze, _) => "sıkışma, yön belirsiz",
        _ => noun_state(kind, dir),
    }
}

/// "Ne zaman" (a point in time): `bugün`, `dün`, `3 bar önce`; redacted without digits.
#[must_use]
pub fn when(a: Ago) -> String {
    match a.n {
        0 if a.daily => "bugün".into(),
        0 => "bu barda".into(),
        1 if a.yesterday => "dün".into(),
        1 if a.redacted => "bir bar önce".into(),
        n if !a.redacted => format!("{} bar önce", int(n)),
        _ => match a.vague() {
            Vague::Few if a.daily => "birkaç gün önce",
            Vague::Few => "birkaç bar önce",
            Vague::Week => "yaklaşık bir hafta önce",
            Vague::Weeks => "birkaç hafta önce",
            Vague::Month => "yaklaşık bir ay önce",
            Vague::MoreThanMonth => "bir aydan uzun süre önce",
            Vague::While => "bir süre önce",
            _ => "uzun süre önce",
        }
        .into(),
    }
}

/// "Ne zamandır" (a duration up to now): `bugün itibarıyla`, `dünden beri`, `5 bardır`.
fn since(a: Ago) -> String {
    match a.n {
        0 if a.daily => "bugün itibarıyla".into(),
        0 => "bu bar itibarıyla".into(),
        1 if a.yesterday => "dünden beri".into(),
        1 if a.redacted => Word::noun("bir bar").copula(),
        n if !a.redacted => Word::noun(&format!("{} bar", int(n))).copula(),
        _ => Word::noun(match a.vague() {
            Vague::Few if a.daily => "birkaç gün",
            Vague::Few => "birkaç bar",
            Vague::Week => "yaklaşık bir hafta",
            Vague::Weeks => "birkaç hafta",
            Vague::Month => "yaklaşık bir ay",
            Vague::MoreThanMonth => "bir aydan uzun süre",
            Vague::While => "bir süre",
            _ => "uzun süre",
        })
        .copula(),
    }
}

/// Threshold name (used when the value is hidden, or alongside it), and whether the value
/// alone reads naturally (`70'in üstünde`).
fn key_name(p: &Plan<'_>, key: &str, subject: &Subject) -> (Word, bool) {
    match key {
        "level" if *subject == Subject::Rsi => (
            Word::possessed(if p.dir == Direction::Down {
                "aşırı satım eşiği"
            } else {
                "aşırı alım eşiği"
            }),
            true,
        ),
        "level" => (Word::possessed("kırılım seviyesi"), true),
        "slow" | "sma50" => (Word::noun("uzun ortalama"), false),
        "fast" | "sma20" => (Word::noun("kısa ortalama"), false),
        "price" | "p1_price" | "p2_price" => (Word::possessed("dönüş noktası"), false),
        "threshold" => (Word::noun("eşik"), false),
        "avg5" => (Word::possessed("son günlerin ortalaması"), false),
        "half_width" => (Word::possessed("bant yarı genişliği"), false),
        "vol_avg" | "avg20" => (Word::possessed("hacim ortalaması"), false),
        "middle" => (Word::noun("orta bant"), false),
        k => (Word::noun(&k.replace('_', " ")), true),
    }
}

/// Threshold in the genitive: `70'in`, `uzun ortalamanın (142,10)`, `eşiğin`.
fn thr_gen(p: &Plan<'_>, with_value: bool) -> String {
    let Some(c) = p.cond.as_ref() else {
        return "eşiğin".into();
    };
    let (name, bare) = key_name(p, &c.key, &c.subject);
    match p.thr() {
        Some(v) if bare => Word::number(&fmt(v, key_cat(&c.key, &c.subject), L)).gen(),
        Some(v) if with_value => format!(
            "{} ({})",
            name.gen(),
            fmt(v, key_cat(&c.key, &c.subject), L)
        ),
        _ => name.gen(),
    }
}

/// Subject phrase: `THYAO` (close), `THYAO'da RSI`, `THYAO'da bant genişliği`.
fn subject(p: &Plan<'_>) -> String {
    let i = Word::proper(p.inst);
    match p.cond.as_ref().map(|c| &c.subject) {
        Some(Subject::Close) => p.inst.to_owned(),
        Some(Subject::Rsi) => format!("{} RSI", i.loc()),
        Some(Subject::Bandwidth) => format!("{} bant genişliği", i.loc()),
        // S-A fix (M2b): the side is a word, never "net, ortalamanın altında".
        Some(Subject::Net) => format!("{} kurumsal net {}", i.loc(), net_side(p)),
        Some(Subject::CloseDev) => format!("{} kapanışın orta banttan uzaklığı", i.loc()),
        Some(Subject::Other(s)) => format!("{} {}", i.loc(), s.replace('_', " ")),
        None => i.loc(),
    }
}

/// `alış` / `satış` of a net-flow condition (above = buying).
fn net_side(p: &Plan<'_>) -> &'static str {
    if p.cond.as_ref().is_none_or(|c| c.above) {
        "alış"
    } else {
        "satış"
    }
}

/// Net flow (S-A fix): "her zamankinden güçlü" with the recent average as the yardstick:
/// ` (son günlerin ortalaması 452.000)` when numbers may be shown, else nothing.
fn usual_paren(p: &Plan<'_>) -> String {
    p.thr()
        .map(|v| format!(" (son günlerin ortalaması {})", fmt(v, Cat::Count, L)))
        .unwrap_or_default()
}

fn num_opt(v: Option<f64>, cat: Cat) -> Option<String> {
    v.map(|x| fmt(x, cat, L))
}

fn paren(parts: &[(&str, Option<String>)]) -> String {
    let xs: Vec<String> = parts
        .iter()
        .filter_map(|(name, v)| v.as_ref().map(|v| format!("{name} {v}")))
        .collect();
    if xs.is_empty() {
        String::new()
    } else {
        format!(" ({})", xs.join(", "))
    }
}

/// Event clause (past tense with when), without the final period.
fn event_clause(p: &Plan<'_>) -> String {
    let i = Word::proper(p.inst);
    let w = when(p.lead_ago);
    let up = p.dir != Direction::Down;
    let noun = noun_event(p.kind, p.dir);
    match p.kind {
        MeaningKind::Breakout => {
            let at = num_opt(p.val(&["level"]), Cat::Price)
                .map(|l| format!("{} ", Word::number(&l).loc_ki()))
                .unwrap_or_default();
            let (obj, verb) = if up {
                (Word::noun("direnç").acc(), "yukarı kırdı")
            } else {
                (Word::noun("destek").acc(), "aşağı kırdı")
            };
            let cl = paren(&[("kapanış", num_opt(p.val(&["close"]), Cat::Price))]);
            format!("{} {w} {at}{obj} {verb}{cl}: {noun}", p.inst)
        }
        MeaningKind::Extreme => {
            let lvl = num_opt(p.val(&["level"]), Cat::Osc).map_or_else(
                || {
                    Word::possessed(if up {
                        "aşırı alım eşiği"
                    } else {
                        "aşırı satım eşiği"
                    })
                    .gen()
                },
                |l| Word::number(&l).gen(),
            );
            let v = num_opt(p.val(&["value", "rsi"]), Cat::Osc)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            let mv = if up {
                format!("üstüne {}", past("çık"))
            } else {
                format!("altına {}", past("in"))
            };
            format!("{} RSI {w} {lvl} {mv}{v}: {noun}", i.loc())
        }
        MeaningKind::Continuation => {
            let side = if up { "yükseliş" } else { "düşüş" };
            let vals = paren(&[
                ("kapanış", num_opt(p.val(&["close"]), Cat::Price)),
                (
                    "uzun ortalama",
                    num_opt(p.val(&["slow", "sma50"]), Cat::Price),
                ),
            ]);
            format!(
                "{} {w} hareketli ortalamalar {side} yönünde {}{vals}: {noun}",
                i.loc(),
                past("dizil")
            )
        }
        MeaningKind::Reversal => {
            let (a, b) = if up {
                ("yeni dip", "daha yüksek dip")
            } else {
                ("yeni tepe", "daha düşük tepe")
            };
            let vals = paren(&[
                (
                    "fiyat",
                    num_opt(p.val(&["p2_price", "price", "close"]), Cat::Price),
                ),
                ("RSI", num_opt(p.val(&["p2_rsi", "rsi"]), Cat::Osc)),
            ]);
            format!(
                "{} {w} {noun} işareti {}: fiyat {a} yaparken RSI {b} {}{vals}",
                i.loc(),
                past("çık"),
                past("yap")
            )
        }
        MeaningKind::Squeeze => {
            let vals = paren(&[
                (
                    "bant genişliği",
                    num_opt(p.val(&["bandwidth", "bw"]), Cat::Ratio),
                ),
                ("eşik", num_opt(p.val(&["threshold"]), Cat::Ratio)),
            ]);
            format!(
                "{} {w} Bollinger bantları {}{vals}: {noun}",
                i.loc(),
                past("daral")
            )
        }
        MeaningKind::Flow if p.has("net") => {
            let side = if up { "alış" } else { "satış" };
            let n = num_opt(p.val(&["net"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            let a = num_opt(p.val(&["avg5"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            format!(
                "{} {w} kurumsal net {side}{n} {}{a} belirgin biçimde {}: {noun}",
                i.loc(),
                Word::possessed("son günlerin ortalaması").acc(),
                past("aş")
            )
        }
        MeaningKind::Flow if p.has("volume") => {
            let v = num_opt(p.val(&["volume"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            let a = num_opt(p.val(&["vol_avg", "avg20"]), Cat::Count)
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            format!(
                "{} {w} hacim{v} {}{a} belirgin üstüne {}: {noun}",
                i.loc(),
                Word::possessed("ortalaması").gen(),
                past("çık")
            )
        }
        MeaningKind::Flow => format!("{} {w} {noun} {}", i.loc(), past("görül")),
    }
}

fn holding_clause(p: &Plan<'_>) -> String {
    let noun = capitalize(noun_state(p.kind, p.dir));
    let d = since(p.lead_ago);
    let Some(c) = p.cond.as_ref() else {
        return format!(
            "{} {d} {} sürüyor",
            Word::proper(p.inst).loc(),
            noun_state(p.kind, p.dir)
        );
    };
    let cur = p
        .current()
        .map(|v| format!("; şu an {}", fmt(v, c.subject.cat(), L)))
        .unwrap_or_default();
    if c.subject == Subject::Net {
        return format!(
            "{} {d} her zamankinden güçlü{}{cur}. {noun} sürüyor",
            subject(p),
            usual_paren(p)
        );
    }
    let side = if c.above { "üstünde" } else { "altında" };
    format!(
        "{} {d} {} {side}{cur}. {noun} sürüyor",
        subject(p),
        thr_gen(p, true)
    )
}

fn ended_clause(p: &Plan<'_>) -> String {
    let w = when(p.lead_ago);
    let noun = capitalize(noun_state(p.kind, p.dir));
    let Some(c) = p.cond.as_ref() else {
        return format!(
            "{} {w} {} {}; artık sürmüyor. {noun} sona erdi",
            Word::proper(p.inst).loc(),
            noun_state(p.kind, p.dir),
            past("başla")
        );
    };
    if c.subject == Subject::Net {
        let ex = p
            .exit_ago
            .map(|a| format!(", {} yeniden olağan düzeyine {}", when(a), past("gerile")))
            .unwrap_or_default();
        let cur = p
            .current()
            .map(|v| format!("şu an {}, ", fmt(v, Cat::Count, L)))
            .unwrap_or_default();
        return format!(
            "{} {w} her zamankinden güçlü hâle {}{}{ex}; {cur}artık her zamankinden güçlü değil. {noun} sona erdi",
            subject(p),
            past("gel"),
            usual_paren(p)
        );
    }
    let (enter, exit, side) = if c.above {
        (
            format!("üstüne {}", past("çık")),
            format!("altına {}", past("in")),
            "üstünde",
        )
    } else {
        (
            format!("altına {}", past("in")),
            format!("üstüne {}", past("çık")),
            "altında",
        )
    };
    let ex = p
        .exit_ago
        .map(|a| format!(", {} yeniden {exit}", when(a)))
        .unwrap_or_default();
    let cur = p
        .current()
        .map(|v| format!("şu an {}, ", fmt(v, c.subject.cat(), L)))
        .unwrap_or_default();
    format!(
        "{} {w} {} {enter}{ex}; {cur}artık {} {side} değil. {noun} sona erdi",
        subject(p),
        thr_gen(p, true),
        thr_gen(p, false)
    )
}

fn expectation_clause(p: &Plan<'_>) -> String {
    let noun = noun_state(p.kind, p.dir);
    let Some(c) = p.cond.as_ref() else {
        return format!(
            "{} {noun} beklentisi var; koşulu {} teyit olur",
            Word::proper(p.inst).loc(),
            crate::realize::morph::conditional("gerçekleş")
        );
    };
    let within = match p.within() {
        Some(n) => format!("{} bar içinde ", int(u64::from(n))),
        None if p.bounded() => "birkaç bar içinde ".into(),
        None => String::new(),
    };
    if c.subject == Subject::Net {
        return format!(
            "{} {within}her zamankinden güçlü hâle {}{} {noun} teyit olur",
            subject(p),
            crate::realize::morph::conditional("gel"),
            usual_paren(p)
        );
    }
    let verb = match (&c.subject, c.above) {
        (Subject::Close, true) => {
            format!("üstünde {}", crate::realize::morph::conditional("kapan"))
        }
        (Subject::Close, false) => {
            format!("altında {}", crate::realize::morph::conditional("kapan"))
        }
        (_, true) => format!("üstüne {}", crate::realize::morph::conditional("çık")),
        (_, false) => format!("altına {}", crate::realize::morph::conditional("in")),
    };
    format!(
        "{} {within}{} {verb} {noun} teyit olur",
        subject(p),
        thr_gen(p, true)
    )
}

fn family_name(f: EvidenceFamily) -> &'static str {
    match f {
        EvidenceFamily::Price => "fiyat",
        EvidenceFamily::Volume => "hacim",
        EvidenceFamily::Flow => "para akışı",
        EvidenceFamily::Derivative => "türev piyasası",
        EvidenceFamily::News => "haber akışı",
    }
}

/// "de/da" clitic after a word (harmony with its last vowel).
fn da(w: &str) -> &'static str {
    let lv = w
        .chars()
        .rev()
        .find(|c| "aıoueiöü".contains(*c))
        .unwrap_or('e');
    if "aıou".contains(lv) {
        "da"
    } else {
        "de"
    }
}

fn remark(p: &Plan<'_>) -> Option<String> {
    let s = when(p.stage_ago);
    let no_longer = if p.class == Class::Ended {
        ""
    } else {
        "; artık geçerli değil"
    };
    // Who the remark is about: the expectation, or the signal.
    let who = Word::noun(if p.class == Class::Expectation {
        "beklenti"
    } else {
        "sinyal"
    });
    Some(match p.remark {
        Remark::None => return None,
        Remark::Candidate => "Henüz aday: bar kapanmadı".into(),
        Remark::Strengthened => format!(
            "{} aynı aileden yeni bir kanıtla {}",
            capitalize(&s),
            past("güçlen")
        ),
        Remark::Confirmed => {
            let fam = p.latest_family.map_or("yeni bir kanıt ailesi", family_name);
            let n = p
                .families()
                .map(|n| format!("; {} bağımsız kanıt ailesi destekliyor", int(n as u64)))
                .unwrap_or_default();
            format!(
                "{} {fam} {} teyit {}{n}",
                capitalize(&s),
                da(fam),
                past("et")
            )
        }
        Remark::Weakened => format!(
            "{} ters yönde bir sinyal {}; görünüm {}",
            capitalize(&s),
            past("gel"),
            past("zayıfla")
        ),
        // S-A fix (M5): with the barrier rule a realized extreme means the stretch went on.
        Remark::Realized if p.kind == MeaningKind::Extreme => format!(
            "Uzama {}: fiyat {s} {} devam {}; sinyal gerçekleşti sayıldı",
            past("sür"),
            match p.dir {
                Direction::Up => "yükselmeye",
                Direction::Down => "düşmeye",
                Direction::Neutral => "aynı yönde ilerlemeye",
            },
            past("et")
        ),
        Remark::Invalidated if p.kind == MeaningKind::Extreme => format!(
            "Uzama sürmedi: fiyat {s} geri {}; sinyal gerçekleşmedi sayıldı{no_longer}",
            match p.dir {
                Direction::Up => past("düş"),
                Direction::Down => past("yüksel"),
                Direction::Neutral => past("dön"),
            }
        ),
        Remark::Realized if p.class == Class::Expectation => {
            format!("Beklenti {s} {}", past("gerçekleş"))
        }
        Remark::Realized => format!("Beklenen sonuç {s} {}", past("gerçekleş")),
        Remark::Invalidated => format!(
            "{} {s} geçersiz {}{no_longer}",
            capitalize(&who.written),
            past("ol")
        ),
        Remark::Expired => {
            let h = p
                .horizon()
                .map(|h| format!("{} barlık ", int(u64::from(h))))
                .unwrap_or_default();
            format!(
                "{} {h}ömrü {s} sonuçsuz {}{no_longer}",
                capitalize(&who.gen()),
                past("dol")
            )
        }
        Remark::Retracted => format!(
            "Dayandığı veri düzeltildiği için {s} geri {}; karneye yazılmadı{no_longer}",
            past("çekil")
        ),
    })
}

/// The validity tail of an open story. A closed story has none: its stage remark is the
/// validity clause and says how it ended (FIX-E1 B2, K42: never "still holds" next to a
/// closed glyph).
fn tail(p: &Plan<'_>) -> Option<&'static str> {
    if p.closed() || matches!(p.class, Class::Holding | Class::Ended) {
        return None;
    }
    Some(match (p.class, p.validity) {
        (Class::Expectation, Validity::Yes) => "Koşul hâlâ açık",
        (Class::Expectation, Validity::Unknown) => "Koşulun bugünkü durumu bilinmiyor",
        (_, Validity::Yes) => "Hâlâ geçerli",
        (_, Validity::No) => "Artık geçerli değil",
        (_, Validity::Unknown) => "Hâlâ geçerli olup olmadığı bilinmiyor",
    })
}

fn ledger(p: &Plan<'_>) -> String {
    match p.ledger() {
        LedgerView::Empty => "Henüz karnesi yok".into(),
        LedgerView::Counts(n, r) => format!(
            "Geçmişte bu sinyal {} seferin {} beklendiği gibi sonuçlandı",
            int(u64::from(n)),
            Word::number(&int(u64::from(r))).poss3_loc()
        ),
        LedgerView::Thin(n, r) => format!(
            "Karnesi henüz az örnekli: {} seferin {} tuttu",
            int(u64::from(n)),
            Word::number(&int(u64::from(r))).poss3_loc()
        ),
        LedgerView::ThinQual => "Karnesi henüz az örnekli".into(),
        LedgerView::Qual(Tendency::Mostly) => {
            "Geçmişte bu tür sinyal çoğunlukla beklendiği gibi sonuçlandı".into()
        }
        LedgerView::Qual(Tendency::Even) => {
            "Geçmişte bu tür sinyal kabaca yarı yarıya tuttu".into()
        }
        LedgerView::Qual(Tendency::Rarely) => "Geçmişte bu tür sinyal çoğunlukla tutmadı".into(),
    }
}

fn server(p: &Plan<'_>) -> Option<String> {
    if !p.server {
        return None;
    }
    if !p.redacted {
        return Some("Sunucu doğruladı".into());
    }
    let lic = match p.entitlement {
        Some(e) if e.starts_with("flow") => "kurumsal akış lisansı",
        _ => "veri lisansı",
    };
    Some(format!("Sunucu doğruladı; rakamlar {lic} gerektiriyor"))
}

/// Renders a plan in Turkish.
#[must_use]
pub fn render(p: &Plan<'_>) -> String {
    let main = match p.class {
        Class::Event => event_clause(p),
        Class::Holding => holding_clause(p),
        Class::Ended => ended_clause(p),
        Class::Expectation => expectation_clause(p),
    };
    let mut parts = vec![main];
    parts.extend(remark(p));
    parts.extend(tail(p).map(str::to_owned));
    parts.push(ledger(p));
    parts.extend(server(p));
    let mut out = parts.join(". ");
    out.push('.');
    out
}
