# S — Son ölçüm: sınama aralığı (2026-10-19 → 2026-11-20)

Seçim `M25/RESULTS.md` ve `M26/RESULTS.md`'de, **sınama açılmadan** commit edildi (`8bb7571`);
sınama ilk kez bu dosyanın ölçümünde açıldı (`--split test`). Ürün kaydı `rp4` (`EngineConfig::named`,
`c1bb56b`) sınamada da deney crate'leriyle bayt eşit sonuç verdi (5 dk/15 dk/1 sa `yukdus`, günlük
`r3`). 100 sembol (BIST100), 24 işlem günü (yarım gün 2026-10-28 dahil); günlükte 473 gün.

Yeniden üretim (önbellekten):

```sh
H=target/wt/h/release/harness   # rounds/RP4/experiments/harness, cargo build --release
$H m25 --data fixtures/local/rp4/full --rule <r3|rp4|teyit2|umut> --period <5m|15m|1h|1d> --split test --out fixtures/local/rp4/test
$H m25 --data fixtures/local/rp4/tradebars/full --rule <r3|rp4> --period <5m|15m|1h> --split test --out fixtures/local/rp4/test-trades
$H cases --data fixtures/local/rp4/full --rule rp4 --period 5m --split test [--day 2026-11-20] --out fixtures/local/rp4/cases
```

## 1. M25 — REST barları (yönlü satır; fark = kural − r3, sembol kümeli bootstrap %95)

| Periyot | Kural | gün | n | p_real | p_inv | p_exp | ECR_own (fark, %95) | ECR_ref | SENS (fark, %95) | REV | Brier / taban | kal. farkı (fark) | gerçek−yalancı | ömür |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 5m | `r3` | 24 | 39.711 | 0,441 | 0,454 | 0,106 | 0,156 (0,000 [0,000, 0,000]) | 0,156 | 0,198 (0,000 [0,000, 0,000]) | 0,238 | 0,246 / 0,247 | 0,032 (0,000 [0,000, 0,000]) | -0,013 | 4/10 |
| 5m | `rp4` | 24 | 35.453 | 0,363 | 0,333 | 0,304 | 0,065 (-0,091 [-0,097, -0,085]) | 0,112 | 0,097 (-0,100 [-0,110, -0,090]) | 0,198 | 0,232 / 0,232 | 0,038 (0,007 [0,001, 0,013]) | 0,030 | 7/17 |
| 5m | `teyit2` | 24 | 32.559 | 0,341 | 0,337 | 0,321 | 0,063 (-0,093 [-0,098, -0,087]) | 0,071 | 0,123 (-0,075 [-0,091, -0,061]) | 0,143 | 0,225 / 0,226 | 0,036 (0,004 [0,000, 0,008]) | 0,004 | 8/17 |
| 5m | `umut` | 24 | 34.965 | 0,395 | 0,462 | 0,144 | 0,027 (-0,129 [-0,135, -0,123]) | 0,036 | 0,181 (-0,017 [-0,024, -0,010]) | 0,173 | 0,239 / 0,240 | 0,039 (0,007 [0,003, 0,012]) | 0,007 | 8/15 |
| 15m | `r3` | 24 | 14.232 | 0,466 | 0,426 | 0,109 | 0,147 (0,000 [0,000, 0,000]) | 0,147 | 0,141 (0,000 [0,000, 0,000]) | 0,242 | 0,248 / 0,250 | 0,048 (0,000 [0,000, 0,000]) | 0,040 | 4/10 |
| 15m | `rp4` | 24 | 12.810 | 0,384 | 0,332 | 0,283 | 0,070 (-0,076 [-0,086, -0,068]) | 0,116 | 0,040 (-0,101 [-0,109, -0,092]) | 0,199 | 0,238 / 0,238 | 0,061 (0,014 [0,005, 0,021]) | 0,052 | 7/15 |
| 15m | `teyit2` | 24 | 11.621 | 0,367 | 0,331 | 0,302 | 0,059 (-0,088 [-0,098, -0,078]) | 0,068 | 0,086 (-0,055 [-0,066, -0,044]) | 0,144 | 0,232 / 0,234 | 0,053 (0,005 [-0,003, 0,011]) | 0,036 | 8/15 |
| 15m | `umut` | 24 | 12.593 | 0,415 | 0,445 | 0,140 | 0,028 (-0,119 [-0,128, -0,111]) | 0,039 | 0,125 (-0,016 [-0,024, -0,007]) | 0,177 | 0,243 / 0,244 | 0,054 (0,006 [-0,001, 0,013]) | 0,043 | 7/15 |
| 1h | `r3` | 24 | 3.621 | 0,544 | 0,390 | 0,065 | 0,121 (0,000 [0,000, 0,000]) | 0,121 | 0,074 (0,000 [0,000, 0,000]) | 0,203 | 0,250 / 0,254 | 0,090 (0,000 [0,000, 0,000]) | 0,154 | 4/10 |
| 1h | `rp4` | 24 | 3.135 | 0,463 | 0,321 | 0,216 | 0,040 (-0,081 [-0,099, -0,064]) | 0,073 | 0,011 (-0,063 [-0,073, -0,052]) | 0,183 | 0,256 / 0,254 | 0,105 (0,015 [0,001, 0,030]) | 0,142 | 7/14 |
| 1h | `teyit2` | 24 | 2.854 | 0,447 | 0,296 | 0,257 | 0,039 (-0,082 [-0,099, -0,066]) | 0,044 | 0,059 (-0,015 [-0,027, -0,002]) | 0,144 | 0,251 / 0,254 | 0,096 (0,006 [-0,009, 0,022]) | 0,152 | 8/15 |
| 1h | `umut` | 24 | 3.159 | 0,488 | 0,402 | 0,110 | 0,019 (-0,102 [-0,121, -0,085]) | 0,028 | 0,070 (-0,004 [-0,018, 0,011]) | 0,140 | 0,254 / 0,257 | 0,106 (0,015 [0,001, 0,029]) | 0,147 | 7/13 |
| 1d | `r3` | 473 | 8.659 | 0,468 | 0,464 | 0,068 | 0,140 (0,000 [0,000, 0,000]) | 0,140 | 0,034 (0,000 [0,000, 0,000]) | 0,315 | 0,251 / 0,250 | 0,047 (0,000 [0,000, 0,000]) | 0,004 | 4/10 |
| 1d | `rp4` | 473 | 8.659 | 0,468 | 0,464 | 0,068 | 0,140 (0,000 [0,000, 0,000]) | 0,140 | 0,034 (0,000 [0,000, 0,000]) | 0,315 | 0,251 / 0,250 | 0,047 (0,000 [0,000, 0,000]) | 0,004 | 4/10 |
| 1d | `yukdus` | 473 | 7.453 | 0,377 | 0,371 | 0,252 | 0,032 (-0,107 [-0,119, -0,096]) | 0,081 | 0,002 (-0,032 [-0,039, -0,026]) | 0,253 | 0,233 / 0,239 | 0,051 (0,004 [-0,008, 0,016]) | 0,005 | 7/15 |

**Günlük (ölçüt 7):** `rp4` günlükte `r3`'ün aynısıdır (bayt eşit sonuç). R3 başvurusu ve bu round:

| Ölçü | R3/M5 (gerçek, 5 sembol) | RP4 kalibrasyon `r3` (98 sembol, 2018→2024) | RP4 sınama `r3` = `rp4` (2025-01→2026-11) |
|---|---|---|---|
| gerçek − yalancı eş | 0,088 | 0,087 | 0,004 |
| yönlü isabet (p_real) | 0,50–0,54 | 0,507 | 0,468 |
| süresi dolan | — | 0,073 | 0,068 |
| Brier / taban | — | 0,246 / 0,250 | 0,251 / 0,250 |

**Hedef (5 dk erken kapanma ≤ başvurunun yarısı, kalibrasyon bozulmadan):** `rp4` 0,065 / `r3`
0,156 = **0,42 → tuttu**. Kalibrasyon: Brier kendi tabanına eşit (0,232 / 0,232; `r3` 0,246 / 0,247:
iki kuralda da beceri yok); kalibrasyon farkı +0,007 [+0,001, +0,013] kötü (SPEC §4'ün +0,03 sınırı
içinde, aralık 0'ı kapsamıyor: küçük ama gerçek bir kötüleşme). Süresi dolan 0,106 → 0,304.
Tek bar duyarlılığı 0,198 → 0,097 (kısmen yapısal; `M25/RESULTS.md` okunuş 1).

Kalibrasyondaki sıra sınamada korundu: `SENS` `rp4` < `teyit2` < `umut`; `ECR_own` `umut` en düşük.

## 2. M25 — işlemden barlar (canlı yolun bar kurucusu, `qp live export-trade-bars`)

| Periyot | Kural | n | p_real | p_exp | ECR_own | SENS | Brier / taban |
|---|---|---|---|---|---|---|---|
| 5m | `r3` | 40.153 | 0,441 | 0,105 | 0,156 | 0,198 | 0,246 / 0,248 |
| 5m | `rp4` | 35.862 | 0,363 | 0,304 | 0,065 | 0,098 | 0,231 / 0,232 |
| 15m | `r3` | 14.417 | 0,468 | 0,107 | 0,148 | 0,141 | 0,248 / 0,250 |
| 15m | `rp4` | 12.966 | 0,386 | 0,281 | 0,071 | 0,039 | 0,236 / 0,238 |
| 1h | `r3` | 3.618 | 0,544 | 0,065 | 0,121 | 0,074 | 0,256 / 0,249 |
| 1h | `rp4` | 3.133 | 0,463 | 0,216 | 0,040 | 0,011 | 0,265 / 0,249 |

İki kaynak aynı sonucu verir (fark ≤ 0,002). İşlem barlarında karne yalnız 2026-10-05'ten başlar
(geçmiş işlem önbelleği); 1 sa Brier/kalibrasyon farkı bu yüzden REST'tekinden kötü görünür
(karne kısa), kuralın özelliği değildir.

**M27 ikinci ölçü — aynı günlerde iki kaynaktan doğan kapsüller** (`harness births`, sınama, doğum =
(sembol, doğum barı, karne anahtarı)):

| Kural | Periyot | REST | işlem | ikisinde | yalnız REST | yalnız işlem | farklı pay |
|---|---|---|---|---|---|---|---|
| `r3` | 5m | 43.610 | 44.103 | 43.335 | 275 | 768 | 0,024 |
| `r3` | 1h | 3.925 | 3.903 | 3.894 | 31 | 9 | 0,010 |
| `rp4` | 5m | 39.401 | 39.862 | 39.141 | 260 | 721 | 0,024 |
| `rp4` | 1h | 3.480 | 3.459 | 3.450 | 30 | 9 | 0,011 |

"Yalnız işlem" doğumlarının çoğu REST serisinde eksik barların olduğu yerlerdedir (5 dk'da 2.561 bar
yalnız işlem tarafında, `M27/RESULTS.md`); fiyat farkları bir sembolün geriye dönük düzeltmesinden
(karşılaştırma düzeltilmiş REST'le, işlem barları düzeltilmemiş).

## 3. Arkin'in gördüğü durum (5 dk, düşüş iddiası, ertesi barda "tutmadı", sonra iddia yönünde)

`harness cases` (aynı motor, aynı dedektörler; doğum iki kuralda aynı): 2026-11-20 günü, BIST100,
5 dk. Ayrıntı (sembol, saat, ATR birimiyle kapanış yolu) yalnız yerelde:
`fixtures/local/rp4/cases/cases.rp4.5m.2026-11-20.local.jsonl` (Arkin ekranda gördüğü sembolü burada bulabilir).

| | REST barları | işlem barları |
|---|---|---|
| `r3` düşüş iddiası, **ertesi barda** tutmadı | 79 | 79 |
| bunlardan ufuk içinde kapanışı hedefe ulaşan (Arkin'in durumu) | **22** | **22** |
| aynı hikâye `rp4` ile: gerçekleşti | 7 | 7 |
| tutmadı (daha sonra, geniş eşikte) | 7 | 7 |
| süresi doldu | 5 | 5 |
| gün sonunda açık | 2 | 2 |
| `rp4`'te doğmadı (tema birleşmesi farkı) | 1 | 1 |

Bütün sınama aralığında (24 gün): 556 durum (işlem barlarında 574); `rp4` ile 161 gerçekleşti, 256
tutmadı, 64 süresi doldu, 2 açık, 73 doğmadı. Örnek (yerel dosyadaki ilk satır, sembolsüz): doğum
16:25, `r3` eşiği ≈ 0,95 ATR; ertesi kapanış +1,24 ATR (iddianın tersine) → `r3` "tutmadı"; sonraki
kapanışlar −0,83 … −1,66 ATR (iddia yönünde, `r3` hedefini aştı); `rp4`'te eşik 1,9 ATR, ne hedef ne
durdurma → **süresi doldu**.

**Dürüst okuma:** `rp4` Arkin'in gördüğü "ertesi barda tutmadı" durumunu ortadan kaldırır (en erken
durdurma 2,0× eşikle), ama bu durumların yalnız üçte biri "gerçekleşti"ye döner; yarısına yakını yine
"tutmadı" ile kapanır. Bu deseni en iyi `umut` çözüyor (aynı günde 22'nin 13'ü gerçekleşti); `umut`
önceden yazılı sıralamada (`SENS`) üçüncü oldu.

Hızlandırılmış benzetimde (`qp sim --start 2026-11-20 --rule r3|rp4`, canlı yol, işlemden 5 dk bar)
aynı gün: `r3` 1.834 kapsül doğumu, `rp4` 1.672. Benzetimin olay akışından kurulan 5 dk barları
`export-trade-bars` dosyalarıyla **9.588 / 9.588 bar o/h/l/c bit eşit** (eksik 0); yani tablo 2 ve bu
bölümün işlem sütunu benzetimin gördüğü barlarla hesaplandı.

## 4. M26 — sınama (sonuç kuralı `r3`)

| Periyot | Set | doğum / sembol·gün | havuz isabeti | açılış/kapanış `ft` |
|---|---|---|---|---|
| 1h | `r3` | 1,64 | 0,544 [0,527, 0,561] | +0,061 [+0,027, +0,094] |
| 1h | `rp4` (`mevsim`) | 1,52 | 0,546 [0,529, 0,564] | +0,043 [+0,011, +0,076] |

5 dk, 15 dk ve günlükte `rp4` seti ürün setiyle bit eşit (sonuçlar aynı). Sınama aralığında 1 sa
dedektörlerinin isabeti kalibrasyondakinden yüksek (0,47 → 0,54): rejim farkı. Gün içinde 0,55'i
geçen yalnız `rp4`'ün 1 sa kırılımı (0,575 [0,554, 0,596]).

## 5. Günlük periyot (ölçüt 7)

`rp4` günlükte `r3`'ün aynısıdır (sınamada bayt eşit sonuç); günlük davranış **değişmedi**. R3 başvurusu
ile karşılaştırma tablo 1'in altındadır: kalibrasyon aralığında `r3` R3'ün gerçek veri ölçümünü yeniden
üretiyor (gerçek − yalancı eş 0,087 ↔ 0,088); 2025-01 → 2026-11 sınama aralığında aynı kuralın bu farkı
0,004'e düşüyor — piyasa döneminin etkisi, RP4 değişikliğinin değil.
