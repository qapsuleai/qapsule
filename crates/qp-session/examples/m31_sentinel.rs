//! (R5B/M31) The sentinel's notification filter over the M31 window.
//!
//! Sets up the harness session (`session/init.r5b.json`) on a [`ReplayHost`], adds the
//! person's three watches (`m31/watches.json`) from the agent's (`claude`, on behalf of
//! `arkin`) marked connection with `add_watch`, subscribes it to `intents` and `capsules`
//! (`all: true`; a second agent connection to `capsules` without `all` is the feed the
//! sentinel's `in_feed` is checked against), steps the window day by day **as the host**
//! (`ReplayHost::host_control`, not the agent), collects the events and applies the three
//! filters of `qp_capsule::sentinel`. Writes:
//!
//! - `<out>/events.jsonl`: every collected event (`{day, conn, event}`), raw;
//! - `<out>/<filter>/notifications.jsonl`: `{day, capsule, stage, text}` per line;
//! - `<out>/check.json`: the sentinel's `in_feed` against the engine's feed subscription.
//!
//! Deterministic: no model, no network, no clock; two runs give byte-equal files.
//!
//! ```text
//! cargo run -p qp-session --release --example m31_sentinel -- <out> [harness]
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use qp_capsule::sentinel::{
    apply, Filter, SentinelAudience, SentinelEvent, Transition, WatchMatch,
};
use qp_schema::capsule::{Audience, Stage};
use qp_schema::protocol::{ReplayControl, SessionInit};
use qp_session::ReplayHost;
use serde_json::{json, Value};

/// The agent's connection (`all: true` capsules + intents) and its feed connection.
const C: &str = "c-claude";
const F: &str = "f-claude";

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("usage: m31_sentinel <out> [harness]")?);
    let harness = args.next().map_or_else(
        || root.join("rounds/R5B/experiments/harness"),
        PathBuf::from,
    );
    let init_v: Value = qp_session::read_json(&harness.join("session/init.r5b.json"))?;
    let pos: Value = qp_session::read_json(&harness.join("session/position.json"))?;
    let watches: Value = qp_session::read_json(&harness.join("m31/watches.json"))?;
    let steps = pos["m31_steps"]
        .as_u64()
        .ok_or("position.json: m31_steps")?;
    if !pos["position"].is_null() {
        return Err("position.json: only the fixed position (null) is supported".into());
    }
    let init: SessionInit =
        serde_json::from_value(init_v.clone()).map_err(|e| format!("init: {e}"))?;
    let mut host = ReplayHost::new(init, &root)?;

    // The agent: marked connection, on behalf of arkin.
    for conn in [C, F] {
        host.session_mut().mark_agent_connection(conn);
        let r = first_reply(
            &mut host,
            conn,
            &json!({"type":"hello","protocol":"qp/0","principal":"claude","surface":{"kind":"headless"}}),
        )?;
        if r["type"] != "welcome" {
            return Err(format!("hello {conn}: {r}"));
        }
    }
    let ws = watches["watches"]
        .as_array()
        .ok_or("watches.json: watches")?;
    for (i, w) in ws.iter().enumerate() {
        let cmd = json!({"type":"command","id":format!("w{}", i + 1),
            "ops":[{"op":"add_watch","text":w["text"],"match":w["match"]}]});
        let r = first_reply(&mut host, C, &cmd)?;
        if r["type"] != "ack" {
            return Err(format!("add_watch {i}: {r}"));
        }
    }
    for (conn, id, topic) in [
        (C, "s-int", json!({"kind":"intents"})),
        (C, "s-cap", json!({"kind":"capsules","all":true})),
        (F, "s-feed", json!({"kind":"capsules"})),
    ] {
        let r = first_reply(
            &mut host,
            conn,
            &json!({"type":"subscribe","id":id,"topic":topic}),
        )?;
        if r["type"] == "error" {
            return Err(format!("subscribe {id}: {r}"));
        }
    }
    // Watch texts by id (the person's watches, as the authority stored them).
    let texts: BTreeMap<String, String> = host
        .session()
        .document()
        .intents
        .iter()
        .filter(|i| i.watch.is_some())
        .map(|i| (i.id.clone(), i.text.clone()))
        .collect();
    let audience = sentinel_audience(&init_v, ws)?;

    // Step the window as the host.
    let mut raw = Vec::new();
    let mut events = Vec::new();
    let mut feed: BTreeSet<(u32, String, String)> = BTreeSet::new();
    for day in 1..=u32::try_from(steps).map_err(|e| e.to_string())? {
        let (_, lines) = host
            .host_control(&ReplayControl::Step { n: Some(1) })
            .map_err(|e| format!("step {day}: {e:?}"))?;
        for l in lines {
            if l.conn != C && l.conn != F {
                continue;
            }
            let m = serde_json::to_value(&l.msg).map_err(|e| e.to_string())?;
            if m["type"] != "notify" {
                continue;
            }
            let ev = &m["event"];
            raw.push(json!({"day": day, "conn": l.conn, "event": ev}));
            match (l.conn.as_str(), ev["kind"].as_str()) {
                (C, Some("capsule")) => {
                    events.push(SentinelEvent::Transition(transition(day, ev)?))
                }
                (C, Some("intent_matched")) => {
                    let intent = str_of(ev, "intent")?;
                    events.push(SentinelEvent::Match(WatchMatch {
                        day,
                        text: texts.get(&intent).cloned(),
                        intent,
                        capsule: str_of(ev, "capsule")?,
                        stage: from(&ev["stage"])?,
                    }));
                }
                (F, Some("capsule")) => {
                    feed.insert((day, str_of(&ev["capsule"], "id")?, stage_of(ev)?));
                }
                _ => {}
            }
        }
    }

    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    write_lines(&out.join("events.jsonl"), &raw)?;
    for f in Filter::ALL {
        let rows: Vec<Value> = apply(f, &events, &audience)
            .into_iter()
            .map(|n| json!({"day": n.day, "capsule": n.capsule, "stage": n.stage, "text": n.text}))
            .collect();
        let dir = out.join(f.name());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        write_lines(&dir.join("notifications.jsonl"), &rows)?;
    }
    // The sentinel's feed rule against the engine's feed subscription.
    let mine: BTreeSet<(u32, String, String)> = events
        .iter()
        .filter_map(|e| match e {
            SentinelEvent::Transition(t) if audience.in_feed(t) => {
                Some((t.day, t.capsule.clone(), stage_name(t.stage)))
            }
            _ => None,
        })
        .collect();
    let only = |a: &BTreeSet<(u32, String, String)>, b: &BTreeSet<(u32, String, String)>| {
        a.difference(b).cloned().collect::<Vec<_>>()
    };
    let check = json!({"transitions": events.iter().filter(|e| matches!(e, SentinelEvent::Transition(_))).count(),
        "matches": events.iter().filter(|e| matches!(e, SentinelEvent::Match(_))).count(),
        "engine_feed": feed.len(), "sentinel_feed": mine.len(),
        "only_engine": only(&feed, &mine), "only_sentinel": only(&mine, &feed)});
    write_lines(&out.join("check.json"), &[check])?;
    Ok(())
}

fn sentinel_audience(init: &Value, watches: &[Value]) -> Result<SentinelAudience, String> {
    let a: Audience = serde_json::from_value(init["document"]["audiences"]["arkin"].clone())
        .map_err(|e| format!("audience: {e}"))?;
    Ok(SentinelAudience {
        positions: a
            .positions
            .iter()
            .filter(|(_, q)| **q != 0.0)
            .map(|(s, _)| s.clone())
            .collect(),
        watchlist: a.watchlist.iter().cloned().collect(),
        intents_on: watches
            .iter()
            .filter_map(|w| w["match"]["instrument"].as_str().map(str::to_owned))
            .collect(),
        sensitivity: a.effective(),
    })
}

fn transition(day: u32, ev: &Value) -> Result<Transition, String> {
    let c = &ev["capsule"];
    Ok(Transition {
        day,
        capsule: str_of(c, "id")?,
        instrument: str_of(c, "instrument")?,
        kind: from(&c["kind"])?,
        direction: from(&c["direction"])?,
        stage: from(&ev["transition"]["stage"])?,
        tier: c["tier"]
            .as_u64()
            .and_then(|t| u8::try_from(t).ok())
            .ok_or("capsule: tier")?,
        relevant: c["relevant"].as_bool().unwrap_or(false),
        coin_flip: c["coin_flip"].as_bool().unwrap_or(false),
    })
}

fn stage_of(ev: &Value) -> Result<String, String> {
    Ok(stage_name(from::<Stage>(&ev["transition"]["stage"])?))
}

fn stage_name(s: Stage) -> String {
    serde_json::to_value(s)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn from<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("{v}: {e}"))
}

fn str_of(v: &Value, k: &str) -> Result<String, String> {
    v[k].as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("missing `{k}` in {v}"))
}

fn write_lines(path: &Path, rows: &[Value]) -> Result<(), String> {
    let mut f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for r in rows {
        writeln!(f, "{r}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn first_reply(host: &mut ReplayHost, conn: &str, msg: &Value) -> Result<Value, String> {
    host.handle_raw(conn, &msg.to_string())
        .into_iter()
        .find(|l| l.conn == conn)
        .map(|l| serde_json::to_value(&l.msg).unwrap_or(Value::Null))
        .ok_or_else(|| format!("no reply on {conn}"))
}
