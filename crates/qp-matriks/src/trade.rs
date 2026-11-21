//! Trades (işlem) from the Matriks trade endpoint (RP4, K66).
//!
//! `TradeMessage` (`proto/Trade.proto`) on `mx/trade/<SYM>` / `mx/tradeex/<SYM>` becomes a
//! [`Trade`] with **level-1 fields only**: symbol, price, quantity, side
//! (`activeBidOrAsk`), time, trade number (`orderNo`) and `isTradeEx`.
//!
//! **Buyer and seller (fields 7 and 8, level 2, AKDE) are never kept.** Decoding goes
//! through a private prost type that does not declare them, so prost skips them on the
//! wire: they are not copied into any struct, not stored, not logged.
//! [`sanitize_trade_payload`] re-encodes a payload without them (for a raw log).
//!
//! **Time.** The documentation shows `timestamp` as nanoseconds since the start of the
//! day (Istanbul); this is **not verified** on a live connection yet. The unit is a
//! setting ([`TradeTimeUnit`]); [`TradeTimeUnit::Auto`] picks, per message, the reading
//! closest to the reference time (arrival). The choice is left to a measurement on the
//! first trading day (RP4 report).
//!
//! **Price.** The wire price is a 32-bit float. It is widened through its shortest decimal
//! text (`12.34f32` → `12.34`), not bit by bit (`12.340000152587890625`), so a live price
//! equals the same price parsed from REST text.
//!
//! Pure: no network, no clock; the reference time is passed in.

use std::cmp::Ordering;

use prost::Message as _;

use crate::DecodeError;

/// Level-1 view of `TradeMessage`. Fields 7 (`buyer`) and 8 (`seller`) are deliberately
/// not declared: prost skips undeclared fields while decoding and keeps nothing of them.
#[derive(Clone, PartialEq, prost::Message)]
struct TradeLevel1 {
    #[prost(string, required, tag = "1")]
    symbol: String,
    #[prost(string, optional, tag = "2")]
    order_no: Option<String>,
    #[prost(float, optional, tag = "3")]
    price: Option<f32>,
    #[prost(int64, optional, tag = "4")]
    quantity: Option<i64>,
    #[prost(string, optional, tag = "5")]
    active_bid_or_ask: Option<String>,
    #[prost(int64, optional, tag = "6")]
    timestamp: Option<i64>,
    #[prost(bool, optional, tag = "9")]
    is_trade_ex: Option<bool>,
}

/// Which side was active (`activeBidOrAsk`). The value set is not documented; the first
/// letter is read (`B…` = bid side active, `A…`/`S…` = ask side active).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Aggressor {
    /// Bid side active (buyer-initiated).
    Bid,
    /// Ask side active (seller-initiated).
    Ask,
    /// Anything else, as sent.
    Other(String),
}

impl Aggressor {
    /// Reads `activeBidOrAsk`.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw.trim().chars().next().map(|c| c.to_ascii_uppercase()) {
            Some('B') => Aggressor::Bid,
            Some('A' | 'S') => Aggressor::Ask,
            _ => Aggressor::Other(raw.trim().to_owned()),
        }
    }
}

/// Unit of the wire `timestamp` (module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TradeTimeUnit {
    /// Nanoseconds since local midnight (documented; unverified). The date is the local
    /// date of the reference time.
    #[default]
    NanosFromMidnight,
    /// Unix epoch milliseconds.
    EpochMillis,
    /// Unix epoch microseconds.
    EpochMicros,
    /// Unix epoch nanoseconds.
    EpochNanos,
    /// Per message, the reading of the four above closest to the reference time.
    Auto,
}

impl TradeTimeUnit {
    /// Short name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::NanosFromMidnight => "ns_from_midnight",
            Self::EpochMillis => "epoch_ms",
            Self::EpochMicros => "epoch_us",
            Self::EpochNanos => "epoch_ns",
            Self::Auto => "auto",
        }
    }

    const CONCRETE: [TradeTimeUnit; 4] = [
        Self::NanosFromMidnight,
        Self::EpochMillis,
        Self::EpochMicros,
        Self::EpochNanos,
    ];
}

/// Decoding settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeDecodeConfig {
    /// Unit of `timestamp`.
    pub unit: TradeTimeUnit,
    /// Local UTC offset for [`TradeTimeUnit::NanosFromMidnight`], minutes (Istanbul: 180,
    /// all year since 2016).
    pub utc_offset_min: i32,
}

impl Default for TradeDecodeConfig {
    fn default() -> Self {
        Self {
            unit: TradeTimeUnit::NanosFromMidnight,
            utc_offset_min: 180,
        }
    }
}

const NS_PER_MS: i64 = 1_000_000;
const MS_PER_DAY: i64 = 86_400_000;
const NS_PER_DAY: i64 = MS_PER_DAY * NS_PER_MS;

/// UTC epoch nanoseconds of a raw `timestamp` in `unit` (not `Auto`), with the reference
/// time `ref_ms` (UTC ms) giving the date for `NanosFromMidnight`.
fn resolve(raw: i64, unit: TradeTimeUnit, ref_ms: i64, off_min: i32) -> Option<i64> {
    match unit {
        TradeTimeUnit::NanosFromMidnight => {
            if !(0..NS_PER_DAY).contains(&raw) {
                return None;
            }
            let off_ms = i64::from(off_min) * 60_000;
            let day = (ref_ms + off_ms).div_euclid(MS_PER_DAY);
            let midnight_ms = day * MS_PER_DAY - off_ms;
            midnight_ms.checked_mul(NS_PER_MS)?.checked_add(raw)
        }
        TradeTimeUnit::EpochMillis => raw.checked_mul(NS_PER_MS),
        TradeTimeUnit::EpochMicros => raw.checked_mul(1000),
        TradeTimeUnit::EpochNanos => Some(raw),
        TradeTimeUnit::Auto => None,
    }
}

/// Resolves `raw` per `cfg`; returns the UTC epoch ns and the unit actually used.
#[must_use]
pub fn resolve_trade_time(
    raw: i64,
    cfg: &TradeDecodeConfig,
    ref_ms: i64,
) -> Option<(i64, TradeTimeUnit)> {
    if cfg.unit != TradeTimeUnit::Auto {
        return resolve(raw, cfg.unit, ref_ms, cfg.utc_offset_min).map(|ns| (ns, cfg.unit));
    }
    let ref_ns = i128::from(ref_ms) * i128::from(NS_PER_MS);
    TradeTimeUnit::CONCRETE
        .iter()
        .filter_map(|&u| resolve(raw, u, ref_ms, cfg.utc_offset_min).map(|ns| (ns, u)))
        .min_by_key(|(ns, _)| (i128::from(*ns) - ref_ns).unsigned_abs())
}

/// One trade, level-1 fields only (module docs). No buyer, no seller.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    /// Matriks symbol code.
    pub symbol: String,
    /// Trade price.
    pub price: f64,
    /// Quantity (lots); `None` at level 0.
    pub quantity: Option<i64>,
    /// Active side (`activeBidOrAsk`); `None` at level 0.
    pub side: Option<Aggressor>,
    /// Trade time, UTC epoch nanoseconds.
    pub timestamp_ns: i64,
    /// Trade number (`orderNo`; REST `mx_id`); `None` at level 0.
    pub order_no: Option<String>,
    /// A special (reported) trade: `isTradeEx`, or received on `mx/tradeex/…`.
    pub is_trade_ex: bool,
}

/// De-duplication key of a trade (see [`Trade::dedup_key`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TradeKey {
    /// By trade number (the normal case).
    No(String),
    /// No trade number (level 0): time, price bits, quantity and kind.
    Fields(i64, u64, Option<i64>, bool),
}

impl Trade {
    /// UTC epoch milliseconds (floor).
    #[must_use]
    pub fn t_ms(&self) -> i64 {
        self.timestamp_ns.div_euclid(NS_PER_MS)
    }

    /// Duplicates share this key: the trade number when there is one (the rule), else
    /// the time, price, quantity and kind.
    #[must_use]
    pub fn dedup_key(&self) -> TradeKey {
        match self.order_no.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => TradeKey::No(n.to_owned()),
            _ => TradeKey::Fields(
                self.timestamp_ns,
                self.price.to_bits(),
                self.quantity,
                self.is_trade_ex,
            ),
        }
    }

    /// Exchange order: time, then trade number (numerically when it is all digits), then
    /// the rest. Total and deterministic; the order trades are applied to bars in.
    #[must_use]
    pub fn exchange_cmp(&self, other: &Trade) -> Ordering {
        fn num(o: &Option<String>) -> (u8, u128, &str) {
            match o.as_deref().map(str::trim) {
                Some(s) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
                    (0, s.parse().unwrap_or(u128::MAX), s)
                }
                Some(s) => (1, 0, s),
                None => (2, 0, ""),
            }
        }
        self.timestamp_ns
            .cmp(&other.timestamp_ns)
            .then_with(|| num(&self.order_no).cmp(&num(&other.order_no)))
            .then_with(|| self.price.total_cmp(&other.price))
            .then_with(|| self.quantity.cmp(&other.quantity))
            .then_with(|| self.is_trade_ex.cmp(&other.is_trade_ex))
    }

    /// The bar input (`qp_source::TradeTick`).
    #[must_use]
    pub fn tick(&self) -> qp_source::TradeTick {
        qp_source::TradeTick {
            t_ms: self.t_ms(),
            price: self.price,
            quantity: self.quantity.map(|q| q as f64),
            special: self.is_trade_ex,
        }
    }
}

/// Why a trade payload gave no [`Trade`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeError {
    /// Not a `TradeMessage`.
    Decode(DecodeError),
    /// No (or a non-finite / non-positive) price.
    NoPrice,
    /// No `timestamp`, or one that does not resolve in the configured unit.
    NoTime,
}

impl std::fmt::Display for TradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeError::Decode(e) => write!(f, "trade: {e}"),
            TradeError::NoPrice => f.write_str("trade: no valid price"),
            TradeError::NoTime => f.write_str("trade: no usable timestamp"),
        }
    }
}

impl std::error::Error for TradeError {}

/// `f32` → `f64` through the shortest decimal text that round-trips the `f32`.
#[must_use]
pub fn widen_price(x: f32) -> f64 {
    format!("{x}").parse().unwrap_or(f64::from(x))
}

/// Decodes a `TradeMessage` payload (level 1 only). `ref_ms`: reference time (arrival,
/// UTC ms) for the date of a since-midnight `timestamp` and for `Auto`. `special_topic`:
/// the message came on `mx/tradeex/…`.
///
/// # Errors
/// [`TradeError`].
pub fn decode_trade(
    payload: &[u8],
    cfg: &TradeDecodeConfig,
    ref_ms: i64,
    special_topic: bool,
) -> Result<(Trade, TradeTimeUnit), TradeError> {
    let m = TradeLevel1::decode(payload).map_err(|e| TradeError::Decode(e.into()))?;
    let price = m
        .price
        .filter(|p| p.is_finite() && *p > 0.0)
        .map(widen_price)
        .ok_or(TradeError::NoPrice)?;
    let (timestamp_ns, unit) = m
        .timestamp
        .and_then(|raw| resolve_trade_time(raw, cfg, ref_ms))
        .ok_or(TradeError::NoTime)?;
    Ok((
        Trade {
            symbol: m.symbol,
            price,
            quantity: m.quantity,
            side: m.active_bid_or_ask.as_deref().map(Aggressor::parse),
            timestamp_ns,
            order_no: m.order_no,
            is_trade_ex: special_topic || m.is_trade_ex.unwrap_or(false),
        },
        unit,
    ))
}

/// The payload re-encoded with level-1 fields only (buyer and seller dropped), for a raw
/// log. Field values are unchanged.
///
/// # Errors
/// Not a `TradeMessage`.
pub fn sanitize_trade_payload(payload: &[u8]) -> Result<Vec<u8>, DecodeError> {
    Ok(TradeLevel1::decode(payload)?.encode_to_vec())
}

/// Trade history for a gap (REST `trade.gz`; RP4). The live connector's REST client is
/// adapted to this by its caller; tests use a fake.
pub trait TradeHistory {
    /// Trades of `symbol` with time in `[start_ms, end_ms]` (UTC ms, inclusive), in any
    /// order; duplicates allowed (the feed removes them by trade number).
    ///
    /// # Errors
    /// A text for the caller's log (already masked).
    fn trades(&self, symbol: &str, start_ms: i64, end_ms: i64) -> Result<Vec<Trade>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::TradeMessage;

    // 2026-11-18 10:00:00 Istanbul = 07:00:00 UTC.
    const T10_MS: i64 = 1_794_985_200_000;
    const SINCE_MIDNIGHT_10H_NS: i64 = 10 * 3_600 * 1_000_000_000;

    fn wire(ts: i64) -> Vec<u8> {
        TradeMessage {
            symbol: "TEST1".into(),
            order_no: Some("123456".into()),
            price: Some(12.34),
            quantity: Some(150),
            active_bid_or_ask: Some("B".into()),
            timestamp: Some(ts),
            buyer: Some("BUYERX".into()),
            seller: Some("SELLERY".into()),
            is_trade_ex: Some(false),
        }
        .encode_to_vec()
    }

    #[test]
    fn calendar_constant_is_ten_o_clock() {
        assert_eq!(
            T10_MS,
            (qp_calendar::days_from_civil(2026, 11, 18) * 86_400 + 7 * 3600) * 1000
        );
    }

    #[test]
    fn decodes_level1_and_never_keeps_buyer_or_seller() {
        let ref_ms = T10_MS + 250;
        let (t, unit) = decode_trade(
            &wire(SINCE_MIDNIGHT_10H_NS),
            &TradeDecodeConfig::default(),
            ref_ms,
            false,
        )
        .unwrap();
        assert_eq!(unit, TradeTimeUnit::NanosFromMidnight);
        assert_eq!(t.symbol, "TEST1");
        assert_eq!(t.price, 12.34);
        assert_eq!(t.quantity, Some(150));
        assert_eq!(t.side, Some(Aggressor::Bid));
        assert_eq!(t.order_no.as_deref(), Some("123456"));
        assert!(!t.is_trade_ex);
        assert_eq!(t.t_ms(), T10_MS);
        let text = format!("{t:?}");
        assert!(
            !text.contains("BUYERX") && !text.contains("SELLERY"),
            "{text}"
        );

        // Sanitized payload: same level-1 fields, no buyer / seller bytes.
        let clean = sanitize_trade_payload(&wire(5)).unwrap();
        assert!(!clean.windows(6).any(|w| w == b"BUYERX"));
        assert!(!clean.windows(7).any(|w| w == b"SELLERY"));
        let back = TradeMessage::decode(clean.as_slice()).unwrap();
        assert_eq!((back.buyer, back.seller), (None, None));
        assert_eq!(back.order_no.as_deref(), Some("123456"));
        assert_eq!(back.timestamp, Some(5));
    }

    #[test]
    fn time_unit_is_configurable_and_auto_picks_the_closest() {
        let ref_ms = T10_MS + 400;
        let cases = [
            (TradeTimeUnit::NanosFromMidnight, SINCE_MIDNIGHT_10H_NS),
            (TradeTimeUnit::EpochMillis, T10_MS),
            (TradeTimeUnit::EpochMicros, T10_MS * 1000),
            (TradeTimeUnit::EpochNanos, T10_MS * 1_000_000),
        ];
        for (unit, raw) in cases {
            let cfg = TradeDecodeConfig {
                unit,
                ..TradeDecodeConfig::default()
            };
            let (t, u) = decode_trade(&wire(raw), &cfg, ref_ms, false).unwrap();
            assert_eq!((t.t_ms(), u), (T10_MS, unit), "{}", unit.name());
            let auto = TradeDecodeConfig {
                unit: TradeTimeUnit::Auto,
                ..TradeDecodeConfig::default()
            };
            let (t, u) = decode_trade(&wire(raw), &auto, ref_ms, false).unwrap();
            assert_eq!((t.t_ms(), u), (T10_MS, unit), "auto for {}", unit.name());
        }
        // An epoch value read as since-midnight does not resolve; a negative one neither.
        let cfg = TradeDecodeConfig::default();
        assert_eq!(
            decode_trade(&wire(T10_MS * 1_000_000), &cfg, ref_ms, false).unwrap_err(),
            TradeError::NoTime
        );
        assert_eq!(
            decode_trade(&wire(-1), &cfg, ref_ms, false).unwrap_err(),
            TradeError::NoTime
        );
        // The date of a since-midnight value is the local date of the reference: a
        // reference just after local midnight (21:00:30 UTC) is the next local day.
        let local_midnight_utc = T10_MS - 10 * 3_600_000;
        let (t, _) =
            decode_trade(&wire(1_000_000), &cfg, local_midnight_utc + 30_000, false).unwrap();
        assert_eq!(t.t_ms(), local_midnight_utc + 1);
    }

    #[test]
    fn special_flag_price_widening_side_and_errors() {
        let (t, _) = decode_trade(&wire(1), &TradeDecodeConfig::default(), T10_MS, true).unwrap();
        assert!(t.is_trade_ex, "tradeex topic marks the trade special");
        assert_eq!(widen_price(0.1f32), 0.1);
        assert_eq!(widen_price(101.25f32), 101.25);
        assert_eq!(Aggressor::parse("ask"), Aggressor::Ask);
        assert_eq!(Aggressor::parse("S"), Aggressor::Ask);
        assert_eq!(Aggressor::parse("?"), Aggressor::Other("?".into()));
        let no_price = TradeMessage {
            symbol: "TEST1".into(),
            timestamp: Some(1),
            ..TradeMessage::default()
        }
        .encode_to_vec();
        assert_eq!(
            decode_trade(&no_price, &TradeDecodeConfig::default(), T10_MS, false).unwrap_err(),
            TradeError::NoPrice
        );
        assert!(matches!(
            decode_trade(&[0xff, 0xff], &TradeDecodeConfig::default(), T10_MS, false),
            Err(TradeError::Decode(_))
        ));
    }

    #[test]
    fn dedup_key_and_exchange_order() {
        let mk = |ns: i64, no: Option<&str>| Trade {
            symbol: "TEST1".into(),
            price: 10.0,
            quantity: Some(1),
            side: None,
            timestamp_ns: ns,
            order_no: no.map(str::to_owned),
            is_trade_ex: false,
        };
        assert_eq!(mk(1, Some("7")).dedup_key(), mk(2, Some("7")).dedup_key());
        assert_ne!(mk(1, None).dedup_key(), mk(2, None).dedup_key());
        let mut v = [
            mk(5, Some("10")),
            mk(5, Some("9")),
            mk(4, Some("11")),
            mk(5, None),
        ];
        v.sort_by(Trade::exchange_cmp);
        let order: Vec<_> = v.iter().map(|t| t.order_no.clone()).collect();
        assert_eq!(
            order,
            vec![
                Some("11".to_owned()),
                Some("9".to_owned()),
                Some("10".to_owned()),
                None
            ]
        );
    }
}

// ---------------------------------------------------------------------------------------
// (RP4) Historical trades from the REST CSV (`qp_live::rest::RestClient::trades_csv`).

/// Side of a REST trade row: the CSV `bidask` column carries the ASCII code of the
/// aggressor letter (`98` = `b`, buyer-initiated; `97` = `a`, seller-initiated) or `null`
/// (auction match, no aggressor). Letters are accepted too.
#[must_use]
pub fn rest_side(raw: &str) -> Option<Aggressor> {
    let r = raw.trim();
    if r.is_empty() || r.eq_ignore_ascii_case("null") {
        return None;
    }
    if let Ok(code) = r.parse::<u8>() {
        return Some(Aggressor::parse(&char::from(code).to_string()));
    }
    Some(Aggressor::parse(r))
}

fn civil_days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Local midnight (UTC epoch ns) of a `date_time` like `2026-11-16T09:55:19.000+0300`
/// (date part and the `±HHMM` offset; the clock part is not used).
fn local_midnight_ns(date_time: &str) -> Option<i64> {
    let b = date_time.as_bytes();
    if b.len() < 10 {
        return None;
    }
    let y: i64 = date_time.get(0..4)?.parse().ok()?;
    let m: i64 = date_time.get(5..7)?.parse().ok()?;
    let d: i64 = date_time.get(8..10)?.parse().ok()?;
    let off = date_time
        .rfind(['+', '-'])
        .filter(|&i| i > 10)
        .and_then(|i| {
            let s = &date_time[i..];
            let sign = if s.starts_with('-') { -1 } else { 1 };
            let digits: String = s[1..].chars().filter(char::is_ascii_digit).collect();
            let hh: i64 = digits.get(0..2)?.parse().ok()?;
            let mm: i64 = digits.get(2..4).unwrap_or("00").parse().ok()?;
            Some(sign * (hh * 60 + mm))
        })?;
    Some((civil_days(y, m, d) * 86_400 - off * 60) * 1_000_000_000)
}

/// Parses a level-1 REST trade CSV (`symbol,date_time,timestamp_ns,bidask,mx_id,price,
/// quantity`, header first; columns found by name). Time = the row date's local midnight
/// (from `date_time` and its offset) + `timestamp_ns` (nanoseconds since local midnight,
/// the exchange's own time; `date_time` is whole seconds and lags it by 1–2 s on auction
/// rows). Rows that do not parse are counted in the second return value, not guessed.
/// Buyer/seller columns, if ever present, are not read.
#[must_use]
pub fn parse_rest_csv(csv: &[u8]) -> (Vec<Trade>, usize) {
    let text = String::from_utf8_lossy(csv);
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return (Vec::new(), 0);
    };
    let cols: Vec<&str> = header.split(',').map(str::trim).collect();
    let at = |n: &str| cols.iter().position(|c| *c == n);
    let (Some(i_sym), Some(i_dt), Some(i_ns), Some(i_side), Some(i_id), Some(i_px), Some(i_q)) = (
        at("symbol"),
        at("date_time"),
        at("timestamp_ns"),
        at("bidask"),
        at("mx_id"),
        at("price"),
        at("quantity"),
    ) else {
        return (Vec::new(), 0);
    };
    let mut out = Vec::new();
    let mut bad = 0usize;
    let mut day_cache: Option<(String, i64)> = None;
    for l in lines {
        if l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        let get = |i: usize| f.get(i).map(|s| s.trim());
        let parsed = (|| {
            let dt = get(i_dt)?;
            // Cache key: the date and the offset suffix.
            let key = format!(
                "{}{}",
                dt.get(0..10)?,
                dt.get(dt.len().saturating_sub(5)..)?
            );
            let mid = match &day_cache {
                Some((k, v)) if *k == key => *v,
                _ => {
                    let v = local_midnight_ns(dt)?;
                    day_cache = Some((key, v));
                    v
                }
            };
            let ns: i64 = get(i_ns)?.parse().ok()?;
            let price: f64 = get(i_px)?.parse().ok()?;
            let qty = get(i_q).and_then(|q| q.parse::<i64>().ok());
            let id = get(i_id).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
            (price.is_finite() && price > 0.0).then_some(Trade {
                symbol: get(i_sym)?.to_owned(),
                price,
                quantity: qty,
                side: rest_side(get(i_side).unwrap_or("")),
                timestamp_ns: mid + ns,
                order_no: id.map(str::to_owned),
                is_trade_ex: false,
            })
        })();
        match parsed {
            Some(t) => out.push(t),
            None => bad += 1,
        }
    }
    (out, bad)
}

#[cfg(test)]
mod rest_csv_tests {
    use super::*;

    #[test]
    fn parses_rest_rows_with_exchange_time() {
        let csv = b"symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity\n\
            AAA,2026-11-16T09:55:19.000+0300,35717182000000,null,1,312.25,4\n\
            AAA,2026-11-16T10:00:00.000+0300,36000000000001,98,2,312.5,10\n\
            AAA,2026-11-16T10:00:01.000+0300,36001000000000,97,3,312,7\n\
            AAA,bad,1,98,4,1,1\n";
        let (t, bad) = parse_rest_csv(csv);
        assert_eq!(bad, 1);
        assert_eq!(t.len(), 3);
        // 2026-11-16 00:00 +03:00 = 2026-11-15 21:00 UTC.
        let mid = 1_763_240_400_000_000_000_i64 + 365 * 86_400 * 1_000_000_000;
        assert_eq!(
            civil_days(2026, 11, 16) * 86_400 - 3 * 3600,
            mid / 1_000_000_000
        );
        assert_eq!(t[0].timestamp_ns, mid + 35_717_182_000_000);
        assert_eq!(t[0].side, None);
        assert_eq!(t[1].side, Some(Aggressor::Bid));
        assert_eq!(t[2].side, Some(Aggressor::Ask));
        assert_eq!(t[1].order_no.as_deref(), Some("2"));
        assert_eq!(t[2].price.to_bits(), 312.0_f64.to_bits());
        assert_eq!(t[1].quantity, Some(10));
        assert!(!t[0].is_trade_ex);
    }
}
