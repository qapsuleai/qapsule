# RP1 — Araştırma kaydı: hesap çekirdeği performansı

Round: RP1 (K18). Soru: ajan "hangi indikatör, hangi periyot, hangi parametre bu hareketi
en iyi açıklıyor?" diye sorduğunda cevap insanın fark etmeyeceği kadar hızlı gelsin; çekirdek
nasıl kurulmalı? Değişmezler: deterministik, tek çalıştırmayla ve aarch64 / x86_64 / wasm32
arasında **bit-eşit**, `f64`, LLM/ağ/saat/rastgelelik yok.

Kod: `rp1/integrate` dalı (kazananların birleşimi). Her deney kendi dalında, tabandan
(`caab6e3`) bağımsız yürüdü; ayrıntılı notları `rounds/RP1/experiments/E*.md`. Ölçüm tabanı ve
başlangıç ölçümleri `rounds/RP1/BASELINE.md`, ham çıktılar `rounds/RP1/raw/`.

## 0. Özet

- 8 yönde 21 yaklaşım denendi: 9'u alındı, 10'u alınmadı ya da sonraya kaldı, 1'i gözlem,
  1'i yalnızca analiz (GPU). İşe yaramayanlar aşağıda kayıtlı.
- **Kazananlar birlikte** (profil A, medyan ms; aynı CPU'da taban → final):
  - Referans makine (M4 Max): SMA 15,1→4,3 · EMA 9,6→3,8 · RSI 24,7→9,0 · MACD 41,4→7,5 ·
    Bollinger 46,3→8,6.
  - GitHub koşucusu (AMD EPYC 9V74): SMA 17,4→12,6 · EMA 21,6→4,7 · RSI 62,9→38,6 ·
    MACD 69,1→11,0 · Bollinger 72,0→11,8 — **K17 dizüstü hedefi beş indikatörde de tutuyor**.
  - WASM (CI, Node 22): en yavaş RSI 118,5→49,2 ms (< 150), MACD 159,9→25,2.
- **Bit-eşitlik testleri değişmeden geçiyor**; `cross_target_digest` = `7b1e99d40889f83c`
  (aarch64, x86_64 Rosetta, gerçek Linux x86_64 ×5, wasm32); her profilin özeti tabanla aynı.
- **Yeni yöntem:** kanıtlı geç başlangıç ("sandviç", E8) — EMA/RSI'nin son penceresini
  geçmişi koşmadan, tam çalıştırmayla **kanıtlı bit-eşit** hesaplar; 1M barda 20–45×.

## 1. Literatür ve var olan motorlar (T2)

Bu bölüm modelin bilgisinden yazıldı (ağdan doğrulanmadı); her satırın sonunda bizim için
çıkan ders var.

| Kaynak | Ne yapıyor | Bizim için ders |
|--------|-----------|-----------------|
| TA-Lib (C) | İndikatör başına tek parametreli döngü; "unstable period" kavramı: EMA/RSI'nin değeri geçmişe bağlı, doğru sonuç için fazladan ısınma barı önerilir. | Geçmişe bağımlılık bilinen sorun; ama TA-Lib'in çözümü sezgisel (yaklaşık). E8 bunu **kanıtlı** hâle getiriyor. |
| Tulip Indicators (C) | Toplu ve akışkan (stream) sürümler; tek parametre. | Bizim `Core` (peek/commit) tasarımıyla aynı yön; tarama yok. |
| vectorbt / pandas-ta (numba) | Parametre ızgarasını yayınlayarak (broadcast) tek çağrıda hesaplar; `IndicatorFactory` ortak ara sonuçları önbelleğe alır; numba `parallel=True` ile sütunlar arası paralellik. | Izgara üzerinde ortak alt hesap paylaşımı (E1) ve çok iş parçacığı (E3) yerleşik pratik; bit-eşitlik hedefi yok. |
| Polars / Apache Arrow kernels | Sütunlu bellek, SIMD hesap çekirdekleri, kayan pencere fonksiyonları. | Sütunlu çıktı + SIMD (E2, E6). Kayan toplamların sayısal kararlılığı için periyodik yeniden hesap yapıyorlar — bizde tanım değişikliği olur (E7). |
| kdb+/q | Vektör dili; `ema` gibi özyinelemeler `scan` ile sıralı. | Özyineleme bar ekseninde paralelleşmiyor; paralellik varyasyon/seri ekseninde (bizim şeritler). |
| Halide (algoritma / zamanlama ayrımı) | Ne hesaplanacağı ile nasıl (vektör, iş parçacığı, döşeme) ayrı; zamanlama sonucu değiştirmez. | Bizim ilkemiz aynı: "aynı işlemler, aynı sıra; yalnızca zamanlama farklı" → şeritler, iş parçacıkları, paylaşım bit-eşit. |
| Tekrarlanabilir kayan nokta: Intel MKL CNR, ReproBLAS (Demmel–Nguyen), Kahan/Neumaier | Paralel toplamada sonucun iş parçacığı sayısından bağımsız olması için sabit sıra ya da ikili (binned) toplama. | Biz toplama sırasını hiç değiştirmiyoruz (paralellik yalnızca bağımsız varyasyonlar arasında), bu yüzden bu tekniklere gerek kalmadı. Tamsayı kuruş toplamı (E7.6) tam toplamanın en basit hâli. |
| IEEE 754-2008 `fusedMultiplyAdd` | Tek yuvarlamalı `a·b+c`. | Doğru yuvarlandığı için hedefler arası eşittir ama tanımı değiştirir; FMA'sız hedeflerde yazılım emülasyonu 5 kat yavaş (E7.1). |
| Propp–Wilson "coupling from the past" (1996), monoton Markov zincirleri | Monoton bir sistemde en üst ve en alt durumdan başlatılan iki zincir birleşirse aradaki her başlangıç da aynı yerdedir; birleşme kesin örneği **kanıtlar**. | E8'in "sandviç"i bunun deterministik, yuvarlamalı kayan nokta karşılığı: EMA/RSI güncellemesi durumunda monoton; alt/üst sınır yörüngeleri birleşince gerçek yörünge de odur. |
| Artımlı hesap: Salsa (Rust), differential dataflow / DBSP | Girdi değişince yalnızca etkilenen düğümleri yeniden hesaplayan bağımlılık grafiği; içerik/sürüm anahtarlı not alma (memoization). | Hesap grafiği ve içerik adresli önbellek (E4); canlı barda yalnızca son barın yeniden hesabı zaten `Core` tasarımında var. |
| İçerik adresli depolama (Nix, Bazel, Unison) | Çıktı, girdilerin özetiyle adreslenir; aynı girdi = aynı sonuç, tekrar hesap yok. | `ResultCache` anahtarı (girdi özeti + hesap + parametre bitleri); determinizm, önbelleği güvenli kılıyor. |
| GPU hesap (Metal, WebGPU/WGSL, Vulkan) | Metal ve WGSL'de f64 yok; Vulkan'da isteğe bağlı ve çoğu mobil GPU'da yok. | f64 ve bit-eşitlik kuralı altında GPU yalnızca tam IEEE emülasyonuyla mümkün; bugünkü yük şeklinde kazanç beklenmiyor (E7.7). |

**Aday yaklaşımlar ve beklenen kazanç** (deneylerden önce; sonuçlar bölüm 3'te):

| Yön | Deney | Beklenen |
|-----|-------|---------|
| Ortak alt hesap paylaşımı (ızgara) | E1 | MACD ~3×, Bollinger ~5–10× |
| Veri paralelliği (SoA şeritler, SIMD) ve bit-eşitlik | E2 | RSI x86'da ~2×, EMA ~2–3×; B profiline seri şeritleri |
| İş parçacıkları | E3 | 4 çekirdekte ~3,5× |
| Hesap grafiği (zincirlerde önek paylaşımı) | E4 | C profilinde 4–7× |
| İçerik adresli sonuç önbelleği | E4 | Tekrar eden soruda ~50× |
| Canlı güncelleme / geçmişe bağlama | E5 | attach ~2× |
| Bellek yerleşimi, çıktı trafiği | E6 | %30–50 (sıfır doldurma) |
| Değişmezleri bozanlar: FMA, karşılıklı çarpma, önek toplamı (parametre uzayı yapısı), f32, geç başlama, tamsayı kuruş, GPU | E7 | ayrı bölümde |
| Tembel/artımlı hesap: kanıtlı geç başlangıç (yeni yöntem) | E8 | uzun geçmişte 10–100× |

## 3. Denenen yaklaşımlar

Her yaklaşımın ayrıntılı kaydı (denemeden önce yazılmış hipotez, yapılan, ölçüm tabloları,
bit-eşitlik kanıtı, karar) `rounds/RP1/experiments/` altında ve kendi dalında. Burada
özet; işe yaramayanlar dahil.

| # | Yön | Deney (dal) | Hipotez (kısa) | Sonuç (ölçüm) | Karar |
|---|-----|-------------|----------------|---------------|-------|
| 1 | Ortak alt hesap paylaşımı (ızgara) | E1 (`rp1/e1-share`, alt ajan) | MACD çizgisi (fast, slow) başına, Bollinger orta/std periyot başına bir kez; EMA tekilleştirme; RSI kazanç/kayıp bölmesi paylaşılır. MACD ~2,5×, Bollinger 4–5× | A (aarch64): MACD 40,9→7,6 (5,4×), Bollinger 45,0→8,9 (5,0×), RSI 24,4→9,1 (2,7×); WASM 7,4× / 6,3× / 3,0×; bit-eşit | **al** |
| 2 | Veri paralelliği: SoA şerit çekirdekleri | E2.1 (`rp1/e2-simd`) | Dallanmasız kararlı döngü 2 genişlikli SIMD'ye iner; x86 RSI ~2× | A: EMA 21,6→6,4, MACD 69→16,4, RSI 62,9→47,8 ms (CI AMD); WASM MACD 160→37 ms; bit-eşit | **al** |
| 3 | Seri şeritleri (B profili) | E2.1 | Aynı uzunluktaki seriler aynı parametreyle şeritlerde | B: EMA 25,9→5,7, RSI 62→21, MACD 44→13 ms (aarch64) | **al** |
| 4 | Hedefe göre şerit sayısı | E2.1 | x86'da 16 yazmaç → daha az şerit | aarch64 [4,4,4,2,2], x86 [2,4,4,2,2], wasm [1,2,2,1,1]; 8 şerit her yerde taşıyor | **al** |
| 5 | Çalışma anında AVX2 (`pulp`, `unsafe`sız) | E2.2 | x86 RSI ~1,5–2× daha | Kapanış yolu hedef özelliğini taşımıyor: ikilide 0 VEX komutu | **alma / sonra** |
| 6 | wasm `simd128` | E2.3 | WASM'da 1,5–2× | Gürültü düzeyinde (±%5) | **alma** |
| 7 | İş parçacıkları | E3 (`rp1/e3-threads`) | 4 çekirdekte ~3,5× | 4 → 3,5–3,9×, 8 → 6–7×, 16'da ~8×'de doyuyor; sıra-deterministik | **al** (ek API) |
| 8 | Hesap grafiği: zincirlerde önek paylaşımı | E4.1 (`rp1/e4-graph`) | C'de 4–7× | C: 99,95→21,7, 175,8→34,0, 161,7→26,0 ms (E2 ile birlikte, aarch64) | **al** |
| 9 | İçerik adresli sonuç önbelleği | E4.2 | Tekrar eden soruda ~50× | İsabet 0,5 µs, 1.000 varyasyon tekrar: 23,3 → 0,44 ms; 80 MB | **sonra** (bellek politikası) |
| 10 | Canlı: commit-only attach (`Study::extend`) | E5.1 (`rp1/e5-live`) | attach ~2× | aarch64 28,8→14,9 ms, WASM 60,8→19,0 ms; D özeti aynı | **al** |
| 11 | Canlı: tek biçimli SoA banka | E5 | — | Denenmedi: tik yolu 1,8 µs/500 hesap, sınır değil (ölçüm gerekçesi) | **alma** |
| 12 | Bellek: yeniden kullanılan sütunu sıfırlamamak | E6.1 (`rp1/e6-output`) | %30–50 | %3–23 (memset beklenenden hızlı) | **al** |
| 13 | Çıktı yolu payı / indirgeme | E6.2 | — | Okuma bile 4–8× maliyet; indirgeme API'leri (E3) ve pencere (E8) bu yön | gözlem |
| 14 | Kanıtlı geç başlangıç ("sandviç") — **yeni yöntem** | E8 (`rp1/e8-sandwich`) | Uzun geçmişte 10–100×, bit-eşit kanıtlı | 1M bar, son 500: EMA 44,5×, RSI 21,6×; 100/100 kanıtlandı | **al** (deneysel API) |
| 15 | FMA | E7.1 (`rp1/e7-beyond`) | aarch64 1,2–1,5× | aarch64 1,08×; x86/wasm 4,5–5,5× **yavaş**; tanımı değiştirir | **alma** (değişmez) |
| 16 | Karşılıklı çarpma (RSI) | E7.2 | x86 ~1,5× | 1,1–1,3×; değerlerin %83'ü değişir | **alma** (değişmez) |
| 17 | Parametre uzayı yapısı: önek toplamından tüm SMA'lar | E7.3 | 3–5× | 10k: 2–7×; 1M'de göreli hata 3e-9 | **alma** (değişmez + doğruluk) |
| 18 | f32 | E7.4 | 1,5–2× | Kazanç yok/kayıp; göreli 1,8e-5 | **alma** (değişmez) |
| 19 | Görünür pencereden düz geç başlama | E7.5 | 20× | Isınma yetmezse 1e-1…1e-6 hata | **alma**; kanıtlı hali #14 |
| 20 | Tamsayı kuruş toplamı (SMA) | E7.6 | 1,5× | 1,4–2,4×, daha doğru, hedefler arası eşit; tanım değişir | **sonra** (ana akış kararı) |
| 21 | GPU | E7.7 | — | f64 yok (Metal, WGSL); tam emülasyonla CPU'yu geçmiyor (analiz, ölçülmedi) | **sonra** |

Farklı yön sayısı (GOAL'daki listeye göre): ortak alt hesap (1, 8), bellek ve önbellek
davranışı (12, 13), veri paralelliği ve bit-eşitlik (2–6, 15), iş parçacıkları (7), artımlı/
tembel hesap (10, 14, 19), parametre uzayında yapı (17, 20), içerik adresli önbellek (9), GPU
(21) — sekiz yön, 21 yaklaşım.

### 3.1 Yeni yöntem: kanıtlı geç başlangıç (E8)

Bilinen motorlar (TA-Lib "unstable period") EMA/RSI'nin geçmişe bağımlılığını fazladan
ısınma barıyla *yaklaşık* çözüyor. RP1'de bulunan yöntem bunu **kanıtlı ve bit-eşit** yapıyor:

1. EMA ve RSI'nin kararlı güncellemesi, yuvarlamalı kayan noktada da durumunda monoton
   azalmayan (`fl(s·c)`, `c ≥ 0`, ve `fl(k + ·)` monoton; bileşimleri de).
2. Bar `s−1`'deki gerçek durumun sınırları ucuzca bilinir: EMA için girdilerin aralığı
   (yuvarlama payıyla genişletilmiş), RSI için `[0, en büyük kazanç/kayıp]`.
3. Alt ve üst sınırdan başlayan iki yörünge gerçek yörüngeyi her barda arasında tutar.
   İkisi bit bit eşitlendiği anda gerçek yörünge de onlara eşittir; sonrası aynı işlemler.
4. Eşitlenmezse daha erken başlanır, sonunda tam çalıştırma: sonuç her zaman doğru.

Bu, Propp–Wilson'un "coupling from the past" fikrinin deterministik, yuvarlamalı aritmetikteki
karşılığı. Ölçüm: eşitlenme EMA'da ≈ 19·p, RSI'de ≈ 38·p bar sürüyor; 1M barlık seride son
500 bar için EMA 44,5×, RSI 21,6×, 100 periyodun 100'ü kanıtlandı. SMA/Bollinger'in kayan
toplamı büzülmediği için uygulanamaz (E7.5'te fark ~1e-15 düzeyinde kalıcı).

## 4. Önce / sonra tabloları (kriter 3)

"Önce" = taban `caab6e3`, "sonra" = `rp1/integrate` (`0cd37ad`; E1+E2+E3+E4+E5+E6+E8 birlikte,
E7 yalnızca ölçüm programı). Medyan ms (D'de µs ya da ms). Referans makine: Apple M4 Max,
11 koşu, yük ortalaması ≈ 4. WASM: aynı makinede Node 24 (wasm32-wasip1, `--profile wasm`), 11
koşu. CI: GitHub `ubuntu-latest`, aynı commit için **5 çalışma**
([43625622117](https://github.com/qapsuleai/qapsule/actions/runs/43625622117), deneme 1–5);
koşucu CPU'su çalışmadan çalışmaya değişti: AMD EPYC 9V74 (deneme 2, 4, 5) ve AMD EPYC 7763
(deneme 1, 3). Taban CI ölçümü 9V74'te (çalışma 43618730149); aynı CPU ile karşılaştırma
yapıldı, 7763 ayrıca verildi. CI'da her satır 7 koşunun medyanı; tabloda üç 9V74 çalışmasının
medyanı. Çalışmalar arası yayılım A'da %2'nin altında (ör. RSI 38,5 / 39,3 / 38,6), B
Bollinger'de ≈ %11 (60,8–67,5). Taban CI ölçümü tek çalışma (denetim L8). Ham çıktılar `rounds/RP1/raw/`
(yerel) ve CI günlükleri.

### A — tek seri, 1.000 parametre (10.000 bar)

| Hesap | M4 Max önce → sonra | WASM (M4, Node 24) önce → sonra | CI 9V74 önce → sonra | CI 7763 sonra (2 çalışma) | CI WASM Node 22 (9V74) önce → sonra |
|-------|--------------------:|--------------------------------:|---------------------:|--------------------------:|------------------------------------:|
| SMA | 15,07 → **4,34** (3,5×) | 15,62 → 10,89 (1,4×) | 17,44 → 12,62 | 15,03 / 14,44 | 26,44 → 18,83 |
| EMA | 9,62 → **3,80** (2,5×) | 26,20 → 10,43 (2,5×) | 21,59 → **4,68** (4,6×) | 5,29 / 5,26 | 37,63 → 22,41 |
| RSI | 24,73 → **8,97** (2,8×) | 56,02 → 18,03 (3,1×) | 62,87 → **38,62** (1,6×) | 42,15 / 41,52 | 118,46 → 49,19 |
| MACD | 41,40 → **7,50** (5,5×) | 75,70 → 9,54 (7,9×) | 69,07 → **11,04** (6,3×) | 13,20 / 13,02 | 159,91 → **25,16** (6,4×) |
| Bollinger | 46,27 → **8,56** (5,4×) | 69,74 → 11,08 (6,3×) | 71,98 → **11,75** (6,1×) | 14,20 / 14,27 | 120,71 → 21,55 |

x86_64 Rosetta (yalnızca bilgi): SMA 15,54 → 6,75 · EMA 22,35 → 4,60 · RSI 27,77 → 14,48 ·
MACD 72,56 → 8,70 · Bollinger 64,43 → 10,74.

### B — 2.500 seri (500 sembol × 5 zaman dilimi, 9,25 M bar), tek hesap

| Hesap | M4 Max önce → sonra | WASM (M4) önce → sonra | CI 9V74 önce → sonra | CI 7763 sonra |
|-------|--------------------:|-----------------------:|---------------------:|--------------:|
| SMA | 14,43 → 6,28 (2,3×) | 15,36 → 11,39 | 17,75 → 15,35 | 17,29 / 16,93 |
| EMA | 25,93 → 5,40 (4,8×) | 17,61 → 12,26 | 40,35 → 9,17 (4,4×) | 10,43 / 9,67 |
| RSI | 61,99 → 18,52 (3,3×) | 61,43 → 31,98 | 98,27 → 42,23 (2,3×) | 47,79 / 47,43 |
| MACD | 44,16 → 12,41 (3,6×) | 68,85 → 18,31 | 88,98 → 20,14 (4,4×) | 24,06 / 23,44 |
| Bollinger | 42,84 → 32,37 (1,3×) | 65,29 → 40,92 | 113,99 → 66,51 (1,7×) | 77,42 / 77,72 |

### C — zincirli hesaplar, 1.000 varyasyon (10.000 bar)

| Aile | M4 Max önce → sonra | WASM (M4) önce → sonra | CI 9V74 önce → sonra | CI 7763 sonra |
|------|--------------------:|-----------------------:|---------------------:|--------------:|
| ema>rsi | 99,95 → 11,56 (8,6×) | 189,54 → 21,43 (8,8×) | 200,66 → 43,00 (4,7×) | 47,73 / 46,65 |
| rsi>bollinger | 175,84 → 14,35 (12,3×) | 340,67 → 17,48 (19,5×) | 335,97 → 19,86 (16,9×) | 25,29 / 25,25 |
| sma>ema>rsi | 161,71 → 16,02 (10,1×) | 379,57 → 30,82 (12,3×) | 300,34 → 49,51 (6,1×) | 56,53 / 54,98 |

### D — canlı banka (500 hesap)

| Ölçüm | M4 Max önce → sonra | WASM (M4) önce → sonra | CI 9V74 önce → sonra | CI 7763 sonra |
|-------|--------------------:|-----------------------:|---------------------:|--------------:|
| tik (500 hesabın hepsi), medyan µs | 1,79 → 1,75 | 4,25 → 4,17 | 3,50 → 3,53 | 4,70 / 4,66 |
| yeni bar, medyan µs | 2,67 → 2,67 | 5,12 → 6,00 | 5,18 → 5,38 | 7,49 / 7,45 |
| attach (kur + 10k bar), ms | 26,28 → **13,78** (`extend`) | 51,11 → **18,11** | 51,82 → **17,33** | 20,11 / 19,81 |

D'nin tik ve yeni bar yolu bilinçli olarak değiştirilmedi (sınır değil, BASELINE).

**Gerilemeler (denetim L3):** aynı kaynakta olmasına rağmen WASM'da D "yeni bar" 5,1 → 6,0 µs
ve eski attach yolu (`push` döngüsü) 51,1 → 60,3 ms (+%18) yavaşladı — kod yerleşimi
etkisi (bkz. §8); yeni attach yolu `extend` 18,1 ms. Rosetta'da B Bollinger 61,9 → 123,7 ms
(2×); gerçek x86'da (CI) aynı satır 114,0 → 66,5 ms iyileşti, yani Rosetta'ya özgü (2 şeritli
SoA Bollinger çekirdeğinin Rosetta çevirisi).

## 5. K17 dizüstü hedefleri GitHub Linux koşucusunda (kriter 2)

K17: 10.000 bar × 1.000 varyasyon, tek çekirdek, yerel < 50 ms, WASM < 150 ms. CI ölçümü
bilgi amaçlı, kapı değil. Aşağıdaki sayılar final commit `0cd37ad`'nin 5 CI çalışmasının
**en kötüsü** (her biri 7 koşunun medyanı):

| Hesap | Yerel en kötü (CPU) | < 50 | WASM en kötü | < 150 | R1 / taban (9V74) |
|-------|--------------------:|:----:|-------------:|:-----:|------------------:|
| SMA | 15,03 (7763) | ✅ | 21,86 | ✅ | 17,4 / 26,4 |
| EMA | 5,29 (7763) | ✅ | 25,70 | ✅ | 21,6 / 37,6 |
| RSI | 42,15 (7763) | ✅ | 57,34 | ✅ | **62,9** / 118,5 |
| MACD | 13,20 (7763) | ✅ | 29,24 | ✅ | **69,1** / **159,9** |
| Bollinger | 14,27 (7763) | ✅ | 27,21 | ✅ | **72,0** / 120,7 |

Ara ölçüm (`rp1/integrate`, E1 hariç, çalışma 43622672559, Intel Xeon Platinum 8573C):
SMA 11,7 · EMA 5,2 · RSI 33,8 · MACD 13,2 · Bollinger 40,5 ms; WASM en kötü RSI 69,7 —
yani hedef üç farklı koşucu CPU'sunda tutuyor. Pay en dar RSI'de (7763'te hedefin %84'ü):
x86'da RSI bölme biriminin çıkışıyla sınırlı (bölüm 7).

## 6. Değişmezleri bozan ama umut vadeden yaklaşımlar (kriter 5)

Hiçbiri çekirdeğe alınmadı. Ölçüm programı `crates/qp-compute/examples/rp1_beyond.rs`
(davranış değiştirmez), ayrıntı `experiments/E7-beyond.md`. Karar ana akışındır.

| Yaklaşım | Hız (aarch64 / x86 / wasm) | Bit bedeli | Bozduğu | Karar için gereken |
|----------|---------------------------|------------|---------|--------------------|
| Tamsayı kuruş toplamı (SMA; Bollinger'e de genişler) | 2,35× / 1,85× / 1,9× | Değerlerin %96'sı ≤ 130 ulp farklı; sonuç **daha doğru** (toplam tam), hedefler arası eşit | SMA tanımı (R1 özetleri değişir) | Fiyatın tick çarpanı olduğu bilgisi (enstrüman tick boyu şemada); zincir girdilerinde uygulanamaz. **Umut vadeden tek tanım değişikliği.** |
| Önek toplamı (parametre uzayı yapısı) | 6,8× / 4,5× / 2,0× | 1M barda göreli 3,3e-9 (kayan toplamdan ~1000 kat kötü) | Tanım + doğruluk | Kahan/iki-katlı önek toplamıyla yeniden ölçülmeli |
| RSI'de bölme yerine karşılıklı çarpma | 1,30× / 1,08× / 1,33× | %83 değer ≤ 128 ulp | Tanım | Kazanç küçük; önerilmez |
| FMA | 1,08× / **0,22×** / **0,18×** | %32 değer ≤ 8 ulp; hedefler arası eşit (doğru yuvarlanmış `fma`) | Tanım | x86-64-v3 tabanı + wasm'da FMA olmadığı sürece zararlı; önerilmez |
| f32 | 0,91× / 0,58× / 1,05× | göreli 1,8e-5 | f64 kuralı | Önerilmez |
| GPU | ölçülmedi (analiz) | f32/df64 bit-eşit değil; tam f64 emülasyonu mümkün ama pahalı | f64 / bit-eşitlik (emülasyonsuz) | Bugünkü yük şeklinde kazanç yok; 100k+ varyasyonlu iş için yeniden düşünülmeli |
| `unsafe` ile AVX2/AVX-512 (E2.2) | ölçülemedi (Rosetta AVX çevirmiyor) | bit-eşit kalır (aynı IEEE işlemleri) | **Proje kuralı** `unsafe_code = "forbid"` (değişmez değil) | Ya kural gevşetilir ya `pulp`'ın açık SIMD tipleriyle çekirdekler yeniden yazılır; x86 RSI için beklenen 1,5–2× |

## 7. Bir sonraki sınır (kriter 6)

**Bugün en yavaş olan** (final, CI 9V74 / 7763 ve M4):

1. **x86'da RSI taraması** (38,6–42,2 ms; hedefin %77–84'ü). Varyasyon-bar başına 4 bölme
   (`ag/p`, `al/p`, `ag/al`, `100/(1+rs)`); bit-eşitlik bölmeyi çarpmaya çevirmeye izin
   vermiyor, SSE2 `divpd` 2 genişlikli. Aşmak için: **çalışma anında geniş SIMD** (AVX2 4×,
   AVX-512 8× `vdivpd`) — `unsafe` kuralının kapsamlı bir istisnası ya da `pulp` tipleriyle
   yeniden yazım (mimari karar).
2. **Çıktı yazımı.** E1 ve E6'nın ortak bulgusu: paylaşım ve şeritlerden sonra süre giderek
   "her varyasyonun her sütununu yaz" işine kayıyor (1.000 varyasyonda 80–320 MB yazma; okuma
   bile 4–8× maliyet). Ajan çoğu zaman varyasyon başına bir skor ya da görünür pencere istiyor.
   Aşmak için: **sorgu biçimli hesap API'si** — istenen çıktılar, bar aralığı ya da indirgeme
   (`sweep_reduce_par` ve `late::*_tail` bunun ilk adımları); doküman/şemada "hesap sorgusu"
   kavramı (öneri, şema değişikliği bu round'un kapsamı dışında).
3. **B'de Bollinger** (CI 66–78 ms): tek parametreli, seri şeritlerinde düz pencere seçimi için
   iki M2 formülü birden + 1 bölme + karekök + 4 çıktı. Seriler arası paylaşılacak bir şey yok;
   kazanç ancak çıktıyı azaltmaktan (2) ya da geniş SIMD'den (1) gelir.
4. **Uzun geçmiş.** Tam uzunluklu çıktı (K13) geçmiş uzadıkça doğrusal büyüyor (1M bar × 1.000
   varyasyon ≈ saniyeler). Kanıtlı geç başlangıç (E8) EMA/RSI için bunu pencere boyuna indiriyor;
   **SMA/Bollinger için kayan toplam büzülmediğinden** aynı kanıt yok. Aşmak için: tamsayı kuruş
   toplamı gibi tam (exact) bir toplam tanımı (bölüm 6) — o zaman herhangi bir başlangıçtan
   tam toplamla bit-eşit pencere mümkün olur. Bu bir **tanım kararı**.
5. **WASM'da iş parçacığı yok.** E3 yerelde 4 çekirdekte 3,5–3,9× veriyor; tarayıcıda
   SharedArrayBuffer + COOP/COEP + wasm thread derlemesi gerekir (kabuk/dağıtım kararı).

## 8. Bileşim notları (kazançlar toplanmadı)

- E1 ile E2 aynı işi farklı yerden hızlandırıyor; birleşimde paylaşılabilen ızgara
  parçaları E1'in yolundan, geri kalan her şey (B, SMA/EMA, paylaşılmayan kümeler, C'nin son
  aşaması) E2'nin şerit çekirdeklerinden geçiyor (`sweep/share.rs` → `soa_plain`). RSI'de 4'ten
  az varyasyon şerit çekirdeğine gidiyor (E1 raporu: tek varyasyonlu RSI'de paylaşım yolu
  x86'da yavaştı).
- Birleşimde WASM'da A Bollinger beklenmedik biçimde 39,5 ms'ye çıktı (E1 tek başına 11,9).
  Node `--cpu-prof` ile nedeni bulundu: paylaşılan Bollinger koşusu ziyaretçi kapanışıyla
  birlikte büyük bir çağırana gömülüyor ve wasm `opt-level s` altında kötü derleniyordu.
  `bollinger_run`'ı `#[inline(never)]` yapmak 11,0 ms'ye indirdi (bitler aynı). E1'in raporunda
  da benzer üç kod yerleşimi etkisi var: çekirdek hızı, aynı kaynakta derleyicinin satır içi
  alma kararlarına duyarlı; CI'daki şerit tablosu ve profil ölçümleri bunu izlemek için duruyor.
- C artık A'nın çekirdek hızına bağlı (E4 son aşamayı tarama olarak koşuyor): E4 tek başına
  3–6×, E1+E2 ile birlikte 9–20×.
