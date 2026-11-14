//! Property tests: any finite f64 survives a JSON round-trip bit-for-bit.

mod common;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use qp_schema::{Bar, Instrument, Series, SeriesData, SeriesSource};

fn finite() -> impl Strategy<Value = f64> {
    prop::num::f64::NORMAL | prop::num::f64::SUBNORMAL | prop::num::f64::ZERO
}

prop_compose! {
    fn bar()(t in any::<i64>(), o in finite(), h in finite(), l in finite(), c in finite(), v in finite()) -> Bar {
        Bar { t, o, h, l, c, v }
    }
}

prop_compose! {
    fn series()(id in "[a-z0-9-]{1,16}", symbol in proptest::option::of("[A-Z]{3,6}"), bars in prop::collection::vec(bar(), 0..32)) -> Series {
        Series {
            id,
            instrument: symbol.map(|symbol| Instrument { symbol, exchange: None, calendar: None }),
            interval: Some("1d".into()),
            source: SeriesSource::Inline {},
            data: Some(SeriesData::Bars { bars }),
        }
    }
}

fn same_bits(a: &Bar, b: &Bar) -> bool {
    a.t == b.t
        && [a.o, a.h, a.l, a.c, a.v]
            .iter()
            .zip([b.o, b.h, b.l, b.c, b.v].iter())
            .all(|(x, y)| x.to_bits() == y.to_bits())
}

proptest! {
    // Reproducible in CI: fixed seed, no failure-persistence files. `PROPTEST_CASES`
    // still overrides the case count.
    #![proptest_config(ProptestConfig {
        rng_seed: RngSeed::Fixed(0x5172_2026),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn bar_roundtrips_exactly(b in bar()) {
        let json = serde_json::to_string(&b).unwrap();
        let back: Bar = serde_json::from_str(&json).unwrap();
        prop_assert!(same_bits(&b, &back), "{b:?} -> {json} -> {back:?}");
    }

    #[test]
    fn series_roundtrips_and_validates(s in series()) {
        let json = serde_json::to_string(&s).unwrap();
        let back: Series = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(&s, &back);
        let value = serde_json::to_value(&s).unwrap();
        let errors = common::validation_errors("Series", &value);
        prop_assert!(errors.is_empty(), "{:?}", errors);
    }
}
