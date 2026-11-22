//! R4/T2 (GOAL criterion 1, §21 with §19): on a replay of 30 synthetic instruments
//! (daily bars moved by their hourly bars, the capsule engine on every one) the authority
//! and a replica fed from its own provider behave as in RP2/R3: claims are shown, none
//! wrongly or early, and the replica's document digest equals the authority's after
//! every step. Two runs give byte-identical message streams.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::protocol::{
    ClaimRef, ClaimState, Event, Maturity, OutboundLine, ReplayInit, ServerMsg,
};
use qp_schema::Condition;
use qp_session::replica::Shown;
use qp_session::{demo_init, document_digest, ReplayHost, Replica, ReplicaOptions};
use serde_json::json;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const WATCHED: usize = 5;

fn host() -> ReplayHost {
    let mut init = demo_init(ReplayInit {
        instruments: Some(30),
        days: Some(160),
        history: Some(120),
        fine: Some("1h".to_owned()),
        ..ReplayInit::default()
    });
    // RSI(14) on the first instruments (their bound series are added by `bind_all`).
    let mut doc: qp_schema::Document = match init.document {
        qp_schema::protocol::DocumentRef::Inline(d) => *d,
        qp_schema::protocol::DocumentRef::Path(_) => unreachable!(),
    };
    for i in 1..=WATCHED {
        doc.series.push(
            serde_json::from_value(json!({"id":format!("rsi{i:02}"),"interval":"1d",
                "source":{"kind":"study","study":"rsi","inputs":[format!("syn{i:02}")],"params":{"period":14.0}}}))
            .unwrap(),
        );
    }
    init.document = qp_schema::protocol::DocumentRef::Inline(Box::new(doc));
    let none = std::env::temp_dir().join("qp-replay-test-no-local-data");
    ReplayHost::with_local_dir(init, &root(), &none).unwrap()
}

fn watches() -> Vec<(String, Condition)> {
    let mut v = Vec::new();
    for i in 1..=WATCHED {
        for (op, k) in [("cross_above", 60.0), ("cross_below", 40.0)] {
            v.push((
                format!("{op}{i:02}"),
                serde_json::from_value(
                    json!({"kind":"threshold","series":format!("rsi{i:02}"),"op":op,"value":k}),
                )
                .unwrap(),
            ));
        }
    }
    v
}

struct Run {
    stream: String,
    steps: usize,
    differing: usize,
    violations: Vec<String>,
    shown: BTreeMap<&'static str, usize>,
    capsules: usize,
    final_doc_equal: bool,
}

#[allow(clippy::too_many_lines)]
fn run() -> Run {
    let mut h = host();
    let mut prov = qp_source::MemoryProvider::new();
    for (k, bars) in h.replay().feeds() {
        prov.add_feed(k.clone(), bars.to_vec()).unwrap();
    }
    prov.entitle("guest", ["*".to_owned()]).unwrap();
    let mut r = Replica::with_memory("guest", prov);
    r.set_options(ReplicaOptions::default());
    let mut said: BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)> = BTreeMap::new();
    let mut stream = String::new();
    let deliver = |h: &mut ReplayHost,
                   r: &mut Replica,
                   said: &mut BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)>,
                   stream: &mut String,
                   lines: Vec<OutboundLine>| {
        let mut q: std::collections::VecDeque<_> = lines.into();
        while let Some(l) = q.pop_front() {
            stream.push_str(&serde_json::to_string(&l).unwrap());
            stream.push('\n');
            if l.conn != "r" {
                continue;
            }
            if let ServerMsg::Notify {
                event:
                    Event::Claim {
                        claim,
                        state,
                        evidence,
                        ..
                    },
                ..
            } = &l.msg
            {
                said.insert((claim.clone(), evidence.t), (*state, evidence.maturity));
            }
            r.on_message(&l.msg);
            for m in r.take_outbox() {
                q.extend(h.handle_raw("r", &m.to_string()));
            }
        }
    };
    let hello =
        json!({"type":"hello","protocol":"qp/0","principal":"guest","surface":{"kind":"phone"}});
    let lines = h.handle_raw("r", &hello.to_string());
    for (id, c) in watches() {
        r.watch(&id, c);
    }
    deliver(&mut h, &mut r, &mut said, &mut stream, lines);
    let mut differing = 0;
    let mut violations = Vec::new();
    let mut shown: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut steps = 0;
    let mut compared = 0;
    let mut last_pos = (u64::MAX, u64::MAX);
    while let Some((st, lines)) = h.take_step() {
        steps += 1;
        deliver(&mut h, &mut r, &mut said, &mut stream, lines);
        for ev in &st.events {
            r.data_event(ev).unwrap();
        }
        // The documents change only with log entries: compare whenever either side's
        // log position moved (between such steps both stay as compared).
        let pos = (h.session().log().len() as u64, r.seq());
        if pos != last_pos {
            last_pos = pos;
            compared += 1;
            if r.document_digest().as_deref()
                != Some(document_digest(h.session().document()).as_str())
            {
                differing += 1;
            }
        }
        for it in r.display() {
            if it.state != Shown::Shown {
                continue;
            }
            let kind = match it.claim {
                ClaimRef::Condition { .. } => "condition",
                ClaimRef::Intent { .. } => "intent",
                ClaimRef::Capsule { .. } => "capsule",
            };
            *shown.entry(kind).or_default() += 1;
            let key = (it.claim.clone(), it.t);
            let s = said.get(&key).copied();
            if !matches!(s, Some((ClaimState::Asserted, _))) {
                violations.push(format!(
                    "step {steps}: {key:?} shown but not asserted ({s:?})"
                ));
            }
            let own = r.bars(&it.series).unwrap_or_default();
            let Some(j) = own.iter().position(|b| b.t == it.t) else {
                violations.push(format!("step {steps}: {key:?} shown, own data lacks t"));
                continue;
            };
            if it.label == Some(Maturity::Final) {
                if j + 1 >= own.len() {
                    violations.push(format!("step {steps}: {key:?} final before its bar closed"));
                }
                if !matches!(s, Some((_, Maturity::Final))) {
                    violations.push(format!(
                        "step {steps}: {key:?} final before the authority's"
                    ));
                }
            } else {
                violations.push(format!(
                    "step {steps}: {key:?} provisional with live display off"
                ));
            }
        }
    }
    assert!(h.notices().is_empty(), "{:?}", h.notices());
    eprintln!("compared digests at {compared} of {steps} steps");
    Run {
        stream,
        steps,
        differing,
        violations,
        shown,
        capsules: h.session().document().capsules.len(),
        final_doc_equal: r.document_digest().as_deref()
            == Some(document_digest(h.session().document()).as_str()),
    }
}

#[test]
fn authority_and_replica_on_a_30_instrument_replay() {
    let a = run();
    assert!(a.steps > 300, "{}", a.steps);
    assert_eq!(
        a.differing, 0,
        "replica digest != authority on {} steps",
        a.differing
    );
    assert!(a.final_doc_equal);
    assert!(
        a.violations.is_empty(),
        "{:?}",
        &a.violations[..a.violations.len().min(5)]
    );
    assert!(a.capsules > 0, "the engine made capsules");
    assert!(
        a.shown.get("condition").copied().unwrap_or(0) > 0,
        "{:?}",
        a.shown
    );
    eprintln!(
        "steps {}, capsules {}, shown {:?}, stream {} bytes",
        a.steps,
        a.capsules,
        a.shown,
        a.stream.len()
    );
    let b = run();
    assert_eq!(
        a.stream, b.stream,
        "two runs: byte-identical message stream"
    );
}
