# Boyama yolları (RP3/T2)

Bir boyama yolu, çekirdeğin ürettiği sahneyi (`SceneDesc`, K24) tuvale nasıl
taşıdığımızdır. Sahneyi **çekirdek** kurar; yol yalnızca boyar. Yollar bu klasörde yaşar,
birbirini görmeden eklenir ve ölçüm ekranındaki "Karşılaştır" sınamasıyla aynı senaryoda
karşılaştırılır.

## Seçim

- Adres parametresi: `?paint=<ad>`. Varsayılan `ref`.
- Bilinmeyen ad → `ref`, neden `unknown`; `supported()` yanlış → `ref`, neden
  `unsupported`. Seçim `core.set_paint(istenen, kullanılan, neden)` ile çekirdeğe
  bildirilir; ölçüm özetinde `paint: {requested, used, fallback}` olarak görünür.
- Kayıt: `paint/index.js` içindeki `PATHS`. Yeni bir yol = bu klasörde bir dosya + `PATHS`'e
  bir satır. Başka dosyaya dokunulmaz.

## Sözleşme

Her yol bir `default` nesne verir:

| Alan | Ne |
|------|----|
| `name` | Adres parametresindeki ad. |
| `supported()` | Özellik sınaması (ör. `'gpu' in navigator`). Yan etkisiz, eşzamanlı. |
| `init(core, env)` | Her yeni çekirdekte (her başlangıç çerçevesinde) ve yol değişince çağrılır. `env.repaint()` görünür her şeyi yeniden boyatır (eşzamansız hazırlanan bir yol hazır olunca çağırır). |
| `paintFace(canvas, id, force, px)` | `id` kapsülünün yüzünü `canvas`'a boyar. `px` = `[w, h]` aygıt pikseli (çekirdeğin `face_size().px`'i). Yeni bir şey boyadıysa `true`, sahne değişmediyse `false`. |
| `paintScene(canvas, force, px)` | Açık kapsülün sahnesi; `px` = `open().px`. Aynı dönüş. |

Ölçüm (`face`, `scene` süreleri) `main.js`'te, çağrının iki yanında alınır; her yol aynı
yerden ölçülür. Eşzamansız iş yapan bir yol (GPU kuyruğu) ek süresini
`core.metric_span('<yol>_gpu', t0, t1)` ile kendi bildirir.

Yol değişince (karşılaştırma) kabuk tuvalleri **yeniden yaratır**: bir yol kendi bağlam
türünü (`2d`, `webgl2`, `webgpu`) serbestçe alabilir. Normal açılışta tuvaller ilk kez
yaratılır, değiştirilmez.

### `ref` (başvuru)

R4'ün davranışı bayt bayt: çekirdek tiny-skia ile aygıt ölçeğinde tarar (`face_rgba`,
`scene_rgba`), yol tuvalin boyutunu ayarlar ve `putImageData` ile kopyalar. Altın
görsellerle uyumlu yol budur.

### `cpu` (RP3/M12a, varsayılan kapalı: `?paint=cpu`)

Başvuru çizicinin kendi kodu (tiny-skia + fontdue), gereksiz işsiz. Çekirdek
(`crates/qp-wasm/src/shell/paint_cpu.rs`, `crates/qp-render/src/retained.rs`) her yüzün
ve açık sahnenin son karesini tutar; yalnız değişen ilkellerin dokunduğu dikdörtgeni,
değişmeyen ön ilkellerin (ızgara, geçmiş barlar) görüntüsünden başlayarak yeniden boyar.
Pikseller `ref` ile **bayt bayt aynı** (`tests/cpu-pixels.mjs`). Yol, kareyi WASM belleğine
açılan bir görünümle okur (sınırdan kopya yok), boyutu değişmeyen tuvali yeniden
ayırmaz, `putImageData(img, 0, 0, dx, dy, dw, dh)` ile yalnız değişen dikdörtgeni koyar.
Grafikte sürükleme/kıstırma sürerken ölçek 1'de boyar; bitince (`core.cpu_moving()`
yanlış) `env.repaint()` ile aygıt ölçeğinde yeniden boyatır. Sözleşmeye ek yok; `main.js`
değişmedi. Ölçüm için teknikler kapatılabilir: `?cpu=nodirty,nolayer,noreduce,noreuse,noview`
ya da `off`; `?cpu_variants=1` her biri için ayrı bir yol kaydeder (karşılaştırmada hepsi
koşar). Ayrıntı ve ölçümler: `rounds/RP3/experiments/M12a.md`.

### `fixture` (sınama demirbaşı)

Seçilecek bir yol değildir. Yalnızca adreste `fixture=1` varken desteklenir;
`tests/compare.mjs`'nin ikinci yolu odur. Boyama listesi arayüzünü ve yazı tipini gerçek
tarayıcıda çalıştırır, sonra başvuru piksellerini boyar.

## Piksel olmayan yollar için çekirdek arayüzü

- `core.face_scene(id, force)` → yüzün **boyama listesi** (JSON metni); değişmediyse boş
  dizge. `face_rgba` ile aynı kısa devre ve aynı kayıt (bir yol ikisinden birini kullanır).
- `core.scene_desc(force)` → açık sahnenin boyama listesi; aynı kural (`scene_rgba` gibi).
- `core.font()` → başvuru çizicinin gömülü yazı tipi dosyasının baytları (IBM Plex Mono
  Regular, OFL; `crates/qp-render/assets/fonts/IBMPlexMono-Regular.ttf`). Tuvalle metin
  çizen yol **bu dosyayı** kaydeder, başka yazı tipi kullanmaz:
  `const f = new FontFace('QpMono', core.font()); document.fonts.add(f); await f.load();`
  (Ağ isteği yok; CSP `font-src 'self'` buna engel değil.)

### Boyama listesi biçimi (v1)

`SceneDesc`'in ilkelleri **boyama sırasında** (`SceneDesc::prims`: `under`, panellerin
katmanları, `over`), başvuru çizicinin kendi ölçekleme kuralıyla
(`qp_render::cpu::scale_prim`) **aygıt ölçeğinde**. Kabukta aritmetik yasak olduğu için
(R3) yolun ihtiyaç duyduğu her sayı ve metin hazır gelir.

```json
{"v": 1, "w": 1170, "h": 900, "scale": 3, "bg": "#0f1115ff", "miter": 4,
 "prims": [
  {"k": "rect",   "x": 0, "y": 0, "w": 30, "h": 3, "c": "#rrggbbaa"},
  {"k": "line",   "pts": [[x, y], ...], "lw": 3.0, "c": "#rrggbbaa"},
  {"k": "fill",   "pts": [[x, y], ...], "c": "#rrggbbaa"},
  {"k": "circle", "x": 10.5, "y": 20.0, "r": 6.0, "a1": 6.283185307179586, "c": "#rrggbbaa"},
  {"k": "text",   "x": 12, "y": 33, "t": "THYAO", "font": "33px QpMono", "align": "left", "c": "#rrggbbaa"}
 ]}
```

| İlkel | Başvuru çizicideki anlamı | Tuval 2B karşılığı |
|-------|---------------------------|--------------------|
| arka plan | `bg` ile bütün yüzey | `fillStyle = bg; fillRect(0, 0, w, h)` |
| `rect` | Piksele hizalı dolu dikdörtgen, **kenar yumuşatmasız** | `fillRect(x, y, w, h)` (tamsayı) |
| `line` | Yumuşatılmış çoklu çizgi, `butt` uç, `miter` birleşim, miter sınırı `miter` (4; tuvalin varsayılanı 10) | `lineWidth = lw; lineCap = 'butt'; lineJoin = 'miter'; miterLimit = miter; moveTo/lineTo; stroke()` |
| `fill` | Yumuşatılmış kapalı çokgen, sıfırdan farklı sarım kuralı | `moveTo/lineTo; closePath(); fill('nonzero')` |
| `circle` | Yumuşatılmış dolu daire | `arc(x, y, r, 0, a1); fill()` |
| `text` | Tek satır; `x` çapa, `y` taban çizgisi; `align: right` → metin `x`'te biter | `font = font; textAlign = align; textBaseline = 'alphabetic'; fillText(t, x, y)` |

Renkler düz (önçarpımsız) RGBA'dır, `#rrggbbaa` biçiminde.

### `canvas` (RP3/M12b): tarayıcının Canvas 2D bağlamı

Çekirdek tarama yapmaz; tarayıcının kendi (WebKit/Chromium'da donanım hızlandırmalı) 2B
bağlamı boyar. Ayrıntı ve ölçümler: `rounds/RP3/experiments/M12b.md`.

- Bağlam `getContext('2d', {alpha: false})` (her sahne önce opak arka planını boyar).
  Tuvalin boyutu yalnız değişince atanır (arka tampon yeniden yaratılmaz).
- Yazı tipi: `init`'te `new FontFace('QpMono', core.font())`; yüklenene kadar hiçbir şey
  boyanmaz, yüklenince `env.repaint()`. Yüklenemezse tarayıcının yedek yazı tipiyle boyar.
- `supported()`: `FontFace`, `document.fonts` ve 2B bağlam varsa. Yoksa `ref`'e düşer
  (`unsupported`; `tests/paint-canvas.mjs` `FontFace`'i silerek sınar).
- Çeşitler (deney için; `?canvas=` değeri olduğu gibi çekirdeğe gider, `canvas_config`):
  `glyphs` (metni tarayıcı glif glif çizer), `run` (metni tarayıcı tek parça çizer,
  `textAlign` ile), `nobatch`, `full` (değişen bölge yok, hep tamamı), `notext` (tanı:
  metinsiz). Varsayılan: glif atlası + toplu yollar + değişen bölge. Tanı için
  `?cx_sync=1` her boyamadan sonra bir piksel geri okur (ölçülen süre tarayıcının taramasını
  da kapsar).

#### Sözleşme eki: canvas komut akışı

Boyama listesi v1'in (JSON) yerine, aynı ilkellerden (aynı sıra, aynı `scale_prim`)
çekirdekte kurulan düz bir **komut akışı**: `core.canvas_face(id, force)` /
`core.canvas_scene(force)` → `Float32Array` (değişmediyse boş; `face_rgba`/`scene_rgba` ile
aynı kısa devre ve kayıt; `force`: tuval yeni, tamamını boya), `core.canvas_strings()` →
son akışın dizge tablosu (JSON dizisi). Akış `kod, bağımsız değişkenler…` dizisidir; dizge
bağımsız değişkenleri (`S[i]`) tablonun sırasıdır. Yol akışı bir yineleyiciyle okur
(dizinle okumak aritmetik ister, R3) ve her kodu tek bir çağrıya çevirir:

| Kod | Ad | Bağımsız değişkenler | Tuval çağrısı |
|----:|----|----------------------|---------------|
| 1 | `FILL_STYLE` | s | `fillStyle = S[s]` |
| 2 | `STROKE_STYLE` | s | `strokeStyle = S[s]` |
| 3 | `LINE_WIDTH` | v | `lineWidth = v` |
| 4 | `FONT` | s | `font = S[s]` |
| 5 | `ALIGN` | s | `textAlign = S[s]` |
| 6 | `RECT` | x y w h | `fillRect(x, y, w, h)` |
| 7 | `BEGIN` | — | `beginPath()` |
| 8 | `MOVE` | x y | `moveTo(x, y)` |
| 9 | `LINE` | x y | `lineTo(x, y)` |
| 10 | `CLOSE` | — | `closePath()` |
| 11 | `ARC` | x y r a1 | `arc(x, y, r, 0, a1)` |
| 12 | `STROKE` | — | `stroke()` |
| 13 | `FILL` | — | `fill()` (sıfırdan farklı) |
| 14 | `TEXT` | s x y | `fillText(S[s], x, y)` |
| 15 | `CLIP` | x y w h | `save(); beginPath(); rect(x, y, w, h); clip()` |
| 16 | `RESTORE` | — | `restore()` |
| 17 | `INIT` | cap join miter base | `lineCap = S[cap]; lineJoin = S[join]; miterLimit = miter; textBaseline = S[base]; imageSmoothingEnabled = false` |
| 18 | `GLYPH` | sx sy w h dx dy | `drawImage(atlas, sx, sy, w, h, dx, dy, w, h)` |
| 19 | `ATLAS` | w h y0 rows | atlas tuvali `w × h` (değişince); `putImageData(core.canvas_atlas_px(), 0, y0)` (`w × rows`) |

Çekirdeğin akışa kattıkları (yol yalnız uygular):

- Her akış `INIT` ile başlar. Tam boyama: arka plan `RECT` ile bütün yüzey. Kısmi boyama:
  `INIT`'ten hemen sonra `CLIP` (değişen bölge, tam piksel), bölgeye arka plan ve sınırları
  bölgeye değen **bütün** ilkeller sırasıyla, sonda `RESTORE`. Değişen bölge, aynı tuvalin
  bir önceki listesiyle sıra sıra karşılaştırılarak bulunur (ilkel sınırları kenar
  yumuşatması, miter birleşimi ve glif taşmasını kapsayacak kadar geniş); bölge tuvalin
  yarısından büyükse tam boyama.
- Biçem yalnız değişince yazılır; ardışık, aynı opak renkli (dolgularda aynı sarım yönlü)
  çizgi/dolgu/daireler tek yol + tek `stroke()`/`fill()` olur. Her dairenin kendi alt yolu
  vardır (`MOVE` ile başlar).
- Metin (varsayılan, **glif atlası**): çekirdek her glifi başvuru çizicinin yaptığı gibi
  fontdue ile aynı boyda tarar, metnin renginde düz RGBA olarak atlasa koyar (raf
  yerleşimi, 1024 px genişlik, yükseklik 128 → 2048 ikiye katlanarak) ve glifi başvuru
  çizicinin kalem konumunda (tam piksele yuvarlanmış kalem + fontdue `xmin`, taban çizgisi −
  yükseklik − `ymin`) `GLYPH` ile basar. Yeni glif gerekince akışta `INIT`'ten (ve varsa
  `CLIP`'ten) sonra `ATLAS` gelir; yol satırları `core.canvas_atlas_px()`'ten alıp atlas
  tuvaline koyar (atlas büyüyünce tuval temizlenir; çekirdek o zaman kullanılan bütün
  satırları yeniden gönderir). Atlas tuvali `willReadFrequently: true` ile alınır (CPU
  tamponu): GPU tamponlu atlasta başsız WebKit, aynı boyamada yüklenen glifleri ara sıra
  kırmızı/mavi kanalları yer değiştirmiş çizdi (M12b.md). Atlas dolarsa o glifi tarayıcı
  `fillText` ile çizer; atlas sonraki akışta baştan başlar (bellek üst sınırı: çekirdekte
  ve tarayıcıda birer 1024 × 2048 × 4 B = 8 MB).
- Metin (`glyphs` çeşidi): glif başına `TEXT`, başvuru çizicinin kalem konumunda, sola
  hizalı; boşluklar atlanır. (`run` çeşidi: metin başına bir `TEXT`, `ALIGN` ile.)
- Başvuru çizicinin hiçbir şey boyamadığı durumlar aynıdır: genişliği/yüksekliği 0 olan
  dikdörtgen, 2'den az noktalı çizgi/dolgu, yarıçapı ≤ 0 daire atlanır; geçersiz yazı boyu
  varsa akış boştur (başvuru da boş döner). Genişliği 0 olan çizgi (tiny-skia'da kıl
  çizgisi) `lineWidth = 1` olur.

### Yolun yapamadıkları

Yol yalnızca listedeki ilkelleri, verilen sırada, verilen sayılarla boyar. **Yerleşim,
metin kurma, sıralama, süzme, koordinat hesabı yapmaz** (`docs/ARCHITECTURE.md` "Kabuk
sınırı", RP3 TASKS "Mimari sınır"). Bunu `node shells/web/arch-check.mjs` denetler: her
`paint/*.js` dosyası `main.js` ile aynı R1–R5 kurallarından geçer; R6'da bir yol yalnızca bu
klasördeki kardeş dosyaları içe aktarabilir (`./ad.js`), dinamik `import()` yasaktır.
Çekirdeğe yalnızca `ARCH.md` izin listesindeki yöntemlerle ve `core` adıyla erişilir.
Gölgelendirici (shader) kaynağı dizge olarak durabilir; dizge içerikleri taranmaz.

## Karşılaştırma (ölçüt 8)

Ölçüm sekmesi → "Karşılaştır". Senaryoyu çekirdek sürer (`bench_start`, `bench_tick`;
`crates/qp-wasm/src/shell/bench.rs`); kabuk her karede dönen eylemleri olduğu gibi uygular
(`paint`: yola geç, `tab`: sayfayı göster, `scroll`: akışın `scrollTop`'u, `done`). Yollar
aynı sayfada sırayla çalışır (yeniden yükleme yok); her yol aynı adımdan (120) ve aynı
hızla (4 adım/sn) başlar. Ayrıntı: `rounds/RP3/experiments/T2.md`.
