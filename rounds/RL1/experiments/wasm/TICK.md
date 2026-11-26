# RL1 / PA — program serisinin tik maliyeti (PS §1.3, kabul 17)

`cargo run --release -p qp-plane --example tick_bench` (Apple M4 Max, yerel, 2026-11-26).
500 barlık saatlik seri, `Cache::apply_delta` süresi, 2000 tikin medyanı (ns; `Instant`
çözünürlüğü ≈ 42 ns). "Düzeltme": son (canlı) barın düzeltilmesi (`update_last`); "yeni bar":
canlı bar kapanır, yeni canlı bar (`push`). Program sma = PR §14 `add_study` programı
(`sma(period)`), kırılım = PR §12.5 olay programı.

| Seri | Adet | Düzeltme ns/tik | Yeni bar ns/tik |
|---|---:|---:|---:|
| `add_study` sma | 1 | 42 | 42 |
| `add_study` sma | 8 | 84 | 125 |
| `add_study` sma | 32 | 292 | 375 |
| program sma | 1 | 42 | 83 |
| program sma | 8 | 167 | 500 |
| program sma | 32 | 709 | 1.834 |
| program kırılım (olay) | 1 | 42 | 83 |
| program kırılım (olay) | 8 | 291 | 667 |
| program kırılım (olay) | 32 | 1.125 | 2.708 |

- Program serisi eşdeğer `add_study` serisinin ≈ 2–5 katı (çalıştırıcı bütün bar alanlarını
  alır, canlı barda geçici değer hesaplar, yeni barda önceki canlı barı kapatır ve kapanan barın
  değerini yeniden yazar). 32 örnekte tik başına en çok ≈ 2,7 µs: K17 (tarama hedefleri, onlarca
  ms) ve K52 (tik p95 ≲ 1 ms) bütçesinin çok altında. Tik başına baştan koşu yok (sınama
  `incremental_path_equals_rebuild_bit_for_bit` canlı düzeltme ve yeni barda yuva düşmediğini
  `Cache::gen` ile doğrular).
- WASM'da tik ölçümü bu işte yapılmadı (C görevi: K17/K52/WASM ölçümleri).
