//! Marla end to end over the engine (moved out of `marla.rs`).

use super::*;
use crate::engine::EngineConfig;
use crate::pipeline::simulate_engine;
use crate::realize::parts::parts;
use crate::sentence::{closed_override, condition_now, sentence_lead, validity_now, Reader};
use crate::InstrumentInput;
use qp_facts::{default_detectors, FlowPoint};
use qp_schema::Fact;

fn inputs() -> Vec<InstrumentInput> {
    qp_source::synthetic_market(8, 320, 3)
        .into_iter()
        .enumerate()
        .map(|(i, inst)| {
            let flow = qp_source::synthetic_flow(&inst.bars, 11 + i as u64)
                .into_iter()
                .map(|r| FlowPoint {
                    t: r.t,
                    values: r.values,
                })
                .collect();
            InstrumentInput {
                series: format!("{}.1day", inst.symbol),
                symbol: inst.symbol,
                bars: inst.bars,
                flow: Some(flow),
            }
        })
        .collect()
}

const INTERNAL: [&str; 9] = [
    " bar",
    "teyit",
    "aile",
    "sinyal",
    "olgu",
    "dedektör",
    "signal",
    "confirm",
    "detector",
];

#[test]
fn every_event_keeps_the_rules() {
    let ins = inputs();
    let (engine, _) = simulate_engine(&ins, &default_detectors, EngineConfig::r3());
    let cal = qp_calendar::ExchangeCalendar::new(&qp_source::synthetic_calendar()).unwrap();
    let mut seen = 0;
    let mut redacted_seen = 0;
    for e in engine.events() {
        let rec = engine
            .open_stories()
            .iter()
            .chain(engine.closed_stories())
            .find(|s| s.id == e.capsule)
            .unwrap();
        let input = ins.iter().find(|i| i.series == e.story.series).unwrap();
        let now = e.transition.bar;
        let bars = &input.bars[..=usize::try_from(now).unwrap()];
        let facts: Vec<Fact> = rec.facts.iter().filter(|f| f.bar <= now).cloned().collect();
        let lead = &facts[0];
        let latest = facts.last().unwrap();
        let (sh, cur) = condition_now(lead, &facts, bars);
        let sh = closed_override(&e.story, sh);
        let validity = validity_now(&e.story, sh);
        let m = lead.meaning.as_ref().unwrap();
        let lead_view = sentence_lead(lead).unwrap_or_else(|| lead.clone());
        for reader in [Reader::entitled(), Reader::unentitled()] {
            let redacted = !reader.may_see(&m.license, &m.instrument);
            let inp = SentenceInput {
                story: &e.story,
                lead: &lead_view,
                latest: if latest.id == lead.id {
                    &lead_view
                } else {
                    latest
                },
                now_bar: now,
                now_t: bars[bars.len() - 1].t,
                still_holds: sh,
                current: if redacted { None } else { cur },
                redacted,
            };
            let p = parts(&e.story, inp.now_t, validity, &cal, Period::Daily, redacted);
            let both = speak_both(&inp, &p);
            assert_eq!(both, speak_both(&inp, &p), "deterministic");
            let [tr, en] = &both;
            for v in [tr, en] {
                assert!(len(&v.headline) <= MAX_HEADLINE, "{}", v.headline);
                assert!(len(&v.detail) <= MAX_DETAIL, "{}", v.detail);
                assert!(
                    sentence_count(&v.detail) <= MAX_DETAIL_SENTENCES,
                    "{}",
                    v.detail
                );
                assert!(len(&v.ledger) <= MAX_LEDGER, "{}", v.ledger);
                let all = format!(
                    "{} {} {} {}",
                    v.headline,
                    v.detail,
                    v.ledger,
                    v.glossary
                        .iter()
                        .map(|g| format!("{} {}", g.term, g.text))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .to_lowercase();
                for w in INTERNAL {
                    assert!(!all.contains(w), "{w}: {all}");
                }
                if redacted {
                    let digits = all.replace(&e.story.instrument.to_lowercase(), "");
                    assert!(!digits.bytes().any(|b| b.is_ascii_digit()), "{all}");
                }
            }
            assert_eq!(tr.glossary.len(), en.glossary.len());
            for (a, b) in [
                (&tr.headline, &en.headline),
                (&tr.detail, &en.detail),
                (&tr.ledger, &en.ledger),
            ] {
                assert_eq!(digit_runs(a), digit_runs(b), "{a} | {b}");
            }
            seen += 1;
            redacted_seen += usize::from(redacted);
        }
    }
    assert!(seen > 200, "{seen}");
    assert!(redacted_seen > 10, "{redacted_seen}");
}

#[test]
fn examples() {
    // One readable example per surface, pinned (breakout up, entitled, born today).
    let ins = inputs();
    let (engine, _) = simulate_engine(&ins, &default_detectors, EngineConfig::r3());
    let cal = qp_calendar::ExchangeCalendar::new(&qp_source::synthetic_calendar()).unwrap();
    let e = engine
        .events()
        .iter()
        .find(|e| {
            e.story.history.len() == 1
                && e.story.direction == Direction::Up
                && e.story.kind == MeaningKind::Breakout
        })
        .unwrap();
    let rec = engine
        .open_stories()
        .iter()
        .chain(engine.closed_stories())
        .find(|s| s.id == e.capsule)
        .unwrap();
    let input = ins.iter().find(|i| i.series == e.story.series).unwrap();
    let now = e.transition.bar;
    let bars = &input.bars[..=usize::try_from(now).unwrap()];
    let lead = &rec.facts[0];
    let (sh, cur) = condition_now(lead, std::slice::from_ref(lead), bars);
    let validity = validity_now(&e.story, sh);
    let inp = SentenceInput {
        story: &e.story,
        lead,
        latest: lead,
        now_bar: now,
        now_t: bars[bars.len() - 1].t,
        still_holds: sh,
        current: cur,
        redacted: false,
    };
    let p = parts(&e.story, inp.now_t, validity, &cal, Period::Daily, false);
    let tr = speak(&inp, &p, Lang::Tr);
    let en = speak(&inp, &p, Lang::En);
    assert!(
        tr.headline.contains("son 20 günün en yüksek fiyatını ("),
        "{}",
        tr.headline
    );
    assert!(
        tr.headline.ends_with("· bugün, hâlâ geçerli"),
        "{}",
        tr.headline
    );
    assert!(
        en.headline.contains("rose above its 20-day high ("),
        "{}",
        en.headline
    );
    assert!(tr.detail.starts_with("O gün kapanış "), "{}", tr.detail);
    assert!(tr.detail.contains("eşiğin %"), "{}", tr.detail);
    assert!(
        tr.detail
            .contains("sonuç en geç 10 işlem günü içinde belli olacak")
            || tr
                .detail
                .contains("Sonuç en geç 10 işlem günü içinde belli olacak"),
        "{}",
        tr.detail
    );
    assert_eq!(tr.glossary[0].term, "en yüksek fiyat");
    assert_eq!(en.glossary.last().unwrap().term, "similar cases");
}
