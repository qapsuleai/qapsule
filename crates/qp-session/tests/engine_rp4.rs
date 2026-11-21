//! (RP4, `docs/CAPSULE.md` §9.9) The capsule engine's named constant record on the
//! product path: `init.capsules.engine` absent and `"r3"` give the same document; `"rp4"`
//! runs the `rp4` outcome rule and detector set (on daily bars: the same stories as `r3`,
//! the two seasonal detectors under version 2 in the ledger key); an unknown name is
//! refused at session start.

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

const BARS_SEED: u64 = 7;
const K: usize = 600;
const UPTO: usize = 700;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

fn init(engine: Option<&str>) -> SessionInit {
    let mut v = json!({
        "document": {
            "schema_version": "0", "id": "doc-rp4", "title": "Sabit kaydı",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": K, "seed": BARS_SEED}}],
            "entitlements": {"host": ["*"]}
        },
        "capsules": {"series": ["px"]}
    });
    if let Some(e) = engine {
        v["capsules"]["engine"] = json!(e);
    }
    serde_json::from_value(v).unwrap()
}

fn run(engine: Option<&str>) -> Session {
    let mut s = Session::new(init(engine), &root()).unwrap();
    let bars = qp_source::synthetic_bars(UPTO, BARS_SEED);
    for (i, b) in bars.iter().enumerate().skip(K) {
        let ev: DataEvent = serde_json::from_value(
            json!({"feed": feed(), "seq": i - K + 1, "change": {"kind":"bar","bar": b}}),
        )
        .unwrap();
        s.data_event(&ev).unwrap();
    }
    s
}

fn doc(s: &Session) -> Value {
    serde_json::to_value(s.document()).unwrap()
}

#[test]
fn absent_and_r3_are_the_same() {
    let a = run(None);
    let b = run(Some("r3"));
    assert!(!a.document().capsules.is_empty());
    assert_eq!(doc(&a).to_string(), doc(&b).to_string());
}

#[test]
fn rp4_on_daily_bars_keeps_the_stories() {
    let a = run(None);
    let b = run(Some("rp4"));
    let (da, db) = (a.document(), b.document());
    // Same stories (id, state, stage, born / last bar); the rule and the ledger key of
    // the two seasonal detectors differ.
    assert_eq!(da.capsules.len(), db.capsules.len());
    let mut v2 = 0;
    for (x, y) in da.capsules.iter().zip(&db.capsules) {
        assert_eq!(x.id, y.id);
        assert_eq!(x.state, y.state);
        let (sx, sy) = (x.story.as_ref().unwrap(), y.story.as_ref().unwrap());
        assert_eq!(
            (sx.stage, sx.born_bar, sx.last_bar),
            (sy.stage, sy.born_bar, sy.last_bar)
        );
        assert_eq!(sx.resolution.as_ref().unwrap().rule, "barrier");
        assert_eq!(sy.resolution.as_ref().unwrap().rule, "rp4");
        if sx.ledger_key.ends_with("breakout.range@1")
            || sx.ledger_key.ends_with("flow.volume_surge@1")
        {
            v2 += 1;
            assert_eq!(sy.ledger_key, sx.ledger_key.replace("@1", "@2"));
        } else {
            assert_eq!(sx.ledger_key, sy.ledger_key);
        }
    }
    assert!(v2 > 0, "a seasonal detector's story");
    // Ledger rows: same outcomes, keys as above.
    assert!(!da.ledger.entries.is_empty());
    assert_eq!(da.ledger.entries.len(), db.ledger.entries.len());
    for (x, y) in da.ledger.entries.iter().zip(&db.ledger.entries) {
        assert_eq!(x.outcome, y.outcome);
    }
    // A reader finds the record from the stories.
    assert_eq!(qp_session::capsule::engine_of_doc(da), "r3");
    assert_eq!(qp_session::capsule::engine_of_doc(db), "rp4");
}

#[test]
fn unknown_record_is_refused() {
    let Err(e) = Session::new(init(Some("r9")), &root()) else {
        panic!("an unknown record must be refused");
    };
    assert!(
        e.contains("init.capsules.engine") && e.contains("rp4"),
        "{e}"
    );
}
