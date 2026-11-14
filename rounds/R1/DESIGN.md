# R1 — Orkestratör Tasarım Notu

Bu not, alt görevlerin birbirine uyması için orkestratörün koyduğu sözleşmedir.
Alt ajanlar bu sözleşmeye uyar; değiştirmek gerekiyorsa gerekçesini kendi raporuna yazar.

## Crate yerleşimi

| Crate | Görev | İçerik |
|-------|-------|--------|
| `qp-schema` | T1 | Tek `Bar`, tek `Series`; `Instrument`, `Calendar` (veri), `display_tz`; JSON Schema doğrulaması (`validate`). |
| `qp-compute` | T2, T3 | `Study` arayüzü, SMA/EMA/RSI/MACD/Bollinger, tarama (`sweep`), ölçüm düzeneği. |
| `qp-source` (yeni) | T4 | `SeriesSource` arayüzü (çekirdek tarafı), dosya ve sentetik kaynak. Ayrı crate: T2 ile paralel yazım çakışmasın, `qp-compute` kaynak türünü bilmesin. |
| `qp-calendar` (yeni) | T6 | Takvime göre bar sınırları (gün içi, günlük, haftalık, aylık), BIST ve NYSE örnek takvimleri. Ayrı crate: saat dilimi verisinin WASM boyutuna etkisi ayrı ölçülebilsin. |
| `qp-cli` | T5 | `qp compute`; `qp schema validate` kütüphaneyi çağırır. |
| `qp-render`, `qp-wasm` | T7 | Çizici `qp-schema` barlarını ve `qp-compute` SMA'sını kullanır; wasm hesap bağlaması. |

## Ortak kurallar

- Tüm fiyat ve hesap değerleri `f64`. `f32` yok.
- Hesap çıktısı uzunluğu = girdi bar sayısı. Isınma barları `None` (JSON'da `null`) (K13).
- Bit-eşitlik: yalnızca IEEE temel işlemler (`+ - * /`, `sqrt`). `mul_add`, `powf`, `exp`,
  `ln`, `sin` vb. hesap yolunda kullanılmaz (libm uygulamaları hedefe göre değişebilir).
  Derleyici kendiliğinden FMA üretmez (Rust `-C target-feature=+fma` olsa da `a*b+c`
  birleştirilmez); bu testle gösterilir (aarch64, x86_64, wasm32 özetleri).
- Toplama sırası sabittir; paralel/SIMD iyileştirmeleri tek tek çalıştırmayla bit-eşit kalmalı.
- Sabit tohumlu rastgelelik yalnızca testlerde (proptest için `PROPTEST_RNG_SEED` /
  sabit `Config`), hesap kodunda hiç.
- Hesapta saat, ağ, LLM yok.

## Şema sözleşmesi (T1)

- `qp_schema::Bar { t: i64 (açılış zamanı, epoch ms UTC), o, h, l, c, v: f64 }` — tek tanım.
- `fixtures/bars/*.json` bir `Series` belgesidir (`source: {kind: inline}`,
  `data: {kind: bars, bars: [...]}`) ve üretilen `Series.schema.json` ile doğrulanır.
  Bar değerleri baytı baytına aynı kalır (altın görseller değişmemeli).
- Tek değerli çıktılar `Point { t, v: Option<f64> }`; ısınma `null`.
- Çok çıktılı hesap sonucu için şemada bir tip bulunur (ör. `StudyOutput { study, params,
  t: Vec<i64>, outputs: BTreeMap<String, Vec<Option<f64>>> }`); `qp compute` bunu basar.
- `Instrument { symbol, exchange, calendar }` ve `Calendar { id, tz (IANA), sessions,
  holidays, half_days }` veri tipleridir; mantık `qp-calendar`'dadır.
- `display_tz` gösterime aittir (Document/View), hesaba girmez.
- `qp_schema::validate(type_name, &serde_json::Value) -> Result<(), Vec<String>>`
  (üretilen şemaya karşı) ve anlamsal bar kontrolü (`validate_bars`: sonlu değerler,
  OHLC tutarlılığı, artan zaman) `qp-schema` içinde. CLI incedir.

## `Study` sözleşmesi (T2)

- Girdi alanları adlıdır: `open, high, low, close, volume` ya da başka bir hesabın çıktısı.
- Çıktılar adlıdır (MACD: `macd, signal, histogram`; Bollinger: `middle, upper, lower,
  width` ya da T2'nin belgelediği dört ad).
- Üç kip: toplu (`run`), akışkan (`push`), son barı düzelt (`update_last`). Toplu = akışkan =
  düzelt, bit-eşit. Yeni bar O(1) (pencere uzunluğundan bağımsız; halka tamponu kabul).
