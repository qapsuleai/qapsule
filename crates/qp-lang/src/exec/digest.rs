//! The cross-target determinism digest of the executor (PR §10; VERIFY 9): a fixed suite
//! of programs (the PR §12.5 examples, the five PR §14 `add_study` programs, a program
//! using every node and output kind) run on fixed synthetic bars — closed window in UTC,
//! on the XIST session, and streamed with revised live bars — and `try` summaries, all
//! hashed. aarch64, x86_64 and wasm32 must print the same value
//! (`fixtures/compute/digest-program.txt`, `examples/digest.rs`).

use qp_calendar::{builtin, ExchangeCalendar, Interval};
use qp_schema::{Author, Bar};
use serde_json::Value;

use super::summary::{try_program, RecordCtx, TryRequest};
use super::{Align, ExecOptions, Executor};
use crate::check::{check, Binding, CheckCtx};
use crate::limits::{Limit, Limits};
use crate::sha256::sha256_hex;

/// The suite, as `qp-ir/0` JSON.
pub const DIGEST_SUITE: [&str; 9] = [
    // PR §12.5 indicator.
    r#"{"version":"qp-ir/0","params":[{"key":"fast","kind":"int","min":2,"max":200,"default":12},{"key":"slow","kind":"int","min":2,"max":400,"default":26}],"nodes":[{"id":"c","op":"input","field":"close"},{"id":"ef","op":"ema","x":"c","period":{"param":"fast"}},{"id":"es","op":"ema","x":"c","period":{"param":"slow"}},{"id":"diff","op":"sub","a":"ef","b":"es"},{"id":"tr","op":"ema","x":"diff","period":9}],"outputs":[{"name":"spread","kind":"series","node":"diff"},{"name":"trigger","kind":"series","node":"tr"}]}"#,
    // PR §12.5 condition.
    r#"{"version":"qp-ir/0","params":[{"key":"len","kind":"int","min":2,"max":500,"default":20}],"nodes":[{"id":"c","op":"input","field":"close"},{"id":"h","op":"input","field":"high"},{"id":"hi","op":"highest","x":"h","period":{"param":"len"}},{"id":"top","op":"lag","x":"hi","n":1},{"id":"up","op":"gt","a":"c","b":"top"},{"id":"ok","op":"defined","x":"top"},{"id":"k","op":"and","a":"up","b":"ok"}],"outputs":[{"name":"kirilim","kind":"event","node":"k"}]}"#,
    // PR §12.5 signal.
    r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},{"id":"r","op":"rsi","x":"c","period":14},{"id":"lv","op":"const","value":30},{"id":"cb","op":"cross_below","a":"r","b":"lv"},{"id":"entry","op":"cooldown","trigger":"cb","bars":12}],"outputs":[{"name":"rsi_dip","kind":"signal","node":"entry","side":"al","rule":{"target_pct":3.0,"stop_pct":1.5,"max_bars":24},"source":{"strategy":"rsi_dip","version":"1"}}]}"#,
    // PR §12.5 higher timeframe.
    r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},{"id":"dc","op":"input","field":"close","tf":"1d"},{"id":"sm","op":"sma","x":"dc","period":10},{"id":"dm","op":"align","x":"sm"},{"id":"x","op":"cross_above","a":"c","b":"dm"}],"outputs":[{"name":"above_daily","kind":"event","node":"x"},{"name":"daily_ma","kind":"series","node":"dm"}]}"#,
    // PR §14 add_study programs.
    r#"{"version":"qp-ir/0","params":[{"key":"period","kind":"int","min":1,"max":2000,"default":20}],"nodes":[{"id":"_1","op":"input","field":"close"},{"id":"_2","op":"sma","x":"_1","period":{"param":"period"}}],"outputs":[{"name":"sma","kind":"series","node":"_2"}]}"#,
    r#"{"version":"qp-ir/0","nodes":[{"id":"_1","op":"input","field":"close"},{"id":"e","op":"ema","x":"_1","period":20},{"id":"r","op":"rsi","x":"_1","period":14}],"outputs":[{"name":"ema","kind":"series","node":"e"},{"name":"rsi","kind":"series","node":"r"}]}"#,
    r#"{"version":"qp-ir/0","nodes":[{"id":"_1","op":"input","field":"close"},{"id":"m","op":"macd","x":"_1","fast":12,"slow":26,"signal":9},{"id":"b","op":"bollinger","x":"_1","period":20,"k":2.0}],"outputs":[{"name":"macd","kind":"series","node":"m.macd"},{"name":"histogram","kind":"series","node":"m.histogram"},{"name":"upper","kind":"series","node":"b.upper"},{"name":"bandwidth","kind":"series","node":"b.bandwidth"}]}"#,
    // Every other node: new kernels, atr (base and 4h), history, state nodes, line, buckets.
    r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},{"id":"v","op":"input","field":"volume"},{"id":"sd","op":"stdev","x":"c","period":30},{"id":"pr","op":"percentrank","x":"v","period":20},{"id":"lo","op":"lowest","x":"c","period":15},{"id":"a","op":"atr","period":14},{"id":"a4","op":"atr","period":5,"tf":"4h"},{"id":"a4b","op":"align","x":"a4"},{"id":"q","op":"div","a":"a","b":"a4b"},{"id":"n","op":"neg","x":"q"},{"id":"ab","op":"abs","x":"n"},{"id":"mn","op":"min","a":"ab","b":"sd"},{"id":"ti","op":"time"},{"id":"bi","op":"bar_index"},{"id":"ln","op":"line","a":{"t":1704078000000,"price":99.0},"b":{"t":1704157200000,"price":101.0}},{"id":"gl","op":"gt","a":"c","b":"ln"},{"id":"st","op":"streak","c":"gl"},{"id":"bs","op":"bars_since","c":"gl"},{"id":"on","op":"once","c":"gl"},{"id":"ll","op":"lt","a":"c","b":"lo"},{"id":"le","op":"le","a":"c","b":"lo"},{"id":"rr","op":"rearm","trigger":"le","reset":"gl"},{"id":"ne","op":"not","x":"ll"},{"id":"o1","op":"or","a":"rr","b":"on"},{"id":"o2","op":"and","a":"o1","b":"ne"},{"id":"k1","op":"bucket","x":"pr","edges":[25,50,75]},{"id":"k2","op":"bucket","x":"st","edges":[1,3]},{"id":"k3","op":"bucket","x":"bs","edges":[5]},{"id":"mx","op":"max","a":"mn","b":"ti"},{"id":"bx","op":"mul","a":"bi","b":"mx"}],"outputs":[{"name":"mn","kind":"series","node":"mn"},{"name":"bx","kind":"series","node":"bx"},{"name":"g","kind":"signal","node":"o2","side":"sat","rule":{"target_pct":1.0,"stop_pct":2.0,"max_bars":10},"source":{"strategy":"all","version":"2"}},{"name":"reg","kind":"state","dims":[{"name":"rank","node":"k1"},{"name":"run","node":"k2"},{"name":"since","node":"k3"}]}]}"#,
    // Equality, ordering and the remaining arithmetic.
    r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},{"id":"o","op":"input","field":"open"},{"id":"h","op":"input","field":"high"},{"id":"l","op":"input","field":"low"},{"id":"ad","op":"add","a":"h","b":"l"},{"id":"mu","op":"mul","a":"ad","b":"c"},{"id":"di","op":"div","a":"mu","b":"o"},{"id":"g","op":"ge","a":"c","b":"o"},{"id":"e","op":"eq","a":"c","b":"o"},{"id":"ne","op":"ne","a":"c","b":"o"},{"id":"x","op":"cross_above","a":"di","b":"mu"},{"id":"y","op":"or","a":"g","b":"e"},{"id":"z","op":"and","a":"y","b":"ne"},{"id":"w","op":"or","a":"z","b":"x"}],"outputs":[{"name":"di","kind":"series","node":"di"},{"name":"w","kind":"event","node":"w"}]}"#,
];

/// 2024-01-01 00:00 UTC.
const T0: i64 = 1_704_067_200_000;
const HOUR: i64 = 3_600_000;

/// Fixed random-walk bars at the given open times (xorshift64*, fixed seed: test data,
/// not computation).
fn walk(ts: &[i64]) -> Vec<Bar> {
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut unit = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        #[allow(clippy::cast_precision_loss)]
        let u = (s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
        u
    };
    let mut c = 100.0;
    ts.iter()
        .map(|&t| {
            let o = c;
            c *= 1.0 + (unit() - 0.5) * 0.04;
            Bar {
                t,
                o,
                h: o.max(c) * (1.0 + unit() * 0.01),
                l: o.min(c) * (1.0 - unit() * 0.01),
                c,
                v: (unit() * 1000.0).floor(),
            }
        })
        .collect()
}

fn checked(json: &str, limits: Limits) -> Option<crate::checked::Checked> {
    let v: Value = serde_json::from_str(json).ok()?;
    let ctx = CheckCtx {
        limits,
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        ..CheckCtx::default()
    };
    check(&v, &ctx).checked
}

/// Streams `bars` as live bars, each revised once before its final values.
fn streamed(c: &crate::checked::Checked, bars: &[Bar], align: &Align) -> Option<String> {
    let opts = ExecOptions {
        align: align.clone(),
        ..ExecOptions::default()
    };
    let mut x = Executor::new(c, opts).ok()?;
    for b in bars {
        x.push(Bar {
            c: b.c * 1.001,
            ..b.clone()
        })
        .ok()?;
        x.update_last(b.clone()).ok()?;
    }
    let live = x.canonical();
    x.close_live();
    Some(live + &x.canonical())
}

/// The digest: SHA-256 hex of every run's canonical text and every try summary.
#[must_use]
pub fn program_digest() -> String {
    let limits = Limits::default().with(Limit::MaxCostPerBar, 10_000);
    let utc: Vec<i64> = (0..3000).map(|i| T0 + i * HOUR).collect();
    let session: Vec<i64> = builtin("XIST")
        .and_then(|c| ExchangeCalendar::new(&c).ok())
        .map(|cal| {
            cal.bars_between(Interval::Minutes(60), T0, T0 + 120 * 24 * HOUR)
                .iter()
                .map(|s| s.open)
                .collect()
        })
        .unwrap_or_default();
    let runs = [
        (walk(&utc), Align::Utc),
        (walk(&session), Align::for_calendar(Some("XIST"))),
    ];
    let ctx = RecordCtx {
        instance: "d".to_owned(),
        owner: "digest".to_owned(),
        author: Author::Human {
            name: "digest".to_owned(),
        },
        instrument: "SYN".to_owned(),
        period: "1h".to_owned(),
        series: "px".to_owned(),
        anchor_t: Some(T0),
        feeds: Vec::new(),
    };
    let mut text = String::new();
    for (k, p) in DIGEST_SUITE.iter().enumerate() {
        let Some(c) = checked(p, limits) else {
            text.push_str(&format!("program {k} does not check\n"));
            continue;
        };
        for (bars, align) in &runs {
            let opts = ExecOptions {
                align: align.clone(),
                ..ExecOptions::default()
            };
            match Executor::run_closed(&c, bars, opts) {
                Ok(x) => text.push_str(&x.canonical()),
                Err(e) => text.push_str(&format!("program {k}: {e}\n")),
            }
            text.push_str(&streamed(&c, &bars[..bars.len() / 3], align).unwrap_or_default());
            let req = TryRequest {
                checked: &c,
                bars,
                from_t: None,
                to_t: None,
                align: align.clone(),
                limits,
                ctx: ctx.clone(),
            };
            if let Ok(s) = try_program(&req) {
                text.push_str(&serde_json::to_string(&s).unwrap_or_default());
            }
        }
    }
    sha256_hex(text.as_bytes())
}
