//! Transcripts 71 (add), 72 (check), 73 (try), 76 (an agent's proposal).

use serde_json::{json, Value};

use super::{
    add, clock, cmd, hello, ir, q, reply, run, thyao, transcript, In, HEAVY, HEAVY_TRY,
    SIGNAL_STATE, SMA,
};

pub fn t71() -> Value {
    let sig = json!([{"op":"add_program","id":"s1","program":ir(SIGNAL_STATE),"inputs":["px"]}]);
    let steps = vec![
        hello("a", "arkin", "A person (owner)."),
        hello("h", "host", "The host (moves the clock)."),
        cmd("a", "c1", None, json!([add("p1", SMA, true)]), "add_program: one `put_series` per output (`p1.avg`, `p1.up`), then one layer per output; `ack.created` lists the series, then the layers."),
        q("a", "q1", json!({"kind":"get","id":"p1.avg"}), "The output series: `study: program`, the canonical IR inline, `owner`, `since_t` = the expansion's clock, the full parameter set."),
        q("a", "q2", json!({"kind":"study","series":"p1.up"}), "The event series: 0/1 on closed bars, `null` on the live bar (K13)."),
        cmd("a", "c2", None, json!([{"op":"add_program","program":ir(SMA),"inputs":["px"],"params":{"len":9}}]), "The same program again, no id (`prog-<seq>-0`), other values: a second instance."),
        q("a", "q3", json!({"kind":"list","collection":"series"}), "Both instances: same `program_id`, different `instance_id`."),
        cmd("a", "c3", None, json!([add("p1", SMA, false)]), "An existing instance id: `duplicate`."),
        clock(1_760_000_100_000, "Later."),
        cmd("a", "c4", None, json!([{"op":"set_program_params","program":"p1","params":{"len":7}}]), "New values: the same series ids and layers, a new `instance_id` and `since_t`."),
        q("a", "q4", json!({"kind":"get","id":"p1.up"}), "`since_t` moved to the new clock."),
        cmd("a", "c5", None, json!([{"op":"set_program_params","program":"p1","params":{"len":99}}]), "Out of range: `E_PARAM_RANGE` at `/ops/0/params`."),
        cmd("a", "c6", None, json!([{"op":"add_intent","id":"i1","text":"breakout","trigger":{"kind":"threshold","series":"p1.up","op":"ge","value":1}}]), "An intent rests on the event output."),
        cmd("a", "c7", None, json!([{"op":"remove_program","program":"p1"}]), "remove_program: `in_use`, `expected` = what rests on it."),
        cmd("a", "c8", None, json!([{"op":"remove","id":"i1"}]), "The intent goes."),
        cmd("a", "c9", None, json!([{"op":"remove_program","program":"p1"}]), "Now the series and their layers go."),
        cmd("a", "c10", None, sig, "A signal/state output is try-only: `E_TRY_ONLY` (`unsupported`)."),
    ];
    let init = thyao();
    let (out, s) = run(&init, steps);
    let a = reply(&out, "c1");
    assert_eq!(a["status"], "applied", "{a}");
    assert_eq!(a["created"][0], "p1.avg");
    assert_eq!(reply(&out, "c3")["error"]["code"], "duplicate");
    assert_eq!(reply(&out, "c7")["error"]["code"], "in_use");
    assert_eq!(reply(&out, "c10")["error"]["reason"], "E_TRY_ONLY");
    let up = &reply(&out, "q2")["data"]["values"]["outputs"]["up"];
    assert!(up.as_array().unwrap().last().unwrap().is_null(), "{up}");
    transcript(
        "71-program-add",
        "RL1 (PS §3.1, §1.1): add_program expands to `put_series` per output (program inline, `since_t`, `owner`) and layers with `show`; two instances of one program; set_program_params (same ids and layers, new `instance_id` and `since_t`); remove_program and `in_use`; signal/state outputs are `E_TRY_ONLY`.",
        &["add_program", "set_program_params", "remove_program", "in_use", "E_TRY_ONLY", "program"],
        init,
        out,
        &s,
    )
}

pub fn t72() -> Value {
    let mut bad = ir(SMA);
    bad["nodes"][1]["op"] = json!("smaa");
    let steps = vec![
        hello("a", "arkin", "A person."),
        q("a", "q1", json!({"kind":"check_program","source":SMA,"inputs":["px"]}), "Text: ok, cost bound to `px` (bars, total), outputs (documentable), the IR, limits, the static `W_CONSECUTIVE_EVENTS` warning."),
        q("a", "q2", json!({"kind":"check_program","source":"x = sma(close, 3)\nseries y = sma(close, 5000)\nseries z = rsi(x, 0)\nseries w = close[-1]\n"}), "Every checker error at once, each with its `span`: window limit, parameter range, look-ahead."),
        q("a", "q3", json!({"kind":"check_program","source":"x = smaa(close, 3)\nseries z = x\n","lang":"tr"}), "An unknown function (a text error, in Turkish): `did_you_mean`."),
        q("a", "q4", json!({"kind":"check_program","program":bad,"params":{"len":99}}), "IR: errors carry `at` (the IR path); a value out of range."),
        q("a", "q5", json!({"kind":"check_program","source":"reject E_LOOKAHEAD \"the next bar's close is not known at this bar's close\"\n"}), "A coded refusal: `ok: true` with `reject`."),
        q("a", "q6", json!({"kind":"check_program","source":HEAVY_TRY}), "It checks, but its live cost alone is over the per-owner limit: `W_LIVE_BUDGET` (`add_program` would refuse it)."),
        q("a", "q7", json!({"kind":"check_program","source":SIGNAL_STATE}), "Signal and state outputs check; not documentable."),
        q("a", "q8", json!({"kind":"check_program"}), "Neither program nor source: `invalid`."),
        cmd("a", "c1", None, json!([{"op":"add_program","program":bad,"inputs":["rsi14"],"params":{"len":99}}]), "add_program: the first error at the top (`E_NOT_BAR_SERIES`), every error in `errors[]`; nothing applied."),
    ];
    let init = thyao();
    let (out, s) = run(&init, steps);
    assert_eq!(reply(&out, "q1")["data"]["ok"], true);
    assert!(
        reply(&out, "q2")["data"]["errors"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
    let w = &reply(&out, "q6")["data"]["warnings"];
    assert!(
        w.as_array()
            .unwrap()
            .iter()
            .any(|w| w["reason"] == "W_LIVE_BUDGET"),
        "{w}"
    );
    assert!(
        reply(&out, "c1")["error"]["errors"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
    assert_eq!(s.log().len(), 0);
    transcript(
        "72-program-check",
        "RL1 (PS §3.2, PR §11): check_program — every error at once (`span` in text, `at` in IR, `did_you_mean`), reasons, warnings (`W_CONSECUTIVE_EVENTS`, `W_LIVE_BUDGET`), cost, the IR of a text, a coded refusal; add_program's `errors[]`.",
        &["check_program", "program-errors", "warnings", "cost", "reject"],
        init,
        out,
        &s,
    )
}

/// Before the synthetic feeds' first bar (2025-01-02): every bar is known after the
/// instances' `since_t`, so the document's event series carries every event.
const EARLY: i64 = 1_735_000_000_000;

fn bound(id: &str, symbol: &str, last: u32) -> Value {
    json!({"id": id, "instrument": {"symbol": symbol, "exchange": "BIST", "calendar": "XIST"},
           "interval": "1d",
           "source": {"kind": "bind", "symbol": symbol, "interval": "1d", "window": {"last": last}}})
}

fn try_init() -> Value {
    json!({
        "document": {"schema_version": "0", "id": "doc-try", "title": "Dene",
                     "series": [bound("px", "THYAO", 300), bound("big", "BIG", 20000)]},
        "clock": EARLY, "epoch": 0,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"viewer","author":{"kind":"human","name":"viewer"},"role":"viewer"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ],
        "data": {
            "feeds": [{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":300,"seed":5}},
                      {"feed":{"symbol":"BIG","interval":"1d"},"synthetic":{"n":20000,"seed":9}}],
            "entitlements": {"host":["*"], "arkin":["*"]}
        }
    })
}

pub fn t73() -> Value {
    let steps = vec![
        hello("a", "arkin", "Entitled to both feeds."),
        hello("v", "viewer", "Not entitled."),
        q("a", "q1", json!({"kind":"try_program","source":SMA,"inputs":["px"]}), "Series and event summaries over the closed bars: `events`, `runs`, `notifications`, `consecutive`, `isolated`, first/last events with `known_t`."),
        q("a", "q2", json!({"kind":"try_program","source":SIGNAL_STATE,"inputs":["px"]}), "Signal records (`mode: replay`, `label: simulated`) and state keys."),
        q("a", "q3", json!({"kind":"try_program","source":HEAVY_TRY,"inputs":["big"]}), "20 000 bars and four signals open on every bar: the run stops at `program.try.max_work` (`stopped: budget`); what came before stands."),
        cmd("a", "c1", None, json!([add("p1", SMA, false)]), "The program in the document; its `since_t` precedes every bar."),
        q("a", "q4", json!({"kind":"try_program","instance":"p1","range":{"from_t":EARLY}}), "Try of the instance from the same `from_t`."),
        q("a", "q5", json!({"kind":"study","series":"p1.up"}), "The document's event series: its event bars are the try's (more than zero)."),
        q("v", "q6", json!({"kind":"try_program","source":SMA,"inputs":["px"]}), "No data entitlement: `no_entitlement`."),
    ];
    let init = try_init();
    let (out, s) = run(&init, steps);
    let st = &reply(&out, "q3")["data"]["stopped"];
    assert_eq!(st["reason"], "budget", "{st}");
    let tried = reply(&out, "q4")["data"]["event"][0]["events"]
        .as_u64()
        .unwrap();
    let doc = reply(&out, "q5")["data"]["values"]["outputs"]["up"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v.as_f64() == Some(1.0))
        .count() as u64;
    assert!(tried > 0 && tried == doc, "try {tried} vs document {doc}");
    assert_eq!(reply(&out, "q6")["error"]["code"], "no_entitlement");
    transcript(
        "73-program-try",
        "RL1 (PS §3.2, PR §8.3): try_program — series, event (`consecutive`, `isolated`), signal (`simulated` records) and state summaries; the work budget stops a run; from the same `from_t` the try's events equal the document's event series (more than zero); a reader without the data is `no_entitlement`.",
        &["try_program", "events", "signals", "state", "budget", "no_entitlement"],
        init,
        out,
        &s,
    )
}

pub fn t76() -> Value {
    let p1 = add("p1", SMA, true);
    let heavy = |id: &str| json!({"op":"add_program","id":id,"program":ir(HEAVY),"inputs":["px"]});
    let steps: Vec<In> = vec![
        hello("a", "arkin", "A person (owner)."),
        hello("s", "scout", "An agent working for arkin."),
        cmd(
            "s",
            "c1",
            Some("direct"),
            json!([p1.clone()]),
            "An agent's add_program sent `direct`: `forbidden`, `proposal_only`.",
        ),
        cmd(
            "s",
            "c2",
            None,
            json!([p1]),
            "As `auto`: a proposal; nothing in the document yet.",
        ),
        cmd(
            "s",
            "c3",
            None,
            json!([{"op":"accept_proposal","proposal":"prop-1"}]),
            "It cannot decide its own proposal: `human_only`.",
        ),
        cmd(
            "a",
            "c4",
            None,
            json!([{"op":"accept_proposal","proposal":"prop-1"}]),
            "The person accepts.",
        ),
        q(
            "a",
            "q1",
            json!({"kind":"log","limit":1}),
            "The entry: author `scout`, `approved_by` the person.",
        ),
        q(
            "a",
            "q2",
            json!({"kind":"get","id":"p1.avg"}),
            "`owner` is the person the agent works for.",
        ),
        cmd(
            "s",
            "c5",
            Some("propose"),
            json!([{"op":"remove_program","program":"p1"}]),
            "remove_program: never from an agent (`agent_cannot_write`), not even as a proposal.",
        ),
        cmd(
            "s",
            "c6",
            None,
            json!([{"op":"set_program_params","program":"p1","params":{"len":8}}]),
            "New values: a proposal.",
        ),
        cmd(
            "s",
            "c7",
            None,
            json!([heavy("h0")]),
            "A costly program (≈ 600 per bar): proposed while there is room.",
        ),
        cmd(
            "a",
            "c8",
            None,
            json!([heavy("h1"), heavy("h2"), heavy("h3")]),
            "The person fills their live budget with three.",
        ),
        cmd(
            "a",
            "c9",
            None,
            json!([{"op":"accept_proposal","proposal":"prop-4"}]),
            "Accepting h0 now is over the budget: `E_BUDGET` (re-checked at acceptance).",
        ),
        q(
            "a",
            "q3",
            json!({"kind":"list","collection":"proposals","state":"pending"}),
            "It stays pending.",
        ),
    ];
    let init = thyao();
    let (out, s) = run(&init, steps);
    assert_eq!(reply(&out, "c1")["error"]["reason"], "proposal_only");
    assert_eq!(reply(&out, "c4")["status"], "applied");
    assert_eq!(reply(&out, "c5")["error"]["reason"], "agent_cannot_write");
    assert_eq!(reply(&out, "c2")["proposal"], "prop-1");
    assert_eq!(reply(&out, "c7")["proposal"], "prop-4");
    assert_eq!(
        reply(&out, "c8")["status"],
        "applied",
        "{}",
        reply(&out, "c8")
    );
    assert_eq!(
        reply(&out, "c9")["error"]["reason"],
        "E_BUDGET",
        "{}",
        reply(&out, "c9")
    );
    transcript(
        "76-program-agent-proposal",
        "RL1 (PS §2.2, §2.3): an agent's add_program is a proposal (`direct` → `proposal_only`); remove_program is `agent_cannot_write`; only a person decides, the entry is approved by the person and the program is owned by the person the agent works for; the live budget is re-checked when a proposal is accepted (`E_BUDGET`, it stays pending).",
        &["add_program", "proposal", "agent-cannot-write", "human-decision", "budget"],
        init,
        out,
        &s,
    )
}
