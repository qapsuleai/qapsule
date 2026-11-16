//! Independent check of qp_capsule::realize::elapsed against hand-written answers.
use qp_calendar::{builtin, ExchangeCalendar};
use qp_capsule::realize::elapsed::{elapsed, phrase, Period};
use qp_capsule::Lang;

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// Istanbul local (UTC+3, no DST since 2016) -> UTC ms.
fn t(s: &str) -> i64 {
    let (date, time) = s.split_once(' ').unwrap_or((s, "00:00"));
    let p: Vec<i64> = date.split('-').map(|x| x.parse().unwrap()).collect();
    let q: Vec<i64> = time.split(':').map(|x| x.parse().unwrap()).collect();
    (days_from_civil(p[0], p[1], p[2]) * 86400 + (q[0] - 3) * 3600 + q[1] * 60) * 1000
}

fn main() {
    let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
    // (period, event, now, expected TR, expected EN, note) — expected written by hand from the
    // REAL Borsa Istanbul calendar (official holidays), independent of qp-calendar's data.
    let cases: &[(&str, &str, &str, &str, &str, &str)] = &[
        // Daily, 2026 (holidays in the builtin sample list)
        ("1d", "2026-03-19 00:00", "2026-03-23 00:00", "perşembe günü", "on Thursday", "Ramazan B. 20 Mar Fri + weekend; 19 Mar half day"),
        ("1d", "2026-04-22 00:00", "2026-04-27 00:00", "2 işlem günü önce", "2 trading days ago", "23 Apr Thu holiday: 24 Fri, 27 Mon"),
        ("1d", "2026-05-15 00:00", "2026-05-20 00:00", "2 işlem günü önce", "2 trading days ago", "19 May Tue holiday: 18 Mon, 20 Wed"),
        // Daily, 2026 Kurban Bayramı 27-29 May (Wed-Fri), 26 May half day — NOT in builtin list
        ("1d", "2026-05-26 00:00", "2026-06-01 00:00", "salı günü", "on Tuesday", "Kurban B. 27-29 May 2026"),
        ("1d", "2026-05-25 00:00", "2026-06-02 00:00", "3 işlem günü önce", "3 trading days ago", "26 half, 27-29 holiday, 1 Jun, 2 Jun"),
        // Daily, 2026: 15 Jul Wed holiday — NOT in builtin list
        ("1d", "2026-07-14 00:00", "2026-07-16 00:00", "salı günü", "on Tuesday", "15 Jul 2026 holiday"),
        // Daily, 2025 (no 2025 holiday in builtin list)
        ("1d", "2025-04-30 00:00", "2025-05-02 00:00", "çarşamba günü", "on Wednesday", "1 May 2025 Thu"),
        ("1d", "2025-03-28 00:00", "2025-04-02 00:00", "cuma günü", "on Friday", "Ramazan B. 31 Mar-1 Apr 2025"),
        ("1d", "2025-06-04 00:00", "2025-06-12 00:00", "4 işlem günü önce", "4 trading days ago", "5 Jun half; 6 Jun, 9 Jun Kurban"),
        ("1d", "2025-10-27 00:00", "2025-11-04 00:00", "5 işlem günü önce", "5 trading days ago", "29 Oct 2025 Wed"),
        ("1d", "2025-07-14 00:00", "2025-07-23 00:00", "geçen hafta", "last week", "15 Jul 2025 Tue holiday; 6 trading days, previous ISO week"),
        // Daily, weekend only
        ("1d", "2026-11-13 00:00", "2026-11-16 00:00", "cuma günü", "on Friday", "weekend"),
        ("1d", "2026-11-12 00:00", "2026-11-13 00:00", "dün", "yesterday", "plain"),
        ("1d", "2026-10-02 00:00", "2026-11-16 00:00", "2 Ekim'de", "on 2 October", ">=20 trading days, same year"),
        ("1d", "2025-12-31 00:00", "2026-02-02 00:00", "31 Aralık 2025'te", "on 31 December 2025", "year shown"),
        // 60 minutes
        ("60m", "2026-03-19 12:00", "2026-03-23 11:00", "perşembe 12:00'de", "on Thursday at 12:00", "half day + Ramazan B. + weekend"),
        ("60m", "2026-04-30 15:00", "2026-05-04 10:00", "perşembe 15:00'te", "on Thursday at 15:00", "1 May + weekend"),
        ("60m", "2026-05-26 12:00", "2026-06-01 10:00", "salı 12:00'de", "on Tuesday at 12:00", "Kurban B. 2026 (not in list)"),
        ("60m", "2025-04-30 17:00", "2025-05-02 10:00", "çarşamba 17:00'de", "on Wednesday at 17:00", "1 May 2025 (not in list)"),
        ("60m", "2026-11-13 17:00", "2026-11-16 10:00", "cuma 17:00'de", "on Friday at 17:00", "weekend"),
        ("60m", "2026-11-16 10:00", "2026-11-16 15:00", "5 saat önce", "5 hours ago", "same day"),
        ("60m", "2026-11-16 14:00", "2026-11-16 15:00", "1 saat önce", "an hour ago", "same day, one hour"),
        ("60m", "2026-11-12 16:00", "2026-11-16 10:00", "2 işlem günü önce", "2 trading days ago", "Thu -> Mon"),
        ("60m", "2025-06-04 16:00", "2025-06-10 10:00", "2 işlem günü önce", "2 trading days ago", "Kurban 2025: 5 half, 6/9 holiday"),
    ];
    let mut ok = 0;
    let mut bad = 0;
    for (p, e, n, tr_exp, en_exp, note) in cases {
        let per = Period::parse(p).unwrap();
        let el = elapsed(&cal, per, t(e), t(n));
        let (tr, en) = (phrase(&el, Lang::Tr), phrase(&el, Lang::En));
        let pass = tr == *tr_exp && en == *en_exp;
        if pass { ok += 1 } else { bad += 1 }
        println!(
            "{} | {p:>3} | {e} -> {n} | got: {tr} / {en} | expected: {tr_exp} / {en_exp} | {note}",
            if pass { "PASS" } else { "FAIL" }
        );
    }
    println!("pass {ok} fail {bad}");
}
