//! R4 denetimi, VERIFY 12: süre ifadesi denetçinin kendi örnekleriyle (XIST takvimi).
//! Her satır: periyot, olay barının açılışı ve şimdiki barın açılışı (İstanbul yerel),
//! beklenen (denetçinin takvimden elle çıkardığı) TR / EN ve ürünün verdiği.
use qp_calendar::{builtin, ExchangeCalendar};
use qp_capsule::realize::elapsed::{elapsed, phrase, phrase_vague, Period};
use qp_capsule::Lang;

/// Istanbul local "YYYY-MM-DD HH:MM" (UTC+3, no DST since 2016) -> UTC ms.
fn t(s: &str) -> i64 {
    let (d, hm) = s.split_once(' ').unwrap();
    let p: Vec<i64> = d.split('-').map(|x| x.parse().unwrap()).collect();
    let q: Vec<i64> = hm.split(':').map(|x| x.parse().unwrap()).collect();
    // days from civil (Howard Hinnant)
    let (y, m, dd) = (p[0], p[1], p[2]);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days * 86_400 + q[0] * 3600 + q[1] * 60 - 3 * 3600) * 1000
}

fn main() {
    let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        // period, event, now, expected TR, expected EN
        ("1d", "2025-06-13 10:00", "2025-06-16 10:00", "cuma günü", "on Friday"),
        ("1d", "2025-06-05 10:00", "2025-06-10 10:00", "perşembe günü", "on Thursday"), // Kurban: 6 Fri, 9 Mon closed
        ("1d", "2025-06-04 10:00", "2025-06-10 10:00", "2 işlem günü önce", "2 trading days ago"),
        ("1d", "2025-12-31 10:00", "2026-01-02 10:00", "çarşamba günü", "on Wednesday"), // 1 Jan closed
        ("1d", "2025-10-28 10:00", "2025-10-30 10:00", "salı günü", "on Tuesday"), // 29 Oct closed, 28 half day
        ("1d", "2025-03-28 10:00", "2025-04-02 10:00", "cuma günü", "on Friday"), // Ramazan 31 Mar, 1 Apr closed
        ("1d", "2025-06-16 10:00", "2025-06-17 10:00", "dün", "yesterday"),
        ("1d", "2025-06-17 10:00", "2025-06-17 10:00", "bugün", "today"),
        ("1d", "2025-06-09 10:00", "2025-06-17 10:00", "geçen hafta", "last week"), // event on a holiday Monday (no bar in reality)
        ("1d", "2025-06-10 10:00", "2025-06-16 10:00", "4 işlem günü önce", "4 trading days ago"), // Tue..Mon: 4 işlem günü (2–5 kuralı geçen haftadan önce gelir)
        ("1d", "2025-06-02 10:00", "2025-06-16 10:00", "8 işlem günü önce", "8 trading days ago"),
        ("1d", "2025-05-12 10:00", "2025-06-16 10:00", "12 Mayıs'ta", "on 12 May"),
        ("1d", "2024-12-02 10:00", "2025-01-15 10:00", "2 Aralık 2024'te", "on 2 December 2024"),
        ("60m", "2025-06-16 11:00", "2025-06-16 15:00", "4 saat önce", "4 hours ago"),
        ("60m", "2025-06-16 15:00", "2025-06-16 15:00", "az önce", "just now"),
        ("60m", "2025-06-13 15:00", "2025-06-16 11:00", "cuma 15:00'te", "on Friday at 15:00"),
        ("60m", "2025-06-05 11:00", "2025-06-10 11:00", "perşembe 11:00'de", "on Thursday at 11:00"),
        ("60m", "2025-12-31 17:00", "2026-01-02 10:00", "çarşamba 17:00'de", "on Wednesday at 17:00"),
        ("60m", "2025-06-16 17:00", "2025-06-17 10:00", "dün 17:00'de", "yesterday at 17:00"),
        ("60m", "2025-06-11 10:00", "2025-06-16 12:00", "3 işlem günü önce", "3 trading days ago"),
        ("60m", "2025-10-28 12:00", "2025-10-30 10:00", "salı 12:00'de", "on Tuesday at 12:00"),
    ];
    let mut bad = 0;
    for (p, e, n, etr, een) in cases {
        let per = Period::parse(p).unwrap();
        let el = elapsed(&cal, per, t(e), t(n));
        let (tr, en) = (phrase(&el, Lang::Tr), phrase(&el, Lang::En));
        let vtr = phrase_vague(&el, Lang::Tr);
        let ok = tr == *etr && en == *een;
        if !ok {
            bad += 1;
        }
        println!(
            "{} {:>3} {} -> {} | ürün TR \"{}\" EN \"{}\" (rakamsız \"{}\") | beklenen \"{}\" / \"{}\"",
            if ok { "OK  " } else { "FARK" },
            p,
            e,
            n,
            tr,
            en,
            vtr,
            etr,
            een
        );
    }
    println!("{} örnek, {} fark", cases.len(), bad);
}
