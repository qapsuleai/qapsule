# qp-calendar

**Borsa takvimi** ("Takvim", K14). Bar sınırlarını borsanın saat dilimine, seanslarına ve
tatillerine göre kurar; izleyicinin saat dilimi yalnız gösterimdir (`display_tz`). Saat
dilimleri IANA tzdb'den üretilmiş gömülü geçiş tablolarıdır (`src/tzdata.rs`,
`examples/gen_tzdata.rs`; K16); hesap tamsayı aritmetiğidir, her hedefte aynı sonucu verir.

**Katman:** çekirdek (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`. Geliştirme: `qp-schema` (`validate`), `jiff` (yalnız
tablo üreticisi ve sınama kâhini), `serde_json`.

**Dışa açık yüzey (`lib.rs`):**

- `ExchangeCalendar`, `format_display` — takvim üstünde bar sınırları ve gösterim.
- `Interval`, `Span` — `Nm`/`Nh`, `1d`, `1w`, `1M` aralıkları ve bar açıklığı `[açılış, kapanış)`.
- `builtin` (`XIST` 2018–2027 tatil ve yarım günleri; `XNYS` örnek günler),
  `XIST_HOLIDAYS`, `XIST_HALF_DAYS`, `XIST_HALF_DAY_CLOSE`.
- `zone` (`Zone`, `LocalTime`), sivil tarih işlevleri (`days_from_civil`, `civil_from_days`,
  `parse_date`, `weekday_index`), `CalendarError`.

**Düzen:** `src/` düz (8 dosya); konu klasörü yok.

**Ne değildir:** veri kaynağı değildir (barları `qp-source` üretir); takvim verisinin tipi
burada değil `qp_schema::Calendar`'dadır; saat okumaz, ağa çıkmaz.
