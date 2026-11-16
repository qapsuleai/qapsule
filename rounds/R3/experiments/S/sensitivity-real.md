# R3/S-A — Hassasiyet benzetimi (gerçek yerel veri, yalnız toplu)

Üreten: `cargo run --release -p qp-capsule --example sensitivity -- real` (belirleyici; iki koşu bayt eşit). Motor `EngineConfig::r3()` (tema `kinddir`, sonuç `barrier`, sıralama `confirm`), yedi varsayılan dedektör. Tanımlar `crates/qp-capsule/examples/sensitivity.rs` başında.

- Veri: `real-1day` (6 enstrüman, 2219 işlem günü, sayılan 2169), `real-1hour` (6 enstrüman, 501 işlem günü, sayılan 495).
- Kitle: izleme listesi 6 enstrüman; pozisyon 3 (AEFES 100, EREGL -50, ISCTR 100); etkin niyet SISE; evren 6 enstrüman.
- Günlük kapsül = o işlem günü evrende **doğan** ve seviyeden (ya da pozisyon/niyetten) geçen hikâye; ikincil: o gün geçen her hikâye olayı. İlk 50 bar (ısınma) sayılmaz.
- Sistem seviyeyi hiç değiştirmedi (her koşuda kitle önce/sonra karşılaştırıldı, assert): **evet**.

## Seviye başına günlük kapsül (kitle; 2664 gün havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 1.88 | 1 | 6 | 21 | 0.41 |
| `balanced` doğum | 2.09 | 1 | 7 | 22 | 0.39 |
| `all` doğum | 2.71 | 1 | 8 | 27 | 0.32 |
| `important` tüm olaylar | 4.96 | 2 | 15 | 50 | 0.17 |
| `balanced` tüm olaylar | 5.44 | 2 | 17 | 54 | 0.15 |
| `all` tüm olaylar | 7.15 | 3 | 22 | 65 | 0.09 |

## Kullanıcı grubu varsayılanı (seviye seçilmemiş)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `habituated` → `all` doğum | 2.71 | 1 | 8 | 27 | 0.32 |
| `future` → `important` doğum | 1.88 | 1 | 6 | 21 | 0.41 |

## Pozisyon ve niyet: seviyeden bağımsız geçenler (havuz)

| Seviye | Geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Yalnız pozisyon/niyetle geçen olay | Bunların aşamaları |
|---|---:|---:|---:|---|
| `important` | 5006 | 4220 | 11040 | born 4220, confirmed 15, expired 341, invalidated 1874, realized 2103, strengthened 1199, weakened 1288 |
| `balanced` | 5575 | 3151 | 8671 | born 3151, confirmed 8, expired 286, invalidated 1482, realized 1603, strengthened 980, weakened 1161 |
| `all` | 7219 | 0 | 0 | — |

## Piyasa geneli (kitle yok; M3 karşılaştırması için, havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 0.30 | 0 | 1 | 10 | 0.82 |
| `balanced` doğum | 0.91 | 0 | 3 | 12 | 0.61 |
| `all` doğum | 2.71 | 1 | 8 | 27 | 0.32 |

## Veri kümesi başına (kitle, doğum)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyet geçişi |
|---|---|---:|---:|---:|---:|---:|---:|
| `real-1day` | `important` | 0.83 | 1 | 2 | 6 | 0.50 | 1598 |
| `real-1day` | `balanced` | 0.93 | 1 | 2 | 7 | 0.47 | 1187 |
| `real-1day` | `all` | 1.22 | 1 | 3 | 9 | 0.39 | 0 |
| `real-1hour` | `important` | 6.48 | 6 | 11 | 21 | 0.01 | 2622 |
| `real-1hour` | `balanced` | 7.18 | 7 | 12 | 22 | 0.01 | 1964 |
| `real-1hour` | `all` | 9.22 | 9 | 15 | 27 | 0.00 | 0 |

Gerçek veri `fixtures/local/` altında kalır; burada yalnız toplu sayılar var. Gerçek veride akış ailesi yok (kurumsal akış lisanslı, yerelde yok).
