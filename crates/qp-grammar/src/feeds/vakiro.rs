//! M9 alternative `vakiro`: the feed in time sections.
//!
//! The feed reads newest to oldest, cut by everyday time sections whose texts are written
//! here in the reader's language, from the exchange calendar day:
//!
//! 1. **Since your last look** (only with a last look): every capsule whose latest
//!    transition became known after it (`FeedCtx::fresh`, the core's rule). The header
//!    says when that look was ("dün 15:00"); with nothing new it says so and stays alone.
//! 2. The rest, by the local day of the latest transition (`CapsuleItem.t`, the same time
//!    the face's elapsed phrase reads) against now's local day: **today**, **yesterday**,
//!    **last 7 days**, **earlier**. Without a clock (`now`), all of it is **earlier**.
//!
//! Sections run newest to oldest; inside a section the authority's order stays (importance,
//! then latest transition): the section already says when, the order says what matters
//! most. Empty sections have no header. A capsule about the reader's position or intent
//! (`relevant`) carries the mark "pozisyonun" / "your position".
//!
//! Local day: the exchange's zone, `Europe/Istanbul`, fixed UTC+03:00 since 2016 (no
//! daylight saving); the shell core's display zone is the same (`qp_wasm::shell::DISPLAY_TZ`).
//! Integer arithmetic only; same items, language, `seen` and `now` → same rows.

use qp_schema::protocol::{CapsuleItem, Language};

use super::{FeedCtx, FeedRow};

/// Rules / parameters: last-look section, calendar sections (today, yesterday, 7 days,
/// earlier), position mark, zone offset.
pub const RULES: u32 = 4;

const MS_PER_MIN: i64 = 60_000;
const MS_PER_DAY: i64 = 86_400_000;
/// `Europe/Istanbul` (UTC+03:00, no daylight saving since 2016).
const ZONE_OFFSET_MS: i64 = 3 * 60 * MS_PER_MIN;
/// The rolling-week section covers this many local days back (yesterday excluded).
const WEEK_DAYS: i64 = 7;

fn local_ms(t: i64) -> i64 {
    t + ZONE_OFFSET_MS
}

fn local_day(t: i64) -> i64 {
    local_ms(t).div_euclid(MS_PER_DAY)
}

/// Monday = 0 (1970-01-01 was a Thursday).
fn weekday(day: i64) -> usize {
    usize::try_from((day + 3).rem_euclid(7)).unwrap_or(0)
}

fn hhmm(t: i64) -> String {
    let m = local_ms(t).rem_euclid(MS_PER_DAY) / MS_PER_MIN;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Sections after the last-look one, oldest last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Today,
    Yesterday,
    Week,
    Earlier,
}

fn section(t: i64, now: Option<i64>) -> Section {
    let Some(now) = now else {
        return Section::Earlier;
    };
    match local_day(now) - local_day(t) {
        d if d <= 0 => Section::Today,
        1 => Section::Yesterday,
        d if d <= WEEK_DAYS => Section::Week,
        _ => Section::Earlier,
    }
}

fn section_text(s: Section, lang: Language) -> &'static str {
    match (s, lang) {
        (Section::Today, Language::Tr) => "Bugün",
        (Section::Today, Language::En) => "Today",
        (Section::Yesterday, Language::Tr) => "Dün",
        (Section::Yesterday, Language::En) => "Yesterday",
        (Section::Week, Language::Tr) => "Son 7 gün",
        (Section::Week, Language::En) => "Last 7 days",
        (Section::Earlier, Language::Tr) => "Daha önce",
        (Section::Earlier, Language::En) => "Earlier",
    }
}

const WEEKDAYS_TR: [&str; 7] = [
    "pazartesi",
    "salı",
    "çarşamba",
    "perşembe",
    "cuma",
    "cumartesi",
    "pazar",
];
const WEEKDAYS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// When the last look was, in everyday words: "bugün 15:00", "dün 15:00", "pazartesi
/// 15:00" (within the week), "9 gün önce".
fn when(seen: i64, now: Option<i64>, lang: Language) -> String {
    let d = now.map_or(0, |n| local_day(n) - local_day(seen));
    let tr = matches!(lang, Language::Tr);
    match d {
        d if d <= 0 => {
            if tr {
                format!("bugün {}", hhmm(seen))
            } else {
                format!("today {}", hhmm(seen))
            }
        }
        1 => {
            if tr {
                format!("dün {}", hhmm(seen))
            } else {
                format!("yesterday {}", hhmm(seen))
            }
        }
        d if d < WEEK_DAYS => {
            let w = weekday(local_day(seen));
            if tr {
                format!("{} {}", WEEKDAYS_TR[w], hhmm(seen))
            } else {
                format!("{} {}", WEEKDAYS_EN[w], hhmm(seen))
            }
        }
        d => {
            if tr {
                format!("{d} gün önce")
            } else {
                format!("{d} days ago")
            }
        }
    }
}

fn mark(i: &CapsuleItem, lang: Language) -> Option<String> {
    i.relevant.then(|| {
        match lang {
            Language::Tr => "pozisyonun",
            Language::En => "your position",
        }
        .to_owned()
    })
}

fn push_rows(rows: &mut Vec<FeedRow>, items: &[&CapsuleItem], lang: Language) {
    rows.extend(items.iter().map(|i| FeedRow::Capsule {
        id: i.id.clone(),
        mark: mark(i, lang),
    }));
}

/// The layout (module docs).
pub fn layout(c: &FeedCtx<'_>) -> Vec<FeedRow> {
    let fresh = |k: usize| c.seen.is_some() && c.fresh.get(k).copied().unwrap_or(false);
    let mut rows = Vec::new();
    if let Some(seen) = c.seen {
        let new: Vec<&CapsuleItem> = c
            .items
            .iter()
            .enumerate()
            .filter(|(k, _)| fresh(*k))
            .map(|(_, i)| i)
            .collect();
        let at = when(seen, c.now, c.lang);
        let text = match (new.is_empty(), c.lang) {
            (false, Language::Tr) => format!("Son baktığından beri · {at}"),
            (false, Language::En) => format!("Since your last look · {at}"),
            (true, Language::Tr) => format!("Son baktığından beri yeni bir şey yok · {at}"),
            (true, Language::En) => format!("Nothing new since your last look · {at}"),
        };
        rows.push(FeedRow::Header { text });
        push_rows(&mut rows, &new, c.lang);
    }
    let rest: Vec<(Section, &CapsuleItem)> = c
        .items
        .iter()
        .enumerate()
        .filter(|(k, _)| !fresh(*k))
        .map(|(_, i)| (section(i.t, c.now), i))
        .collect();
    for s in [
        Section::Today,
        Section::Yesterday,
        Section::Week,
        Section::Earlier,
    ] {
        let part: Vec<&CapsuleItem> = rest
            .iter()
            .filter(|(x, _)| *x == s)
            .map(|(_, i)| *i)
            .collect();
        if part.is_empty() {
            continue;
        }
        rows.push(FeedRow::Header {
            text: section_text(s, c.lang).to_owned(),
        });
        push_rows(&mut rows, &part, c.lang);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_helpers() {
        // 2023-09-19 12:00 UTC = Tuesday 15:00 in Istanbul.
        let t = 1_695_124_800_000;
        assert_eq!(hhmm(t), "15:00");
        assert_eq!(weekday(local_day(t)), 1);
        // 22:30 UTC is already the next local day.
        let late = 1_695_162_600_000; // 2023-09-19 22:30 UTC
        assert_eq!(local_day(late), local_day(t) + 1);
        assert_eq!(hhmm(late), "01:30");
        let day = MS_PER_DAY;
        assert_eq!(when(t, Some(t + day), Language::Tr), "dün 15:00");
        assert_eq!(when(t, Some(t + day), Language::En), "yesterday 15:00");
        assert_eq!(when(t, Some(t + 2 * day), Language::Tr), "salı 15:00");
        assert_eq!(when(t, Some(t + 9 * day), Language::En), "9 days ago");
        assert_eq!(when(t, None, Language::Tr), "bugün 15:00");
        assert_eq!(section(t, Some(t)), Section::Today);
        assert_eq!(section(t, Some(t + day)), Section::Yesterday);
        assert_eq!(section(t, Some(t + 7 * day)), Section::Week);
        assert_eq!(section(t, Some(t + 8 * day)), Section::Earlier);
        assert_eq!(section(t, None), Section::Earlier);
    }

    #[test]
    fn empty_feed_with_a_last_look_says_nothing_new() {
        let ctx = FeedCtx {
            items: &[],
            lang: Language::Tr,
            seen: Some(1_695_124_800_000),
            fresh: &[],
            now: Some(1_695_211_200_000),
        };
        assert_eq!(
            layout(&ctx),
            vec![FeedRow::Header {
                text: "Son baktığından beri yeni bir şey yok · dün 15:00".into()
            }]
        );
        let none = FeedCtx { seen: None, ..ctx };
        assert!(layout(&none).is_empty());
    }
}
