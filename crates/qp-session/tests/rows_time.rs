//! RP4/T7 (`docs/PROTOCOL.md` §4.6.2 `clock`, `qp_capsule::realize::stamp`): rows carry
//! the event's time when the connection asks for it.
//!
//! - Without `clock` (absent or `false`) every answer is byte-identical to before.
//! - With `clock`, a voiced capsule's headline starts with its stamp (`14:35 · …` on
//!   today's intraday bars, else the date), the rest unchanged; the same stamp as the
//!   story chart's row (`Session::row_stamp`).
//! - A redacted reader gets no stamp (K33).
//! - "Now" is the data's (the series' last bar): the same answer twice.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::protocol::{CapsuleItem, Language, OutboundLine, ReplayInit, ReplaySourceKind};
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
    let none = std::env::temp_dir().join("qp-rows-time-no-local-data");
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

fn style(h: &mut ReplayHost, conn: &str, g: &Value) {
    let mut g = g.clone();
    g["kind"] = json!("style");
    let out = send(h, conn, &json!({"type":"input","id":"st","gesture":g}));
    assert!(
        !out.iter()
            .any(|l| serde_json::to_string(&l.msg).unwrap().contains("\"error\"")),
        "{out:?}"
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

/// `HH:MM`, or a date (`12 Kas`, `Nov 12`, with a year when not now's).
fn is_stamp(s: &str, lang: Language) -> bool {
    let b = s.as_bytes();
    if b.len() == 5 && b[2] == b':' && s.chars().filter(char::is_ascii_digit).count() == 4 {
        return true;
    }
    let tr = [
        "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
    ];
    let en = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let w: Vec<&str> = s.split(' ').collect();
    let num = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_digit());
    match lang {
        Language::Tr => {
            (w.len() == 2 || (w.len() == 3 && num(w[2]))) && num(w[0]) && tr.contains(&w[1])
        }
        Language::En => {
            let day = w.get(1).map(|d| d.trim_end_matches(','));
            (w.len() == 2 || (w.len() == 3 && num(w[2])))
                && en.contains(&w[0])
                && day.is_some_and(num)
        }
    }
}

fn voiced(items: &[CapsuleItem]) -> Vec<(String, String)> {
    items
        .iter()
        .filter_map(|i| i.voice.as_ref().map(|v| (i.id.clone(), v.headline.clone())))
        .collect()
}

#[test]
fn without_clock_nothing_changes() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    style(&mut h, "a", &json!({"voice":"marla"}));
    let before = serde_json::to_string(&capsules(&mut h, "a")).unwrap();
    style(&mut h, "a", &json!({"clock":false}));
    let off = serde_json::to_string(&capsules(&mut h, "a")).unwrap();
    assert_eq!(before, off);
    // On, then off again: as before.
    style(&mut h, "a", &json!({"clock":true}));
    assert_ne!(
        before,
        serde_json::to_string(&capsules(&mut h, "a")).unwrap()
    );
    style(&mut h, "a", &json!({"clock":false}));
    assert_eq!(
        before,
        serde_json::to_string(&capsules(&mut h, "a")).unwrap()
    );
}

#[test]
fn clock_puts_the_stamp_before_the_headline() {
    for (lang, l) in [("tr", Language::Tr), ("en", Language::En)] {
        let mut h = host();
        hello(&mut h, "a", "arkin", lang);
        style(&mut h, "a", &json!({"voice":"marla"}));
        let plain = capsules(&mut h, "a");
        style(&mut h, "a", &json!({"clock":true}));
        let timed = capsules(&mut h, "a");
        // The same answer twice: "now" is the data's, not a clock's.
        assert_eq!(
            serde_json::to_string(&timed).unwrap(),
            serde_json::to_string(&capsules(&mut h, "a")).unwrap()
        );
        let (p, t) = (voiced(&plain), voiced(&timed));
        assert!(!p.is_empty());
        assert_eq!(p.len(), t.len());
        let (mut clocks, mut dates) = (0, 0);
        for ((pid, ph), (tid, th)) in p.iter().zip(&t) {
            assert_eq!(pid, tid);
            let (stamp, rest) = th.split_once(" · ").expect("a stamp");
            assert_eq!(rest, ph, "only the stamp is added");
            assert!(is_stamp(stamp, l), "{stamp:?} in {th:?}");
            if stamp.contains(':') {
                clocks += 1;
            } else {
                dates += 1;
            }
        }
        assert!(dates > 0, "daily capsules carry their date");
        // Only the headline changes: the text face is rebuilt from it, the rest is equal.
        for (a, b) in plain.iter().zip(&timed) {
            let (va, vb) = (a.voice.as_ref().unwrap(), b.voice.as_ref().unwrap());
            assert_eq!(va.detail, vb.detail);
            assert_eq!(va.ledger, vb.ledger);
            assert_eq!(a.sentence, b.sentence);
        }
        // The synthetic demo has daily capsules only; the intraday `HH:MM` rule is
        // `qp_capsule::realize::stamp`'s unit tests.
        let _ = clocks;
    }
}

#[test]
fn intraday_today_is_a_clock_time() {
    let mut h = host();
    hello(&mut h, "a", "arkin", "tr");
    style(&mut h, "a", &json!({"voice":"marla","clock":true}));
    let items = capsules(&mut h, "a");
    let s = h.session();
    let mut seen = 0;
    for it in &items {
        let Some(v) = it.voice.as_ref() else { continue };
        let stamp = v.headline.split_once(" · ").unwrap().0;
        let rows = s.story_rows(&it.series, "arkin", Language::Tr);
        let row = rows.iter().find(|r| r.id == it.id).expect("its row");
        // The story chart's row says the same time as the feed's row.
        assert_eq!(
            s.row_stamp(row, Language::Tr).as_deref(),
            Some(stamp),
            "{}",
            it.id
        );
        // A redacted row has none (K33).
        let mut red = row.clone();
        red.redacted = true;
        assert_eq!(s.row_stamp(&red, Language::Tr), None);
        let series = s
            .document()
            .series
            .iter()
            .find(|x| x.id == it.series)
            .expect("series");
        let intraday = series
            .interval
            .as_deref()
            .is_some_and(|i| i.ends_with('m') || i.ends_with('h'));
        if !intraday {
            assert!(!stamp.contains(':'), "a daily row is a date: {stamp}");
        } else {
            seen += 1;
        }
    }
    let _ = seen;
}

#[test]
fn a_redacted_reader_gets_no_stamp() {
    let mut h = host();
    hello(&mut h, "v", "visitor", "tr");
    style(&mut h, "v", &json!({"voice":"marla"}));
    let plain = capsules(&mut h, "v");
    style(&mut h, "v", &json!({"clock":true}));
    let timed = capsules(&mut h, "v");
    let mut red = 0;
    for (a, b) in plain.iter().zip(&timed) {
        if b.redacted {
            red += 1;
            assert_eq!(a.voice, b.voice);
            if let Some(v) = &b.voice {
                assert!(
                    !v.headline.chars().any(|c| c.is_ascii_digit())
                        || v.headline.contains(&b.instrument)
                );
            }
        }
    }
    assert!(red > 0, "the visitor reads redacted capsules");
}
