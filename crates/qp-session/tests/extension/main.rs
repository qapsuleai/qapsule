//! (RF1 E, `rounds/RF1/PLAN.md` §7; `docs/ARCHITECTURE.md` "Yeni alt sistem nasıl eklenir")
//! The extension point, tried from outside the authority:
//!
//! - (b) `Tally`, a subsystem that lives only in this test, is composed with the standard
//!   ones through `qp-session`'s public API (no line in `compose.rs`, nothing in
//!   `qp-authority`): it owns an op, writes to the log through an engine entry, answers a
//!   query, pushes on its topic; `replay` rebuilds the document from the changes alone and
//!   a replica built with the same list follows it.
//! - (a) The real subsystem: without `qp-watch`'s line `add_watch` is `unsupported` for a
//!   writer; with it, it is applied.
//! - (c) `qp-authority` depends on no subsystem crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod rig;
mod tally;

use qp_session::{document_digest, Session, Subsystems, STANDARD};
use serde_json::json;

use rig::{hello, list, send, with_list};
use tally::{is_mark, TallyState, SUMMARY, TALLY};

#[test]
fn ext_tally_counts_its_op_through_an_engine_entry() {
    let mut s = with_list(list(&[&TALLY]));
    let initial = s.document().clone();
    hello(&mut s, "a", "arkin");
    let r = send(
        &mut s,
        "a",
        json!({"type":"subscribe","id":"t","topic":{"kind":"proposals"}}),
    );
    assert_eq!(r[0]["msg"]["type"], "ack", "{r:?}");
    let mut pushed = 0;
    for (i, (from, to)) in [(1, 5), (2, 8), (3, 9)].into_iter().enumerate() {
        let r = send(
            &mut s,
            "a",
            json!({"type":"command","id":format!("c{i}"),"ops":[
                {"op":"set_view_range","view":"main","from_bar":from,"to_bar":to}]}),
        );
        assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
        pushed += r
            .iter()
            .filter(|l| l["msg"]["type"] == "notify" && l["msg"]["sub"] == "t")
            .filter(|l| l["msg"]["event"]["author"] == "qp")
            .count();
    }
    // Another op is not counted.
    let r = send(
        &mut s,
        "a",
        json!({"type":"command","id":"e","ops":[{"op":"set_emphasis","view":"main","emphasis":[]}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");

    // The registry called Tally's handler: each command entry carries its expansion's mark
    // (the built-in expansion alone would not).
    let marked = s
        .log()
        .iter()
        .filter(|e| !e.request.is_empty())
        .map(|e| e.changes.iter().filter(|c| is_mark(c)).count())
        .collect::<Vec<_>>();
    assert_eq!(marked, [1, 1, 1, 0], "set_view_range ×3, set_emphasis");

    // Its state (host API: the slot by type), the log, the document.
    let t = s.slots().get::<TallyState>().unwrap();
    assert_eq!(t.count, 3);
    assert_eq!(pushed, 3, "one push per tally entry");
    let tallies: Vec<_> = s
        .log()
        .iter()
        .filter(|e| e.summary.starts_with(SUMMARY))
        .collect();
    assert_eq!(tallies.len(), 3);
    assert!(tallies
        .iter()
        .all(|e| e.author == "qp" && e.request.is_empty()));
    assert_eq!(tallies[2].summary, "tally 3");
    let n = s
        .document()
        .narratives
        .iter()
        .find(|n| n.id == "tally")
        .unwrap();
    assert_eq!(n.provenance.based_on.len(), 3);

    // Its query.
    let r = send(
        &mut s,
        "a",
        json!({"type":"query","id":"q","query":{"kind":"describe"}}),
    );
    assert_eq!(r[0]["msg"]["data"]["item"]["tally"], 3, "{r:?}");

    // Replay: the changes alone rebuild the document (no subsystem code).
    let replayed = qp_ops::apply::replay(&initial, s.log()).unwrap();
    assert_eq!(document_digest(&replayed), document_digest(s.document()));
}

#[test]
fn ext_tally_a_replica_with_the_same_list_follows() {
    let subs = list(&[&TALLY]);
    let mut s = with_list(subs);
    let mut r = qp_authority::Replica::with_memory("arkin", qp_source::MemoryProvider::new(), subs);
    let hello_msg =
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}});
    let w = s.handle_raw("r", &hello_msg.to_string());
    rig::deliver(&mut s, &mut r, w);
    hello(&mut s, "a", "arkin");
    for (i, to) in [5, 8].into_iter().enumerate() {
        let lines = s.handle_raw(
            "a",
            &json!({"type":"command","id":format!("c{i}"),"ops":[
                {"op":"set_view_range","view":"main","from_bar":1,"to_bar":to}]})
            .to_string(),
        );
        rig::deliver(&mut s, &mut r, lines);
    }
    assert_eq!(s.slots().get::<TallyState>().unwrap().count, 2);
    assert_eq!(r.seq(), s.log().len() as u64);
    let doc = r.document().unwrap();
    assert!(
        doc.narratives.iter().any(|n| n.id == "tally"),
        "the tally item reached it"
    );
    assert_eq!(
        r.document_digest().unwrap(),
        document_digest(s.document()),
        "the replica follows the authority's document"
    );
}

#[test]
fn ext_without_the_watch_line_add_watch_is_unsupported() {
    let add = json!({"type":"command","id":"w","ops":[{"op":"add_watch","text":"x",
        "match":{"instrument":"THYAO"}}]});
    // STANDARD without `qp-watch`: a writer gets `unsupported`.
    let no_watch: Vec<_> = STANDARD
        .list()
        .iter()
        .copied()
        .filter(|s| s.name() != "watch")
        .collect();
    assert_eq!(no_watch.len() + 1, STANDARD.list().len());
    let mut s = with_list(Subsystems::new(Box::leak(no_watch.into_boxed_slice())));
    hello(&mut s, "a", "arkin");
    let r = send(&mut s, "a", add.clone());
    assert_eq!(r[0]["msg"]["error"]["code"], "unsupported", "{r:?}");
    // With STANDARD (the composition), it is applied.
    let mut s = Session::new(rig::init(), &rig::root()).unwrap();
    hello(&mut s, "a", "arkin");
    let r = send(&mut s, "a", add);
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
}

#[test]
fn ext_the_authority_depends_on_no_subsystem_crate() {
    let root = rig::root();
    let cfg: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("scripts/arch-config.json")).unwrap(),
    )
    .unwrap();
    // The subsystem crates: after the contract, before the authority (arch-config order).
    let order: Vec<String> = cfg["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "oturum")
        .unwrap()["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    let pos = |n: &str| order.iter().position(|c| c == n).unwrap();
    let subsystems = &order[pos("qp-subsystem") + 1..pos("qp-authority")];
    assert!(subsystems.iter().any(|c| c == "qp-watch"), "{order:?}");
    // qp-authority's dependencies as Cargo resolves them (every manifest form: inline,
    // `.workspace`, `[dependencies.x]`, renamed; normal, dev and build).
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let out = std::process::Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let deps: Vec<String> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "qp-authority")
        .unwrap()["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        deps.iter().any(|d| d == "qp-subsystem"),
        "the contract: {deps:?}"
    );
    for c in subsystems {
        assert!(
            !deps.contains(c),
            "qp-authority depends on subsystem crate {c}"
        );
    }
}
