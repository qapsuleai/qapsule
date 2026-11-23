//! Synthetic inputs of the characterisation tests: the sample bars and studies
//! (`tests/support`), three R3 facts (level, band, swing points), findings with capsules in
//! every story stage, a voice, headline pieces, story rows and feed items. No market data.

use qp_grammar::{compile, HeadlineParts, MemProvider};
use qp_look::look::{self as look_registry, Look};
use qp_look::GrammarError;
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, GeoPoint, Geometry, LedgerStat, License,
    LicenseClass, MeaningKind, Stage, Story, TimeKind, Trust, Validity,
};
use qp_schema::grammar::View;
use qp_schema::protocol::{
    CapsuleItem, CapsuleSentence, CapsuleVoice, GlossEntry, Language, StoryFace, StoryOutcome,
    StoryRow,
};
use qp_schema::scene::{Prim, SceneDesc};
use qp_schema::{
    Author, Capsule, CapsuleState, Condition, Depth, Fact, Finding, Point, Provenance,
};

use crate::support;

/// Every registered look, in registry order.
pub fn looks() -> Vec<&'static str> {
    look_registry::names()
}

/// The look record named `name` (unknown: the default).
pub fn look(name: &str) -> &'static Look {
    look_registry::or_default(Some(name))
}

pub fn prov() -> Provenance {
    Provenance {
        author: Author::Engine {
            name: "t".into(),
            version: "0".into(),
        },
        at: 0,
        based_on: Vec::new(),
        approved_by: None,
    }
}

fn meaning(kind: MeaningKind, geometry: Vec<Geometry>, licensed: bool) -> FactMeaning {
    FactMeaning {
        kind,
        direction: Direction::Up,
        family: EvidenceFamily::Price,
        time: TimeKind::Event,
        edge: Edge::Point,
        detector: "breakout.range".into(),
        detector_version: 1,
        instrument: "SAMPLE".into(),
        strength: 0.7,
        license: License {
            class: if licensed {
                LicenseClass::Licensed
            } else {
                LicenseClass::Public
            },
            entitlement: licensed.then(|| "flow/SAMPLE".to_owned()),
        },
        geometry,
        condition: None,
        server: licensed,
    }
}

fn fact(id: &str, bar: u64, ev: &[(&str, f64)], m: Option<FactMeaning>) -> Fact {
    let bars = support::sample_bars();
    Fact {
        id: id.into(),
        kind: "breakout.range.up".into(),
        series: "px".into(),
        bar,
        t: bars[usize::try_from(bar).unwrap()].t,
        text: format!("fact {id}"),
        evidence: ev.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
        evidence_ref: None,
        meaning: m,
        provenance: prov(),
    }
}

/// The R3 facts: `f-brk` (level 160..=180), `f-sq` (band 170..=185), `f-div` (two swing
/// points), `f-lock` (licensed, no evidence values: the redacted case) and the plain
/// `f-rsi` (RSI crossing 70, no meaning).
pub fn facts() -> Vec<Fact> {
    let lvl = Geometry::Level {
        key: "level".into(),
        from_bar: 160,
        to_bar: 180,
    };
    let band = Geometry::Band {
        upper: "u".into(),
        lower: "l".into(),
        from_bar: 170,
        to_bar: 185,
    };
    let pts = Geometry::Points {
        points: vec![
            GeoPoint {
                bar: 172,
                key: "a".into(),
            },
            GeoPoint {
                bar: 188,
                key: "b".into(),
            },
        ],
    };
    let mut rsi = fact("f-rsi", 150, &[("level", 70.0)], None);
    rsi.series = "rsi14".into();
    rsi.kind = "cross_above".into();
    vec![
        fact(
            "f-brk",
            180,
            &[("level", 121.456_789), ("close", 122.5)],
            Some(meaning(MeaningKind::Breakout, vec![lvl.clone()], false)),
        ),
        fact(
            "f-sq",
            185,
            &[("u", 125.0), ("l", 115.0)],
            Some(meaning(MeaningKind::Squeeze, vec![band], false)),
        ),
        fact(
            "f-div",
            188,
            &[("a", 118.0), ("b", 124.0)],
            Some(meaning(MeaningKind::Reversal, vec![pts], false)),
        ),
        fact(
            "f-lock",
            180,
            &[],
            Some(meaning(MeaningKind::Breakout, vec![lvl], true)),
        ),
        rsi,
    ]
}

fn story(kind: MeaningKind, stage: Stage, born: u64, trust: Trust, n: u32) -> Story {
    Story {
        instrument: "SAMPLE".into(),
        theme: "breakout-up".into(),
        kind,
        direction: Direction::Up,
        time: TimeKind::Event,
        stage,
        families: vec![EvidenceFamily::Price, EvidenceFamily::Volume],
        strength: 0.7,
        importance: 1.0,
        tier: 1,
        series: "px".into(),
        born_bar: born,
        last_bar: born + 2,
        horizon_bars: 20,
        ledger_key: "breakout/breakout.range@1".into(),
        ledger_at_birth: LedgerStat {
            n,
            realized: n * 3 / 5,
        },
        trust,
        history: Vec::new(),
        condition: None,
        resolution: None,
    }
}

/// `(finding, fact, kind, stage, trust, ledger n)`: one capsule each.
pub const CAPSULES: &[(&str, &str, MeaningKind, Stage, Trust, u32)] = &[
    (
        "fnd-brk",
        "f-brk",
        MeaningKind::Breakout,
        Stage::Confirmed,
        Trust::Replica,
        30,
    ),
    (
        "fnd-parts",
        "f-brk",
        MeaningKind::Breakout,
        Stage::Born,
        Trust::Replica,
        4,
    ),
    (
        "fnd-sq",
        "f-sq",
        MeaningKind::Squeeze,
        Stage::Strengthened,
        Trust::Server,
        12,
    ),
    (
        "fnd-div",
        "f-div",
        MeaningKind::Reversal,
        Stage::Weakened,
        Trust::Replica,
        0,
    ),
    (
        "fnd-lock",
        "f-lock",
        MeaningKind::Breakout,
        Stage::Born,
        Trust::Server,
        30,
    ),
    (
        "fnd-real",
        "f-brk",
        MeaningKind::Breakout,
        Stage::Realized,
        Trust::Replica,
        30,
    ),
    (
        "fnd-inv",
        "f-brk",
        MeaningKind::Breakout,
        Stage::Invalidated,
        Trust::Replica,
        30,
    ),
    (
        "fnd-exp",
        "f-brk",
        MeaningKind::Breakout,
        Stage::Expired,
        Trust::Replica,
        30,
    ),
    (
        "fnd-ret",
        "f-brk",
        MeaningKind::Breakout,
        Stage::Retracted,
        Trust::Replica,
        30,
    ),
];

/// Every finding with a capsule.
pub fn capsule_findings() -> Vec<&'static str> {
    CAPSULES.iter().map(|c| c.0).collect()
}

fn finding(id: &str, facts: &[&str]) -> Finding {
    Finding {
        id: id.into(),
        claim: format!("claim of {id}"),
        facts: facts.iter().map(|f| (*f).to_owned()).collect(),
        confidence: 0.5,
        invalidation: Condition::Text { text: "-".into() },
        provenance: prov(),
    }
}

pub fn voice(lang: Language) -> CapsuleVoice {
    let tr = lang == Language::Tr;
    CapsuleVoice {
        voice: "marla".into(),
        headline: if tr {
            "SAMPLE son 20 günün en yüksek fiyatını (121,46) aştı · bugün, hâlâ geçerli".into()
        } else {
            "SAMPLE broke its 20-day high (121.46) · today, still holds".into()
        },
        detail: "O gün kapanış eşiğin üstünde oldu. Sonuç on işlem günü içinde belli olacak."
            .into(),
        ledger: "Benzer otuz durumun çoğunda fiyat yükseldi.".into(),
        glossary: vec![
            GlossEntry {
                term: "en yüksek fiyat".into(),
                text: "Sayılan dönemde görülen en yüksek fiyat.".into(),
            },
            GlossEntry {
                term: "kapanış".into(),
                text: "Günün son fiyatı.".into(),
            },
        ],
        face: String::new(),
    }
}

pub fn parts(lang: Language, redacted: bool) -> HeadlineParts {
    let tr = lang == Language::Tr;
    HeadlineParts {
        lang,
        instrument: "SAMPLE".into(),
        event: if tr {
            "yukarı kırılım"
        } else {
            "upside breakout"
        }
        .into(),
        direction_word: if tr { "yukarı" } else { "up" }.into(),
        when: if redacted {
            "birkaç gün önce"
        } else {
            "3 işlem günü önce"
        }
        .into(),
        moved: if tr { "dün" } else { "yesterday" }.into(),
        validity_phrase: if tr { "hâlâ geçerli" } else { "still holds" }.into(),
        ledger_short: if redacted {
            "az örnek"
        } else {
            "7'de 3 tuttu"
        }
        .into(),
        stage: Stage::Born,
        validity: Validity::Valid,
        redacted,
        ledger_n: (!redacted).then_some(7),
        few_cases: true,
    }
}

/// Points series `q` (20 points after the last bar).
fn points() -> Vec<Point> {
    let last = support::sample_bars().last().unwrap().clone();
    (0..=20)
        .map(|k| Point {
            t: last.t + k * 86_400_000,
            v: Some(last.c * (1.0 + 0.004 * f64::from(u32::try_from(k).unwrap()))),
        })
        .collect()
}

/// The provider: sample bars and studies (`px`, `sma20`, `sma50`, `rsi14`, `macd`, `bb`),
/// points `q`, the facts, findings `fnd-rsi` (no capsule), `fnd-two` (two facts),
/// `fnd-missing` (an absent fact) and the capsule findings; `fnd-brk` has a voice,
/// `fnd-parts` and `fnd-sq` have headline pieces (`fnd-sq` redacted).
pub fn provider(lang: Language) -> MemProvider {
    let mut p = support::provider().with_points("q", points());
    for f in facts() {
        p = p.with_fact(f);
    }
    p = p
        .with_finding(finding("fnd-rsi", &["f-rsi"]))
        .with_headline("fnd-rsi", "RSI(14) crossed 70.")
        .with_finding(finding("fnd-two", &["f-rsi", "f-brk"]))
        .with_finding(finding("fnd-missing", &["nope"]));
    for (fid, fact, kind, stage, trust, n) in CAPSULES {
        let f = facts().into_iter().find(|f| f.id == *fact).unwrap();
        p = p.with_finding(finding(fid, &[fact])).with_capsule(Capsule {
            id: format!("cap-{fid}"),
            finding: (*fid).into(),
            state: CapsuleState::Live,
            revision: 0,
            headline: format!("SAMPLE {kind:?} {stage:?} ({fid})."),
            glyphs: Vec::new(),
            view: None,
            depth: Depth::Capsule,
            expires_at: None,
            story: Some(story(*kind, *stage, f.bar, *trust, *n)),
            provenance: prov(),
        });
    }
    p.with_voice("fnd-brk", voice(lang))
        .with_headline_parts("fnd-parts", parts(lang, false))
        .with_headline_parts("fnd-sq", parts(lang, true))
        .with_lang(lang)
}

/// Compiles `v` at `w`×`h` in `look`, with an optional face design.
pub fn compile_in(
    v: &View,
    p: &MemProvider,
    look: &str,
    w: u32,
    h: u32,
    face: Option<&str>,
) -> Result<SceneDesc, GrammarError> {
    let mut o = qp_designs::options(w, h);
    o.look = Some(look.to_owned());
    o.face = face.map(str::to_owned);
    compile(v, p, &o)
}

/// Texts of a scene in paint order (`layer:text`).
pub fn texts(s: &SceneDesc) -> Vec<String> {
    let mut out = Vec::new();
    for p in &s.panels {
        for l in &p.layers {
            for q in &l.prims {
                if let Prim::Text { text, .. } = q {
                    out.push(format!("{}:{text}", l.id));
                }
            }
        }
    }
    out
}

/// The text of the first text primitive of the first layer reading field `field`.
pub fn field_text(s: &SceneDesc, field: &str) -> Option<String> {
    s.panels
        .iter()
        .flat_map(|p| &p.layers)
        .filter(|l| l.fields.iter().any(|f| f == field))
        .flat_map(|l| &l.prims)
        .find_map(|q| match q {
            Prim::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
}

/// `WxH panels/layers/prims/items terms=…`: a scene's shape.
pub fn shape(s: &SceneDesc) -> String {
    let layers: Vec<_> = s.panels.iter().flat_map(|p| &p.layers).collect();
    let prims: usize = layers.iter().map(|l| l.prims.len()).sum();
    let items: usize = layers.iter().map(|l| l.items.len()).sum();
    let terms: Vec<&str> = s.terms.iter().map(|t| t.term.as_str()).collect();
    format!(
        "{}x{} p{} l{} prims{} items{} over{} terms={}",
        s.width,
        s.height,
        s.panels.len(),
        layers.len(),
        prims,
        items,
        s.over.len(),
        terms.join("/")
    )
}

/// `n` story rows over the sample bars, deterministic (lives of 3–20 bars, outcomes in
/// turn, the last three open).
pub fn story_rows(n: u64) -> Vec<StoryRow> {
    let bars = support::sample_bars();
    let t = |b: u64| bars[usize::try_from(b).unwrap()].t;
    let outcomes = [
        Stage::Realized,
        Stage::Invalidated,
        Stage::Expired,
        Stage::Retracted,
    ];
    let kinds = [
        MeaningKind::Breakout,
        MeaningKind::Reversal,
        MeaningKind::Squeeze,
        MeaningKind::Extreme,
    ];
    (0..n)
        .map(|i| {
            let born = 100 + (i * 89) % 90;
            let len = 3 + (i * 7) % 18;
            let open = i + 3 >= n;
            let last = (born + len).min(199);
            let stage = if open {
                Stage::Born
            } else {
                outcomes[usize::try_from(i % 4).unwrap()]
            };
            StoryRow {
                id: format!("cap.S.{i}"),
                finding: format!("fnd-s{i}"),
                series: "px".into(),
                kind: kinds[usize::try_from(i % 4).unwrap()],
                direction: [Direction::Up, Direction::Down, Direction::Neutral]
                    [usize::try_from(i % 3).unwrap()],
                tier: u8::try_from(1 + i % 3).unwrap(),
                born_bar: born,
                born_t: t(born),
                last_bar: last,
                last_t: t(last),
                closed_bar: (!open).then_some(last),
                closed_t: (!open).then(|| t(last)),
                stage,
                outcome: StoryOutcome::of(stage),
                validity: if open {
                    Validity::Valid
                } else {
                    Validity::Closed
                },
                in_feed: i % 5 != 0,
                hidden: None,
                coin_flip: false,
                face: StoryFace {
                    event: "yukarı kırılım".into(),
                    direction_word: "yukarı".into(),
                    when: "dün".into(),
                    moved: "dün".into(),
                    validity_phrase: if open {
                        "hâlâ geçerli"
                    } else {
                        "gerçekleşti"
                    }
                    .into(),
                    ledger_short: "7'de 3 tuttu".into(),
                    few_cases: false,
                    ledger_n: Some(7),
                },
                redacted: i % 4 == 3,
            }
        })
        .collect()
}

/// A feed item per capsule finding (stage, validity and bar from [`CAPSULES`]).
pub fn feed_items() -> Vec<CapsuleItem> {
    let bars = support::sample_bars();
    CAPSULES
        .iter()
        .enumerate()
        .map(|(i, (fid, _, kind, stage, trust, n))| {
            let born = 150 + u64::try_from(i).unwrap() * 5;
            let closed = matches!(
                stage,
                Stage::Realized | Stage::Invalidated | Stage::Expired | Stage::Retracted
            );
            CapsuleItem {
                id: format!("cap-{fid}"),
                finding: (*fid).into(),
                instrument: "SAMPLE".into(),
                series: "px".into(),
                kind: *kind,
                direction: Direction::Up,
                time: TimeKind::Event,
                stage: *stage,
                state: CapsuleState::Live,
                tier: u8::try_from(1 + i % 3).unwrap(),
                importance: 1.0 - 0.1 * f64::from(u32::try_from(i).unwrap()),
                confirmations: u8::try_from(i % 2).unwrap(),
                trust: *trust,
                born_bar: born,
                last_bar: born + 2,
                t: bars[usize::try_from(born + 2).unwrap()].t,
                ledger_at_birth: LedgerStat {
                    n: *n,
                    realized: n / 2,
                },
                sentence: CapsuleSentence {
                    text: format!("{fid} sentence"),
                    tr: format!("{fid} cümle"),
                    en: format!("{fid} sentence"),
                },
                validity: if closed {
                    Validity::Closed
                } else if i == 3 {
                    Validity::NoLonger
                } else {
                    Validity::Valid
                },
                relevant: i == 2,
                redacted: i == 4,
                coin_flip: i == 1,
                voice: None,
            }
        })
        .collect()
}
