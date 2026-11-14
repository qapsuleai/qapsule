//! Hand-written rules: Istanbul fixed +03:00 (valid only since 2016-09-07), New York US
//! DST rules (valid only since 2007). No historical data: size reference only.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
fn nth_sunday(y: i64, m: i64, n: i64) -> i64 {
    let first = days_from_civil(y, m, 1);
    let wd = (first + 4).rem_euclid(7); // 0 = Sunday
    first + (7 - wd) % 7 + 7 * (n - 1)
}
fn ny_offset(t: i64) -> i32 {
    let days = t.div_euclid(86_400);
    let y = (days as f64 / 365.2425) as i64 + 1970;
    let start = nth_sunday(y, 3, 2) * 86_400 + 7 * 3600; // 02:00 EST = 07:00 UTC
    let end = nth_sunday(y, 11, 1) * 86_400 + 6 * 3600; // 02:00 EDT = 06:00 UTC
    if t >= start && t < end { -4 * 3600 } else { -5 * 3600 }
}
#[no_mangle]
pub extern "C" fn offset_s(z: u32, t: i64) -> i32 { if z == 0 { 3 * 3600 } else { ny_offset(t) } }
#[no_mangle]
pub extern "C" fn local_to_utc(z: u32, y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
    let local = days_from_civil(y.into(), m.into(), d.into()) * 86_400 + i64::from(hh * 3600 + mm * 60);
    let guess = local - i64::from(offset_s(z, local));
    local - i64::from(offset_s(z, guess))
}
