//! The two worlds (licensed flow) and reader projection (`as_reader`): arkin (owner) and
//! ana (viewer) are entitled to everything, bora to the bars only, can to nothing; agents
//! aa (for ana) and cc (for can) under policy `r5b`. Every reader subscribes to every
//! topic; commands from entitled and public-view writers (sequence mapping), data events,
//! comments, undo; then the same reads by every reader (withheld licensed items).

use serde_json::{json, Value};

use super::order::{data_steps, Run};
use super::rig::{init_of, Rig};
use super::snap::Book;

const CAP: &str = "cap.THYAO.continuation.down.84";

fn init() -> Value {
    let mut v = init_of("39-capsule-entitlement");
    let extra = init_of("67-comment");
    for p in extra["principals"].as_array().unwrap() {
        if p["author"]["kind"] == "agent" {
            v["principals"].as_array_mut().unwrap().push(p.clone());
        }
    }
    v["agent_policy"] = json!("r5b");
    v
}

const READERS: [(&str, &str); 6] = [
    ("o", "arkin"),
    ("a", "ana"),
    ("b", "bora"),
    ("c", "can"),
    ("A", "aa"),
    ("C", "cc"),
];

#[test]
fn charac_session_worlds_flow() {
    let mut book = Book::new("worlds");
    let mut r = Run::new(&mut book, "w", Rig::json(init()));
    for (c, who) in READERS {
        r.hello(c, who, "headless");
    }
    for (c, _) in READERS {
        r.sub(c, "ch", json!({"kind":"changes"}));
        r.sub(c, "ka", json!({"kind":"capsules","all":true}));
        r.sub(c, "cl", json!({"kind":"claims"}));
        r.sub(c, "st", json!({"kind":"stories","instrument":"THYAO"}));
        r.sub(c, "cm", json!({"kind":"comments","instrument":"THYAO"}));
        r.sub(c, "in", json!({"kind":"intents"}));
    }
    // Writers in both worlds.
    let aud = json!({"group":"future","sensitivity":"important","watchlist":["THYAO"]});
    r.cmd(
        "owner_intent",
        "o",
        json!([{"op":"add_intent","id":"io","text":"sahip"}]),
    );
    r.cmd(
        "bora_aud",
        "b",
        json!([{"op":"set_audience","audience":aud}]),
    );
    r.cmd(
        "can_sens",
        "c",
        json!([{"op":"set_sensitivity","sensitivity":"all"}]),
    );
    r.cmd(
        "ana_watch",
        "a",
        json!([{"op":"add_watch","text":"ana","match":{"instrument":"THYAO"}}]),
    );
    r.cmd(
        "can_watch",
        "c",
        json!([{"op":"add_watch","text":"can","match":{"instrument":"THYAO"}}]),
    );
    for (i, ev) in data_steps("39-capsule-entitlement").iter().enumerate() {
        r.data(&format!("bar{i}"), ev);
    }
    // Comments: an entitled agent, an unentitled one, a licensed number.
    let cm = |text: &str| json!([{"op":"add_comment","about":{"capsule":CAP},"cites":[CAP],"lang":"tr","text":text}]);
    r.cmd(
        "aa_comment",
        "A",
        cm("THYAO kısa ortalamanın altında kalıyor [1]."),
    );
    r.cmd("cc_comment", "C", cm("THYAO'da düşüş eğilimi sürüyor [1]."));
    r.cmd(
        "cc_number",
        "C",
        cm("THYAO kısa ortalamanın (5,23) altında [1]."),
    );
    r.cmd(
        "aa_advice",
        "A",
        cm("THYAO'yu şimdi almak için fırsat [1]."),
    );
    r.cmd(
        "owner_after",
        "o",
        json!([{"op":"add_intent","id":"io2","text":"sonra"}]),
    );
    r.cmd("bora_undo", "b", json!([{"op":"undo"}]));
    r.cmd("owner_undo", "o", json!([{"op":"undo"}]));
    // Agent allow-list under r5b.
    r.cmd("aa_fact", "A", json!([{"op":"add_intent","text":"x"}]));
    r.cmd(
        "aa_view",
        "A",
        json!([{"op":"put_view","view":{"id":"va","x":{"kind":"time"},
        "y":[{"kind":"linear"}],"layers":[{"mark":"candle","series":"px","panel":0}]}}]),
    );
    r.cmd(
        "aa_view_direct_ok",
        "o",
        json!([{"op":"put_view","view":{"id":"vo","x":{"kind":"time"},
        "y":[{"kind":"linear"}],"layers":[{"mark":"candle","series":"px","panel":0}]}}]),
    );
    // The same reads by every reader.
    let qs = [
        ("q_log", json!({"kind":"log"})),
        ("q_caps", json!({"kind":"capsules","since_bar":80})),
        ("q_cap", json!({"kind":"capsule","capsule":CAP})),
        (
            "q_flow",
            json!({"kind":"capsule","capsule":"cap.THYAO.flow.up.90"}),
        ),
        ("q_ledger", json!({"kind":"ledger","by_kind":true})),
        (
            "q_stories",
            json!({"kind":"stories","instrument":"THYAO","range":{"last":10}}),
        ),
        ("q_intents", json!({"kind":"list","collection":"intents"})),
        ("q_comments", json!({"kind":"list","collection":"comments"})),
        ("q_doc", json!({"kind":"document"})),
        ("q_describe", json!({"kind":"describe"})),
    ];
    for (c, _) in READERS {
        for (l, q) in &qs {
            r.query(&format!("{l}_{c}"), c, q.clone());
        }
    }
    r.finish();
    book.check();
}

/// The same reader projection with no licensed flow at all (one world): what the
/// public-view readers get changes (claims on document capsules, authority seqs).
#[test]
fn charac_session_worlds_single() {
    let mut book = Book::new("worlds-single");
    let mut v = init();
    v["capsules"]["flow"] = json!([]);
    let mut r = Run::new(&mut book, "s", Rig::json(v));
    for (c, who) in READERS {
        r.hello(c, who, "headless");
        r.sub(c, "ch", json!({"kind":"changes"}));
        r.sub(c, "ka", json!({"kind":"capsules","all":true}));
        r.sub(c, "cl", json!({"kind":"claims"}));
    }
    r.cmd(
        "owner_intent",
        "o",
        json!([{"op":"add_intent","id":"io","text":"sahip"}]),
    );
    r.cmd(
        "bora_aud",
        "b",
        json!([{"op":"set_audience","audience":{"group":"future","watchlist":["THYAO"]}}]),
    );
    for (i, ev) in data_steps("39-capsule-entitlement").iter().enumerate() {
        r.data(&format!("bar{i}"), ev);
    }
    for (c, _) in READERS {
        r.query(&format!("q_log_{c}"), c, json!({"kind":"log"}));
        r.query(
            &format!("q_caps_{c}"),
            c,
            json!({"kind":"capsules","since_bar":80}),
        );
    }
    r.finish();
    book.check();
}
