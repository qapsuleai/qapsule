//! `qp live inspect`: aggregate statistics of a raw log ([`inspect`], [`inspect_trades`]; RG1: split out of `live.rs`).

use super::*;

fn pct(v: &mut [i64], p: f64) -> i64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let i = ((v.len() - 1) as f64 * p).round() as usize;
    v[i]
}

/// Shape of a time string without its digits: `2026-11-19T14:02:03.123+03:00` →
/// `dddd-dd-ddTdd:dd:dd.ddd+dd:dd`.
pub(super) fn shape(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_digit() { 'd' } else { c })
        .collect()
}

/// Parses an ISO-like time (`YYYY-MM-DDTHH:MM:SS[.f][Z|±HH:MM]`) to UTC ms; a time
/// without offset is read as Istanbul (UTC+3). Returns (ms, had_offset).
pub(crate) fn parse_iso_ms(s: &str) -> Option<(i64, bool)> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ') {
        return None;
    }
    let num = |a: usize, n: usize| -> Option<i64> { s.get(a..a + n)?.parse().ok() };
    let (y, mo, d, h, mi, se) = (
        num(0, 4)?,
        num(5, 2)?,
        num(8, 2)?,
        num(11, 2)?,
        num(14, 2)?,
        num(17, 2)?,
    );
    let mut i = 19;
    let mut frac_ms = 0i64;
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        i = start;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let digits = &s[start..i];
        let mut v: String = digits.chars().take(3).collect();
        while v.len() < 3 {
            v.push('0');
        }
        frac_ms = v.parse().ok()?;
    }
    let rest = &s[i..];
    let (off_min, had) = if rest.is_empty() {
        (180, false)
    } else if rest == "Z" {
        (0, true)
    } else {
        let sign = match rest.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let r = rest[1..].replace(':', "");
        let oh: i64 = r.get(0..2)?.parse().ok()?;
        let om: i64 = r.get(2..4).unwrap_or("00").parse().ok()?;
        (sign * (oh * 60 + om), true)
    };
    let day = qp_calendar::days_from_civil(y, u32::try_from(mo).ok()?, u32::try_from(d).ok()?);
    let ms = ((day * 86_400 + h * 3600 + mi * 60 + se) - off_min * 60) * 1000 + frac_ms;
    Some((ms, had))
}

/// (RP4) Aggregate statistics of the trade messages of a raw log (no values: counts,
/// field presence, time-unit estimate, arrival lags, the side value set).
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct TradeInspect {
    /// `mx/trade/…` messages.
    pub regular: u64,
    /// `mx/tradeex/…` messages.
    pub special: u64,
    /// Messages per symbol.
    pub per_symbol: BTreeMap<String, u64>,
    /// Payloads that are not a `TradeMessage` (or were kept empty).
    pub decode_errors: u64,
    /// Payload symbol differs from the topic's.
    pub symbol_mismatch: u64,
    /// Messages carrying each field (wire tags; buyer / seller: presence only).
    pub fields: BTreeMap<&'static str, u64>,
    /// `TradeTimeUnit::Auto`'s pick per message (arrival as reference).
    pub auto_unit: BTreeMap<&'static str, u64>,
    /// Messages whose `timestamp` is a valid since-midnight nanosecond value.
    pub since_midnight_ns: u64,
    /// Messages without `timestamp`.
    pub no_timestamp: u64,
    /// Arrival − trade time (ms) with the `Auto` reading.
    pub lag_auto_ms: Vec<i64>,
    /// Arrival − trade time (ms) read as since-midnight ns (documented unit).
    pub lag_since_midnight_ms: Vec<i64>,
    /// `activeBidOrAsk` values as sent (at most 8 characters) and their counts.
    pub sides: BTreeMap<String, u64>,
    /// `isTradeEx = true`.
    pub trade_ex_flag: u64,
    /// Trade numbers seen twice for the same symbol.
    pub repeated_trade_no: u64,
    /// The recorder's sanitizing notes (`trade-sanitize …`).
    pub notes: Vec<String>,
}

impl TradeInspect {
    /// Trade messages.
    #[must_use]
    pub fn messages(&self) -> u64 {
        self.regular + self.special
    }
}

/// (RP4) [`TradeInspect`] of raw records.
#[must_use]
pub(crate) fn inspect_trades(recs: &[RawRecord]) -> TradeInspect {
    use qp_matriks::trade::resolve_trade_time;
    use qp_matriks::{
        decode_trade_raw, parse_topic, Topic, TradeDecodeConfig, TradeTimeUnit, TRADE_FIELDS,
    };
    let mut out = TradeInspect::default();
    let mut seen_no: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();
    let auto = TradeDecodeConfig {
        unit: TradeTimeUnit::Auto,
        ..TradeDecodeConfig::default()
    };
    let midnight = TradeDecodeConfig::default();
    for r in recs {
        if r.kind == RecordKind::Note {
            let n = String::from_utf8_lossy(&r.payload);
            if n.starts_with(TRADE_SANITIZE_NOTE) {
                out.notes.push(n.into_owned());
            }
            continue;
        }
        if r.kind != RecordKind::Message {
            continue;
        }
        let Topic::Trade {
            symbol, special, ..
        } = parse_topic(&r.topic).unwrap_mirror().clone()
        else {
            continue;
        };
        if special {
            out.special += 1;
        } else {
            out.regular += 1;
        }
        *out.per_symbol.entry(symbol.clone()).or_default() += 1;
        let Ok(tags) = qp_matriks::wire::field_numbers(&r.payload) else {
            out.decode_errors += 1;
            continue;
        };
        let Ok(v) = decode_trade_raw(&r.payload) else {
            out.decode_errors += 1;
            continue;
        };
        if r.payload.is_empty() {
            out.decode_errors += 1;
            continue;
        }
        for (n, name) in TRADE_FIELDS {
            if tags.contains(&n) {
                *out.fields.entry(name).or_default() += 1;
            }
        }
        if v.symbol != symbol {
            out.symbol_mismatch += 1;
        }
        if let Some(sd) = &v.side {
            *out.sides
                .entry(sd.trim().chars().take(8).collect())
                .or_default() += 1;
        }
        if v.is_trade_ex == Some(true) {
            out.trade_ex_flag += 1;
        }
        // Repeats of a trade number (kept in memory only, never printed).
        if let Some(no) = v.order_no.clone() {
            if !seen_no.insert((symbol.clone(), no)) {
                out.repeated_trade_no += 1;
            }
        }
        let arrival_ms = r.t_us.div_euclid(1000);
        let Some(raw) = v.timestamp else {
            out.no_timestamp += 1;
            continue;
        };
        if let Some((ns, u)) = resolve_trade_time(raw, &auto, arrival_ms) {
            *out.auto_unit.entry(u.name()).or_default() += 1;
            out.lag_auto_ms.push(arrival_ms - ns.div_euclid(1_000_000));
        }
        if let Some((ns, _)) = resolve_trade_time(raw, &midnight, arrival_ms) {
            out.since_midnight_ns += 1;
            out.lag_since_midnight_ms
                .push(arrival_ms - ns.div_euclid(1_000_000));
        }
    }
    out
}

fn print_trade_inspect(t: &mut TradeInspect) {
    let total = t.messages();
    println!(
        "trades: {total} messages ({} mx/trade, {} mx/tradeex), {} symbols, {} decode errors, \
         {} symbol mismatches",
        t.regular,
        t.special,
        t.per_symbol.len(),
        t.decode_errors,
        t.symbol_mismatch
    );
    let mut per: Vec<i64> = t
        .per_symbol
        .values()
        .map(|&n| i64::try_from(n).unwrap_or(i64::MAX))
        .collect();
    let (mn, md, mx) = (
        per.iter().min().copied().unwrap_or(0),
        pct(&mut per, 0.5),
        per.iter().max().copied().unwrap_or(0),
    );
    println!("trades per symbol: min {mn}, p50 {md}, max {mx}");
    println!("trade fields present (messages):");
    for (_, name) in qp_matriks::TRADE_FIELDS {
        let c = t.fields.get(name).copied().unwrap_or(0);
        let tag = if matches!(name, "buyer" | "seller") {
            " (level 2: presence in the kept log; must be 0)"
        } else {
            ""
        };
        println!("  {name}: {c}{tag}");
    }
    for n in &t.notes {
        println!("recorder: {n} (level-2 fields seen on the wire and dropped; counts only)");
    }
    println!(
        "trade time: auto unit picks {:?}; valid as ns since local midnight: {} of {}; \
         no timestamp: {}",
        t.auto_unit,
        t.since_midnight_ns,
        total - t.decode_errors,
        t.no_timestamp
    );
    for (label, v) in [
        ("auto", &mut t.lag_auto_ms),
        ("ns_from_midnight", &mut t.lag_since_midnight_ms),
    ] {
        let n = v.len();
        let (a, b) = (pct(v, 0.5), pct(v, 0.95));
        println!(
            "arrival - trade time [{label}] (ms): n {n}, min {}, p50 {a}, p95 {b}, max {}",
            v.first().copied().unwrap_or(0),
            v.last().copied().unwrap_or(0)
        );
    }
    println!("activeBidOrAsk values (count): {:?}", t.sides);
    println!(
        "isTradeEx=true: {}; repeated trade numbers: {}",
        t.trade_ex_flag, t.repeated_trade_no
    );
}

pub(super) fn inspect(path: &Path) -> Result<(), String> {
    use qp_matriks::{parse_topic, SymbolBook, Topic, UpdateKind};
    let (recs, tail) = read_all(path)?;
    let mut kinds: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut book = SymbolBook::new();
    let mut per_symbol: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut field_upd: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut field_init: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut shapes: BTreeMap<String, u64> = BTreeMap::new();
    let mut lag_upd: Vec<i64> = Vec::new();
    let mut lag_time: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut phases: BTreeMap<i32, u64> = BTreeMap::new();
    let mut sources: BTreeMap<String, u64> = BTreeMap::new();
    let mut clock = qp_matriks::MarketClock::new();
    let mut clock_events: Vec<String> = Vec::new();
    let mut decode_err = 0u64;
    let mut pub_reason: BTreeMap<i32, u64> = BTreeMap::new();
    let (mut first, mut last) = (i64::MAX, i64::MIN);
    for r in &recs {
        first = first.min(r.t_us);
        last = last.max(r.t_us);
        let k = match r.kind {
            RecordKind::Message => "message",
            RecordKind::ConnectionOpened => "connection_opened",
            RecordKind::ConnectionLost => "connection_lost",
            RecordKind::Subscribed => "subscribed",
            RecordKind::SubscribeRejected => "subscribe_rejected",
            RecordKind::Note => "note",
        };
        *kinds.entry(k).or_default() += 1;
        if r.kind == RecordKind::Subscribed {
            if let Topic::Symbol { symbol, .. } = parse_topic(&r.topic) {
                book.resubscribed(&symbol);
            }
        }
        if r.kind != RecordKind::Message {
            continue;
        }
        let t = parse_topic(&r.topic);
        match t.unwrap_mirror() {
            Topic::Symbol { symbol, .. } => match book.apply(symbol, &r.payload) {
                Ok(u) => {
                    let e = per_symbol.entry(symbol.clone()).or_default();
                    let target = if u.kind == UpdateKind::Initial {
                        e.0 += 1;
                        &mut field_init
                    } else {
                        e.1 += 1;
                        &mut field_upd
                    };
                    for n in u.fields.names() {
                        *target.entry(n).or_default() += 1;
                    }
                    if let Some(p) = u.stock_status {
                        *phases.entry(p).or_default() += 1;
                    }
                    if let Some(p) = u.publish_reason {
                        *pub_reason.entry(p).or_default() += 1;
                    }
                    if u.fields.contains_name("updateDate") {
                        if let Some(ud) = book.state(symbol).and_then(|s| s.update_date.clone()) {
                            *shapes.entry(shape(&ud)).or_default() += 1;
                            if let Some((ms, _)) = parse_iso_ms(&ud) {
                                lag_upd.push(r.t_us / 1000 - ms);
                            }
                        }
                    }
                }
                Err(_) => decode_err += 1,
            },
            Topic::Timestamp => match qp_matriks::decode_time(&r.payload) {
                Ok(tt) => {
                    *sources.entry(tt.source.clone()).or_default() += 1;
                    lag_time
                        .entry(tt.source.clone())
                        .or_default()
                        .push(r.t_us / 1000 - tt.millis());
                    for ev in clock.apply(&tt) {
                        clock_events.push(format!("{} {ev:?}", utc_stamp(r.t_us)));
                    }
                }
                Err(_) => decode_err += 1,
            },
            _ => {}
        }
    }
    println!("records: {} ({kinds:?})", recs.len());
    if let Some(t) = tail {
        println!("tail: {t}");
    }
    if first <= last {
        println!(
            "span: {} .. {} ({} s)",
            utc_stamp(first),
            utc_stamp(last),
            (last - first) / 1_000_000
        );
    }
    println!("decode errors: {decode_err}");
    println!("symbols: {} (initial, updates):", per_symbol.len());
    for (s, (i, u)) in &per_symbol {
        println!("  {s}: {i} initial, {u} updates");
    }
    println!("fields in initial messages (count):");
    for (n, c) in &field_init {
        println!("  {n}: {c}");
    }
    println!("fields in update messages (count):");
    let mut fu: Vec<_> = field_upd.iter().collect();
    fu.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (n, c) in fu {
        println!("  {n}: {c}");
    }
    println!("updateDate shapes: {shapes:?}");
    let n = lag_upd.len();
    let (p50, p95, mn, mx) = (
        pct(&mut lag_upd, 0.5),
        pct(&mut lag_upd, 0.95),
        lag_upd.first().copied().unwrap_or(0),
        lag_upd.last().copied().unwrap_or(0),
    );
    println!("arrival - updateDate (ms): n {n}, min {mn}, p50 {p50}, p95 {p95}, max {mx}");
    println!("timestamp sources: {sources:?}");
    for (s, v) in &mut lag_time {
        let n = v.len();
        let (a, b) = (pct(v, 0.5), pct(v, 0.95));
        println!(
            "arrival - timestamp[{s}] (ms): n {n}, min {}, p50 {a}, p95 {b}, max {}",
            v.first().copied().unwrap_or(0),
            v.last().copied().unwrap_or(0)
        );
    }
    println!("clock events: {clock_events:?}");
    println!("stockStatus codes seen (count): {phases:?}");
    println!("publishReason seen (count): {pub_reason:?}");
    let mut tr = inspect_trades(&recs);
    if tr.messages() > 0 || !tr.notes.is_empty() {
        print_trade_inspect(&mut tr);
    }
    Ok(())
}
