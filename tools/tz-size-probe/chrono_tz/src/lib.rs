use chrono::{Offset, TimeZone};
fn tz(z: u32) -> chrono_tz::Tz { if z == 0 { chrono_tz::Europe::Istanbul } else { chrono_tz::America::New_York } }
#[no_mangle]
pub extern "C" fn offset_s(z: u32, t: i64) -> i32 {
    let tz = tz(z);
    tz.timestamp_opt(t, 0).single().map_or(i32::MIN, |dt| dt.offset().fix().local_minus_utc())
}
#[no_mangle]
pub extern "C" fn local_to_utc(z: u32, y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
    tz(z).with_ymd_and_hms(y, m, d, hh, mm, 0).earliest().map_or(i64::MIN, |dt| dt.timestamp())
}
