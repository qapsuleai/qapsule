//! RP2/T5 audit fixes (`rounds/RP2/audits/T5.md`, `rounds/RP2/experiments/T5-fixes.md`):
//! O1 (a capsule made live again is evaluated at once: incremental = from scratch, byte
//! for byte) and the security residual of T4's fix (a finding's invalidation is
//! evaluated only if every principal responsible for it is entitled; a series a
//! condition rests on is not re-pointed at a feed by an unentitled writer).

mod support;

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::Session;
use serde_json::{json, Value};
use support::{code, root, H};

const DAY: i64 = 86_400_000;
const T0: i64 = 1_700_000_000_000;

fn bars(n: usize, c: f64) -> Vec<Value> {
    (0..n)
        .map(|i| json!({"t": T0 + i as i64 * DAY, "o": c, "h": c + 1.0, "l": c - 1.0, "c": c, "v": 1.0}))
        .collect()
}

fn inline_doc() -> SessionInit {
    let inst = |s: &str| json!({"symbol": s});
    serde_json::from_value(json!({
        "document": {
            "schema_version":"0","id":"doc-o1","title":"o1","display_tz":"Europe/Istanbul",
            "series":[
                {"id":"px","instrument":inst("THYAO"),"interval":"1d","source":{"kind":"inline"},
                 "data":{"kind":"bars","bars":bars(30, 100.0)}},
                {"id":"qx","instrument":inst("ASELS"),"interval":"1d","source":{"kind":"inline"},
                 "data":{"kind":"bars","bars":bars(30, 50.0)}}
            ],
            "views":[],"facts":[],"findings":[],"capsules":[],"intents":[],"ledger":{"entries":[]}
        },
        "clock": T0,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"feed","author":{"kind":"engine","name":"feed","version":"0"},"role":"feed"}
        ]
    }))
    .unwrap()
}

fn lines_json(lines: &[OutboundLine]) -> Vec<String> {
    lines
        .iter()
        .map(|l| serde_json::to_string(l).unwrap())
        .collect()
}

/// The incremental session and the from-scratch reference, fed the same inputs.
struct Pair {
    inc: Session,
    full: Session,
}

impl Pair {
    fn new(init: &SessionInit) -> Self {
        let inc = Session::new(init.clone(), &root()).unwrap();
        let mut full = Session::new(init.clone(), &root()).unwrap();
        full.set_incremental(false);
        Self { inc, full }
    }

    fn send(&mut self, conn: &str, msg: &Value) -> Vec<Value> {
        let text = msg.to_string();
        let a = lines_json(&self.inc.handle_raw(conn, &text));
        let b = lines_json(&self.full.handle_raw(conn, &text));
        assert_eq!(a, b, "{conn}: {text}");
        a.iter().map(|l| serde_json::from_str(l).unwrap()).collect()
    }

    fn cmd(&mut self, conn: &str, id: &str, ops: Value) -> Vec<Value> {
        self.send(conn, &json!({"type":"command","id":id,"ops":ops}))
    }

    fn same_state(&self) {
        let d = |s: &Session| serde_json::to_string(s.document()).unwrap();
        let l = |s: &Session| serde_json::to_string(s.log()).unwrap();
        assert_eq!(d(&self.inc), d(&self.full), "document bytes");
        assert_eq!(l(&self.inc), l(&self.full), "log bytes");
    }
}

fn capsule_state(s: &Session, id: &str) -> String {
    let c = s.document().capsules.iter().find(|c| c.id == id).unwrap();
    serde_json::to_value(c.state)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

/// The audit's reproduction of O1: an undo makes an invalidated capsule live again while
/// its invalidation still holds; a tick on an unrelated series follows. Both paths now
/// evaluate the capsule when the undo makes it live (engine entry right after the undo),
/// so the tick writes the same (nothing) in both. Also an undo of the undo of the
/// engine entry and a capsule put back by undoing its deletion.
#[test]
fn o1_capsule_made_live_again_is_evaluated_at_once() {
    let mut p = Pair::new(&inline_doc());
    for (c, who) in [("a", "arkin"), ("f", "feed")] {
        p.send(
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"headless"}}),
        );
    }
    p.send(
        "a",
        &json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}),
    );
    p.cmd(
        "a",
        "c1",
        json!([{"op":"add_fact","id":"f1","kind":"note","series":"qx","bar":10,"text":"x"}]),
    );
    let r = p.cmd(
        "a",
        "c2",
        json!([{"op":"add_finding","id":"fnd","claim":"c","facts":["f1"],"confidence":0.5,
                "invalidation":{"kind":"threshold","series":"qx","op":"gt","value":40.0},"capsule":"h"}]),
    );
    let cap = p.inc.document().capsules[0].id.clone();
    assert_eq!(capsule_state(&p.inc, &cap), "invalidated", "{r:?}");
    assert_eq!(p.inc.log().len(), 3);
    // Undo the engine's invalidation (seq 3): the capsule is live again, its invalidation
    // holds, so the engine invalidates it again right away (seq 5).
    let r = p.send(
        "a",
        &json!({"type":"command","id":"u1","ops":[{"op":"undo","seq":3}]}),
    );
    assert_eq!(p.inc.log().len(), 5, "{r:?}");
    assert_eq!(p.inc.log()[4].author, "qp");
    assert_eq!(capsule_state(&p.inc, &cap), "invalidated");
    // A tick on px (the capsule does not rest on it): no engine entry in either path.
    let r = p.cmd(
        "f",
        "t1",
        json!([{"op":"update_last_bar","series":"px","bar":{"t":T0 + 29 * DAY,"o":100.0,"h":101.0,"l":99.0,"c":100.5,"v":2.0}}]),
    );
    assert_eq!(r.len(), 2, "ack + change: {r:?}");
    p.same_state();
    // Undo the re-invalidation, then undo that undo (the capsule goes live, then back).
    p.send(
        "a",
        &json!({"type":"command","id":"u2","ops":[{"op":"undo","seq":5}]}),
    );
    p.cmd(
        "f",
        "t2",
        json!([{"op":"append_bars","series":"px","bars":[{"t":T0 + 30 * DAY,"o":100.0,"h":101.0,"l":99.0,"c":100.0,"v":2.0}]}]),
    );
    let n = p.inc.log().len() as u64;
    p.send(
        "a",
        &json!({"type":"command","id":"u3","ops":[{"op":"undo","seq":n - 1}]}),
    );
    p.cmd(
        "f",
        "t3",
        json!([{"op":"update_last_bar","series":"px","bar":{"t":T0 + 30 * DAY,"o":100.0,"h":101.0,"l":99.0,"c":99.0,"v":3.0}}]),
    );
    assert_eq!(capsule_state(&p.inc, &cap), "invalidated");
    p.same_state();
    // A threshold that does not hold yet: the capsule stays live; a tick on qx that makes
    // it hold invalidates it in both paths.
    let r = p.cmd(
        "a",
        "c3",
        json!([{"op":"add_finding","id":"fnd2","claim":"c","facts":["f1"],"confidence":0.5,
                "invalidation":{"kind":"threshold","series":"qx","op":"gt","value":60.0},"capsule":"h"}]),
    );
    assert!(r[0]["msg"]["status"] == "applied", "{r:?}");
    let cap2 = p.inc.document().capsules[1].id.clone();
    assert_eq!(capsule_state(&p.inc, &cap2), "live");
    p.cmd(
        "f",
        "t4",
        json!([{"op":"update_last_bar","series":"qx","bar":{"t":T0 + 29 * DAY,"o":50.0,"h":70.0,"l":49.0,"c":65.0,"v":2.0}}]),
    );
    assert_eq!(capsule_state(&p.inc, &cap2), "invalidated");
    p.same_state();
    let replayed = qp_session::replay(p.inc.initial_document(), p.inc.log()).unwrap();
    assert_eq!(
        serde_json::to_string(&replayed).unwrap(),
        serde_json::to_string(p.inc.document()).unwrap()
    );
}

// ---------------------------------------------------------------------------
// Security residual (§18.3)
// ---------------------------------------------------------------------------

fn transcript_init(name: &str) -> SessionInit {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join(format!("fixtures/protocol/conformance/{name}.json")))
            .unwrap(),
    )
    .unwrap();
    serde_json::from_value(t["init"].clone()).unwrap()
}

/// `31-no-entitlement`: arkin entitled (THYAO, ASELS), editor and viewer not.
fn no_entitlement(incremental: bool) -> H {
    let mut h = H::with(transcript_init("31-no-entitlement"));
    h.s.set_incremental(incremental);
    h.hello("a", "arkin", "headless");
    h.hello("e", "editor", "headless");
    h
}

fn inline_series(id: &str, n: usize, c: f64) -> Value {
    json!({"id": id, "source": {"kind":"inline"}, "data": {"kind":"bars","bars": bars(n, c)}})
}

fn bind_thyao(id: &str) -> Value {
    json!({"id": id, "source": {"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":300}}})
}

fn last_close(h: &mut H) -> (i64, f64) {
    let r = h.query("a", json!({"kind":"at","series":"px"}));
    let o = &r["data"]["at"]["ohlcv"];
    (o["t"].as_i64().unwrap(), o["c"].as_f64().unwrap())
}

fn state(h: &H, finding: &str) -> String {
    let c =
        h.s.document()
            .capsules
            .iter()
            .find(|c| c.finding == finding)
            .unwrap();
    serde_json::to_value(c.state)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

/// The owner's finding on an inline series `id` (with a capsule), invalidation `px`-free.
fn owner_finding(h: &mut H, id: &str, value: f64) {
    let r = h.cmd(
        "a",
        &format!("of-{id}"),
        json!([{"op":"put_series","series":inline_series(id, 5, 1.0)},
               {"op":"add_fact","id":format!("f-{id}"),"kind":"note","series":id,"bar":4,"text":"x","evidence":{}},
               {"op":"add_finding","id":format!("fnd-{id}"),"claim":"p","facts":[format!("f-{id}")],"confidence":0.5,
                "invalidation":{"kind":"threshold","series":id,"op":"gt","value":value},"capsule":"p"}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
}

/// Bypass 1 (T4 fix review): the unentitled editor rewrites the invalidation of the
/// owner's finding (`update_finding`, on the owner's inline series: allowed), then the
/// owner rebinds that series to THYAO. The condition is no longer evaluated under the
/// owner alone: whoever last wrote the invalidation (the editor) must be entitled too.
/// The editor's probes (thresholds below and above the close) all leave the capsule
/// live, before and after data events: they learn nothing. Control: the same done by
/// the owner invalidates.
#[test]
fn rewritten_invalidation_is_evaluated_under_its_writer() {
    for incremental in [true, false] {
        let mut outcomes = Vec::new();
        for (k, below) in [true, false, true, false].into_iter().enumerate() {
            let mut h = no_entitlement(incremental);
            let (t, c) = last_close(&mut h);
            owner_finding(&mut h, "own", 1e9);
            let probe = if below { c - 50.0 } else { c + 50.0 };
            let writer = if k < 2 { "e" } else { "a" };
            let r = h.cmd(
                writer,
                "uf",
                json!([{"op":"update_finding","finding":"fnd-own",
                        "invalidation":{"kind":"threshold","series":"own","op":"gt","value":probe}}]),
            );
            assert_eq!(r["status"], "applied", "{r}");
            let r = h.cmd(
                "a",
                "rb",
                json!([{"op":"put_series","series":bind_thyao("own")}]),
            );
            assert_eq!(r["status"], "applied", "{r}");
            let after_rebind = state(&h, "fnd-own");
            let ev: DataEvent =
                serde_json::from_value(json!({"feed":{"symbol":"THYAO","interval":"1d"},
                "seq":1,"change":{"kind":"patch","t":t,"fields":{"c":c + 1.0}}}))
                .unwrap();
            h.s.data_event(&ev).unwrap();
            outcomes.push((writer, below, after_rebind, state(&h, "fnd-own")));
            h.assert_replay();
        }
        // Editor's probes: live whatever the threshold.
        assert_eq!(outcomes[0].2, "live", "{outcomes:?}");
        assert_eq!(outcomes[0].3, "live", "{outcomes:?}");
        assert_eq!(outcomes[1].2, "live", "{outcomes:?}");
        assert_eq!(outcomes[1].3, "live", "{outcomes:?}");
        // Owner's: the condition holds on the data below the close.
        assert_eq!(outcomes[2].2, "invalidated", "{outcomes:?}");
        assert_eq!(outcomes[3].3, "live", "{outcomes:?}");
    }
}

/// Bypass 2: the editor re-points a series a condition rests on at a feed it may not
/// read — directly (`put_series` with a binding) or through a study's inputs — and the
/// owner's capsule would answer. Both are refused (`no_entitlement` at the source); a
/// series no condition reads may still be bound by anyone with the role (the evaluation
/// rule protects it later), and the owner may rebind.
#[test]
fn unentitled_writer_cannot_repoint_a_condition() {
    let mut h = no_entitlement(true);
    owner_finding(&mut h, "own", 1e9);
    let r = h.cmd(
        "e",
        "rb",
        json!([{"op":"put_series","series":bind_thyao("own")}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/series/source", "{r}");
    // Through a study: the owner's finding on a study of an inline series; the editor
    // swaps the study's input for px (bound).
    let r = h.cmd(
        "a",
        "st",
        json!([{"op":"put_series","series":inline_series("base2", 40, 10.0)},
               {"op":"put_series","series":{"id":"st","source":{"kind":"study","study":"sma","inputs":["base2"],"params":{"period":5.0}}}},
               {"op":"add_fact","id":"f-st","kind":"note","series":"base2","bar":4,"text":"x","evidence":{}},
               {"op":"add_finding","id":"fnd-st","claim":"p","facts":["f-st"],"confidence":0.5,
                "invalidation":{"kind":"threshold","series":"st","op":"gt","value":1e9},"capsule":"p"}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.cmd(
        "e",
        "sw",
        json!([{"op":"put_series","series":{"id":"st","source":{"kind":"study","study":"sma","inputs":["px"],"params":{"period":5.0}}}}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/series/source/inputs/0", "{r}");
    // Same through the base series of the study (rests on it).
    let r = h.cmd(
        "e",
        "rb2",
        json!([{"op":"put_series","series":bind_thyao("base2")}]),
    );
    assert_eq!(code(&r), "no_entitlement", "{r}");
    // Unreferenced series: allowed (no condition to answer).
    let r = h.cmd(
        "e",
        "free",
        json!([{"op":"put_series","series":bind_thyao("free")}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    // The owner may.
    let r = h.cmd(
        "a",
        "rb3",
        json!([{"op":"put_series","series":bind_thyao("own")}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
    h.assert_replay();
}

/// Bypass 3: the editor binds a series no condition reads yet (allowed), then the owner
/// writes a finding on it. The owner is entitled, but the binding's writer is not: the
/// invalidation is not evaluated (the editor chose the data the owner's condition
/// reads). Once the owner rewrites the binding, it is.
#[test]
fn binding_written_by_unentitled_principal_is_not_evaluated() {
    for incremental in [true, false] {
        let mut h = no_entitlement(incremental);
        let (_, c) = last_close(&mut h);
        let r = h.cmd(
            "e",
            "b",
            json!([{"op":"put_series","series":bind_thyao("eb")}]),
        );
        assert_eq!(r["status"], "applied", "{r}");
        let r = h.cmd(
            "a",
            "f",
            json!([{"op":"add_fact","id":"f-eb","kind":"note","series":"px","bar":4,"text":"x","evidence":{}},
                   {"op":"add_finding","id":"fnd-eb","claim":"p","facts":["f-eb"],"confidence":0.5,
                    "invalidation":{"kind":"threshold","series":"eb","op":"gt","value":c - 50.0},"capsule":"p"}]),
        );
        assert_eq!(r["status"], "applied", "{r}");
        assert_eq!(state(&h, "fnd-eb"), "live");
        let mut b = bind_thyao("eb");
        b["source"]["window"] = json!({"last": 250});
        let r = h.cmd("a", "rb", json!([{"op":"put_series","series":b}]));
        assert_eq!(r["status"], "applied", "{r}");
        assert_eq!(state(&h, "fnd-eb"), "invalidated");
        h.assert_replay();
    }
}
