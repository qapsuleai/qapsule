//! Every example of PR §12.5 and §14 lowers to the expected IR and prints back to the
//! same text (the examples are written canonically).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qp_lang_text::{lower, print_doc};
use qp_schema::program::{Program, ProgramDoc};
use qp_schema::protocol::Language;

fn canon(doc: &ProgramDoc) -> String {
    match doc {
        ProgramDoc::Program(p) => p.canonical_json().unwrap(),
        ProgramDoc::Reject(r) => serde_json::to_string(r).unwrap(),
    }
}

fn expected(json: &str) -> String {
    let doc: ProgramDoc = serde_json::from_str(json).unwrap();
    canon(&doc)
}

/// Lowers `src`, compares with `ir`, and checks that printing gives `src` back.
fn check(src: &str, ir: &str) {
    let l = lower(src, Language::En).unwrap_or_else(|e| panic!("{src}\n{e:#?}"));
    let doc = l.doc().unwrap();
    assert_eq!(canon(&doc), expected(ir), "{src}");
    assert_eq!(print_doc(&doc), src);
}

#[test]
fn indicator() {
    check(
        "param fast int = 12 in 2..200\n\
         param slow int = 26 in 2..400\n\
         diff = ema(close, fast) - ema(close, slow)\n\
         series spread = diff\n\
         series trigger = ema(diff, 9)\n",
        r#"{"version":"qp-ir/0",
          "params":[{"key":"fast","kind":"int","min":2,"max":200,"default":12},
                    {"key":"slow","kind":"int","min":2,"max":400,"default":26}],
          "nodes":[{"id":"_1","op":"input","field":"close"},
                   {"id":"_2","op":"ema","x":"_1","period":{"param":"fast"}},
                   {"id":"_3","op":"input","field":"close"},
                   {"id":"_4","op":"ema","x":"_3","period":{"param":"slow"}},
                   {"id":"diff","op":"sub","a":"_2","b":"_4"},
                   {"id":"_5","op":"ema","x":"diff","period":9}],
          "outputs":[{"name":"spread","kind":"series","node":"diff"},
                     {"name":"trigger","kind":"series","node":"_5"}]}"#,
    );
}

#[test]
fn condition() {
    check(
        "param len int = 20 in 2..500\n\
         top = highest(high, len)[1]\n\
         event kirilim = close > top and defined(top)\n",
        r#"{"version":"qp-ir/0",
          "params":[{"key":"len","kind":"int","min":2,"max":500,"default":20}],
          "nodes":[{"id":"_1","op":"input","field":"high"},
                   {"id":"_2","op":"highest","x":"_1","period":{"param":"len"}},
                   {"id":"top","op":"lag","x":"_2","n":1},
                   {"id":"_3","op":"input","field":"close"},
                   {"id":"_4","op":"gt","a":"_3","b":"top"},
                   {"id":"_5","op":"defined","x":"top"},
                   {"id":"_6","op":"and","a":"_4","b":"_5"}],
          "outputs":[{"name":"kirilim","kind":"event","node":"_6"}]}"#,
    );
}

#[test]
fn signal() {
    check(
        "r = rsi(close, 14)\n\
         entry = cooldown(cross_below(r, 30), 12)\n\
         signal rsi_dip = entry side al target 3% stop 1.5% max_bars 24 source \"rsi_dip\" \"1\"\n",
        r#"{"version":"qp-ir/0",
          "nodes":[{"id":"_1","op":"input","field":"close"},
                   {"id":"r","op":"rsi","x":"_1","period":14},
                   {"id":"_2","op":"const","value":30.0},
                   {"id":"_3","op":"cross_below","a":"r","b":"_2"},
                   {"id":"entry","op":"cooldown","trigger":"_3","bars":12}],
          "outputs":[{"name":"rsi_dip","kind":"signal","node":"entry","side":"al",
                      "rule":{"target_pct":3.0,"stop_pct":1.5,"max_bars":24},
                      "source":{"strategy":"rsi_dip","version":"1"}}]}"#,
    );
}

#[test]
fn higher_timeframe() {
    check(
        "daily_ma = align(sma(close@1d, 10))\n\
         event above_daily = cross_above(close, daily_ma)\n",
        r#"{"version":"qp-ir/0",
          "nodes":[{"id":"_1","op":"input","field":"close","tf":"1d"},
                   {"id":"_2","op":"sma","x":"_1","period":10},
                   {"id":"daily_ma","op":"align","x":"_2"},
                   {"id":"_3","op":"input","field":"close"},
                   {"id":"_4","op":"cross_above","a":"_3","b":"daily_ma"}],
          "outputs":[{"name":"above_daily","kind":"event","node":"_4"}]}"#,
    );
}

#[test]
fn reject() {
    let src = "reject E_LOOKAHEAD \"the next bar's close is not known at this bar's close\"\n";
    let l = lower(src, Language::En).unwrap();
    assert!(l.is_reject());
    assert_eq!(
        serde_json::to_string(&l.ir).unwrap(),
        r#"{"reject":{"code":"E_LOOKAHEAD","reason":"the next bar's close is not known at this bar's close"}}"#
    );
    assert_eq!(print_doc(&l.doc().unwrap()), src);
}

#[test]
fn study_equivalents() {
    check(
        "param period int = 20 in 1..2000\nseries sma = sma(close, period)\n",
        r#"{"version":"qp-ir/0",
          "params":[{"key":"period","kind":"int","min":1,"max":2000,"default":20}],
          "nodes":[{"id":"_1","op":"input","field":"close"},
                   {"id":"_2","op":"sma","x":"_1","period":{"param":"period"}}],
          "outputs":[{"name":"sma","kind":"series","node":"_2"}]}"#,
    );
    let texts = [
        "param period int = 20 in 1..2000\nseries ema = ema(close, period)\n",
        "param period int = 14 in 1..2000\nseries rsi = rsi(close, period)\n",
        "param fast int = 12 in 1..2000\nparam slow int = 26 in 1..2000\n\
         param signal int = 9 in 1..2000\nm = macd(close, fast, slow, signal)\n\
         series macd = m.macd\nseries signal = m.signal\nseries histogram = m.histogram\n",
        "param period int = 20 in 1..2000\nparam k float = 2 in 0.0001..100\n\
         b = bollinger(close, period, k)\nseries upper = b.upper\nseries middle = b.middle\n\
         series lower = b.lower\nseries bandwidth = b.bandwidth\n",
    ];
    for t in texts {
        let doc = lower(t, Language::En).unwrap().doc().unwrap();
        assert_eq!(print_doc(&doc), t);
    }
    let b = lower(texts[3], Language::En).unwrap().doc().unwrap();
    let ProgramDoc::Program(p) = b else {
        panic!("not a program")
    };
    let json = p.canonical_json().unwrap();
    assert!(
        json.contains(r#""min":0.0001,"max":100.0,"default":2.0"#),
        "{json}"
    );
    assert!(json.contains(
        r#"{"id":"b","op":"bollinger","x":"_1","period":{"param":"period"},"k":{"param":"k"}}"#
    ));
    assert!(json.contains(r#"{"name":"upper","kind":"series","node":"b.upper"}"#));
}

#[test]
fn surface_a_examples_lower() {
    // Background examples (surface-a DOC.md) in qp-ir/0 spelling: `state` for
    // `fingerprint`, a block over several lines, `atr(14, 4h)`, an inline selector.
    let src = "r = rsi(close, 14)\n\
               vol_ratio = volume / sma(volume, 20)\n\
               atr4 = align(atr(14, 4h))\n\
               sig = macd(close, 12, 26, 9).signal\n\
               wide = (high - low) > 0.5 * atr4\n\
               state regime {\n  momentum = bucket(r, [30, 50, 70]),\n  activity = bucket(vol_ratio, [0.8, 1.2])\n}\n\
               series up_run = streak(close > close[1])\n\
               event w = wide and not (sig < 0)\n";
    let l = lower(src, Language::En).unwrap_or_else(|e| panic!("{e:#?}"));
    let ProgramDoc::Program(p) = l.doc().unwrap() else {
        panic!("not a program")
    };
    let printed = qp_lang_text::print(&p);
    assert!(printed.contains(
        "state regime { momentum = bucket(r, [30, 50, 70]), activity = bucket(vol_ratio, [0.8, 1.2]) }"
    ));
    assert!(printed.contains("atr4 = align(atr(14, 4h))"));
    assert!(printed.contains("sig = macd(close, 12, 26, 9)\n"));
    assert!(printed.contains("event w = sig.signal") || printed.contains("not sig.signal < 0"));
    let again: Program = match lower(&printed, Language::En).unwrap().doc().unwrap() {
        ProgramDoc::Program(p) => p,
        ProgramDoc::Reject(_) => panic!("reject"),
    };
    assert_eq!(again.canonical_json().unwrap(), p.canonical_json().unwrap());
}
