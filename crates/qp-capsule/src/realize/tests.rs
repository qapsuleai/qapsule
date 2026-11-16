//! Tests: K26 in both languages, the no-digit family over every redacted combination,
//! determinism, time markers per class, and "every number is an engine slot".

use std::collections::BTreeMap;

use crate::{Lang, Realizer as _, SentenceInput};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, LedgerStat, License, LicenseClass,
    MeaningCondition, MeaningKind, Stage, Story, TimeKind, Transition, Trust,
};
use qp_schema::{Author, CompareOp, Fact, Provenance};

use super::{render, Plan, Template};
use crate::realize::num::round_to;

const T0: i64 = 1_735_689_600_000;
const DAY: i64 = 86_400_000;

/// A test case (owned values for a `SentenceInput`).
#[derive(Debug, Clone)]
struct Case {
    story: Story,
    lead: Fact,
    latest: Fact,
    now_bar: u64,
    still_holds: Option<bool>,
    current: Option<f64>,
    redacted: bool,
}

impl Case {
    fn input(&self) -> SentenceInput<'_> {
        SentenceInput {
            story: &self.story,
            lead: &self.lead,
            latest: &self.latest,
            now_bar: self.now_bar,
            now_t: T0 + i64::try_from(self.now_bar).unwrap() * DAY,
            still_holds: self.still_holds,
            current: self.current,
            redacted: self.redacted,
        }
    }
}

fn template(kind: MeaningKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        MeaningKind::Continuation => ("continuation.ma_stack", "close", "slow"),
        MeaningKind::Reversal => ("reversal.rsi_divergence", "close", "price"),
        MeaningKind::Breakout => ("breakout.range", "close", "level"),
        MeaningKind::Squeeze => ("squeeze.bandwidth", "bandwidth", "threshold"),
        MeaningKind::Extreme => ("extreme.rsi", "rsi", "level"),
        MeaningKind::Flow => ("flow.institutional", "net", "avg5"),
    }
}

fn evidence(kind: MeaningKind, dir: Direction) -> BTreeMap<String, f64> {
    let kv = |v: &[(&str, f64)]| -> BTreeMap<String, f64> {
        v.iter().map(|(k, x)| ((*k).to_owned(), *x)).collect()
    };
    let up = dir != Direction::Down;
    match kind {
        MeaningKind::Continuation => kv(&[("close", 148.2), ("fast", 146.72), ("slow", 142.27)]),
        MeaningKind::Reversal => kv(&[("price", 93.45), ("rsi", 41.3), ("prev_rsi", 36.8)]),
        MeaningKind::Breakout => kv(&[("level", 148.2), ("close", 150.13), ("atr", 3.11)]),
        MeaningKind::Squeeze => kv(&[("bandwidth", 0.034), ("threshold", 0.06)]),
        MeaningKind::Extreme if up => kv(&[("level", 70.0), ("prev", 68.9), ("value", 71.4)]),
        MeaningKind::Extreme => kv(&[("level", 30.0), ("prev", 31.2), ("value", 28.6)]),
        MeaningKind::Flow => kv(&[
            ("avg5", 780_000.0),
            ("net", if up { 2_340_000.0 } else { -2_340_000.0 }),
        ]),
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn case(
    kind: MeaningKind,
    dir: Direction,
    time: TimeKind,
    stage: Stage,
    bars_ago: u64,
    still_holds: Option<bool>,
    redacted: bool,
    ledger: LedgerStat,
) -> Case {
    let inst = "THYAO";
    let now_bar = 480;
    let lead_bar = now_bar - bars_ago;
    let (det, subj, key) = template(kind);
    let above = !(kind == MeaningKind::Squeeze || dir == Direction::Down);
    let cond = match time {
        TimeKind::Event => None,
        _ => Some(MeaningCondition {
            subject: subj.to_owned(),
            op: if above { CompareOp::Gt } else { CompareOp::Lt },
            key: key.to_owned(),
            within_bars: (time == TimeKind::Expectation).then_some(3),
        }),
    };
    let license = if redacted {
        License {
            class: LicenseClass::Licensed,
            entitlement: Some(format!("flow/{inst}")),
        }
    } else {
        License {
            class: LicenseClass::Public,
            entitlement: None,
        }
    };
    let meaning = |d: Direction, edge: Edge, fam: EvidenceFamily, t: TimeKind| FactMeaning {
        kind,
        direction: d,
        family: fam,
        time: t,
        edge,
        detector: det.to_owned(),
        detector_version: 1,
        instrument: inst.to_owned(),
        strength: 0.64,
        license: license.clone(),
        geometry: Vec::new(),
        condition: cond.clone(),
        server: redacted,
    };
    let fact = |code: &str, bar: u64, m: FactMeaning, ev: BTreeMap<String, f64>| Fact {
        id: format!("fx.{inst}.{det}.{code}.{bar}"),
        kind: format!("{det}.{code}"),
        series: format!("{inst}.1d"),
        bar,
        t: T0 + i64::try_from(bar).unwrap() * DAY,
        text: String::new(),
        evidence: ev,
        evidence_ref: None,
        meaning: Some(m),
        provenance: Provenance {
            author: Author::Engine {
                name: "qp-facts".to_owned(),
                version: "test".to_owned(),
            },
            at: 0,
            based_on: Vec::new(),
            approved_by: None,
        },
    };
    let fam = if kind == MeaningKind::Flow {
        EvidenceFamily::Flow
    } else {
        EvidenceFamily::Price
    };
    let ev = evidence(kind, dir);
    let lead_edge = if time == TimeKind::State {
        Edge::Enter
    } else {
        Edge::Point
    };
    let lead = fact(
        "lead",
        lead_bar,
        meaning(dir, lead_edge, fam, time),
        ev.clone(),
    );
    let ended_state = time == TimeKind::State && still_holds == Some(false);
    let current = (time == TimeKind::State && !redacted).then(|| {
        let k = ev.get(key).copied().unwrap_or(0.0);
        let s = if above { 1.0 } else { -1.0 };
        let f = if still_holds == Some(true) {
            1.0 + s * 0.02
        } else {
            1.0 - s * 0.03
        };
        round_to(k * f, 4)
    });
    let (latest, reason) = match stage {
        Stage::Confirmed => (
            fact(
                "surge",
                now_bar,
                FactMeaning {
                    family: EvidenceFamily::Volume,
                    ..meaning(dir, Edge::Point, EvidenceFamily::Volume, TimeKind::Event)
                },
                [("avg20".to_owned(), 1.0e6), ("volume".to_owned(), 2.5e6)]
                    .into_iter()
                    .collect(),
            ),
            "new_family",
        ),
        Stage::Weakened if ended_state => (
            fact(
                "exit",
                now_bar,
                meaning(dir, Edge::Exit, fam, time),
                ev.clone(),
            ),
            "state_ended",
        ),
        Stage::Weakened => (
            fact(
                "opp",
                now_bar,
                meaning(dir, Edge::Point, fam, TimeKind::Event),
                ev.clone(),
            ),
            "opposite_fact",
        ),
        Stage::Strengthened => (
            fact(
                "same",
                now_bar,
                meaning(dir, Edge::Point, fam, time),
                ev.clone(),
            ),
            "same_family",
        ),
        Stage::Retracted => (lead.clone(), "data_corrected"),
        _ => (lead.clone(), "outcome"),
    };
    let mut families = vec![fam];
    if stage == Stage::Confirmed {
        families.push(EvidenceFamily::Volume);
    }
    families.sort();
    let t_of = |b: u64| T0 + i64::try_from(b).unwrap() * DAY;
    let mut history = vec![Transition {
        stage: Stage::Born,
        bar: lead_bar,
        t: t_of(lead_bar),
        facts: vec![lead.id.clone()],
        reason: "new_story".to_owned(),
    }];
    if stage != Stage::Born {
        history.push(Transition {
            stage,
            bar: now_bar,
            t: t_of(now_bar),
            facts: Vec::new(),
            reason: reason.to_owned(),
        });
    }
    Case {
        story: Story {
            instrument: inst.to_owned(),
            theme: kind.name().to_owned(),
            kind,
            direction: dir,
            time,
            stage,
            families,
            strength: 0.64,
            importance: 1.0,
            tier: 1,
            series: format!("{inst}.1d"),
            born_bar: lead_bar,
            last_bar: if stage == Stage::Born {
                lead_bar
            } else {
                now_bar
            },
            horizon_bars: 20,
            ledger_key: format!("{}/{det}@1", kind.name()),
            ledger_at_birth: ledger,
            trust: if redacted {
                Trust::Server
            } else {
                Trust::Replica
            },
            history,
            condition: None,
            resolution: None,
        },
        lead,
        latest,
        now_bar,
        still_holds,
        current,
        redacted,
    }
}

fn k26() -> Case {
    let mut c = case(
        MeaningKind::Extreme,
        Direction::Up,
        TimeKind::State,
        Stage::Weakened,
        3,
        Some(false),
        false,
        LedgerStat {
            n: 140,
            realized: 87,
        },
    );
    c.current = Some(55.0);
    c
}

const STAGES: [Stage; 9] = [
    Stage::Candidate,
    Stage::Born,
    Stage::Strengthened,
    Stage::Weakened,
    Stage::Confirmed,
    Stage::Realized,
    Stage::Invalidated,
    Stage::Expired,
    Stage::Retracted,
];

fn dirs(kind: MeaningKind) -> Vec<Direction> {
    if kind == MeaningKind::Squeeze {
        vec![Direction::Neutral]
    } else {
        vec![Direction::Up, Direction::Down]
    }
}

/// Every combination of kind × direction × time kind × stage × still_holds × bars-ago ×
/// ledger, for `redacted`.
fn all_cases(redacted: bool) -> Vec<Case> {
    let mut out = Vec::new();
    for kind in MeaningKind::ALL {
        for dir in dirs(kind) {
            for time in [TimeKind::Event, TimeKind::State, TimeKind::Expectation] {
                for stage in STAGES {
                    for holds in [None, Some(true), Some(false)] {
                        for ago in [0, 1, 2, 5, 12, 22, 30, 100] {
                            for (n, r) in [(0, 0), (8, 3), (140, 87), (140, 60), (140, 30)] {
                                out.push(case(
                                    kind,
                                    dir,
                                    time,
                                    stage,
                                    ago,
                                    holds,
                                    redacted,
                                    LedgerStat { n, realized: r },
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn say(c: &Case, lang: Lang) -> String {
    Template.sentence(&c.input(), lang)
}

#[test]
fn k26_tr() {
    let s = say(&k26(), Lang::Tr);
    assert_eq!(
        s,
        "THYAO'da RSI 3 bar önce 70'in üstüne çıktı, bugün yeniden altına indi; şu an 55, \
         artık 70'in üstünde değil. Aşırı alım sona erdi. Geçmişte bu sinyal 140 seferin \
         87'sinde beklendiği gibi sonuçlandı."
    );
}

#[test]
fn k26_en() {
    let s = say(&k26(), Lang::En);
    assert_eq!(
        s,
        "3 bars ago, THYAO's RSI crossed above 70; it fell back below today and is now 55, \
         no longer above 70. The overbought state has ended. Historically, this signal played \
         out in 87 of 140 cases."
    );
}

#[test]
fn k26_redacted_has_no_digits() {
    let mut c = k26();
    c.redacted = true;
    c.current = None;
    let tr = say(&c, Lang::Tr);
    let en = say(&c, Lang::En);
    assert_eq!(
        tr,
        "THYAO'da RSI birkaç gün önce aşırı alım eşiğinin üstüne çıktı, bugün yeniden altına \
         indi; artık aşırı alım eşiğinin üstünde değil. Aşırı alım sona erdi. Geçmişte bu tür \
         sinyal çoğunlukla beklendiği gibi sonuçlandı."
    );
    assert!(!en.bytes().any(|b| b.is_ascii_digit()), "{en}");
}

const TR_NUMBER_WORDS: [&str; 24] = [
    "sıfır", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz", "on", "yirmi", "otuz",
    "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan", "yüz", "bin", "milyon", "milyar",
    "yarım", "yüzde",
];
const EN_NUMBER_WORDS: [&str; 31] = [
    "zero", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve", "thirteen", "fourteen", "fifteen", "twenty", "thirty", "forty", "fifty", "sixty",
    "seventy", "eighty", "ninety", "hundred", "thousand", "million", "half", "quarter", "dozen",
    "twice", "percent",
];

fn words(s: &str) -> Vec<String> {
    s.chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn redacted_never_has_digits_or_number_words() {
    let cases = all_cases(true);
    assert!(cases.len() > 10_000);
    for c in &cases {
        let p = Plan::new(&c.input());
        for lang in [Lang::Tr, Lang::En] {
            // The guard is not involved: the template family itself has no digit.
            let s = render(&p, lang);
            assert!(!s.bytes().any(|b| b.is_ascii_digit()), "{s}");
            for w in words(&s) {
                let bad = match lang {
                    Lang::Tr => TR_NUMBER_WORDS.iter().any(|n| {
                        w == *n || (n.chars().count() >= 4 && *n != "altı" && w.starts_with(n))
                    }),
                    Lang::En => EN_NUMBER_WORDS.contains(&w.as_str()),
                };
                assert!(!bad, "number word `{w}` in: {s}");
            }
            // No licensed value in any rounded form (they are all digits, but be explicit).
            for v in c.lead.evidence.values() {
                for k in 0..=2 {
                    let r = format!("{}", round_to(v.abs(), k));
                    assert!(!s.contains(&r), "{r} in {s}");
                }
            }
        }
    }
}

#[test]
fn deterministic() {
    for c in all_cases(false).iter().step_by(7) {
        for lang in [Lang::Tr, Lang::En] {
            assert_eq!(say(c, lang), say(c, lang));
        }
    }
}

fn tr_has_past(s: &str) -> bool {
    words(s).iter().any(|w| {
        ["dı", "di", "du", "dü", "tı", "ti", "tu", "tü"]
            .iter()
            .any(|e| w.ends_with(e) && w.chars().count() > 3)
    })
}

#[test]
fn time_markers_per_class() {
    for c in all_cases(false).iter().chain(all_cases(true).iter()) {
        let p = Plan::new(&c.input());
        let tr = render(&p, Lang::Tr);
        let en = render(&p, Lang::En).to_lowercase();
        let trw = words(&tr);
        let has = |w: &str| trw.iter().any(|x| x == w);
        match p.class {
            super::Class::Event => {
                assert!(tr_has_past(&tr), "{tr}");
                assert!(
                    has("önce") || has("bugün") || has("dün") || tr.contains("bu barda"),
                    "{tr}"
                );
                assert!(
                    en.contains("ago") || en.contains("today") || en.contains("yesterday"),
                    "{en}"
                );
            }
            super::Class::Holding => {
                assert!(tr.contains("sürüyor"), "{tr}");
                assert!(!has("artık"), "{tr}");
                assert!(en.contains("still"), "{en}");
                assert!(!en.contains("no longer"), "{en}");
            }
            super::Class::Ended => {
                assert!(tr_has_past(&tr) && has("artık") && has("değil"), "{tr}");
                // S-A: a net-flow state "became stronger than usual" (no "crossed").
                assert!(
                    en.contains("no longer") && (en.contains("crossed") || en.contains("became")),
                    "{en}"
                );
            }
            super::Class::Expectation => {
                assert!(
                    trw.iter().any(|w| w.ends_with("sa") || w.ends_with("se")),
                    "{tr}"
                );
                assert!(en.starts_with("if "), "{en}");
            }
        }
    }
}

/// Engine slots (same list as the harness's invented-number check).
fn slots(c: &Case) -> Vec<f64> {
    let mut v: Vec<f64> = c.lead.evidence.values().copied().collect();
    v.extend(c.latest.evidence.values().copied());
    v.extend(c.current);
    let l = c.story.ledger_at_birth;
    v.extend([
        f64::from(l.n),
        f64::from(l.realized),
        f64::from(l.n - l.realized),
    ]);
    #[allow(clippy::cast_precision_loss)]
    v.extend([
        (c.now_bar - c.lead.bar) as f64,
        (c.now_bar - c.story.last_bar) as f64,
        c.story.families.len() as f64,
        f64::from(c.story.horizon_bars),
        3.0, // within_bars
    ]);
    v
}

#[test]
fn every_number_is_a_slot() {
    for c in all_cases(false).iter().step_by(3) {
        let sl = slots(c);
        for (lang, dec, thou) in [(Lang::Tr, ',', '.'), (Lang::En, '.', ',')] {
            let s = say(c, lang);
            let cs: Vec<char> = s.chars().collect();
            let mut i = 0;
            while i < cs.len() {
                if cs[i].is_ascii_digit() {
                    let st = i;
                    while i < cs.len()
                        && (cs[i].is_ascii_digit()
                            || ((cs[i] == dec || cs[i] == thou)
                                && i + 1 < cs.len()
                                && cs[i + 1].is_ascii_digit()))
                    {
                        i += 1;
                    }
                    let tok: String = cs[st..i].iter().filter(|c| **c != thou).collect();
                    let tok = tok.replace(dec, ".");
                    let k = tok.split_once('.').map_or(0, |(_, f)| f.len());
                    let v: f64 = tok.parse().unwrap();
                    let ok = sl
                        .iter()
                        .any(|x| (round_to(x.abs(), i32::try_from(k).unwrap()) - v).abs() < 1e-9);
                    assert!(ok, "invented `{tok}` in: {s}");
                } else {
                    i += 1;
                }
            }
        }
    }
}

#[test]
fn examples_render() {
    // A few fixed sentences, both languages (also used in NOTES.md).
    let ex = [
        case(
            MeaningKind::Breakout,
            Direction::Up,
            TimeKind::Event,
            Stage::Born,
            5,
            None,
            false,
            LedgerStat {
                n: 140,
                realized: 62,
            },
        ),
        case(
            MeaningKind::Extreme,
            Direction::Up,
            TimeKind::State,
            Stage::Born,
            5,
            Some(true),
            false,
            LedgerStat { n: 8, realized: 3 },
        ),
        case(
            MeaningKind::Breakout,
            Direction::Up,
            TimeKind::Expectation,
            Stage::Born,
            1,
            Some(true),
            false,
            LedgerStat {
                n: 140,
                realized: 62,
            },
        ),
        case(
            MeaningKind::Flow,
            Direction::Up,
            TimeKind::Event,
            Stage::Confirmed,
            1,
            None,
            true,
            LedgerStat {
                n: 140,
                realized: 87,
            },
        ),
    ];
    for c in &ex {
        for lang in [Lang::Tr, Lang::En] {
            let s = say(c, lang);
            assert!(!s.is_empty());
            println!("{s}");
        }
    }
    assert_eq!(
        say(&ex[0], Lang::Tr),
        "THYAO 5 bar önce 148,20'deki direnci yukarı kırdı (kapanış 150,13): yukarı kırılım. \
         Hâlâ geçerli olup olmadığı bilinmiyor. Geçmişte bu sinyal 140 seferin 62'sinde \
         beklendiği gibi sonuçlandı."
    );
    assert_eq!(
        say(&ex[2], Lang::Tr),
        "THYAO 3 bar içinde 148,20'nin üstünde kapanırsa yukarı kırılım teyit olur. Koşul \
         hâlâ açık. Geçmişte bu sinyal 140 seferin 62'sinde beklendiği gibi sonuçlandı."
    );
}

/// S-A fix (M2b): a net-flow condition is said as net buying / selling stronger than
/// usual, never as net flow above / below the average.
#[test]
fn net_flow_condition_says_buying_or_selling_stronger_than_usual() {
    let mut n = 0;
    for c in all_cases(false).iter().chain(all_cases(true).iter()) {
        let p = Plan::new(&c.input());
        if p.kind != MeaningKind::Flow || p.cond.is_none() || p.class == super::Class::Event {
            continue;
        }
        n += 1;
        let tr = render(&p, Lang::Tr);
        let en = render(&p, Lang::En).to_lowercase();
        let (tr_side, en_side) = if p.dir == Direction::Down {
            ("net satış", "net institutional selling")
        } else {
            ("net alış", "net institutional buying")
        };
        assert!(
            tr.contains(tr_side) && tr.contains("her zamankinden güçlü"),
            "{tr}"
        );
        assert!(
            en.contains(en_side) && en.contains("stronger than usual"),
            "{en}"
        );
        for bad in ["ortalamasının altında", "ortalamasının altına", "net akış"] {
            assert!(!tr.contains(bad), "{tr}");
        }
        for bad in [
            "below the recent average",
            "above the recent average",
            "institutional flow in",
        ] {
            assert!(!en.contains(bad), "{en}");
        }
    }
    assert!(n > 0);
}

/// S-A fix (M5): a realized / invalidated extreme story says whether the stretch went on.
#[test]
fn extreme_outcome_says_whether_the_stretch_continued() {
    let mut n = 0;
    for c in all_cases(false).iter().chain(all_cases(true).iter()) {
        let p = Plan::new(&c.input());
        if p.kind != MeaningKind::Extreme {
            continue;
        }
        let tr = render(&p, Lang::Tr);
        let en = render(&p, Lang::En);
        match p.stage {
            Stage::Realized => {
                n += 1;
                assert!(
                    tr.contains("Uzama sürdü") && tr.contains("gerçekleşti sayıldı"),
                    "{tr}"
                );
                assert!(en.contains("The stretch continued"), "{en}");
                assert!(!en.contains("expected outcome"), "{en}");
            }
            Stage::Invalidated => {
                n += 1;
                assert!(
                    tr.contains("Uzama sürmedi") && tr.contains("gerçekleşmedi sayıldı"),
                    "{tr}"
                );
                assert!(en.contains("The stretch did not continue"), "{en}");
            }
            _ => {}
        }
    }
    assert!(n > 0);
}
