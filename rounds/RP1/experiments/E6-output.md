# E6 — Bellek yerleşimi ve çıktı trafiği

Dal: `rp1/e6-output` (taban `rp1/e2-simd`, çünkü ölçülen şey E2 çekirdeklerinin yanındaki
çıktı yolu). Yürüten: orkestratör.

## Hipotezler (denemeden önce yazıldı)

**H6.1 — Gereksiz sıfır doldurma.** `soa_prepare` her varyasyonda her sütunu `clear()` +
`resize(n, 0.0)` ile sıfırlıyor; çekirdek ardından sütunun her hücresini (ısınmada `NaN`,
sonra değer) yeniden yazıyor. Yani her tarama 1.000 × sütun × 10k × 8 B fazladan `memset`
yapıyor (SMA/EMA/RSI 80 MB, MACD 240 MB, Bollinger 320 MB). E2 sonrası SMA ≈ 5 ms ve
80 MB memset ≈ 2–4 ms → SMA/EMA'da süre %30–50 kısalabilir; çok çıktılılarda da benzer pay.

**H6.2 — Çıktıyı hiç yazmamak (indirgeme).** Ajanın çoğu sorusu varyasyon başına bir
indirgeme ister (son değer, pencere). Yazma tamamen kalkarsa kalan süre saf aritmetiktir;
bu, çıktı yolunun payını ölçmenin yolu. (Uygulamada E3'ün `sweep_reduce_par`'ı ve E8'in
pencere API'si bu yöndedir.)

## Deney 6.1 — Yeniden kullanılan sütunu sıfırlamamak

**Yapılan.** `soa_prepare`: sütun zaten doğru uzunluktaysa `clear()`+`resize()` yapılmıyor.
Çekirdekler her hücreyi yazdığı için sonuç değişmez (testler ve tüm profil özetleri aynı).

**Ölçüm** (aarch64, aynı oturumda E2 ↔ E6 üç kez dönüşümlü, 9 koşunun medyanı, ms):

| Profil / hesap | E2 | E6 | Kazanç |
|----------------|---:|---:|-------:|
| A SMA | 5,03–5,13 | 4,42–4,57 | %12 |
| A EMA | 4,51–4,66 | 4,05–4,10 | %10 |
| A RSI | 19,7–20,0 | 19,0–19,4 | %3 (gürültü içinde) |
| A MACD | 9,9–10,3 | 8,8–9,2 | %13 |
| A Bollinger | 29,7–30,1 | 28,3–28,4 | %5 |
| B SMA | 7,7–7,9 | 6,4–6,9 | %15 |
| B EMA | 6,8–7,2 | 5,6–5,8 | %17 |
| B MACD | 16,2–17,0 | 12,7–12,8 | %23 |
| B Bollinger | 38,7–39,0 | 33,7–34,2 | %12 |

**Değerlendirme.** Hipotezdeki %30–50'nin altında: M4'te `memset` ≈ 50+ GB/s, 80–320 MB
doldurma 1,5–5 ms. Kazanç çıktısı çok olan (MACD, Bollinger) ve kısa serili (B) durumlarda
daha büyük.

**Karar: al.**

## Deney 6.2 — Çıktı yolunun payı (gözlem)

Yazılan sütunu okumak bile pahalı: E3'ün tablosunda "son değer" indirgemesi ile "her çıktıyı
oku" indirgemesi arasında tek iş parçacığında 4–8 kat fark var (ör. SMA 15 → 97 ms, R1
çekirdekleriyle). Yani ajan için asıl kazanç çıktıyı hiç üretmemek ya da yalnızca istenen
pencereyi üretmek: E3 `sweep_reduce_par` (varyasyon başına indirgeme) ve E8 `late::*_tail`
(yalnızca pencere, geçmiş koşmadan) bu yönün uygulamaları. Çekirdek içinde "yazmadan
indirgeme" (çekirdeğin çıktı yerine katlayıcı alması) sonraki adım olarak önerilir.

**Kalan.** `sweep_series_each` pencere içindeki sonuçları `clone_from` ile tutuyor (ikinci bir
yazma geçişi); tampon takası (swap) ile kaldırılabilir.
