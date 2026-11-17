# RP3 — Araştırma ve denenen yaklaşımlar

Tarih: 2026-11-17. GOAL ölçüt 12: denenen her yaklaşım, işe yaramayanlar dahil. Bu belge
yalnız alt görev notlarını derler; yeni ölçüm içermez. Her sayının kaynağı satırın son
sütunundadır; sayılar notlardaki gibi, ölçüm ayarıyla birlikte verilmiştir.

**Ayar kısaltmaları** (T1, `rounds/RP3/CALIBRATION.md`):

- **PHONE**: Playwright WebKit 26.5, `__XPC_JSC_useJIT=false` (JIT kapalı JavaScriptCore),
  DPR 3, 402×874, demo WASM'ı. Arkin'in iPhone ölçümünün koşulunu (düz HTTP, JIT'siz) kurar.
  Okuma çarpanı `PHONE_BIAS`: yüz ×1,0, sahne ×1,85, sahneli tik ×1,6.
- **PHONE_JIT_EST**: aynı WebKit, JIT açık, DPR 3 (HTTPS'teki iPhone için tahmin; kalibre değil).
- **Chromium N×**: Chromium, CDP `setCPUThrottlingRate` N kat, çoğunlukla DPR 3. CDP yalnız
  ana iş parçacığını yavaşlatır; GPU/raster sürecini yavaşlatmaz.
- WebKit'in `performance.now()`'ı 1 ms adımlıdır: WebKit satırlarında 0 = "1 ms'den az".
- (Güncelleme, round sonu) M12a/b/c, M14 ve S dalları `main`'e birleşti (`4c7868a`); "dalda" etiketleri kaldırıldı. Varsayılan yol `canvas` (öneri, Ö53), `cpu` ve `gpu` `?paint=` ile seçilir.
  `rp3/m12c-gpu`, `rp3/m14-freq`); not `git show <dal>:rounds/RP3/experiments/<ad>.md` ile okunur.
- M12a/b/c'nin PHONE sayıları otomatik karşılaştırma senaryosundandır (`compare.mjs` ve
  türevleri; `ref` sahne p50 ≈ 25 ms). T1'in kalibrasyon senaryosu farklıdır (`ref` sahne
  38 ms). İki senaryonun sayıları birbiriyle karşılaştırılmaz; her satırdaki `ref` ile
  karşılaştırılır.

**Durum değerleri:** *alındı* (kodda kaldı) · *alınmadı*
(denendi, bırakıldı) · *öneri* (ölçüldü ya da tahmin edildi, uygulanmadı) · *ölçülmedi*.

---

## 1. Kısa özet

Round üç şey öğrendi. (1) **Cihaz görüntüleyicidir (K48)**: tespit, sıralama ve yüz
derlemesi otoriteye, açık kapsülün grafiği ve sınaması cihaza verilince cihazın tik işi
evrenden bağımsız oldu (Chromium, DPR 3, kısıtsız; 30 → 3.000 enstrümanda akışta 1,66 →
1,81 ms/adım, kapsül açıkken 5,26 → 5,77 ms/adım; tam yansıda 2,66 → 128,45 ms/adım) (M15).
(2) **Arkin'in iPhone'u JIT'siz çalışıyordu**: iOS 27, güvenli bağlam olmayan sayfalarda
(`http://`, LAN IP'si) JS'i ve WASM'ı yorumlayıcıyla çalıştırıyor; WebKit bunu kasıtlı bir
"Enhanced Security" davranışı olarak kapattı ([WebKit 324968](https://bugs.webkit.org/show_bug.cgi?id=324968),
RESOLVED INVALID). Masaüstü → telefon farkının (sahne ×101) payı: JIT kapalı ≈ %55, piksel
(DPR 1 → 3) ≈ %34, açıklanamayan kalıntı ≈ %14, motor ≈ %0; SIMD, wasm-opt, kopya ve
`putImageData` farkın nedeni değil (T1). R4'ün "CDP 4× ≈ orta sınıf" eşlemesi yanlış
mekanizmaydı. Aynı iPhone'un HTTPS ölçümü yapılmadı; round'un en önemli açık sorusu bu.
(3) **Boyama yolları**: PHONE'da üç yol da `ref`'ten belirgin hızlı. `canvas` (Canvas 2D +
fontdue glif atlası) yüz 33 → 2 ms, sahne 25 → 5 ms; `gpu` (çekirdekte üçgenleme + ince
WebGPU) yüz 33 → 4, sahne 25 → 6 ms, ama WebGPU güvenli bağlam istediği için düz HTTP'de
hiç devreye girmiyor; `cpu` (tutulan yüzeyler, görünüm, harekette ölçek 1) sahne 25 → 6 ms,
yüz 33 → 23 ms ve referansla bayt bayt aynı. SIMD, wasm-opt, kirli dikdörtgen/önek katmanı
bu senaryoda ölçülebilir kazanç getirmedi. Açılışta asıl kazanç sıkıştırma ve WebKit'teki
çift WASM indirmesinin kaldırılması oldu (WebKit'te aktarılan bayt 8.850.801 → 1.334.452 B;
M13). Uzun oturumdaki 152 MB'ın nedeni sayfanın yeniden başlamasıydı (T4).

---

## 2. Araştırma (T3)

Kaynak: `rounds/RP3/experiments/T3-research.md` (web ve WebKit `main` kaynak kodu,
erişim 2026-11-17). WebKit `main` iOS 27 ile gelen WebKit'ten ileride olabilir; kaynaktan
çıkan davranışlar "cihazda doğrulanmalı" sayıldı.

### 2.1 Tarayıcı ve WebKit gerçekleri

| Konu | Bulgu | Kaynak | Ölçümle durumu |
|---|---|---|---|
| iOS'ta 2B tuval | `UseGPUProcessForCanvasRenderingEnabled` Cocoa/iOS'ta açık: tuval GPU sürecinde; `CanvasUsesAcceleratedDrawing` iOS cihazda açık; tampon `Accelerated`, `willReadFrequently: true` ise `Unaccelerated` | WebKit `UnifiedWebPreferences.yaml` (~6807, ~1540), `CanvasRenderingContext2DBase.cpp` `allocateImageBuffer()` | `willReadFrequently` M12b'de atlas tuvalini CPU'ya almak için kullanıldı (renk takası hatasını giderdi). iPhone'da GPU sürecinin süresi ölçülmedi |
| `putImageData` | Kirli dikdörtgen argümanları destekleniyor, yalnız o bölge gönderiliyor; her çağrı RGBA'yı GPU sürecine taşır (çıkarım) | `CanvasRenderingContext2DBase.cpp` ~3240 | T1: kopya + put JIT'siz sahnede < %3 (0,5 / 43,5 ms), JIT'li V8'de ≈ %23. "Telefonu put yavaşlatıyor" varsayımı JIT'siz kipte **çürütüldü** |
| `canvas.width = w` her boyamada | Boyut aynıysa tampon tutulur ama durum sıfırlanır + tam `clearRect`; boyut değişirse yeni tampon | `HTMLCanvasElement.cpp` ~673, `CanvasRenderingContext2DBase.cpp` ~366 | T1 "boyut+put" sütunu 0,00–0,25 ms: küçük. M12a ve M12b boyut değişmedikçe atamıyor |
| Alternatif tuval API'leri | `transferFromImageBitmap` (iOS 11.3+), `OffscreenCanvas` 2B (iOS 16.4+), WebGL'li OffscreenCanvas (Safari 17+), `willReadFrequently` (iOS 18+), `desynchronized` 2B (iOS 15+) | caniuse, web-features, WebKit 253431, Chrome blog | `desynchronized`: M12b'de Chromium 4×'te etkisiz. Diğerleri ölçülmedi |
| Tuval alan ve bellek sınırı | `maxCanvasArea()` iOS'ta 8192² = 67 M piksel; toplam tuval belleği tarihsel olarak 224/256/384 MB, iOS 27 değeri doğrulanamadı; lightweight-charts tuvali bırakırken 1×1 yapıyor | `CanvasBase.cpp` ~119, pqina, Apple forum 687866, lightweight-charts `releaseCanvas` | Ölçülmedi; tuval sayısı T4'te 41'de sabit |
| WebGPU | Safari 26'da (iOS 26) varsayılan açık; `disableInLockdownMode: true`; iOS 26 öncesi yok; Chrome-on-iOS'ta açık olduğu doğrulanamadı | WebKit WWDC25 blogu, `UnifiedWebPreferences.yaml` ~7516, WebKit 299237, Apple forum 770862 | M12c: WebGPU **güvenli bağlam ister**; LAN'dan düz HTTP ile açılan sayfada iki motorda da `navigator.gpu` yok (ölçüldü). T3 bunu öngörmemişti |
| JSC'de WASM katmanları | IPInt yorumlayıcı → BBQ → OMG (`thresholdForBBQOptimizeAfterWarmUp = 150`, OMG 50.000) | `OptionsList.h`, WebKit blogları | T1: `useOMGJIT=false` WASM'ı 2,3× yavaşlatıyor. "1., 10., 100. boyama" ısınma ölçümü **yapılmadı** |
| WASM SIMD | simd128 Safari 16.4+; Relaxed SIMD Safari'de yok; T3 beklentisi taramada 1,5–3× (tahmin) | Emscripten belgesi, caniuse | **Çürütüldü**: T1 JIT'siz −%9, V8 −%2; M12a'da kazanç yok (§3.3) |
| Akışlı derleme | `instantiateStreaming` `application/wasm` ister; `Content-Encoding` ile çalışır | MDN, WebKit 26.6 | M13: iki tarayıcıda akışlı yol kullanılıyor, yedek uyarısı yok (doğrulandı) |
| Derlenmiş kod önbelleği | Safari'nin WASM makine kodunu önbelleğe aldığına kaynak yok | WebKit 166481, Mozilla 1469395 | M13: Chromium sıcak açılışta WASM hazır 1.036 → 125 ms (`no-store` kaldırılınca). Safari için ölçülmedi (Playwright WebKit'te HTTP önbelleği çalışmıyor) |
| brotli | Safari brotli'yi yalnız HTTPS'te ilan eder (genel bilgi, doğrulanamadı) | — | M13: WebKit 26.5 hem localhost hem LAN'da `gzip, deflate`; Chromium localhost'ta br, LAN'da `gzip, deflate`. Telefonda yalnız gzip işe yarar (**doğrulandı**, masaüstü motorlarında) |
| 60 Hz sınırı | `PreferPageRenderingUpdatesNear60FPSEnabled` varsayılan açık (visionOS hariç); WKWebView'ı etkilemiyor bildirilmiş | `UnifiedWebPreferences.yaml` ~5131, WebKit 294338 | Başsız WebKit 60 Hz; T2 yenileme kestirimi bunu kullanıyor |
| Zamanlayıcı çözünürlüğü | WebKit `timePrecision { 1_ms }` | `Performance.cpp` | Bütün WebKit ölçümlerinde 1 ms adım görüldü; T2'nin uzun kare eşiği buna göre düzeltildi |
| Düşük Güç Kipi | rAF 30 fps'e kısılır ("This is intentional", 2020) | WebKit 215745 | iOS 27'de doğrulanmadı; ölçülmedi |
| Benzer ürünler | lightweight-charts: bölme başına iki tuval, geçersizlik düzeyleri, rAF başına bir çizim, tam DPR; uPlot tek Canvas 2D; Flutter Safari'de CanvasKit varsayılan; Figma WebGPU + oturum ortasında WebGL'e düşen yedek | ilgili kaynak kodları ve bloglar | rAF başına bir çizim M14'te alındı |

### 2.2 Sıralı adaylar ve ölçümdeki akıbetleri

T3'ün beklenen kazançları tahmindi. "Doğrulandı" = ölçüm kazancı gösterdi; "çürütüldü" =
ölçüm beklenen kazancı göstermedi.

**Boyama yolu**

| # | Aday (T3) | Akıbet | Kaynak |
|---|---|---|---|
| P1 | Tik başına iş evrenden bağımsız (K48) + görünmeyen yüzü boyamama + kare başına bir boyama | **Doğrulandı.** Görüntüleyicide tik işi 30 → 3.000'de +%9 / +%10 (Chromium DPR 3, kısıtsız). Kare başına en çok bir boyama yapı gereği; geçiş / veri çerçevesi 1,00–1,13. PHONE'da akıştaki boşa iş −%20 (16,0 → 12,9 ms / veri çerçevesi) | M15, M14 |
| P2 | H1 statik/canlı katman + kirli dikdörtgen | **Kısmen denendi, kazanç gösterilemedi.** Kirli dikdörtgen ve önek katmanı (M12a), değişen bölge (M12b) doğru çalışıyor ama senaryoda tik başına sahne nadiren değiştiği için ölçülebilir katkı yok. Çekirdekte katman işaretlemesi (H1'in kendisi) yapılmadı | M12a §B, M12b |
| P3 | Harekette düşük ölçek / H7 uyarlanır kalite | **Doğrulandı (sabit ölçek 1 olarak).** M12a'da kapatınca sürüklemede sahne 8,2 → 18,8 ms (Chromium 20×), PHONE 7 → 15 ms. Uyarlanır (süreye bağlı) sürüm yapılmadı | M12a §B, §E |
| P4 | Canvas 2D yolu, metin için glif atlası (H2) | **Doğrulandı.** PHONE yüz 33 → 2 ms, sahne 25 → 5 ms (ana iş parçacığı kaydı); atlas görsel farkı %2,5–10'dan %0,06–1,06'ya indirdi | M12b |
| P5 | simd128 (+ wasm-opt -O3) | **Çürütüldü.** T1: JIT'siz −%9, V8 −%2; M12a: Chromium 20× ve PHONE'da fark gürültü içinde. wasm-opt -O3: JIT'siz −%5 hız, gzip +%4 | T1, M12a §C, M13 |
| P6 | `canvas.width` atamamak, kirli `putImageData`, `willReadFrequently` | İlk ikisi M12a'da alındı (`reuse` ablasyonu §3.3). `willReadFrequently` yalnız M12b atlasında (hata için) kullanıldı; `putImageData` yolunda hız deneyi **ölçülmedi** | M12a, M12b |
| P7 | H3 yüzler sunucuda (görüntü) | **Biçim değişti:** yüzler sunucudan **hazır sahne** (`SceneDesc`) olarak geliyor, piksel değil; cihaz boyuyor. Görüntü (PNG) olarak gönderme yapılmadı | M15 |
| P8 | H6 Worker + OffscreenCanvas | **Ölçülmedi** | — |
| P9 | H4 WebGL2 bileşim / H5 öteleme | **Ölçülmedi.** M12c, kendi köşe/çizim listesinin WebGL2'ye taşınmasını öneriyor | M12c §8 |
| P10 | vello (WebGPU) / vello_gpu | **Seçilmedi.** Yerine çekirdekte üçgenleme + glif atlası + ince WebGPU: soğuk başlangıç 4–9 ms (vello R0: 1.567 ms), paket +8,1 KB gzip (vello R0: +534 KB gzip) | M12c §1 |

**Açılış**

| # | Aday | Akıbet | Kaynak |
|---|---|---|---|
| S1 | Önceden sıkıştırılmış br + gzip, `Accept-Encoding`'e göre | **Doğrulandı, alındı.** Telefonda (HTTP, LAN) yalnız gzip: WASM 4.377.635 → 1.288.744 B | M13 |
| S2 | Görüntüleyici paketi / paketin bölünmesi | **Ölçüldü, uygulanmadı.** Yansısız paket gzip ≈ 0,54 MB (R4/T6); M15'in görüntüleyici biçimine bağlı | M13 (d) |
| S3 | Sunucuda hazırlanmış ilk ekran | **Tahmin, uygulanmadı.** 4×/40 Mbit'te ilk kapsül ≈ 0,3–0,5 s olabilir | M13 (e) |
| S4 | wasm-opt -Oz / -O3 | **Çürütüldü (aktarım için).** Ham −%9–15, gzip +%4 | M13 (b), T1 |
| S5 | Uzun önbellek + özetli dosya adı | **Kısmen:** `no-cache` + `ETag` alındı; dosya adları özet taşımadığı için `immutable` kullanılmadı | M13 (a) |
| S6 | Akışlı derleme | **Doğrulandı** (zaten vardı) | M13 (c) |

**Güncelleme sıklığı**

| # | Aday | Akıbet | Kaynak |
|---|---|---|---|
| U1 | Veri olaylarını birleştir, rAF'te tek boyama | **Alındı.** Bekleme en çok bir kare (PHONE 15–17 ms, Chromium 8,4 ms). 4 adım/sn'de az birleştiriyor (1,00–1,13 geçiş / veri çerçevesi) | M14 |
| U2 | Görünmeyen yüz/katman boyanmaz | **Alındı.** Karar çekirdekte (`faces_to_paint`) | M14 |
| U3 | Ölçüm düzeltmesi: 1 ms nicemleme, 60 Hz, Düşük Güç uyarısı | **Alındı** (yenileme kestirimi, `long` = 1,5 × aralık). Düşük Güç Kipi uyarısı yapılmadı | T2 |
| U4 | Bütçe uyarlaması (H7) | **Yapılmadı** (yerine sabit "harekette ölçek 1", M12a) | M12a |

---

## 3. Denenen yaklaşımlar

### 3.1 Kalibrasyon düğmeleri (T1)

Hedef: PHONE sayılarının iPhone p50'sini (tik 74, yüz 32, sahne 71 ms) ±%30 içinde vermesi.

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| WebKit 26.5, `__XPC_JSC_useJIT=false`, DPR 3 (**PHONE**) | Yüz 31 / tik 12 / sahne 38 ms → oran 0,97 · 0,16 · 0,54. Yalnız yüz ±%30 içinde; sahneli tiklerde tik 45 ms (×0,61). T1 denetiminde %5 içinde yeniden üretildi; son denetimde ve ana oturumun aynı derlemeli tekrarında tik 28–29 ms (×0,38), yüz ve sahne aynı — tik ortancası kararsız (CALIBRATION §4a) | alındı (kalibre ayar) | CALIBRATION §3–4, T1-verify C |
| Düz `JSC_*` ortam değişkenleri | WebContent sürecine ulaşmıyor; JS 147–152 ms, WASM 27–28 ms (etkisiz) | alınmadı | CALIBRATION §2, T1-verify B |
| `__XPC_JSC_useJIT=false` (doğrulama döngüsü) | WASM 28 → 643 ms (23×), JS 147 → 597 ms (4×); WebKit hata kaydındaki telefon oranlarıyla uyumlu (JS 4–5,6×, h3-js ≈ 26×) | alındı | CALIBRATION §2 |
| `__XPC_JSC_useOMGJIT=false` (yalnız BBQ) | WASM 2,3× yavaş | alınmadı | CALIBRATION §2 |
| `__XPC_JSC_useWasmIPInt=false` + BBQ/OMG kapalı | Bir koşuda WebContent'i çökertti; ölçümde kullanılmadı | alınmadı | T1-verify B |
| Chromium CDP 4× (R4'ün "orta sınıf" ayarı) | Yüz 8,7 / sahne 12,0 ms → 0,27 · 0,17: telefonu ≈ 3 kat hafif gösteriyor | alınmadı | CALIBRATION §3 |
| Chromium CDP 16× (**CDP16**) | Yüz 36,8 / sahne 50,4 / tik 14,9 ms → 1,15 · 0,71 · 0,20: yüz ve sahne ±%30 içinde, **yanlış mekanizmayla** (her işi eşit yavaşlatır; yorumlayıcı WASM'ı JS'ten çok daha fazla yavaşlatır, JS ağırlıklı yolu ≈ 4–5 kat kötü gösterir). Yalnız sayısal yedek; JS/WASM payı karşılaştıran kararda kullanılmaz | alınmadı (yedek) | CALIBRATION §0, §4 |
| Chromium CDP 20× | Yüz 48,2 / sahne 65,0 ms → 1,51 · 0,92 | alınmadı | CALIBRATION §3 |
| DPR 1 / 2 / 3 (PHONE motoru) | Sahne 11 / 22 / 38 ms; yüz 9 / 18 / 31 ms. Tarama ≈ a + b·piksel (JIT'siz a ≈ 3,5 ms, b ≈ 33 ns/piksel) | alındı (DPR 3) | CALIBRATION §3, §5.1 |
| `taskpolicy -b` (E çekirdeğe itme) | JIT açık 3,6×, kapalı 6× yavaş; PHONE + bg yüz 140 / sahne 181 ms (4,38 · 2,55); yüklü makinede oynak | alınmadı | CALIBRATION §2–3, §8 |
| `taskpolicy -t 1…5` | Etkisiz (aynı süre) | alınmadı | CALIBRATION §2 |
| WASM SIMD (`+simd128`) | PHONE + SIMD: yüz 31 / sahne 38 (değişmedi); parça ölçümünde tarama JIT'siz −%9, V8 −%2 | alınmadı | CALIBRATION §3, §5.1 |
| wasm-opt `-O3` (binaryen 117) | PHONE: yüz 30 / sahne 36 (−%5); ham −%8, gzip +%4 | alınmadı | CALIBRATION §2–3 |
| WebKit JIT açık, DPR 3 (**PHONE_JIT_EST**) | Yüz 2 / sahne 3 / tik 1 ms; telefonda yüz ≈ 2–4 ms, sahne ≈ 3–6 ms beklenir (**doğrulanmadı**) | öneri (tahmin ayarı) | CALIBRATION §6 |
| `MID_ANDROID_EST` = Chromium CDP 5×, DPR 3 | Tek çekirdek oranından türetildi; 4× ölçümüne göre yüz ≈ 11, sahne ≈ 15 ms. Telefon ölçümü yok | öneri (tahmin) | CALIBRATION §6 |
| Payların ayrıştırılması (log çarpan) | Sahne: piksel %34, motor −%3, JIT kapalı %55, kalıntı %14 (toplam ×101). Yüz: %32 / −%5 / %72 / %1 (×46) | — | CALIBRATION §5.3 |

Kalıntılar: tek ayar üç ölçüyü birden ±%30'a sokmuyor (VERIFY 3 bu hâliyle karşılanmadı;
T1-verify C). Tik, sahne yeniden boyama karışımına bağlı (telefonun karışımı bilinmiyor);
telefonda sahne yolu yüzden orantısız pahalı (sahne/yüz telefonda 2,2, benzetimde 1,2–1,4);
p95'ler kalibre değil.

### 3.2 İş bölümü (M15, K48)

Ölçüm ayarı: başsız Chromium, 390×844, DPR 3, işlemci kısıtı **yok** (T1 kalibrasyonu henüz
`main`'de değildi). Masaüstü sayısıdır; ölçütün konusu bağımsızlık oranıdır.

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| (a) Tam yansı (R4; `?mode=replica`) | Tik işi akışta 2,66 → 9,90 → 128,45 ms/adım (30 / 300 / 3.000); WASM 28,7 → 178,9 → 1.765,0 MB; başlangıç çerçevesi 522,5 KB → 52.413,7 KB; 3.000'de sayfa hazır 27,3 s | alınmadı (başvuru olarak kaldı) | M15 §3.1 |
| (b) **Görüntüleyici, hazır yüz sahnesi** (M15b) | Tik işi akışta 1,66 → 1,72 → 1,81 ms/adım (+%9), açık 5,26 → 5,58 → 5,77 (+%10); WASM 13,8 → 16,6 MB; adım başına ~7 KB (akış) / ~6 KB (açık); sınama 0,8–1,1 ms. Denetimde yeniden üretildi (3.000: akış 1,70, açık 5,51 ms/adım) | alındı (`main`, varsayılan) | M15 §3.1, M15-verify §4 |
| Görüntüleyicide veri aboneliği | Akış ekranında `data: []` / 0 bar; açıkken yalnız açık enstrüman (~121 bar). Tam yansı 3.660 / 36.600 / 366.000 bar tutuyor | alındı | M15 §4 |
| Görüntüleyici yüz/sahne paritesi | Yüz ve açık sahne özetleri tam yansıyla her adımda eşit (yerel ve tarayıcı); RP2 senaryosu viewer varyantı gecikme 0: 186 adım / 0 ihlal, gecikme 25: 211 adım / 0 ihlal | alındı | M15 §5, M15-verify §3, §5 |
| (c) **M15c**: yüz için kompakt veri dilimi, cihazda derleme | Yapılmadı (süre). Beklenti (tahmin): yüz başına bayt düşer (60 bar ≈ 3 KB, sahne ≈ 10–20 KB), cihaz işi yüz derlemesi kadar artar (masaüstü ~1 ms/yüz, telefonda R4'e göre ~30 ms) | ölçülmedi | M15 §6 |
| `refresh_headlines` kirli küme (barındırıcı) | 3.000'de adım ortalaması 19 s idi; 1.000'de 838 → 422 ms/adım, 300'de 29 → 20 ms/adım. Çıktı bayt bayt aynı | alındı | M15 §1, §3.2 |
| Sunucunun cihaz başına işi | 30 / 300 / 3.000: akışta 0,81 / 15,9 / 761 ms/adım (~N^1,7; akış sorgusu her adımda bütün kapsülleri yeniden sıralıyor). Açılış → ilk sahne 3.000'de 1.754 ms (sunucu tarafı) | öneri (akışın kitle başına önbelleği, motorun açık hikâye dizini) | M15 §3.2, Öneriler |
| Artımlı dilim | Dilim açık enstrümanın bütün geçmişini taşıyor (10–100 KB) | öneri | M15 §6 |

### 3.3 Boyama yolları (M12a / M12b / M12c)

Üç yol aynı tabandan (`ea0dd7f`) birbirinden habersiz kuruldu; hepsi `?paint=<ad>` ile,
varsayılan kapalı. **Hiçbiri telefonda ölçülmedi.**

#### Yollar, özet

| Yol | PHONE (ref → yol) | PHONE_JIT_EST | Chromium 20× sahne p50 | Görsel fark | Paket (WASM gzip) | Durum | Kaynak |
|---|---|---|---|---|---|---|---|
| `cpu` — akıllı yazılım çizici | Sahne 25 → **6** ms (p95 30 → 15), yüz 33 → 23, sürükleme uzun kare 0,252 → 0,002; tik p50 14 → 23 (açık soru) | sahne 2 → 0 ms, yüz 2 → 2 | 46,4 → 8,4 | **0 piksel** (2 motor × 49 karşılaştırma) | +10.095 B | (`rp3/m12a-cpu`) | M12a §A, §E |
| `canvas` — Canvas 2D + glif atlası | Yüz 33 → **2**, sahne 25 → **5**, tik p95/p99 48/133 → 22/23, sürükleme uzun kare 0,25 → 0 | yüz 2 → < 1, sahne 2 → < 1 | 46,2 → 8,3 (tarama dahil üst sınır `cx_sync` 13,5) | >2 düzey en kötü %0,30 (DPR 3) / %1,06 (DPR 1) | +9.674 B | (`rp3/m12b-canvas`) | M12b Özet, Ölçüm |
| `gpu` — çekirdekte üçgenleme + ince WebGPU | Yüz 33 → **4**, sahne 25 → **6**, tik p95 49 → 18; **ama düz HTTP'de WebGPU yok, yol `ref`'e düşüyor** | yüz 2 → 1, sahne 2 → 2 | 45,3 → 9,9 | >2 düzey ≤ %0,31, hepsi kenarda (düz alanda 0) | +8,1 KB | (`rp3/m12c-gpu`) | M12c Özet, §4.7 |

Ölçüm uyarıları: `canvas` ve `gpu` yollarında süre ana iş parçacığındaki **komut kaydıdır**;
tarama GPU/raster sürecinde olur ve CDP kısıtı onu yavaşlatmaz (Chromium N× satırları bu
yollar lehine sapar). iPhone'da GPU sürecinin süresi bilinmiyor.

#### M12a `cpu` — teknikler ve ablasyon

Ablasyon: aynı sayfada 8 yol, evre 4 s. Chromium 20× 3 koşu; PHONE tek koşu (yalnız yön).

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| Görünüm + `demultiply`'ın kalkması (`view`): WASM belleğinden doğrudan `Uint8ClampedArray`, opak arka planda dönüşüm yok | Kapatınca (Chromium 20×) yüz 37,6 → 50,2, sahne p95 26,5 → 44,7, kaydırma kare p95 25,8 → 49,3; PHONE sahne 7/14 → 9/26. En büyük tek pay | alındı | M12a §B, §E |
| Harekette ölçek 1 (`reduce`) | Kapatınca sürüklemede sahne 8,2 → 18,8 ms, kare p95 25,3 → 33,9 (Chromium 20×); PHONE drag sahne 7 → 14. Sürerken görüntü bulanık (ürün kararı açık) | alındı | M12a §B, §E |
| Tuvali tutma / "zaten gösteriyor" (`reuse`) | Kapatınca kaydırmada yeniden boyanan yüz 10 → 26, kare p95 25,8 → 41,4 (Chromium 20×) | alındı | M12a §B |
| İlk sürüm, "zaten gösteriyor" kısa devresi olmadan | Kaydırmada 40 yüz boyaması, `cpu_raster` p50 17,8 ms, yüz p50 25,4 (Chromium 20×); düzeltmeyle 10 boyama, kare scroll p95 32,9 → 25,1 | alınmadı (düzeltildi) | M12a "Denenip bırakılanlar" 1 |
| Kirli dikdörtgen (`dirty`) | Bu senaryoda **ölçülebilir katkı yok** (`cpu-nodirty` sahne 7,7 ↔ `cpu` 8,2, Chromium 20×; PHONE'da kapalıyken biraz daha iyi, tek koşu). Yerel sayımda yüzlerin değişen alanı %23,6, grafiğin %43 | alındı (kazanç kanıtlanmadı) | M12a §B, §E |
| Önek katmanı (`layer`, ≥ 16 ilkel) | Ölçülebilir katkı yok (`cpu-nolayer` 7,9); sahne başına +4,2 MB. Yüzlerde hiç kullanılmadı (16'dan az önek ilkeli) | alındı (öneri: kapatılabilir) | M12a §B, "Denenip bırakılanlar" 2 |
| Hepsi kapalı (`cpu-off`) | Sahne 43,3 ≈ `ref` 46,3: kalıcı glif önbelleği ve pixmap yeniden kullanımı tek başına küçük | — | M12a §B |
| Kırpma maskesi / ötelemeli küçük pixmap | Yapılmadı: tiny-skia'da maske memset yolunu kapatıyor, öteleme yuvarlamayı değiştirebilir → bayt eşitliği garanti olmaz | alınmadı | M12a "Denenip bırakılanlar" 3 |
| SIMD (`+simd128`, tiny-skia `simd`) | Chromium 20× `cpu` sahne 8,4 / 25,1 (skaler 8,4 / 23,7); WebKit `ref` 2 → 4 ms; PHONE (E3) `cpu` sahne 5 / 14 / 15, `ref` 25 / 28. Bayt bayt aynı (70/70 tuval). Paket −174 KB ham | alınmadı | M12a §C, §E |
| `draw_prim`'e ayırmanın `ref`'e etkisi | Chromium 20× taban `ref` sahne 43 / 54,9 ↔ dal 46,4: yük farkı içinde | — | M12a §D |
| Bellek | Yerel 28,6 MB (önek katmanı olmadan 24,3); karşılaştırmada WASM `ref` sonunda 37,9 MB, `cpu` sonunda 92,7 MB (~+50 MB, sıra etkisi dahil) | öneri (`keep` düşürmek) | M12a "Paket boyutu, kod, bellek" |

#### M12b `canvas` — denemeler

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| Metin: `run` (metin başına bir `fillText`) | En kötü fark: DPR 1 `feed-tr-dark.face-1` >2 düzey %10,27 (glyphs %9,63) | alınmadı | M12b "Denenen ve kalanlar" |
| Metin: `glyphs` (glif başına `fillText`, başvuru kalemi) | Konum aynı, glifler daha kalın: >2 düzey %2,5–10, metinde en büyük fark 187–194; hız atlasla aynı (Chromium 4× yüz 1,7 / sahne 1,7) | alınmadı (çeşit olarak duruyor) | M12b Görsel fark |
| Metin: **glif atlası** (fontdue pikselleri, `drawImage`) | >2 düzey %0,06–1,06; metin farkı ±1 düzey; hız aynı | alındı | M12b |
| `alpha: true`, `textRendering` (`geometricPrecision` / `optimizeSpeed`) | Hiçbir piksel değişmedi | alınmadı | M12b |
| `desynchronized: true` | Chromium 4× yüz 1,5 / sahne 1,5 (varsayılan 1,6 / 1,5): etkisiz | alınmadı | M12b Çeşitler |
| Düz `Float32Array` komut akışı + dizge tablosu (JSON v1 yerine) | Ayrıştırma yok, tek `memcpy`; JSON v1 ayrıca ölçülmedi | alındı | M12b |
| Toplu yollar kapalı (`nobatch`) | `feed-tr-dark` yüzlerinde piksel farkı yok; süre ölçülmedi | — | M12b |
| Kısmi boyama (değişen bölge) / kapalı (`full`) | Senaryoda süre farkı yok (Chromium 4× yüz 1,3 ↔ 1,6, sahne 1,5 ↔ 1,5); kısmi = tam 14/14 bayt bayt aynı | alındı (kazanç kanıtlanmadı) | M12b |
| `cx_sync=1` (her boyamadan sonra 1 piksel geri okuma: tarama dahil üst sınır) | Chromium 4× sahne 2,3 (ref 8,7), 20× 13,5 (ref 46,7), WebKit 1 (ref 2): tarama dahil `ref`'in ≈ 1/4'ü | — (ölçüm çeşidi) | M12b Çeşitler |
| GPU tamponlu atlas tuvali (WebKit) | **Hata:** başsız WebKit 26.5'te 3 koşudan 2'sinde atlas glifleri kırmızı/mavi kanalları takaslı (`#c8cdd7` yerine `#d7cdc8`) | alınmadı | M12b |
| Atlas tuvali CPU'da (`willReadFrequently: true`) | 3/3 + son tam koşu temiz; Chromium 4× / 20× süre GPU atlasla aynı | alındı | M12b |
| Bellek | JS yığını en çok 79–125 MB → 12–18 MB (Chromium); WASM'da piksel tamponu yok, atlas üst sınırı 2 × 8 MB | — | M12b Bellek |

#### M12c `gpu` — denemeler

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| vello 0.12 + WebGPU (K12'nin seçimi) | Yeniden derlenmedi; R0 ölçümü: soğuk boru hattı 1.567 ms, paket +1,67 MB ham / +534 KB gzip; glifleri kendi taramasıyla çizer (fontdue'den farklı) | alınmadı | M12c §1 |
| Çekirdekte üçgenleme + fontdue `r8unorm` atlası + tek boru hattı, 4× MSAA | Soğuk başlangıç Chromium 6–9 ms, WebKit 4–6 ms; paket +8,1 KB gzip; WASM hazır ve ilk kapsül değişmedi (Chromium 144,2 → 138,1 ms) | alındı | M12c §1, §4.6 |
| Doğrudan kip (her yüz tuvaline kendi `webgpu` bağlamı) | Sunulmuş WebGPU tuvali geri okunamıyor: `drawImage` sonrası `0,0,0,0` (Chromium 151, WebKit 26.5) → `qp.png` ve geçiş anlık görüntüsü boş, ilk görsel fark ölçümü %100; tuval `2d` alamadığı için yedek de olmuyor. Hız: Chromium 4× yüz 1,4 (kopya 1,6) | alınmadı (`?gpu_direct=1` ölçüm için) | M12c §2.1, §4.5 |
| Kopya kipi (paylaşılan ekran dışı WebGPU tuvali → sayfanın 2B tuvaline `drawImage`) | Kopya bedeli Chromium 4×'te ≈ 0,2 ms, WebKit'te ≤ 1 ms; geri okuma, geçiş ve yedek çalışıyor | alındı | M12c §2.2, §4.5 |
| WebKit'te geçici (memoryless) `stencil8` çoklu örnek hedefi | WebKit 26.5 reddediyor ("depth stencil texture is not renderable"), yüzler boş; geçici kullanım yalnız renk hedefinde | alınmadı | M12c §2.3 |
| Başsız Chromium kabuğu (varsayılan `launch()`) | `navigator.gpu` var, adaptör `null` → `ref`, `unsupported` | — (ortam bulgusu) | M12c §5 |
| `--enable-unsafe-webgpu` (SwiftShader) | Yazılım GPU'su; yüz 2,1 → 0,4 ms (yalnız ana iş parçacığı; GPU ölçümü değil) | alınmadı (ölçüm aracı olarak) | M12c §4.2, §5 |
| `--disable-features=WebGPU` | WebGPU'yu kapatmadı; "kapalı" durum başlangıç betiğiyle benzetildi | alınmadı | M12c §5 |
| Düz HTTP, LAN adresi (Arkin'in kurulumu) | `isSecureContext = false`, `navigator.gpu` yok, yol `ref`'e düşüyor (iki motor, ölçüldü); `127.0.0.1`'de çalışıyor | — (belirleyici kısıt) | M12c §4.7 |
| WebGL2 arka ucu (aynı köşe/çizim listesi) | Yapılmadı; ~200 satırlık ikinci JS dosyası tahmini; WebGL güvenli bağlam istemez (cihazda doğrulanmalı) | öneri | M12c §8 |
| Bellek | Açılışta WASM 25,1 → 19,7 MB, JS yığını 25,8 → 7,2 MB (Chromium); GPU belleği hesap: MSAA renk yüz 10,6 MB / sahne 16,8 MB (geçici ekle döşemeli GPU'da sıfır) | — | M12c §4.4 |

### 3.4 Güncelleme sıklığı (M14)

Ölçüm: önce = `main` `0512196`, sonra = `rp3/m14-freq`; 3 koşu medyanı, yalnız `ref`.

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| Kare başına en çok bir boyama (bayraklar çekirdekte birikir, rAF'te tek `repaint`) | Geçiş / veri çerçevesi 1,00–1,13; bekleme en çok 15–17 ms (PHONE) / 8,4 ms (Chromium 4×). **4 adım/sn'de az birleştiriyor**; kazancın çoğu aşağıdaki bayraklardan ve tam yansıdaki çoklu mesajdan | alındı | M14 §1.1, §2.4 |
| Kesin bayraklar: olaysız veri çerçevesi (`{"data":[]}`) `feed`/`scene` kaldırmaz | PHONE görüntüleyici idle: iş / veri çerçevesi 16,0 → 12,9 ms (−%20), tik p50 4 → 0 ms. Chromium 4× idle 5,0 → 3,5 ms (−%30), grafik 5,3 → 4,2 ms (−%21) | alındı | M14 §1.2, §2.1–2.2 |
| Tam yansıda grafik açıkken birleştirme | Chromium 4× `?mode=replica` chart_ticks: 13,5 → 5,8 ms / veri çerçevesi (−%57) | alındı | M14 §2.3 |
| Hangi yüzün boyanacağı çekirdekte (`faces_to_paint`), katman açıkken hiçbiri; sahne yalnız akış sekmesinde | Katman açıkken yüz boyaması iki kodda da 0 (önceki kod da bir kare gecikmeyle aynı sonucu veriyordu) | alındı | M14 §1.2 |
| Kare süreleri | PHONE p95'ler eşit; chart p99 48 → 41, drag p95 33 → 29, drag uzun kare %26,7 → %25,2: boyamanın kendisi belirleyici (M12'nin işi) | — | M14 §2.1, §2.4 |
| Grafik açıkken değişmeyen sahnenin kurulmaması | **Çekirdekte güvenle yapılamıyor**: her adımda açık enstrümanın oluşan barına `patch` geliyor, girdiler gerçekten değişiyor; "bu değişiklik çizileni etkilemez" bilgisi derleyiciden gelmeli. Sahne kurulumu 31 / 31 veri çerçevesi, boyanan 6 | alınmadı; öneri (`qp-grammar` okuduğu bar aralığını döndürsün; PHONE'da ~6 ms/tik, adımların ~%80'i) | M14 §1.3, Öneriler 1 |
| JIT sınaması (`core.jit_probe()`, 1.000.000 adımlık xorshift; eşik 15 ms) | WebKit JIT açık 5 / 2 / 2 ms; PHONE 19–42 ms; Chromium 1× 2,8–4,4; CDP 4× 9,7–11,0; CDP 16× 39,8–43,1. Ayrım payı dar (11 ↔ 19); ≥ 6× kısılmış masaüstü "kapalı" okunur. Telefonda değerleri bilinmiyor | alındı | M14 §3 |
| Özette `secure_context`, `webgpu`, `insecure_context` / `jit_off` uyarıları | Başsız testlerde `secure_context: true`; telefonun LAN adresinde `false` olur | alındı | M14 §3 |

### 3.5 Açılış (M13)

Ölçüm: Chromium 4× + 40 Mbit/s / 20 ms RTT, DPR 3, 5 koşu medyanı; WebKit'te CDP yok
(yerel döngü, kazanç bayttan hesaplandı: 40 Mbit/s'te 1 MB ≈ 0,2 s).

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| Önceden sıkıştırılmış gzip (+ br varsa), `Accept-Encoding`'e göre seçim | WASM 4.377.635 B → 1.288.744 B gzip (−%70,6) / 891.688 B br (−%79,6). Chromium 4× soğuk ilk kapsül 1.907 → 1.230 ms (br), gzip ile 1.310 ms; WASM hazır 1.035 → 369 ms (br) / 452 ms (gzip) | alındı | M13 Özet, Ölçüm 2 |
| `<link rel=preload as=fetch>` kaldırıldı | WebKit preload'ı `fetch()`'e vermiyor, WASM iki kez iniyordu (WebKit 236009, 268370). WebKit'te aktarılan toplam 8.850.801 → 1.334.452 B (−%84,9); 40 Mbit/s'te ≈ 1,77 s → ≈ 0,27 s. Chromium'daki bedel: WASM isteği ≈ 46 ms geç, ilk kapsül +53 ms | alındı | M13 Özet, Ölçüm 1 |
| `no-store` yerine `no-cache` + `ETag` (304) | Chromium sıcak açılışta WASM hazır 1.036 → 125 ms, aktarılan 2.396 B | alındı | M13 (a), Ölçüm 2 |
| `immutable` + özetli dosya adı | Dosya adları özet taşımadığı için kullanılmadı | alınmadı | M13 (a) |
| `wasm-opt -Os` / `-Oz` | Ham 4.377.296 → 3.972.283 / 3.700.818 B (−%9…15), ama gzip 1.288.330 → 1.342.669 / 1.341.138 B (**+%4**), br 891.270 → 945.438 / 951.216 B. Derleme süresine katkısı ≈ 2 ms | alınmadı | M13 (b) |
| `opt-level = "z"` | gzip 1.149.380 B (−%10,8, −139 KB ≈ 28 ms @ 40 Mbit); çalışma hızını etkiler, bütün tik/yüz/sahne ölçümlerini kaydırırdı | öneri (M12/M14 sonrası birlikte ölç) | M13 (b) |
| `panic=abort`, `codegen-units=1`, strip | Zaten öyle (wasm32 abort; `wasm` profili `release`'ten miras; `demo.sh` adsız derliyor; ad bölümü ham dosyanın %22,4'ü) | — (değişiklik yok) | M13 (b) |
| WASM derleme süresi | `WebAssembly.compile` Chromium 1× 3/2/2 ms, 4× 7/5/4 ms, WebKit 6/4/4 ms (bugünkü / -Os / -Oz): **derleme açılışın darboğazı değil** | — | M13 (b) |
| Akışlı derleme ve bağlantının örtüşmesi | `application/wasm` iki motorda geliyor, yedek yok; `/events` WASM'dan önce açılıyor (`connected`, `wasm_ready`'den 20–80 ms sonra) | alındı (zaten vardı) | M13 (c) |
| Paketin bölünmesi (görünüm / yansı) | Kod payı: serde %25,9, alloc %15,6, tiny-skia %8,9, qp_schema %7,5 …; yansısız paket gzip ≈ 0,54 MB (R4/T6), telefonda ≈ 150 ms kazanç. M15 biçimine bağlı, iki wasm-bindgen paketi ister | öneri | M13 (d) |
| İlk ekranın sunucuda hazırlanması | 4×'te `connected → feed` ≈ 710 ms; tahmin: ilk kapsül ≈ 0,3–0,5 s (4×/40 Mbit) | öneri (tahmin) | M13 (e) |
| `main.js` / `style.css` / `index.html` sıkıştırması | ≈ 25 KB ≈ 5 ms | öneri | M13 Öneriler 6 |
| HTTP/1.1 keep-alive | Her durağan istek yeni TCP bağlantısı; telefonda istek başına ≈ 1 RTT | öneri | M13 Öneriler 5 |
| Ön yüklemenin Chromium bedelini geri almak (klasik betikle erken `fetch`) | ≈ 1 RTT; `main.js` değişikliği gerekir | öneri | M13 Öneriler 7 |

Telefon için tahmin (ölçüm değil): WASM hazır 1,8 s → ≈ 0,3–0,5 s, ilk kapsül 3,9 → ≈ 2,4 s;
üst sınıf hedefi (ilk kapsül soğuk ≤ 1,5 s) **yalnız M13 ile tutmuyor** (M13 Özet).

### 3.6 Bellek (T4)

Ölçüm: sentetik pazar, 30 enstrüman, 390×844, **DPR 1**, 64 adım/sn; WASM MB, 3 koşu medyanı.

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| Kök neden: sayfanın yeniden başlaması | Arkin'in masaüstü ölçümünde `start_frame` 836.034 ms; her 250 adımda yeniden başlamada 1.268. adımda 156 MB (Arkin: 1.260. adımda 152 MB) | — (bulgu) | T4 Özet |
| Başlangıç çerçevesinin tipli ayrıştırılması (`core_from_start`) | Kurulum tepesi 1.250. adımda 200,5 → 16,9 MB, 5.000. adımda 791,0 → 67,2 MB | alındı | T4 "Kaynaklar", "Ne sınırlandı" |
| Veri çerçevesinin tipli ayrıştırılması | İleri sarma (tek çerçeve) 250 adım 20,1 → 2,1 MB; 1.250 adım 100,4 → 16,7 MB | alındı | T4 |
| Gönderilen ileti kaydına sınır (1.024–2.048) | Jestsiz oynatmada ~18.000. adımda devreye girer; en çok ~2,7 MB | alındı | T4 |
| Yüz özeti haritasının akışla budanması | ≤ 40 girdi | alındı | T4 |
| Eski çekirdeğin serbest bırakılması (`free()`), T4 yaması (yeni çekirdekten **önce**) | Her 250 adımda yeniden başlama, 2.500. adım: önce 347,5, yalnız free 311,8, yalnız Rust 104,1, ikisi **64,5** MB (Chromium) | alındı (yerleşim aşağıda) | T4 "Önce / sonra" |
| İlk bütünleştirme yerleşimi (`free`, `new Shell` + `metric_inherit` **sonrası**, c769553) | ~2.530. adımda **88,2 MB** (eski ve yeni yansı aynı anda canlı) | alınmadı | T4-verify 3b |
| `metric_detach` → `free` → `new Shell` → `metric_attach` (9170fba) | ~1.265. adımda ≈ 39, ~2.530. adımda 61,4 MB (4 koşu medyanı); yeniden başlamasız akışla ayırt edilmiyor (62,4 MB) | alındı | T4-verify Ek |
| Sürekli oturumun büyümesi (tam yansı) | 5.000 adımda 17,5 → 102 MB (~17,6 KB/adım; Chromium ve WebKit aynı): izleyici günlüğü %60, belge + okuma kopyası %30, barlar/dizinler ~%9. Düzeltmelerle değişmedi (101,9 MB) | öneri (izleyicide `inverse` tutmamak, sınırlı kuyruk; görüntüleyici biçimi cihazdan kaldırır) | T4 "Kaynaklar", Öneriler 2–3 |
| Boyama, grafik, kaydırma birikmesi | Yok: JS yığını 2,0–6,6 MB, DOM 201–202, tuval 41, değişkeler bayt düzeyinde aynı | — | T4 |
| Başlangıç çerçevesinin bütün olay geçmişini taşıması (tam yansı) | 5.000. adımda 150.000 olay, 22 MB; görüntüleyicide yok (`events: []`, M15) | öneri (`?mode=replica` için) | T4 Öneriler 5, M15 §3.1 |

### 3.7 Ölçüm düzeneği (T2)

| Yaklaşım | Sonuç | Durum | Kaynak |
|---|---|---|---|
| Uzun kare eşiği sabit 16,7 ms (R4) | 60 Hz tamsayı ms damgalarında (16, 17, 17 …) normal kareleri uzun sayıyordu (telefonda `long_16_7` ≈ 0,4–0,6) | alınmadı | T2 §1a |
| Yenileme kestirimi + `long` = 1,5 × aralık, `very_long` = 2,5 × | Başsız WebKit'te bütün evrelerde `long` 0–0,008 | alındı | T2 §1a |
| Yeni çekirdek ölçümleri devralır (`metric_inherit`), ilk işaret korunur | Açılış sürelerinin dakikalar sonra yeniden yazılması bitti | alındı | T2 §1b |
| Karşılaştırmada sürükleme tik akışından önce | Yakınlaştırılmış aralık yeni barları izlemiyor, tikler sahneyi neredeyse hiç boyatmıyordu (8 s'de 4–5 sahne) | alınmadı (sıra ters çevrildi) | T2 §3 |
| Sürüklemeden önce yakınlaştırmasız | Sürükleme sınırda kaldı (Chromium'da 8 s'de 9 sahne boyaması) | alınmadı | T2 §3 |
| Kare başına 1–2 px sürükleme | Chromium 9 / WebKit 456 sahne boyaması (yuvarlama kalanı atılıyor); 12 px sabit adımla Chromium 128, WebKit 120–122 | alınmadı (12 px seçildi); ürün hatası adayı öneri | T2 §3, Öneriler 1 |
| Yollar aynı sayfada sırayla | Sonraki yol ısınmış JIT ve büyümüş WASM belleğiyle başlıyor (ref 23,0 → fixture 42,2 MB); bellek karşılaştırması sıraya bağlı | alındı (sınırlı); öneri: taze sayfa / ters sıra | T2 §3, M12a/M12b Öneriler |

---

### 3.8 Bütünleştirme ve birlikte ölçüm (S)

| Yaklaşım | Sonuç (ölçüm, ayar) | Durum | Kaynak |
|---|---|---|---|
| Yolları görüntüleyici kipi ve M14 tek boyama geçişi üstünde birleştirmek | Yolların sahneyi `ShellCore::face()/scene()`'den alması sayesinde görüntüleyici için değişiklik gerekmedi; yolun kendi yeniden boyama isteği (yazı tipi, GPU aygıtı, `cpu` hareket sonu) bir sonraki geçişe ertelendi | alındı | S §1 |
| Yol sırasının etkisi (ileri / ters, 3'er koşu) | PHONE'da ≈1 ms, Chromium'da ≤ %15 fark; M12a'nın "`cpu` tik `ref`'ten sonra yüksek" sorusu yeniden üretilmedi | ölçüldü | S §3 |
| "Değişti mi" sınaması: JSON `scene_digest` | PHONE grafik açıkken kare başına 7,0 (ref) / 7,6 ms (canvas) — ölçüldü (`change_check` aralığı) | **bırakıldı** | S §1.5 |
| "Değişti mi" sınaması: `scene_fingerprint` (SipHash 64, bütün piksel alanları, serileştirme yok); `view_digest` aynı | kontrol 7,6 → 0,7 ms/kare; canvas tik+boyama 14,4 → **7,3** ms/kare (p95 19–20 → 11); ref'in akış ekranında yüz yeniden boyaması 12 → 4 (JSON, piksele girmeyen alanlarda da değişiyordu) | alındı | S §1.5, §6; `S-verify.md` |
| Kalan maliyet (PHONE, canvas, grafik açık) | sahne kurma 3,2 + kontrol 0,7 + boyama 0,3 + kabuk (açık katman JSON'u, metin, kanıt DOM'u) ≈2,4 ms/kare | öneri: A36, katman metnini yeniden kurmamak | S §7 |
| Varsayılan yol `canvas`, `ref` yedek; verify altınları `?paint=ref` ile | verify 18/18; toleranslı test `paint-canvas.mjs`, yol bağımsızlığı `paint-paths.mjs` | alındı (öneri Ö53) | S §1 |

## 4. Açık sorular ve gerçek cihazda doğrulanacaklar

1. **Aynı iPhone'da HTTPS ölçümü** (güvenli bağlam, JIT açık). Arkin'in koşusunun JIT'siz
   olduğu güçlü bir çıkarım (PHONE yüzü %5 içinde veriyor), ama doğrudan kanıt değil
   (T1-verify A). `PHONE_JIT_EST` tahmini: yüz ≈ 2–4 ms, sahne ≈ 3–6 ms. WebKit'in önerdiği
   "Enable EnhancedSecurity Heuristics" bayrağının kapatılması da telefonda denenmeli.
2. **Hangi kipte hedef?** Kilit Modu kullanıcıları ve her `http://` sayfa JIT'siz kalır;
   ürün hedefinin JIT'siz kipte bir tabanı olmalı mı (ana akış kararı; CALIBRATION §9).
3. **Sahne kalıntısı ×1,85**: telefonda büyük tamponda piksel başına daha pahalı olan ne
   (bellek bant genişliği, tuval yüklemesi, termal)? Masaüstünde hiçbir düğme bunu üretmedi.
4. **`canvas` ve `gpu` yollarının GPU süreci süresi** iPhone'da ölçülmedi; benzetimde yalnız
   ana iş parçacığı kaydı ölçüldü. Telefonda ölçüm ekranının **Karşılaştır**'ı ile.
5. **WebKit atlas renk takası** (M12b) iPhone'da görülüyor mu; `willReadFrequently` ile CPU
   atlası telefonda ek yükleme bedeli getiriyor mu.
6. **WebGPU düz HTTP'de yok**: `gpu` yolunun kazancı yalnız HTTPS'te, orada JIT de açık
   (`ref` zaten 2–3 ms). Chrome-on-iOS'ta `navigator.gpu`'nun açık olduğu doğrulanmadı.
   WebGL2 arka ucunun güvenli bağlam istemediği cihazda doğrulanmalı.
7. **Kirli dikdörtgen / önek katmanı / kısmi boyama** canlı veride (son bar her tikte
   değişir) değer taşıyor mu; demo replay'i bunu üretmiyor (M12a, M12b).
8. **M12a PHONE tik p50'si** bazı koşularda `ref`'ten yüksek (23 ↔ 14): sıra / çöp toplama
   etkisi mi? Yolları ters sırada ya da taze sayfada koşturan ölçüm ayırır.
9. **Sürükleme sırasında ölçek 1** (bulanık) kabul edilebilir mi, yoksa ölçek 2 mi (Arkin).
10. **`cpu` yolunun ~+50 MB WASM belleği** telefonun sekme sınırında risk mi (`keep` ve önek
    katmanı ayarlanabilir).
11. **Birleştirme tik patlamalarında** (gerçek canlı akış, hızlı oynatma) ne kazandırıyor;
    4 adım/sn'de ölçülemedi (M14).
12. **JIT sınamasının** telefondaki gerçek değerleri (15 ms eşiği yeterli mi; M14).
13. **Açılış tahmini**: telefonda WASM hazır 1,8 s → ≈ 0,3–0,5 s inmeli; inmezse çift
    indirme + ham paket açıklaması yanlıştır. Ölçüm ekranına WASM'ın Resource Timing kayıt
    sayısı ve `transferSize` eklenirse çift indirme telefonda doğrudan görülür (M13).
14. **İlk kapsül ≤ 1,5 s** hedefi M13 ile tek başına tutmuyor; kalan ≈ 2 s işlemci işinin
    görüntüleyici biçimi ve yeni boyama yollarıyla birlikte ölçülmesi gerekir.
15. **Sunucunun cihaz başına işi** 3.000 enstrümanda 761 ms/adım (M15 §3.2): görüntüleyici
    biçiminin sunucu tarafı ölçeklenmesi açık.
16. **Orta sınıf Android** (Chrome, JIT açık) ölçümü `MID_ANDROID_EST`'i kalibre eder;
    telefon ölçümü yok.
17. Doğrulanamayanlar (T3): iOS 27'de toplam tuval belleği sınırı; Safari'de derlenmiş WASM
    önbelleği; iOS'ta hızlı bellek davranışı; Düşük Güç Kipi 30 fps'in iOS 27'de sürdüğü;
    ısıl kısma eşikleri; JSC katman ısınmasının ilk boyamalara etkisi (1., 10., 100. boyama
    ölçümü yapılmadı).
