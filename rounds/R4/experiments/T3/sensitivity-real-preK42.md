# R4/T3 — K42 öncesi kod (4ee7fc6) ile aynı güncel yerel veride hassasiyet benzetimi (karşılaştırma tabanı; yalnız toplu)

Üreten: `cargo run --release -p qp-capsule --example sensitivity -- real` (belirleyici; iki koşu bayt eşit). Motor `EngineConfig::r3()` (tema `kinddir`, sonuç `barrier`, sıralama `confirm`), yedi varsayılan dedektör. Tanımlar `crates/qp-capsule/examples/sensitivity.rs` başında.

- Veri: `real-1day` (31 enstrüman, 2220 işlem günü, sayılan 2170), `real-1hour` (31 enstrüman, 502 işlem günü, sayılan 496).
- Kitle: izleme listesi 10 enstrüman; pozisyon 3 (AEFES 100, ASELS -50, EREGL 200); etkin niyet BRSAN; evren 11 enstrüman.
- Günlük kapsül = o işlem günü evrende **doğan** ve seviyeden (ya da pozisyon/niyetten) geçen hikâye; ikincil: o gün geçen her hikâye olayı. İlk 50 bar (ısınma) sayılmaz.
- Sistem seviyeyi hiç değiştirmedi (her koşuda kitle önce/sonra karşılaştırıldı, assert): **evet**.

## Seviye başına günlük kapsül (kitle; 2666 gün havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 2.24 | 1 | 7 | 22 | 0.34 |
| `balanced` doğum | 2.86 | 1 | 9 | 29 | 0.29 |
| `all` doğum | 4.90 | 2 | 16 | 44 | 0.15 |
| `important` tüm olaylar | 6.00 | 3 | 19 | 58 | 0.12 |
| `balanced` tüm olaylar | 7.42 | 3 | 24 | 71 | 0.08 |
| `all` tüm olaylar | 12.99 | 6 | 43 | 115 | 0.02 |

## Kullanıcı grubu varsayılanı (seviye seçilmemiş)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `habituated` → `all` doğum | 4.90 | 2 | 16 | 44 | 0.15 |
| `future` → `important` doğum | 2.24 | 1 | 7 | 22 | 0.34 |

## Pozisyon ve niyet: seviyeden bağımsız geçenler (havuz)

| Seviye | Geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Yalnız pozisyon/niyetle geçen olay | Bunların aşamaları |
|---|---:|---:|---:|---|
| `important` | 5984 | 4248 | 11113 | born 4248, confirmed 17, expired 323, invalidated 1895, realized 2134, strengthened 1179, weakened 1317 |
| `balanced` | 7612 | 3267 | 8864 | born 3267, confirmed 8, expired 265, invalidated 1517, realized 1678, strengthened 961, weakened 1168 |
| `all` | 13056 | 0 | 0 | — |

## Piyasa geneli (kitle yok; M3 karşılaştırması için, havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 1.78 | 1 | 5 | 38 | 0.47 |
| `balanced` doğum | 4.56 | 2 | 15 | 57 | 0.20 |
| `all` doğum | 13.88 | 6 | 45 | 125 | 0.02 |

## Veri kümesi başına (kitle, doğum)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyet geçişi |
|---|---|---:|---:|---:|---:|---:|---:|
| `real-1day` | `important` | 0.96 | 1 | 2 | 8 | 0.42 | 1582 |
| `real-1day` | `balanced` | 1.21 | 1 | 3 | 10 | 0.35 | 1222 |
| `real-1day` | `all` | 2.09 | 2 | 5 | 13 | 0.19 | 0 |
| `real-1hour` | `important` | 7.85 | 7 | 13 | 22 | 0.00 | 2666 |
| `real-1hour` | `balanced` | 10.04 | 10 | 16 | 29 | 0.00 | 2045 |
| `real-1hour` | `all` | 17.18 | 16 | 26 | 44 | 0.00 | 0 |

Gerçek veri `fixtures/local/` altında kalır; burada yalnız toplu sayılar var. Gerçek veride akış ailesi yok (kurumsal akış lisanslı, yerelde yok).
