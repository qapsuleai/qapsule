# R3/S-A — Hassasiyet benzetimi (sentetik)

Üreten: `cargo run --release -p qp-capsule --example sensitivity -- synthetic` (belirleyici; iki koşu bayt eşit). Motor `EngineConfig::r3()` (tema `kinddir`, sonuç `barrier`, sıralama `confirm`), yedi varsayılan dedektör. Tanımlar `crates/qp-capsule/examples/sensitivity.rs` başında.

- Veri: `synthetic-30x500-s1` (30 enstrüman, 500 işlem günü, sayılan 450), `synthetic-30x500-s2` (30 enstrüman, 500 işlem günü, sayılan 450), `synthetic-30x500-s3` (30 enstrüman, 500 işlem günü, sayılan 450).
- Kitle: izleme listesi 10 enstrüman; pozisyon 3 (SYN01 100, SYN04 -50, SYN12 200); etkin niyet SYN07; evren 11 enstrüman.
- Günlük kapsül = o işlem günü evrende **doğan** ve seviyeden (ya da pozisyon/niyetten) geçen hikâye; ikincil: o gün geçen her hikâye olayı. İlk 50 bar (ısınma) sayılmaz.
- Sistem seviyeyi hiç değiştirmedi (her koşuda kitle önce/sonra karşılaştırıldı, assert): **evet**.

## Seviye başına günlük kapsül (kitle; 1350 gün havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 1.59 | 1 | 3 | 7 | 0.20 |
| `balanced` doğum | 2.06 | 2 | 4 | 9 | 0.13 |
| `all` doğum | 3.23 | 3 | 6 | 11 | 0.04 |
| `important` tüm olaylar | 4.94 | 5 | 9 | 18 | 0.03 |
| `balanced` tüm olaylar | 6.12 | 6 | 10 | 21 | 0.01 |
| `all` tüm olaylar | 9.12 | 9 | 14 | 28 | 0.00 |

## Kullanıcı grubu varsayılanı (seviye seçilmemiş)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `habituated` → `all` doğum | 3.23 | 3 | 6 | 11 | 0.04 |
| `future` → `important` doğum | 1.59 | 1 | 3 | 7 | 0.20 |

## Pozisyon ve niyet: seviyeden bağımsız geçenler (havuz)

| Seviye | Geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Yalnız pozisyon/niyetle geçen olay | Bunların aşamaları |
|---|---:|---:|---:|---|
| `important` | 2143 | 1229 | 3177 | born 1229, confirmed 19, expired 97, invalidated 617, realized 515, strengthened 319, weakened 381 |
| `balanced` | 2779 | 849 | 2267 | born 849, confirmed 12, expired 74, invalidated 441, realized 364, strengthened 203, weakened 324 |
| `all` | 4357 | 0 | 0 | — |

## Piyasa geneli (kitle yok; M3 karşılaştırması için, havuz)

| | Mean | Median | p90 | Max | Zero-day share |
|---|---:|---:|---:|---:|---:|
| `important` doğum | 1.79 | 2 | 4 | 8 | 0.15 |
| `balanced` doğum | 3.78 | 4 | 6 | 14 | 0.03 |
| `all` doğum | 8.68 | 8 | 13 | 23 | 0.00 |

`important` satırı M3'ün bütünleşik ölçümüyle (`M3/RESULTS.md`, `confirm`: 1,79 / 2 / 4 / 8, sıfır gün 0,150) aynı tanımdır; aynı sayıları vermesi ürün modüllerinin ölçülenle eş olduğunun bir başka göstergesidir.

## Veri kümesi başına (kitle, doğum)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyet geçişi |
|---|---|---:|---:|---:|---:|---:|---:|
| `synthetic-30x500-s1` | `important` | 1.57 | 1 | 3 | 7 | 0.22 | 412 |
| `synthetic-30x500-s1` | `balanced` | 2.06 | 2 | 4 | 9 | 0.13 | 276 |
| `synthetic-30x500-s1` | `all` | 3.27 | 3 | 6 | 11 | 0.05 | 0 |
| `synthetic-30x500-s2` | `important` | 1.64 | 1 | 3 | 7 | 0.19 | 405 |
| `synthetic-30x500-s2` | `balanced` | 2.05 | 2 | 4 | 7 | 0.13 | 283 |
| `synthetic-30x500-s2` | `all` | 3.22 | 3 | 5 | 9 | 0.04 | 0 |
| `synthetic-30x500-s3` | `important` | 1.56 | 1 | 3 | 6 | 0.19 | 412 |
| `synthetic-30x500-s3` | `balanced` | 2.06 | 2 | 4 | 8 | 0.11 | 290 |
| `synthetic-30x500-s3` | `all` | 3.19 | 3 | 5 | 10 | 0.04 | 0 |
