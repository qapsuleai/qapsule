//! Security review: extreme or non-finite domains and values must terminate quickly with an
//! error or a sane scene (no infinite grid loops).

mod common;

use std::time::{Duration, Instant};

use qp_grammar::detmath::nice_step;
use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::grammar::View;

fn view(domain: Option<[f64; 2]>, series: &str) -> View {
    let mut v = common::view(&format!(
        r#"{{"id":"h","x":{{"kind":"time"}},"panels":[{{"id":0,"y":{{"kind":"linear"}}}}],
            "layers":[{{"id":"c","mark":"candle","series":"px","panel":0}},
                      {{"id":"l","mark":"line","series":"{series}","panel":0}}]}}"#
    ));
    v.panels[0].y.domain = domain;
    v
}

fn timed<T>(f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    assert!(
        t.elapsed() < Duration::from_secs(2),
        "took {:?}",
        t.elapsed()
    );
    r
}

fn provider(vals: Vec<Option<f64>>) -> MemProvider {
    common::provider().with_values("x", "px", vec![("v", vals)], "v", &["v"])
}

#[test]
fn nice_step_rejects_hostile_input() {
    for raw in [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        0.0,
        -1.0,
        5e-324,
    ] {
        assert!(nice_step(raw).is_nan(), "{raw}");
    }
    assert!(nice_step(1e300).is_finite());
    let _ = nice_step(f64::MAX); // terminates (NaN when the step would overflow)
}

#[test]
fn extreme_fixed_domains() {
    let p = common::provider();
    let o = CompileOptions::new(1200, 600);
    let s = timed(|| compile(&view(Some([0.0, 1e308]), "sma20"), &p, &o)).unwrap();
    assert_eq!(s.panels[0].y.max, 1e308);
    // Span overflows: rejected with a clear error.
    let e = timed(|| compile(&view(Some([-1e308, 1e308]), "sma20"), &p, &o)).unwrap_err();
    assert_eq!(e.path, "/panels/0/y/domain");
    for d in [
        [f64::NAN, 1.0],
        [0.0, f64::INFINITY],
        [1.0, 1.0],
        [2.0, 1.0],
    ] {
        assert!(
            timed(|| compile(&view(Some(d), "sma20"), &p, &o)).is_err(),
            "{d:?}"
        );
    }
    // A huge offset with a tiny span (tick index beyond 2^53).
    let s = timed(|| compile(&view(Some([1e300, 1e300 * (1.0 + 1e-15)]), "sma20"), &p, &o));
    assert!(s.is_ok());
}

#[test]
fn extreme_and_non_finite_values() {
    let o = CompileOptions::new(1200, 600);
    let cases: Vec<Vec<Option<f64>>> = vec![
        (0..200)
            .map(|i| Some(1e300 + f64::from(i) * 1e285))
            .collect(),
        (0..200)
            .map(|i| Some(if i % 2 == 0 { 1e308 } else { -1e308 }))
            .collect(),
        (0..200)
            .map(|i| Some(if i % 2 == 0 { f64::MAX } else { f64::MAX / 3.0 }))
            .collect(),
        (0..200)
            .map(|i| Some([f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0][i % 4]))
            .collect(),
    ];
    for vals in cases {
        let p = provider(vals);
        let s = timed(|| compile(&view(None, "x"), &p, &o)).unwrap();
        let y = s.panels[0].y;
        assert!(
            y.min.is_finite() && y.max.is_finite() && y.min < y.max,
            "{y:?}"
        );
    }
}
