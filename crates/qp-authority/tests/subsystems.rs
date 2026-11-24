//! (RF1 S3.0) The registry's call points with a test-only subsystem: the phases are called
//! where the authority does that work today, the outbox is emitted at the end of the
//! message or settle point, registered kinds are asked first, and the draft hooks
//! (`Adjust`, `Settled`, claim kinds) do what their contract says. With nothing
//! registered every path is today's (S1 characterization, `compare.sh`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use qp_authority::{Session, Subsystems};
use qp_ops::expand::Ctx;
use qp_plane::error::E;
use qp_schema::protocol::{
    ChangeOp, ClaimRef, DisputeVerdict, ErrorCode, Op, OutboundLine, ProtocolError, Query,
    QueryResult, ServerMsg,
};
use qp_schema::Document;
use qp_subsystem::{ClaimKind, Moment, OpHandler, Phase, QueryHandler, SessionCtx, Subsystem};
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn session() -> Session {
    let init = qp_authority::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap();
    Session::new(init, &root(), Subsystems::NONE).unwrap()
}

/// What the probe saw, in order.
#[derive(Default)]
struct Seen(Vec<String>);

fn note(ctx: &mut SessionCtx<'_>, s: String) {
    if let Some(seen) = ctx.slots.get_mut::<Seen>() {
        seen.0.push(s);
    }
}

struct Probe;
impl Subsystem for Probe {
    fn name(&self) -> &'static str {
        "probe"
    }
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.slots.put(Seen::default());
    }
    fn phases(&self) -> &'static [Phase] {
        &[
            Phase::Check,
            Phase::AfterCommit,
            Phase::Connect,
            Phase::Subscribe,
            Phase::Unsubscribe,
            Phase::Proposals,
            Phase::Adjust,
        ]
    }
    fn collect(&self, phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        note(ctx, format!("{phase:?}"));
        match at {
            Moment::Entry { seq } => {
                let summary = ctx.view.log()[*seq as usize - 1].summary.clone();
                note(ctx, format!("entry {seq} {summary}"));
            }
            Moment::Sub { conn, .. } => {
                let line = OutboundLine {
                    conn: (*conn).to_owned(),
                    msg: ServerMsg::Error {
                        id: Some("probe".into()),
                        error: E::new(ErrorCode::Invalid, "probe").build(),
                    },
                };
                ctx.out.notify(line);
            }
            Moment::Adjust { symbol, .. } => {
                let view = ctx.view.document().views[0].id.clone();
                for _ in 0..2 {
                    let c = ChangeOp::SetEmphasis {
                        view: view.clone(),
                        emphasis: Vec::new(),
                    };
                    ctx.out.entry(vec![c], format!("plan {symbol}"));
                }
            }
            _ => {}
        }
    }
    fn check(
        &self,
        conn: &str,
        _ops: &[Op],
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError> {
        note(ctx, format!("check {conn}"));
        if conn == "deny" {
            return Err(E::new(ErrorCode::Forbidden, "probe refuses")
                .reason("probe")
                .build());
        }
        Ok(())
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        &[&SeenQuery]
    }
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        &[&Narrative]
    }
    fn claims(&self) -> &'static [&'static dyn ClaimKind] {
        &[&IntentClaims]
    }
}

/// `describe` answered by the probe (asked before the built-in query).
struct SeenQuery;
impl QueryHandler for SeenQuery {
    fn kind(&self) -> &'static str {
        "describe"
    }
    fn answer(
        &self,
        _conn: &str,
        _q: &Query,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<QueryResult, ProtocolError> {
        let seen = ctx
            .slots
            .get::<Seen>()
            .map(|s| s.0.clone())
            .unwrap_or_default();
        Ok(QueryResult::Item {
            collection: qp_schema::protocol::Collection::Findings,
            item: json!(seen),
        })
    }
}

/// `add_narrative` expanded by the probe into nothing (asked before the built-in one).
struct Narrative;
impl OpHandler for Narrative {
    fn op(&self) -> &'static str {
        "add_narrative"
    }
    fn expand(
        &self,
        _doc: &Document,
        _op: &Op,
        _at: &str,
        _ctx: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError> {
        Ok(Vec::new())
    }
}

/// Intent claims: every dispute of one is upheld.
struct IntentClaims;
impl ClaimKind for IntentClaims {
    fn kind(&self) -> &'static str {
        "intent"
    }
    fn verdict(
        &self,
        _conn: &str,
        _claim: &ClaimRef,
        _t: i64,
        _ctx: &mut SessionCtx<'_>,
    ) -> DisputeVerdict {
        DisputeVerdict::Upheld
    }
}

fn send(s: &mut Session, conn: &str, msg: Value) -> Vec<Value> {
    s.handle_raw(conn, &msg.to_string())
        .iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

fn hello(s: &mut Session, conn: &str, principal: &str) {
    let r = send(
        s,
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":"desktop"}}),
    );
    assert_eq!(r[0]["msg"]["type"], "welcome", "{r:?}");
}

fn probe_log(s: &mut Session, conn: &str) -> Value {
    let r = send(
        s,
        conn,
        json!({"type":"query","id":"q","query":{"kind":"describe"}}),
    );
    r[0]["msg"]["data"]["item"].clone()
}

#[test]
fn an_empty_registry_calls_nothing_and_a_probe_sees_every_phase_in_order() {
    let mut s = session();
    assert_eq!(s.next_wake(), None);
    s.register(&Probe).unwrap();
    assert!(
        s.register(&Probe).is_err(),
        "a kind registered twice is refused"
    );
    hello(&mut s, "a", "arkin");
    hello(&mut s, "deny", "arkin");
    // Check refuses before expansion; nothing is written.
    let n = s.log().len();
    let r = send(
        &mut s,
        "deny",
        json!({"type":"command","id":"c0","ops":[{"op":"undo"}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["reason"], "probe", "{r:?}");
    assert_eq!(s.log().len(), n);
    // An op the probe owns is expanded by it (into nothing): the entry has no changes.
    let r = send(
        &mut s,
        "a",
        json!({"type":"command","id":"c1","ops":[{"op":"add_narrative","about":"nope","steps":[]}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    assert!(s.log().last().unwrap().changes.is_empty());
    // Subscribe and unsubscribe: the probe's notification closes the message.
    let r = send(
        &mut s,
        "a",
        json!({"type":"subscribe","id":"s1","topic":{"kind":"changes"}}),
    );
    assert_eq!(r.last().unwrap()["msg"]["id"], "probe", "{r:?}");
    let r = send(
        &mut s,
        "a",
        json!({"type":"unsubscribe","id":"u1","sub":"s1"}),
    );
    assert_eq!(r[0]["msg"]["status"], "unsubscribed");
    assert_eq!(r.last().unwrap()["msg"]["id"], "probe", "{r:?}");
    let log = probe_log(&mut s, "a");
    let got: Vec<&str> = log
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let entry = format!("entry {} ", n + 1);
    assert_eq!(got.len(), 8, "{log}");
    assert_eq!(
        got[..5],
        ["Connect", "Connect", "check deny", "check a", "AfterCommit"],
        "{log}"
    );
    assert!(got[5].starts_with(&entry), "{log}");
    assert_eq!(got[6..], ["Subscribe", "Unsubscribe"], "{log}");
}

#[test]
fn adjust_writes_one_engine_entry_for_every_plan() {
    let mut s = session();
    s.register(&Probe).unwrap();
    let n = s.log().len();
    let _ = s.host_adjust("THYAO", 0.5, 0);
    assert_eq!(s.log().len(), n + 1, "one entry for both plans");
    let e = s.log().last().unwrap();
    assert_eq!(e.summary, "adjust THYAO");
    assert_eq!(e.changes.len(), 2);
    // Without a subsystem: nothing.
    let mut bare = session();
    assert!(bare.host_adjust("THYAO", 0.5, 0).is_empty());
    assert_eq!(bare.log().len(), n);
}

#[test]
fn a_registered_claim_kind_answers_its_disputes() {
    let mut s = session();
    hello(&mut s, "a", "arkin");
    let d = json!({"type":"dispute","id":"d","claim":{"kind":"intent","intent":"i1"},"t":0});
    let before = send(&mut s, "a", d.clone());
    assert_eq!(before[0]["msg"]["data"]["verdict"], "unknown", "{before:?}");
    s.register(&Probe).unwrap();
    let after = send(&mut s, "a", d);
    assert_eq!(after[0]["msg"]["data"]["verdict"], "upheld", "{after:?}");
}

/// Notes every settled answer as a notification to connection `host`.
struct Settle;
impl Subsystem for Settle {
    fn name(&self) -> &'static str {
        "settle"
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::Settled]
    }
    fn collect(&self, _phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        if let Moment::Settled { series, .. } = at {
            let error = E::new(ErrorCode::Invalid, format!("settled {series}")).build();
            let msg = ServerMsg::Error { id: None, error };
            ctx.out.notify(OutboundLine {
                conn: "host".into(),
                msg,
            });
        }
    }
}

#[test]
fn the_settled_phase_follows_a_settled_answer() {
    const T0: i64 = 1_759_708_800_000;
    const DAY: i64 = 86_400_000;
    let path = root().join("fixtures/protocol/conformance/37-dispute-retract.json");
    let t: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut init = t["init"].clone();
    init["data"].as_object_mut().unwrap().remove("settled");
    let mut s = Session::new(
        serde_json::from_value(init).unwrap(),
        &root(),
        Subsystems::NONE,
    )
    .unwrap();
    s.register(&Settle).unwrap();
    let bar = |i: i64, c: f64| qp_schema::Bar {
        t: T0 + i * DAY,
        o: c,
        h: c + 0.5,
        l: c - 0.5,
        c,
        v: 1000.0,
    };
    let lines = s.settled_bars("px", &[bar(3, 103.0), bar(4, 104.0)]);
    let last = serde_json::to_value(lines.last().unwrap()).unwrap();
    assert_eq!(last["msg"]["error"]["message"], "settled px", "{last}");
}
