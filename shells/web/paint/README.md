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

### `fixture` (sınama demirbaşı)

Seçilecek bir yol değildir. Yalnızca adreste `fixture=1` varken desteklenir;
`tests/compare.mjs`'nin ikinci yolu odur. Boyama listesi arayüzünü ve yazı tipini gerçek
tarayıcıda çalıştırır, sonra başvuru piksellerini boyar.

### `gpu` (RP3/M12c, WebGPU)

`?paint=gpu`. Çekirdek sahneyi üçgenlere çevirir (`gpu_face` / `gpu_scene`,
`crates/qp-wasm/src/shell/paint_gpu.rs`), yol köşe baytlarını ve glif atlasının değişen
satırlarını yükler, çizim listesini olduğu gibi uygular. Tek GPU aygıtı bütün tuvallere
hizmet eder; kenar yumuşatma 4× MSAA (K12). `supported()` = `navigator.gpu` var ve
aygıt isteği başarısız olmadı. Aygıt hazır olana kadar boyamaz (tuvale bağlam almaz),
hazır olunca `env.repaint()`; bağdaştırıcı/aygıt alınamazsa `ref` ile boyar ve
`set_paint('gpu', 'ref', 'unsupported')` bildirir; aygıt kaybında yenisini ister.
Ölçüm ve görsel fark: `rounds/RP3/experiments/M12c.md`, test `tests/gpu.mjs`.

**Sözleşmeye eklenen tek genel değişiklik (`main.js`):** `window.qp.png` başka bağlam
türüyle (WebGPU) boyanmış bir tuvali `snapshot` ile 2B kopyadan okur; 2B tuval eskisi gibi
doğrudan okunur.

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
