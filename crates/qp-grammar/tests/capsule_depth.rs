//! R3 grammar additions (`docs/GRAMMAR.md` §18): the `glyph` mark, panel-relative
//! positions, segment/extent `x2`, geometry origins, the capsule card and the chart's
//! evidence geometry (including the redacted case), and the glyph rule cross-check
//! against the engine (`qp_capsule::glyph_spec`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use qp_grammar::depth::{
    capsule_view, chart_view, evidence_layers, geometry_field, ledger_text, story_glyph,
    GLYPH_MIN_LEDGER_N,
};
use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, GeoPoint, Geometry, LedgerStat, License,
    LicenseClass, MeaningKind, Stage, Story, TimeKind, Trust,
};
use qp_schema::grammar::{Mark, Origin};
use qp_schema::scene::{DataSource, Prim, SceneDesc};
use qp_schema::{Author, Capsule, CapsuleState, Depth, Fact, Finding, Provenance};

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

fn meaning(kind: MeaningKind, geometry: Vec<Geometry>, entitlement: Option<&str>) -> FactMeaning {
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
            class: if entitlement.is_some() {
                LicenseClass::Licensed
            } else {
                LicenseClass::Public
            },
            entitlement: entitlement.map(str::to_owned),
        },
        geometry,
        condition: None,
        server: false,
    }
}

fn story(stage: Stage, lab: LedgerStat) -> Story {
    Story {
        instrument: "SAMPLE".into(),
        theme: "breakout-up".into(),
        kind: MeaningKind::Breakout,
        direction: Direction::Up,
        time: TimeKind::Event,
        stage,
        families: vec![EvidenceFamily::Price, EvidenceFamily::Volume],
        strength: 0.7,
        importance: 1.0,
        tier: 1,
        series: "px".into(),
        born_bar: 180,
        last_bar: 182,
        horizon_bars: 20,
        ledger_key: "breakout/breakout.range@1".into(),
        ledger_at_birth: lab,
        trust: Trust::Replica,
        history: Vec::new(),
        condition: None,
        resolution: None,
    }
}

/// A provider with an R3 breakout fact at bar 180 (level geometry over 160..=180), its
/// finding and capsule; `evidence` false = redacted (no values).
fn r3(evidence: bool) -> MemProvider {
    let bars = support::sample_bars();
    let mut ev = std::collections::BTreeMap::new();
    if evidence {
        ev.insert("level".to_owned(), 121.5);
        ev.insert("close".to_owned(), bars[180].c);
    }
    let fact = Fact {
        id: "f-brk".into(),
        kind: "breakout.range.up".into(),
        series: "px".into(),
        bar: 180,
        t: bars[180].t,
        text: "breakout".into(),
        evidence: ev,
        evidence_ref: None,
        meaning: Some(meaning(
            MeaningKind::Breakout,
            vec![Geometry::Level {
                key: "level".into(),
                from_bar: 160,
                to_bar: 180,
            }],
            (!evidence).then_some("flow/SAMPLE"),
        )),
        provenance: prov(),
    };
    let finding = Finding {
        id: "fnd-brk".into(),
        claim: "breakout".into(),
        facts: vec![fact.id.clone()],
        confidence: 0.5,
        invalidation: qp_schema::Condition::Text { text: "-".into() },
        provenance: prov(),
    };
    let capsule = Capsule {
        id: "cap-brk".into(),
        finding: "fnd-brk".into(),
        state: CapsuleState::Live,
        revision: 0,
        headline: "SAMPLE broke its 20-bar high two bars ago, on volume.".into(),
        glyphs: Vec::new(),
        view: None,
        depth: Depth::Capsule,
        expires_at: None,
        story: Some(story(
            Stage::Confirmed,
            LedgerStat {
                n: 30,
                realized: 18,
            },
        )),
        provenance: prov(),
    };
    support::provider()
        .with_fact(fact)
        .with_finding(finding)
        .with_capsule(capsule)
}

fn layer_ids(v: &qp_schema::grammar::View) -> Vec<&str> {
    v.layers.iter().filter_map(|l| l.id.as_deref()).collect()
}

fn texts(s: &SceneDesc) -> Vec<String> {
    s.prims()
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn story_glyph_equals_the_engine_rule() {
    let mut n = 0;
    for stage in [
        Stage::Candidate,
        Stage::Born,
        Stage::Confirmed,
        Stage::Realized,
        Stage::Expired,
        Stage::Retracted,
    ] {
        for lab in [
            LedgerStat { n: 0, realized: 0 },
            LedgerStat { n: 19, realized: 7 },
            LedgerStat { n: 20, realized: 7 },
            LedgerStat {
                n: 140,
                realized: 87,
            },
        ] {
            for trust in [Trust::Replica, Trust::Server] {
                for fams in 0..7 {
                    let mut s = story(stage, lab);
                    s.trust = trust;
                    s.families = [
                        EvidenceFamily::Price,
                        EvidenceFamily::Volume,
                        EvidenceFamily::Flow,
                        EvidenceFamily::Derivative,
                        EvidenceFamily::News,
                        EvidenceFamily::News,
                    ][..fams.min(6)]
                        .to_vec();
                    for now in [150, 182, 183, 190, 400] {
                        assert_eq!(
                            story_glyph(&s, now),
                            qp_capsule::glyph_spec(&s, now, GLYPH_MIN_LEDGER_N)
                        );
                        n += 1;
                    }
                }
            }
        }
    }
    assert!(n > 1000);
}

#[test]
fn ledger_line() {
    assert_eq!(
        ledger_text(&LedgerStat { n: 0, realized: 0 }),
        "henüz karne yok"
    );
    assert_eq!(
        ledger_text(&LedgerStat {
            n: 140,
            realized: 87
        }),
        "Karne 87/140"
    );
}

#[test]
fn geometry_fields_address_bars() {
    let f = Fact {
        meaning: Some(meaning(
            MeaningKind::Reversal,
            vec![
                Geometry::Band {
                    upper: "u".into(),
                    lower: "l".into(),
                    from_bar: 3,
                    to_bar: 9,
                },
                Geometry::Points {
                    points: vec![
                        GeoPoint {
                            bar: 4,
                            key: "a".into(),
                        },
                        GeoPoint {
                            bar: 8,
                            key: "b".into(),
                        },
                    ],
                },
            ],
            None,
        )),
        ..r3(true).fact_owned("f-brk")
    };
    assert_eq!(geometry_field(&f, "geometry.0.from_bar"), Some(3.0));
    assert_eq!(geometry_field(&f, "geometry.0.to_bar"), Some(9.0));
    assert_eq!(geometry_field(&f, "geometry.1.points.1.bar"), Some(8.0));
    for bad in [
        "geometry.2.from_bar",
        "geometry.1.from_bar",
        "geometry.0.points.0.bar",
        "geometry.1.points.2.bar",
        "geometry.x.to_bar",
        "level",
    ] {
        assert_eq!(geometry_field(&f, bad), None, "{bad}");
    }
}

trait FactOf {
    fn fact_owned(&self, id: &str) -> Fact;
}

impl FactOf for MemProvider {
    fn fact_owned(&self, id: &str) -> Fact {
        use qp_grammar::DataProvider;
        self.fact(id).unwrap().into_owned()
    }
}

#[test]
fn capsule_card_view_and_scene() {
    let p = r3(true);
    let v = capsule_view("fnd-brk", 60, &p, "capsule-fnd-brk").unwrap();
    assert_eq!(
        layer_ids(&v),
        ["glyph", "sentence", "ledger", "line", "level", "fact"]
    );
    assert_eq!(v.panels.len(), 2);
    let s = compile(&v, &p, &CompileOptions::new(390, 300)).unwrap();
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "Karne 18/30"), "{t:?}");
    assert!(t.iter().any(|x| x.starts_with("SAMPLE broke")), "{t:?}");
    // The card panel draws no grid, value labels or axis lines.
    let card = s.panels[0].rect;
    let in_card =
        |x: i32, y: i32| x >= card.left && x < card.right + 1 && y >= card.top && y <= card.bottom;
    assert!(!s.under.iter().any(|p| match p {
        Prim::Rect { x, y, .. } => in_card(*x, *y),
        Prim::Text { x, baseline, .. } => in_card(*x, *baseline),
        _ => false,
    }));
    // The glyph item: data = the finding, a box of the card glyph size.
    let g = &s.panels[0].layers[0];
    assert_eq!(g.mark, Mark::Glyph);
    assert_eq!(
        g.data,
        DataSource::Finding {
            finding: "fnd-brk".into()
        }
    );
    let hit = g.items[0].hit;
    assert_eq!((hit.right - hit.left, hit.bottom - hit.top), (40, 40));
    assert_eq!(g.items[0].values["confirmations"], 2.0);
    assert!(!g.prims.is_empty());
    // Deterministic.
    assert_eq!(s, compile(&v, &p, &CompileOptions::new(390, 300)).unwrap());
}

/// (R4/M11) The outcome style is selected by name from the compile options; `r3` is the
/// default and draws exactly what no selection draws; an unknown name is `invalid`.
#[test]
fn outcome_style_is_a_compile_option() {
    let p = r3(true);
    let v = capsule_view("fnd-brk", 60, &p, "capsule-fnd-brk").unwrap();
    let base = compile(&v, &p, &CompileOptions::new(390, 300)).unwrap();
    let mut o = CompileOptions::new(390, 300);
    o.outcome = Some("r3".into());
    assert_eq!(compile(&v, &p, &o).unwrap(), base);
    o.outcome = Some("no-such-style".into());
    let e = compile(&v, &p, &o).unwrap_err();
    assert_eq!(e.kind, qp_look::ErrorKind::Invalid);
    assert_eq!(e.path, "(options)/outcome");
}

#[test]
fn chart_view_draws_level_geometry_with_fact_origins() {
    let p = r3(true);
    let v = chart_view("fnd-brk", &p, "present-fnd-brk").unwrap();
    assert_eq!(layer_ids(&v), ["candles", "level", "glyph"]);
    let level = &v.layers[1];
    assert_eq!(level.mark, Mark::Rule);
    let o = |k: &str| match &level.origin[k] {
        Origin::Fact { fact, field } => (fact.clone(), field.clone()),
        x => panic!("{x:?}"),
    };
    assert_eq!(o("/encoding/y"), ("f-brk".into(), "level".into()));
    assert_eq!(
        o("/encoding/x"),
        ("f-brk".into(), "geometry.0.from_bar".into())
    );
    assert_eq!(
        o("/encoding/x2"),
        ("f-brk".into(), "geometry.0.to_bar".into())
    );
    let s = compile(&v, &p, &CompileOptions::new(1200, 600)).unwrap();
    let l = &s.panels[0].layers[1];
    assert_eq!(l.constants["/encoding/y"].value, 121.5);
    assert_eq!(l.constants["/encoding/x"].value, 160.0);
    // A segment, not a full-width rule: it ends at bar 180's column.
    let Prim::Rect { x, w, .. } = l.prims[0] else {
        panic!("{:?}", l.prims[0])
    };
    let right = x + i32::try_from(w).unwrap();
    assert!(right < s.panels[0].rect.right - 100, "{right}");
}

#[test]
fn redacted_evidence_draws_no_geometry_and_says_why() {
    let p = r3(false);
    let v = chart_view("fnd-brk", &p, "present-fnd-brk").unwrap();
    assert_eq!(layer_ids(&v), ["candles", "evidence", "glyph"]);
    let s = compile(&v, &p, &CompileOptions::new(1200, 600)).unwrap();
    let t = texts(&s);
    assert!(
        t.iter()
            .any(|x| x == "Kanıt değerleri veri yetkisi gerektirir: flow/SAMPLE"),
        "{t:?}"
    );
    // No number of the redacted fact appears anywhere.
    assert!(!t.iter().any(|x| x.contains("121.5")));
}

#[test]
fn r2_findings_keep_the_r2_views() {
    // A pre-R3 finding (no meaning, no capsule story): same layers as R2.
    let p = r3(true);
    let bars = support::sample_bars();
    let old = Fact {
        id: "f-old".into(),
        kind: "cross_above".into(),
        series: "px".into(),
        bar: 190,
        t: bars[190].t,
        text: "-".into(),
        evidence: std::collections::BTreeMap::new(),
        evidence_ref: None,
        meaning: None,
        provenance: prov(),
    };
    let p = p.with_fact(old).with_finding(Finding {
        id: "fnd-old".into(),
        claim: "old".into(),
        facts: vec!["f-old".into()],
        confidence: 0.5,
        invalidation: qp_schema::Condition::Text { text: "-".into() },
        provenance: prov(),
    });
    let c = capsule_view("fnd-old", 60, &p, "c").unwrap();
    assert_eq!(layer_ids(&c), ["line", "fact"]);
    let v = chart_view("fnd-old", &p, "v").unwrap();
    assert_eq!(layer_ids(&v), ["candles"]);
}

fn first_err(p: &MemProvider, layer: &str) -> qp_look::GrammarError {
    let v = support::view(&format!(
        r#"{{"id":"t","x":{{"kind":"time"}},"panels":[{{"id":0,"y":{{"kind":"linear"}}}}],
           "layers":[{{"id":"candles","mark":"candle","series":"px","panel":0}},{layer}]}}"#
    ));
    compile(&v, p, &CompileOptions::new(800, 400)).unwrap_err()
}

#[test]
fn glyph_and_segment_errors_are_typed() {
    let p = r3(true);
    let cases: &[(&str, qp_look::ErrorKind, &str)] = &[
        (
            r#"{"id":"g","mark":"glyph","panel":0}"#,
            qp_look::ErrorKind::Invalid,
            "/finding",
        ),
        (
            r#"{"id":"g","mark":"glyph","fact":"f-brk","panel":0}"#,
            qp_look::ErrorKind::Invalid,
            "/mark",
        ),
        (
            r#"{"id":"g","mark":"glyph","finding":"nope","panel":0}"#,
            qp_look::ErrorKind::NotFound,
            "/finding",
        ),
        (
            r#"{"id":"g","mark":"glyph","finding":"fnd-brk","panel":0,"encoding":{"color":"up"}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/color",
        ),
        (
            r#"{"id":"g","mark":"glyph","finding":"fnd-brk","panel":0,"encoding":{"size":{"field":"value","range":[8,30]}}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/size",
        ),
        (
            r#"{"id":"g","mark":"glyph","finding":"fnd-brk","panel":0,"encoding":{"x":180}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/x",
        ),
        (
            r#"{"id":"l","mark":"line","series":"px","panel":0,"encoding":{"y":{"panel":0.5}}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/y",
        ),
        (
            r#"{"id":"s","mark":"rule","panel":0,"encoding":{"y":121.5,"x2":180},
                "origin":{"/encoding/y":{"kind":"fact","fact":"f-brk","field":"level"},
                          "/encoding/x2":{"kind":"fact","fact":"f-brk","field":"geometry.0.to_bar"}}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/x",
        ),
        (
            r#"{"id":"s","mark":"rule","panel":0,"encoding":{"y":121.5,"x":160,"x2":180},
                "origin":{"/encoding/y":{"kind":"fact","fact":"f-brk","field":"level"},
                          "/encoding/x":{"kind":"fact","fact":"f-brk","field":"geometry.0.from_bar"}}}"#,
            qp_look::ErrorKind::NoOrigin,
            "/encoding/x2",
        ),
        (
            r#"{"id":"s","mark":"rule","panel":0,"encoding":{"y":121.5,"x":161,"x2":180},
                "origin":{"/encoding/y":{"kind":"fact","fact":"f-brk","field":"level"},
                          "/encoding/x":{"kind":"fact","fact":"f-brk","field":"geometry.0.from_bar"},
                          "/encoding/x2":{"kind":"fact","fact":"f-brk","field":"geometry.0.to_bar"}}}"#,
            qp_look::ErrorKind::NoOrigin,
            "/encoding/x",
        ),
        (
            r#"{"id":"p","mark":"point","series":"px","panel":0,"encoding":{"x":3}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/x",
        ),
        (
            r#"{"id":"c","mark":"line","series":"px","panel":0,"encoding":{"x2":3}}"#,
            qp_look::ErrorKind::Invalid,
            "/encoding/x2",
        ),
    ];
    for (layer, kind, path) in cases {
        let e = first_err(&p, layer);
        assert_eq!((e.kind, e.path.as_str()), (*kind, *path), "{layer}: {e}");
    }
}

#[test]
fn evidence_layers_for_band_and_points() {
    let mut f = r3(true).fact_owned("f-brk");
    f.evidence = [("u", 130.0), ("l", 120.0), ("a", 125.0), ("b", 127.0)]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
    f.meaning = Some(meaning(
        MeaningKind::Squeeze,
        vec![
            Geometry::Band {
                upper: "u".into(),
                lower: "l".into(),
                from_bar: 170,
                to_bar: 185,
            },
            Geometry::Points {
                points: vec![
                    GeoPoint {
                        bar: 172,
                        key: "a".into(),
                    },
                    GeoPoint {
                        bar: 183,
                        key: "b".into(),
                    },
                ],
            },
        ],
        None,
    ));
    let mut taken = vec!["candles".to_owned()];
    let ls = evidence_layers(&qp_look::look::R4, &f, 0, &mut taken);
    let ids: Vec<&str> = ls.iter().filter_map(|l| l.id.as_deref()).collect();
    assert_eq!(ids, ["band", "swing", "swing-point", "swing-point-2"]);
    let p = support::provider().with_fact(f);
    let mut layers = vec![support::view(
        r#"{"id":"t","x":{"kind":"time"},"layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}"#,
    )
    .layers[0]
        .clone()];
    layers.extend(ls);
    let v = qp_schema::grammar::View {
        layers,
        ..support::view(r#"{"id":"t","x":{"kind":"time"},"layers":[]}"#)
    };
    let s = compile(&v, &p, &CompileOptions::new(1200, 600)).unwrap();
    let band = &s.panels[0].layers[1];
    assert_eq!(band.items.len(), 16, "band covers bars 170..=185 only");
    assert_eq!(band.items[0].bar, Some(170));
    let pts = &s.panels[0].layers[3];
    assert_eq!(pts.items[0].bar, Some(172));
    assert_eq!(pts.items[0].values["y"], 125.0);
    let seg = &s.panels[0].layers[2];
    assert!(matches!(seg.prims[0], Prim::Polyline { .. }));
    assert_eq!(seg.items[0].values["y2"], 127.0);
}

/// K33 (security review): a licensed fact's chart-depth view carries none of its evidence
/// values — geometry is bound by origin only — and for an unentitled reader (a provider
/// whose fact has the evidence redacted) neither the view nor the compiled scene contains
/// any of them; a view made for an entitled reader does not compile to zeros for an
/// unentitled one (typed error instead).
#[test]
fn licensed_evidence_never_reaches_an_unentitled_reader() {
    let secret = [
        ("level", 121.456_789),
        ("atr", 1.234_567),
        ("close", 122.987_654),
    ];
    let mut f = r3(true).fact_owned("f-brk");
    f.evidence = secret.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect();
    let m = f.meaning.as_mut().unwrap();
    m.license = License {
        class: LicenseClass::Licensed,
        entitlement: Some("flow/SAMPLE".into()),
    };
    m.server = true;
    let entitled = r3(true).with_fact(f.clone());
    let mut redacted = f.clone();
    redacted.evidence.clear();
    let unentitled = r3(true).with_fact(redacted);
    let leaks = |s: &str| {
        secret
            .iter()
            .filter(|(_, v)| s.contains(&format!("{v}")))
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    };

    // Entitled: the geometry is drawn, but the view itself holds no evidence number.
    let v = chart_view("fnd-brk", &entitled, "v").unwrap();
    assert_eq!(layer_ids(&v), ["candles", "level", "glyph"]);
    let vj = serde_json::to_string(&v).unwrap();
    assert!(leaks(&vj).is_empty(), "view carries {:?}", leaks(&vj));
    let s = compile(&v, &entitled, &CompileOptions::new(1200, 600)).unwrap();
    assert_eq!(
        s.panels[0].layers[1].constants["/encoding/y"].value,
        121.456_789
    );

    // Unentitled: no geometry layer, the entitlement note; nothing in view or scene.
    let v2 = chart_view("fnd-brk", &unentitled, "v").unwrap();
    assert_eq!(layer_ids(&v2), ["candles", "evidence", "glyph"]);
    let s2 = compile(&v2, &unentitled, &CompileOptions::new(1200, 600)).unwrap();
    for text in [
        serde_json::to_string(&v2).unwrap(),
        serde_json::to_string(&s2).unwrap(),
    ] {
        assert!(leaks(&text).is_empty(), "leaks {:?}", leaks(&text));
    }
    assert!(texts(&s2).iter().any(|t| t.contains("flow/SAMPLE")));

    // The entitled reader's view compiled for the unentitled reader: an error, never 0.
    let e = compile(&v, &unentitled, &CompileOptions::new(1200, 600)).unwrap_err();
    assert_eq!(e.kind, qp_look::ErrorKind::NotFound, "{e}");
    assert_eq!(e.layer.as_deref(), Some("level"));
}

/// The `(px, role, font)` of every text of `s`.
fn text_roles(s: &SceneDesc) -> Vec<(f32, Option<String>, Option<String>, String)> {
    s.prims()
        .filter_map(|p| match p {
            Prim::Text {
                px,
                role,
                font,
                text,
                ..
            } => Some((*px, role.clone(), font.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

/// (R5/T3) The look is a compile option: `r4` named explicitly draws exactly what no
/// look draws (views and scenes equal, no `font`/`role` in the scene); another look
/// changes sizes, colours and the face's height; `roles` writes each text's role;
/// an unknown look is `invalid`.
#[test]
fn looks_are_design_records() {
    use qp_grammar::depth::{capsule_view_in, face_view, face_view_in};
    use qp_grammar::faces::{height, height_in};
    use qp_look::look;
    let p = r3(true);
    let id = "capsule-fnd-brk";
    for f in [None, Some("vadrel"), Some("tavrik"), Some("fotolay")] {
        let v = face_view(f, "fnd-brk", 60, &p, id).unwrap();
        assert_eq!(
            v,
            face_view_in(&look::R4, f, "fnd-brk", 60, &p, id).unwrap()
        );
        assert_eq!(height(f), height_in(&look::R4, f));
        let mut o = CompileOptions::new(390, height(f));
        o.face = f.map(str::to_owned);
        let a = compile(&v, &p, &o).unwrap();
        o.look = Some("r4".into());
        assert_eq!(a, compile(&v, &p, &o).unwrap(), "{f:?}");
        assert!(text_roles(&a)
            .iter()
            .all(|t| t.1.is_none() && t.2.is_none()));
        // With roles: the same scene but for the role names.
        o.roles = true;
        let r = compile(&v, &p, &o).unwrap();
        let tr = text_roles(&r);
        // The reference card and the presented face draw every text in a role; the
        // eliminated M8 designs keep sizes of their own (no role for those).
        if matches!(f, None | Some("vadrel")) {
            assert!(tr.iter().all(|t| t.1.is_some()), "{f:?} {tr:?}");
        }
        assert_eq!(texts(&a), texts(&r));
    }
    // The `r4` card.
    let v = capsule_view("fnd-brk", 60, &p, id).unwrap();
    assert_eq!(
        v,
        capsule_view_in(&look::R4, "fnd-brk", 60, &p, id).unwrap()
    );

    // The trial look `ornek`: larger type, its own colours and face height.
    let ornek = look::by_name(Some("ornek")).unwrap();
    let v = face_view_in(ornek, Some("vadrel"), "fnd-brk", 60, &p, id).unwrap();
    let h = height_in(ornek, Some("vadrel"));
    assert!(h > height(Some("vadrel")));
    let mut o = CompileOptions::new(390, h);
    o.face = Some("vadrel".into());
    o.look = Some("ornek".into());
    o.roles = true;
    let s = compile(&v, &p, &o).unwrap();
    assert_eq!(s.background, ornek.dark.bg);
    let tr = text_roles(&s);
    let headline: Vec<_> = tr
        .iter()
        .filter(|t| t.1.as_deref() == Some("headline"))
        .collect();
    assert!(!headline.is_empty(), "{tr:?}");
    #[allow(clippy::cast_possible_truncation)]
    let hp = ornek.types.headline.px as f32;
    assert!(headline.iter().all(|t| t.0.to_bits() == hp.to_bits()));
    // Its card: body-size sentence, label-size ledger.
    let v = capsule_view_in(ornek, "fnd-brk", 60, &p, id).unwrap();
    let mut o = CompileOptions::new(390, 300);
    o.look = Some("ornek".into());
    o.theme = Some("light".into());
    o.roles = true;
    let s = compile(&v, &p, &o).unwrap();
    assert_eq!(s.background, ornek.light.bg);
    let tr = text_roles(&s);
    #[allow(clippy::cast_possible_truncation)]
    let (label_px, body_px) = (ornek.types.label.px as f32, ornek.types.body.px as f32);
    assert!(tr.iter().any(|t| t.3 == "Karne 18/30"
        && t.1.as_deref() == Some("label")
        && t.0.to_bits() == label_px.to_bits()));
    assert!(tr.iter().any(|t| t.3.starts_with("SAMPLE")
        && t.1.as_deref() == Some("body")
        && t.0.to_bits() == body_px.to_bits()));
    assert_eq!(s, compile(&v, &p, &o).unwrap());
    // (R5/T3b) No scene text under the trial look's smallest role (13 px): the face, the
    // card and the chart with its evidence note, in both schemes.
    #[allow(clippy::cast_possible_truncation)]
    let least = ornek.types.note.px.min(ornek.types.figure.px) as f32;
    assert!(least >= 13.0);
    let views = [
        (
            face_view_in(ornek, Some("vadrel"), "fnd-brk", 60, &p, id).unwrap(),
            h,
        ),
        (capsule_view_in(ornek, "fnd-brk", 60, &p, id).unwrap(), 300),
        (
            qp_grammar::depth::chart_view_in(ornek, "fnd-brk", &p, id).unwrap(),
            300,
        ),
    ];
    for (v, vh) in &views {
        for theme in ["dark", "light"] {
            let mut o = CompileOptions::new(390, *vh);
            o.look = Some("ornek".into());
            o.theme = Some(theme.into());
            let s = compile(v, &p, &o).unwrap();
            for t in text_roles(&s) {
                assert!(t.0 >= least, "{}: {t:?}", v.id);
            }
        }
    }
    o.look = Some("no-such-look".into());
    let e = compile(&v, &p, &o).unwrap_err();
    assert_eq!(e.kind, qp_look::ErrorKind::Invalid);
}

/// (R5/M20) `nobra` question cards over the R3 breakout: the entitled reader sees the
/// evidence geometry inside "Neye dayanıyor?" and the counted track record; the reader
/// whose evidence values are withheld (licensed `flow/SAMPLE`) gets "Neden rakam
/// göremiyorum?", no geometry layer and no digit in any question card; in both looks.
#[test]
fn nobra_question_cards_over_entitled_and_withheld_evidence() {
    use qp_grammar::open::{capsule_view as open_view, OpenCtx};
    use qp_schema::protocol::Language;
    for look in [&qp_look::look::R4, &qp_look::look::ORNEK] {
        for (evidence, ask) in [(true, None), (true, Some("before")), (false, Some("why"))] {
            let p = r3(evidence);
            let ctx = OpenCtx {
                ask,
                lang: Language::Tr,
                check: None,
                frame: [390, 620],
            };
            let v = open_view(
                Some("nobra"),
                look,
                "fnd-brk",
                60,
                &p,
                "capsule-fnd-brk",
                &ctx,
            )
            .unwrap();
            let mut o = CompileOptions::new(390, 620);
            o.look = Some(look.name.to_owned());
            let s = compile(&v, &p, &o).unwrap();
            let names: Vec<&str> = s.panels.iter().filter_map(|x| x.name.as_deref()).collect();
            let has_level = v.layers.iter().any(|l| l.id.as_deref() == Some("level"));
            let card_texts: Vec<String> = s
                .panels
                .iter()
                .filter(|x| x.name.as_deref() != Some("ask:chart"))
                .flat_map(|x| &x.layers)
                .flat_map(|l| &l.prims)
                .filter_map(|x| match x {
                    Prim::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            let all = card_texts.join(" | ");
            if evidence {
                assert_eq!(
                    names,
                    [
                        "ask:what",
                        "ask:who",
                        "ask:before",
                        "ask:basis",
                        "ask:chart"
                    ]
                );
                assert!(has_level, "{:?}", layer_ids(&v));
                if ask == Some("before") {
                    assert!(
                        all.contains("30 hikâyenin") && all.contains("18 tanesinde"),
                        "{all}"
                    );
                }
                assert!(all.contains("Fiyat ve hacim verisine"), "{all}");
            } else {
                assert_eq!(
                    names,
                    [
                        "ask:what",
                        "ask:who",
                        "ask:before",
                        "ask:why",
                        "ask:basis",
                        "ask:chart"
                    ]
                );
                assert!(!has_level, "{:?}", layer_ids(&v));
                assert!(all.contains("Neden rakam göremiyorum?"), "{all}");
                assert!(all.contains("flow/SAMPLE"), "{all}");
                let digits = all.bytes().filter(u8::is_ascii_digit).count();
                assert_eq!(digits, 0, "{all}");
                // Not a number of the redacted fact anywhere in the scene.
                assert!(!texts(&s).iter().any(|x| x.contains("121.5")));
            }
            // Every text of the cards starts inside its card.
            for x in &s.panels {
                for l in &x.layers {
                    for it in &l.items {
                        assert!(it.hit.top >= x.rect.top, "{}: {:?}", l.id, it.hit);
                    }
                }
            }
        }
    }
}
