//! R6/T3 (`docs/PROTOCOL.md` §24, `docs/LANGUAGE.md` "Piyasa durumu"; VERIFY 11): the
//! host's market status as an input of the authority.
//!
//! - Without a status every answer is as before; a status that says "continuous trading,
//!   fresh data" for every instrument changes no capsule either.
//! - A closed market: the `viewer` event carries the status, every capsule says "seans
//!   kapalı" (R4 sentence and the voice's headline/detail; TR and EN).
//! - A halted instrument / closing auction: only that instrument's capsules say so.
//! - Stale data: no capsule speaks as live; it says the data is stale and since when (no
//!   digits for a redacted reader).
//! - The status survives a backward seek (a new authority).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::market::{MarketSource, MarketStatus, PhaseCode};
use qp_schema::protocol::{CapsuleItem, OutboundLine, ReplayInit, ReplaySourceKind};
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

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
    let none = std::env::temp_dir().join("qp-market-no-local-data");
    ReplayHost::with_local_dir(init, &root(), &none).expect("demo host")
}

fn send(h: &mut ReplayHost, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    h.handle_raw(conn, &msg.to_string())
}

fn hello(h: &mut ReplayHost, conn: &str, who: &str, lang: &str) {
    send(
        h,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,
                "surface":{"kind":"phone","lang":lang}}),
    );
}

fn voice(h: &mut ReplayHost, conn: &str, v: &str) {
    send(
        h,
        conn,
        &json!({"type":"input","id":"st","gesture":{"kind":"style","voice":v}}),
    );
}

fn capsules(h: &mut ReplayHost, conn: &str) -> Vec<CapsuleItem> {
    let out = send(
        h,
        conn,
        &json!({"type":"query","id":"q","query":{"kind":"capsules"}}),
    );
    let m = out
        .iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .find(|m| m["id"] == "q")
        .expect("answer");
    serde_json::from_value(m["data"]["capsules"].clone()).unwrap_or_else(|e| panic!("{e}: {m}"))
}

fn status(open: Option<bool>, stale: bool, phases: &[(&str, i32, &str)]) -> MarketStatus {
    MarketStatus {
        // 2026-11-19 15:10 UTC = 18:10 Istanbul.
        exchange_time: 1_795_101_000_000,
        open,
        holiday: None,
        source: MarketSource::Calendar,
        delayed: true,
        stale,
        phases: phases
            .iter()
            .map(|(s, c, k)| {
                (
                    (*s).to_owned(),
                    PhaseCode {
                        code: *c,
                        key: (*k).to_owned(),
                    },
                )
            })
            .collect(),
    }
}

fn symbols(items: &[CapsuleItem]) -> Vec<String> {
    let mut s: Vec<String> = items.iter().map(|i| i.instrument.clone()).collect();
    s.sort();
    s.dedup();
    s
}

#[test]
fn continuous_fresh_status_changes_no_capsule() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    voice(&mut h, "a", "marla");
    let before = capsules(&mut h, "a");
    assert!(!before.is_empty());
    let all: Vec<(&str, i32, &str)> = symbols(&before)
        .iter()
        .map(|s| {
            (
                Box::leak(s.clone().into_boxed_str()) as &str,
                2,
                "continuous",
            )
        })
        .collect();
    let lines = h.set_market_status(status(Some(true), false, &all));
    assert!(lines.is_empty(), "no viewer, nothing to tell: {lines:?}");
    let after = capsules(&mut h, "a");
    assert_eq!(
        serde_json::to_string(&before).unwrap(),
        serde_json::to_string(&after).unwrap()
    );
}

#[test]
fn closed_market_is_said_in_the_viewer_feed_verify_11() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    voice(&mut h, "a", "marla");
    let out = send(
        &mut h,
        "a",
        &json!({"type":"subscribe","id":"viewer","topic":{"kind":"viewer","limit":40}}),
    );
    // Without a status: no `market` anywhere.
    assert!(!serde_json::to_string(&out).unwrap().contains("\"market\""));
    let lines = h.set_market_status(status(Some(false), false, &[]));
    assert_eq!(lines.len(), 1, "{lines:?}");
    let ev = serde_json::to_value(&lines[0].msg).unwrap();
    let v = &ev["event"]["viewer"];
    assert_eq!(v["market"]["open"], false, "{v}");
    let feed: Vec<qp_schema::protocol::ViewerItem> =
        serde_json::from_value(v["feed"].clone()).unwrap();
    assert!(!feed.is_empty());
    for it in feed.iter().map(|r| &r.capsule) {
        assert!(
            it.sentence.tr.contains("Seans kapalı"),
            "{}",
            it.sentence.tr
        );
        assert!(
            it.sentence.en.contains("The session is closed"),
            "{}",
            it.sentence.en
        );
        let vo = it.voice.as_ref().expect("marla");
        assert!(vo.headline.ends_with(" · seans kapalı"), "{}", vo.headline);
        assert!(
            vo.detail.ends_with("Seans kapalı; son seansın verisiyle."),
            "{}",
            vo.detail
        );
        assert!(vo.face.contains("seans kapalı"));
    }
    // The same status again: nothing new.
    assert!(h
        .set_market_status(status(Some(false), false, &[]))
        .is_empty());
    // The status line the shell shows (core text).
    let (line, st) = qp_capsule::market::status_line(
        h.session().market_status().unwrap(),
        qp_schema::protocol::Language::Tr,
    );
    assert_eq!((line.as_str(), st.key()), ("Seans kapalı", "closed"));
}

#[test]
fn phases_speak_only_for_their_instrument() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "en");
    let items = capsules(&mut h, "a");
    let syms = symbols(&items);
    assert!(syms.len() >= 2, "{syms:?}");
    let (halted, auction) = (syms[0].as_str(), syms[1].as_str());
    h.set_market_status(status(
        Some(true),
        false,
        &[(halted, 10, "suspend"), (auction, 5, "closing_session")],
    ));
    for it in capsules(&mut h, "a") {
        let t = &it.sentence.text;
        if it.instrument == halted {
            assert!(
                t.ends_with("Trading is halted; the price is as of the halt."),
                "{t}"
            );
        } else if it.instrument == auction {
            assert!(
                t.ends_with("In the closing auction; the closing price is being set."),
                "{t}"
            );
        } else {
            let plain = items.iter().find(|i| i.id == it.id).unwrap();
            assert_eq!(&plain.sentence, &it.sentence);
        }
    }
}

#[test]
fn stale_data_is_not_shown_as_live() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    hello(&mut h, "v", "visitor", "tr");
    voice(&mut h, "a", "marla");
    voice(&mut h, "v", "marla");
    h.set_market_status(status(Some(true), true, &[]));
    for it in capsules(&mut h, "a") {
        let vo = it.voice.as_ref().unwrap();
        assert!(vo.headline.ends_with(" · veri bayat"), "{}", vo.headline);
        assert!(
            vo.detail
                .ends_with("Bağlantı yok, veri bayat (son veri 18:10); yazılanlar o ana göre."),
            "{}",
            vo.detail
        );
    }
    // A redacted reader: the same, without digits.
    let v = capsules(&mut h, "v");
    for it in v.iter().filter(|i| i.redacted) {
        let vo = it.voice.as_ref().unwrap();
        assert!(vo
            .detail
            .ends_with("Bağlantı yok, veri bayat; yazılanlar son veriye göre."));
    }
    let (line, st) = qp_capsule::market::status_line(
        h.session().market_status().unwrap(),
        qp_schema::protocol::Language::Tr,
    );
    assert_eq!(line, "Bağlantı yok — veri bayat · son veri 18:10");
    assert_eq!(st.key(), "stale");
}

#[test]
fn status_survives_a_backward_seek() {
    let mut h = host();
    h.set_market_status(status(Some(false), false, &[]));
    let fwd = json!({"type":"replay","id":"r","control":{"kind":"step","n":3}});
    hello(&mut h, "a", "arkin", "tr");
    send(&mut h, "a", &fwd);
    let back = json!({"type":"replay","id":"r2","control":{"kind":"seek","step":1}});
    send(&mut h, "a", &back);
    assert_eq!(h.session().market_status().unwrap().open, Some(false));
    hello(&mut h, "b", "arkin", "tr");
    for it in capsules(&mut h, "b") {
        assert!(it.sentence.text.contains("Seans kapalı"));
    }
}

/// (Security review, §20.5) The status is projected per reader: an unentitled reader gets
/// no other instrument's phase (only open/closed and stale), its capsules do not speak of
/// the phase, and nobody gets the feed's numeric codes.
#[test]
fn unentitled_reader_gets_no_phases() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    hello(&mut h, "v", "visitor", "tr");
    let syms = symbols(&capsules(&mut h, "a"));
    let sym = syms[0].clone();
    for c in ["a", "v"] {
        send(
            &mut h,
            c,
            &json!({"type":"subscribe","id":"viewer","topic":{"kind":"viewer","limit":40}}),
        );
    }
    let lines = h.set_market_status(status(Some(true), false, &[(&sym, 10, "suspend")]));
    let market_of = |conn: &str| -> Value {
        let l = lines.iter().find(|l| l.conn == conn).expect("viewer event");
        serde_json::to_value(&l.msg).unwrap()["event"]["viewer"]["market"].clone()
    };
    let a = market_of("a");
    assert_eq!(a["phases"][&sym], json!({"key": "suspend"}), "{a}");
    let v = market_of("v");
    assert!(v.get("phases").is_none(), "{v}");
    assert_eq!(v["open"], true);
    assert!(!serde_json::to_string(&lines).unwrap().contains("\"code\""));
    for it in capsules(&mut h, "v") {
        assert!(
            !it.sentence.text.contains("durduruldu"),
            "{}",
            it.sentence.text
        );
    }
    for it in capsules(&mut h, "a").iter().filter(|i| i.instrument == sym) {
        assert!(
            it.sentence.text.contains("durduruldu"),
            "{}",
            it.sentence.text
        );
    }
}
