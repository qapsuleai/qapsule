//! R6/T3 (`docs/PROTOCOL.md` §24; VERIFY 11): the shell core shows the market status.
//!
//! - Without a status: the status JSON has no `market`, the digests are as before.
//! - A closed market (recorded/fake status, given as the authority's `viewer` event or as
//!   the host's `{"market":…}` frame): the status line says "Seans kapalı" / "Session
//!   closed"; the feed rows say it too; no face shows the live mark.
//! - Stale data: "Bağlantı yok — veri bayat · son veri HH:MM"; no capsule shows as live.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use std::collections::BTreeMap;

use qp_schema::market::{MarketSource, MarketStatus, PhaseCode};
use qp_wasm::drive::{parity_host, pump_viewer};

/// The connection id `pump_viewer` drives (`qp_wasm::drive`).
const CONN: &str = "w";
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

fn status(open: Option<bool>, stale: bool) -> MarketStatus {
    MarketStatus {
        // 2026-11-19 15:10 UTC = 18:10 Istanbul.
        exchange_time: 1_795_101_000_000,
        open,
        holiday: None,
        source: MarketSource::Calendar,
        delayed: true,
        stale,
        phases: BTreeMap::new(),
    }
}

/// A viewer core on the parity host, its feed shown.
fn viewer(
    lang: &str,
) -> (
    ShellCore,
    qp_session::ReplayHost,
    Vec<qp_schema::data::FeedKey>,
) {
    let mut host = parity_host();
    host.set_record_events(true);
    let mut core = ShellCore::new_viewer("arkin", lang, &[]).expect("core");
    let mut served = Vec::new();
    pump_viewer(&mut core, &mut host, &mut served);
    (core, host, served)
}

fn texts(core: &mut ShellCore) -> Vec<String> {
    core.feed_json()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["text"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn without_status_nothing_changes() {
    let (core, _, _) = viewer("tr");
    assert!(core.status_json().get("market").is_none());
    assert_eq!(core.market_json(), Value::Null);
    assert!(core.feed().iter().all(|i| core.market_live(&i.instrument)));
}

#[test]
fn closed_market_from_the_authority_says_session_closed() {
    let (mut core, mut host, _) = viewer("tr");
    assert!(!core.feed().is_empty());
    let lines = host.set_market_status(status(Some(false), false));
    let mut ch = json!({});
    for l in lines.iter().filter(|l| l.conn == CONN) {
        ch = core.on_message(&l.msg);
    }
    assert_eq!(ch["status"], true, "{ch}");
    assert_eq!(
        core.status_json()["market"],
        json!({"text": "Seans kapalı", "state": "closed"})
    );
    for t in texts(&mut core) {
        assert!(t.contains("eans kapalı"), "{t}");
    }
    // No capsule shows as live while the market is closed.
    assert!(core.feed().iter().all(|i| !core.market_live(&i.instrument)));
}

#[test]
fn market_frame_and_stale_data() {
    let (mut core, _, _) = viewer("en");
    let fr = json!({"market": status(Some(true), true)}).to_string();
    let ch = core.on_frame(&fr).expect("frame");
    assert_eq!(ch["status"], true);
    assert_eq!(
        core.market_json(),
        json!({"text": "No connection — data stale · last data 18:10", "state": "stale"})
    );
    assert!(core.feed().iter().all(|i| !core.market_live(&i.instrument)));
    // Open again, delayed, the instrument in continuous trading: live.
    let mut open = status(Some(true), false);
    let sym = core.feed()[0].instrument.clone();
    open.phases.insert(
        sym.clone(),
        PhaseCode {
            code: 2,
            key: "continuous".to_owned(),
        },
    );
    core.on_frame(&json!({ "market": open }).to_string())
        .expect("frame");
    assert_eq!(core.market_json()["text"], "Live · delayed 15 min");
    assert!(core.market_live(&sym));
    // The same frame again changes nothing.
    let ch = core
        .on_frame(&json!({ "market": open }).to_string())
        .unwrap();
    assert_eq!(ch, json!({}));
}

/// (RP4/T7, A48) The closing auction (18:00–18:10): the calendar says the continuous
/// session is over, the price cards put the instruments in the closing auction; the
/// status line says "Kapanış seansında" / "Closing auction", not "closed".
#[test]
fn closing_auction_is_not_closed() {
    for (lang, want) in [("tr", "Kapanış seansında"), ("en", "Closing auction")] {
        let (mut core, _, _) = viewer(lang);
        let mut st = status(Some(false), false);
        for i in core.feed() {
            st.phases.insert(
                i.instrument.clone(),
                PhaseCode {
                    code: 5,
                    key: "closing_session".to_owned(),
                },
            );
        }
        assert!(!st.phases.is_empty());
        core.on_frame(&json!({ "market": st }).to_string())
            .expect("frame");
        assert_eq!(core.market_json(), json!({"text": want, "state": "closed"}));
        // No capsule shows as live in the auction.
        assert!(core.feed().iter().all(|i| !core.market_live(&i.instrument)));
        // The auction over (the cards say closed): the line says closed again.
        for p in st.phases.values_mut() {
            *p = PhaseCode {
                code: 1,
                key: "closed".to_owned(),
            };
        }
        core.on_frame(&json!({ "market": st }).to_string())
            .expect("frame");
        assert_eq!(core.market_json()["state"], "closed");
        assert_ne!(core.market_json()["text"], want);
    }
}
