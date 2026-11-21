# RP4 — Ağ günlüğü (REST ve canlı)

Yalnız orkestratör, sırayla. Değer, adres, belirteç yok; yalnız sayılar. Canlı bağlantı bu
round'da **açılmadı** (borsa kapalı: 2026-11-20 Cuma 23:34'te başlandı, hafta sonu); canlı
doğrulamalar "ilk iş günü" listesinde (`REPORT.md`).

| # | Zaman (İstanbul) | Ne | Sonuç |
|---|---|---|---|
| N0 | 2026-11-21 ~02:05 | Kimlik kilidini Arkin'in 2026-11-20 sabahı başlattığı `qp demo --live` (anlık kip, 12 sa açık, borsa kapalı) tutuyordu. Arkin'in onayıyla durduruldu: SIGINT etkisiz (arka plan süreci), Arkin'in ikinci onayıyla SIGTERM. Ham kaydının (`20261120T082104Z-demo.qpraw`, 72 MB) son ≤ 8 KiB tamponu yazılmamış olabilir; dosya 17:42'den beri büyümüyordu. | Durdu; kilit boş. |
| N1 | 2026-11-21 02:08–02:40 | 4 belirteç isteği, hepsi HTTP 403 (gövdesiz). Neden: Arkin'in VPN'i kopmuştu (Arkin). Veri isteği 0. Belirteç hatasında yalnız temizlenmiş hata kodu raporlanacak şekilde istemci düzeltildi (`AuthError::HttpCode`). | Yeniden deneme yapılmadı; Arkin VPN'i bağladı. |
| N2 | 2026-11-21 12:46 | Deneme: THYAO işlemleri 2026-11-16→20 (1 istek). | 295.462 satır, 20 MB CSV (2,7 MB gz). |
| N3 | 2026-11-21 12:47–12:49 | 5 dk bar derinliği denemeleri (THYAO): 1 yıllık tek istek HTTP 400; 7 gün (2025-11-03) 485 satır; 31 gün 2.231; 92 gün 6.303. | 5 dk için 90 günlük pencere. |
| D1a | 12:50–13:11 | İşlemler, BIST100, 2026-10-05→2026-11-20, 14 günlük pencere (`trades`) | 399 istek, 64.389.066 işlem, 0 hata |
| D1b | 13:11–13:20 | Düzeltilmiş barlar BIST100: 5 dk (90 günlük 5 pencere, 2025-11-03→), 1 sa (tek istek), 1 gün (2018→) | 500 + 100 + 100 istek, 0 hata |
| D1c | 13:20–13:22 | M27 düzeltilmemiş barlar BIST30: 1 dk (4 pencere), 5 dk, 1 sa, 1 gün (2026-10-05→2026-11-20) | 210 istek, 0 hata |
| D1d | 13:22–14:05 | Ölçek: 2026-11-20 işlemleri, BISTTÜM'ün BIST100 dışı 480 sembolü (1 günlük pencere). **Hata:** ilk koşu BIST100'ü de 1 günlük pencereyle yeniden istedi; fark edilince durduruldu: **15 istek aynı veriyi ikinci kez çekti** (dosyaları silindi; veri 11-16→11-20 penceresinde zaten vardı) | 55 + 440 istek, 0 hata |
| D1e | 14:05 | Özel işlemler (`trade_ex`) BIST30, 2026-10-05→2026-10-18 | 30 istek, 57 satır |
| D1f | 2026-11-21 ~15:10 | `trade_ex` BIST30, 2026-10-19→2026-11-20 (M27 günlük farkını kapatmak için; T2'nin açık sorusu) | 90 istek, 208 satır, 0 hata |
| **Toplam** | | REST veri isteği **1.933** (işlem 1.016, bar 917; başarısız 5, hepsi N2–N3 denemeleri ve N1 sırasında), belirteç reddi 4; tekrar 15 | Önbellek: işlemler 610 MB, barlar 408 MB (`fixtures/local/`) |
