//! Option chosen in R1 (K16): qp-calendar's embedded transition tables.
use qp_calendar::{LocalTime, Zone};
fn tz(z: u32) -> Zone { Zone::get(if z == 0 { "Europe/Istanbul" } else { "America/New_York" }).unwrap() }
#[no_mangle]
pub extern "C" fn offset_s(z: u32, t: i64) -> i32 { tz(z).offset_at(t) }
#[no_mangle]
pub extern "C" fn local_to_utc(z: u32, y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
    let local = qp_calendar::days_from_civil(y.into(), m, d) * 86_400 + i64::from(hh * 3600 + mm * 60);
    match tz(z).local_to_utc(local) { LocalTime::Unique(t) | LocalTime::Ambiguous(t, _) => t, LocalTime::Gap => i64::MIN }
}
