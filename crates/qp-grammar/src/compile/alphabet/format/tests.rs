//! Number and date formats by language.

use super::*;

#[test]
fn numbers_and_dates_by_language() {
    let (tr, en) = (Language::Tr, Language::En);
    assert_eq!(NumFmt::new(3000.0, None, tr).fmt(2925.95), "2.925,95");
    assert_eq!(NumFmt::new(3000.0, None, en).fmt(-2925.95), "-2,925.95");
    assert_eq!(NumFmt::new(2.5e6, Some(5e5), tr).fmt(1.5e6), "1,5 Mn");
    assert_eq!(NumFmt::new(2.5e6, Some(1e6), en).fmt(2e6), "2M");
    assert_eq!(NumFmt::new(100.0, Some(0.025), tr).fmt(0.075), "0,075");
    assert_eq!(signed(-0.0128 * 100.0, 2, true, tr), "-%1,28");
    assert_eq!(signed(22.43, 2, false, en), "+22.43");
    // 2025-07-03 00:00 UTC.
    assert_eq!(civil(1_751_500_800_000), (2025, 7, 3));
    assert_eq!(pill_date(1_751_500_800_000, false, tr), "3 Tem 2025");
    assert_eq!(pill_date(1_751_535_000_000, true, en), "3 Jul 09:30");
    assert_eq!(civil(0), (1970, 1, 1));
}
