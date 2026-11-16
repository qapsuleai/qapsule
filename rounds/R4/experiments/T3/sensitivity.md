# R4/T3 (K42 sonrası; R3/S-A aracı) — Hassasiyet benzetimi (sentetik)

Üreten: `cargo run --release -p qp-capsule --example sensitivity -- synthetic` (belirleyici; iki koşu bayt eşit). Motor `EngineConfig::r3()` (tema `kinddir`, sonuç `barrier`, sıralama `confirm`), yedi varsayılan dedektör. Tanımlar `crates/qp-capsule/examples/sensitivity.rs` başında.

- Veri: `synthetic-30x500-s1` (30 enstrüman, 500 işlem günü, sayılan 450), `synthetic-30x500-s2` (30 enstrüman, 500 işlem günü, sayılan 450), `synthetic-30x500-s3` (30 enstrüman, 500 işlem günü, sayılan 450).
- Kitle: izleme listesi 10 enstrüman; pozisyon 3 (SYN01 100, SYN04 -50, SYN12 200); etkin niyet SYN07; evren 11 enstrüman.
- Günlük kapsül = o işlem günü evrende **doğan** ve seviyeden (ya da pozisyon/niyetten) geçen hikâye; ikincil: o gün geçen her hikâye olayı. İlk 50 bar (ısınma) sayılmaz.
- Sistem seviyeyi hiç değiştirmedi (her koşuda kitle önce/sonra karşılaştırıldı, assert): **evet**.

## Seviye başına günlük kapsül (kitle; 1350 gün havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 0.67 | 1 | 2 | 4 | 0.50 |
| `balanced` doğum | 1.42 | 1 | 3 | 7 | 0.23 |
| `all` doğum | 3.23 | 3 | 6 | 11 | 0.04 |
| `important` tüm olaylar | 3.30 | 3 | 7 | 13 | 0.11 |
| `balanced` tüm olaylar | 4.97 | 5 | 9 | 18 | 0.03 |
| `all` tüm olaylar | 9.12 | 9 | 14 | 28 | 0.00 |

## Kullanıcı grubu varsayılanı (seviye seçilmemiş)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `habituated` → `all` doğum | 3.23 | 3 | 6 | 11 | 0.04 |
| `future` → `important` doğum | 0.67 | 1 | 2 | 4 | 0.50 |

## Pozisyon ve niyet: seviyeden bağımsız geçenler (havuz)

| Seviye | Geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Yalnız pozisyon/niyetle geçen olay | Bunların aşamaları |
|---|---:|---:|---:|---|
| `important` | 906 | 0 | 996 | invalidated 617, weakened 379 |
| `balanced` | 1923 | 0 | 772 | invalidated 442, weakened 330 |
| `all` | 4357 | 0 | 0 | — |

## Piyasa geneli (kitle yok; M3 karşılaştırması için, havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 1.79 | 2 | 4 | 8 | 0.16 |
| `balanced` doğum | 3.78 | 4 | 6 | 13 | 0.02 |
| `all` doğum | 8.68 | 8 | 13 | 23 | 0.00 |

`important` satırı M3'ün bütünleşik ölçümüyle (`M3/RESULTS.md`, `confirm`: 1,79 / 2 / 4 / 8, sıfır gün 0,150) aynı tanımdır; aynı sayıları vermesi ürün modüllerinin ölçülenle eş olduğunun bir başka göstergesidir.

## Veri kümesi başına (kitle, doğum)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyet geçişi |
|---|---|---:|---:|---:|---:|---:|---:|
| `synthetic-30x500-s1` | `important` | 0.65 | 0 | 2 | 4 | 0.50 | 0 |
| `synthetic-30x500-s1` | `balanced` | 1.45 | 1 | 3 | 6 | 0.22 | 0 |
| `synthetic-30x500-s1` | `all` | 3.27 | 3 | 6 | 11 | 0.05 | 0 |
| `synthetic-30x500-s2` | `important` | 0.73 | 1 | 2 | 4 | 0.46 | 0 |
| `synthetic-30x500-s2` | `balanced` | 1.44 | 1 | 3 | 5 | 0.24 | 0 |
| `synthetic-30x500-s2` | `all` | 3.22 | 3 | 5 | 9 | 0.04 | 0 |
| `synthetic-30x500-s3` | `important` | 0.63 | 0 | 2 | 4 | 0.52 | 0 |
| `synthetic-30x500-s3` | `balanced` | 1.38 | 1 | 3 | 7 | 0.24 | 0 |
| `synthetic-30x500-s3` | `all` | 3.19 | 3 | 5 | 10 | 0.04 | 0 |
