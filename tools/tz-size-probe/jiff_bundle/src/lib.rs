use jiff::{civil::date, tz::TimeZone, Timestamp};
fn tz(z: u32) -> TimeZone { jiff::tz::db().get(if z == 0 { "Europe/Istanbul" } else { "America/New_York" }).unwrap() }
#[no_mangle]
pub extern "C" fn offset_s(z: u32, t: i64) -> i32 {
    tz(z).to_offset(Timestamp::from_second(t).unwrap()).seconds()
}
#[no_mangle]
pub extern "C" fn local_to_utc(z: u32, y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
    date(y as i16, m as i8, d as i8).at(hh as i8, mm as i8, 0, 0).to_zoned(tz(z)).map_or(i64::MIN, |zd| zd.timestamp().as_second())
}
