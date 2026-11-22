//! Disputes and the authority's settled-bar hygiene (R3/M7, `docs/PROTOCOL.md` §19.9):
//! a wrong final claim the authority made from its own feed is withdrawn (and a capsule
//! resting on it retracted) when the provider's settled bars contradict it, either by
//! the authority's own verification of closed bars or by a replica's dispute; a claim the
//! feed missed is asserted from settled data; the replica disputes once.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_schema::Bar;
use qp_session::{Replica, ReplicaOptions, Session};
use serde_json::{json, Value};

const T0: i64 = 1_759_708_800_000;
const DAY: i64 = 86_400_000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn transcript() -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/37-dispute-retract.json"),
        )
        .unwrap(),
    )
    .unwrap()
}

/// The init of transcript 37; `settled`: its test settled source (or none), with
/// `verify_closed` set as given.
fn init(settled: bool, verify_closed: bool) -> SessionInit {
    let mut init = transcript()["init"].clone();
    if settled {
        init["data"]["settled"]["verify_closed"] = json!(verify_closed);
    } else {
        init["data"].as_object_mut().unwrap().remove("settled");
    }
    serde_json::from_value(init).unwrap()
}

fn bar(i: i64, c: f64) -> Bar {
    Bar {
        t: T0 + i * DAY,
        o: c,
        h: c + 0.5,
        l: c - 0.5,
        c,
        v: 1000.0,
    }
}

fn ev(seq: u64, change: Value) -> DataEvent {
    serde_json::from_value(
        json!({"feed":{"symbol":"THYAO","interval":"1d"},"seq":seq,"change":change}),
    )
    .unwrap()
}

fn msgs(lines: &[OutboundLine], conn: &str) -> Vec<Value> {
    lines
        .iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .collect()
}

fn connect(s: &mut Session, conn: &str, who: &str, subs: &[(&str, Value)]) {
    let w = s.handle_raw(
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"desktop"}})
            .to_string(),
    );
    assert_eq!(msgs(&w, conn)[0]["type"], "welcome");
    for (id, topic) in subs {
        let r = s.handle_raw(
            conn,
            &json!({"type":"subscribe","id":id,"topic":topic}).to_string(),
        );
        assert_eq!(msgs(&r, conn)[0]["status"], "subscribed", "{r:?}");
    }
}

fn claim_events(ms: &[Value]) -> Vec<&Value> {
    ms.iter()
        .filter(|m| m["event"]["kind"] == "claim")
        .map(|m| &m["event"])
        .collect()
}

/// The bad tick of transcript 37: t4 at 85 instead of 104, then t5 opens.
fn bad_tick(s: &mut Session) -> Vec<OutboundLine> {
    let mut lines = s
        .data_event(&ev(
            1,
            json!({"kind":"patch","t":T0 + 4 * DAY,"fields":{"c":85.0,"l":85.0}}),
        ))
        .unwrap();
    lines.extend(
        s.data_event(&ev(2, json!({"kind":"bar","bar":bar(5, 101.0)})))
            .unwrap(),
    );
    lines
}

#[test]
fn hygiene_withdraws_a_wrong_final_claim_and_retracts_its_capsule() {
    let mut s = Session::new(init(true, true), &root()).unwrap();
    connect(
        &mut s,
        "a",
        "arkin",
        &[
            ("k", json!({"kind":"claims"})),
            ("ch", json!({"kind":"changes"})),
        ],
    );
    let lines = bad_tick(&mut s);
    let ms = msgs(&lines, "a");
    let cl = claim_events(&ms);
    // Asserted provisional, then (t4 closed) the settled check runs at once (test
    // source): no final assertion survives; the claim is withdrawn as data_corrected.
    let last = cl.last().expect("claim events");
    assert_eq!(last["state"], "withdrawn", "{ms:?}");
    assert_eq!(last["reason"], "data_corrected");
    assert_eq!(last["evidence"]["values"][0]["value"], 104.0);
    let retract = ms
        .iter()
        .find(|m| m["event"]["changes"][0]["op"] == "put_capsule")
        .expect("retraction entry");
    let story = &retract["event"]["changes"][0]["capsule"]["story"];
    assert_eq!(story["stage"], "retracted");
    assert_eq!(
        story["history"].as_array().unwrap().last().unwrap()["reason"],
        "data_corrected"
    );
    assert!(
        s.document().ledger.entries.is_empty(),
        "never in the ledger"
    );
}

#[test]
fn settled_bars_assert_a_claim_the_feed_missed() {
    // Host path (no test source): the host verifies closed bars itself.
    let mut s = Session::new(init(false, false), &root()).unwrap();
    s.set_settle_check(true);
    connect(
        &mut s,
        "a",
        "arkin",
        &[
            ("k", json!({"kind":"claims"})),
            (
                "c",
                json!({"kind":"condition","condition":{"kind":"threshold","series":"px","op":"gt","value":110.0}}),
            ),
        ],
    );
    // t5 opens at 101; its closing tick (112, crossing 110) is lost on the feed; t6 opens.
    let mut lines = s
        .data_event(&ev(1, json!({"kind":"bar","bar":bar(5, 101.0)})))
        .unwrap();
    lines.extend(
        s.data_event(&ev(2, json!({"kind":"bar","bar":bar(6, 105.0)})))
            .unwrap(),
    );
    assert!(
        claim_events(&msgs(&lines, "a")).is_empty(),
        "nothing at t5 on the feed's data"
    );
    let reqs = s.take_settle_requests();
    assert!(
        reqs.iter()
            .any(|r| r.series == "px" && r.from_t <= T0 + 5 * DAY && r.to_t >= T0 + 5 * DAY),
        "{reqs:?}"
    );
    let mut b5 = bar(5, 112.0);
    b5.o = 101.0;
    b5.l = 100.5;
    let lines = s.settled_bars("px", &[bar(4, 104.0), b5]);
    let ms = msgs(&lines, "a");
    let cl = claim_events(&ms);
    assert_eq!(cl.len(), 1, "{ms:?}");
    assert_eq!(cl[0]["state"], "asserted");
    assert_eq!(cl[0]["evidence"]["t"], T0 + 5 * DAY);
    assert_eq!(cl[0]["evidence"]["maturity"], "final");
    assert_eq!(cl[0]["reason"], "data_corrected");
    // The same settled answer again changes nothing.
    assert!(claim_events(&msgs(&s.settled_bars("px", &[b5_again()]), "a")).is_empty());
}

fn b5_again() -> Bar {
    let mut b5 = bar(5, 112.0);
    b5.o = 101.0;
    b5.l = 100.5;
    b5
}

#[test]
fn a_dispute_waits_for_the_host_settled_answer() {
    let mut s = Session::new(init(false, false), &root()).unwrap();
    connect(&mut s, "r", "device", &[("k", json!({"kind":"claims"}))]);
    let _ = bad_tick(&mut s);
    let d = json!({"type":"dispute","id":"d","claim":{"kind":"capsule","capsule":"cap-floor","finding":"fnd-floor"},
        "t":T0 + 4 * DAY,"digest":"0000000000000abc"});
    let r = s.handle_raw("r", &d.to_string());
    let m = &msgs(&r, "r")[0];
    assert_eq!(m["data"]["verdict"], "pending", "{m}");
    let reqs = s.take_settle_requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!((reqs[0].from_t, reqs[0].to_t), (T0, T0 + 4 * DAY));
    // The host's settled answer: t4 closed at 104.
    let lines = s.settled_bars("px", &[bar(3, 103.0), bar(4, 104.0)]);
    let ms = msgs(&lines, "r");
    let cl = claim_events(&ms);
    assert_eq!(cl.len(), 1, "{ms:?}");
    assert_eq!(cl[0]["state"], "withdrawn");
    assert_eq!(cl[0]["reason"], "dispute_upheld");
    let cap = &s.document().capsules[0];
    assert_eq!(
        cap.story.as_ref().unwrap().stage,
        qp_schema::capsule::Stage::Retracted
    );
    // Idempotent: the same dispute again is a duplicate; the claim is gone anyway.
    let r = s.handle_raw("r", &d.to_string());
    assert_eq!(msgs(&r, "r")[0]["data"]["verdict"], "unknown");
}

#[test]
fn disputes_are_bounded_per_claim_and_digests_checked() {
    let mut s = Session::new(init(false, false), &root()).unwrap();
    connect(&mut s, "r", "device", &[("k", json!({"kind":"claims"}))]);
    let _ = bad_tick(&mut s);
    let dispute = |i: usize, digest: Value| {
        json!({"type":"dispute","id":format!("d{i}"),
            "claim":{"kind":"capsule","capsule":"cap-floor","finding":"fnd-floor"},
            "t":T0 + 4 * DAY,"digest":digest})
        .to_string()
    };
    // A digest is 16 lowercase hex digits: anything else is refused (no unbounded keys).
    for bad in ["abc", "ZZZZZZZZZZZZZZZZ", "0000000000000000aa"] {
        let r = s.handle_raw("r", &dispute(0, json!(bad)));
        let m = &msgs(&r, "r")[0];
        assert_eq!(m["type"], "error", "{m}");
        assert_eq!(m["error"]["path"], "/digest");
    }
    let mut verdicts = Vec::new();
    for i in 0..=qp_session::MAX_DISPUTE_DIGESTS {
        let r = s.handle_raw("r", &dispute(i, json!(format!("{i:016x}"))));
        verdicts.push(msgs(&r, "r")[0]["data"]["verdict"].clone());
    }
    assert!(verdicts[..qp_session::MAX_DISPUTE_DIGESTS]
        .iter()
        .all(|v| v == "pending"));
    assert_eq!(verdicts[qp_session::MAX_DISPUTE_DIGESTS], "limited");
}

/// An unentitled principal cannot tell an asserted claim from an absent one: both are
/// `unknown` (no existence oracle).
#[test]
fn an_unentitled_dispute_learns_nothing() {
    let mut init = transcript()["init"].clone();
    init["data"].as_object_mut().unwrap().remove("settled");
    init["principals"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"guest","author":{"kind":"human","name":"guest"},"role":"viewer"}));
    let mut s = Session::new(serde_json::from_value(init).unwrap(), &root()).unwrap();
    connect(&mut s, "g", "guest", &[]);
    let _ = bad_tick(&mut s);
    let ask = |s: &mut Session, t: i64| {
        let d = json!({"type":"dispute","id":"d",
            "claim":{"kind":"capsule","capsule":"cap-floor","finding":"fnd-floor"},
            "t":t,"digest":"0123456789abcdef"});
        msgs(&s.handle_raw("g", &d.to_string()), "g")
    };
    let asserted = ask(&mut s, T0 + 4 * DAY);
    let absent = ask(&mut s, T0 + 3 * DAY);
    assert_eq!(asserted, absent);
    assert_eq!(asserted[0]["data"]["verdict"], "unknown");
    assert!(s.take_settle_requests().is_empty());
}

/// Authority ↔ replica in step: the replica's own data has the true t4; it refetches,
/// shows the claim disputed, sends one `dispute`; the authority (test settled source)
/// upholds it and the replica drops the claim.
#[test]
fn the_replica_disputes_once_and_the_claim_falls() {
    let mut a = Session::new(init(true, false), &root()).unwrap();
    let data = transcript()["init"]["data"].clone();
    let mut r = Replica::with_memory("device", qp_session::tape::memory_provider(&data).unwrap());
    r.set_options(ReplicaOptions {
        dispute: true,
        ..ReplicaOptions::default()
    });
    let deliver = |a: &mut Session, r: &mut Replica, lines: Vec<OutboundLine>| {
        let mut q: VecDeque<OutboundLine> = lines.into();
        let mut disputes = 0;
        while let Some(l) = q.pop_front() {
            if l.conn != "r" {
                continue;
            }
            r.on_line(&serde_json::to_string(&l).unwrap()).unwrap();
            for (series, t) in r.take_refetch() {
                // The host's settled refetch: its own data is already settled.
                r.refetched(&series, t);
            }
            for m in r.take_outbox() {
                if m["type"] == "dispute" {
                    disputes += 1;
                }
                q.extend(a.handle_raw("r", &m.to_string()));
            }
        }
        disputes
    };
    let hello =
        json!({"type":"hello","protocol":"qp/0","principal":"device","surface":{"kind":"phone"}});
    let w = a.handle_raw("r", &hello.to_string());
    let mut n = deliver(&mut a, &mut r, w);
    // The replica's own feed: t5 opens (its t4 stays 104).
    r.data_event(&ev(1, json!({"kind":"bar","bar":bar(5, 101.0)})))
        .unwrap();
    let lines = bad_tick(&mut a);
    n += deliver(&mut a, &mut r, lines);
    assert_eq!(n, 1, "one dispute");
    assert!(r.display().is_empty(), "{:?}", r.display());
    assert_eq!(r.disputes_sent(), 1);
    assert_eq!(
        a.document().capsules[0].story.as_ref().unwrap().stage,
        qp_schema::capsule::Stage::Retracted
    );
}

/// A host provider whose bars the test changes (a split snapshot, a re-delivered
/// correction).
struct HostBars(std::rc::Rc<std::cell::RefCell<Vec<Bar>>>);

impl qp_source::SourceProvider for HostBars {
    fn entitled(&self, _principal: &str, _feed: &qp_schema::data::FeedKey) -> bool {
        true
    }

    fn resolve(
        &self,
        _principal: &str,
        _binding: &qp_schema::data::DataBinding,
    ) -> qp_source::Resolution {
        qp_source::Resolution::Ready {
            bars: self.0.borrow().clone(),
            seq: 0,
        }
    }
}

/// An authority over transcript 37's document whose data comes from `bars`, with a
/// `claims` subscription and a condition subscription `px > 110` on connection `a`.
fn host_session(bars: &std::rc::Rc<std::cell::RefCell<Vec<Bar>>>, hygiene: bool) -> Session {
    let mut init = transcript()["init"].clone();
    init.as_object_mut().unwrap().remove("data");
    let init: SessionInit = serde_json::from_value(init).unwrap();
    let mut s = Session::with_provider(init, &root(), Box::new(HostBars(std::rc::Rc::clone(bars))))
        .unwrap();
    s.set_settle_check(hygiene);
    connect(
        &mut s,
        "a",
        "arkin",
        &[
            ("k", json!({"kind":"claims"})),
            (
                "c",
                json!({"kind":"condition","condition":{"kind":"threshold","series":"px","op":"gt","value":110.0}}),
            ),
        ],
    );
    let _ = s.take_settle_requests();
    s
}

/// (R3/M7, AUD2) With hygiene on, a re-resolution that changes closed bars is a
/// provider correction: the claim its new data makes at a closed bar is asserted at once
/// and the changed range is queued for settled verification. Hygiene off: as before.
#[test]
fn a_resync_that_corrects_closed_bars_asserts_the_claims_it_makes() {
    let bars = std::rc::Rc::new(std::cell::RefCell::new(
        (0..6).map(|i| bar(i, 100.0 + i as f64)).collect::<Vec<_>>(),
    ));
    let mut s = host_session(&bars, true);
    // Unchanged data: nothing asserted, nothing to verify.
    let lines = s.resync_bindings();
    assert!(claim_events(&msgs(&lines, "a")).is_empty());
    assert!(s.take_settle_requests().is_empty());
    // The provider re-delivers t3 corrected to 112 (crossing 110).
    bars.borrow_mut()[3] = bar(3, 112.0);
    let lines = s.resync_bindings();
    let ms = msgs(&lines, "a");
    let cl = claim_events(&ms);
    assert_eq!(cl.len(), 1, "{ms:?}");
    assert_eq!(cl[0]["state"], "asserted");
    assert_eq!(cl[0]["evidence"]["t"], T0 + 3 * DAY);
    assert_eq!(cl[0]["evidence"]["maturity"], "final");
    let reqs = s.take_settle_requests();
    assert_eq!(reqs.len(), 1, "{reqs:?}");
    assert_eq!((reqs[0].from_t, reqs[0].to_t), (T0 + 3 * DAY, T0 + 4 * DAY));

    bars.borrow_mut()[3] = bar(3, 103.0);
    let mut s = host_session(&bars, false);
    bars.borrow_mut()[3] = bar(3, 112.0);
    let lines = s.resync_bindings();
    assert!(claim_events(&msgs(&lines, "a")).is_empty());
    assert!(s.take_settle_requests().is_empty());
}

/// (R3/M7, AUD2) A final claim on a bar the replica's lagging feed has not closed but the
/// host filled from the settled source (`set_settled`): when its own data does not hold
/// it, the replica asks for the settled range once (the bars before may miss
/// corrections) and never disputes before its bar closes.
#[test]
fn a_settled_but_open_bar_that_disagrees_asks_for_a_refetch() {
    let mut a = Session::new(init(true, false), &root()).unwrap();
    let data = transcript()["init"]["data"].clone();
    let mut r = Replica::with_memory("device", qp_session::tape::memory_provider(&data).unwrap());
    r.set_options(ReplicaOptions {
        dispute: true,
        ..ReplicaOptions::default()
    });
    let pump = |a: &mut Session, r: &mut Replica, lines: Vec<OutboundLine>| {
        let mut q: VecDeque<OutboundLine> = lines.into();
        while let Some(l) = q.pop_front() {
            if l.conn == "r" {
                r.on_line(&serde_json::to_string(&l).unwrap()).unwrap();
            }
            for m in r.take_outbox() {
                assert_ne!(m["type"], "dispute", "no dispute before the bar closes");
                q.extend(a.handle_raw("r", &m.to_string()));
            }
        }
    };
    let hello =
        json!({"type":"hello","protocol":"qp/0","principal":"device","surface":{"kind":"phone"}});
    let w = a.handle_raw("r", &hello.to_string());
    pump(&mut a, &mut r, w);
    // The authority's feed: t4 closes at 85 (a final claim at t4); the replica's feed
    // lags: its t4 is still open at 104.
    let lines = bad_tick(&mut a);
    pump(&mut a, &mut r, lines);
    let t4 = T0 + 4 * DAY;
    let item = r.display().iter().find(|i| i.t == t4).expect("claim at t4");
    assert_eq!(item.reason, Some("not_held"), "{item:?}");
    assert!(
        r.take_refetch().is_empty(),
        "open bar, not settled: no refetch"
    );
    // The host fills t4 from the settled source; still open on its chart.
    r.set_settled("px", t4);
    assert_eq!(r.take_refetch(), vec![("px".to_owned(), t4)]);
    let item = r.display().iter().find(|i| i.t == t4).expect("claim at t4");
    assert_eq!(item.state, qp_session::replica::Shown::Pending);
    // Once only; after the refetch it stays pending (not disputed) while open.
    r.refetched("px", t4);
    assert!(r.take_refetch().is_empty());
    let item = r.display().iter().find(|i| i.t == t4).expect("claim at t4");
    assert_eq!(item.state, qp_session::replica::Shown::Pending);
    assert!(r.take_outbox().is_empty());
}
