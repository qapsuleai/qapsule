//! Calibration of `C_DIR` and `C_NEUTRAL` (see `NOTES.md`): unconditional barrier rates
//! with every bar ≥ 120 as a "birth" (no detector, no story). Directional: expired share
//! and up-first share for `k = c·√h`; neutral: expansion share for `k_n = c·√h` (ATR(100)).
//! The constants were chosen on the synthetic market only; `-- real` prints the same
//! aggregate rates for the local real bars (`fixtures/local/bars`, never committed) as a
//! check. Prints aggregates only.
use qp_schema::Bar;

fn rates(series: &[Vec<Bar>], h: usize, k: f64, period: usize, neutral: bool) -> (f64, f64) {
    let (mut n, mut exp, mut up) = (0u64, 0u64, 0u64);
    for b in series {
        if b.len() < 150 + h {
            continue;
        }
        for born in 120..b.len() - h {
            let Some(a) = m5_barrier::wilder_atr(&b[..=born], period) else {
                continue;
            };
            let p0 = b[born].c;
            let mut res = 0;
            for x in &b[born + 1..=born + h] {
                let d = x.c - p0;
                if neutral {
                    if d.abs() >= k * a {
                        res = 1;
                        break;
                    }
                } else if d >= k * a {
                    res = 1;
                    break;
                } else if d <= -k * a {
                    res = 2;
                    break;
                }
            }
            n += 1;
            exp += u64::from(res == 0);
            up += u64::from(res == 1);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let r = (exp as f64 / n as f64, up as f64 / n as f64);
    r
}

fn main() {
    let mut sets: Vec<(String, Vec<Vec<Bar>>)> = Vec::new();
    let mut syn = Vec::new();
    for seed in 1..=3u64 {
        for inst in qp_source::synthetic_market(30, 500, seed) {
            syn.push(inst.bars);
        }
    }
    sets.push(("synthetic".into(), syn));
    if std::env::args().any(|a| a == "real") {
        let dir = qp_source::local_bars_dir();
        for p in ["1day", "1hour"] {
            let v: Vec<Vec<Bar>> = qp_source::load_local_bars(&dir, p)
                .into_iter()
                .map(|(_, b)| b)
                .collect();
            sets.push((format!("real-{p}"), v));
        }
    }
    for (name, s) in &sets {
        println!("== {name}");
        for c in [0.25, 0.3, 0.35, 0.4] {
            let mut line = format!("dir c {c}:");
            for h in [10usize, 15, 20] {
                #[allow(clippy::cast_precision_loss)]
                let k = c * (h as f64).sqrt();
                let (e, u) = rates(s, h, k, 14, false);
                line += &format!("  h{h} k{k:.2} exp {e:.3} up {u:.3}");
            }
            println!("{line}");
        }
        for c in [0.5, 0.6, 0.7, 0.8] {
            let mut line = format!("neu c {c}:");
            for h in [10usize, 20] {
                #[allow(clippy::cast_precision_loss)]
                let k = c * (h as f64).sqrt();
                let (e, _) = rates(s, h, k, 100, true);
                line += &format!("  h{h} k{k:.2} realized {:.3}", 1.0 - e);
            }
            println!("{line}");
        }
    }
}
