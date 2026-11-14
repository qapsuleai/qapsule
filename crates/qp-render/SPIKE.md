# R0/T3 — Çizim yığını denemesi (A1 için kanıt)

Durum: deneme tamamlandı. Bu dosya `DECISIONS.md` A1'i kapatmak için kanıt sunar; kararı
ana akış yazar.

## Özet

- **Referans çizici: tiny-skia 0.12 + fontdue 0.9 + kendi tamsayı metin birleştiricimiz +
  gömülü OFL font.** Aynı kod; aarch64 yerel, x86_64 (Rosetta, SSE2 ve AVX2/FMA kod üretimi),
  wasm32 (skaler ve simd128) üzerinde **bit bit aynı** piksel ve **bayt bayt aynı PNG**
  üretti. Tarayıcıda canvas'tan geri okunan pikseller ve canvas ekran görüntüsü altın görselle
  **0 piksel** farklı.
- **vello_cpu 0.4**: 100k bar'da 2,3x daha hızlı ama mimariler arası kayma var (stres
  sahnesinde 8 piksel, ±1). Altın görsel referansı olamaz.
- **vello 0.12 (wgpu 30, GPU)**: Metal'de ve tarayıcıda WebGPU'da çalışıyor, en hızlısı.
  Ama **Area AA çalıştırmadan çalıştırmaya deterministik değil** (31 çalıştırmada 3 farklı
  çıktı). MSAA8/16 deterministik ve yerel Metal ile tarayıcı WebGPU arasında 0 fark. Soğuk
  boru hattı derlemesi ~1,5 sn.
- **A1 önerisi:** Referans/headless/altın yol = tiny-skia + fontdue (bu crate). Tarayıcıda
  aynı kod wasm olarak varsayılan ve yedek yol. Etkileşimli GPU yolu (vello/WebGPU, MSAA)
  R2'de etkileşim ölçümleriyle eklenir; GPU çıktısı altın görsele bayt eşitliğiyle değil,
  referansa karşı toleransla test edilir. Zincir: **WebGPU (vello) → CPU wasm (referans)**.
  WebGL2 katmanı için kanıt yok: CPU wasm 1200x600 kareyi ~3 ms'de çiziyor.

## Ortam

Apple M4 Max, macOS (Darwin 27), Rust 1.92.0, wasm-pack 0.14.0, wasm-bindgen 0.2.130,
Playwright 1.62.0 (Chromium 151.0.7922.34; headless shell ve WebGPU için tam Chromium
`--enable-unsafe-webgpu`), binaryen 132 (yalnızca boyut ölçümü). x86_64 satırları Rosetta 2
altında çalıştı (gerçek x86 donanımı değil; bkz. Riskler).

## Denenen seçenekler

| Yığın | Sürüm | Metin | Nerede |
|-------|-------|-------|--------|
| A. tiny-skia + fontdue (referans) | tiny-skia 0.12.0 (`default-features=false, std`; `simd` özelliği isteğe bağlı), fontdue 0.9.5 (`default-features=false`), png 0.18.1 | fontdue kapsama bitmap'i + tamsayı source-over birleştirici, kalem tam piksele yuvarlanır | `src/cpu.rs` |
| B. vello_cpu | 0.4.0 (u8 ve f32 boru hatları, isteğe bağlı çok iş parçacığı) | glifo/skrifa outline rasterleştirici, glif konumları referansla aynı | `src/spikes/vello_cpu.rs` (özellik `spike-vello-cpu[-mt]`) |
| C. vello (GPU) | 0.12.0, wgpu 30.0.2 (Metal yerel, WebGPU tarayıcı) | GPU outline, glif konumları referansla aynı | `src/spikes/vello_gpu.rs` (özellik `spike-vello-gpu`) |

Üç yığın da aynı `Scene`'i (`src/scene.rs`) çizer: yerleşim `f64` ile bir kez hesaplanır,
farklar yalnızca rasterleştiriciden gelir. İki sahne var: `sample` (200 bar, 1200x600) ve
`stress` (kesirli koordinat ve genişlikte AA çizgiler, yarı saydam dolgular, 7–40 px arası
kesirli boyutlarda metin, 600x300; altın görsel `fixtures/golden/stress.png`).

## Ölçümler

Süreler ms, medyan. "100k bar" = 100.000 deterministik sentetik bar, 1200x600 (LOD yok).

| Ölçüm | A. tiny-skia+fontdue | B. vello_cpu (u8, 1T) | C. vello GPU (Metal / WebGPU) |
|-------|---------------------|------------------------|-------------------------------|
| Yerel derleme / wasm32 derleme | evet / evet | evet / evet | evet / evet (WebGPU) |
| İlk kare, yerel (font + JSON + sahne + raster [+PNG]) | 10–13 | 5–7 (PNG yok) | cihaz+boru hattı: soğuk 1468, sıcak 37–56; sonra ilk kare 18–21 |
| Kararlı kare, yerel (sample) | 0,75 | 0,76 (MT 0,70) | 1,2–1,6 (geri okuma dahil) |
| PNG kodlama | 3,7 | — | — |
| 100k bar, yerel aarch64 | 38 raster + 1 sahne | 16,7 (MT 15,3) | 8–9 |
| 100k bar, x86_64 (Rosetta) | 67 | 34 (MT 26) | ölçülmedi |
| Tarayıcı: wasm init / ilk kare / ikinci kare | 6 / 13 / 3,2 | — / 11,5 / 3,1 | — / soğuk 1567, sıcak 49 / 3,4 |
| Tarayıcı: 100k bar | 80 | 77 | 37 |
| Tekrar çalıştırma aynı mı | evet | evet (1T ve MT aynı) | **Area: hayır** (3/31, en çok 3 px ±1); MSAA8/16: evet |
| aarch64 ↔ x86_64 | **0 px** (skaler, simd, AVX2/FMA) | sample 0 px; **stress 8 px ±1** (f32: 4 px) | ölçülmedi |
| Yerel ↔ tarayıcı | **0 px**, PNG baytları aynı (skaler ve simd128) | sample 0 px; **stress 8 px ±1** | Area stress 2 px ±1; MSAA8/16 0 px |
| Canvas ekran görüntüsü ↔ altın | **0 px** | — | — |
| Referansa göre fark (sample) | — | 6708 px, en çok 60, ort. 0,020 | 6906–6931 px, en çok 63–85, ort. 0,02–0,05 |

WASM boyutu (`--profile wasm`: opt-level s, fat LTO; bayt):

| Paket | ham | gzip -9 | wasm-opt -Oz | -Oz + gzip |
|-------|-----|---------|--------------|------------|
| A (varsayılan `qp-wasm`) | 959.199 | 362.710 | 829.745 | 330.987 |
| A + simd128 | 871.891 | 329.522 | 750.043 | 297.692 |
| A + B | 2.205.925 | 806.050 | 1.783.402 | 722.661 |
| A + C | 2.625.062 | 896.316 | 2.133.475 | 795.994 |
| A + B + C | 3.241.415 | 1.113.038 | 2.620.817 | 988.556 |

A'nın veri bölümü ~210 KB; bunun 135 KB'ı gömülü font. JS yapıştırıcısı A için ~10 KB,
C ile ~48 KB.

## Determinizm bulguları

1. **Referans yol tam deterministik.** Aynı FNV özetleri (`sample_rgba_fnv
   1fc9e1e055002efd`, `bars100k_rgba_fnv a718ba2b22abb123`) aarch64 skaler/simd, x86_64
   skaler, x86_64 `target-cpu=haswell` + simd (AVX2, FMA kullanılabilir kod üretimi) ve
   wasm32 skaler/simd128'de aynı. Stres altın görseli de hepsinde 0 px.
2. **Neden:** Düz renk dolgular tiny-skia'nın tamsayılı lowp boru hattından geçer; x86'daki
   `_mm_rcp_ps`/`rsqrt` yaklaşıklamaları yalnızca highp (gradyan vb.) yolunda, kullanmıyoruz.
   Rust FMA'yı kendiliğinden birleştirmez. Sahnede transandantal fonksiyon yok (`log10`/
   `powf` yerine döngü ile 1-2-5 adımı, tarih için tamsayı `civil_from_days`); sayı
   biçimleme `core::fmt` ile. Metin birleştirici tamsayı (`div255` tüm aralıkta testli).
   Dikdörtgenler tam piksele hizalı, AA kapalı.
3. **simd özelliği** ölçümde pikseli değiştirmedi ama varsayılan kapalı tutuldu (x86 highp
   yolu yaklaşıklama kullanıyor; ileride gradyan eklenirse risk). simd128 wasm'da ~%10 daha
   küçük, hız farkı ölçülemedi.
4. **vello_cpu** bir mimari içinde deterministik (1T = MT) ama aarch64 (NEON) ile
   x86_64/wasm arasında AA kenarlarında ±1 kayma var. Bayt eşitliğine dayalı altın test için
   uygun değil.
5. **vello GPU Area AA** aynı cihazda aynı girdiyle farklı çıktı verebiliyor (yerel Metal
   ve tarayıcı WebGPU'da tekrarlanabilir şekilde). MSAA modları deterministik. GPU çıktısı
   sürücüye bağlı olduğu için yalnızca bu makinede ölçüldü.
6. Altın PNG'nin bayt eşitliği `png`/`fdeflate`/`flate2` sürümlerine de bağlı (`Cargo.lock`
   sabitliyor). Test uyuşmazlıkta piksel farkını ayrıca raporlar, böylece kodlayıcı
   değişikliği ile çizim değişikliği ayırt edilir.

## Metin bulguları

- fontdue kapsama maskesi + tamsayı birleştirici, tüm hedeflerde bit-eşit. Kalem tam piksele
  yuvarlanıyor (alt piksel konumlama yok); 11 px etiketlerde okunur, stres sahnesinde 7–40 px
  arası sorunsuz.
- vello_cpu ve vello aynı glif konumlarıyla outline rasterleştiriyor; referanstan farkın
  çoğu metin ve AA çizgi kenarlarında (ortalama kanal farkı 0,02).
- Font: IBM Plex Mono Regular, SIL OFL 1.1, değiştirilmeden gömülü
  (`assets/fonts/`, lisans `OFL.txt`). Sistem fontuna bağımlılık yok. "Plex" ayrılmış ad
  olduğu için alt kümeleme (subset) değiştirilmiş sürüm sayılır ve yeniden adlandırma
  gerektirir; bu yüzden R0'da tam font gömüldü.
- Şekillendirme (shaping) yok: kerning, ligatür, sağdan sola ve karmaşık yazılar
  desteklenmiyor. Tek aralıklı rakam/Latin etiketler için yeterli.

## A1 önerisi

1. **Referans yazılım çizici: tiny-skia + fontdue** (bu crate, skaler varsayılan). Headless
   PNG, CLI (`qp render`), altın görsel testleri ve tarayıcıda wasm yedeği aynı kod.
   Kanıt: tüm hedeflerde 0 px fark, tarayıcıda PNG baytları dahil aynı, 300–330 KB gzip.
2. **Etkileşimli GPU yolu: vello (wgpu) + WebGPU**, R2'de etkileşimle birlikte. Kanıt:
   tarayıcıda 100k bar 37 ms (CPU 80 ms), kararlı kare ~3 ms; MSAA modları yerel ile
   tarayıcı arasında 0 px. Koşullar: MSAA (Area değil), soğuk derleme maliyeti (~1,5 sn)
   için önce CPU kareyi göstermek, GPU testlerini referansa karşı toleransla yapmak.
3. **Yedek zinciri: WebGPU → CPU wasm.** vello WebGL2 desteklemiyor (compute shader
   gerekiyor); CPU yolu zaten hızlı olduğu için ayrı bir WebGL2 katmanı önerilmiyor.
   vello_hybrid (WebGL2 destekli) ölçülmedi; R2'de gerekirse aday.
4. vello_cpu bu aşamada alınmıyor: hız avantajı LOD ile kapatılabilir, determinizm kaybı
   referans rolüyle bağdaşmıyor.

## Riskler

- x86_64 sonuçları Rosetta 2 altında; gerçek x86 Linux donanımında altın test CI'da
  (`ubuntu-latest`, `cargo test --workspace`) koştuğunda doğrulanmış olacak.
- tiny-skia geometride `f32` kullanır; sahne `f64` hesaplayıp sınırda `f32`'ye çevirir.
  Hesap değerleri `f64` kuralı korunur, piksel koordinatları `f32`.
- 100k bar için LOD yok: CPU'da 38 ms (yerel) / 80 ms (wasm); etkileşimde yetmez.
- WASM boyutu: font 135 KB; `serde_json`, `png` ve tiny-skia boru hatları kodun çoğu.
- GPU: sürücüye bağlı çıktı, Area AA determinizmi, soğuk boru hattı derlemesi, WebGPU'nun
  her tarayıcıda olmaması (headless shell'de adaptör yok; tam Chromium + bayrak gerekti).
- `putImageData` bir çalıştırmada 102–166 ms ölçüldü, diğerlerinde ~1,3 ms (ilk canvas
  kurulumu); ölçüm gürültüsü olarak not edildi.

## R2 önerileri

- Piksel sütunu başına min/max LOD (100k bar → ~1100 sütun).
- vello/WebGPU yolunu etkileşim (pan/zoom) ile ölçmek; GPU görsel testleri için tolerans
  eşiği belirlemek (ölçülen: ort. kanal farkı ≤ 0,05).
- `qp-render`'ın kendi `BarsFile` tipini `qp-schema`'nın seri tipiyle birleştirmek (R0'da
  bilerek ayrı).
- Font: ayrılmış adı olmayan bir OFL font seçip alt kümelemek (wasm'da ~100 KB kazanç);
  şekillendirme gerekirse rustybuzz/swash değerlendirmek.
- wasm boyutu için `serde_json` yerine daha küçük ayrıştırıcı, PNG'yi wasm paketinden
  ayırmak.

## Yeniden üretme

```sh
# altın test (ve yeniden üretme)
cargo test -p qp-render --test golden
QP_UPDATE_GOLDEN=1 cargo test -p qp-render --test golden
# örnek veri (deterministik, commit'li)
cargo run -p qp-render --example gen_sample
# referans ölçümleri
cargo run --release -p qp-render --example bench [--features simd] [--target x86_64-apple-darwin]
# alternatif yığınlar
cargo run --release -p qp-render --example spike_vello_cpu --features spike-vello-cpu-mt
cargo run --release -p qp-render --example spike_vello_gpu --features spike-vello-gpu
# wasm + tarayıcı karşılaştırması (doğrulama girişi; eşik QP_WEB_MAX_DIFF_PIXELS, varsayılan 0)
shells/web-spike/run.sh
# bu dosyadaki tüm sayılar (GPU, Rosetta, tam Chromium gerekir)
shells/web-spike/run-spikes.sh   # çıktı: target/spikes/report/
```
