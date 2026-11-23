//! The exact order of outbound messages per moment, across several connections with
//! different roles and principals, every one subscribed to every topic it may read:
//! command → ack → change → finding/condition/intent/proposal events; data events (bars,
//! settle, live path, claims before intents, view changed); the clock (proposal expiry);
//! subscribe/unsubscribe; connection open (hello); presence (input, attend); queries.

use serde_json::{json, Value};

use super::rig::{init_of, transcript, Rig};
use super::snap::Book;

/// Steps of a scenario, each recorded as one case `<scenario>.<nn>.<label>`.
pub struct Run<'a> {
    pub book: &'a mut Book,
    pub name: &'static str,
    pub rig: Rig,
    n: usize,
}

impl<'a> Run<'a> {
    pub fn new(book: &'a mut Book, name: &'static str, rig: Rig) -> Self {
        Self {
            book,
            name,
            rig,
            n: 0,
        }
    }

    pub fn rec(&mut self, label: &str, out: &[Value]) {
        self.book
            .lines(&format!("{}.{:02}.{label}", self.name, self.n), out);
        self.n += 1;
    }

    pub fn send(&mut self, label: &str, conn: &str, msg: Value) {
        let out = self.rig.send(conn, &msg);
        self.rec(label, &out);
    }

    pub fn cmd(&mut self, label: &str, conn: &str, ops: Value) {
        let out = self.rig.cmd(conn, label, ops);
        self.rec(label, &out);
    }

    pub fn data(&mut self, label: &str, ev: &Value) {
        let out = self.rig.data(ev);
        self.rec(label, &out);
    }

    pub fn hello(&mut self, conn: &str, who: &str, kind: &str) {
        let out = self.rig.hello(conn, who, kind);
        self.rec(&format!("hello_{conn}"), &out);
    }

    pub fn sub(&mut self, conn: &str, id: &str, topic: Value) {
        let out = self.rig.sub(conn, id, topic);
        self.rec(&format!("sub_{conn}_{id}"), &out);
    }

    pub fn query(&mut self, label: &str, conn: &str, q: Value) {
        let out = self.rig.query(conn, label, q);
        self.rec(label, &out);
    }

    pub fn finish(self) {
        let (d, s) = self.rig.fin();
        self.book.put(&format!("{}.final", self.name), &d, &s);
    }
}

/// The data events a transcript delivers, in order.
pub fn data_steps(name: &str) -> Vec<Value> {
    transcript(name)["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s.get("data").cloned())
        .collect()
}

const T0: i64 = 1_760_000_000_000;

#[test]
fn charac_session_order_commands() {
    let mut book = Book::new("order-commands");
    let mut init = init_of("06-proposal-reject-expire");
    init["principals"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"viewer","author":{"kind":"human","name":"viewer"},"role":"viewer"}));
    let mut r = Run::new(&mut book, "cmd", Rig::json(init));
    // Open: before hello, a bad hello, then four principals (and arkin twice).
    r.send(
        "before_hello",
        "x",
        json!({"type":"query","id":"q","query":{"kind":"describe"}}),
    );
    r.hello("a", "arkin", "desktop");
    r.hello("a2", "arkin", "phone");
    r.hello("v", "viewer", "phone");
    r.hello("s", "scout", "headless");
    r.hello("e", "editor", "headless");
    r.hello("h", "host", "headless");
    let cond = json!({"kind":"threshold","series":"px","op":"lt","value":100.0});
    for c in ["a", "a2", "v", "s", "e"] {
        r.sub(c, "ch", json!({"kind":"changes"}));
        r.sub(
            c,
            "fi",
            json!({"kind":"finding","finding":"fnd-overbought"}),
        );
        r.sub(c, "co", json!({"kind":"condition","condition":cond}));
        r.sub(c, "pr", json!({"kind":"proposals"}));
        r.sub(c, "in", json!({"kind":"intents"}));
        r.sub(c, "vw", json!({"kind":"view","view":"main"}));
        r.sub(c, "at", json!({"kind":"attention","principal":"arkin"}));
    }
    r.sub("v", "ids", json!({"kind":"changes","ids":["rsi14"]}));
    r.sub("v", "bad", json!({"kind":"finding","finding":"nope"}));
    r.sub("v", "ch", json!({"kind":"changes"}));
    // Commands.
    r.cmd(
        "intent",
        "a",
        json!([{"op":"add_intent","id":"i-low","text":"düşerse","trigger":cond}]),
    );
    r.cmd(
        "inval",
        "a",
        json!([{"op":"update_finding","finding":"fnd-overbought","invalidation":cond}]),
    );
    r.cmd(
        "params",
        "e",
        json!([{"op":"set_params","series":"rsi14","params":{"period":21}}]),
    );
    r.cmd(
        "prop1",
        "s",
        json!([{"op":"set_params","series":"rsi14","params":{"period":9}}]),
    );
    r.cmd("prop2", "s", json!([{"op":"add_intent","text":"öneri"}]));
    r.cmd(
        "accept",
        "a",
        json!([{"op":"accept_proposal","proposal":"prop-4"}]),
    );
    r.cmd("viewer_write", "v", json!([{"op":"add_intent","text":"x"}]));
    // The clock: nothing, then proposal expiry.
    r.send("clock_30s", "h", json!({"type":"clock","now":T0 + 30_000}));
    r.cmd(
        "prop3",
        "s",
        json!([{"op":"add_intent","text":"süresi dolacak"}]),
    );
    r.send("clock_61s", "h", json!({"type":"clock","now":T0 + 61_000}));
    r.send(
        "clock_200s",
        "h",
        json!({"type":"clock","now":T0 + 200_000}),
    );
    r.send("clock_back", "h", json!({"type":"clock","now":T0}));
    r.send(
        "clock_by_owner",
        "a",
        json!({"type":"clock","now":T0 + 300_000}),
    );
    // The live path: an appended bar under 100 invalidates the capsule, meets the
    // condition and triggers the intent.
    r.cmd(
        "bar_low",
        "a",
        json!([{"op":"append_bars","series":"px","bars":[
        {"t":1_759_968_000_000_i64,"o":124.0,"h":124.5,"l":98.0,"c":99.0,"v":3_000_000.0}]}]),
    );
    r.cmd(
        "bar_last",
        "a",
        json!([{"op":"update_last_bar","series":"px","bar":
        {"t":1_759_968_000_000_i64,"o":124.0,"h":124.5,"l":97.0,"c":98.0,"v":3_100_000.0}}]),
    );
    r.cmd("undo_bar", "a", json!([{"op":"undo"}]));
    // Presence: input, attend.
    r.send("look", "a", json!({"type":"input","id":"g1","gesture":{"kind":"look","view":"main","from_bar":100,"to_bar":199}}));
    r.send(
        "zoom",
        "a2",
        json!({"type":"input","id":"g2","gesture":{"kind":"zoom","view":"main","factor":2.0}}),
    );
    r.send("tap", "a", json!({"type":"input","id":"g3","gesture":{"kind":"tap","view":"main","panel":1,"bar":150}}));
    r.send("attend", "s", json!({"type":"attend","id":"at1","directive":{"to":"arkin","steps":[
        {"target":{"view":"main","panel":1,"series":"rsi14","from_bar":150,"to_bar":150},"say":"burada"}]}}));
    r.send(
        "attend_bad",
        "s",
        json!({"type":"attend","id":"at2","directive":{"to":"nobody","steps":[]}}),
    );
    // Unsubscribe: the next change skips that subscription.
    r.send(
        "unsub",
        "v",
        json!({"type":"unsubscribe","id":"u","sub":"ch"}),
    );
    r.send(
        "unsub_again",
        "v",
        json!({"type":"unsubscribe","id":"u","sub":"ch"}),
    );
    r.cmd(
        "after_unsub",
        "a",
        json!([{"op":"set_params","series":"rsi14","params":{"period":14}}]),
    );
    // Queries and reads.
    r.query("q_log", "e", json!({"kind":"log","after_seq":3}));
    r.query(
        "q_list",
        "v",
        json!({"kind":"list","collection":"proposals"}),
    );
    r.query(
        "q_attention",
        "s",
        json!({"kind":"attention","principal":"arkin"}),
    );
    r.query(
        "q_screen",
        "v",
        json!({"kind":"screen","principal":"arkin"}),
    );
    r.query("q_bad", "v", json!({"kind":"get","id":"nope"}));
    r.send("raw_bad", "v", json!({"type":"nope"}));
    r.finish();
    book.check();
}

/// Transcript 69's world (agent policy `r5b`, capsule engine on a bound synthetic feed,
/// ana with her agent bot, eve): every reader subscribed to every topic; watches and
/// comments; then every data event of the transcript, each one a case.
#[test]
fn charac_session_order_data() {
    let mut book = Book::new("order-data");
    let mut r = Run::new(&mut book, "data", Rig::of("69-watch"));
    r.hello("a", "ana", "phone");
    r.hello("b", "bot", "headless");
    r.hello("e", "eve", "headless");
    r.hello("h", "host", "headless");
    for c in ["a", "b", "e", "h"] {
        r.sub(c, "ch", json!({"kind":"changes"}));
        r.sub(c, "ka", json!({"kind":"capsules","all":true}));
        r.sub(c, "ki", json!({"kind":"capsules","instrument":"THYAO"}));
        r.sub(c, "cl", json!({"kind":"claims"}));
        r.sub(c, "in", json!({"kind":"intents"}));
        r.sub(c, "st", json!({"kind":"stories","instrument":"THYAO"}));
        r.sub(c, "cm", json!({"kind":"comments","instrument":"THYAO"}));
        r.sub(c, "pr", json!({"kind":"proposals"}));
    }
    r.cmd(
        "w_bot",
        "b",
        json!([{"op":"add_watch","text":"her şey","match":{"instrument":"THYAO"},"max_matches":3}]),
    );
    r.cmd(
        "w_neg",
        "b",
        json!([{"op":"add_watch","text":"olumsuz","match":{"instrument":"THYAO","negative":true}}]),
    );
    r.cmd(
        "w_eve",
        "e",
        json!([{"op":"add_watch","text":"eve","match":{"instrument":"THYAO"}}]),
    );
    r.cmd("w_exp", "a", json!([{"op":"add_watch","text":"süreli","match":{"instrument":"THYAO"},"expires_t":T0 + 1}]));
    let evs = data_steps("69-watch");
    assert!(evs.len() >= 20, "{}", evs.len());
    for (i, ev) in evs.iter().enumerate() {
        r.data(&format!("bar{i}"), ev);
        if i == 3 {
            r.send("clock", "h", json!({"type":"clock","now":T0 + 10}));
        }
        if i == 6 {
            r.send(
                "unsub_cl",
                "a",
                json!({"type":"unsubscribe","id":"u","sub":"cl"}),
            );
        }
    }
    r.query(
        "q_intents",
        "a",
        json!({"kind":"list","collection":"intents"}),
    );
    r.query(
        "q_intents_eve",
        "e",
        json!({"kind":"list","collection":"intents"}),
    );
    r.query("q_log", "b", json!({"kind":"log","after_seq":2}));
    r.finish();
    book.check();
}

/// Transcript 33's world (two bound feeds and an inline series; `view` and `condition`
/// subscriptions): live revisions, new bars, corrections and an inline append, with
/// every reader subscribed to everything.
#[test]
fn charac_session_order_live_path() {
    let mut book = Book::new("order-live");
    let mut r = Run::new(&mut book, "live", Rig::of("33-tick-wakes-affected"));
    let t = transcript("33-tick-wakes-affected");
    // The transcript's own sends first (hellos, subscriptions), then more subscriptions.
    let mut steps: Vec<Value> = t["steps"].as_array().unwrap().clone();
    let first_data = steps.iter().position(|s| s.get("data").is_some()).unwrap();
    for s in steps.drain(..first_data) {
        let m = &s["send"];
        let out = r.rig.send(m["conn"].as_str().unwrap(), &m["msg"]);
        r.rec("pre", &out);
    }
    for c in ["a", "b"] {
        r.sub(c, "all", json!({"kind":"changes"}));
        r.sub(c, "cl", json!({"kind":"claims"}));
    }
    for (i, s) in steps.iter().enumerate() {
        if let Some(ev) = s.get("data") {
            r.data(&format!("ev{i}"), ev);
        } else if let Some(m) = s.get("send") {
            let out = r.rig.send(m["conn"].as_str().unwrap(), &m["msg"]);
            r.rec(&format!("send{i}"), &out);
        }
    }
    // A data event the host refuses (seq gap / unknown feed): its text.
    let bad: qp_schema::data::DataEvent =
        serde_json::from_value(json!({"feed":{"symbol":"NOPE","interval":"1d"},
        "seq":1,"change":{"kind":"patch","t":1,"fields":{"c":1.0}}}))
        .unwrap();
    let got = match r.rig.data_try(&bad) {
        Ok(v) => format!("accepted, {} line(s)", v.len()),
        Err(e) => format!("refused: {e}"),
    };
    r.book.note("live.refused", &got);
    r.finish();
    book.check();
}
