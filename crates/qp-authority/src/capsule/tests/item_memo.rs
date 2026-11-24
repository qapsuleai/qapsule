//! Tests of the reader item memo (`qp_plane::capidx::ItemMemo`) through the session (RP4/T6).

use std::collections::BTreeMap;

use qp_schema::market::{MarketSource, MarketStatus, PhaseCode};
use qp_schema::protocol::{OutboundLine, ReplayInit, ReplaySourceKind};
use serde_json::json;

use crate::{demo_init, ReplayHost};

/// The demo world (synthetic, as `qp demo`), small; `visitor` reads without data.
fn host() -> ReplayHost {
    let mut ent = BTreeMap::new();
    for p in ["host", "arkin"] {
        ent.insert(p.to_owned(), vec!["*".to_owned()]);
    }
    ent.insert("visitor".to_owned(), Vec::new());
    let mut init = demo_init(ReplayInit {
        source: ReplaySourceKind::Synthetic,
        instruments: Some(8),
        days: Some(260),
        history: Some(120),
        fine: Some("1h".to_owned()),
        entitlements: ent,
        ..ReplayInit::default()
    });
    init.principals.push(
        serde_json::from_value(json!(
            {"id": "visitor", "author": {"kind": "human", "name": "visitor"}, "role": "owner"}))
        .expect("visitor"),
    );
    let none = std::env::temp_dir().join("qp-t6-no-local-data");
    ReplayHost::with_local_dir(
        init,
        std::path::Path::new("."),
        &none,
        crate::Subsystems::NONE,
    )
    .expect("demo host")
}

fn text(lines: &[OutboundLine]) -> String {
    serde_json::to_string(lines).expect("json")
}

fn send(h: &mut ReplayHost, conn: &str, m: &serde_json::Value) -> String {
    text(&h.handle_raw(conn, &m.to_string()))
}

fn market(t: i64, open: bool, phase: Option<(&str, i32)>) -> MarketStatus {
    MarketStatus {
        exchange_time: t,
        open: Some(open),
        holiday: Some(false),
        source: MarketSource::Calendar,
        delayed: false,
        stale: false,
        phases: phase
            .map(|(s, c)| {
                (
                    s.to_owned(),
                    PhaseCode {
                        code: c,
                        key: format!("p{c}"),
                    },
                )
            })
            .into_iter()
            .collect(),
    }
}

#[allow(clippy::too_many_lines)]
/// (RP4/T6) Incremental = from scratch: two identical hosts take the same steps,
/// messages and market statuses in a fixed order; one keeps its item memo, the other
/// forgets it before every input. Every line both send (viewer events, `capsules`
/// answers, notifications) is byte-equal, over readers with and without a voice, a
/// clock, both languages and a reader without data (projection: no memo).
#[test]
fn incremental_items_equal_from_scratch() {
    let mut a = host();
    let mut b = host();
    let setup = [
        (
            "h",
            json!({"type":"hello","protocol":"qp/0","principal":"host","surface":{"kind":"phone","lang":"tr"}}),
        ),
        (
            "h",
            json!({"type":"input","id":"s1","gesture":{"kind":"style","voice":"marla","clock":true}}),
        ),
        (
            "h",
            json!({"type":"subscribe","id":"v","topic":{"kind":"viewer","limit":40}}),
        ),
        (
            "e",
            json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone","lang":"en"}}),
        ),
        (
            "e",
            json!({"type":"subscribe","id":"v","topic":{"kind":"viewer"}}),
        ),
        (
            "x",
            json!({"type":"hello","protocol":"qp/0","principal":"visitor","surface":{"kind":"phone","lang":"tr"}}),
        ),
        (
            "x",
            json!({"type":"subscribe","id":"v","topic":{"kind":"viewer"}}),
        ),
    ];
    for (c, m) in &setup {
        assert_eq!(send(&mut a, c, m), send(&mut b, c, m), "setup {m}");
    }
    let query = json!({"type":"query","id":"q","query":{"kind":"capsules"}});
    let mut steps = 0;
    for i in 0..160 {
        // Inputs other than data, at fixed steps.
        let extra: Option<(&str, serde_json::Value)> = match i {
            20 => Some((
                "h",
                json!({"type":"input","id":"s2","gesture":{"kind":"style","voice":"selvi"}}),
            )),
            45 => Some((
                "e",
                json!({"type":"input","id":"s3","gesture":{"kind":"style","voice":"marla","clock":true}}),
            )),
            70 => Some((
                "h",
                json!({"type":"input","id":"s4","gesture":{"kind":"style","voice":"r4","clock":false}}),
            )),
            100 => Some((
                "h",
                json!({"type":"input","id":"s5","gesture":{"kind":"style","voice":"marla"}}),
            )),
            _ => None,
        };
        if let Some((c, m)) = extra {
            b.session().forget_item_memo();
            assert_eq!(send(&mut a, c, &m), send(&mut b, c, &m), "step {i}: {m}");
        }
        if i % 30 == 10 {
            let t = a.session().clock();
            let m = market(t, i % 60 == 10, (i % 60 == 40).then_some(("S1", 3)));
            b.session().forget_item_memo();
            assert_eq!(
                text(&a.set_market_status(m.clone())),
                text(&b.set_market_status(m)),
                "step {i}: market"
            );
        }
        b.session().forget_item_memo();
        let (sa, sb) = (a.take_step(), b.take_step());
        let (Some((_, la)), Some((_, lb))) = (sa, sb) else {
            break;
        };
        steps += 1;
        assert_eq!(text(&la), text(&lb), "step {i}");
        if i % 7 == 3 {
            for c in ["h", "e", "x"] {
                b.session().forget_item_memo();
                assert_eq!(
                    send(&mut a, c, &query),
                    send(&mut b, c, &query),
                    "step {i}: {c}"
                );
            }
        }
    }
    assert!(steps > 100, "the replay ran ({steps} steps)");
    let m = a.session().item_memo.borrow();
    // Every step moves every instrument's bars here (all 8 step together), so most
    // lookups miss; the reuse is between steps' messages and the queries.
    assert!(
        m.hits > 1000 && m.misses > 0,
        "the memo was used: {} / {}",
        m.hits,
        m.misses
    );
}
