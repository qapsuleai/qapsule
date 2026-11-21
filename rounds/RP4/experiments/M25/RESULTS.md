# M25 — Çıkış kuralı: kalibrasyon sonuçları ve seçim (sınama açılmadan önce)

Düzenek ve puanlama kuralı: `harness/SPEC.md` (eb1a8c3), aralıklar `SPLIT.md` (cc476de); ikisi
de alternatif dallarından önce. Alternatifler ayrı alt ajanlarca, ayrı dallarda, yalnız
kalibrasyon kopyasıyla (`fixtures/local/rp4/calib/`; tam veri klasörü bu evrede okunamaz
yapıldı: `chmod 000`) kuruldu: `rp4/m25a-taban` (N fiyat adımı tabanı), `rp4/m25b-teyit2`
(iki kapanışla teyit), `rp4/m25c-yukdus` (yüksek/düşükle karar), `rp4/m25d-olcek` (periyoda
göre ölçeklenen eşik), `rp4/m25e-savunu` (`umut`: zamanla daralan durdurma; savunucunun
kendi kuralı). Bütün sayılar orkestratörce değerlendirme dalında (`rp4/eval`) yeniden üretildi
ve ajanların bildirdikleriyle aynı. **Bu dosya sınama aralığı açılmadan commit edildi.**

Yeniden üretim: `harness m25 --data fixtures/local/rp4/calib --rule <ad> --period <p> --split calib`;
aday−aday: `--reference <ad>`; `harness report`.

### 5m (kalibrasyon, yönlü satır)


Veri: 100 sembol, 241 işlem günü (2025-11-03→2026-10-16).

| Kural | n | p_real | p_inv | p_exp | ECR_own | ECR_ref | REV | SENS | Brier | Brier taban | kal. farkı | gerçek−yalancı | ömür ort./p90 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `r3` | 383.867 | 0,415 | 0,467 | 0,118 | 0,148 | 0,148 | 0,250 | 0,212 | 0,243 | 0,243 | 0,021 | -0,052 | 4/11 |
| `taban` | 331.377 | 0,284 | 0,290 | 0,426 | 0,069 | 0,087 | 0,173 | 0,193 | 0,203 | 0,203 | 0,023 | -0,006 | 10/20 |
| `teyit2` | 317.036 | 0,315 | 0,344 | 0,341 | 0,061 | 0,071 | 0,147 | 0,132 | 0,216 | 0,216 | 0,025 | -0,029 | 9/19 |
| `yukdus` | 346.776 | 0,342 | 0,342 | 0,316 | 0,066 | 0,115 | 0,218 | 0,099 | 0,225 | 0,225 | 0,023 | -0,000 | 7/19 |
| `olcek` | 341.131 | 0,331 | 0,354 | 0,315 | 0,065 | 0,090 | 0,169 | 0,279 | 0,222 | 0,222 | 0,025 | -0,022 | 8/19 |
| `umut` | 339.791 | 0,367 | 0,482 | 0,151 | 0,026 | 0,036 | 0,186 | 0,199 | 0,232 | 0,232 | 0,026 | -0,025 | 8/16 |

### 15m (kalibrasyon, yönlü satır)

| Kural | n | p_real | p_inv | p_exp | ECR_own | ECR_ref | REV | SENS | Brier | Brier taban | kal. farkı | gerçek−yalancı | ömür ort./p90 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `r3` | 138.347 | 0,432 | 0,450 | 0,118 | 0,136 | 0,136 | 0,235 | 0,156 | 0,245 | 0,245 | 0,027 | -0,018 | 4/10 |
| `taban` | 130.466 | 0,382 | 0,388 | 0,230 | 0,107 | 0,115 | 0,211 | 0,150 | 0,236 | 0,236 | 0,030 | -0,006 | 5/13 |
| `teyit2` | 114.368 | 0,331 | 0,336 | 0,333 | 0,051 | 0,060 | 0,129 | 0,098 | 0,222 | 0,221 | 0,030 | -0,005 | 8/16 |
| `yukdus` | 125.959 | 0,354 | 0,344 | 0,303 | 0,058 | 0,109 | 0,202 | 0,038 | 0,229 | 0,229 | 0,028 | 0,010 | 7/16 |
| `olcek` | 126.471 | 0,368 | 0,374 | 0,258 | 0,070 | 0,092 | 0,168 | 0,186 | 0,233 | 0,233 | 0,031 | -0,005 | 6/15 |
| `umut` | 122.876 | 0,378 | 0,468 | 0,154 | 0,022 | 0,032 | 0,163 | 0,135 | 0,235 | 0,235 | 0,032 | -0,002 | 8/15 |

### 1h (kalibrasyon, yönlü satır)

| Kural | n | p_real | p_inv | p_exp | ECR_own | ECR_ref | REV | SENS | Brier | Brier taban | kal. farkı | gerçek−yalancı | ömür ort./p90 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `r3` | 35.040 | 0,468 | 0,441 | 0,091 | 0,128 | 0,128 | 0,216 | 0,091 | 0,251 | 0,249 | 0,057 | 0,028 | 4/10 |
| `taban` | 34.437 | 0,450 | 0,424 | 0,126 | 0,123 | 0,125 | 0,212 | 0,091 | 0,249 | 0,247 | 0,056 | 0,026 | 5/10 |
| `teyit2` | 28.705 | 0,366 | 0,334 | 0,300 | 0,046 | 0,057 | 0,138 | 0,061 | 0,234 | 0,232 | 0,050 | 0,032 | 9/15 |
| `yukdus` | 31.330 | 0,387 | 0,345 | 0,268 | 0,043 | 0,087 | 0,180 | 0,012 | 0,240 | 0,237 | 0,063 | 0,042 | 8/15 |
| `olcek` | 32.993 | 0,432 | 0,403 | 0,165 | 0,083 | 0,098 | 0,177 | 0,104 | 0,248 | 0,245 | 0,059 | 0,029 | 6/12 |
| `umut` | 30.916 | 0,406 | 0,458 | 0,136 | 0,021 | 0,032 | 0,151 | 0,077 | 0,243 | 0,241 | 0,060 | 0,029 | 8/15 |

### 1d (kalibrasyon, yönlü satır)

| Kural | n | p_real | p_inv | p_exp | ECR_own | ECR_ref | REV | SENS | Brier | Brier taban | kal. farkı | gerçek−yalancı | ömür ort./p90 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `r3` | 28.122 | 0,507 | 0,420 | 0,073 | 0,159 | 0,159 | 0,244 | 0,091 | 0,246 | 0,250 | 0,065 | 0,087 | 3/10 |
| `taban` | 27.006 | 0,470 | 0,388 | 0,143 | 0,151 | 0,153 | 0,237 | 0,064 | 0,245 | 0,249 | 0,092 | 0,082 | 4/10 |
| `teyit2` | 22.457 | 0,403 | 0,324 | 0,274 | 0,053 | 0,066 | 0,141 | 0,053 | 0,235 | 0,241 | 0,082 | 0,079 | 8/15 |
| `yukdus` | 24.591 | 0,439 | 0,319 | 0,242 | 0,044 | 0,093 | 0,195 | 0,034 | 0,235 | 0,246 | 0,099 | 0,120 | 6/15 |
| `olcek` | 28.122 | 0,507 | 0,420 | 0,073 | 0,159 | 0,159 | 0,244 | 0,091 | 0,246 | 0,250 | 0,065 | 0,087 | 3/10 |
| `umut` | 24.540 | 0,462 | 0,423 | 0,115 | 0,018 | 0,030 | 0,168 | 0,078 | 0,241 | 0,249 | 0,087 | 0,093 | 7/14 |

| Fark | periyot | SENS | ECR_own | ECR_ref | p_exp |
|---|---|---|---|---|---|
| `teyit2` − `umut` | 15m | -0,037 [-0,046, -0,030] | +0,029 [+0,027, +0,030] | +0,028 [+0,026, +0,030] | +0,179 [+0,174, +0,183] |
| `teyit2` − `umut` | 1h | -0,016 [-0,021, -0,011] | +0,025 [+0,021, +0,030] | +0,025 [+0,021, +0,029] | +0,164 [+0,158, +0,169] |
| `teyit2` − `umut` | 5m | -0,067 [-0,079, -0,058] | +0,035 [+0,034, +0,037] | +0,035 [+0,033, +0,036] | +0,190 [+0,185, +0,197] |
| `yukdus` − `teyit2` | 15m | -0,060 [-0,068, -0,050] | +0,008 [+0,005, +0,010] | +0,049 [+0,046, +0,052] | -0,030 [-0,034, -0,027] |
| `yukdus` − `teyit2` | 1h | -0,049 [-0,053, -0,045] | -0,003 [-0,008, +0,002] | +0,030 [+0,024, +0,035] | -0,032 [-0,038, -0,026] |
| `yukdus` − `teyit2` | 5m | -0,033 [-0,048, -0,015] | +0,005 [+0,003, +0,007] | +0,044 [+0,041, +0,046] | -0,025 [-0,028, -0,022] |
| `yukdus` − `umut` | 15m | -0,096 [-0,101, -0,091] | +0,036 [+0,034, +0,039] | +0,077 [+0,074, +0,081] | +0,148 [+0,143, +0,154] |
| `yukdus` − `umut` | 1h | -0,065 [-0,070, -0,059] | +0,022 [+0,017, +0,027] | +0,054 [+0,048, +0,060] | +0,132 [+0,126, +0,138] |
| `yukdus` − `umut` | 5m | -0,100 [-0,107, -0,093] | +0,040 [+0,038, +0,042] | +0,078 [+0,076, +0,081] | +0,165 [+0,158, +0,174] |

## Puanlama (SPEC §4, 5 dk, kalibrasyon)

| Kural | sabit | 1 ECR_own ≤ ½ | 2 ECR_ref | 3 Brier/kal. | 4 süresi dolan ≤ 0,40 | 5 SENS | aday |
|---|---|---|---|---|---|---|---|
| `taban` | 8 | geçti | geçti | geçti | **kaldı** (0,426) | geçti | hayır |
| `teyit2` | 9 | geçti | geçti | geçti | geçti | geçti | evet |
| `yukdus` | 7 | geçti | geçti | geçti | geçti | geçti | evet |
| `olcek` | 8 | geçti | geçti | geçti | geçti | **kaldı** (+0,068) | hayır |
| `umut` | 9 | geçti | geçti | geçti | geçti | geçti | evet |

## Seçim (SPEC §4 sıralaması)

(a) 5 dk `SENS`: `yukdus` 0,099 < `teyit2` 0,132 < `umut` 0,199; aday−aday farklarının %95
aralıkları 0'ı kapsamıyor (5 dk, 15 dk, 1 sa'te aynı sıra) → ayırt edildi. **Seçilen gün içi
kural: `yukdus`** (5 dk, 15 dk, 1 sa). Günlükte RP4 kural seçmez: **günlük `r3` kalır**
(`yukdus`'un günlük kalibrasyon farkı 0,065 → 0,099 kötü; §6 sınamada denetlenecek).

**Seçimin dürüst okunuşu (sıralamayı değiştirmez, öneriye girer):**

1. `yukdus` kararı barın yüksek/düşüğüyle verir; `SENS` kapanışı oynatır. Kapanış eşiği
   aşmadıkça fitil değişmez, bu yüzden `yukdus`'un düşük `SENS`'i kısmen yapısaldır (savunucusu
   da yazdı: bariyer r3 kadarken `SENS` 0,070, `ECR_own` 0,223). Fitil gürültüsüne duyarlılığı
   düzenek ölçmüyor. Puanlama kuralı önceden yazıldığı için sıra değiştirilmedi.
2. Bütün adaylarda kazancın büyük kısmı bariyerin genişlemesinden gelir; süresi dolan payı
   0,118 → 0,32–0,34 (`umut`'ta 0,151).
3. `ECR_ref` (r3 eşiğiyle erken kapanma) `yukdus`'ta yalnız 0,148 → 0,115; `teyit2` 0,071,
   `umut` 0,036. "Tek barlık gürültüyle kapanma" sorusuna en doğrudan cevap `umut`'tur.
4. Brier her kuralda kendi tabanına eşit: karne gün içinde öngörü becerisi taşımıyor (r3'te de
   öyle). "Kalibrasyon bozulmadı" denebilir, "iyileşti" denemez.
5. 5 dk'da gerçek − yalancı eş r3'te −0,052 (dedektörler 5 dk'da yönü tutturamıyor); `yukdus`
   −0,000. M26'ya bakınız.

Karar Arkin'in: `DECISIONS.md` "Önerilen (RP4)". Sınama aralığındaki son ölçüm `S/RESULTS.md`.
