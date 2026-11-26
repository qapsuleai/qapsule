//! Shared helpers of the program series tests: fixed bars, programs, documents.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use qp_schema::{Bar, Document};
use serde_json::{json, Value};

/// 2024-01-01 00:00 UTC (a Monday).
pub const T0: i64 = 1_704_067_200_000;
pub const HOUR: i64 = 3_600_000;

/// Breakout over the previous `len` highs (PR §12.5 condition): an event output.
pub const BREAKOUT: &str = r#"{"version":"qp-ir/0","params":[{"key":"len","kind":"int","min":2,"max":500,"default":20}],"nodes":[{"id":"c","op":"input","field":"close"},{"id":"h","op":"input","field":"high"},{"id":"hi","op":"highest","x":"h","period":{"param":"len"}},{"id":"top","op":"lag","x":"hi","n":1},{"id":"up","op":"gt","a":"c","b":"top"},{"id":"ok","op":"defined","x":"top"},{"id":"k","op":"and","a":"up","b":"ok"}],"outputs":[{"name":"kirilim","kind":"event","node":"k"}]}"#;

/// EMA spread (PR §12.5 indicator): two series outputs.
pub const SPREAD: &str = r#"{"version":"qp-ir/0","params":[{"key":"fast","kind":"int","min":2,"max":200,"default":12},{"key":"slow","kind":"int","min":2,"max":400,"default":26}],"nodes":[{"id":"c","op":"input","field":"close"},{"id":"ef","op":"ema","x":"c","period":{"param":"fast"}},{"id":"es","op":"ema","x":"c","period":{"param":"slow"}},{"id":"diff","op":"sub","a":"ef","b":"es"},{"id":"tr","op":"ema","x":"diff","period":9}],"outputs":[{"name":"spread","kind":"series","node":"diff"},{"name":"trigger","kind":"series","node":"tr"}]}"#;

/// Close against the daily SMA (PR §12.5 higher timeframe): an event and a series.
pub const DAILY: &str = r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},{"id":"dc","op":"input","field":"close","tf":"1d"},{"id":"sm","op":"sma","x":"dc","period":3},{"id":"dm","op":"align","x":"sm"},{"id":"x","op":"cross_above","a":"c","b":"dm"}],"outputs":[{"name":"above_daily","kind":"event","node":"x"},{"name":"daily_ma","kind":"series","node":"dm"}]}"#;

/// Close above a constant: an event output.
pub const ABOVE: &str = r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},{"id":"lv","op":"const","value":105},{"id":"up","op":"gt","a":"c","b":"lv"}],"outputs":[{"name":"up","kind":"event","node":"up"}]}"#;

/// xorshift64* random-walk bars, `step` ms apart (test data, not computation).
pub fn walk(n: usize, seed: u64, t0: i64, step: i64) -> Vec<Bar> {
    let mut s = seed | 1;
    let mut unit = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        #[allow(clippy::cast_precision_loss)]
        let u = (s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
        u
    };
    let mut c = 100.0;
    (0..n)
        .map(|i| {
            let o = c;
            c *= 1.0 + (unit() - 0.5) * 0.04;
            Bar {
                t: t0 + i64::try_from(i).unwrap() * step,
                o,
                h: o.max(c) * (1.0 + unit() * 0.01),
                l: o.min(c) * (1.0 - unit() * 0.01),
                c,
                v: (unit() * 1000.0).floor(),
            }
        })
        .collect()
}

/// A program output series `<instance>.<output>` on bar series `px`.
pub fn program_series(
    instance: &str,
    ir: &str,
    output: &str,
    params: &Value,
    since_t: i64,
) -> Value {
    let ir: Value = serde_json::from_str(ir).unwrap();
    json!({
        "id": format!("{instance}.{output}"),
        "interval": "1h",
        "source": {"kind": "study", "study": "program", "inputs": ["px"], "params": {},
            "output": output,
            "program": {"instance": instance, "program_id": "sha256:0", "instance_id": "sha256:0",
                "scope": "document", "owner": "arkin", "since_t": since_t,
                "params": params, "ir": ir}}
    })
}

/// A document with inline hourly bar series `px` (UTC) and `extra` series.
pub fn doc(bars: &[Bar], extra: Vec<Value>) -> Document {
    let mut series = vec![json!({
        "id": "px", "interval": "1h", "source": {"kind": "inline"},
        "data": {"kind": "bars", "bars": bars}
    })];
    series.extend(extra);
    serde_json::from_value(json!({"schema_version": "0", "id": "d", "series": series})).unwrap()
}

/// Replaces the bars of `px`.
pub fn set_bars(d: &mut Document, bars: &[Bar]) {
    let s = d.series.iter_mut().find(|s| s.id == "px").unwrap();
    s.data = Some(qp_schema::SeriesData::Bars {
        bars: bars.to_vec(),
    });
}

/// Bit pattern of a value column (for bit-equality).
pub fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}
