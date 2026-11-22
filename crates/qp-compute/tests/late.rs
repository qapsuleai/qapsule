//! RP1/E8: certified late start = full run, bit for bit, on every bar of the tail, for
//! random walks, flat and constant stretches, zeros, short series and every `from`.

mod support;

use qp_compute::late::{ema_tail, rsi_tail};
use qp_compute::sweep::bench_closes;

#[allow(clippy::needless_range_loop)]
fn series(kind: usize, n: usize) -> Vec<f64> {
    let mut x = bench_closes(n, 1000 + kind as u64);
    match kind {
        1 => x.iter_mut().for_each(|v| *v = 42.17), // constant
        2 => {
            // long flat stretches
            for i in 0..n {
                if (i / 300) % 2 == 1 {
                    x[i] = x[i - i % 300];
                }
            }
        }
        3 => {
            for i in 0..n {
                if (i / 500) % 3 == 1 {
                    x[i] = 0.0;
                }
            }
        }
        4 => x
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v *= 1.0 + (i % 7) as f64 * 1e-3),
        _ => {}
    }
    x
}

fn check(name: &str, x: &[f64], period: usize, from: usize) -> bool {
    let full = support::batch(name, &[("period", period as f64)], x);
    let mut out = vec![0.0; x.len() - from];
    let cert = if name == "ema" {
        ema_tail(x, period, from, &mut out)
    } else {
        rsi_tail(x, period, from, &mut out)
    };
    for (j, v) in out.iter().enumerate() {
        let want = full[0][from + j].map_or(f64::NAN.to_bits(), f64::to_bits);
        assert_eq!(
            v.to_bits(),
            want,
            "{name}({period}) from {from} bar {} cert {cert:?}",
            from + j
        );
    }
    cert.met.is_some()
}

#[test]
fn late_start_equals_full_run() {
    let mut certified = 0;
    let mut total = 0;
    let mut must = 0;
    for kind in 0..5 {
        for n in [0, 1, 30, 3000] {
            let x = series(kind, n);
            for period in [1, 2, 3, 9, 14, 20, 50, 199] {
                for from in [0, n / 3, n.saturating_sub(500), n.saturating_sub(1), n] {
                    for name in ["ema", "rsi"] {
                        total += 1;
                        let ok = check(name, &x, period, from);
                        certified += usize::from(ok);
                        // A random walk's last 500 bars of 3,000 must be certified for
                        // short periods (the point of the method).
                        if kind == 0 && n == 3000 && from == n - 500 && (2..=50).contains(&period) {
                            assert!(ok, "{name}({period}) not certified");
                            must += 1;
                        }
                    }
                }
            }
        }
    }
    println!("certified {certified} of {total} (required {must})");
    assert!(must > 0);
}
