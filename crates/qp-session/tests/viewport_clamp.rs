//! Regression: a saved viewport must be clamped when the bar series gets shorter
//! (security review finding: `last - width` underflow in `pan`).

mod common;

use common::H;
use serde_json::json;

#[test]
fn pan_and_zoom_after_series_shrinks_do_not_underflow() {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    let bars: Vec<_> = (0..50_i64)
        .map(|i| json!({"t": 1759968000000_i64 + i * 86_400_000, "o": 124.0, "h": 125.0, "l": 123.0, "c": 124.5, "v": 1000.0}))
        .collect();
    let ack = h.cmd(
        "a",
        "c1",
        json!([{"op": "append_bars", "series": "px", "bars": bars}]),
    );
    assert_eq!(ack["status"], "applied", "{ack}");
    // Look at every bar (0..=249): the saved viewport is wider than the series will be.
    let r = h.one(
        "a",
        json!({"type":"input","id":"i1","gesture":{"kind":"look","view":"main","from_bar":0,"to_bar":249}}),
    );
    assert_eq!(r["data"]["viewport"]["to_bar"], 249, "{r}");
    // Undo the append: the series is back to 200 bars (last index 199).
    let ack = h.cmd("a", "c2", json!([{"op": "undo"}]));
    assert_eq!(ack["status"], "applied", "{ack}");
    // Reads that use the stale viewport must not panic either.
    let r = h.one(
        "a",
        json!({"type":"query","id":"q1","query":{"kind":"screen"}}),
    );
    assert_eq!(r["type"], "result", "{r}");
    assert_eq!(
        r["data"]["screens"][0]["viewport"]["to_bar"], 199,
        "screen reports a clamped viewport: {r}"
    );
    let r = h.one(
        "a",
        json!({"type":"query","id":"q2","query":{"kind":"attention","principal":"arkin"}}),
    );
    assert_eq!(r["type"], "result", "{r}");
    for (id, gesture) in [
        ("i2", json!({"kind":"pan","view":"main","bars":5})),
        ("i3", json!({"kind":"pan","view":"main","bars":-5})),
        ("i4", json!({"kind":"zoom","view":"main","factor":2.0})),
    ] {
        let r = h.one("a", json!({"type":"input","id":id,"gesture":gesture}));
        assert_eq!(r["type"], "result", "{id}: {r}");
        let vp = &r["data"]["viewport"];
        let (from, to) = (
            vp["from_bar"].as_u64().unwrap(),
            vp["to_bar"].as_u64().unwrap(),
        );
        assert!(
            from <= to && to <= 199,
            "{id}: viewport {from}..{to} outside 0..=199"
        );
    }
}
