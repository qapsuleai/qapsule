//! Tests of the `selvi` voice: a sweep over every story transition of the chosen engine on
//! the synthetic market (daily and hourly, with the licensed flow), for an entitled and an
//! unentitled reader, checking the voice's own rules; and a few exact phrasings.

use qp_facts::FlowPoint;
use qp_schema::capsule::Validity;
use qp_schema::Fact;

use super::{speak, Surfaces, Voice, VoiceInput, MAX_DETAIL, MAX_GLOSS, MAX_HEADLINE, MAX_LEDGER};
use qp_capsule::engine::EngineConfig;
use qp_capsule::pipeline::simulate_engine;
use qp_capsule::realize::elapsed::Period;
use qp_capsule::realize::parts::parts;
use qp_capsule::sentence::{closed_override, condition_now, sentence_lead, validity_now, Reader};
use qp_capsule::{InstrumentInput, Lang, SentenceInput};

fn inputs(hourly: bool) -> Vec<InstrumentInput> {
    let insts = if hourly {
        qp_source::synthetic_market_hourly(3, 60, 2)
    } else {
        qp_source::synthetic_market(6, 300, 1)
    };
    let suffix = if hourly { "60m" } else { "1day" };
    insts
        .into_iter()
        .enumerate()
        .map(|(i, inst)| {
            let flow = qp_source::synthetic_flow(&inst.bars, 17 + i as u64)
                .into_iter()
                .map(|r| FlowPoint {
                    t: r.t,
                    values: r.values,
                })
                .collect();
            InstrumentInput {
                series: format!("{}.{suffix}", inst.symbol),
                symbol: inst.symbol,
                bars: inst.bars,
                flow: Some(flow),
            }
        })
        .collect()
}

/// Every voice of every transition (both readers), with the instrument symbol.
fn sweep(hourly: bool) -> Vec<(String, bool, Validity, Voice)> {
    let inputs = inputs(hourly);
    let period = if hourly {
        Period::Intraday(60)
    } else {
        Period::Daily
    };
    let cal = qp_calendar::ExchangeCalendar::new(&qp_source::synthetic_calendar()).unwrap();
    let (engine, _) = simulate_engine(&inputs, &qp_facts::default_detectors, EngineConfig::r3());
    let mut out = Vec::new();
    for e in engine.events() {
        let rec = engine
            .open_stories()
            .iter()
            .chain(engine.closed_stories())
            .find(|s| s.id == e.capsule)
            .unwrap();
        let input = inputs.iter().find(|i| i.series == e.story.series).unwrap();
        let now = e.transition.bar;
        let now_i = usize::try_from(now).unwrap();
        let bars = &input.bars[..=now_i];
        let facts: Vec<Fact> = rec.facts.iter().filter(|f| f.bar <= now).cloned().collect();
        let lead = facts.first().unwrap().clone();
        let latest = facts.last().unwrap().clone();
        let (sh, cur) = condition_now(&lead, &facts, bars);
        let still = closed_override(&e.story, sh);
        let validity = validity_now(&e.story, still);
        let m = lead.meaning.clone().unwrap();
        for reader in [Reader::entitled(), Reader::unentitled()] {
            let redacted = !reader.may_see(&m.license, &m.instrument);
            let lead_view = sentence_lead(&lead).unwrap_or_else(|| lead.clone());
            let inp = SentenceInput {
                story: &e.story,
                lead: &lead_view,
                latest: if latest.id == lead.id {
                    &lead_view
                } else {
                    &latest
                },
                now_bar: now,
                now_t: bars[now_i].t,
                still_holds: still,
                current: if redacted { None } else { cur },
                redacted,
            };
            let p = parts(&e.story, bars[now_i].t, validity, &cal, period, redacted);
            let v = speak(&VoiceInput {
                sentence: &inp,
                parts: &p,
                calendar: &cal,
                period,
            });
            out.push((e.story.instrument.clone(), redacted, validity, v));
        }
    }
    out
}

fn surfaces(s: &Surfaces) -> Vec<String> {
    let mut v = vec![s.headline.clone(), s.detail.clone(), s.ledger.clone()];
    v.extend(s.glossary.iter().map(|g| format!("{}: {}", g.term, g.text)));
    v
}

fn numbers(s: &str, lang: Lang) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let cs: Vec<char> = s.chars().collect();
    for (i, &c) in cs.iter().enumerate() {
        let sep = match lang {
            Lang::Tr => c == ',' || c == '.',
            Lang::En => c == '.' || c == ',',
        };
        if c.is_ascii_digit()
            || (sep && !cur.is_empty() && cs.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            if c.is_ascii_digit() {
                cur.push(c);
            } else {
                cur.push('|');
            }
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.sort();
    out
}

#[test]
fn sweep_keeps_the_rules() {
    let banned = [
        "teyit",
        "sinyal",
        "dedektör",
        "beklendiği gibi",
        "önemli bir eşi",
        "signal",
        "confirm",
        "family",
        "detector",
        "as expected",
        "key level",
        "should",
    ];
    let banned_words = ["bar", "bars", "aile", "olgu", "al", "sat", "buy", "sell"];
    let mut n = 0;
    for hourly in [false, true] {
        for (inst, redacted, validity, v) in sweep(hourly) {
            n += 1;
            for lang in [Lang::Tr, Lang::En] {
                let s = v.get(lang);
                assert!(!s.headline.is_empty() && !s.detail.is_empty() && !s.ledger.is_empty());
                assert!(s.headline.chars().count() <= MAX_HEADLINE, "{}", s.headline);
                assert!(s.detail.chars().count() <= MAX_DETAIL, "{}", s.detail);
                assert!(s.ledger.chars().count() <= MAX_LEDGER, "{}", s.ledger);
                for g in &s.glossary {
                    assert!(g.text.chars().count() <= MAX_GLOSS, "{}", g.text);
                }
                for t in surfaces(s) {
                    let low = t.to_lowercase();
                    for b in banned {
                        assert!(!low.contains(b), "{b:?} in {t}");
                    }
                    for w in low.split(|c: char| !c.is_alphabetic()) {
                        assert!(!banned_words.contains(&w), "{w:?} in {t}");
                    }
                    if redacted {
                        let stripped = t.replace(&inst, " ");
                        assert!(
                            !stripped.bytes().any(|c| c.is_ascii_digit()),
                            "digit for a redacted reader: {t}"
                        );
                    }
                }
                let h = s.headline.to_lowercase();
                let says = match (validity, lang) {
                    (Validity::Valid, Lang::Tr) => h.contains("hâlâ geçerli"),
                    (Validity::Valid, Lang::En) => h.contains("still holds"),
                    (Validity::NoLonger, Lang::Tr) => h.contains("artık geçerli değil"),
                    (Validity::NoLonger, Lang::En) => h.contains("no longer holds"),
                    (Validity::Closed, _) => !h.contains("geçerli") && !h.contains("holds"),
                };
                assert!(says, "{validity:?}: {}", s.headline);
            }
            // TR and EN carry the same numbers on every surface.
            for (a, b) in [
                (&v.tr.headline, &v.en.headline),
                (&v.tr.detail, &v.en.detail),
                (&v.tr.ledger, &v.en.ledger),
            ] {
                let strip = |s: &str| s.replace(&inst, " ");
                let (na, nb) = (numbers(&strip(a), Lang::Tr), numbers(&strip(b), Lang::En));
                assert_eq!(na.len(), nb.len(), "{a} | {b}");
            }
            assert_eq!(v.tr.glossary.len(), v.en.glossary.len());
        }
    }
    assert!(n > 200, "the sweep covers enough situations ({n})");
}

#[test]
fn deterministic() {
    let a = sweep(false);
    let b = sweep(false);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.3, y.3);
    }
}

#[test]
fn phrasings() {
    let all = sweep(false);
    // A born, still valid breakout up says the line by name, value and period.
    let born_up = all
        .iter()
        .find(|(_, red, val, v)| {
            !red && *val == Validity::Valid && v.tr.headline.contains("en yükseği olan")
        })
        .expect("a born breakout up");
    assert!(
        born_up.3.tr.headline.contains("20 günün"),
        "{}",
        born_up.3.tr.headline
    );
    assert!(
        born_up.3.en.headline.contains("20-day high of"),
        "{}",
        born_up.3.en.headline
    );
    assert!(born_up.3.tr.headline.ends_with("Hâlâ geçerli."));
    // The ledger names the expectation: natural frequency, horizon, direction.
    let counted = all
        .iter()
        .find(|(_, red, _, v)| !red && v.tr.ledger.starts_with("Geçmişte"))
        .expect("a counted ledger");
    assert!(
        counted.3.tr.ledger.contains("işlem günü içinde"),
        "{}",
        counted.3.tr.ledger
    );
    assert!(
        counted.3.en.ledger.contains("past cases"),
        "{}",
        counted.3.en.ledger
    );
    // A redacted reader learns which entitlement opens the figures and who verified it.
    let red = all
        .iter()
        .find(|(_, red, _, _)| *red)
        .expect("a redacted reader");
    assert!(
        red.3.tr.detail.contains("yetkisiyle açılır"),
        "{}",
        red.3.tr.detail
    );
    assert!(
        red.3.en.detail.contains("entitlement"),
        "{}",
        red.3.en.detail
    );
    assert_eq!(red.3.tr.trust, "Sunucu doğruladı");
    // Every screen explains its track-record term.
    assert!(all.iter().all(
        |(_, _, _, v)| v.tr.glossary.iter().any(|g| g.term == "benzer")
            && v.en.glossary.iter().any(|g| g.term == "past cases")
    ));
}
