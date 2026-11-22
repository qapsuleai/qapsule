//! R4/K42-FEED (`rounds/R4/experiments/K42-FEED.md`): a negative transition (K42/A22) of a
//! held instrument's capsule, known in the current session, reaches the reader's feed (the
//! shell's `capsules` query with `audience` and `FEED_LIMIT`, and the feed rows of every
//! layout), even when the importance order would put it below the page size. The demo's
//! host (`qp demo --synthetic`), the human test's audience (`rounds/R4/human-test/plan.json`
//! G2), sensitivity `balanced`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use qp_schema::protocol::{CapsuleItem, QueryResult, ServerMsg};
use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::{ShellCore, FEED_LIMIT};
use serde_json::{json, Value};

const WATCH: [&str; 8] = [
    "SYN02", "SYN03", "SYN04", "SYN05", "SYN22", "SYN25", "SYN26", "SYN28",
];

fn send(h: &mut ReplayHost, m: &Value) -> Vec<ServerMsg> {
    h.handle_raw("k", &m.to_string())
        .into_iter()
        .filter(|l| l.conn == "k")
        .map(|l| l.msg)
        .collect()
}

fn capsules(h: &mut ReplayHost, mut q: Value) -> Vec<CapsuleItem> {
    q["kind"] = json!("capsules");
    send(h, &json!({"type":"query","id":"q","query":q}))
        .into_iter()
        .find_map(|m| match m {
            ServerMsg::Result {
                data: QueryResult::Capsules { capsules, .. },
                ..
            } => Some(capsules),
            _ => None,
        })
        .expect("capsules answer")
}

/// A bare protocol client: seek to `t0`, set the audience (long `held`), seek to `t0 + 9`
/// (one session); the answers `audience` without and with the feed's limit.
fn window(t0: u64, held: [&str; 2]) -> (Vec<CapsuleItem>, Vec<CapsuleItem>) {
    let mut h = parity_host();
    send(
        &mut h,
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin",
            "surface":{"kind":"phone","screen":[390,844],"input":["touch"],"lang":"tr"}}),
    );
    send(
        &mut h,
        &json!({"type":"replay","id":"r0","control":{"kind":"seek","step":t0}}),
    );
    let aud = json!({"group":"habituated","sensitivity":"balanced","watchlist":WATCH,
        "positions":{held[0]:1.0, held[1]:1.0}});
    send(
        &mut h,
        &json!({"type":"command","id":"c","ops":[{"op":"set_audience","audience":aud}]}),
    );
    send(
        &mut h,
        &json!({"type":"replay","id":"r1","control":{"kind":"seek","step":t0 + 9}}),
    );
    let all = capsules(&mut h, json!({"audience":true}));
    let feed = capsules(&mut h, json!({"audience":true,"limit":FEED_LIMIT}));
    (all, feed)
}

fn ids(v: &[CapsuleItem]) -> Vec<&str> {
    v.iter().map(|i| i.id.as_str()).collect()
}

fn ranked(feed: &[CapsuleItem]) -> bool {
    feed.windows(2).all(|w| {
        w[0].importance > w[1].importance
            || (w[0].importance == w[1].importance
                && (w[0].t > w[1].t || (w[0].t == w[1].t && w[0].id < w[1].id)))
    })
}

#[test]
fn current_negative_transition_of_a_position_is_in_the_feed() {
    // (window start, held, the capsules with the session's negative transition)
    let cases: [(u64, [&str; 2], &[&str]); 3] = [
        (
            762,
            ["SYN09", "SYN12"],
            &["cap.SYN12.breakout.up.203", "cap.SYN12.extreme.up.203"],
        ),
        (801, ["SYN19", "SYN01"], &["cap.SYN01.reversal.down.205"]),
        (906, ["SYN09", "SYN12"], &["cap.SYN12.squeeze.neutral.215"]),
    ];
    for (t0, held, negs) in cases {
        let (all, feed) = window(t0, held);
        assert!(all.len() > FEED_LIMIT as usize, "{t0}: the limit cuts");
        assert_eq!(feed.len(), FEED_LIMIT as usize, "{t0}: at most the limit");
        assert!(ranked(&feed), "{t0}: the answer stays in rank order");
        for id in negs {
            let it = all
                .iter()
                .find(|i| i.id == *id)
                .unwrap_or_else(|| panic!("{t0}: {id} passes the audience"));
            assert!(it.relevant && it.stage.is_negative(), "{t0}: {id}");
            let row = all.iter().position(|i| i.id == *id).unwrap();
            assert!(
                row >= FEED_LIMIT as usize,
                "{t0}: {id} ranks below the page size"
            );
            assert!(ids(&feed).contains(id), "{t0}: {id} is in the feed");
        }
        // Only those displace: the rest is the top of the ranking.
        let rest: Vec<&str> = ids(&feed)
            .into_iter()
            .filter(|i| !negs.contains(i))
            .collect();
        assert_eq!(rest, ids(&all)[..rest.len()], "{t0}");
    }
}

#[test]
fn feed_without_a_current_negative_transition_is_unchanged() {
    // The M9 scenario's window (its negative transition ranks inside the limit) and the
    // human test's two G2 windows: the answers (and so their keys) are as before the fix.
    let (all, feed) = window(600, ["SYN09", "SYN12"]);
    assert_eq!(ids(&feed), ids(&all)[..FEED_LIMIT as usize]);
    assert!(ids(&feed).contains(&"cap.SYN09.flow.up.183"));
    let (all, feed) = window(882, ["SYN19", "SYN01"]);
    assert_eq!(ids(&feed), ids(&all)[..FEED_LIMIT as usize]);
}

#[test]
fn shell_feed_rows_show_it_in_every_layout() {
    let mut w = parity_host();
    w.set_record_events(true);
    let data = w.init().data.clone().expect("replay init has data");
    let evs = w.recorded_events();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    let pump = |core: &mut ShellCore, w: &mut ReplayHost| {
        for _ in 0..1000 {
            let out = core.take_outbox();
            if out.is_empty() {
                break;
            }
            for m in out {
                for l in w.handle_raw("s", &m.to_string()) {
                    if l.conn == "s" {
                        core.on_message(&l.msg);
                    }
                }
            }
        }
    };
    let seek = |core: &mut ShellCore, w: &mut ReplayHost, step: u64| {
        let before = w.recorded_len();
        core.seek(&step.to_string());
        pump(core, w);
        let evs: Vec<_> = w.recorded_since(before);
        for ev in &evs {
            core.data_event(ev);
        }
        pump(core, w);
    };
    pump(&mut core, &mut w);
    seek(&mut core, &mut w, 762);
    core.set_audience(&json!({"group":"habituated","sensitivity":"balanced",
        "watch":WATCH,"long":["SYN09","SYN12"],"short":[]}));
    pump(&mut core, &mut w);
    core.mark_seen();
    pump(&mut core, &mut w);
    seek(&mut core, &mut w, 771);
    let want = "cap.SYN12.breakout.up.203";
    assert!(core.feed().iter().any(|i| i.id == want));
    for name in qp_grammar::feeds::LAYOUTS.iter().map(|l| l.name) {
        core.set_look(&json!({"feed": name}));
        pump(&mut core, &mut w);
        let rows = core.feed_json();
        let found = rows
            .as_array()
            .expect("rows")
            .iter()
            .any(|r| r.get("id").and_then(Value::as_str) == Some(want));
        assert!(found, "layout {name}: {want} in the rows");
    }
}
