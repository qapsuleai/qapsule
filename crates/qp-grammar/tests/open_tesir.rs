//! (R5/M20) The `tesir` open capsule design: claim → basis → who verified → track record,
//! top to bottom; the trust row opens its explanation; a reader without the entitlement
//! gets the templated basis row and no number; every look gives the same blocks.

mod common;

use qp_grammar::open::{self, DeviceCheck, TextInput};
use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, Geometry, LedgerStat, License, LicenseClass,
    MeaningKind, Stage, Story, TimeKind, Trust, Validity,
};
use qp_schema::protocol::{CapsuleVoice, GlossEntry, Language};
use qp_schema::scene::{Prim, SceneDesc};
use qp_schema::{Author, Capsule, CapsuleState, Depth, Fact, Finding, Provenance};

const W: u32 = 390;

fn prov() -> Provenance {
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

fn voice(redacted: bool) -> CapsuleVoice {
    CapsuleVoice {
        voice: "marla".into(),
        headline: if redacted {
            "SAMPLE son yirmi günün en yüksek fiyatını aştı · bugün, hâlâ geçerli".into()
        } else {
            "SAMPLE son 20 günün en yüksek fiyatını (121,46) aştı · bugün, hâlâ geçerli".into()
        },
        detail:
            "O gün kapanış eşiğin üstünde oldu. Sonuç en geç on işlem günü içinde belli olacak."
                .into(),
        ledger: "Benzer otuz durumun çoğunda fiyat yükseldi.".into(),
        glossary: vec![
            GlossEntry {
                term: "en yüksek fiyat".into(),
                text: "Sayılan dönemde görülen en yüksek fiyat.".into(),
            },
            GlossEntry {
                term: "benzer durum".into(),
                text: "Aynı tür ve yönde, bundan önce kapanmış iddialar.".into(),
            },
        ],
        face: String::new(),
    }
}

fn fact(evidence: bool, licensed: bool) -> Fact {
    let bars = common::sample_bars();
    let mut ev = std::collections::BTreeMap::new();
    if evidence {
        ev.insert("level".to_owned(), 121.456_789);
    }
    Fact {
        id: "f-brk".into(),
        kind: "breakout.range.up".into(),
        series: "px".into(),
        bar: 180,
        t: bars[180].t,
        text: "breakout".into(),
        evidence: ev,
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind: MeaningKind::Breakout,
            direction: Direction::Up,
            family: if licensed {
                EvidenceFamily::Flow
            } else {
                EvidenceFamily::Price
            },
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
            geometry: vec![Geometry::Level {
                key: "level".into(),
                from_bar: 160,
                to_bar: 180,
            }],
            condition: None,
            server: licensed,
        }),
        provenance: prov(),
    }
}

fn provider(evidence: bool, licensed: bool, lang: Language) -> MemProvider {
    let finding = Finding {
        id: "fnd-brk".into(),
        claim: "breakout".into(),
        facts: vec!["f-brk".into()],
        confidence: 0.5,
        invalidation: qp_schema::Condition::Text { text: "-".into() },
        provenance: prov(),
    };
    let capsule = Capsule {
        id: "cap-brk".into(),
        finding: "fnd-brk".into(),
        state: CapsuleState::Live,
        revision: 0,
        headline: "SAMPLE broke its 20-bar high.".into(),
        glyphs: Vec::new(),
        view: None,
        depth: Depth::Capsule,
        expires_at: None,
        story: Some(Story {
            instrument: "SAMPLE".into(),
            theme: "breakout-up".into(),
            kind: MeaningKind::Breakout,
            direction: Direction::Up,
            time: TimeKind::Event,
            stage: Stage::Born,
            families: vec![EvidenceFamily::Price],
            strength: 0.7,
            importance: 1.0,
            tier: 1,
            series: "px".into(),
            born_bar: 180,
            last_bar: 182,
            horizon_bars: 20,
            ledger_key: "breakout/breakout.range@1".into(),
            ledger_at_birth: LedgerStat {
                n: 30,
                realized: 18,
            },
            trust: if licensed {
                Trust::Server
            } else {
                Trust::Replica
            },
            history: Vec::new(),
            condition: None,
            resolution: None,
        }),
        provenance: prov(),
    };
    common::provider()
        .with_fact(fact(evidence, licensed))
        .with_finding(finding)
        .with_capsule(capsule)
        .with_voice("fnd-brk", voice(!evidence))
        .with_lang(lang)
}

fn scene_of(p: &MemProvider, look: &str) -> SceneDesc {
    let l = qp_grammar::look::or_default(Some(look));
    let v = (open::by_name(Some("tesir")).view)(l, "fnd-brk", 120, p, "capsule-fnd-brk").unwrap();
    let h = open::height_in(l, Some("tesir")).unwrap();
    let mut o = CompileOptions::new(W, h);
    o.look = Some(look.to_owned());
    compile(&v, p, &o).unwrap()
}

/// `(layer id, text, top)` of every text, in paint order.
fn texts(s: &SceneDesc) -> Vec<(String, String, i32)> {
    let mut out = Vec::new();
    for p in &s.panels {
        for l in &p.layers {
            for q in &l.prims {
                if let Prim::Text { text, baseline, .. } = q {
                    out.push((l.id.clone(), text.clone(), *baseline));
                }
            }
        }
    }
    out
}

fn first_top(s: &SceneDesc, id: &str) -> i32 {
    texts(s)
        .iter()
        .find(|(l, _, _)| l == id)
        .map(|(_, _, y)| *y)
        .unwrap_or_else(|| panic!("no text in `{id}`"))
}

#[test]
fn blocks_read_top_to_bottom_in_every_look() {
    for look in qp_grammar::look::names() {
        let s = scene_of(&provider(true, false, Language::Tr), look);
        let order = [
            "headline",
            "open-h-basis",
            "open-basis",
            "open-basis2",
            "open-why",
            "open-h-trust",
            "open-trust",
            "open-trust2",
            "open-h-ledger",
            "ledger",
        ];
        let tops: Vec<i32> = order.iter().map(|id| first_top(&s, id)).collect();
        assert!(tops.windows(2).all(|w| w[0] < w[1]), "{look}: {tops:?}");
        // Nothing is clipped: every text of the scene is whole (no "…").
        for (id, t, _) in texts(&s) {
            assert!(!t.ends_with('…'), "{look}: `{id}` clipped: {t}");
        }
        // The column fits its frame.
        let h = open::height_in(qp_grammar::look::or_default(Some(look)), Some("tesir")).unwrap();
        assert_eq!(s.height, h);
        // The evidence in small: the level geometry is drawn.
        assert!(s
            .panels
            .iter()
            .flat_map(|p| &p.layers)
            .any(|l| l.id == "level"));
    }
}

#[test]
fn basis_names_the_threshold_with_its_value() {
    let s = scene_of(&provider(true, false, Language::Tr), "r4");
    let t = texts(&s);
    let basis: Vec<&str> = t
        .iter()
        .filter(|(l, _, _)| l == "open-basis")
        .map(|(_, x, _)| x.as_str())
        .collect();
    assert_eq!(basis, ["En yüksek fiyat · 121,46"]);
    let en = scene_of(&provider(true, false, Language::En), "r4");
    let b2: Vec<String> = texts(&en)
        .into_iter()
        .filter(|(l, _, _)| l == "open-h-basis")
        .map(|(_, x, _)| x)
        .collect();
    assert_eq!(b2, ["BASIS"]);
}

#[test]
fn trust_row_opens_its_explanation_and_follows_the_device() {
    let p = provider(true, false, Language::Tr);
    let s = scene_of(&p, "r4");
    let row = |s: &SceneDesc| {
        texts(s)
            .into_iter()
            .find(|(l, _, _)| l == "open-trust")
            .map(|(_, x, _)| x)
            .unwrap()
    };
    assert_eq!(row(&s), "Sunucu doğruladı");
    let term = s
        .terms
        .iter()
        .find(|t| t.layer == "open-trust")
        .expect("trust term");
    assert_eq!(term.term, "Kim doğruladı?");
    assert!(term.text.chars().count() <= 160 && !term.text.chars().any(|c| c.is_ascii_digit()));
    let panel = s
        .panels
        .iter()
        .find(|p| p.name.as_deref() == Some("trust"))
        .unwrap();
    assert_eq!(term.hit, panel.rect, "the whole block is the tap target");

    // The device's check changes the words, not the layout nor the tap target.
    let d = scene_of(&p.clone().with_check("fnd-brk", DeviceCheck::Device), "r4");
    assert_eq!(row(&d), "Bu cihazda sınandı");
    let td = d.terms.iter().find(|t| t.layer == "open-trust").unwrap();
    assert_eq!(td, term);
    let rects = |s: &SceneDesc| s.panels.iter().map(|p| p.rect).collect::<Vec<_>>();
    assert_eq!(rects(&s), rects(&d));
}

#[test]
fn an_unentitled_reader_gets_the_templated_basis_and_no_number() {
    let p = provider(false, true, Language::Tr);
    let s = scene_of(&p, "r4");
    let t = texts(&s);
    let basis: Vec<&str> = t
        .iter()
        .filter(|(l, _, _)| l.starts_with("open-basis"))
        .map(|(_, x, _)| x.as_str())
        .collect();
    assert_eq!(basis[0], "Kilitli: flow/SAMPLE yetkisi");
    assert!(basis[1].contains("lisanslı"), "{basis:?}");
    assert!(!s
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .any(|l| l.id == "level"));
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("121.4"), "the evidence value leaked");
    // No digit in any text but the instrument and the entitlement name.
    for (id, x, _) in &t {
        let rest = x.replace("SAMPLE", "");
        assert!(!rest.chars().any(|c| c.is_ascii_digit()), "`{id}`: {x}");
    }
    let row = t.iter().find(|(l, _, _)| l == "open-trust2").unwrap();
    assert!(row.1.contains("sınayamaz"), "{row:?}");
}

#[test]
fn the_text_column_says_the_same_without_a_scene() {
    let f = fact(false, true);
    let v = voice(true);
    let t = TextInput {
        lang: Language::Tr,
        headline: "SAMPLE",
        voice: Some(&v),
        facts: std::slice::from_ref(&f),
        trust: Trust::Server,
        check: None,
        ledger: LedgerStat {
            n: 30,
            realized: 18,
        },
        validity: Validity::Valid,
    };
    let col = (open::by_name(Some("tesir")).text.unwrap())(&t);
    let lines: Vec<&str> = col.lines().collect();
    assert_eq!(lines[0], v.headline);
    let i = lines.iter().position(|l| *l == "DAYANAK").unwrap();
    assert_eq!(lines[i + 1], "Kilitli: flow/SAMPLE yetkisi");
    assert!(lines.contains(&"KİM DOĞRULADI") && lines.contains(&"Sunucu doğruladı"));
    assert!(lines.contains(&"KARNE") && lines.contains(&v.ledger.as_str()));
    let rest = col.replace("SAMPLE", "");
    assert!(!rest.chars().any(|c| c.is_ascii_digit()), "{col}");
}

#[test]
fn the_reference_is_untouched() {
    let p = provider(true, false, Language::Tr);
    let l = &qp_grammar::look::R4;
    let a = (open::by_name(None).view)(l, "fnd-brk", 120, &p, "v").unwrap();
    let b = qp_grammar::depth::capsule_view_in(l, "fnd-brk", 120, &p, "v").unwrap();
    assert_eq!(a, b);
}
