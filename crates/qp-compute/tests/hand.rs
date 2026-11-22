//! Small examples computed by hand (exact fractions written out in the comments).

mod support;

use support::{batch, close};

const TOL: f64 = 1e-12;

fn check(name: &str, got: &[Option<f64>], want: &[Option<f64>]) {
    assert_eq!(got.len(), want.len(), "{name}: length");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        match (g, w) {
            (None, None) => {}
            (Some(g), Some(w)) => assert!(close(*g, *w, TOL), "{name}[{i}]: got {g}, want {w}"),
            _ => panic!("{name}[{i}]: got {g:?}, want {w:?}"),
        }
    }
}

#[test]
fn sma_3() {
    // [1,2,3,4,8]: (1+2+3)/3=2, (2+3+4)/3=3, (3+4+8)/3=5
    let out = batch("sma", &[("period", 3.0)], &[1.0, 2.0, 3.0, 4.0, 8.0]);
    check(
        "sma",
        &out[0],
        &[None, None, Some(2.0), Some(3.0), Some(5.0)],
    );
}

#[test]
fn ema_2() {
    // a = 2/3. seed (2+4)/2 = 3; 6*2/3 + 3/3 = 5; 8*2/3 + 5/3 = 7; 4*2/3 + 7/3 = 5
    let out = batch("ema", &[("period", 2.0)], &[2.0, 4.0, 6.0, 8.0, 4.0]);
    check(
        "ema",
        &out[0],
        &[None, Some(3.0), Some(5.0), Some(7.0), Some(5.0)],
    );
}

#[test]
fn rsi_2() {
    // changes +1, -0.5, +1, -0.5
    // bar2: ag=(1+0)/2=0.5, al=(0+0.5)/2=0.25, rs=2   -> 100-100/3
    // bar3: ag=(0.5+1)/2=0.75, al=(0.25+0)/2=0.125, rs=6 -> 100-100/7
    // bar4: ag=(0.75+0)/2=0.375, al=(0.125+0.5)/2=0.3125, rs=1.2 -> 100-100/2.2
    let out = batch("rsi", &[("period", 2.0)], &[10.0, 11.0, 10.5, 11.5, 11.0]);
    check(
        "rsi",
        &out[0],
        &[
            None,
            None,
            Some(100.0 - 100.0 / 3.0),
            Some(100.0 - 100.0 / 7.0),
            Some(100.0 - 100.0 / 2.2),
        ],
    );
}

#[test]
fn rsi_edge_cases() {
    // No move at all -> 50; only gains -> 100.
    let out = batch("rsi", &[("period", 2.0)], &[5.0, 5.0, 5.0, 6.0]);
    check("rsi", &out[0], &[None, None, Some(50.0), Some(100.0)]);
}

#[test]
fn macd_2_3_2() {
    // x = [1,3,2,5,4,6]
    // ema2 (a=2/3): -, 2, 2, 4, 4, 16/3
    // ema3 (a=1/2): -, -, 2, 3.5, 3.75, 4.875
    // macd: -, -, 0, 1/2, 1/4, 11/24
    // signal = ema2(macd): -, -, -, (0+1/2)/2=1/4, 1/4*2/3+1/4*1/3=1/4, 11/24*2/3+1/12=7/18
    // histogram: -, -, -, 1/4, 0, 11/24-7/18=5/72
    let out = batch(
        "macd",
        &[("fast", 2.0), ("slow", 3.0), ("signal", 2.0)],
        &[1.0, 3.0, 2.0, 5.0, 4.0, 6.0],
    );
    check(
        "macd",
        &out[0],
        &[
            None,
            None,
            Some(0.0),
            Some(0.5),
            Some(0.25),
            Some(11.0 / 24.0),
        ],
    );
    check(
        "signal",
        &out[1],
        &[None, None, None, Some(0.25), Some(0.25), Some(7.0 / 18.0)],
    );
    check(
        "histogram",
        &out[2],
        &[None, None, None, Some(0.25), Some(0.0), Some(5.0 / 72.0)],
    );
}

#[test]
fn bollinger_3_2() {
    // x = [1,2,3,4,8], period 3, k 2, population std.
    // bar2: [1,2,3] mean 2, var 2/3; bar3: [2,3,4] mean 3, var 2/3; bar4: [3,4,8] mean 5, var 14/3
    let s1 = (2.0_f64 / 3.0).sqrt();
    let s2 = (14.0_f64 / 3.0).sqrt();
    let out = batch(
        "bollinger",
        &[("period", 3.0), ("k", 2.0)],
        &[1.0, 2.0, 3.0, 4.0, 8.0],
    );
    check(
        "middle",
        &out[0],
        &[None, None, Some(2.0), Some(3.0), Some(5.0)],
    );
    check(
        "upper",
        &out[1],
        &[
            None,
            None,
            Some(2.0 + 2.0 * s1),
            Some(3.0 + 2.0 * s1),
            Some(5.0 + 2.0 * s2),
        ],
    );
    check(
        "lower",
        &out[2],
        &[
            None,
            None,
            Some(2.0 - 2.0 * s1),
            Some(3.0 - 2.0 * s1),
            Some(5.0 - 2.0 * s2),
        ],
    );
    check(
        "bandwidth",
        &out[3],
        &[
            None,
            None,
            Some(4.0 * s1 / 2.0),
            Some(4.0 * s1 / 3.0),
            Some(4.0 * s2 / 5.0),
        ],
    );
}

#[test]
fn bollinger_flat_window_has_zero_width() {
    let out = batch("bollinger", &[("period", 3.0), ("k", 2.0)], &[7.0; 6]);
    for col in &out[1..] {
        assert_eq!(&col[2..], &[col[2]; 4]);
    }
    assert_eq!(out[1][2], Some(7.0));
    assert_eq!(out[2][2], Some(7.0));
    assert_eq!(out[3][2], Some(0.0));
}
