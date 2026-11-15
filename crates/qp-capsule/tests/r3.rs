//! R3/S-A: the chosen strategies (`EngineConfig::r3`, `realize::Template`) with the real
//! detectors (`qp_facts::default_detectors`): realizer fixture, look-ahead, history
//! replay, sentences from the engine (K26, K33).

#[path = "../examples/support/mod.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet};

use qp_capsule::engine::{Engine, EngineConfig};
use qp_capsule::pipeline::simulate_engine;
use qp_capsule::realize::Template;
use qp_capsule::sentence::{condition_now, sentences_at, Reader};
use qp_capsule::{simulate, InstrumentInput, Lang, Realizer, SentenceInput, SimResult};
use qp_facts::{default_detectors, FlowPoint};
use qp_schema::capsule::{MeaningKind, Stage, Story};
use qp_schema::{Bar, Fact};
use serde::Deserialize;

fn json<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_string(x).unwrap()
}

// ---------------------------------------------------------------------------
// Realizer fixture (byte-identity with the measured M2 template, S-A.md)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Situation {
    id: String,
    story: Story,
    lead: Fact,
    latest: Fact,
    now_bar: u64,
    now_t: i64,
    still_holds: Option<bool>,
    current: Option<f64>,
    redacted: bool,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
struct Pair {
    tr: String,
    en: String,
}

#[derive(Deserialize)]
struct SentenceFixture {
    measured: BTreeMap<String, Pair>,
    product_changes: BTreeMap<String, Pair>,
}

/// Every M2 harness situation: the product realizer writes exactly the measured
/// sentence, except the documented fixes (`product_changes`, 14 situations). The fixture
/// itself is checked against the measured crate by `rounds/R3/experiments/S/crosscheck`.
#[test]
fn realizer_equals_measured_m2_template_except_documented_fixes() {
    let sits: Vec<Situation> =
        serde_json::from_str(include_str!("fixtures/m2_inputs.json")).unwrap();
    let fx: SentenceFixture =
        serde_json::from_str(include_str!("fixtures/m2_sentences.json")).unwrap();
    assert_eq!(sits.len(), 73);
    assert_eq!(fx.measured.len(), 73);
    assert_eq!(fx.product_changes.len(), 14);
    for s in &sits {
        let inp = SentenceInput {
            story: &s.story,
            lead: &s.lead,
            latest: &s.latest,
            now_bar: s.now_bar,
            now_t: s.now_t,
            still_holds: s.still_holds,
            current: s.current,
            redacted: s.redacted,
        };
        let got = Pair {
            tr: Template.sentence(&inp, Lang::Tr),
            en: Template.sentence(&inp, Lang::En),
        };
        let want = fx
            .product_changes
            .get(&s.id)
            .unwrap_or_else(|| &fx.measured[&s.id]);
        assert_eq!(&got, want, "situation {}", s.id);
    }
}

// ---------------------------------------------------------------------------
// Look-ahead with real detectors and the chosen config (verifier note N4)
// ---------------------------------------------------------------------------

fn small_market(seed: u64) -> Vec<InstrumentInput> {
    support::synthetic(seed, 8, 260)
}

fn run(inp: &[InstrumentInput]) -> SimResult {
    simulate(inp, &default_detectors, EngineConfig::r3())
}

/// Bars with `t >= cut` (and their flow) replaced by another seeded walk.
fn perturb_from(inp: &[InstrumentInput], cut: i64, seed: u64) -> Vec<InstrumentInput> {
    inp.iter()
        .enumerate()
        .map(|(k, x)| {
            let mut z = support::mix(seed ^ ((k as u64) << 20));
            let mut u = move || {
                z = support::mix(z);
                (z >> 11) as f64 / (1u64 << 53) as f64
            };
            let mut out = x.clone();
            let mut c = 0.0;
            for (i, b) in x.bars.iter().enumerate() {
                if b.t < cut {
                    c = b.c;
                    continue;
                }
                let o = c;
                c = (c * (1.0 + 0.06 * (u() - 0.5))).max(0.01);
                out.bars[i] = Bar {
                    t: b.t,
                    o,
                    h: o.max(c) * (1.0 + 0.01 * u()),
                    l: o.min(c) * (1.0 - 0.01 * u()),
                    c,
                    v: 1000.0 + 5e6 * u(),
                };
                if let Some(p) = out.flow.as_mut().and_then(|f| f.get_mut(i)) {
                    p.values = BTreeMap::from([("net".to_owned(), 1e6 * (u() - 0.5))]);
                }
            }
            out
        })
        .collect()
}

/// Bars with `t > cut` permuted (records move, time stamps stay); flow moves along.
fn shuffle_after(inp: &[InstrumentInput], cut: i64, seed: u64) -> Vec<InstrumentInput> {
    inp.iter()
        .enumerate()
        .map(|(k, x)| {
            let mut z = support::mix(seed ^ ((k as u64) << 24));
            let idx: Vec<usize> = (0..x.bars.len()).filter(|&i| x.bars[i].t > cut).collect();
            let mut perm = idx.clone();
            for j in (1..perm.len()).rev() {
                z = support::mix(z);
                perm.swap(j, (z % (j as u64 + 1)) as usize);
            }
            let mut out = x.clone();
            for (d, s) in idx.iter().zip(&perm) {
                let t = x.bars[*d].t;
                out.bars[*d] = Bar {
                    t,
                    ..x.bars[*s].clone()
                };
                if let (Some(fo), Some(fi)) = (out.flow.as_mut(), x.flow.as_ref()) {
                    fo[*d] = FlowPoint {
                        t,
                        values: fi[*s].values.clone(),
                    };
                }
            }
            out
        })
        .collect()
}

fn ledger_at(r: &SimResult, keys: &BTreeSet<String>, t: i64) -> Vec<(String, u32, u32)> {
    keys.iter()
        .map(|k| {
            let s = r.ledger_stat(k, t);
            (k.clone(), s.n, s.realized)
        })
        .collect()
}

#[test]
fn look_ahead_real_detectors_chosen_config() {
    let base_in = small_market(5);
    let base = run(&base_in);
    assert!(base.events.len() > 300, "{}", base.events.len());
    assert!(base.records.len() > 50, "{}", base.records.len());
    let n = base_in[0].bars.len();
    for q in [1usize, 2, 3] {
        let cut = base_in[0].bars[n * q / 4].t;
        for (name, alt_in) in [
            ("perturb", perturb_from(&base_in, cut, 17 + q as u64)),
            ("shuffle", shuffle_after(&base_in, cut, 29 + q as u64)),
        ] {
            let alt = run(&alt_in);
            // `perturb` changes t >= cut, `shuffle` only t > cut.
            let past = |e: &&qp_capsule::StoryEvent| {
                if name == "perturb" {
                    e.transition.t < cut
                } else {
                    e.transition.t <= cut
                }
            };
            let a: Vec<_> = base.events.iter().filter(past).collect();
            let b: Vec<_> = alt.events.iter().filter(past).collect();
            assert!(!a.is_empty());
            assert_eq!(
                json(&a),
                json(&b),
                "{name} cut {q}/4: events before the cut"
            );
            // ledger_at_birth of every story born before the cut.
            let births = |r: &SimResult| -> Vec<(String, u32, u32)> {
                r.events
                    .iter()
                    .filter(|e| e.transition.stage == Stage::Born && e.transition.t < cut)
                    .map(|e| {
                        (
                            e.capsule.clone(),
                            e.story.ledger_at_birth.n,
                            e.story.ledger_at_birth.realized,
                        )
                    })
                    .collect()
            };
            assert_eq!(
                births(&base),
                births(&alt),
                "{name} cut {q}/4: ledger_at_birth"
            );
            // The ledger as of the cut, for every key either run knows.
            let keys: BTreeSet<String> = base
                .events
                .iter()
                .chain(&alt.events)
                .map(|e| e.story.ledger_key.clone())
                .collect();
            assert_eq!(
                ledger_at(&base, &keys, cut),
                ledger_at(&alt, &keys, cut),
                "{name} cut {q}/4: ledger at the cut"
            );
            // The future really changed.
            assert_ne!(json(&base.events), json(&alt.events), "{name} cut {q}/4");
        }
    }
}

// ---------------------------------------------------------------------------
// History replay (criterion 4)
// ---------------------------------------------------------------------------

#[test]
fn history_replay_reproduces_capsule_history() {
    let inp = small_market(6);
    let (e1, facts) = simulate_engine(&inp, &default_detectors, EngineConfig::r3());
    let (e2, facts2) = simulate_engine(&inp, &default_detectors, EngineConfig::r3());
    assert_eq!(json(&facts), json(&facts2));
    assert_eq!(json(&e1.snapshot()), json(&e2.snapshot()));
    assert!(e1.events().len() > 300);

    // Replay the recorded facts (as an authority's log would) into a fresh engine.
    let mut fresh = Engine::new(EngineConfig::r3());
    let mut by_bar: BTreeMap<(String, u64), Vec<Fact>> = BTreeMap::new();
    for f in &facts {
        let inst = f.meaning.as_ref().unwrap().instrument.clone();
        by_bar.entry((inst, f.bar)).or_default().push(f.clone());
    }
    let mut order: Vec<(i64, &str, &str, usize, usize)> = Vec::new();
    for (k, x) in inp.iter().enumerate() {
        for (i, b) in x.bars.iter().enumerate() {
            order.push((b.t, x.symbol.as_str(), x.series.as_str(), k, i));
        }
    }
    order.sort_unstable();
    for (_, sym, series, k, i) in order {
        let fs = by_bar
            .get(&(sym.to_owned(), i as u64))
            .cloned()
            .unwrap_or_default();
        fresh.on_bar(sym, series, i as u64, &inp[k].bars, &fs);
    }
    assert_eq!(json(&e1.snapshot()), json(&fresh.snapshot()));
    assert_eq!(json(&e1.events()), json(&fresh.events()));
}

// ---------------------------------------------------------------------------
// Sentences from the engine (K26, K33)
// ---------------------------------------------------------------------------

const DAY: i64 = 86_400_000;
const T0: i64 = 1_735_689_600_000;

fn bars_from(closes: &[f64]) -> Vec<Bar> {
    let mut prev = closes[0];
    closes
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let o = prev;
            prev = c;
            Bar {
                t: T0 + i as i64 * DAY,
                o,
                h: o.max(c) + 0.3,
                l: o.min(c) - 0.3,
                c,
                v: 1_000_000.0,
            }
        })
        .collect()
}

/// 60 quiet bars, a 12-bar rally (RSI crosses 70), then a 12-bar slide (RSI falls back
/// well below 70).
fn k26_closes() -> Vec<f64> {
    let mut c = Vec::new();
    for i in 0..60 {
        c.push(100.0 + if i % 2 == 0 { 0.4 } else { -0.4 });
    }
    let mut x = 100.0;
    for _ in 0..12 {
        x += 1.5;
        c.push(x);
    }
    for i in 0..12 {
        x -= if i % 3 == 2 { -0.3 } else { 1.0 };
        c.push(x);
    }
    c
}

fn has_digit(s: &str) -> bool {
    s.bytes().any(|b| b.is_ascii_digit())
}

/// K26 end to end: RSI crossed 70 some bars ago and is lower now → past tense + "artık …
/// değil" / "no longer", with the current RSI computed from the bars.
#[test]
fn k26_rsi_crossed_70_now_lower_is_past_tense_no_longer() {
    let bars = bars_from(&k26_closes());
    let inp = vec![InstrumentInput {
        symbol: "THYAO".to_owned(),
        series: "THYAO.1day".to_owned(),
        bars: bars.clone(),
        flow: None,
    }];
    let (eng, facts) = simulate_engine(&inp, &default_detectors, EngineConfig::r3());
    let enter = facts
        .iter()
        .find(|f| f.kind == "extreme.rsi.overbought")
        .expect("RSI crossed 70");
    let exit = facts
        .iter()
        .find(|f| f.kind == "extreme.rsi.overbought_end")
        .expect("RSI fell back below 70");
    assert!(exit.bar > enter.bar);
    let cap = eng
        .events()
        .iter()
        .find(|e| e.story.kind == MeaningKind::Extreme && e.transition.facts == [enter.id.clone()])
        .map(|e| e.capsule.clone())
        .expect("extreme story");
    let now = (bars.len() - 1) as u64;
    let s = sentences_at(&eng, &Template, &cap, &bars, now, &Reader::unentitled()).unwrap();
    // Current RSI from the bars: the detector's own value would be the same study.
    eprintln!("now:\n  {}\n  {}", s.tr, s.en);
    let rsi_now = s.current.expect("current RSI");
    assert!(rsi_now < 70.0, "{rsi_now}");
    assert_eq!(s.still_holds, Some(false));
    assert!(!s.redacted);
    let ago = now - enter.bar;
    assert!(s.tr.contains(&format!("{ago} bar önce")), "{}", s.tr);
    assert!(s.tr.contains("70'in üstüne çıktı"), "{}", s.tr);
    assert!(s.tr.contains("artık 70'in üstünde değil"), "{}", s.tr);
    assert!(s.tr.contains("şu an"), "{}", s.tr);
    assert!(!s.tr.contains("sürüyor"), "{}", s.tr);
    assert!(s.en.contains(&format!("{ago} bars ago")), "{}", s.en);
    assert!(s.en.contains("no longer above 70"), "{}", s.en);
    assert!(!s.en.contains("still in place"), "{}", s.en);

    // While RSI was still above 70 (the enter bar) the same story is in the present.
    let s0 = sentences_at(&eng, &Template, &cap, &bars, enter.bar, &Reader::entitled()).unwrap();
    eprintln!("enter bar:\n  {}\n  {}", s0.tr, s0.en);
    assert_eq!(s0.still_holds, Some(true));
    assert!(s0.tr.contains("sürüyor"), "{}", s0.tr);
    assert!(s0.en.contains("still in place"), "{}", s0.en);
    // The value it reports then is the detector's own evidence value (same study).
    assert_eq!(s0.current, enter.evidence.get("value").copied());

    // Deterministic, and nothing after `now` is read.
    let again = sentences_at(
        &eng,
        &Template,
        &cap,
        &bars[..=now as usize],
        now,
        &Reader::unentitled(),
    );
    assert_eq!(Some(s.clone()), again);
    let (h, c) = condition_now(
        enter,
        std::slice::from_ref(enter),
        &bars[..=enter.bar as usize],
    );
    assert_eq!((h, c), (Some(true), s0.current));
    // Before the story was born there is no sentence.
    assert!(sentences_at(
        &eng,
        &Template,
        &cap,
        &bars,
        enter.bar - 1,
        &Reader::entitled()
    )
    .is_none());
}

/// K33: a licensed (server) flow fact read by an unentitled reader has no digit at all;
/// an entitled reader sees the numbers.
#[test]
fn licensed_flow_sentence_has_no_digits_for_an_unentitled_reader() {
    let letters = ["AAA", "BBB", "CCC", "DDD", "EEE", "FFF"];
    let inp: Vec<InstrumentInput> = support::synthetic(3, 6, 260)
        .into_iter()
        .zip(letters)
        .map(|(mut x, l)| {
            x.symbol = format!("SYN{l}");
            x.series = format!("SYN{l}.1day");
            x
        })
        .collect();
    let (eng, _) = simulate_engine(&inp, &default_detectors, EngineConfig::r3());
    let mut checked = 0;
    for st in eng.open_stories().iter().chain(eng.closed_stories()) {
        let lead = &st.facts[0];
        if lead.meaning.as_ref().unwrap().detector != "flow.institutional" {
            continue;
        }
        let k = inp
            .iter()
            .position(|x| x.symbol == st.story.instrument)
            .unwrap();
        let now = st.story.last_bar;
        let u = sentences_at(
            &eng,
            &Template,
            &st.id,
            &inp[k].bars,
            now,
            &Reader::unentitled(),
        )
        .unwrap();
        assert!(u.redacted);
        assert!(!has_digit(&u.tr) && !has_digit(&u.en), "{}\n{}", u.tr, u.en);
        assert!(u.tr.contains("kurumsal akış lisansı"), "{}", u.tr);
        assert!(
            u.tr.starts_with(&st.story.instrument),
            "fallback used: {}",
            u.tr
        );
        let e = sentences_at(
            &eng,
            &Template,
            &st.id,
            &inp[k].bars,
            now,
            &Reader {
                entitlements: vec!["flow/*".to_owned()],
            },
        )
        .unwrap();
        assert!(!e.redacted);
        assert!(has_digit(&e.tr), "{}", e.tr);
        checked += 1;
    }
    assert!(checked >= 3, "{checked}");
}

#[test]
fn reader_entitlement_matching() {
    use qp_schema::capsule::{License, LicenseClass};
    let lic = |e: Option<&str>| License {
        class: LicenseClass::Licensed,
        entitlement: e.map(str::to_owned),
    };
    let r = |v: &[&str]| Reader {
        entitlements: v.iter().map(|s| (*s).to_owned()).collect(),
    };
    let public = License {
        class: LicenseClass::Public,
        entitlement: None,
    };
    assert!(Reader::unentitled().may_see(&public, "X"));
    assert!(!Reader::unentitled().may_see(&lic(Some("flow/*")), "X"));
    assert!(Reader::entitled().may_see(&lic(Some("flow/*")), "X"));
    assert!(r(&["flow/*"]).may_see(&lic(Some("flow/*")), "X"));
    assert!(r(&["flow/X"]).may_see(&lic(Some("flow/*")), "X"));
    assert!(!r(&["flow/Y"]).may_see(&lic(Some("flow/*")), "X"));
    assert!(!r(&["depth/X"]).may_see(&lic(Some("flow/*")), "X"));
    assert!(r(&["X"]).may_see(&lic(Some("flow/X")), "X"));
    assert!(!r(&["flow/*"]).may_see(&lic(None), "X"));
}
