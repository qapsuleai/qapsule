//! Nested study addresses (K63): a chain built from an address is bit-identical to the
//! same chain built by hand, in batch, streaming and "revise the last bar" modes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use qp_compute::address::{is_interval, Source, MAX_DEPTH};
use qp_compute::graph::sweep_chains;
use qp_compute::{run, Address, Chain, Field, Study};

/// Deterministic synthetic bars: a wavy random-ish walk around 100 (no RNG: an LCG with a
/// fixed seed), long enough for two levels of warmup.
fn bars(n: usize) -> Vec<qp_schema::Bar> {
    let mut s: u64 = 0x2545_f491_4f6c_dd1d;
    let mut c = 100.0_f64;
    (0..n)
        .map(|i| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[allow(clippy::cast_precision_loss)]
            let u = (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
            let o = c;
            c += u * 2.0;
            #[allow(clippy::cast_possible_wrap)]
            let t = i as i64 * 300_000;
            qp_schema::Bar {
                t,
                o,
                h: o.max(c) + u.abs(),
                l: o.min(c) - u.abs() * 0.5,
                c,
                v: 1000.0 + u * 100.0,
            }
        })
        .collect()
}

type Bits = Vec<Vec<Option<u64>>>;

fn bits(cols: &[Vec<Option<f64>>]) -> Bits {
    cols.iter()
        .map(|c| c.iter().map(|v| v.map(f64::to_bits)).collect())
        .collect()
}

/// Batch run over one input column.
fn batch(study: &mut dyn Study, x: &[f64]) -> Bits {
    bits(&run(study, &[x]).unwrap())
}

/// Streaming run with every bar revised twice before its final value (push a wrong
/// value, revise to another wrong one, revise to the real one).
fn revised(study: &mut dyn Study, x: &[f64]) -> Bits {
    study.reset();
    let n_out = study.outputs().len();
    let mut out: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_out];
    let mut o = vec![None; n_out];
    for &xi in x {
        study.push(&[xi * 0.97], &mut o);
        study.update_last(&[xi * 1.05], &mut o);
        study.update_last(&[xi], &mut o);
        for (col, v) in out.iter_mut().zip(&o) {
            col.push(*v);
        }
    }
    bits(&out)
}

/// Plain streaming run (push only).
fn streamed(study: &mut dyn Study, x: &[f64]) -> Bits {
    study.reset();
    let n_out = study.outputs().len();
    let mut out: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_out];
    let mut o = vec![None; n_out];
    for &xi in x {
        study.push(&[xi], &mut o);
        for (col, v) in out.iter_mut().zip(&o) {
            col.push(*v);
        }
    }
    bits(&out)
}

/// Checks an address against a hand-built chain on every run mode, plus K13 (one value
/// per bar, exactly `warmup` leading `None`s) and the graph-stage form.
fn check(address: &str, field: Field, mut hand: Box<dyn Study>) {
    let bars = bars(400);
    let x = field.column(&bars);
    let a = Address::parse(address).unwrap_or_else(|e| panic!("{address}: {e}"));
    let mut built = a.build().unwrap();

    assert_eq!(built.inputs(), hand.inputs(), "{address}: inputs");
    assert_eq!(built.outputs(), hand.outputs(), "{address}: outputs");
    assert_eq!(built.warmup(), hand.warmup(), "{address}: warmup");
    assert_eq!(built.params(), hand.params(), "{address}: params");

    let want = batch(hand.as_mut(), &x);
    let got = batch(built.as_mut(), &x);
    assert_eq!(got, want, "{address}: batch");
    assert_eq!(streamed(built.as_mut(), &x), want, "{address}: streaming");
    assert_eq!(
        revised(built.as_mut(), &x),
        want,
        "{address}: revise last bar"
    );
    assert_eq!(revised(hand.as_mut(), &x), want, "{address}: hand revised");

    // K13: as long as the bars, warmup is exactly the leading `None`s.
    for (col, w) in got.iter().zip(built.warmup()) {
        assert_eq!(col.len(), bars.len(), "{address}: length");
        assert!(col[..w].iter().all(Option::is_none), "{address}: warmup");
        assert!(col[w..].iter().all(Option::is_some), "{address}: values");
    }

    // The same chain as graph stages (sweep path): NaN where the chain has `None`.
    let (f, stages) = a.stages();
    assert_eq!(f, field);
    let mut seen = false;
    sweep_chains(&[stages], &x, |_, c| {
        seen = true;
        let sweep: Bits = c
            .columns
            .iter()
            .map(|col| {
                col.iter()
                    .map(|v| (!v.is_nan()).then(|| v.to_bits()))
                    .collect()
            })
            .collect();
        assert_eq!(sweep, want, "{address}: graph stages");
    })
    .unwrap();
    assert!(seen);
}

#[test]
fn rsi_14() {
    check(
        "RSI:14",
        Field::Close,
        support::study("rsi", &[("period", 14.0)]),
    );
}

#[test]
fn ema_of_rsi_with_symbol_and_interval() {
    let a = Address::parse("ISCTR:5m:EMA[RSI:18]:20").unwrap();
    assert_eq!(a.symbol.as_deref(), Some("ISCTR"));
    assert_eq!(a.interval.as_deref(), Some("5m"));
    assert_eq!(a.root.study, "ema");
    assert!(matches!(&a.root.source, Source::Study(n) if n.study == "rsi"));
    check(
        "ISCTR:5m:EMA[RSI:18]:20",
        Field::Close,
        Box::new(Chain::new(
            support::study("rsi", &[("period", 18.0)]),
            0,
            support::study("ema", &[("period", 20.0)]),
        )),
    );
    // Without the prefix: the same chain.
    check(
        "EMA[RSI:18]:20",
        Field::Close,
        Box::new(Chain::new(
            support::study("rsi", &[("period", 18.0)]),
            0,
            support::study("ema", &[("period", 20.0)]),
        )),
    );
}

#[test]
fn two_levels() {
    let hand = || -> Box<dyn Study> {
        Box::new(Chain::new(
            Box::new(Chain::new(
                support::study("rsi", &[("period", 14.0)]),
                0,
                support::study("ema", &[("period", 10.0)]),
            )),
            0,
            support::study("sma", &[("period", 5.0)]),
        ))
    };
    check("SMA[EMA[RSI:14]:10]:5", Field::Close, hand());
    check("THYAO:1d:sma[Ema[rsi:14]:10]:5", Field::Close, hand());
}

#[test]
fn three_levels_with_field_source() {
    check(
        "RSI[SMA[EMA[high]:3]:4]:6",
        Field::High,
        Box::new(Chain::new(
            Box::new(Chain::new(
                qp_compute::create("ema", &support::params(&[("period", 3.0)]), Field::High)
                    .unwrap(),
                0,
                support::study("sma", &[("period", 4.0)]),
            )),
            0,
            support::study("rsi", &[("period", 6.0)]),
        )),
    );
}

#[test]
fn multi_output_selectors() {
    // Inner selector: EMA of the MACD signal line (output 1).
    let hand = || -> Box<dyn Study> {
        Box::new(Chain::new(
            support::study("macd", &[("fast", 12.0), ("slow", 26.0), ("signal", 9.0)]),
            1,
            support::study("ema", &[("period", 5.0)]),
        ))
    };
    check("EMA[MACD(signal):12:26:9]:5", Field::Close, hand());
    check("EMA[MACD.SIGNAL:12,26,9]:5", Field::Close, hand());
    // Main output of MACD is `macd` (index 0).
    check(
        "EMA[MACD]:5",
        Field::Close,
        Box::new(Chain::new(
            support::study("macd", &[]),
            0,
            support::study("ema", &[("period", 5.0)]),
        )),
    );
    // Outer selector: the address names column 1 (`upper`) of a Bollinger of RSI.
    let a = Address::parse("BB(upper)[RSI:14]:20:2.5").unwrap();
    assert_eq!(a.output(), 1);
    check(
        "BB(upper)[RSI:14]:20:2.5",
        Field::Close,
        Box::new(Chain::new(
            support::study("rsi", &[("period", 14.0)]),
            0,
            support::study("bollinger", &[("period", 20.0), ("k", 2.5)]),
        )),
    );
    assert_eq!(Address::parse("MACD").unwrap().output(), 0);
    assert_eq!(Address::parse("BB").unwrap().output(), 0);
}

#[test]
fn defaults_aliases_and_case() {
    assert_eq!(
        Address::parse("RSI").unwrap(),
        Address::parse("rsi:14").unwrap()
    );
    assert_eq!(
        Address::parse("MACD").unwrap(),
        Address::parse("macd:12:26:9").unwrap()
    );
    assert_eq!(
        Address::parse("bollingerbands:20:2").unwrap(),
        Address::parse("BB").unwrap()
    );
    assert_eq!(
        Address::parse("Ema[Rsi:18]:20").unwrap(),
        Address::parse("EMA[RSI:18]:20").unwrap()
    );
}

#[test]
fn canonical_text_round_trips() {
    for (input, canon) in [
        ("isctr:5m:ema[rsi:18]:20", "isctr:5m:EMA[RSI:18]:20"),
        ("RSI", "RSI:14"),
        ("EMA[MACD.signal:12,26,9]:5", "EMA[MACD(signal):12:26:9]:5"),
        ("sma[HIGH]:3", "SMA[high]:3"),
        (
            "BOLLINGER(upper)[RSI:14]:20:2.5",
            "BB(upper)[RSI:14]:20:2.5",
        ),
    ] {
        let a = Address::parse(input).unwrap();
        assert_eq!(a.to_string(), canon);
        assert_eq!(Address::parse(canon).unwrap(), a);
    }
}

#[test]
fn intervals() {
    for ok in [
        "1m", "5m", "15m", "1440m", "1h", "4h", "24h", "1d", "1w", "1M",
    ] {
        assert!(is_interval(ok), "{ok}");
        let a = Address::parse(&format!("X:{ok}:RSI:14")).unwrap();
        assert_eq!(a.interval.as_deref(), Some(ok));
    }
    for bad in ["0m", "1441m", "25h", "2d", "m", "5", "5x", "1D", ""] {
        assert!(!is_interval(bad), "{bad}");
    }
}

/// `(address, byte offset, part of the message)`.
#[test]
fn errors_are_positioned_and_clear() {
    let cases: &[(&str, usize, &str)] = &[
        ("FOO:14", 0, "unknown study `FOO`"),
        ("EMA[FOO:3]:20", 4, "unknown study `FOO`"),
        ("ISCTR:5m:XYZ[RSI:14]:20", 9, "unknown study `XYZ`"),
        ("ISCTR:5x:RSI:14", 6, "invalid interval `5x`"),
        ("MACD:12:26", 10, "missing parameter `signal`"),
        ("BB:20", 5, "missing parameter `k`"),
        ("RSI:14:3", 7, "too many parameters"),
        ("EMA[RSI:18", 10, "unbalanced `[` at byte 3"),
        ("SMA[EMA[RSI:14]:10", 18, "unbalanced `[` at byte 3"),
        ("EMA[RSI:18]]:20", 11, "unbalanced `]`"),
        ("EMA[RSI:18]:20]", 14, "unbalanced `]`"),
        ("EMA[]:20", 4, "expected a study name"),
        ("", 0, "expected a study name"),
        ("EMA[RSI:x]:20", 8, "expected a number"),
        ("EMA:", 4, "expected a number"),
        ("RSI:2.5", 0, "must be an integer"),
        ("RSI:0", 0, "between 1 and"),
        ("MACD:26:12:9", 0, "fast"),
        ("EMA[MACD(sig)]:5", 9, "no output `sig`"),
        ("EMA[MACD(signal]:5", 15, "expected `)`"),
        ("SMA[hl2]:20", 4, "source `hl2` is not supported"),
        ("RSI 14", 3, "unexpected ` `"),
    ];
    for (address, pos, part) in cases {
        let e = Address::parse(address).expect_err(address);
        assert!(
            e.msg.contains(part),
            "{address}: message `{}` lacks `{part}`",
            e.msg
        );
        assert_eq!(e.pos, *pos, "{address}: position ({e})");
    }
}

#[test]
fn depth_guard() {
    let mut s = "RSI:14".to_owned();
    for _ in 1..MAX_DEPTH {
        s = format!("EMA[{s}]:2");
    }
    let a = Address::parse(&s).unwrap();
    assert_eq!(a.stages().1.len(), MAX_DEPTH);
    let too_deep = format!("EMA[{s}]:2");
    let e = Address::parse(&too_deep).unwrap_err();
    assert!(e.msg.contains("nesting deeper"), "{e}");
}
