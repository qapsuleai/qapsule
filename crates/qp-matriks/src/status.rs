//! Session phase ("seans evresi"): the `stockStatus` codes of `SymbolMessage`.
//!
//! The list is transcribed from the comment block at the end of
//! `proto/Symbol.proto` (codes 1–55 and 99; 19 is absent there). `proto_name`
//! is the name exactly as written in the proto; `key` is its mechanical
//! snake_case; `tr` / `en` are human labels added here (the proto has none).
//! Labels for the less common codes (`*_woEQ`, one-sided auctions, fixed
//! income, PMD) are a best-effort reading of the proto names, not Matriks
//! documentation.

/// One `stockStatus` code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionPhase {
    /// The `stockStatus` code.
    pub code: i32,
    /// Name as written in `proto/Symbol.proto`.
    pub proto_name: &'static str,
    /// Machine key: snake_case of `proto_name`.
    pub key: &'static str,
    /// Turkish label.
    pub tr: &'static str,
    /// English label.
    pub en: &'static str,
}

const fn p(
    code: i32,
    proto_name: &'static str,
    key: &'static str,
    tr: &'static str,
    en: &'static str,
) -> SessionPhase {
    SessionPhase {
        code,
        proto_name,
        key,
        tr,
        en,
    }
}

/// All `stockStatus` codes, sorted by code.
pub const SESSION_PHASES: &[SessionPhase] = &[
    p(1, "Closed", "closed", "Kapalı", "Closed"),
    p(
        2,
        "Continuous",
        "continuous",
        "Sürekli işlem",
        "Continuous trading",
    ),
    p(3, "Uncrossing", "uncrossing", "Eşleştirme", "Uncrossing"),
    p(
        4,
        "OpeningSession",
        "opening_session",
        "Açılış emir toplama",
        "Opening auction",
    ),
    p(
        5,
        "ClosingSession",
        "closing_session",
        "Kapanış emir toplama",
        "Closing auction",
    ),
    p(6, "Break", "break", "Ara", "Break"),
    p(
        7,
        "ContinuousLastTrdPr",
        "continuous_last_trd_pr",
        "Kapanış fiyatından işlemler",
        "Trading at last price",
    ),
    p(
        8,
        "SinglePriceAuction",
        "single_price_auction",
        "Tek fiyat emir toplama",
        "Single price auction",
    ),
    p(
        9,
        "PrimaryContinuous",
        "primary_continuous",
        "Birincil piyasa sürekli işlem",
        "Primary market continuous trading",
    ),
    p(10, "Suspend", "suspend", "Geçici durdurma", "Suspended"),
    p(
        11,
        "OddContLastTrdPr",
        "odd_cont_last_trd_pr",
        "Kesirli: kapanış fiyatından işlemler",
        "Odd lot trading at last price",
    ),
    p(
        12,
        "OddContinuous",
        "odd_continuous",
        "Kesirli sürekli işlem",
        "Odd lot continuous trading",
    ),
    p(
        13,
        "OrderCollectionPeriodAfterCB",
        "order_collection_period_after_cb",
        "Devre kesici sonrası emir toplama",
        "Order collection after circuit breaker",
    ),
    p(
        14,
        "UncrossOneSidedBuyBackHidden",
        "uncross_one_sided_buy_back_hidden",
        "Tek taraflı geri alım eşleştirme (gizli)",
        "One-sided buy-back uncrossing (hidden)",
    ),
    p(
        15,
        "UncrossOneSidedBuyBackOpen",
        "uncross_one_sided_buy_back_open",
        "Tek taraflı geri alım eşleştirme (açık)",
        "One-sided buy-back uncrossing (open)",
    ),
    p(
        16,
        "UncrossOneSidedIssueHidden",
        "uncross_one_sided_issue_hidden",
        "Tek taraflı ihraç eşleştirme (gizli)",
        "One-sided issue uncrossing (hidden)",
    ),
    p(
        17,
        "UncrossOneSidedIssueOpen",
        "uncross_one_sided_issue_open",
        "Tek taraflı ihraç eşleştirme (açık)",
        "One-sided issue uncrossing (open)",
    ),
    p(
        18,
        "AfterOneSidedAuctionPeriod",
        "after_one_sided_auction_period",
        "Tek taraflı müzayede sonrası",
        "After one-sided auction",
    ),
    p(
        20,
        "SinglePriceUncrossingProcessStarted",
        "single_price_uncrossing_process_started",
        "Tek fiyat eşleştirme başladı",
        "Single price uncrossing started",
    ),
    p(
        21,
        "OneSidedAuctionPeriod",
        "one_sided_auction_period",
        "Tek taraflı müzayede",
        "One-sided auction",
    ),
    p(
        22,
        "UncrossIssuingOpen",
        "uncross_issuing_open",
        "İhraç eşleştirme (açık)",
        "Issue uncrossing (open)",
    ),
    p(
        23,
        "UncrossIssuingHidden",
        "uncross_issuing_hidden",
        "İhraç eşleştirme (gizli)",
        "Issue uncrossing (hidden)",
    ),
    p(
        24,
        "UncrossBuyBackOpen",
        "uncross_buy_back_open",
        "Geri alım eşleştirme (açık)",
        "Buy-back uncrossing (open)",
    ),
    p(
        25,
        "UncrossBuyBackHidden",
        "uncross_buy_back_hidden",
        "Geri alım eşleştirme (gizli)",
        "Buy-back uncrossing (hidden)",
    ),
    p(
        26,
        "NonTradablePeriod",
        "non_tradable_period",
        "Seans dışı",
        "Non-tradable period",
    ),
    p(
        27,
        "DisseminationOfPriceLimits",
        "dissemination_of_price_limits",
        "Fiyat limitlerinin yayımı",
        "Price limit dissemination",
    ),
    p(
        28,
        "Derivatives_SettlementPriceDissemination",
        "derivatives_settlement_price_dissemination",
        "VİOP uzlaşma fiyatı yayımı",
        "Derivatives settlement price dissemination",
    ),
    p(
        29,
        "FixedIncome_EOD",
        "fixed_income_eod",
        "Borçlanma araçları gün sonu",
        "Fixed income end of day",
    ),
    p(
        30,
        "FixedIncome_SOD",
        "fixed_income_sod",
        "Borçlanma araçları gün başı",
        "Fixed income start of day",
    ),
    p(
        31,
        "Derivatives_DataFeed",
        "derivatives_data_feed",
        "VİOP veri yayını",
        "Derivatives data feed",
    ),
    p(
        32,
        "Derivatives_SettlementPriceMatching",
        "derivatives_settlement_price_matching",
        "VİOP uzlaşma fiyatı eşleştirme",
        "Derivatives settlement price matching",
    ),
    p(
        33,
        "NonTradingPeriod_AfterHoursMorningSession",
        "non_trading_period_after_hours_morning_session",
        "Seans dışı (sabah ek seansı)",
        "Non-trading period (after-hours morning session)",
    ),
    p(
        34,
        "NonTradingPeriod_AfterHoursEveningSession",
        "non_trading_period_after_hours_evening_session",
        "Seans dışı (akşam ek seansı)",
        "Non-trading period (after-hours evening session)",
    ),
    p(
        35,
        "Continuous_AfterHoursMorningSession",
        "continuous_after_hours_morning_session",
        "Sürekli işlem (sabah ek seansı)",
        "Continuous trading (after-hours morning session)",
    ),
    p(
        36,
        "Continuous_AfterHoursEveningSession",
        "continuous_after_hours_evening_session",
        "Sürekli işlem (akşam ek seansı)",
        "Continuous trading (after-hours evening session)",
    ),
    p(
        37,
        "Closed_AfterHoursMorningSession",
        "closed_after_hours_morning_session",
        "Kapalı (sabah ek seansı)",
        "Closed (after-hours morning session)",
    ),
    p(
        38,
        "Closed_AfterHoursEveningSession",
        "closed_after_hours_evening_session",
        "Kapalı (akşam ek seansı)",
        "Closed (after-hours evening session)",
    ),
    p(
        39,
        "Continuous_AfterCutoff",
        "continuous_after_cutoff",
        "Sürekli işlem (kesim saati sonrası)",
        "Continuous trading (after cutoff)",
    ),
    p(40, "Reserved1", "reserved1", "Ayrılmış 1", "Reserved 1"),
    p(41, "Reserved2", "reserved2", "Ayrılmış 2", "Reserved 2"),
    p(42, "Reserved3", "reserved3", "Ayrılmış 3", "Reserved 3"),
    p(43, "Reserved4", "reserved4", "Ayrılmış 4", "Reserved 4"),
    p(44, "Reserved5", "reserved5", "Ayrılmış 5", "Reserved 5"),
    p(45, "Reserved6", "reserved6", "Ayrılmış 6", "Reserved 6"),
    p(46, "Reserved7", "reserved7", "Ayrılmış 7", "Reserved 7"),
    p(
        47,
        "PMD_SessionClosed",
        "pmd_session_closed",
        "Kıymetli madenler piyasası: seans kapalı",
        "Precious metals market: session closed",
    ),
    p(
        48,
        "FixedIncome_RepoQuotation",
        "fixed_income_repo_quotation",
        "Borçlanma araçları repo kotasyonu",
        "Fixed income repo quotation",
    ),
    p(
        49,
        "FixedIncome_SessionClosed",
        "fixed_income_session_closed",
        "Borçlanma araçları: seans kapalı",
        "Fixed income: session closed",
    ),
    p(
        50,
        "Derivatives_SessionClosed",
        "derivatives_session_closed",
        "VİOP: seans kapalı",
        "Derivatives: session closed",
    ),
    p(
        51,
        "OpeningSession_woEQ",
        "opening_session_wo_eq",
        "Açılış emir toplama (teorik eşleşme yayımsız)",
        "Opening auction (without equilibrium dissemination)",
    ),
    p(
        52,
        "OneSidedAuctionPeriod_woEQ",
        "one_sided_auction_period_wo_eq",
        "Tek taraflı müzayede (teorik eşleşme yayımsız)",
        "One-sided auction (without equilibrium dissemination)",
    ),
    p(
        53,
        "OrderCollectionPeriodAfterCB_woEQ",
        "order_collection_period_after_cb_wo_eq",
        "Devre kesici sonrası emir toplama (teorik eşleşme yayımsız)",
        "Order collection after circuit breaker (without equilibrium dissemination)",
    ),
    p(
        54,
        "ClosingSession_woEQ",
        "closing_session_wo_eq",
        "Kapanış emir toplama (teorik eşleşme yayımsız)",
        "Closing auction (without equilibrium dissemination)",
    ),
    p(
        55,
        "SinglePriceAuction_woEQ",
        "single_price_auction_wo_eq",
        "Tek fiyat emir toplama (teorik eşleşme yayımsız)",
        "Single price auction (without equilibrium dissemination)",
    ),
    p(
        99,
        "StateResetStart",
        "state_reset_start",
        "Durum sıfırlama başlangıcı",
        "State reset start",
    ),
];

/// The session phase for a `stockStatus` code; `None` for codes not in the proto list.
pub fn session_phase(code: i32) -> Option<&'static SessionPhase> {
    SESSION_PHASES
        .binary_search_by_key(&code, |p| p.code)
        .ok()
        .map(|i| &SESSION_PHASES[i])
}

/// The session phase for a machine key (`"continuous"`).
pub fn session_phase_by_key(key: &str) -> Option<&'static SessionPhase> {
    SESSION_PHASES.iter().find(|p| p.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snake(name: &str) -> String {
        let mut out = String::new();
        let chars: Vec<char> = name.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if c == '_' {
                out.push('_');
                continue;
            }
            if c.is_ascii_uppercase() {
                let prev = i.checked_sub(1).map(|j| chars[j]);
                let next = chars.get(i + 1).copied();
                let boundary = match prev {
                    None | Some('_') => false,
                    Some(p) if p.is_ascii_lowercase() || p.is_ascii_digit() => true,
                    Some(p) if p.is_ascii_uppercase() => {
                        next.is_some_and(|n| n.is_ascii_lowercase())
                    }
                    _ => false,
                };
                if boundary {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn table_is_complete_sorted_and_keyed() {
        // 1..=55 except 19, plus 99 (proto comment list).
        let expected: Vec<i32> = (1..=55).filter(|&c| c != 19).chain([99]).collect();
        let codes: Vec<i32> = SESSION_PHASES.iter().map(|p| p.code).collect();
        assert_eq!(codes, expected);
        for ph in SESSION_PHASES {
            assert_eq!(ph.key, snake(ph.proto_name), "key of {}", ph.code);
            assert!(!ph.tr.is_empty() && !ph.en.is_empty());
        }
    }

    #[test]
    fn proto_comment_names_match() {
        // Every "<code> <Name>" line of the proto comment block is in the table, verbatim.
        let src = include_str!("../proto/Symbol.proto");
        let block = src.split("stockstatus meaning:").nth(1).unwrap();
        let mut n = 0;
        for line in block.lines() {
            let mut it = line.split_whitespace();
            let (Some(code), Some(name)) = (it.next(), it.next()) else {
                continue;
            };
            let Ok(code) = code.parse::<i32>() else {
                continue;
            };
            assert_eq!(session_phase(code).unwrap().proto_name, name);
            n += 1;
        }
        assert_eq!(n, SESSION_PHASES.len());
    }

    #[test]
    fn lookups() {
        let c = session_phase(2).unwrap();
        assert_eq!((c.key, c.tr), ("continuous", "Sürekli işlem"));
        assert_eq!(session_phase(5).unwrap().key, "closing_session");
        assert_eq!(session_phase(19), None);
        assert_eq!(session_phase(0), None);
        assert_eq!(session_phase_by_key("suspend").unwrap().code, 10);
    }
}
