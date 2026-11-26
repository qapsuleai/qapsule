//! Transcript 75 (an event output on the live path).

use serde_json::{json, Value};

use super::{clock, cmd, hello, ir, met, q, reply, run, sub, transcript, In, ABOVE};

const DAY: i64 = 86_400_000;
const T0: i64 = 1_759_708_800_000;
const CLOCK: i64 = 1_760_000_000_000;

fn bar(k: i64, c: f64) -> Value {
    json!({"t": T0 + k * DAY, "o": c, "h": c + 0.5, "l": c - 0.5, "c": c, "v": 1000.0})
}

fn data(seq: u64, change: Value, note: &'static str) -> In {
    In::Data(
        json!({"feed": {"symbol": "THYAO", "interval": "1d"}, "seq": seq, "change": change}),
        note,
    )
}

fn init() -> Value {
    let five: Vec<Value> = (0..5).map(|k| bar(k, 100.0 + k as f64)).collect();
    json!({
        "document": {"schema_version": "0", "id": "doc-event", "title": "Olay",
            "series": [
                {"id": "px", "instrument": {"symbol": "THYAO"}, "interval": "1d",
                 "source": {"kind": "bind", "symbol": "THYAO", "interval": "1d", "window": {"last": 10}}},
                {"id": "qx", "instrument": {"symbol": "QX"}, "interval": "1d",
                 "source": {"kind": "inline"}, "data": {"kind": "bars", "bars": five}}
            ]},
        "clock": CLOCK, "epoch": 0,
        "principals": [
            {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
            {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"}, "role": "host"}
        ],
        "data": {"feeds": [{"feed": {"symbol": "THYAO", "interval": "1d"}, "bars": five}],
                 "entitlements": {"host": ["*"], "arkin": ["*"]}}
    })
}

fn threshold(series: &str) -> Value {
    json!({"kind":"condition","condition":{"kind":"threshold","series":series,"op":"ge","value":1.0}})
}

pub fn t75() -> Value {
    let above = ir(ABOVE);
    let ops = json!([
        {"op":"add_program","id":"p1","program":above,"inputs":["px"]},
        {"op":"add_program","id":"q1","program":above,"inputs":["qx"]}
    ]);
    let cmp = json!({"kind":"compare","left":"p1.level","op":"gt","right":"p1.up"});
    let steps = vec![
        hello("a", "arkin", "A person."),
        hello("h", "host", "The host (moves the clock)."),
        cmd("a", "c1", None, ops, "`event up = close > lv` (lv 105) on the bound `px` and on the inline `qx`; `since_t` is the clock."),
        sub("a", "c", threshold("p1.up"), "The existing `threshold ge 1` condition on the event output."),
        sub("a", "k", json!({"kind":"claims"}), "Claims."),
        sub("a", "d", threshold("q1.up"), "The same on `qx`."),
        data(1, json!({"kind":"patch","t":T0 + 4 * DAY,"fields":{"c":106.0,"h":106.5}}), "The live bar crosses 105: an event series has no value on the live bar (`null`), no condition."),
        data(2, json!({"kind":"bar","bar":bar(5, 106.5)}), "The next bar closes it: `condition_met` at the event bar, evidence `final` at once."),
        data(3, json!({"kind":"bar","bar":bar(6, 99.0)}), "The bar after is an event bar too (consecutive): recorded, no second notification."),
        data(4, json!({"kind":"bar","bar":bar(7, 99.5)}), "No event."),
        q("a", "q1", json!({"kind":"study","series":"p1.up"}), "Every event bar in the series; `null` on the live bar."),
        data(5, json!({"kind":"patch","t":T0 + 4 * DAY,"fields":{"c":100.0,"l":99.5}}), "A correction of history (bar 4 no longer above): the series is rebuilt and the recorded claims re-checked."),
        q("a", "q2", json!({"kind":"study","series":"p1.up"}), "Bar 4 is no event now."),
        cmd("a", "c2", None, json!([{"op":"append_bars","series":"qx","bars":[bar(5, 106.0), bar(6, 100.0), bar(7, 100.0)]}]), "Several bars in one entry on `qx`: the 0,1,0 pattern passes without notification (the documented loss bound)."),
        q("a", "q3", json!({"kind":"study","series":"q1.up"}), "The series still records the event bar."),
        cmd("a", "c3", None, json!([{"op":"add_intent","text":"x","trigger":cmp}]), "An event series as a `compare` operand: `E_EVENT_OPERAND`."),
        clock(T0 + 7 * DAY + 3_600_000, "After bar 6 closed, before bar 7 closes."),
        cmd("a", "c4", None, json!([{"op":"set_program_params","program":"p1","params":{"lv":98.0}}]), "New level 98: the last closed bar (99) would be above, but it closed before the new `since_t`: no retroactive trigger."),
        q("a", "q4", json!({"kind":"study","series":"p1.up"}), "Bars known before `since_t` are 0."),
        data(6, json!({"kind":"bar","bar":bar(8, 99.8)}), "Bar 7 (99.5 > 98) closes after `since_t`: `condition_met`."),
    ];
    let init = init();
    let (out, s) = run(&init, steps);
    let by_note = |n: &str| {
        out.iter()
            .find(|st| st["note"].as_str().unwrap().starts_with(n))
            .unwrap()
    };
    assert_eq!(met(by_note("The live bar crosses")), 0);
    assert_eq!(met(by_note("The next bar closes")), 1);
    assert_eq!(met(by_note("The bar after")), 0);
    assert_eq!(met(by_note("Several bars")), 0);
    assert_eq!(met(by_note("New level 98")), 0);
    assert_eq!(met(by_note("Bar 7 (99.5")), 1);
    let q3 = &reply(&out, "q3")["data"]["values"]["outputs"]["up"];
    assert!(
        q3.as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_f64() == Some(1.0)),
        "{q3}"
    );
    assert_eq!(reply(&out, "c3")["error"]["reason"], "E_EVENT_OPERAND");
    transcript(
        "75-program-event",
        "RL1 (PS §1.2): an event output under `threshold ge 1` — met when the event bar closes, evidence `final`; consecutive event bars give one notification; several bars in one entry pass without one (loss bound); a history correction rebuilds the series and re-checks claims; new values trigger nothing retroactively (`since_t`); `compare` with an event series is `E_EVENT_OPERAND`.",
        &["event", "condition_met", "since_t", "append_bars", "E_EVENT_OPERAND", "program"],
        init,
        out,
        &s,
    )
}
