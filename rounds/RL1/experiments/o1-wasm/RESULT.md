# RL1 / Ö1 — Kabuk WASM paketinde küçültme ölçümü (A70, K93)

Tarih 2026-11-26, Apple M4 Max, rustc 1.92.0, wasm-pack 0.14.0, binaryen `wasm-opt` 117
(wasm-pack'in önbelleğinde zaten vardı: `~/Library/Caches/.wasm-pack/wasm-opt-50385c9e73ccee70`;
yeni araç kurulmadı, ağdan indirme olmadı). Taban: `main` `08bcb4d6`.

## Sonuç

**Uygulandı:** her `wasm-pack --profile wasm` kuruluşunda `wasm-opt -O1
--duplicate-function-elimination --reorder-functions --reorder-locals`
(`crates/qp-wasm/Cargo.toml`, `[package.metadata.wasm-pack.profile.custom]`; betiklerden
`--no-opt` kalktı). Kabuk paketi gzip −%21,9, brotli −%18,0, ham −%26,7. Açılış biraz hızlı,
K52 ve WASM hesap süreleri gürültü içinde, özetler ve altınlar aynı.

## Boyut (kabuk paketi: `--target web --features replica`, `qp_wasm_bg.wasm`)

| Değişke | ham B | gzip -9 B | brotli -q 11 B |
|---|---:|---:|---:|
| V0 önce (verify adım 18 / İ8 ölçüsü: `--no-opt`, isimli) | 9.004.332 | 2.485.078 | 1.699.389 |
| VS önce, yayımlanan (`demo.sh`: `CARGO_PROFILE_WASM_STRIP=true`, `--no-opt`) | 7.089.796 | 2.116.069 | 1.429.139 |
| `wasm-opt -Os` | 6.352.333 | 2.176.831 | 1.508.987 |
| `wasm-opt -Oz` | 6.282.443 | 2.176.995 | — |
| `wasm-opt -O2` / `-O3` | 6.468.671 / 6.415.087 | 2.188.541 / 2.183.827 | — |
| `wasm-opt -O1` | 6.681.070 | 2.064.972 | 1.411.755 |
| `wasm-opt -O1 --reorder-functions` | 6.616.665 | 1.943.577 | 1.396.063 |
| **VW seçilen: `-O1 --duplicate-function-elimination --reorder-functions --reorder-locals`** | **6.598.168** | **1.939.823** | **1.394.034** |
| `opt-level="z"` (isimli, `--no-opt`) | 9.098.735 | 2.409.584 | 1.700.208 |
| `opt-level="z"` + strip | 6.439.501 | 1.889.060 | 1.320.083 |
| `opt-level="z"` + strip + `wasm-opt -Os` | 5.662.731 | 1.983.401 | 1.393.815 |
| `opt-level=3` / `2` + strip | 10.910.968 / 10.691.040 | 2.969.742 / 2.899.859 | — |

VW, V0'a göre: ham −%26,7, gzip −%21,9, brotli −%18,0. Yayımlanan VS'ye göre: −%6,9 / −%8,3 / −%2,5.

Notlar:
- `-Os`/`-Oz`/`-O2`/`-O3` ham dosyayı küçültür ama gzip ve brotli'yi (yayımlanana göre)
  büyütür; kazancın çoğu `--reorder-functions`'tan gelir (sık çağrılanlar başa, sıkıştırma iyileşir).
- `wasm-opt` `-g` olmadan isim bölümünü atar (V0'da `name` 1.911.249 B). `strip = true` ise
  `target_features` bölümünü de siler (wasm-bindgen uyarır); bu yüzden `demo.sh`'taki
  `CARGO_PROFILE_WASM_STRIP=true` kaldırıldı, isimleri artık wasm-opt atıyor. Ölçülen paket =
  yayımlanan paket.
- `opt-level="z"` en küçük gzip'i verir ama WASM hesabını yavaşlatır (aşağıda; bollinger ×2,5)
  ve isimli paketin hamını büyütür (İ8'i aşar). Paket başına `opt-level` (`qp-compute` = s)
  fat LTO altında etkisiz (bayt bayt aynı çıktı). Elendi.
- `codegen-units = 1` ve `lto = "fat"` zaten profilde; wasm32'de `panic` zaten abort.
- Varsayılan modül (verify adım 8, `cargo build --profile wasm`, wasm-pack'siz) değişmedi.

## Açılış ve K52 (headless Chromium, CPU ×4; iki tur, V0 VS VW sırasıyla)

`node tests/startup.mjs <qp> 4 7` (7 soğuk açılış medyanı, ms; `.gz`/`.br` önceden sıkıştırılmış,
WASM `br` ile geldi) ve `node tests/metrics.mjs <qp> 4`:

| Değişke | WASM hazır soğuk | ilk kapsül soğuk | WASM hazır ılık | aktarılan WASM B | K52 tik / sahne / veri geçişi p95 |
|---|---|---|---|---:|---|
| V0 | 140,5 / 138,3 | 304,9 / 304,9 | 87,1 / 86,6 | 1.699.809 | 0,6/31,6/4,7 · 0,7/32,5/4,7 |
| VS | 136,8 / 136,5 | 298,2 / 296,4 | 80,9 / 76,7 | 1.429.559 | 1,0/33,5/5,1 · 0,6/32,4/4,8 |
| VW | 126,1 / 129,2 | 283,8 / 291,6 | 71,7 / 77,1 | 1.394.454 | 0,4/28,3/4,8 · 0,6/32,7/5,4 |

K52 uyarısı yok; kare p95 (drag/idle/scroll/zoom) her değişkede 10,1–10,3 ms.

## WASM hesap (`crates/qp-wasm/bench/sweep_bench.mjs`, Node, medyan ms; özet her değişkede aynı)

| Değişke | sma | ema | rsi | macd | bollinger | `compute_digest` |
|---|---|---|---|---|---|---|
| önce (`--no-opt`) | 12,81 | 11,20 | 19,16 | 10,87 | 11,89 | 7b1e99d40889f83c |
| `opt-level="z"` + strip | 14,56 | 16,50 | 19,90 | 12,02 | 29,64 | 7b1e99d40889f83c |
| VW (iki koşu) | 13,35 / 13,50 | 10,86 / 11,05 | 18,90 / 18,84 | 10,67 / 11,07 | 11,98 / 12,14 | 7b1e99d40889f83c |

VW'de `sma` ≈ +%5 (aynı saatte `--no-opt` 12,51); öbürleri eşit ya da az hızlı.

## İ8 tavanı

İ8 (K93; `rounds/RF1/DECISION.md`): taban 8.789.414 / 2.432.388, en çok +%2,5 → tavan ham
9.009.149, gzip 2.493.198. Önce pay: ham 4.817 B, gzip 8.120 B. Sonra (VW): 6.598.168 /
1.939.823 → eski tavanın 2.410.981 B (ham) / 553.375 B (gzip) altında.

**Önerilen yeni tavan** (aynı kural, yeni değer üstüne +%2,5): ham **6.763.122**, gzip
**1.988.319** (pay 164.954 / 48.496 B). İ8 `scripts/arch-config.json`'da kayıtlı değil (bir
`arch-check` kuralı değil; RF1 denetimi de not etmiş) ve oraya istisna eklemek karar satırı
ister; bu yüzden dosyaya dokunulmadı, sayı ana akışın kararına bırakıldı.

## Paket bileşimi (V0, isim bölümünden; jenerik örnek, tanımlayan crate'e sayılır — yaklaşık)

Kod bölümü 6.132.535 B, 18.630 fonksiyon; veri bölümü 930.739 B. En büyükler: serde %20,3,
alloc %15,9, core %10,5, qp_schema %6,7, tiny_skia %5,8, qp_shellcore %4,3, serde_json %4,1,
regex_automata %3,5, serde_core %3,3, qp_grammar %2,6, qp_designs %2,3, qp_voice %2,0,
regex_syntax %2,0, qp_capsule %1,8, qp_authority %1,7, qp_compute %1,6, aho_corasick %1,4,
qp_ops %1,3, qp_plane %0,9. (`twiggy` kurulu değil; sayım küçük bir bölüm okuyucusuyla.)
serde + alloc + core ≈ %50: büyümenin ana kaynağı yeni crate'lerin serde ve koleksiyon jenerikleri.

## Komutlar

```
export CARGO_TARGET_DIR=<ana ağaç>/target
wasm-pack build crates/qp-wasm --target web --profile wasm [--no-opt] --out-dir <dış> -- --features replica
CARGO_PROFILE_WASM_STRIP=true / CARGO_PROFILE_WASM_OPT_LEVEL=z|2|3 (değişkeler)
wasm-opt <geçişler> --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
  --enable-reference-types --enable-multivalue --enable-mutable-globals in.wasm -o out.wasm
gzip -9 -c | wc -c ; brotli -q 11 -c | wc -c
node shells/web/tests/startup.mjs <qp> 4 7 ; node shells/web/tests/metrics.mjs <qp> 4
node crates/qp-wasm/bench/sweep_bench.mjs <nodejs pkg>
```

## Doğrulama

`QP_VERIFY_BROWSER=1 scripts/verify.sh` (ortak `target/`): 21/21 PASS, 883 s. Adım 8 varsayılan modül
ham 2.354.868 / gzip 773.639 (değişmedi); adım 12 arm64 = x86_64 = wasm32 `7b1e99d40889f83c`
(wasm32 artık wasm-opt'lu paketle); adım 15 yansı wasm32 = yerli; adım 18 altınlar ve sahneler
aynı, `sizes.json` {6.598.168, 1.939.823}.

## Açık nokta

- `wasm-pack` binaryen'i ilk kuruluşta GitHub'dan indirir (sürüm wasm-pack'e bağlı). CI'da ağ
  var; çevrimdışı ve önbelleksiz makinede verify'ın WASM adımları düşer. Bu makinede önbellek
  vardı.
