# E7 — Değişmezleri bozan ama hızlı olabilecek yaklaşımlar: ölçüm ve bedel

Dal: `rp1/e7-beyond` (taban `caab6e3`). Yürüten: orkestratör (alt ajan başlatılamadı).
Bu yaklaşımların hiçbiri çekirdeğe alınmadı. Ölçüm programı `crates/qp-compute/examples/rp1_beyond.rs`
hiçbir kütüphane yolunu değiştirmiyor; dal birleştirilse de davranış değişmez.

## Hipotezler (denemeden önce yazıldı)

| # | Yaklaşım | Beklenen hız | Beklenen bit bedeli |
|---|----------|--------------|---------------------|
| 7.1 | FMA (`mul_add`) EMA'da | aarch64 ≈ 1,2–1,5 kat (zincir 2 işlemden 1'e); x86_64 temel hedefte ve wasm'da FMA komutu yok → yazılım `fma` çağrısı, 5–20 kat **yavaş** | Değerlerin çoğu 1 ulp civarı farklı; doğru yuvarlanmış `fma` her hedefte aynı biti verir, yani hedefler arası eşitlik **korunur**, bozulan "tanım" (R1 özetleri değişir) |
| 7.2 | Bölme yerine karşılıklı çarpma (RSI `/pf`) | x86'da RSI ≈ 1,5 kat (bölme çıkışı sınırı) | Değerlerin önemli kısmı 1 ulp farklı; hedefler arası eşit kalır; tanım değişir |
| 7.3 | Parametre uzayı yapısı: tüm SMA periyotları tek önek toplamından | Bağımlılık zinciri yok, vektörleşir: 1.000 periyot × 10k ≈ 3–5 kat | Önek toplamı büyüdükçe yuvarlama hatası büyür: 10k barda ~1e-12 göreli, 1M barda ~1e-10; kayan toplam tanımından neredeyse her değer farklı |
| 7.4 | f32 | Bellek ve SIMD genişliği 2 kat → ≈ 1,5–2 kat | Göreli hata 1e-7 düzeyi; fiyat 2 ondalıkta 100'lük hissede 1e-5 → görünür; f64 kuralını bozar |
| 7.5 | Görünür pencereden başlamak (geç başlama) | Görünür pencere 500 bar, geçmiş 10k → 20 kat | EMA/RSI geometrik yakınsar (EMA(20) için 200 barlık ısınmada ~1e-8 göreli), SMA kayan toplamı ise farklı bitlerle sürer (~1e-14) |
| 7.6 | Tamsayı kuruş toplamı (SMA) | Tamsayı toplama/çıkarma, vektörleşir: ≈ 1,5 kat | Tam toplam; f64 kayan toplamından farklı (çoğu değer 1 ulp) |
| 7.7 | GPU | Ölçülmedi: analiz (aşağıda) | f64 yok ya da çok yavaş; emülasyonla bit-eşitlik mümkün ama pahalı |

## Ölçüm

Program: `cargo run --release -p qp-compute --example rp1_beyond -- --reps 5` (x86_64:
`--target x86_64-apple-darwin`, Rosetta; wasm: `--target wasm32-wasip1 --profile wasm`, Node 24).
Her prob, "bugünkü tanım" çekirdeğini ve "değiştirilmiş" çekirdeği **aynı yazım biçiminde**
(4 şerit, SoA) ölçer; bugünkü tanım çekirdeğinin kütüphaneyle bit-eşit olduğu program içinde
`assert` ile doğrulanır. Fark sütunları değiştirilmiş çıktının kütüphanenin tek çalıştırmasına
göre farkıdır. Girdi profil A serisi (10.000 bar), 1.000 periyot (2…1001); "1M" satırları
1.000.000 bar × 5 periyot.

| Prob | aarch64 bugün → yeni (ms) | x86_64 Rosetta | wasm32 | Farklı değer | En büyük göreli | En büyük ulp | Hedefler arası özet |
|------|--------------------------:|---------------:|-------:|-------------:|----------------:|-------------:|---------------------|
| 7.1 EMA FMA | 6,81 → 6,30 (1,08×) | 11,7 → 53,8 (**0,22×**) | 10,7 → 57,8 (**0,18×**) | %32,4 | 1,0e-15 | 8 | aynı (`699b755e…`) |
| 7.2 RSI karşılıklı çarpma | 12,4 → 9,6 (1,30×) | 14,7 → 13,7 (1,08×) | 20,8 → 15,6 (1,33×) | %83,4 | 2,4e-14 | 128 | aynı (`92390283…`) |
| 7.3 SMA önek toplamı, 10k | 14,0 → 2,05 (**6,8×**) | 15,8 → 3,6 (4,5×) | 15,6 → 7,8 (2,0×) | %98,4 | 1,1e-12 | 7.925 | aynı |
| 7.3 SMA önek toplamı, 1M | 6,2 → 3,3 (1,9×) | 7,9 → 5,5 | 8,0 → 4,6 | %99,98 | **3,3e-9** | 1,6e7 | aynı |
| 7.6 SMA tamsayı kuruş, 10k | 14,0 → 6,0 (2,35×) | 15,8 → 8,6 (1,85×) | 15,6 → 8,2 (1,9×) | %96,1 | 1,8e-14 | 130 | aynı |
| 7.6 SMA tamsayı kuruş, 1M | 6,2 → 3,1 (2,0×) | 7,9 → 4,8 | 8,0 → 5,9 | %99,9 | 6,4e-12 | 31.600 | aynı |
| 7.4 EMA f32 (8 şerit) | 6,3 → 6,9 (0,91×) | 8,3 → 14,5 (0,58×) | 10,7 → 10,1 (1,05×) | %100 | **1,8e-5** | — | aynı |

**7.5 Geç başlama** (son 500 barın en büyük göreli hatası; çalıştırma, pencereden `w` bar önce
başlıyor; 10.000 barlık seri):

| Hesap | p+1 | 50 | 200 | 1.000 | 5.000 |
|-------|----:|---:|----:|------:|------:|
| EMA(20) | 6,2e-3 | 5,8e-5 | 9,4e-11 | **0** | **0** |
| EMA(200) | 6,9e-3 | 6,9e-3 | 6,9e-3 | 6,1e-6 | **0** |
| RSI(14) | 1,9e-1 | 1,4e-2 | 1,4e-7 | **0** | **0** |
| RSI(100) | 8,4e-3 | 8,4e-3 | 1,0e-3 | 2,3e-6 | **0** |
| SMA(20) | 6,2e-15 | 7,0e-15 | 7,0e-15 | 7,6e-15 | 1,0e-15 |

Beklenmeyen bulgu: EMA ve RSI gibi büzülen özyinelemeli hesaplarda geç başlatılan çalıştırma
yeterli ısınmadan sonra tam çalıştırmayla **bit bit aynı** oluyor (iki yörünge kayan noktada
birleşiyor ve aynı işlemlerle aynı kalıyor). SMA'nın kayan toplamı ise büzülmüyor; fark kalıcı.
Bu, kanıtlanabilir bit-eşit bir tembel başlangıç yöntemine yol açtı: **E8**.

## Değerlendirme ve öneriler (ana akış kararı için)

| Yaklaşım | Bozduğu | Kazanç | Öneri |
|----------|---------|--------|-------|
| 7.1 FMA | Tanım (R1 özetleri değişir); hedefler arası eşitlik **korunur** (doğru yuvarlanmış `fma`) | aarch64 %8; x86_64 temel ve wasm'da 4,5–5,5 kat **kayıp** (FMA komutu yok, yazılım `fma`) | **Alma.** Ancak hedef CPU tabanı x86-64-v3'e yükseltilir ve wasm relaxed-simd'e geçerse yeniden ölçülmeli; o zaman bile wasm'ın `fma`'sı belirlenimsiz (relaxed) olduğundan bit-eşitlik bozulur. |
| 7.2 Karşılıklı çarpma | Tanım; hedefler arası eşit | RSI 1,1–1,3 kat | **Alma.** Kazanç küçük, değerlerin %83'ü değişiyor. |
| 7.3 Önek toplamı (parametre uzayı yapısı) | Tanım ve **doğruluk**: 1M barda göreli 3e-9 (kayan toplamdan ~1000 kat kötü) | SMA taramasında 2–7 kat | **Alma** (doğruluk kaybı uzun seride büyüyor). Doğruluk için önek toplamı Kahan/iki-katlı toplamla tutulursa ayrı değerlendirilebilir; yine tanım değişir. |
| 7.6 Tamsayı kuruş | Tanım (f64 kayan toplam yerine tam toplam); f64 kuralı çıktıda korunur, ara toplam tamsayı | SMA 1,4–2,4 kat; ayrıca **daha doğru** (toplam tam) | **Sonra.** İlginç: sonuç tanım olarak "daha doğru" ve hedefler arası eşit. Fiyatların kuruş (ya da tick) çarpanı olduğu bilinmeli (şemada enstrüman tick boyu); hacim ve türetilmiş girdilerde (zincir) uygulanamaz. Tanım değişikliği → ana akış kararı. |
| 7.4 f32 | f64 kuralı; doğruluk 1e-5 | Yok ya da kayıp | **Alma.** |
| 7.5 Geç başlama (düz) | Bit-eşitlik (yetersiz ısınmada) | Görünür pencere ≪ geçmiş olduğunda geçmiş kadar | Düz hali **alma**; kanıtlı hali E8. |

### 7.7 GPU (ölçülmedi; analiz)

- **f64 desteği:** Apple GPU'larında Metal Shading Language `double` desteklemez. WebGPU/WGSL'de
  f64 tipi yok (yalnızca `f16` uzantısı). Vulkan'da `shaderFloat64` isteğe bağlı; mobil
  GPU'ların çoğunda yok; masaüstü tüketici GPU'larında f64 hızı f32'nin 1/32–1/64'ü.
- **f32 ya da double-f32 (df64):** IEEE f64 ile bit-eşit değil (df64 ≈ 48 bit mantis, farklı
  yuvarlama). f64 kuralını ve hedefler arası eşitliği bozar.
- **Tam IEEE f64 emülasyonu (32 bit tamsayı işlemleriyle):** bit-eşitlik ilke olarak
  mümkün. Kaba maliyet: toplama ≈ 30–60, çarpma ≈ 50–100, bölme ≈ 200–400 tamsayı işlemi.
  Tarama şekli (1.000 bağımsız varyasyon × 10.000 sıralı bar) GPU'ya kötü uyar: paralellik
  yalnızca varyasyon sayısı kadar (≈ 1.000 iş parçacığı; bir M4 Max GPU'su 5.000+ ALU'ya
  sahip), her iş parçacığı 10.000 adımlık bağımlı zincir koşar. RSI için iş parçacığı başına
  ≈ 10.000 × ~1.000 bağımlı tamsayı işlemi ≈ 1e7 adım × 4–8 döngü gecikme ≈ 30–60 ms —
  bugünkü CPU süresiyle aynı mertebe, üstüne gönderim/eşitleme gecikmesi (0,1–1 ms).
- **Ne değişirse anlamlı olur:** (a) çok daha geniş paralellik (ör. 100k varyasyon ya da
  binlerce sembol × periyot; profil B'nin büyüğü); (b) f64'ün kaldırılması (f32 hesap) —
  kuzey yıldızının 3. ilkesi ve f64 kuralı açısından ana akış kararı; (c) bar ekseninde
  paralel tarama (prefix-scan) — EMA/RSI'nin doğrusal özyinelemesi tarama olarak yazılabilir
  ama bu işlem sırasını değiştirir, bit-eşitliği bozar.
- **Öneri: sonra.** Bugünkü yük şekillerinde GPU kazanç vaat etmiyor; bit-eşit emülasyon
  maliyeti CPU'yu geçemiyor. Ölçüm için `wgpu` tabanlı ayrı bir karalama crate'i gerekir
  (çekirdeğe bağımlılık eklenmez).
