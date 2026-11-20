# RP4 — Kalibrasyon ve sınama aralıkları (önceden yazıldı, değiştirilmez)

Yazıldığı an: 2026-11-20 23:45 (İstanbul), **hiçbir RP4 verisi çekilmeden önce**. Bu dosya
commit edildikten sonra değiştirilmez; düzenek (H), alternatif dalları ve son ölçüm bu
dosyanın commit'inden sonra gelir (`git log` ile doğrulanır, VERIFY 3).

Takvim: XIST (`fixtures/calendar/xist.json`); 2026-10-28 yarım gün, 2026-10-29 tatil.
Tarihler borsa günüdür (İstanbul), aralıklar kapalıdır.

## Gün içi (1 dk, 5 dk, 15 dk, 1 sa)

| Aralık | Başlangıç | Bitiş | İşlem günü |
|---|---|---|---|
| **Kalibrasyon** | REST'in verdiği en eski gün, en erken **2025-11-03** | **2026-10-16** | REST'e göre (rapor sayar; en az 20 hedef) |
| **Sınama** | **2026-10-19** | **2026-11-20** | 24 (yarım gün 2026-10-28 dahil) |

- Kalibrasyonun başlangıcı veri varlığına göre belirlenir (REST'in o sembol ve periyot için
  verdiği en eski gün; bu bir sonuç değil, kapsama bilgisidir). Sınırın tarihi sabittir.
- Bir hikâye **doğum barının** gününe göre aralığa girer. Kalibrasyon hikâyesinin sonucu
  sınama aralığına taşarsa (doğum ≤ 2026-10-16, kapanış sonrası) kalibrasyonda sayılmaz;
  kalibrasyon ölçümü yalnız `t < 2026-10-19 00:00 İstanbul` barlarını görür (veri o anda kesilir).
- Sınama ölçümü, ısınma için sınamadan önceki barları okur (ATR, dedektör pencereleri), ama
  yalnız doğumu sınama aralığında olan hikâyeleri sayar.

## Günlük (1 gün)

| Aralık | Başlangıç | Bitiş |
|---|---|---|
| **Kalibrasyon** | 2018-01-02 (ya da REST'in verdiği en eski) | **2024-12-31** |
| **Sınama** | **2025-01-02** | **2026-11-20** |

Günlükte RP4 sabit seçmez (yalnız "kötüleşmedi" denetimi, ölçüt 7); aralık yine önceden yazılı.

## İşlem (trade) verisi

- Çekilecek aralık: **2026-10-05 → 2026-11-20** (15 günlük sınırın altında pencereler:
  10-05..10-18, 10-19..11-01, 11-02..11-15, 11-16..11-20). Sınama aralığının tamamını ve
  kalibrasyonun son iki haftasını (M27 ve benzetim ısınması) kapsar.
- M27 (bar kaynağı) bir ölçümdür, sabit seçmez; özel işlemlerin (`tradeex`) bara girip girmediği
  kalibrasyon günlerinde (10-05..10-16) belirlenir, sınama günlerinde yalnız raporlanır.

## Erişim kuralı

- Alternatif ajanları (M25a–…, M26a–…) yalnız kalibrasyon aralığının önbellek dosyalarını alır
  (sınama günleri kesilmiş ayrı bir klasör); sınama aralığının yolu onlara verilmez.
- Sınama aralığı yalnız son ölçümde (S) açılır; düzenek bunu bir bayrakla (`--split test`) yapar
  ve kalibrasyon kipinde sınama barlarını okumaz (kod ve test).
