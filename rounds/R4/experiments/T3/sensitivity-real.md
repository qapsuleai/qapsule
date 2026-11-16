# R4/T3 (K42 sonrası; R3/S-A aracı) — Hassasiyet benzetimi (gerçek yerel veri, yalnız toplu)

Üreten: `cargo run --release -p qp-capsule --example sensitivity -- real` (belirleyici; iki koşu bayt eşit). Motor `EngineConfig::r3()` (tema `kinddir`, sonuç `barrier`, sıralama `confirm`), yedi varsayılan dedektör. Tanımlar `crates/qp-capsule/examples/sensitivity.rs` başında.

- Veri: `real-1day` (31 enstrüman, 2220 işlem günü, sayılan 2170), `real-1hour` (31 enstrüman, 502 işlem günü, sayılan 496).
- Kitle: izleme listesi 10 enstrüman; pozisyon 3 (AEFES 100, ASELS -50, EREGL 200); etkin niyet BRSAN; evren 11 enstrüman.
- Günlük kapsül = o işlem günü evrende **doğan** ve seviyeden (ya da pozisyon/niyetten) geçen hikâye; ikincil: o gün geçen her hikâye olayı. İlk 50 bar (ısınma) sayılmaz.
- Sistem seviyeyi hiç değiştirmedi (her koşuda kitle önce/sonra karşılaştırıldı, assert): **evet**.

## Seviye başına günlük kapsül (kitle; 2666 gün havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 0.73 | 0 | 2 | 11 | 0.63 |
| `balanced` doğum | 1.63 | 1 | 5 | 18 | 0.45 |
| `all` doğum | 4.90 | 2 | 16 | 44 | 0.15 |
| `important` tüm olaylar | 3.16 | 1 | 10 | 41 | 0.30 |
| `balanced` tüm olaylar | 5.23 | 2 | 16 | 61 | 0.15 |
| `all` tüm olaylar | 12.99 | 6 | 43 | 115 | 0.02 |

## Kullanıcı grubu varsayılanı (seviye seçilmemiş)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `habituated` → `all` doğum | 4.90 | 2 | 16 | 44 | 0.15 |
| `future` → `important` doğum | 0.73 | 0 | 2 | 11 | 0.63 |

## Pozisyon ve niyet: seviyeden bağımsız geçenler (havuz)

| Seviye | Geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Yalnız pozisyon/niyetle geçen olay | Bunların aşamaları |
|---|---:|---:|---:|---|
| `important` | 1934 | 0 | 3206 | invalidated 1890, weakened 1316 |
| `balanced` | 4352 | 0 | 2623 | invalidated 1507, weakened 1116 |
| `all` | 13056 | 0 | 0 | — |

## Piyasa geneli (kitle yok; M3 karşılaştırması için, havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 1.97 | 1 | 6 | 38 | 0.41 |
| `balanced` doğum | 4.62 | 2 | 15 | 60 | 0.21 |
| `all` doğum | 13.88 | 6 | 45 | 125 | 0.02 |

## Veri kümesi başına (kitle, doğum)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyet geçişi |
|---|---|---:|---:|---:|---:|---:|---:|
| `real-1day` | `important` | 0.31 | 0 | 1 | 5 | 0.74 | 0 |
| `real-1day` | `balanced` | 0.65 | 0 | 2 | 6 | 0.55 | 0 |
| `real-1day` | `all` | 2.09 | 2 | 5 | 13 | 0.19 | 0 |
| `real-1hour` | `important` | 2.56 | 2 | 5 | 11 | 0.11 | 0 |
| `real-1hour` | `balanced` | 5.91 | 6 | 10 | 18 | 0.01 | 0 |
| `real-1hour` | `all` | 17.18 | 16 | 26 | 44 | 0.00 | 0 |

Gerçek veri `fixtures/local/` altında kalır; burada yalnız toplu sayılar var. Gerçek veride akış ailesi yok (kurumsal akış lisanslı, yerelde yok).
