# RL1 / C — Başarım: K17, K52, tik maliyeti, paket boyutu (kabul 17, VERIFY 6, 19)

Apple M4 Max, macOS, rustc 1.92, node v24.3.0, headless Chromium 151; 2026-11-26; kod
`rl1/close-c` (`main` `64e30256` + bu görev). Ham çıktılar burada özetlendi (scratchpad'den
silindi). Sayılar gürültülü; karşılaştırma son kayıtlı ölçümle göz kararı.

## (a) K17 — tarama hedefi (dizüstü: yerel < 50 ms, WASM < 150 ms)

`scripts/refactor/perf.sh` (yerel, `sweep_bench --reps 15`) ve `wasm-sweep-bench.sh`'ın iki adımı
(wasm-pack nodejs + `crates/qp-wasm/bench/sweep_bench.mjs`), 10.000 bar × 1.000 varyasyon,
medyan ms. Özetler aynı (`compute_digest` `7b1e99d40889f83c`).

| Hesap | yerel şimdi | yerel RF1 (`rounds/RF1/experiments/baseline/perf.txt`) | WASM şimdi (2 koşu) | WASM Ö1 VW (`o1-wasm/RESULT.md`) |
|---|---:|---:|---:|---:|
| sma | 4,58 | 4,39 | 13,47 / 13,50 | 13,35 / 13,50 |
| ema | 4,05 | 4,05 | 11,28 / 10,86 | 10,86 / 11,05 |
| rsi | 9,49 | 9,59 | 19,95 / 19,54 | 18,90 / 18,84 |
| macd | 8,20 | 8,63 | 10,76 / 11,08 | 10,67 / 11,07 |
| bollinger | 8,88 | 9,71 | 11,91 / 11,76 | 11,98 / 12,14 |

**K17 içinde** (en kötü yerel 9,5 ms ≤ 50; WASM 20 ms ≤ 150); farklar gürültü düzeyinde.

### Program yolu (yeni ölçüm)

`cargo run --release -p qp-lang --example program_sweep` (yeni örnek): PR §14 `sma` programı,
aynı kapanışlar (`bench_closes`) ve aynı ızgara (`bench_grid("sma")`, period 2..1001), varyasyon
başına bir `Executor::run_closed`, tek çekirdek, 7 koşu (WASM 5) medyanı. Özet `d9a393c2f397ae9e`
= `sweep_bench`'in `sma` özeti (PR §14 bit eşitliği 10 milyon değerde, iki hedefte).

| Hedef | yalnız çalıştırıcı (min / med / max ms) | denetim + çalıştırıcı (med ms) | K17 |
|---|---|---:|---|
| yerel aarch64 | 257,0 / 264,3 / 282,4 | 275,7 | **aşıyor** (≈ 5,3×; tarama çekirdeği 4,6 ms) |
| wasm32-wasip1 (node:wasi) | 387,0 / 391,7 / 392,5 | 401,3 | **aşıyor** (≈ 2,6×) |

Program yolu K17'nin tarama hedefini **tutmuyor**: çalıştırıcı varyasyon başına ≈ 26 ns/bar
(yerel), tarama çekirdeği ≈ 0,46 ns/bar (şerit/SoA, ortak alt hesap, K19). K17 tarama API'sinin
(`qp_compute::sweep`) hedefidir; programlar bu round taramaya bağlanmadı (parametre taraması
`dene` değil, R7/RS1 işi). Canlı yol ve `dene` bütçeleri (tik µs düzeyi, aşağıda; `try` en çok
20.000 bar) bundan etkilenmez. Öneri: program taramasında §14 eşdeğeri düğümleri `sweep_each`
çekirdeğine indirmek (aynı bitler, kanıtı yukarıdaki özet eşitliği).

## (b) K52 — tarayıcı kalibrasyonu, program katmanlı ve katmansız

`shells/web/tests/metrics.mjs` (RF1/Ö1 yöntemi, CPU ×4, telefon sayfası) ve onun program
eklentili kopyası `rounds/RL1/experiments/perf/k52-program.mjs`: `qp demo --mcp`, ajan
(`claude`) her enstrümana bir PR §12.5 kırılım programı önerir (`program_add`), sayfadaki kişi
kabul eder (`window.qp.decide`; 30/30 kabul, belgede 30 program serisi), ölçüm sayaçları
sıfırlanır, sonra metrics.mjs'nin aynı oynatma/kaydırma/açma/sürükleme akışı. İki kip:
varsayılan izleyici sayfası (metrics.mjs'nin ölçtüğü) ve tam yansı (`?mode=replica`; yansı her
seriyi, program serilerini de, her veri çerçevesinde kendi hesaplar). p95 ms; her satır 2 koşu.

| Kip | Program | tik p95 (ort.) | sahne p95 | veri geçişi p95 | yüz p95 | sürükleme uzun kare | K52 uyarısı |
|---|---:|---|---|---|---|---|---|
| metrics.mjs (taban, sıfırlamasız) | 0 | 0,8 (0,12) | 28,1 | 4,7 | 1,7 | 0 | yok |
| izleyici | 0 | 0,4 / 0,4 (0,11) | 26,7 / 29,8 | 4,4 / 4,6 | 1,6 | 0 | yok |
| izleyici | 30 | 0,4 / 0,3 (0,10) | **45,0 / 43,3** | 4,5 / 4,4 | 1,6 | 0 | yok |
| yansı | 0 | 1,3 / 1,3 (0,78 / 0,74) | 12,4 / 12,5 | 5,8 / 6,0 | 2,1–2,2 | 0 | yok |
| yansı | 30 | 1,3 / 1,3 (0,87 / 0,80) | 13,4 / 12,2 | 6,0 / 5,6 | 2,1–2,2 | 0 | yok |

- **Tik ve veri geçişi:** 30 program serisi tik p95'i değiştirmiyor; yansı kipinde tik ortalaması
  ≈ +0,07–0,09 ms (≈ +%10), K52 bütçesinin (tik + boyama p95 ≤ 8 ms) çok altında. Kare p95 her
  koşuda 10,1–10,3 ms, uzun kare yok.
- **İzleyici sahnesi:** sahne p95'i (10 örneğin en büyüğü) 27–30 → 43–45 ms; p50 aynı (1,9–2,1),
  `scene_build` en büyüğü 20–22 → 36–38 ms, p95'i aynı (1,3–1,4): tek bir kuruluş uzuyor.
  Açıklama (ölçülmedi, olası): açılan kapsülün enstrümanında program serisi olduğundan
  izleyicinin açık dilimi onu ilk kuruluşta geçmiş üstünden baştan hesaplıyor (tek seferlik). Tabanda
  da sahne p95'i bu düzenekte 8 ms'nin üstünde (Ö1: 28–33 ms) — K52 sahne hedefi gerçek telefonda
  ölçülür (R4/RP3 kalibrasyonu); burada yalnız oran: ilk açılış ≈ +15 ms. Öneri: açık dilimde
  program serisini ilk boyamadan sonra hesaplamak ya da yansının hesabını taşımak (R7 notu).
- Taban satırı RF1/Ö1 ile aynı yöntem (metrics.mjs): tik 0,8 / sahne 28,1 / veri 4,7 (Ö1 VW:
  0,4–0,6 / 28,3–32,7 / 4,8–5,4) — gürültü içinde.

## (c) Program değerlendirmesinin tik maliyeti, WASM

`qp-plane` `tick_bench` (TICK.md'nin aynısı) wasm32-wasip1'e derlenip node:wasi'de koşuldu; 500
barlık saatlik seri, `Cache::apply_delta`, 2000 tikin medyanı, ns/tik. Yerel sütunlar bu koşudan
(TICK.md ile gürültü içinde aynı).

| Seri | Adet | WASM düzeltme | WASM yeni bar | yerel düzeltme | yerel yeni bar |
|---|---:|---:|---:|---:|---:|
| `add_study` sma | 1 / 8 / 32 | 125 / 292 / 458 | 125 / 333 / 625 | 41 / 83 / 291 | 42 / 125 / 375 |
| program sma | 1 / 8 / 32 | 84 / 459 / 959 | 209 / 1.084 / 2.875 | 42 / 167 / 625 | 83 / 458 / 1.708 |
| program kırılım (olay) | 1 / 8 / 32 | 83 / 417 / 1.542 | 167 / 1.041 / 4.083 | 42 / 292 / 1.083 | 84 / 709 / 2.708 |

WASM'da 32 örnekte tik başına en çok ≈ 4,1 µs (yerelin ≈ 1,5×): K52 tik bütçesinin (≤ 8 ms)
binde birinin altında; (b)'deki tarayıcı ölçümü de bunu doğruluyor.

## (d) Kabuk paketi boyutu (verify adım 18 yolu: wasm-pack web, `--features replica`, wasm-opt)

| Ölçüm | ham B | gzip -9 B |
|---|---:|---:|
| **şimdi** (`perf.sh`, `qp_wasm_bg.wasm`) | **7.167.360** | **2.078.388** |
| D6 (b): `qp-lang` bağlanınca (PA kapısı) | 7.154.035 | 2.076.328 |
| **İ8 tavanı (D6)** | **7.332.886** | **2.128.236** |
| Ö1 öncesi (9.004.332 / 2.485.078) | | |

**İ8 içinde:** tavanın %2,3 / %2,3 altında; PA kapısından bu yana +13.325 / +2.060 B (PB'nin
protokol tipleri). Varsayılan modül (verify adım 8, `--profile wasm`, wasm-pack'siz): ham
2.602.756, gzip 817.223 (İ8 ölçüsü değil; RF1 2.345.821 / 769.836).
