# M26 — Dedektör eşikleri gün içinde: kalibrasyon sonuçları ve seçim (sınama açılmadan önce)

Puanlama SPEC §5. Alternatifler: `rp4/m26b-olcek` (`olcek26`: periyoda göre ölçeklenen
pencere/çarpan), `rp4/m26c-mevsim` (`mevsim`: gün içi saat dilimi mevsimselliğine göre
düzeltilmiş hacim ve kırılım; 1 sa altı periyotta ürünle aynı). Başvuru `r3` = ürün seti.
Sonuç kuralı her koşuda `r3` (dedektör etkisini ayırmak için). Sayılar orkestratörce
`rp4/eval`'de yeniden üretildi. **Sınama açılmadan commit edildi.**

## Özet

- **Seçim:** 5 dk ve 15 dk'da aday yok → ürün sabitleri kalır. 1 sa'te `mevsim` aday (yanlış
  tetikleme farkının üst ucu −0,0005: sınırda). `mevsim` 1 sa altında ve günlükte ürünle bit
  eşit olduğu için öneri: **`rp4` dedektör seti = `mevsim`** (yalnız 1 sa ve üstü gün içi
  periyotta fark eder).
- **Gün içinde anlamsız dedektörler (SPEC §5 tanımı):** 5 dk, 15 dk ve 1 sa'te hiçbir dedektör
  0,55'e ulaşmıyor; kırılım, MA dizilimi, RSI aşırılık, hacim patlaması ve RSI uyumsuzluğunun
  isabet aralıklarının üst ucu 0,5'in altında (5 dk havuz 0,413; 15 dk 0,437; 1 sa 0,470;
  günlük 0,509). Sıkışma 15 dk'da 0,521 [0,501, 0,542] (0,5 üstü, 0,55 altı), 1 sa'te aralık 0,5'i
  kapsıyor, 5 dk'da 0,367. **Öneri (kapatma değil, periyot sınırı):** kırılım, MA dizilimi, RSI
  aşırılık en kısa 1 gün; sıkışma en kısa 15 dk; hacim patlaması ve RSI uyumsuzluğu günlükte de
  0,5'in altında (ayrı konu). Not: isabet süresi dolanı ıska sayar ve bariyer simetriktir; yönsüz
  beceri 0,5'in altına düşer.
- **Açılış/kapanış:** 5 dk'da açılış ve kapanış barlarında doğanlar günün geri kalanından **daha
  isabetli** (`ft` −0,050 [−0,057, −0,042]); 15 dk'da fark yok; 1 sa'te daha kötü (+0,032
  [+0,022, +0,042]): yanlış tetikleme yalnız 1 sa'te var. `mevsim` 1 sa'te açılış ıskalarını
  %25, kapanış ıskalarını %19 azaltıyor.
- **Akış yükü:** sembol·gün başına doğum 5 dk 18,2; 15 dk 6,4; 1 sa 1,5; günlük 0,20.

## Ayrıntılı tablolar (`harness report`, kalibrasyon)

### M26 — dedektör eşikleri gün içinde

Düzenek: `rounds/RP4/experiments/harness` (SPEC.md §5). Yalnız toplu sayılar; aralıklar sembol kümeli bootstrap %95. Açılış = günün ilk iki barı, kapanış = günün son barı (kapanış seansı birleşik). `ft` = isabet(geri kalan) − isabet(açılış ∪ kapanış).

## `mevsim` / kural `r3` — 5m — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 23104 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 134031 | 5.8012 | 1.00 | 0.415 [0.409, 0.421] | below | evet | 0.092 | 0.055 [0.046, 0.064] | 0.031 | -0.049 [-0.065, -0.033] | -0.028 [-0.037, -0.020] | -0.100 |
| `continuation.ma_stack` | 99787 | 4.3190 | 1.00 | 0.418 [0.407, 0.427] | below | evet | 0.051 | 0.033 [0.017, 0.047] | 0.017 | -0.006 [-0.030, 0.020] | -0.023 [-0.037, -0.010] | -0.105 |
| `extreme.rsi` | 51227 | 2.2172 | 1.00 | 0.420 [0.412, 0.426] | below | evet | 0.116 | 0.057 [0.044, 0.070] | 0.031 | -0.061 [-0.089, -0.035] | -0.032 [-0.044, -0.020] | -0.106 |
| `flow.volume_surge` | 93788 | 4.0594 | 1.00 | 0.408 [0.389, 0.423] | below | evet | 0.070 | 0.073 [0.061, 0.086] | 0.116 | 0.103 [0.087, 0.120] | -0.092 [-0.105, -0.079] | -0.060 |
| `reversal.rsi_divergence` | 26058 | 1.1279 | 1.00 | 0.419 [0.413, 0.425] | below | evet | 0.030 | 0.026 [-0.011, 0.064] | 0.010 | 0.093 [0.027, 0.152] | -0.043 [-0.076, -0.010] | -0.015 |
| `squeeze.bandwidth` | 14419 | 0.6241 | 1.00 | 0.367 [0.348, 0.383] | below | evet | 0.004 | 0.239 [0.120, 0.349] | 0.005 | 0.328 [0.229, 0.429] | -0.290 [-0.372, -0.211] | -0.212 |
| `(hepsi)` | 419310 | 18.1488 | 1.00 | 0.413 [0.403, 0.421] | below | evet | 0.074 | 0.056 [0.049, 0.064] | 0.044 | 0.038 [0.026, 0.050] | -0.050 [-0.057, -0.042] | -0.096 |

Puanlama (SPEC §5): (1) isabet düşmedi geçti · (2) yük ≤ 1,5× geçti · (3) yanlış tetikleme azaldı kaldı → aday: hayır


## `olcek26` / kural `r3` — 5m — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 23104 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 75397 | 3.2634 | 0.56 | 0.429 [0.422, 0.435] | below | evet | 0.119 | 0.041 [0.031, 0.052] | 0.036 | -0.073 [-0.094, -0.051] | -0.015 [-0.025, -0.005] | -0.086 |
| `continuation.ma_stack` | 99787 | 4.3190 | 1.00 | 0.418 [0.407, 0.427] | below | evet | 0.051 | 0.033 [0.017, 0.047] | 0.017 | -0.006 [-0.030, 0.020] | -0.023 [-0.037, -0.010] | -0.105 |
| `extreme.rsi` | 51227 | 2.2172 | 1.00 | 0.420 [0.412, 0.426] | below | evet | 0.116 | 0.057 [0.044, 0.070] | 0.031 | -0.061 [-0.089, -0.035] | -0.032 [-0.044, -0.020] | -0.106 |
| `flow.volume_surge` | 51320 | 2.2213 | 0.55 | 0.407 [0.386, 0.424] | below | evet | 0.084 | 0.080 [0.061, 0.097] | 0.173 | 0.101 [0.081, 0.121] | -0.094 [-0.113, -0.078] | -0.060 |
| `reversal.rsi_divergence` | 26058 | 1.1279 | 1.00 | 0.419 [0.413, 0.425] | below | evet | 0.030 | 0.026 [-0.011, 0.064] | 0.010 | 0.093 [0.027, 0.152] | -0.043 [-0.076, -0.010] | -0.015 |
| `squeeze.bandwidth` | 5940 | 0.2571 | 0.41 | 0.359 [0.339, 0.377] | below | evet | 0.002 | 0.198 [-0.123, 0.512] | 0.003 | 0.330 [0.089, 0.574] | -0.282 [-0.489, -0.109] | -0.220 |
| `(hepsi)` | 309729 | 13.4059 | 0.74 | 0.418 [0.408, 0.426] | below | evet | 0.081 | 0.051 [0.043, 0.059] | 0.049 | 0.027 [0.015, 0.039] | -0.042 [-0.050, -0.034] | -0.091 |

Puanlama (SPEC §5): (1) isabet düşmedi geçti · (2) yük ≤ 1,5× geçti · (3) yanlış tetikleme azaldı kaldı → aday: hayır


## `r3` / kural `r3` — 5m — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 23104 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 134031 | 5.8012 | 1.00 | 0.415 [0.409, 0.421] | below | evet | 0.092 | 0.055 [0.046, 0.064] | 0.031 | -0.049 [-0.065, -0.033] | -0.028 [-0.037, -0.020] | -0.100 |
| `continuation.ma_stack` | 99787 | 4.3190 | 1.00 | 0.418 [0.407, 0.427] | below | evet | 0.051 | 0.033 [0.017, 0.047] | 0.017 | -0.006 [-0.030, 0.020] | -0.023 [-0.037, -0.010] | -0.105 |
| `extreme.rsi` | 51227 | 2.2172 | 1.00 | 0.420 [0.412, 0.426] | below | evet | 0.116 | 0.057 [0.044, 0.070] | 0.031 | -0.061 [-0.089, -0.035] | -0.032 [-0.044, -0.020] | -0.106 |
| `flow.volume_surge` | 93788 | 4.0594 | 1.00 | 0.408 [0.389, 0.423] | below | evet | 0.070 | 0.073 [0.061, 0.086] | 0.116 | 0.103 [0.087, 0.120] | -0.092 [-0.105, -0.079] | -0.060 |
| `reversal.rsi_divergence` | 26058 | 1.1279 | 1.00 | 0.419 [0.413, 0.425] | below | evet | 0.030 | 0.026 [-0.011, 0.064] | 0.010 | 0.093 [0.027, 0.152] | -0.043 [-0.076, -0.010] | -0.015 |
| `squeeze.bandwidth` | 14419 | 0.6241 | 1.00 | 0.367 [0.348, 0.383] | below | evet | 0.004 | 0.239 [0.120, 0.349] | 0.005 | 0.328 [0.229, 0.429] | -0.290 [-0.372, -0.211] | -0.212 |
| `(hepsi)` | 419310 | 18.1488 | 1.00 | 0.413 [0.403, 0.421] | below | evet | 0.074 | 0.056 [0.049, 0.064] | 0.044 | 0.038 [0.026, 0.050] | -0.050 [-0.057, -0.042] | -0.096 |

## `mevsim` / kural `r3` — 15m — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 23104 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 49721 | 2.1521 | 1.00 | 0.435 [0.429, 0.440] | below | evet | 0.207 | 0.012 [0.000, 0.023] | 0.066 | -0.059 [-0.079, -0.040] | 0.005 [-0.006, 0.017] | -0.080 |
| `continuation.ma_stack` | 31357 | 1.3572 | 1.00 | 0.439 [0.430, 0.447] | below | evet | 0.115 | -0.028 [-0.046, -0.010] | 0.039 | -0.044 [-0.073, -0.019] | 0.032 [0.016, 0.048] | -0.083 |
| `extreme.rsi` | 20007 | 0.8660 | 1.00 | 0.430 [0.422, 0.437] | below | evet | 0.217 | 0.016 [-0.001, 0.032] | 0.060 | -0.069 [-0.097, -0.040] | 0.003 [-0.013, 0.018] | -0.096 |
| `flow.volume_surge` | 30726 | 1.3299 | 1.00 | 0.437 [0.426, 0.447] | below | evet | 0.216 | -0.018 [-0.032, -0.005] | 0.290 | 0.036 [0.023, 0.050] | -0.013 [-0.024, -0.002] | -0.031 |
| `reversal.rsi_divergence` | 11021 | 0.4770 | 1.00 | 0.414 [0.407, 0.422] | below | evet | 0.063 | -0.045 [-0.083, -0.006] | 0.034 | 0.141 [0.097, 0.187] | -0.020 [-0.054, 0.013] | -0.020 |
| `squeeze.bandwidth` | 5347 | 0.2314 | 1.00 | 0.521 [0.501, 0.542] | below | — | 0.025 | -0.106 [-0.189, -0.016] | 0.026 | 0.044 [-0.042, 0.123] | 0.030 [-0.038, 0.097] | -0.058 |
| `(hepsi)` | 148179 | 6.4136 | 1.00 | 0.437 [0.430, 0.442] | below | evet | 0.174 | -0.007 [-0.015, 0.002] | 0.102 | -0.002 [-0.012, 0.009] | 0.005 [-0.002, 0.012] | -0.072 |

Puanlama (SPEC §5): (1) isabet düşmedi geçti · (2) yük ≤ 1,5× geçti · (3) yanlış tetikleme azaldı kaldı → aday: hayır


## `olcek26` / kural `r3` — 15m — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 23104 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 30192 | 1.3068 | 0.61 | 0.441 [0.434, 0.448] | below | evet | 0.208 | 0.007 [-0.006, 0.019] | 0.064 | -0.072 [-0.098, -0.046] | 0.012 [-0.001, 0.024] | -0.074 |
| `continuation.ma_stack` | 31357 | 1.3572 | 1.00 | 0.439 [0.430, 0.447] | below | evet | 0.115 | -0.028 [-0.046, -0.010] | 0.039 | -0.044 [-0.073, -0.019] | 0.032 [0.016, 0.048] | -0.083 |
| `extreme.rsi` | 20007 | 0.8660 | 1.00 | 0.430 [0.422, 0.437] | below | evet | 0.217 | 0.016 [-0.001, 0.032] | 0.060 | -0.069 [-0.097, -0.040] | 0.003 [-0.013, 0.018] | -0.096 |
| `flow.volume_surge` | 18921 | 0.8189 | 0.62 | 0.436 [0.423, 0.448] | below | evet | 0.273 | -0.003 [-0.022, 0.013] | 0.304 | 0.027 [0.006, 0.046] | -0.012 [-0.028, 0.004] | -0.031 |
| `reversal.rsi_divergence` | 11021 | 0.4770 | 1.00 | 0.414 [0.407, 0.422] | below | evet | 0.063 | -0.045 [-0.083, -0.006] | 0.034 | 0.141 [0.097, 0.187] | -0.020 [-0.054, 0.013] | -0.020 |
| `squeeze.bandwidth` | 2372 | 0.1027 | 0.44 | 0.523 [0.497, 0.549] | below | evet | 0.019 | -0.022 [-0.173, 0.122] | 0.032 | 0.038 [-0.079, 0.144] | -0.016 [-0.103, 0.070] | -0.056 |
| `(hepsi)` | 113870 | 4.9286 | 0.77 | 0.437 [0.430, 0.443] | below | evet | 0.177 | -0.004 [-0.014, 0.006] | 0.093 | -0.011 [-0.025, 0.002] | 0.007 [-0.002, 0.016] | -0.072 |

Puanlama (SPEC §5): (1) isabet düşmedi geçti · (2) yük ≤ 1,5× geçti · (3) yanlış tetikleme azaldı kaldı → aday: hayır


## `r3` / kural `r3` — 15m — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 23104 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 49721 | 2.1521 | 1.00 | 0.435 [0.429, 0.440] | below | evet | 0.207 | 0.012 [0.000, 0.023] | 0.066 | -0.059 [-0.079, -0.040] | 0.005 [-0.006, 0.017] | -0.080 |
| `continuation.ma_stack` | 31357 | 1.3572 | 1.00 | 0.439 [0.430, 0.447] | below | evet | 0.115 | -0.028 [-0.046, -0.010] | 0.039 | -0.044 [-0.073, -0.019] | 0.032 [0.016, 0.048] | -0.083 |
| `extreme.rsi` | 20007 | 0.8660 | 1.00 | 0.430 [0.422, 0.437] | below | evet | 0.217 | 0.016 [-0.001, 0.032] | 0.060 | -0.069 [-0.097, -0.040] | 0.003 [-0.013, 0.018] | -0.096 |
| `flow.volume_surge` | 30726 | 1.3299 | 1.00 | 0.437 [0.426, 0.447] | below | evet | 0.216 | -0.018 [-0.032, -0.005] | 0.290 | 0.036 [0.023, 0.050] | -0.013 [-0.024, -0.002] | -0.031 |
| `reversal.rsi_divergence` | 11021 | 0.4770 | 1.00 | 0.414 [0.407, 0.422] | below | evet | 0.063 | -0.045 [-0.083, -0.006] | 0.034 | 0.141 [0.097, 0.187] | -0.020 [-0.054, 0.013] | -0.020 |
| `squeeze.bandwidth` | 5347 | 0.2314 | 1.00 | 0.521 [0.501, 0.542] | below | — | 0.025 | -0.106 [-0.189, -0.016] | 0.026 | 0.044 [-0.042, 0.123] | 0.030 [-0.038, 0.097] | -0.058 |
| `(hepsi)` | 148179 | 6.4136 | 1.00 | 0.437 [0.430, 0.442] | below | evet | 0.174 | -0.007 [-0.015, 0.002] | 0.102 | -0.002 [-0.012, 0.009] | 0.005 [-0.002, 0.012] | -0.072 |

## `mevsim` / kural `r3` — 1h — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 24086 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 11980 | 0.4974 | 0.86 | 0.472 [0.461, 0.481] | below | evet | 0.322 | -0.045 [-0.064, -0.026] | 0.165 | -0.059 [-0.089, -0.032] | 0.050 [0.033, 0.069] | -0.043 |
| `continuation.ma_stack` | 7959 | 0.3304 | 1.00 | 0.484 [0.474, 0.495] | below | evet | 0.303 | 0.003 [-0.022, 0.030] | 0.125 | -0.022 [-0.054, 0.009] | 0.004 [-0.016, 0.025] | -0.038 |
| `extreme.rsi` | 5544 | 0.2302 | 1.00 | 0.473 [0.459, 0.487] | below | evet | 0.433 | -0.026 [-0.057, 0.000] | 0.150 | -0.050 [-0.089, -0.015] | 0.033 [0.008, 0.059] | -0.053 |
| `flow.volume_surge` | 3936 | 0.1634 | 0.86 | 0.458 [0.442, 0.476] | below | evet | 0.223 | -0.005 [-0.040, 0.029] | 0.075 | -0.037 [-0.092, 0.018] | 0.013 [-0.018, 0.045] | -0.009 |
| `reversal.rsi_divergence` | 3379 | 0.1403 | 1.00 | 0.458 [0.440, 0.476] | below | evet | 0.186 | -0.039 [-0.080, 0.001] | 0.094 | -0.022 [-0.075, 0.030] | 0.033 [0.000, 0.063] | 0.024 |
| `squeeze.bandwidth` | 1262 | 0.0524 | 1.00 | 0.504 [0.475, 0.533] | below | evet | 0.151 | -0.023 [-0.110, 0.062] | 0.115 | 0.050 [-0.035, 0.132] | -0.009 [-0.077, 0.058] | -0.075 |
| `(hepsi)` | 34060 | 1.4141 | 0.93 | 0.473 [0.466, 0.481] | below | evet | 0.304 | -0.022 [-0.035, -0.009] | 0.134 | -0.038 [-0.055, -0.020] | 0.027 [0.016, 0.038] | -0.036 |

Puanlama (SPEC §5): (1) isabet düşmedi geçti · (2) yük ≤ 1,5× geçti · (3) yanlış tetikleme azaldı geçti → aday: evet


## `olcek26` / kural `r3` — 1h — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 24086 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 10033 | 0.4165 | 0.72 | 0.474 [0.463, 0.484] | below | evet | 0.431 | -0.059 [-0.077, -0.039] | 0.154 | -0.071 [-0.097, -0.047] | 0.062 [0.044, 0.080] | -0.041 |
| `continuation.ma_stack` | 7959 | 0.3304 | 1.00 | 0.484 [0.474, 0.495] | below | evet | 0.303 | 0.003 [-0.022, 0.030] | 0.125 | -0.022 [-0.054, 0.009] | 0.004 [-0.016, 0.025] | -0.038 |
| `extreme.rsi` | 5544 | 0.2302 | 1.00 | 0.473 [0.459, 0.487] | below | evet | 0.433 | -0.026 [-0.057, 0.000] | 0.150 | -0.050 [-0.089, -0.015] | 0.033 [0.008, 0.059] | -0.053 |
| `flow.volume_surge` | 2788 | 0.1158 | 0.61 | 0.451 [0.433, 0.470] | below | evet | 0.486 | 0.005 [-0.028, 0.037] | 0.237 | -0.020 [-0.066, 0.023] | 0.003 [-0.027, 0.036] | -0.017 |
| `reversal.rsi_divergence` | 3379 | 0.1403 | 1.00 | 0.458 [0.440, 0.476] | below | evet | 0.186 | -0.039 [-0.080, 0.001] | 0.094 | -0.022 [-0.075, 0.030] | 0.033 [0.000, 0.063] | 0.024 |
| `squeeze.bandwidth` | 662 | 0.0275 | 0.52 | 0.530 [0.489, 0.571] | contains | evet | 0.145 | -0.010 [-0.131, 0.104] | 0.121 | 0.092 [-0.019, 0.195] | -0.036 [-0.117, 0.046] | -0.049 |
| `(hepsi)` | 30365 | 1.2607 | 0.83 | 0.474 [0.467, 0.481] | below | evet | 0.370 | -0.028 [-0.041, -0.014] | 0.146 | -0.042 [-0.059, -0.026] | 0.032 [0.020, 0.044] | -0.035 |

Puanlama (SPEC §5): (1) isabet düşmedi geçti · (2) yük ≤ 1,5× geçti · (3) yanlış tetikleme azaldı kaldı → aday: hayır


## `r3` / kural `r3` — 1h — calib

Pencere 2025-11-03 → 2026-10-18; 100 sembol, 241 gün, 24086 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 13988 | 0.5808 | 1.00 | 0.466 [0.455, 0.475] | below | evet | 0.422 | -0.049 [-0.067, -0.030] | 0.150 | -0.066 [-0.094, -0.039] | 0.053 [0.036, 0.071] | -0.048 |
| `continuation.ma_stack` | 7959 | 0.3304 | 1.00 | 0.484 [0.474, 0.495] | below | evet | 0.303 | 0.003 [-0.022, 0.030] | 0.125 | -0.022 [-0.054, 0.009] | 0.004 [-0.016, 0.025] | -0.038 |
| `extreme.rsi` | 5544 | 0.2302 | 1.00 | 0.473 [0.459, 0.487] | below | evet | 0.433 | -0.026 [-0.057, 0.000] | 0.150 | -0.050 [-0.089, -0.015] | 0.033 [0.008, 0.059] | -0.053 |
| `flow.volume_surge` | 4578 | 0.1901 | 1.00 | 0.452 [0.439, 0.467] | below | evet | 0.473 | -0.005 [-0.040, 0.030] | 0.269 | -0.022 [-0.059, 0.012] | 0.011 [-0.021, 0.044] | -0.015 |
| `reversal.rsi_divergence` | 3379 | 0.1403 | 1.00 | 0.458 [0.440, 0.476] | below | evet | 0.186 | -0.039 [-0.080, 0.001] | 0.094 | -0.022 [-0.075, 0.030] | 0.033 [0.000, 0.063] | 0.024 |
| `squeeze.bandwidth` | 1262 | 0.0524 | 1.00 | 0.504 [0.475, 0.533] | below | evet | 0.151 | -0.023 [-0.110, 0.062] | 0.115 | 0.050 [-0.035, 0.132] | -0.009 [-0.077, 0.058] | -0.075 |
| `(hepsi)` | 36710 | 1.5241 | 1.00 | 0.470 [0.463, 0.477] | below | evet | 0.373 | -0.028 [-0.040, -0.016] | 0.153 | -0.043 [-0.057, -0.028] | 0.032 [0.022, 0.042] | -0.039 |

## `mevsim` / kural `r3` — 1d — calib

Pencere 2018-01-02 → 2025-01-01; 98 sembol, 1751 gün, 146195 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 12927 | 0.0884 | 1.00 | 0.515 [0.505, 0.524] | below | — | — | — | — | — | — | — |
| `continuation.ma_stack` | 5872 | 0.0402 | 1.00 | 0.523 [0.510, 0.535] | below | — | — | — | — | — | — | — |
| `extreme.rsi` | 4497 | 0.0308 | 1.00 | 0.526 [0.510, 0.540] | below | — | — | — | — | — | — | — |
| `flow.volume_surge` | 2474 | 0.0169 | 1.00 | 0.468 [0.450, 0.485] | below | evet | — | — | — | — | — | — |
| `reversal.rsi_divergence` | 2562 | 0.0175 | 1.00 | 0.434 [0.414, 0.455] | below | evet | — | — | — | — | — | — |
| `squeeze.bandwidth` | 963 | 0.0066 | 1.00 | 0.579 [0.547, 0.609] | contains | — | — | — | — | — | — | — |
| `(hepsi)` | 29295 | 0.2004 | 1.00 | 0.509 [0.502, 0.516] | below | — | — | — | — | — | — | — |

## `olcek26` / kural `r3` — 1d — calib

Pencere 2018-01-02 → 2025-01-01; 98 sembol, 1751 gün, 146195 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 12927 | 0.0884 | 1.00 | 0.515 [0.505, 0.524] | below | — | — | — | — | — | — | — |
| `continuation.ma_stack` | 5872 | 0.0402 | 1.00 | 0.523 [0.510, 0.535] | below | — | — | — | — | — | — | — |
| `extreme.rsi` | 4497 | 0.0308 | 1.00 | 0.526 [0.510, 0.540] | below | — | — | — | — | — | — | — |
| `flow.volume_surge` | 2474 | 0.0169 | 1.00 | 0.468 [0.450, 0.485] | below | evet | — | — | — | — | — | — |
| `reversal.rsi_divergence` | 2562 | 0.0175 | 1.00 | 0.434 [0.414, 0.455] | below | evet | — | — | — | — | — | — |
| `squeeze.bandwidth` | 963 | 0.0066 | 1.00 | 0.579 [0.547, 0.609] | contains | — | — | — | — | — | — | — |
| `(hepsi)` | 29295 | 0.2004 | 1.00 | 0.509 [0.502, 0.516] | below | — | — | — | — | — | — | — |

## `r3` / kural `r3` — 1d — calib

Pencere 2018-01-02 → 2025-01-01; 98 sembol, 1751 gün, 146195 sembol·gün.

| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `breakout.range` | 12927 | 0.0884 | 1.00 | 0.515 [0.505, 0.524] | below | — | — | — | — | — | — | — |
| `continuation.ma_stack` | 5872 | 0.0402 | 1.00 | 0.523 [0.510, 0.535] | below | — | — | — | — | — | — | — |
| `extreme.rsi` | 4497 | 0.0308 | 1.00 | 0.526 [0.510, 0.540] | below | — | — | — | — | — | — | — |
| `flow.volume_surge` | 2474 | 0.0169 | 1.00 | 0.468 [0.450, 0.485] | below | evet | — | — | — | — | — | — |
| `reversal.rsi_divergence` | 2562 | 0.0175 | 1.00 | 0.434 [0.414, 0.455] | below | evet | — | — | — | — | — | — |
| `squeeze.bandwidth` | 963 | 0.0066 | 1.00 | 0.579 [0.547, 0.609] | contains | — | — | — | — | — | — | — |
| `(hepsi)` | 29295 | 0.2004 | 1.00 | 0.509 [0.502, 0.516] | below | — | — | — | — | — | — | — |

