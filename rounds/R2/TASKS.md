# R2 — Alt Görevler

T1 önce. T2 ardından. T3, T4, T5, T6 T2'den sonra paralel yürüyebilir. T7 bağımsız.

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Gramer tasarımı ve şema: tipler, JSON Schema, `docs/GRAMMAR.md`, sözlük eklemeleri. Altı görünümün JSON taslağı tasarımı sınamak için önce yazılır | `crates/qp-schema/`, `docs/`, `fixtures/views/` | — |
| T2 | `qp-grammar`: Görünüm + veri → sahne tarifi (çizimden bağımsız ara biçim). Doğrulama ve hata iletileri | `crates/qp-grammar/` | T1 |
| T3 | Referans çizici sahne tarifinden çizer; R0 sahnesi gramer yoluna taşınır; altı altın görsel | `crates/qp-render/`, `fixtures/golden/` | T2 |
| T4 | Görüş alanı ve LOD; 100k bar ölçümü; eski viewport/LOD matematiğinin değerlendirmesi | `crates/qp-grammar/` (viewport, lod) | T2 |
| T5 | İsabet testi ve `qp explain`; `qp render` görünüm dosyası kabul eder | `crates/qp-grammar/` (hit), `crates/qp-cli/` | T2 |
| T6 | GPU çizim yolu (K12: vello + WebGPU, MSAA), tolerans karşılaştırması, web gösterimi | `crates/qp-render-gpu/` ya da özellik bayrağı (karar raporda), `crates/qp-wasm/`, `shells/web/` | T2 |
| T7 | R1 denetim bulguları | `crates/qp-compute/` | — |
| T8 | Denetim: bağımsız alt ajan, temiz kopya; altın görselleri gözle inceler | `rounds/R2/AUDIT.md` | hepsi |

## Notlar

- **T1 en önemli görev.** Gramerin amacı bugün bildiğimiz grafikleri çizmek değil, ajanın
  bugün aklımıza gelmeyenleri kurabilmesi. Var olan gramerleri (Vega-Lite, grafik grameri
  literatürü) incele, ama kopyalama; finans zaman serisinin ihtiyaçları (seans boşlukları,
  canlı son bar, çok panel, `null` ısınma) birinci sınıf olmalı. Tasarım kararlarını ve
  elediğin seçenekleri `docs/GRAMMAR.md` sonuna yaz.
- **Sahne tarifi** çizicilerin ortak girdisidir: hem referans çizici hem GPU aynısını
  okur. İsabet testi de sahne tarifinden çalışır; böylece "ekranda ne var" sorusunun tek
  bir doğru cevabı olur.
- **Altın görseller:** var olanlar güncellenmez. Yeni beş görünümün altın görselleri
  eklenir; doğrulukları denetçinin göz incelemesi ve sayısal nokta kontrolüyle sağlanır.
- **GPU:** K12'deki ölçümlere dayan, ama sürümler değişmiş olabilir; kütüphane
  belgelerine bak ve küçük denemeyle doğrula. CI koşucusunda GPU yoksa yazılım
  rasterleştiriciyle çalıştırmayı dene; olmuyorsa açık SKIP.
- **Hesap:** görünümlerdeki RSI, MACD, Bollinger `qp-compute`'tan gelir; çizicide ya da
  gramerde hesap kodu olmaz.
- **GitHub:** `CLAUDE.md` kuralları geçerli (`scripts/qgh`, `scripts/preflight.sh`).
