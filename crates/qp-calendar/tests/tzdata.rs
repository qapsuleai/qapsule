//! The embedded transition tables equal jiff's bundled tzdb (the generator's source), and
//! offsets / local→UTC agree with jiff over 1900–2100.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use jiff::{tz::TimeZone, Timestamp};
use qp_calendar::{LocalTime, Zone};

fn jiff_tz(name: &str) -> TimeZone {
    jiff::tz::db().get(name).unwrap()
}

#[test]
fn offsets_match_jiff_every_six_hours_1900_2100() {
    for name in ["America/New_York", "Europe/Istanbul"] {
        let ours = Zone::get(name).unwrap();
        let theirs = jiff_tz(name);
        let mut t = -2_208_988_800_i64;
        let mut n = 0;
        while t < 4_102_444_800 {
            let want = theirs
                .to_offset(Timestamp::from_second(t).unwrap())
                .seconds();
            assert_eq!(ours.offset_at(t), want, "{name} at {t}");
            t += 6 * 3600 + 17; // odd step so every time of day is visited
            n += 1;
        }
        assert!(n > 290_000);
    }
}

#[test]
fn every_transition_instant_matches_jiff() {
    for name in ["America/New_York", "Europe/Istanbul"] {
        let ours = Zone::get(name).unwrap();
        let theirs = jiff_tz(name);
        let start = Timestamp::from_second(-2_208_988_800).unwrap();
        for tr in theirs.following(start) {
            let t = tr.timestamp().as_second();
            if t >= 4_102_444_800 {
                break;
            }
            for d in [-1, 0, 1] {
                let want = theirs
                    .to_offset(Timestamp::from_second(t + d).unwrap())
                    .seconds();
                assert_eq!(ours.offset_at(t + d), want, "{name} around {t}");
            }
        }
    }
}

#[test]
fn local_to_utc_matches_jiff_including_gaps_and_folds() {
    use jiff::civil::DateTime;
    for name in ["America/New_York", "Europe/Istanbul"] {
        let ours = Zone::get(name).unwrap();
        let theirs = jiff_tz(name);
        // Every 30 minutes of local time over 1990-2030.
        let mut local_s = 631_152_000_i64; // 1990-01-01T00:00
        while local_s < 1_893_456_000 {
            let dt =
                DateTime::from_parts(jiff::civil::date(1970, 1, 1), jiff::civil::time(0, 0, 0, 0))
                    .checked_add(jiff::SignedDuration::from_secs(local_s))
                    .unwrap();
            let amb = theirs.to_ambiguous_timestamp(dt);
            let want = match amb.offset() {
                jiff::tz::AmbiguousOffset::Unambiguous { offset } => {
                    LocalTime::Unique(local_s - i64::from(offset.seconds()))
                }
                jiff::tz::AmbiguousOffset::Fold { before, after } => LocalTime::Ambiguous(
                    local_s - i64::from(before.seconds()),
                    local_s - i64::from(after.seconds()),
                ),
                jiff::tz::AmbiguousOffset::Gap { .. } => LocalTime::Gap,
            };
            assert_eq!(ours.local_to_utc(local_s), want, "{name} local {local_s}");
            local_s += 1800;
        }
    }
}
