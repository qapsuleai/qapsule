# RL1 / PA — D1 şart 1: qp-lang qp-plane'e bağlanınca kabuk paketi (İ8 kapısı)

Tarih 2026-11-26, Apple M4 Max, aynı araçlar (wasm-pack 0.14.0, `--profile wasm` = `wasm-opt -O1
--duplicate-function-elimination --reorder-functions --reorder-locals`). Kuruluş verify adım 18 /
`demo.sh` ile aynı: `wasm-pack build crates/qp-wasm --target web --profile wasm -- --features replica`,
ölçülen `qp_wasm_bg.wasm` (ham, `gzip -9`). Çıktı dizini scratchpad (shells/web/pkg'ye yazılmadı).

| Kuruluş | ham B | gzip -9 B | İ8 tavanına göre (ham / gzip) |
|---|---:|---:|---|
| İ8 tavanı (D4) | 6.763.122 | 1.988.319 | — |
| `main` dc6fe4bd (qp-lang bağlı değil) | 6.935.682 | 2.005.537 | **+172.560 / +17.218 (aşıyor)** |
| `rl1/lang-exec` 473f4b0f (qp-lang bağlı değil) | 6.935.661 | 2.005.483 | **+172.539 / +17.164 (aşıyor)** |
| `rl1/plane-prog` (qp-plane → qp-lang, program serisi hesabı) | 7.154.035 | 2.076.328 | **+390.913 / +88.009 (aşıyor)** |

- Bağlamanın kendi payı: ham +218.374 B (+%3,1), gzip +70.845 B (+%3,5).
- Tavan bağlamadan önce de aşılmış: D4 ölçümünden (`5f16ceff`, 6.598.168 / 1.939.823) bu yana `main`
  ham +337.514, gzip +65.714 büyümüş. Aradaki paket içi değişiklikler: I1 (qp-ir tipleri ve
  protokol eklemeleri, `qp-schema`), I2 (`Stdev`, `PercentRank` çekirdekleri, `qp-compute`).
  Hangisinin ne kadar katkı verdiği ayrıca ölçülmedi.
- D1 şart 1 gereği kodlama durduruldu; karar yürüten oturumda.
