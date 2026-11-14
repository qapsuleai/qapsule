# T6b — Protokol kodlaması: JSON kanonik mi kalmalı?

Tarih: 2026-11-14. Dal: `rd1/t6b-exp`. Kod: `experiments/t6b-encoding/` (workspace üyesi değil).
Ham çıktılar: `rounds/RD1/experiments/raw/t6b/`.

## 1. Hipotez (ölçümden önce yazıldı)

Test edilen çalışma kararı: **JSON (serde_json, `float_roundtrip` açık) kanonik kodlamadır.**

Beklentiler (ölçümden önce, literatür ve sezgiye dayanarak):

- H1 — Canlı tick (`notify` + `set_last_bar` / 1 barlık `append_bars`): JSON mesajı ~200–300 bayt,
  ikili biçimler (CBOR/MessagePack) ~%40–60'ı. 60 mesaj/s'de JSON ~15–20 KB/s; bu, herhangi bir
  ağ veya cihaz için önemsiz. Kodlama/çözme mesaj başına birkaç µs; 16 ms çerçeve bütçesinin
  binde birinin altında. **Sonuç beklentisi: tick yolunda kodlama seçimi önemsiz.**
- H2 — Seri parçası (10 000 bar, RSI/MACD `values`, 10 000 barlık `append_bars`): JSON ham boyutu
  ikili biçimlerin ~2–3 katı; `float_roundtrip` açık JSON çözme, postcard/sütunlu bloğa göre
  5–20 kat yavaş (ms mertebesi, 10 000 bar için 1–5 ms). gzip/brotli sonrası boyut farkı
  daralır (~1,3–1,8x). Sütunlu f64 LE blok, boyutta (8 B/değer) ve hızda (memcpy'ye yakın)
  açık ara en iyisi olur.
- H3 — Anlık görüntü (10 000 barlık `Document`): H2'ye benzer; tek seferlik olduğu için
  birkaç ms'lik fark kullanıcıya görünmez.
- H4 — f64 birebirlik: `float_roundtrip` açık JSON ve tüm ikili biçimler bit düzeyinde birebir;
  `float_roundtrip` kapalı JSON rastgele bit desenlerinde bazı ULP sapmaları gösterir.
- H5 — postcard, `#[serde(tag = "type")]` (iç etiketli enum) ve `skip_serializing_if` ile aynı
  tipleri **çözemez** (self-describing değil); ancak ayna tiple ölçülebilir. CBOR (ciborium /
  minicbor-serde) ve MessagePack (rmp-serde, struct-map modunda) aynı tiplerle çalışır.
- H6 — WASM'de oranlar benzer, mutlak süreler yerel ölçümün ~1,5–3 katı.

Beklenen karar: JSON kanonik kalır; seri verisi (bar ve `values` blokları) için müzakere
edilen isteğe bağlı bir ikili/sütunlu özellik (`features` ile) değerli olabilir, ama v0 için
zorunlu değil.
