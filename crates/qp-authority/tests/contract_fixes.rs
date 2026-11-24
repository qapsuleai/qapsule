//! (RF1 S3.1) The subsystem contract's fixes from the S3.0 audit (`rounds/RF1/experiments/
//! audit-s30.md`), through the authority: a wake the clock reached is spent (B2), the emit
//! step's depth bound (B3), and a registered claim kind or topic keeps the built-in guards
//! (B5). B1 (condition kinds past the live path's memo) is a unit test of `data/live.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use qp_authority::{Session, Subsystems};
use qp_plane::error::E;
use qp_schema::protocol::{ChangeOp, ClaimRef, DisputeVerdict, ErrorCode, ProtocolError, Topic};
use qp_subsystem::{ClaimKind, Moment, Phase, SessionCtx, Subsystem, TopicHandler, MAX_EMIT_DEPTH};
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn session() -> Session {
    let init = qp_authority::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap();
    Session::new(init, &root(), Subsystems::NONE).unwrap()
}

fn send(s: &mut Session, conn: &str, msg: Value) -> Vec<Value> {
    s.handle_raw(conn, &msg.to_string())
        .iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

fn hello(s: &mut Session, conn: &str) {
    let m =
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}});
    let r = send(s, conn, m);
    assert_eq!(r[0]["msg"]["type"], "welcome", "{r:?}");
}

/// Asks to be woken at 100 when registered.
struct Waker;
impl Subsystem for Waker {
    fn name(&self) -> &'static str {
        "waker"
    }
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.out.wake_at(ctx.view.clock() + 100);
    }
}

#[test]
fn a_wake_the_clock_reached_is_spent() {
    let mut s = session();
    s.register(&Waker).unwrap();
    hello(&mut s, "a");
    let at = s.next_wake().expect("asked");
    let _ = s.host_clock(at - 50).unwrap();
    assert_eq!(s.next_wake(), Some(at), "not reached yet");
    let _ = s.host_clock(at).unwrap();
    assert_eq!(s.next_wake(), None);
}

/// Answers every log entry with an engine entry of its own (a loop without the bound).
struct Echo;
impl Subsystem for Echo {
    fn name(&self) -> &'static str {
        "echo"
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::AfterCommit]
    }
    fn collect(&self, _phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        if let Moment::Entry { seq } = at {
            let view = ctx.view.document().views[0].id.clone();
            let c = ChangeOp::SetEmphasis {
                view,
                emphasis: Vec::new(),
            };
            ctx.out.entry(vec![c], format!("echo {seq}"));
        }
    }
}

#[test]
fn requested_entries_stop_at_the_emit_depth() {
    let mut s = session();
    s.register(&Echo).unwrap();
    hello(&mut s, "a");
    let n = s.log().len();
    let view = s.document().views[0].id.clone();
    let op = json!({"op":"set_emphasis","view":view,"emphasis":[]});
    let r = send(&mut s, "a", json!({"type":"command","id":"c","ops":[op]}));
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    let depth = usize::try_from(MAX_EMIT_DEPTH).unwrap();
    assert_eq!(
        s.log().len(),
        n + 1 + depth,
        "the command, then one echo per level"
    );
    let summaries: Vec<&str> = s.log()[n + 1..]
        .iter()
        .map(|e| e.summary.as_str())
        .collect();
    assert_eq!(
        summaries[0],
        format!("echo {}", n + 1),
        "depth first: each echoes the last"
    );
    assert_eq!(summaries[1], format!("echo {}", n + 2));
}

/// Intent claims: every dispute of one is upheld. `changes` subscriptions: refused.
struct Claims;
impl ClaimKind for Claims {
    fn kind(&self) -> &'static str {
        "intent"
    }
    fn verdict(&self, _: &str, _: &ClaimRef, _: i64, _: &mut SessionCtx<'_>) -> DisputeVerdict {
        DisputeVerdict::Upheld
    }
}
struct NoChanges;
impl TopicHandler for NoChanges {
    fn kind(&self) -> &'static str {
        "changes"
    }
    fn subscribe(
        &self,
        _: &str,
        _: &str,
        _: &Topic,
        _: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError> {
        Err(E::new(ErrorCode::Forbidden, "no changes here").build())
    }
}
struct Guarded;
impl Subsystem for Guarded {
    fn name(&self) -> &'static str {
        "guarded"
    }
    fn claims(&self) -> &'static [&'static dyn ClaimKind] {
        &[&Claims]
    }
    fn topics(&self) -> &'static [&'static dyn TopicHandler] {
        &[&NoChanges]
    }
}

#[test]
fn a_registered_claim_kind_keeps_the_dispute_guards() {
    let mut s = session();
    s.register(&Guarded).unwrap();
    hello(&mut s, "a");
    let d = |digest: &str| json!({"type":"dispute","id":"d","claim":{"kind":"intent","intent":"i1"},"t":0,"digest":digest});
    let bad = send(&mut s, "a", d("XYZ"));
    assert_eq!(bad[0]["msg"]["error"]["code"], "invalid", "{bad:?}");
    let first = send(&mut s, "a", d("0123456789abcdef"));
    assert_eq!(first[0]["msg"]["data"]["verdict"], "upheld", "{first:?}");
    let again = send(&mut s, "a", d("0123456789abcdef"));
    assert_eq!(again[0]["msg"]["data"]["verdict"], "duplicate", "{again:?}");
}

#[test]
fn a_refused_subscription_leaves_nothing_behind() {
    let mut s = session();
    s.register(&Guarded).unwrap();
    hello(&mut s, "a");
    let sub = |kind: &str| json!({"type":"subscribe","id":"s1","topic":{"kind":kind}});
    let r = send(&mut s, "a", sub("changes"));
    assert_eq!(r[0]["msg"]["error"]["code"], "forbidden", "{r:?}");
    // The id is free again; the connection holds no viewer.
    let r = send(&mut s, "a", sub("proposals"));
    assert_eq!(r[0]["msg"]["status"], "subscribed", "{r:?}");
    assert!(s.viewer_data("a").is_none());
}
