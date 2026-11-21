//! (R5B, `docs/PROTOCOL.md` §25.4) Comments: validated against what the writer may read,
//! stored with provenance, sent to the readers that may receive them.

mod common;

use common::root;
use qp_schema::protocol::{Principal, SessionInit, Transcript};
use qp_session::Session;
use serde_json::{json, Value};

const CAP: &str = "cap.THYAO.continuation.down.84";

fn init() -> SessionInit {
    let t: Transcript = qp_session::read_json(
        &root().join("fixtures/protocol/conformance/39-capsule-entitlement.json"),
    )
    .unwrap();
    let qp_schema::protocol::InitRef::Inline(i) = t.init else {
        panic!("inline init")
    };
    let mut i = *i;
    for (a, p) in [("aa", "ana"), ("cc", "can")] {
        let pr: Principal = serde_json::from_value(
            json!({"id":a,"author":{"kind":"agent","name":a,"model":"test"},
            "role":"proposer","on_behalf":p}),
        )
        .unwrap();
        i.principals.push(pr);
    }
    i
}

struct H(Session);

impl H {
    fn new() -> Self {
        let mut h = H(Session::new(init(), &root()).unwrap());
        for (c, p) in [("A", "aa"), ("C", "cc"), ("a", "ana"), ("c", "can")] {
            h.send(
                c,
                json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
            );
        }
        h.0.mark_agent_connection("A");
        h.0.mark_agent_connection("C");
        h
    }
    fn send(&mut self, conn: &str, msg: Value) -> Vec<Value> {
        self.0
            .handle_raw(conn, &msg.to_string())
            .iter()
            .map(|l| serde_json::to_value(l).unwrap())
            .collect()
    }
    fn comment(&mut self, conn: &str, text: &str, cites: Value) -> Vec<Value> {
        self.send(
            conn,
            json!({"type":"command","id":"k","ops":[{"op":"add_comment","about":{"capsule":CAP},
                "lang":"tr","text":text,"cites":cites}]}),
        )
    }
}

fn reasons(v: &Value) -> Vec<String> {
    v["msg"]["error"]["reasons"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| r["code"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_comment_that_rests_on_its_evidence_is_stored_with_provenance() {
    let mut h = H::new();
    let out = h.comment(
        "A",
        "THYAO kısa ortalamanın (5,23) altında kalıyor [1].",
        json!([CAP]),
    );
    assert_eq!(out[0]["msg"]["status"], "applied", "{out:?}");
    let c = h.0.document().comments.last().expect("stored").clone();
    let c = serde_json::to_value(c).unwrap();
    assert_eq!(c["about"]["capsule"], CAP);
    assert_eq!(c["provenance"]["author"]["kind"], "agent");
    assert_eq!(c["provenance"]["author"]["name"], "aa");
    assert_eq!(c["on_behalf"], "ana");
    assert_eq!(c["strictness"], "cites_numbers");
    // The entitled person reads it in the capsule answer.
    let r = h.send(
        "a",
        json!({"type":"query","id":"q","query":{"kind":"capsule","capsule":CAP}}),
    );
    assert_eq!(r[0]["msg"]["data"]["detail"]["comments"][0]["id"], c["id"]);
}

#[test]
fn bad_comments_are_rejected_with_machine_readable_reasons() {
    let mut h = H::new();
    let before = h.0.log().len();
    let r = h.comment("A", "THYAO 5,87 seviyesinde [1].", json!([CAP]));
    assert_eq!(r[0]["msg"]["error"]["code"], "comment_rejected", "{r:?}");
    assert_eq!(
        reasons(&r[0]),
        vec!["unsupported_number".to_owned()],
        "{r:?}"
    );
    let r = h.comment("A", "THYAO'yu şimdi almak için fırsat [1].", json!([CAP]));
    assert_eq!(r[0]["msg"]["error"]["code"], "comment_rejected", "{r:?}");
    assert!(
        r[0]["msg"]["error"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["check"] == "advice"),
        "{r:?}"
    );
    let r = h.comment("A", "Kısa ortalamanın altında.", json!(["cap.nope"]));
    assert!(reasons(&r[0]).contains(&"unknown_id".to_owned()), "{r:?}");
    let r = h.comment("A", "Kısa ortalamanın altında.", json!([]));
    assert!(reasons(&r[0]).contains(&"no_cites".to_owned()), "{r:?}");
    assert_eq!(h.0.log().len(), before, "nothing stored");
}

#[test]
fn an_agent_for_an_unentitled_person_cannot_carry_a_licensed_number() {
    let mut h = H::new();
    // `can` reads THYAO without digits; 5,23 is the authority's number.
    let r = h.comment(
        "C",
        "THYAO kısa ortalamanın (5,23) altında [1].",
        json!([CAP]),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "comment_rejected", "{r:?}");
    // The writer is told an unsupported number (no oracle); the host's audit has the
    // licence catch.
    let licensed = r[0]["msg"].to_string();
    let a = h.0.comment_audit().last().unwrap();
    assert!(!a.accepted);
    assert!(
        a.reasons.iter().any(|x| x.code == "licensed_value"),
        "{a:?}"
    );
    // The same draft with a made-up number: the same bytes.
    let r = h.comment(
        "C",
        "THYAO kısa ortalamanın (5,87) altında [1].",
        json!([CAP]),
    );
    assert_eq!(r[0]["msg"].to_string(), licensed);
    assert!(
        !licensed.contains("5,23") && !licensed.contains("licensed"),
        "{licensed}"
    );
    // Not even as an external quote.
    let r = h.send(
        "C",
        json!({"type":"command","id":"k","ops":[{"op":"add_comment","about":{"capsule":CAP},
            "lang":"tr","text":"Bir kaynağa göre 5,23 [1].","cites":[CAP],
            "external":[{"source":"x","quote":"5,23","values":[5.23]}]}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "comment_rejected", "{r:?}");
    // A comment without numbers passes.
    let r = h.comment("C", "THYAO'da düşüş eğilimi sürüyor [1].", json!([CAP]));
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
}

#[test]
fn a_reader_receives_only_the_comments_it_may_see() {
    let mut h = H::new();
    for c in ["a", "c"] {
        h.send(
            c,
            json!({"type":"subscribe","id":"cm","topic":{"kind":"comments","instrument":"THYAO"}}),
        );
    }
    let out = h.comment(
        "A",
        "THYAO kısa ortalamanın (5,23) altında kalıyor [1].",
        json!([CAP]),
    );
    assert_eq!(out[0]["msg"]["status"], "applied", "{out:?}");
    let ev: Vec<&Value> = out
        .iter()
        .filter(|l| l["msg"]["event"]["kind"] == "comment")
        .collect();
    assert_eq!(ev.len(), 1, "{out:?}");
    assert_eq!(
        ev[0]["conn"], "a",
        "the unentitled person gets no number: {out:?}"
    );
    let r = h.send(
        "c",
        json!({"type":"query","id":"q","query":{"kind":"capsule","capsule":CAP}}),
    );
    assert!(
        r[0]["msg"]["data"]["detail"].get("comments").is_none(),
        "{r:?}"
    );
    let all = serde_json::to_string(&h.send(
        "c",
        json!({"type":"query","id":"d","query":{"kind":"document"}}),
    ))
    .unwrap();
    assert!(
        !all.contains("5,23) altında kalıyor"),
        "document leaks the comment"
    );
    // A number-free comment reaches both.
    let out = h.comment("A", "THYAO'da düşüş eğilimi sürüyor [1].", json!([CAP]));
    let n = out
        .iter()
        .filter(|l| l["msg"]["event"]["kind"] == "comment")
        .count();
    assert_eq!(n, 2, "{out:?}");
}

#[test]
fn a_hidden_id_reads_like_a_missing_one_for_a_restricted_writer() {
    let mut h = H::new();
    // A capsule of the authority's world that `can` may not see (licensed flow).
    let ids: Vec<String> =
        h.0.document()
            .capsules
            .iter()
            .map(|c| c.id.clone())
            .collect();
    let hidden = ids
        .into_iter()
        .find(|id| {
            let r = h.send(
                "c",
                json!({"type":"query","id":"q","query":{"kind":"capsule","capsule":id}}),
            );
            r[0]["msg"]["error"]["code"] == "not_found"
        })
        .expect("a capsule hidden from can");
    let a = h.comment("C", "Düşüş eğilimi [1].", json!([hidden]));
    let b = h.comment("C", "Düşüş eğilimi [1].", json!(["cap.THYAO.none.1"]));
    let strip = |v: &Value| {
        v["msg"]
            .to_string()
            .replace(&hidden, "X")
            .replace("cap.THYAO.none.1", "X")
    };
    assert_eq!(strip(&a[0]), strip(&b[0]));
    // `about` a hidden capsule: `not_found` like a missing one, suggestions from the
    // visible ones only.
    let about = |h: &mut H, cap: &str| {
        h.send(
            "C",
            json!({"type":"command","id":"k","ops":[{"op":"add_comment","about":{"capsule":cap},
                "lang":"tr","text":"Düşüş eğilimi [1].","cites":[CAP]}]}),
        )
    };
    let a = about(&mut h, &hidden);
    let b = about(&mut h, "cap.THYAO.none.1");
    assert_eq!(a[0]["msg"]["error"]["code"], "not_found", "{a:?}");
    let dym = a[0]["msg"]["error"]["did_you_mean"].to_string();
    assert!(!dym.contains(&hidden), "{dym}");
    assert_eq!(a[0]["msg"]["error"]["code"], b[0]["msg"]["error"]["code"]);
}

#[test]
fn a_reader_without_the_permission_never_reaches_the_validator() {
    let mut h = H::new();
    let n = h.0.comment_audit().len();
    // `can` (a viewer) may neither write nor propose.
    let r = h.comment("c", "THYAO 5,23 [1].", json!([CAP]));
    assert_eq!(r[0]["msg"]["error"]["code"], "forbidden", "{r:?}");
    assert_eq!(h.0.comment_audit().len(), n);
}

#[test]
fn a_preview_draws_only_what_the_asker_may_read() {
    let mut h = H::new();
    // ana's agent proposes the presenter's view of a finding (bound THYAO bars).
    let r = h.send(
        "A",
        json!({"type":"command","id":"p","ops":[{"op":"show_finding","finding":"fnd.THYAO.continuation.down.84","view":"v-show"}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "proposed", "{r:?}");
    let pid = r[0]["msg"]["proposal"].as_str().unwrap().to_owned();
    let q = json!({"type":"query","id":"v","query":{"kind":"preview","proposal":pid}});
    let ana = h.send("a", q.clone());
    let ana_text = ana[0]["msg"]["data"]["views"][0]["explain"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(ana_text.contains("candle px"), "{ana_text}");
    // can's agent and can: the bars are not readable; no value is described.
    for c in ["C", "c"] {
        let r = h.send(c, q.clone());
        let views = &r[0]["msg"]["data"]["views"];
        let text = views[0]["explain"].as_str().unwrap_or_default();
        assert!(text.contains("not readable by you"), "{r:?}");
        assert!(!text.contains("last c="), "{text}");
    }
}

#[test]
fn the_writer_finds_its_comment_by_list_and_get() {
    let mut h = H::new();
    let out = h.comment(
        "A",
        "THYAO kısa ortalamanın (5,23) altında kalıyor [1].",
        json!([CAP]),
    );
    assert_eq!(out[0]["msg"]["status"], "applied", "{out:?}");
    let id = h.0.document().comments.last().unwrap().id.clone();
    for conn in ["A", "a"] {
        let l = h.send(
            conn,
            json!({"type":"query","id":"l","query":{"kind":"list","collection":"comments"}}),
        );
        let ids: Vec<String> = l[0]["msg"]["data"]["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| c["id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        assert!(ids.contains(&id), "{conn} list: {l:?}");
        let g = h.send(
            conn,
            json!({"type":"query","id":"g","query":{"kind":"get","id":id}}),
        );
        assert_eq!(
            g[0]["msg"]["data"]["item"]["id"],
            id.as_str(),
            "{conn} get: {g:?}"
        );
    }
}
