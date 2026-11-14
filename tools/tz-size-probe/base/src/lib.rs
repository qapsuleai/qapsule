#[no_mangle]
pub extern "C" fn offset_s(z: u32, t: i64) -> i32 { (t as i32).wrapping_add(z as i32) }
#[no_mangle]
pub extern "C" fn local_to_utc(z: u32, y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 { (y as i64) + (m + d + hh + mm + z) as i64 }
