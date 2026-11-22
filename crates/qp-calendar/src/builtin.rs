//! Built-in calendars: [`builtin`] and the `XIST` closure tables (out of `lib.rs`).

use qp_schema::{Calendar, HalfDay, Session, Weekday};

/// Built-in calendars: `XIST` (Borsa Istanbul equities) and `XNYS` (NYSE).
///
/// `XIST` carries Borsa Istanbul's full and half-day closures for 2018–2027 (R4, see
/// [`XIST_HOLIDAYS`] and [`XIST_HALF_DAYS`]). `XNYS` holidays and half days are still a
/// few 2026 sample days only (R1 scope: mechanism, not full data).
#[must_use]
pub fn builtin(id: &str) -> Option<Calendar> {
    let s = |o: &str, c: &str| Session {
        open: o.to_owned(),
        close: c.to_owned(),
    };
    let strs = |v: &[&str]| v.iter().map(|d| (*d).to_owned()).collect::<Vec<_>>();
    let half = |d: &str, c: &str| HalfDay {
        date: d.to_owned(),
        close: c.to_owned(),
    };
    match id {
        // Continuous trading 10:00-18:00 Istanbul time (opening/closing auctions not
        // modelled as separate sessions).
        "XIST" => Some(Calendar {
            id: "XIST".to_owned(),
            tz: "Europe/Istanbul".to_owned(),
            sessions: vec![s("10:00", "18:00")],
            weekend: vec![Weekday::Sat, Weekday::Sun],
            holidays: strs(XIST_HOLIDAYS),
            half_days: XIST_HALF_DAYS
                .iter()
                .map(|d| half(d, XIST_HALF_DAY_CLOSE))
                .collect(),
        }),
        "XNYS" => Some(Calendar {
            id: "XNYS".to_owned(),
            tz: "America/New_York".to_owned(),
            sessions: vec![s("09:30", "16:00")],
            weekend: vec![Weekday::Sat, Weekday::Sun],
            // 2026 samples: New Year, Thanksgiving, Christmas.
            holidays: strs(&["2026-01-01", "2026-11-26", "2026-12-25"]),
            // Day after Thanksgiving and Christmas Eve close at 13:00.
            half_days: vec![half("2026-11-27", "13:00"), half("2026-12-24", "13:00")],
        }),
        _ => None,
    }
}

/// Borsa Istanbul full-day closures 2018–2027 on weekdays (closures falling on a weekend
/// are omitted: the weekend already closes them).
///
/// Fixed holidays (Law 2429): 1 January, 23 April, 1 May, 19 May, 15 July (since 2017),
/// 30 August, 29 October. Religious feasts: Ramadan Feast (3 days) and Feast of the
/// Sacrifice (4 days), dates as published by the Presidency of Religious Affairs
/// (Diyanet). Extraordinary closure: 8–14 February 2023 (market closed after the
/// 6 February earthquakes; the trades of 8 February were cancelled). No bridge-day
/// closures were declared for the exchange in this range (public-sector bridge days such
/// as 3 June 2019 or 8–9 April 2024 were trading days).
///
/// Checked against local real daily bars 2018-01-02 → 2026-11-16 (every weekday without
/// a bar is in this list and vice versa; `rounds/R4/experiments/FIX-E1.md`). 2026-11-17
/// onwards and 2027 follow the official holiday calendar (2027 feasts: Diyanet); Borsa
/// Istanbul publishes its own 2027 calendar in December, so 2027 is provisional.
#[rustfmt::skip]
pub const XIST_HOLIDAYS: &[&str] = &[
    // 2018: 1 Jan; 23 Apr; 1 May; Ramadan Feast 15 Jun (16–17 weekend); Feast of the
    // Sacrifice 21–24 Aug; 30 Aug; 29 Oct. (19 May, 15 Jul weekend.)
    "2018-01-01", "2018-04-23", "2018-05-01", "2018-06-15", "2018-08-21", "2018-08-22",
    "2018-08-23", "2018-08-24", "2018-08-30", "2018-10-29",
    // 2019: Ramadan 4–6 Jun; Sacrifice 11–14 Aug (11 Sun). (19 May weekend.)
    "2019-01-01", "2019-04-23", "2019-05-01", "2019-06-04", "2019-06-05", "2019-06-06",
    "2019-07-15", "2019-08-12", "2019-08-13", "2019-08-14", "2019-08-30", "2019-10-29",
    // 2020: Ramadan 24–26 May (24 Sun); Sacrifice 31 Jul–3 Aug. (30 Aug weekend.)
    "2020-01-01", "2020-04-23", "2020-05-01", "2020-05-19", "2020-05-25", "2020-05-26",
    "2020-07-15", "2020-07-31", "2020-08-03", "2020-10-29",
    // 2021: Ramadan 13–15 May; Sacrifice 20–23 Jul. (1 May weekend.)
    "2021-01-01", "2021-04-23", "2021-05-13", "2021-05-14", "2021-05-19", "2021-07-15",
    "2021-07-20", "2021-07-21", "2021-07-22", "2021-07-23", "2021-08-30", "2021-10-29",
    // 2022: Ramadan 2–4 May; Sacrifice 9–12 Jul. (1 Jan, 23 Apr, 1 May, 29 Oct weekend.)
    "2022-05-02", "2022-05-03", "2022-05-04", "2022-05-19", "2022-07-11", "2022-07-12",
    "2022-07-15", "2022-08-30",
    // 2023: earthquake closure 8–14 Feb; Ramadan 21–23 Apr; Sacrifice 28 Jun–1 Jul.
    // (1 Jan, 23 Apr, 15 Jul, 29 Oct weekend.)
    "2023-02-08", "2023-02-09", "2023-02-10", "2023-02-13", "2023-02-14", "2023-04-21",
    "2023-05-01", "2023-05-19", "2023-06-28", "2023-06-29", "2023-06-30", "2023-08-30",
    // 2024: Ramadan 10–12 Apr; Sacrifice 16–19 Jun (16 Sun). (19 May weekend.)
    "2024-01-01", "2024-04-10", "2024-04-11", "2024-04-12", "2024-04-23", "2024-05-01",
    "2024-06-17", "2024-06-18", "2024-06-19", "2024-07-15", "2024-08-30", "2024-10-29",
    // 2025: Ramadan 30 Mar–1 Apr (30 Sun); Sacrifice 6–9 Jun. (30 Aug weekend.)
    "2025-01-01", "2025-03-31", "2025-04-01", "2025-04-23", "2025-05-01", "2025-05-19",
    "2025-06-06", "2025-06-09", "2025-07-15", "2025-10-29",
    // 2026: Ramadan 20–22 Mar; Sacrifice 27–30 May. (30 Aug weekend.)
    "2026-01-01", "2026-03-20", "2026-04-23", "2026-05-01", "2026-05-19", "2026-05-27",
    "2026-05-28", "2026-05-29", "2026-07-15", "2026-10-29",
    // 2027 (provisional): Ramadan 9–11 Mar; Sacrifice 16–19 May (16 Sun). (1 May weekend.)
    "2027-01-01", "2027-03-09", "2027-03-10", "2027-03-11", "2027-04-23", "2027-05-17",
    "2027-05-18", "2027-05-19", "2027-07-15", "2027-08-30", "2027-10-29",
];

/// Borsa Istanbul half days 2018–2027 on weekdays: the eve (arife) of both feasts and
/// 28 October; trading closes at [`XIST_HALF_DAY_CLOSE`]. Observed in local real hourly
/// bars for 2024-11-15 → 2026-11-13 (every half day ends with the 12:00 bar); earlier half
/// days follow the same rule (daily bars cannot show them).
#[rustfmt::skip]
pub const XIST_HALF_DAYS: &[&str] = &[
    "2018-06-14", "2018-08-20", // (28 Oct Sun)
    "2019-06-03", "2019-10-28", // (Sacrifice eve 10 Aug Sat)
    "2020-07-30", "2020-10-28", // (Ramadan eve 23 May Sat)
    "2021-05-12", "2021-07-19", "2021-10-28",
    "2022-07-08", "2022-10-28", // (Ramadan eve 1 May Sun)
    "2023-04-20", "2023-06-27", // (28 Oct Sat)
    "2024-04-09", "2024-10-28", // (Sacrifice eve 15 Jun Sat)
    "2025-06-05", "2025-10-28", // (Ramadan eve 29 Mar Sat)
    "2026-03-19", "2026-05-26", "2026-10-28",
    "2027-03-08", "2027-10-28", // provisional (Sacrifice eve 15 May Sat)
];

/// Close of continuous trading on a Borsa Istanbul half day (local time).
pub const XIST_HALF_DAY_CLOSE: &str = "12:30";
