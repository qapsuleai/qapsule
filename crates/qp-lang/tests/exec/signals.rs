//! Signals and their outcome rule (PR §6): stop before target, both in one bar → stop,
//! expiry at the horizon's close, open when data ends; only later closed bars resolve;
//! records in the PR §6 form; state keys (PR §5.4).

use qp_lang::{RecordCtx, SignalHit};
use qp_schema::program::{OutcomeKind, RecordLabel, RecordMode, Side};
use qp_schema::{Author, Bar};
use serde_json::{json, Value};

use crate::support::*;

/// A signal on bar 0 only, with `rule`, plus a state keyed on the bar index.
fn on_first_bar(side: &str, rule: Value) -> Value {
    prog(
        json!([
            {"id": "i", "op": "bar_index"},
            {"id": "z", "op": "const", "value": 0},
            {"id": "go", "op": "eq", "a": "i", "b": "z"},
            {"id": "k", "op": "bucket", "x": "i", "edges": [1, 3]}
        ]),
        json!([
            {"name": "sig", "kind": "signal", "node": "go", "side": side, "rule": rule,
             "source": {"strategy": "first", "version": "7"}},
            {"name": "regime", "kind": "state", "dims": [{"name": "phase", "node": "k"}]}
        ]),
    )
}

/// Bars from `(h, l, c)` triples, hourly from `T0`; bar 0 closes at 100.
fn hlc(v: &[(f64, f64, f64)]) -> Vec<Bar> {
    v.iter()
        .enumerate()
        .map(|(i, &(h, l, c))| Bar {
            t: T0 + i as i64 * HOUR,
            o: c,
            h,
            l,
            c,
            v: 1.0,
        })
        .collect()
}

fn hit(side: &str, rule: Value, b: &[Bar]) -> SignalHit {
    let c = checked(&on_first_bar(side, rule));
    let x = run(&c, b);
    x.signals_at(x.output("sig").unwrap(), 0)
        .cloned()
        .expect("a record on bar 0")
}

fn rule() -> Value {
    json!({"target_pct": 2.0, "stop_pct": 1.0, "max_bars": 3})
}

#[test]
fn target_is_reached() {
    let h = hit(
        "al",
        rule(),
        &hlc(&[
            (100.0, 100.0, 100.0),
            (101.0, 99.5, 100.5),
            (102.5, 100.0, 102.0),
        ]),
    );
    assert_eq!((h.outcome, h.exit_bar), (OutcomeKind::Target, Some(2)));
    assert_eq!(h.exit_price, Some(100.0 * (1.0 + 2.0 / 100.0)));
    assert_eq!(h.entry, 100.0);
}

#[test]
fn stop_comes_first_and_wins_a_tie() {
    let h = hit(
        "al",
        rule(),
        &hlc(&[
            (100.0, 100.0, 100.0),
            (100.5, 98.9, 99.0),
            (103.0, 99.5, 102.0),
        ]),
    );
    assert_eq!((h.outcome, h.exit_bar), (OutcomeKind::Stop, Some(1)));
    assert_eq!(h.exit_price, Some(100.0 * (1.0 - 1.0 / 100.0)));
    // Both levels inside one bar: stop.
    let h = hit(
        "al",
        rule(),
        &hlc(&[(100.0, 100.0, 100.0), (103.0, 98.0, 100.0)]),
    );
    assert_eq!((h.outcome, h.exit_bar), (OutcomeKind::Stop, Some(1)));
    // Sell side: the stop is above (h ≥ 101 counts, equal included).
    let h = hit(
        "sat",
        rule(),
        &hlc(&[(100.0, 100.0, 100.0), (101.0, 99.5, 100.0)]),
    );
    assert_eq!((h.outcome, h.exit_bar), (OutcomeKind::Stop, Some(1)));
    let h = hit(
        "sat",
        rule(),
        &hlc(&[(100.0, 100.0, 100.0), (100.5, 97.9, 98.0)]),
    );
    assert_eq!(h.outcome, OutcomeKind::Target);
}

#[test]
fn expiry_and_open() {
    let calm = (100.5, 99.5, 100.2);
    let b = hlc(&[
        (100.0, 100.0, 100.0),
        calm,
        calm,
        (100.5, 99.5, 100.3),
        calm,
    ]);
    let h = hit("al", rule(), &b);
    assert_eq!(
        (h.outcome, h.exit_bar, h.exit_price),
        (OutcomeKind::Expired, Some(3), Some(100.3))
    );
    // The signal bar itself never resolves its record, and data ending first leaves it open.
    let h = hit("al", rule(), &b[..3]);
    assert_eq!(
        (h.outcome, h.exit_bar, h.exit_price),
        (OutcomeKind::Open, None, None)
    );
    let h = hit(
        "al",
        json!({"max_bars": 1}),
        &hlc(&[(100.0, 0.0, 100.0), (500.0, 1.0, 100.4)]),
    );
    assert_eq!(
        (h.outcome, h.exit_bar),
        (OutcomeKind::Expired, Some(1)),
        "no levels: horizon only"
    );
}

#[test]
fn records_have_the_pr6_form() {
    let c = checked(&on_first_bar("al", rule()));
    let b = hlc(&[(100.0, 100.0, 100.0), (102.5, 99.5, 102.0)]);
    let x = run(&c, &b);
    let ctx = RecordCtx {
        instance: "p1".to_owned(),
        owner: "arkin".to_owned(),
        author: Author::Human {
            name: "arkin".to_owned(),
        },
        instrument: "THYAO".to_owned(),
        period: "1h".to_owned(),
        series: "px".to_owned(),
        anchor_t: Some(T0),
        feeds: Vec::new(),
    };
    let r = x.record_at(&c, x.output("sig").unwrap(), 0, &ctx).unwrap();
    assert_eq!(r.id, format!("p1.sig@{T0}"));
    assert_eq!(
        (r.mode, r.label),
        (RecordMode::Replay, Some(RecordLabel::Simulated))
    );
    assert_eq!((r.born_t, r.known_t), (T0, T0 + HOUR));
    assert_eq!((r.side, r.entry), (Side::Al, 100.0));
    assert_eq!(r.outcome.kind, OutcomeKind::Target);
    assert_eq!(r.outcome.exit_t, Some(T0 + HOUR));
    assert_eq!(r.provenance.program_id, c.program_id);
    assert_eq!(r.provenance.instance_id, c.instance_id);
    assert_eq!(
        (
            r.provenance.strategy.as_str(),
            r.provenance.program_version.as_str()
        ),
        ("first", "7")
    );
    let st = r.state.clone().expect("state key at birth");
    assert_eq!(
        (st.output.as_str(), st.key.get("phase")),
        ("regime", Some(&0))
    );
    assert_eq!(
        (
            r.evidence.series.as_str(),
            r.evidence.from_t,
            r.evidence.to_t
        ),
        ("px", T0, T0)
    );
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["mode"], "replay");
    assert_eq!(j["label"], "simulated");
    assert!(
        j["outcome"].get("exit_bar").is_none(),
        "bar indexes stay out of the record"
    );
}

#[test]
fn state_keys_per_closed_bar() {
    let c = checked(&on_first_bar("al", rule()));
    let x = run(&c, &bars(6, 1));
    let k = x.output("regime").unwrap();
    let keys: Vec<Option<Vec<u32>>> = (0..6)
        .map(|i| x.state_at(k, i).map(<[u32]>::to_vec))
        .collect();
    assert_eq!(
        keys,
        vec![
            Some(vec![0]),
            Some(vec![1]),
            Some(vec![1]),
            Some(vec![2]),
            Some(vec![2]),
            Some(vec![2])
        ]
    );
}
