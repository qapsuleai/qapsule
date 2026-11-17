# RP3 / T3 — Araştırma: telefonda 2B çizim, açılış ve güncelleme sıklığı

Tarih: 2026-11-17. Yazan: T3 araştırma alt ajanı. Kod değiştirilmedi.

Bu belge `rounds/RP3/RESEARCH.md` için ham kaynaktır. Her iddianın yanında kaynak ve
erişim tarihi var. WebKit kaynak kodundan yapılan okumalar `WebKit/WebKit` deposunun
**`main` dalından 2026-11-17'de** alındı. `main`, iOS 27 ile gelen WebKit'ten ileride
olabilir; o yüzden kaynak koddan çıkan her davranış "iOS 27'de cihazda doğrulanmalı"
sayılır. Doğrulayamadığım şeyler **doğrulanamadı** diye işaretlendi.

## 0. Çıkış noktası: bugünkü yol ve sayılar

Kabukta (`shells/web/main.js`, 183–189. satırlar) her boyamada şu oluyor:

```js
canvas.width = w; canvas.height = h;                       // her seferinde
canvas.getContext('2d').putImageData(new ImageData(…, w, h), 0, 0);
```

Arkin'in iPhone ölçümü (K47; iOS 27, Chrome-on-iOS yani WebKit/JSC, DPR 3): sahne
1170×900 cihaz pikseli (≈1,05 M piksel, 4,2 MB RGBA) 71 ms, yüz 32 ms, tik p50 74 ms,
ilk kapsül 3,9 s (WASM hazır 1,8 s). Masaüstü (Chrome 156, ölçek ~0,8–1): 2,3–2,6 ms.

Yerel ölçüm (bu araştırmada, karalama dizininde, `shells/web/pkg/qp_wasm_bg.wasm` kopyası):

| Biçim | Bayt | Ham dosyaya göre |
|---|---:|---:|
| Ham | 4.377.168 | 100 % |
| gzip -9 | 1.288.334 | 29,4 % |
| gzip -6 | 1.295.101 | 29,6 % |
| zstd -19 | 957.219 | 21,9 % |
| **brotli -q 11** | **891.179** | **20,4 %** (gzip'ten %31 küçük) |

Bölüm boyutları (elle yazılmış bölüm ayrıştırıcıyla): `code` 3.992.563 B (12.430
fonksiyon), `data` 366.455 B (gömülü yazı tipi burada), `target_features` bölümü var
(simd128 yok; görev metnindeki bilgiyle uyumlu). Kodun %91'i makine kodu: paket büyüklüğü
yazı tipinden değil, cihaza giren hesap kodundan (`replica` özelliği) geliyor. K12'deki
spike çizicisi 363 KB gzip'ti; bugün 1,29 MB.

Kaba bir ayrıştırma (varsayım; T1 ölçecek): telefon/masaüstü farkı ≈30×. Piksel sayısı
tek başına ≈9–14× açıklıyor (DPR 3 ↔ 0,8–1). Kalan ≈2–3× için adaylar: JSC'de SIMD'siz
skaler tiny-skia, ilk karelerin yorumlayıcı/BBQ katmanında koşması (§4), `putImageData`'nın
GPU sürecine kopyası (§1).

---

## 1. WebKit/iOS tuval: `putImageData`, boyut atama, alternatifler, bellek sınırları

### 1.1 iOS'ta 2B tuval GPU sürecinde ve hızlandırılmış

- WebKit tercihi `UseGPUProcessForCanvasRenderingEnabled`: `ENABLE(GPU_PROCESS_BY_DEFAULT)`
  olan platformlarda varsayılan `true` (Cocoa/iOS dahil). Yani tuval çizimi WebContent
  sürecinde değil, ayrı GPU sürecinde yapılır; çizim komutları ve piksel tamponları süreçler
  arası (IPC, paylaşımlı bellek) gider.
  Kaynak: `Source/WTF/Scripts/Preferences/UnifiedWebPreferences.yaml`, satır ~6807
  (https://github.com/WebKit/WebKit/blob/main/Source/WTF/Scripts/Preferences/UnifiedWebPreferences.yaml, erişim 2026-11-17).
- `CanvasUsesAcceleratedDrawing`: `PLATFORM(IOS_FAMILY) && !SIMULATOR` için `true`.
  Aynı dosya, satır ~1540.
- 2B bağlamın tamponu `RenderingMode::Accelerated` olarak ayrılır; **istisna:
  `willReadFrequently: true`** verilirse `Unaccelerated`:
  `renderingMode = !willReadFrequently() && canvasBase().shouldAccelerate() ? Accelerated : Unaccelerated`.
  Kaynak: `Source/WebCore/html/canvas/CanvasRenderingContext2DBase.cpp`,
  `allocateImageBuffer()` (https://github.com/WebKit/WebKit/blob/main/Source/WebCore/html/canvas/CanvasRenderingContext2DBase.cpp, erişim 2026-11-17).
- `putImageData` → `buffer->putPixelBuffer(pixelBuffer, sourceRect, destOffset)`; kirli
  dikdörtgen argümanları (`dirtyX, dirtyY, dirtyWidth, dirtyHeight`) destekleniyor ve yalnızca
  o bölge gönderilir. Aynı dosya, `putImageData` (~3240. satır).

Çıkarım (kaynak koddan; cihazda **doğrulanamadı**): her `putImageData` çağrısı 4,2 MB'lık
RGBA'yı (öncesinde düz alfa → önçarpımlı BGRA dönüşümü dahil) GPU sürecindeki
IOSurface'a taşır. Masaüstü Chrome'da da benzer bir yükleme var ama piksel sayısı 9–14×
küçük. Bu kopyanın 71 ms'in ne kadarı olduğu bilinmiyor; **T1 ölçmeli**: aynı tampona
Rust'ta çizmeden yalnız `putImageData` çağırmak.

### 1.2 `canvas.width = w` her boyamada ne yapıyor?

HTML standardı: width/height özniteliği atandığında (aynı değer olsa bile) bit eşlem ve
bağlam başlangıç durumuna sıfırlanır. WebKit `main`'de:

- `HTMLCanvasElement::didUpdateSizeProperties()` → `sizeChanged = oldSize != newSize`;
  bağlama `didUpdateCanvasSizeProperties(sizeChanged)` çağrılır.
- **Boyut değişmediyse** tampon serbest bırakılmaz; durum yığını sıfırlanır ve
  `context.clearRect(tüm tuval)` çağrılır. **Boyut değiştiyse** `m_buffer = nullptr` —
  bir sonraki çizimde yeni (hızlandırılmış) tampon ayrılır.
  Kaynak: `Source/WebCore/html/HTMLCanvasElement.cpp` (~673. satır) ve
  `CanvasRenderingContext2DBase.cpp` (~366. satır), erişim 2026-11-17.

Çıkarım: bugünkü kod her boyamada gereksiz bir tam-tuval `clearRect` + durum sıfırlaması
yapıyor (hemen ardından `putImageData` zaten her pikseli yazıyor). Beklenen kazanç küçük
(GPU'da 1 M piksel temizleme; tahmini < 1–2 ms, **doğrulanamadı**), risk sıfır: boyut
değişmedikçe atama yapmamak. Boyut her değiştiğinde (döndürme, yakınlaştırma, düşük
ölçek kipi) yeni IOSurface ayrılır; "hareket sırasında düşük ölçek" denenirken bu bedel
ölçüme girmeli (iki ayrı tuval tutmak bir çözüm).

### 1.3 Alternatifler

| Yol | Destek (iOS) | Ne kazandırır | Not |
|---|---|---|---|
| `ImageBitmapRenderingContext.transferFromImageBitmap` | Safari 11.1 / iOS 11.3'ten beri; Baseline 2022 ([caniuse](https://caniuse.com/mdn-api_imagebitmaprenderingcontext_transferfromimagebitmap), [web-features](https://web-platform-dx.github.io/web-features-explorer/features/imagebitmaprenderingcontext), erişim 2026-11-17) | Bit eşlemin sahipliğini tuvale devreder; ek kopya olmayabilir | `createImageBitmap(ImageData)` yine de pikselleri bir kez kopyalar/dönüştürür ve eşzamansızdır; `putImageData`'dan hızlı olduğu **doğrulanamadı**. Firefox'ta `transferToImageBitmap`'in kopya yaptığı bildirilmiş ([Bugzilla 1788206](https://bugzilla.mozilla.org/show_bug.cgi?id=1788206)) |
| `OffscreenCanvas` (2B) | Safari / iOS 16.4'ten beri (2023-03-27); Baseline "widely available" 2025-09-27 ([web-features](https://web-platform-dx.github.io/web-features-explorer/features/offscreen-canvas), erişim 2026-11-17) | Taramayı ve `putImageData`'yı ana iş parçacığından bir Worker'a taşır | Toplam işi azaltmaz; ana iş parçacığını (kaydırma, dokunma) serbest bırakır. Worker'da ikinci bir WASM örneği gerekir (bellek ×2 ya da SharedArrayBuffer) |
| `OffscreenCanvas` + WebGL (Worker) | Safari 17+ ([WebKit bug 253431](https://bugs.webkit.org/show_bug.cgi?id=253431) yorumları) | Worker'da GPU | 16.4–16.6'da yalnız 2B vardı; özellik sınaması Worker içinde `getContext` ile yapılmalı |
| `willReadFrequently: true` | Safari / iOS 18'den beri (2024-09-16) ([web-features](https://web-platform-dx.github.io/web-features-explorer/features/canvas-2d-willreadfrequently)) | WebKit'te tamponu hızlandırılmamış (CPU) yapar (§1.1) | Yalnız `putImageData` yapan tuvalde IOSurface yerine CPU tamponu: daha ucuz mu pahalı mı **doğrulanamadı** (bileşime yine GPU'ya çıkar). Tek satırlık deney |
| `desynchronized: true` | 2B için Safari / iOS 15+ ([web-features](https://web-platform-dx.github.io/web-features-explorer/features/canvas-2d-desynchronized)); WebGL için Safari'de yok | Düşük gecikme ipucu (Chrome'da DOM bileşiminin dışında ön tampona yazma; [Chrome blog](https://developer.chrome.com/blog/desynchronized)) | WebKit'in bu ipucuyla gerçekte ne yaptığı **doğrulanamadı**; boyama maliyetini düşürmesi beklenmez |
| WebGL2 `texSubImage2D` | Var | Yalnızca değişen bölgeyi dokuya yükler; bileşim GPU'da | `putImageData(kirli dikdörtgen)` ile aynı kazanç, daha çok kod |

### 1.4 iOS tuval bellek ve alan sınırları

- Alan sınırı: WebKit `main`'de `maxCanvasArea()` iOS için **8192×8192 = 67 M piksel**
  (masaüstünde 16384²). Aşılırsa konsolda "Canvas area exceeds the maximum limit" ve
  bağlam yok. Kaynak: `Source/WebCore/html/CanvasBase.cpp` (~119. satır), erişim 2026-11-17.
  Eski yazılarda geçen 16.777.216 piksel (4096²) eski sürümlerden
  ([pqina](https://pqina.nl/blog/canvas-area-exceeds-the-maximum-limit/)).
- Toplam tuval belleği: tarihsel olarak 224 / 256 / 384 MB (iOS 15) gibi cihaza bağlı
  sınırlar bildirildi; aşılınca yeni tuvallerde `getContext('2d')` `null` döner
  ([pqina](https://www.pqina.nl/blog/total-canvas-memory-use-exceeds-the-maximum-limit/),
  [Apple forum 687866](https://developer.apple.com/forums/thread/687866), erişim 2026-11-17).
  iOS 27'deki güncel sayı **doğrulanamadı** (`main` kaynağında bu adla bir sınır bulamadım;
  süreç bellek sınırı yine geçerli).
- Pratik: DPR 3'te 390×300 CSS px'lik bir yüz 1170×900 = 4,2 MB. Akışta 20 yüz tuvali =
  84 MB, sahne + geçiş anlık görüntüleri (`snapshot()`, `moveCanvases()`) üstüne biner.
  TradingView lightweight-charts tuvali kaldırırken boyutu 1×1 yapıyor; yorumu: "This
  function fixes the iOS Safari error 'Total canvas memory use exceeds the maximum limit'"
  (`src/gui/canvas-utils.ts`, `releaseCanvas`, https://github.com/tradingview/lightweight-charts, erişim 2026-11-17).
  Bizde ekrandan çıkan yüzlerin ve geçiş tuvallerinin aynı biçimde bırakılması gerekir.

---

## 2. Canvas 2D API: iOS WebKit ve Chromium, metin farkı

- WebKit'te 2B tuval GPU sürecinde, hızlandırılmış (§1.1). Grafik benzeri çizim (birkaç
  yüz `fillRect`/yol/çizgi ve birkaç düzine `fillText`) için kare başına maliyet çağrı
  sayısına bağlıdır, piksel sayısına zayıf bağlıdır. iPhone'da 1170×900 tam bir grafik için
  süre **doğrulanamadı** (yayımlanmış güvenilir bir iOS kıyası bulamadım). uPlot, Canvas 2D
  ile 166.650 noktalık etkileşimli grafiği soğuk başlangıçtan 25 ms'de kurduğunu ve
  sonrasında "~100.000 nokta/ms" ölçeklendiğini söylüyor (Ryzen 7 5850U, Chrome 113)
  ([uPlot README](https://github.com/leeoniya/uPlot), erişim 2026-11-17). Bizim sahnemiz
  yüzlerce ilkel; Canvas 2D yolu iPhone'da birkaç ms aralığında **beklenir, ölçülmeli**.
- `FontFace` ile aynı IBM Plex Mono dosyası yüklenebilir. WebKit metni CoreText ile
  çizer (iOS'ta ipucu/hinting yok, alt-piksel konumlama var); fontdue kendi kapsama
  (coverage) taramasını yapar, ipucu kullanmaz. fontdue'nun belgesi tarama yöntemini
  ayrıntılı anlatmıyor ([docs.rs/fontdue](https://docs.rs/fontdue/latest/fontdue/),
  erişim 2026-11-17). Beklenen görsel fark: glif kenarlarında kenar yumuşatma ve gama
  farkı (±1 px'lik gri tonlar), yatayda alt-piksel kaymalar; yerleşim (glif konumları) kabuk
  yerine çekirdekten gelirse satır/sütun hizası korunur. Bu, M12 için referansla
  **toleranslı** karşılaştırmanın neden zorunlu olduğunu gösteriyor (GOAL değişmezleri).
  Metin farkının büyüklüğü **ölçülmeli** (piksel farkı yüzdesi, maks. fark).
- Mimari sınır (TASKS notu): tarayıcı tuvaliyle boyayan yol yalnızca ilkelleri boyar;
  metin konumu çekirdekten gelir (`fillText` tek glif dizisi, konum verilmiş). Glif başına
  konum vermek gerekirse çağrı sayısı artar; alternatif olarak **çekirdekte fontdue ile
  taranmış glif atlası** bir kez `ImageBitmap`'e konur, metin `drawImage` alt-dikdörtgenleriyle
  basılır: hem referansla aynı glif pikselleri, hem GPU bileşimi (§8, fikir H2).
- `desynchronized` / `willReadFrequently`: §1.3.

---

## 3. WebGPU ve GPU 2B çiziciler

### 3.1 Durum

| Platform | Durum | Kaynak (erişim 2026-11-17) |
|---|---|---|
| Safari 26 (iOS/iPadOS/macOS/visionOS 26) | Varsayılan açık. "WebGPU … is now shipping in Safari 26 beta for macOS, iOS, iPadOS, and visionOS." | [WebKit, WWDC25](https://webkit.org/blog/16993/news-from-wwdc25-web-technology-coming-this-fall-in-safari-26-beta/); [web.dev](https://web.dev/blog/webgpu-supported-major-browsers) |
| WebKit kaynak | `WebGPUEnabled`: `ENABLE(WEBGPU_BY_DEFAULT)` ise `true`; **`disableInLockdownMode: true`** | `UnifiedWebPreferences.yaml` ~7516. satır |
| Cihaz | Kapı OS sürümü: "navigator.gpu requires macOS Tahoe, iOS 26, visionOS 26 or later" (WebKit bakımcısı) | [WebKit bug 299237](https://bugs.webkit.org/show_bug.cgi?id=299237) |
| Chrome / diğer iOS tarayıcıları (WKWebView) | Apple forumundaki yanıt: özellik bayrakları yalnız Safari'yi etkiler; "For WKWebView, the feature will work when its enabled by default … WebGPU is enabled by default there [iOS 26 beta]". Chrome-on-iOS'ta `navigator.gpu`'nun bugün açık olduğu **doğrulanamadı** (kaynak kodla tutarlı, cihazda sınanmalı) | [Apple forum 770862](https://developer.apple.com/forums/thread/770862) (Haziran 2025) |
| Erken hatalar | iOS 26.2'de bir motor için "completely unusable", 26.4 beta 1'de düzeldiği söylendi; iPhone 15 / iOS 26.4'te GPU süreci çökme raporu (WebGPU ile ilişkisi belirsiz) | [WebKit bug 301627](https://bugs.webkit.org/show_bug.cgi?id=301627); [Apple forum 822200](https://developer.apple.com/forums/thread/822200) |
| Android Chrome | Chrome 121'den beri, Android 12+ ve Qualcomm/ARM GPU | [Chrome WebGPU overview](https://developer.chrome.com/docs/web-platform/webgpu/overview) (güncelleme 2025-08-11) |

Sonuç: WebGPU, iOS 26+'da (Chrome-on-iOS dahil olması beklenir) var; Kilit Kipi'nde
(Lockdown Mode) kapalı, eski iOS'ta yok, Android'de GPU üreticisine bağlı. **Yedek şart**
(K12'nin "WebGPU → CPU wasm" zinciri geçerli).

### 3.2 Rust tarafı (crates.io API, 2026-11-17)

| Crate | Son sürüm | Tarih |
|---|---|---|
| vello | 0.12.0 | 2026-11-11 |
| vello_gpu (eski adı vello_hybrid) | 0.4.0 | 2026-11-11 |
| vello_cpu | 0.4.0 | 2026-11-11 |
| wgpu | 30.0.2 | 2026-11-12 |
| tiny-skia | 0.12.0 | 2026-02-02 |
| fontdue | 0.9.5 | 2026-10-19 |

- **vello** (hesaplama gölgelendiricili): web'de WebGPU ister; README "web is not currently
  a primary target" ve Safari/Firefox WebGPU'su için "experimental" diyordu
  ([vello README](https://www.github.com/linebender/vello); README tarihi sürümden eski
  olabilir). K12 ölçümü: Area AA deterministik değil, MSAA8/16 deterministik, **soğuk boru
  hattı ~1,5 s** (M4 Max). iPhone'da soğuk derleme süresi **doğrulanamadı**; daha uzun
  olması beklenir. Paket boyutuna etkisi (wgpu + gölgelendiriciler) ayrıca ölçülmeli.
- **vello_gpu 0.4** (CPU'da yol/şerit hazırlığı, GPU'da tarama/bileşim): `webgl` özelliğiyle
  **WebGL2 arka ucu** var; `webgl_bootstrap` gölgelendiricileri WASM yüklenmeden derlemeye
  başlatıyor. Belgeye göre "a solid GPU-accelerated 2D renderer with broad, reliable feature
  support", vello_cpu'dan "slightly less mature"; bazı karışım kipleri ve maskeler panikliyor;
  glif önbelleği deneysel ([docs.rs/vello_gpu](https://docs.rs/vello_gpu), erişim 2026-11-17).
  Bizim için anlamı: WebGPU olmayan cihazda da GPU yolu (WebGL2) mümkün; ama K12'nin
  "ayrı WebGL2 katmanı yok" kararı bunu dışarıda bırakmıştı → bu seçenek açılırsa
  `DECISIONS.md`'ye yeni satır gerekir.
- Diğer: Flutter skwasm/CanvasKit (Skia'nın WASM'ı, WebGL), Rive renderer (WebGL2 tabanlı),
  `femtovg`/`lyon` (WebGL üstünde yol mozaikleme). Bunların iOS'ta bizim sahnemizdeki
  maliyeti **doğrulanamadı**; Skia-WASM (CanvasKit) birkaç MB ek paket getirir.

### 3.3 WebGL2

iOS 15'ten beri her iOS tarayıcısında var (WebKit'te ANGLE-Metal). WebGPU'dan farklı olarak
Kilit Kipi'nde de çalıştığı bilinir — **doğrulanamadı** (kaynakta yalnız WebGPU için
`disableInLockdownMode` gördüm). En basit GPU kullanımı: çekirdek pikselleri CPU'da tarar,
WebGL2 yalnız dokuyu (ya da kirli bölgesini `texSubImage2D` ile) ekrana koyar; ya da
statik katman dokusu + canlı katman dokusu GPU'da bileşir.

---

## 4. JavaScriptCore'da WebAssembly

- **Katmanlar**: IPInt (yerinde yorumlayıcı) → BBQ → OMG. JSC seçenekleri (`main`):
  `thresholdForBBQOptimizeAfterWarmUp = 150`, `thresholdForOMGOptimizeAfterWarmUp = 50000`,
  `useWasmIPInt = ipintEnabledByDefault()`, `useWasmIPIntSIMD = true`,
  `useWasmFastMemory = true` ("32-bit address space with a signal handler to bounds check").
  Kaynak: `Source/JavaScriptCore/runtime/OptionsList.h` (erişim 2026-11-17).
  WebKit Safari 26 duyurusu: "WebAssembly is first evaluated by the in-place interpreter"
  ([WebKit WWDC25](https://webkit.org/blog/16993/news-from-wwdc25-web-technology-coming-this-fall-in-safari-26-beta/)).
  BBQ, OMG'den ≈4× hızlı derler, ≈2× yavaş kod üretir
  ([WebKit, Assembling WebAssembly](https://webkit.org/blog/7691/webassembly/); 2017 yazısı, oran eskimiş olabilir).
  **Anlamı:** ilk birkaç boyamada tarama döngüleri yorumlayıcı/BBQ'da koşar; sıcak döngüler
  sayaçlara göre OMG'ye geçer. Telefon ölçümünün p50'si sıcak, ama ilk kapsül ve ilk sahne
  soğuk koda düşer. **Ölçüm önerisi (T1):** aynı sahneyi 1., 10., 100. kez boyama süresi.
- **SIMD**: WebAssembly SIMD (simd128) Safari 16.4'ten beri
  ([Emscripten SIMD belgesi](https://sources.debian.org/src/emscripten/3.1.69%2Bdfsg-4/site/source/docs/porting/simd.rst));
  Relaxed SIMD Safari'de yok (caniuse: "3.1 – 26.3 Not supported", 26.5 TP de yok;
  [caniuse](https://caniuse.com/wf-wasm-simd-relaxed), erişim 2026-11-17; iOS 27 durumu
  **doğrulanamadı**). tiny-skia'nın simd128 ile kazancı JSC'de **doğrulanamadı**; K12 skaler ve
  simd128 çıktısının bit-eşit olduğunu gösterdi, yani determinizm riski düşük. Beklenti:
  raster işinde 1,5–3× (tahmin; ölçülmeli). Paket boyutu biraz artabilir.
- **Akışlı derleme**: `instantiateStreaming` sunucu `Content-Type: application/wasm`
  göndermezse `TypeError` ile reddedilir ([MDN](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/instantiateStreaming_static)).
  Gösterim sunucusu bunu zaten gönderiyor (`crates/qp-cli/src/demo.rs:310`). Safari 26.6
  akışlı fonksiyonlara `compileOptions` ekledi ([WebKit 26.6](https://webkit.org/blog/18178/webkit-features-for-safari-26-6/), 2026-07-27).
  Sıkıştırılmış yanıtta (`Content-Encoding: br`) akış yine çalışır (açma tarayıcıda).
- **Derlenmiş kod önbelleği**: Safari'nin WASM makine kodunu yüklemeler arasında
  önbelleğe aldığına dair kaynak bulamadım; IndexedDB'de `WebAssembly.Module` saklama hiç
  yayımlanmadı ([WebKit bug 166481](https://bugs.webkit.org/show_bug.cgi?id=166481),
  [Mozilla 1469395](https://bugzil.la/1469395)). Durum: **doğrulanamadı**. Pratik etkisi:
  sıcak açılışta da derleme/yorumlama yeniden olur; IPInt bunu ucuzlatır, ama indirilen
  bayt HTTP önbelleğinden gelmeli (içerik özetli dosya adı + `immutable`).
- **İş parçacıkları**: Safari 15.2'den beri COOP/COEP ile `SharedArrayBuffer`
  ([WebKit 15.2](https://webkit.org/blog/12140/new-webkit-features-in-safari-15-2/)); COEP
  `credentialless` Safari'de yok, `require-corp` gerekir
  ([StackBlitz](https://blog.stackblitz.com/posts/bringing-webcontainers-to-all-browsers/)).
  Paralel tarama (ör. sahneyi şeritlere bölüp iki Worker'da taramak) mümkün, ama tiny-skia
  tek iş parçacıklı; bölme çekirdekte yapılmalı. Determinizm bozulmaz (her şerit bağımsız)
  ama karmaşıklık yüksek. Flutter da çok iş parçacıklı çizimi yalnız bu başlıklarla açıyor
  ([Flutter wasm](https://docs.flutter.dev/platform-integration/web/wasm)).
- **Bellek büyümesi**: `useWasmFastMemory` sinyal tabanlı sınır denetimi için büyük sanal
  alan ayırır; ayrılamazsa sınır denetimli belleğe düşer ve kod yeniden derlenebilir
  ([webkit-dev 2018 tartışması](https://lists.webkit.org/archives/list/webkit-dev@lists.webkit.org/thread/7O22VB27RUYHHFHGFRVPZVIWZZA2N4BD)).
  iOS 27'deki davranış **doğrulanamadı**. WASM belleği küçülmez; T4'ün bellek ölçümünde
  `memory.buffer.byteLength` tepe değeri izlenmeli.

---

## 5. Açılış

- **Sıkıştırma**: yerel ölçüm (§0): brotli-11 891 KB, gzip 1,29 MB, ham 4,38 MB. Bugün
  ham sunuluyorsa brotli aktarılan baytı **%80** düşürür. Önceden sıkıştırılmış dosya
  (`.wasm.br`, `.wasm.gz`) derleme sırasında üretilir, sunucu `Accept-Encoding`'e göre seçer;
  çalışma anında brotli-11 pahalıdır (brotli yavaş sıkıştırır,
  [brotli vignette](https://cran.csiro.au/web/packages/brotli/vignettes/benchmarks.html)).
  Not: Safari brotli'yi yalnız HTTPS'te ilan eder (genel bilgi; **doğrulanamadı**) —
  yerel ağdaki HTTP gösterimde gzip yedeği gerekebilir. Sunucu `Accept-Encoding`'e bakmalı.
- **wasm-opt**: Rust çıktısında ek ≈%10–20 ([Binaryen sunumu](https://docs.google.com/presentation/d/1EOpX47PszpuYOtkFc5XzDiWEPelH1e-LARsOZ4Ax-0w/htmlpresent),
  [Rust-wasm book](https://rustwasm.github.io/book/reference/code-size.html)); wasm-bindgen'den
  **sonra** çalıştırılır. Bu makinede `wasm-opt` kurulu değil. `-O3` hız için, `-Oz` boyut için;
  determinizm açısından `-O` kipleri kayan nokta anlamını değiştirmez (fast-math yok) —
  yine de altın görsel karşılaştırmasıyla sınanmalı.
- **Paketi bölmek**: en büyük kalem bölmekten önce **K48**: cihaz akış için kapsül
  motorunu koşturmayacaksa, bu kod ilk ekranda gerekmez. `code` bölümü 4,0 MB; K12'nin
  yalnız-çizici paketi 363 KB gzip'ti. Görüntüleyici paketi (çizici + oturum + protokol)
  ilk ekran için; açık enstrümanın hesap kodu (göstergeler, aday, yerel sınama) ikinci paket
  olarak açılışta yüklenir. wasm-bindgen'de iki ayrı WASM modülü iki ayrı derleme demektir;
  ortak tipler JSON/bayt sınırından geçer. Beklenen: ilk paket ~0,3–0,5 MB brotli (tahmin;
  `twiggy` ile ölçülmeli).
- **Tembel özellikler**: yazı tipi (data bölümü ~366 KB'nin çoğu) ilk ekranda zaten gerekli;
  bölmeye değmez.
- **Sunucuda hazırlanmış ilk ekran** (K48 ile uyumlu): akıştaki ilk kapsül yüzleri sunucuda
  aynı referans çiziciyle (`qp render`) taranıp PNG/WebP olarak HTML ile gelir; WASM hazır
  olunca tuval devralır. İlk kapsül süresi WASM'dan bağımsızlaşır. Risk: DPR'ye göre üç
  çözünürlük (1/2/3) ya da `srcset`; sunucu işi kapsül başına bir kez (cihaz başına değil).

---

## 6. iPhone'da kare zamanlaması

- **60 Hz sınırı**: WebKit tercihi `PreferPageRenderingUpdatesNear60FPSEnabled` varsayılan
  `true` (yalnız visionOS'ta `false`) — `UnifiedWebPreferences.yaml` ~5131. satır (erişim
  2026-11-17). WebKit hata kaydına göre bu bayrak rAF dahil tüm sayfa güncelleme hızını
  yönetir ([WebKit bug 294338](https://bugs.webkit.org/show_bug.cgi?id=294338)). Kullanıcı
  Safari'nin Özellik Bayrakları'ndan kapatabilir; WKWebView uygulamalarını (Chrome-on-iOS)
  etkilemediği bildiriliyor. **Sonuç:** ProMotion iPhone'da bile Chrome-on-iOS'ta rAF ≈60 Hz;
  kare bütçesi 16,7 ms.
- **Zamanlayıcı çözünürlüğü**: WebKit `Performance.cpp`'de `static Seconds timePrecision { 1_ms };`
  `performance.now()` ve rAF zaman damgası bu çözünürlüğe indirgenir. Çapraz köken
  yalıtımına bağlı bir gevşetme kodda yok (`allowHighPrecisionTime()` ayrı bir yol; kimin
  çağırdığı **doğrulanamadı**, büyük olasılıkla test düzeneği). Chrome ise yalıtılmış
  sayfada 5 µs'ye iner ([Chrome blog](https://developer.chrome.com/blog/cross-origin-isolated-hr-timers)).
  **Sonuç:** telefondaki tamsayı ms damgaları beklenen davranış. 60 Hz'de kare aralıkları
  16 ya da 17 ms görünür; "> 16,7 ms = uzun kare" eşiği normal karelerin yaklaşık üçte birini
  uzun sayar → kabul ölçütü 9. Öneri: eşik = ölçülen medyan kare aralığının 1,5 katı
  (60 Hz'de 25 ms) ya da "kaçırılan vsync sayısı = round(aralık / medyan) − 1".
  Boyama süreleri de 1 ms nicemli: 0,9 ms'lik masaüstü yüz süresi telefonda 0 ya da 1 görünür;
  kısa işler için N tekrar ortalaması gerekir.
- **Düşük Güç Kipi**: rAF 30 fps'e kısılır; WebKit mühendisi: "This is intentional"
  ([WebKit bug 215745](https://bugs.webkit.org/show_bug.cgi?id=215745), 2020). iOS 27'de hâlâ
  böyle olduğu **doğrulanamadı**. Çapraz köken iframe'de de 30 fps kısıtı bildirildi
  ([Popmotion](https://popmotion.io/blog/20180104-when-ios-throttles-requestanimationframe)).
  Otomatik karşılaştırma ölçümü Düşük Güç Kipi açıkken anlamsız olur; ölçüm ekranı medyan
  kare aralığı ≈33 ms ise uyarı vermeli.
- **Isıl kısma**: iPhone uzun yükte CPU/GPU saatini düşürür; kaynakla sayısal bir eşik
  bulamadım (**doğrulanamadı**). Üç dakikalık otomatik karşılaştırmada yolların sırası
  dönüşümlü olmalı (ABBA) ki ısınma tek bir yolu cezalandırmasın.

---

## 7. Benzer ürünler ne yapıyor

- **TradingView lightweight-charts** (kaynak, `master`, erişim 2026-11-17):
  - Bölme başına **iki tuval**: ana tuval (arka plan, ızgara, seriler, etiketler) ve onun
    üstünde `zIndex: 2` "top canvas" (artı imleci vb.) (`src/gui/pane-widget.ts` 116–130).
  - Geçersizlik düzeyleri `None < Cursor < Light < Full`; `paint(type)` içinde
    düzey `Cursor` ise ana tuval hiç boyanmaz — imleç hareketi yalnız üst tuvali
    boyar (`src/model/invalidate-mask.ts`, `pane-widget.ts` ~474–520).
  - Geçersizlikler birleştirilir (`mergePaneInvalidation`: max düzey) ve **rAF başına bir
    çizim** planlanır (`chart-widget.ts`: `_drawPlanned`, `_invalidateMask`).
  - Gizli bölmede ölçekler güncel tutulur, "only drawing is skipped".
  - HiDPI: `fancy-canvas` ile `device-pixel-content-box` ResizeObserver; bit eşlem boyutu
    fiziksel piksel ([v4 sürüm notu](https://www.tradingview.com/blog/en/release-notes-for-lightweight-charts-4-0-36498),
    [web.dev](https://web.dev/articles/device-pixel-content-box)). Tam DPR'de çiziyor;
    düşük çözünürlüğe inmiyor.
  - iOS tuval belleği için `releaseCanvas` (§1.4).
- **uPlot**: tek Canvas 2D, animasyon yok; akış performansı sorunluysa güncelleme sıklığını
  düşürmeyi öneriyor ([README](https://github.com/leeoniya/uPlot)).
- **Flutter web**: Safari'de varsayılan hâlâ `dart2js + CanvasKit` (WebGL üstünde Skia-WASM);
  skwasm (dart2wasm) Safari 26.2+'da yalnız `wasmAllowList: { webkit: true }` ile; çok iş
  parçacıklı çizim COOP/COEP ister ([Flutter wasm](https://docs.flutter.dev/platform-integration/web/wasm), erişim 2026-11-17).
  Ders: büyük bir ekip bile iOS'ta WASM-GPU yolunu varsayılan yapmakta temkinli.
- **Figma** (2025-09-18): C++ çizici Emscripten ile WASM; WebGPU'ya geçti ama **oturum
  ortasında WebGL'e düşebilen** dinamik yedek kurdu, düşüş oranı yüksek cihazları kara listeye
  aldı; kazanç "on some classes of devices", diğerlerinde nötr; Safari'den söz edilmiyor
  ([Figma blog](https://www.figma.com/blog/figma-rendering-powered-by-webgpu/)). Aynı çizici
  sunucu ve test için yerel derleniyor — bizim "referans çizici başsız + ekranda başka yol"
  modelimizle aynı.
- **Rive, Google Maps**: GPU (WebGL2/WebGPU) çiziciler; mobil web'deki iç stratejileri için
  birincil kaynak bulamadım (**doğrulanamadı**). Bilinen genel teknikler (harita karoları gibi
  önbelleklenmiş katmanlar, harekette düşük ayrıntı) bu ürünlerle ilişkilendirilerek burada
  kanıt sayılmadı.
- **Genel teknikler ve bizdeki karşılığı**:
  | Teknik | Bizde |
  |---|---|
  | Harekette düşük çözünürlük | DPR 3 → 1,5 kaydırma/sürükleme sırasında (piksel ¼), durunca tam |
  | Katman önbelleği (statik + canlı) | Izgara, eksenler, geçmiş barlar bir bit eşlemde; son bar, fiyat çizgisi, işaret, artı ayrı küçük katmanda |
  | Kirli dikdörtgen | `putImageData(img, 0,0, dx,dy,dw,dh)` ya da canlı katmanı yalnız değişen sütun için |
  | Kare başına bir boyama | Veri olayları birikir, rAF'te tek boyama (lightweight-charts modeli) |
  | Görünmeyeni boyamama | `IntersectionObserver` zaten var; tikte görünmeyen yüzler atlanmalı (M14) |

---

## 8. Bizim duruma özgü karma fikirler (denemeye değer)

Hepsi "sahne tarifi tek kaynak" (K24) ve "referans çizici kalır" değişmezine uyar: ekrandaki
yol sahne tarifinden çizer, referans başsız/altın için kalır, ikisi toleransla karşılaştırılır.

- **H1 — Statik/canlı katman ayrımı çekirdekte.** Sahne tarifine her öğe için "katman"
  (statik/canlı) bilgisi çekirdekte eklenir (yerleşim çekirdekte kalır). Statik katman tiny-skia
  ile bir kez taranır, değişince yeniden; canlı katman (son bar + fiyat etiketi + işaret)
  küçük bir şerit olarak taranıp ikinci tuvale ya da kirli dikdörtgene konur. Tik başına
  taranan piksel, sahnenin %3–10'u. Referans çizici aynı iki katmanı üst üste koyarak bugünkü
  görüntüyü üretir (bayt eşitliği bile korunabilir).
- **H2 — Canvas2D geometri + çekirdek glif atlası.** Dikdörtgen/yol/çizgi tarayıcının
  hızlandırılmış 2B tuvaliyle; metin, fontdue ile çekirdekte bir kez taranmış glif atlasından
  (`ImageBitmap`) `drawImage` ile. Metin pikselleri referansla aynı kalır; görsel farkın ana
  kaynağı (yazı tipi) ortadan kalkar; geometri farkı yalnız kenar yumuşatmada.
- **H3 — Yüzler sunucuda, grafik cihazda.** K48 gereği akış kapsülleri "otoritenin iddiası";
  yüzleri sunucu referans çiziciyle 1×/2×/3× tarayıp görüntü olarak verebilir (önbelleklenir,
  CDN). Cihaz yüz için hiç tarama yapmaz; yalnız açık enstrümanın grafiğini çizer. Yüzde
  canlı fiyat gösteriliyorsa yalnız o küçük bölge cihazda (H1 gibi) üstüne basılır.
- **H4 — GPU yalnız bileşim için.** Taramayı CPU'da bırak (deterministik, referansla aynı
  kod), WebGL2 yalnız katman dokularını bileşsin ve kaydırmada dokuyu kaydırsın (yeniden
  tarama yok). WebGPU gerekmez; Kilit Kipi ve eski iOS sorunu yok.
- **H5 — Kaydırmada yeniden tarama yerine öteleme.** Grafik yatay kaydırılırken önceki kare
  bit eşlemini `drawImage` ile kaydırıp yalnız açılan şeridi taramak (eski "blit + şerit"
  tekniği). Tarama ölçekle değişirse (yakınlaştırma) tam tarama gerekir.
- **H6 — Worker'da tarama.** OffscreenCanvas + Worker'da ikinci WASM örneği yalnız çizici
  ile (küçük paket); ana iş parçacığı kaydırmayı ve dokunmayı akıcı tutar. Boyama süresini
  düşürmez, kare p99'unu düşürür.
- **H7 — Uyarlanır kalite.** Ölçülen boyama süresi bütçeyi (≈8 ms) aşarsa bir sonraki
  hareket karesinde ölçeği düşür; durunca tam ölçekte bir kez boya. Determinizm: kalite
  kararı yalnız ekran yolunda; referans etkilenmez. Kararın girdisi zaman olduğundan bu,
  hesap kodunda değil kabuk/çizim tarafında kalmalı (CLAUDE.md: hesapta rastgelelik/zaman yok).

---

## 9. Sıralı aday listesi

Beklenen kazançlar iPhone sayılarına (sahne 71 ms, yüz 32 ms, tik p50 74 / p95 118 ms,
kaydırma kare p95 50 / p99 179 ms, ilk kapsül 3,9 s, WASM hazır 1,8 s) göre **tahmindir**;
her satırın son sütunu onu doğrulayacak ölçümdür. Sıralama: beklenen kazanç ÷ (risk + iş).

### 9.1 Boyama yolu

| # | Aday | Beklenen kazanç | Risk | Doğrulayan ölçüm |
|---|---|---|---|---|
| P1 | **Tik başına iş evrenden bağımsız (K48) + görünmeyen yüzleri boyamama + kare başına bir boyama** | Tik p95 118 → tek grafik/yüz boyaması (≤ 30–70 ms, P2–P4 öncesi). Diğer her şeyin ön koşulu | Düşük | Tik başına boyanan yüz sayısı ve süre; 30/300/3000 evrende aynı |
| P2 | **H1 statik/canlı katman + kirli dikdörtgen** | Tikte sahne 71 → ~3–8 ms (piksel payıyla orantılı + sabit bedel) | Orta (sahne tarifine katman bilgisi; referansta birleştirme) | Tik başına taranan piksel; tik p95 telefonda; referansla birleştirilmiş görüntü 0 px fark |
| P3 | **Harekette düşük ölçek (DPR 3 → 1,5) / H7 uyarlanır** | Kaydırma/sürükleme karesinde boyama ≈¼ (71 → ~18 ms; P5 ile ~8 ms) | Düşük (görüntü harekette yumuşak; iki tuval tutarak yeniden ayırma bedelinden kaçınılır) | Kaydırma kare p95/p99; boyut değişiminde ilk kare süresi |
| P4 | **Canvas2D yolu (M12-b), metin için H2 glif atlası** | Tam sahne 71 → birkaç ms (**doğrulanamadı**); piksel sayısından büyük ölçüde bağımsız | Orta (görsel fark; mimari testi genişletme; iki çizici bakımı) | Tam sahne boyama süresi telefonda; referansla piksel farkı (%, maks.); kabuk koduna alan mantığı girmediği (arch-check) |
| P5 | **simd128 derlemesi** (+ `wasm-opt -O3` dene) | Tarama 1,5–3× (tahmin): sahne 71 → 25–45 ms | Düşük (K12: skaler↔simd bit-eşit) | Aynı sahne, simd'li/simd'siz, 100 tekrar, telefonda; altın görseller değişmedi |
| P6 | **Boyut değişmedikçe `canvas.width` atamamak; `putImageData` kirli bölge argümanları; `willReadFrequently` deneyi** | 1–5 ms (tahmin) | Çok düşük | `put()` içinde atama/`putImageData` ayrı zamanlaması; `willReadFrequently` açık/kapalı |
| P7 | **H3 yüzler sunucuda (görüntü)** | Yüz 32 → ~0 ms cihazda (görüntü çözme + bileşim) | Orta (sunucu işi, DPR'ye göre üretim, canlı alanlar) | Akış kaydırma kare p95; ağdan gelen bayt; ilk kapsül |
| P8 | **H6 Worker + OffscreenCanvas** | Boyama süresi aynı; kaydırma p99 179 → bütçeye yakın | Orta (iki WASM örneği, bellek) | Kaydırma sırasında uzun kare payı; bellek |
| P9 | **H4 WebGL2 bileşim / H5 öteleme** | Kaydırmada yeniden tarama yok | Orta | Kaydırma kare süresi; WebGL bağlam kaybı davranışı |
| P10 | **vello (WebGPU) / vello_gpu (WebGPU+WebGL2)** | Tam sahne düşük ms olabilir; ama soğuk boru hattı ~1,5 s (masaüstü), paket büyür | Yüksek (iOS WebGPU olgunluğu, Kilit Kipi, determinizm yalnız MSAA, K12'ye yeni satır) | Soğuk ve sıcak ilk kare; paket baytı; Chrome-on-iOS'ta `navigator.gpu`; referansla fark |

### 9.2 Açılış

| # | Aday | Beklenen kazanç | Risk | Doğrulayan ölçüm |
|---|---|---|---|---|
| S1 | **Önceden sıkıştırılmış brotli (+gzip yedeği), `Accept-Encoding`'e göre** | Aktarılan WASM 4,38 MB → 0,89 MB (ölçüldü). WASM hazır 1,8 s'nin indirme payı kadar | Düşük (HTTP'de Safari brotli ilan etmeyebilir → gzip) | Aktarılan bayt (Resource Timing `transferSize`), WASM hazır soğuk/sıcak |
| S2 | **Görüntüleyici paketi (K48): kapsül motoru ilk pakette yok; açık enstrüman hesabı ikinci paket** | `code` 4,0 MB'nin büyük kısmı ilk ekrandan çıkar; ilk paket ~0,3–0,5 MB brotli (tahmin) | Orta (iki modül arası sınır) | `twiggy`/bölüm boyutu; WASM hazır; ilk kapsül |
| S3 | **H3/§5 sunucuda hazırlanmış ilk ekran** | İlk kapsül WASM'dan bağımsız: 3,9 s → HTML + görüntü süresi (~<1 s yerel ağda, tahmin) | Orta | İlk kapsül soğuk; görüntü baytı |
| S4 | **`wasm-opt -Oz` (ya da `-O3` hız için)** | Ek %10–20 boyut | Düşük (altın görselle sına) | Boyut; boyama süresi değişimi |
| S5 | **Uzun önbellek + özetli dosya adı** | Sıcak açılışta indirme 0 | Düşük | Sıcak açılış `transferSize` = 0 |
| S6 | **Akışlı derleme** (zaten `application/wasm`) | Var olanı korumak | — | `instantiateStreaming` yolunun kullanıldığı (konsol/izleme) |

### 9.3 Güncelleme sıklığı

| # | Aday | Beklenen kazanç | Risk | Doğrulayan ölçüm |
|---|---|---|---|---|
| U1 | **Veri olaylarını birleştir, rAF'te tek boyama** (lightweight-charts modeli) | Tik patlamalarında boyama sayısı ≤ kare sayısı; tik p95 düşer | Düşük (görünen gecikme ≤ 1 kare ≈ 17 ms) | Kare başına boyama sayısı (≤ 1); olay→ekran gecikmesi |
| U2 | **Görünmeyen yüz/katman boyanmaz; görünür olunca bir kez** | Tikte iş = görünür yüz sayısı × (P7 ile ≈0) | Düşük | Tik başına boyanan yüz sayısı |
| U3 | **Ölçüm düzeltmesi: 1 ms nicemleme, 60 Hz sınırı, Düşük Güç Kipi uyarısı** | Yanlış "uzun kare" sayımı biter (ölçüt 9) | Yok | 60 Hz masaüstünde boş sayfada uzun kare payı ≈0 |
| U4 | **Bütçe uyarlaması (H7)** | Hareketin p99'u bütçede | Düşük | Kaydırma p99; kalite geçiş sayısı |

### 9.4 Önerilen ölçüm sırası (T1/T2 için)

1. `put()` içini parçala: Rust tarama / `canvas.width` / `putImageData` ayrı süreler (1 ms
   nicemleme yüzünden 20 tekrar ortalaması).
2. Aynı sahneyi 1., 10., 100. kez boya (JSC katman ısınması).
3. DPR 1/2/3 ile sahne süresi (piksel doğrusallığı).
4. simd128 açık/kapalı.
5. Sonra P1/U1/U2 (iş bölümü ve sıklık), en son boyama yolları (P2–P4, P10).

## Doğrulanamayanlar (özet)

- iOS 27'de toplam tuval belleği sınırı; `putImageData`'nın gerçek maliyeti; `willReadFrequently`
  ve `desynchronized`'in iOS'taki etkisi.
- Chrome-on-iOS'ta `navigator.gpu`'nun açık olduğu (kaynak kod ve Apple yanıtıyla tutarlı).
- iPhone'da vello soğuk boru hattı süresi; Canvas 2D ile tam sahne süresi.
- Safari'de WASM derlenmiş kod önbelleği; iOS'ta hızlı bellek (fast memory) davranışı;
  iOS 27'de Relaxed SIMD.
- Düşük Güç Kipi 30 fps kısıtının iOS 27'de sürdüğü; ısıl kısma eşikleri.
- Safari'nin HTTP (HTTPS olmayan) üzerinde brotli ilan edip etmediği.
- Rive ve Google Maps'in mobil web iç stratejileri.
