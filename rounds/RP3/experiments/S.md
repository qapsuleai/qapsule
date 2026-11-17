# RP3/S — Boyama yollarının bütünleştirilmesi ve birlikte ölçüm

Dal `rp3/s-paint` (`main` `0512196`'dan). Birleştirilenler, bu sırayla: `rp3/m14-freq`
(`39e7479`), `rp3/m12b-canvas` (`e4d0b43`), `rp3/m12a-cpu` (`65a489f`), `rp3/m12c-gpu`
(`545e7a6`). `main`'e birleştirilmedi.

## 1. Birleştirme ve uyarlama

### 1.1 Çakışmalar (hepsi "iki taraf da kalır")

| Dosya | Çakışma | Çözüm |
|---|---|---|
| `crates/qp-wasm/src/lib.rs` | M12b/M12c `Shell::new` içinde eski (JSON ağaçlı) kurucuyu taşıyordu; `main`'de T4'ün tipli `core_from_start`'ı var. `Shell` yapısına iki alan (`canvas`, `gpu`). M12a/M12b yöntemleri aynı yerde. | T4'ün kurucusu kaldı, iki alan eklendi; üç yolun bağlama yöntemleri art arda. |
| `crates/qp-wasm/src/shell.rs` | `pub mod paint_*` satırları; `ShellCore` alanları (M15/M14 ↔ M12a `cpu`). | Birleşim. |
| `shells/web/arch-check.mjs`, `ARCH.md` | İzin listesi ve gerekçe tabloları (T4, M14, M12b, M12a, M12c). | Birleşim; hiçbir kural gevşetilmedi, her yöntem kendi dalının gerekçesiyle. |
| `shells/web/paint/index.js` | Kayıt satırları. | Birleşim: `ref`, `canvas`, `fixture`, `cpu` (+ değişkeleri yalnız `?cpu_variants=1`), `gpu`. |
| `shells/web/tests/compare.mjs` | M12b'nin seçenekleri (`--paths`, `--throttle`, `--viewport`, `--port`, `--q`, `--jsc`) ↔ M12c'nin "ilk yol ref, fixture listede" denetimi. | Tek seçenek kümesi (aşağıda). |

### 1.2 Yolların M14 boyama geçişinde ve görüntüleyici kipinde çalışması

- Üç yol da sahneyi çekirdekten `ShellCore::face(id)` / `ShellCore::scene()` ile alıyor
  (`paint_canvas.rs`, `paint_cpu.rs`, `paint_gpu.rs`). M15'te bu iki yöntem görüntüleyici
  kipini zaten kapsıyor: yüz, otoritenin hazırladığı `SceneDesc` (`viewer.face(id)`); açık
  sahne, cihazdaki açık enstrüman diliminden (`OpenSide`). Yani yollar görüntüleyicide de
  tam yansıda da aynı sahne tarifini boyuyor; ayrı bir uyarlama gerekmedi. Kanıt:
  `tests/paint-paths.mjs` (görüntüleyici varsayılan kipte, dört yol: yüz ve sahne boyandı,
  özetler `ref` ile bayt bayt aynı) ve `paint-canvas.mjs`, `cpu-pixels.mjs`, `gpu.mjs`'in
  birleşik yapıda yeniden koşusu (§4).
- M14'ün tek boyama geçişi `paint.paintFace` / `paint.paintScene`'i `repaint` içinden
  çağırıyor; sözleşme değişmedi. **Uyarlanan tek şey:** yolların kendi yeniden boyama isteği
  (`env.repaint()`: `canvas` yazı tipi yüklenince, `gpu` aygıt hazır olunca, `cpu`
  sürükleme bitince) eskiden hemen `renderFeed(); renderLayer(true)` çağırıyordu; artık bir
  bayrak koyuyor ve bir sonraki boyama geçişinde (aynı karedeki değişikliklerle birlikte) bir
  kez boyanıyor (M14: kare başına en çok bir boyama). `window.qp.ready()` bu bayrak
  bekliyorken `false`.
- Diğer yollardan habersiz yazılmış ama örtüşen bir şey çıkmadı: her yol kendi çekirdek
  durumunu (`CanvasPainter`, `CpuPaint`, `GpuPainter`) ve "gösterilen" kaydını (`face_shown`,
  `scene_shown` ortak) kullanıyor; karşılaştırmada yol değişince kabuk tuvalleri yeniden
  yaratıyor ve her yol yeni tuvalde tam boyama istiyor.

### 1.3 Karşılaştırma seçenekleri (`tests/compare.mjs`, bench)

`--paths a,b` (adreste `?bench_paths=`), `--order reverse` (`?bench_order=reverse`),
`--throttle`, `--viewport`, `--port`, `--dpr`, `--phase`, `--q`, `--jsc useJIT=false`
(JIT'siz JSC, T1'in PHONE'u), `--channel chromium` (tam Chromium: WebGPU'lu). Varsayılan yol
listesi kayıttaki **desteklenen** yollar (sayfanın kendi `supportedNames()`'i); beklenen
liste sayfadan okunur, `--paths` ile kesişir, `--order reverse` ile ters çevrilir. Seçim ve
ters çevirme çekirdekte (`bench.rs`, `cfg.only`, `cfg.order`; kabuk değeri olduğu gibi
verir; birim testi `the_comparison_selects_and_reverses_paths`). `gpu` WebGPU adaptörü
olmayan tarayıcıda kendi yedeğini bildirirse (`used: ref, fallback: unsupported`) satır
geçerli sayılır ve yazılır.

### 1.4 Seçim (öneri) ve yedek

- **Varsayılan yol `canvas`** (`main.js` `DEFAULT_PAINT`; ana oturumun kararı, öneri olarak):
  güvenli bağlam istemez, JIT'siz telefonda yüz/sahne süreleri en düşük, paket farkı küçük.
  `canvas` desteklenmezse (ör. `FontFace` yok) kayıt `ref`'e düşer (`unsupported`).
- `ref` deterministik başvuru ve yedek: `screens.mjs` artık `?paint=ref` ile boyar
  (`--paint` seçeneği, varsayılan `ref`) ve verify `demo` adımı altın görsellerle bayt
  eşitliğini korur. `cpu` ve `gpu` `?paint=` ile seçilir.

### 1.5 Ortak değişiklik sınaması: JSON özeti yerine parmak izi (ana oturumun isteği)

`ref`, `canvas`, `gpu` ve boyama listesi yüz/sahne teslimini "son teslimden beri değişti mi"
diye sınıyordu (`face_shown`/`scene_shown`): anahtar `scene_digest` (bütün `SceneDesc`'in
JSON'u + FNV-1a). Artık anahtar `qp_render::retained::scene_fingerprint`: boyut, arka plan ve
`SceneDesc::prims()`'in her ilkelinin her alanı, boyama sırasıyla, bit bit, 64 bit SipHash
(sabit anahtarlar; `cpu` yolunun ilkel anahtarı); dizileştirme yok. Sınama tek yerde:
`ShellCore::face_changed` / `scene_changed` (`shell.rs`); süresi `change_check`, sahne kurma
`scene_build` olarak ölçülür (yalnız wasm'da). **Değişmeyen:** `digests()` (eşlik/`view_digest`,
§19.6 FNV-JSON), `drive.rs`, protokol ve altın görseller. Yanlış "değişmedi" yalnız 64 bitlik
bir çakışmayla olur; her piksel alanının değişiminin anahtarı çevirdiği test edildi
(`qp-render/tests/retained.rs::scene_fingerprint_flips_on_any_pixel_field`: boyut, arka plan,
sıra, sayı, beş ilkel türünün her alanı, tür değişimi; görünüm adı ve eşleme gibi piksele
ulaşmayan alanlar anahtarı değiştirmez). Ölçüm ve döküm: §6.1.

## 2. Testler (ölçüt 4, 10; VERIFY 5, 7, 8)

| Test | Ne sınar | verify | Sonuç (birleşik yapı) |
|---|---|---|---|
| `tests/paint-paths.mjs` (yeni) | Her yol (`ref`, `canvas`, `cpu`, `gpu`) `?paint=` ile açılır, ilk yüz ve açık sahne boyanır (tuval boş değil), bildirilen yol doğru; `gpu` WebGPU adaptörü yoksa bildirilen yedek (`used: ref, fallback: unsupported`) doğrulanır; adressiz sayfa `canvas` kullanır. **Anlamsal okumalar** her yolda `ref` ile bayt bayt aynı: sayfanın özetleri (doküman, yüzler, açık sahne, bakış), `feed`, `open`, ve sayfanın **kendi bağlantısında** otoritenin cevapları: 8 `tap_at` cevabı (dokunulan noktanın nokta sorgusu `hit` + etkileşim) ve sonda bir `screen` sorgusu (`seq`/`id` hariç: sayfanın diğer mesaj sayısı zamanlamaya bağlı). | `demo` adımı, Chromium (~12 s) | Chromium: ref/canvas/cpu kendi yolları, gpu → yedek (başsız kabukta adaptör yok); WebKit: dördü de kendi yolu (gpu Apple adaptörü). İki motorda 3 yol = ref: **bayt bayt aynı** (10 otorite cevabı). Not: gösterimde açık kapsül cihazdaki bakış durumu, otoritede görünüm yok; `screen` cevabı `screens: []` (iki yolda aynı). |
| `tests/paint-canvas.mjs` (M12b) | `canvas` ↔ `ref` toleranslı görsel fark (16 görüntü), kısmi = tam, anlamsal okumalar, `FontFace` yokken yedek | `demo` adımı, Chromium, DPR 1 (~40 s) | Geçti (aşağıda §4) |
| `screens.mjs` (`--paint`, varsayılan `ref`) | Sahneler altın görsellerle bayt bayt, sayfa görüntüleri iki koşu arasında | `demo` adımı | Değişmedi |
| `tests/cpu-pixels.mjs`, `tests/gpu.mjs` | `cpu` bayt eşitliği; `gpu` görsel fark + WebGPU yok/adaptör yok/aygıt kaybı | — (uzun; tam Chromium kanalı ister) | Geçti (§4) |
| `metrics_t2::the_comparison_selects_and_reverses_paths` (yeni) | bench yol seçimi ve ters sıra | `test` | Geçti |

## 3. Ölçüm (ölçüt 2, 3)

**Düzenek:** `rounds/RP3/experiments/S/measure.sh` → `tests/compare.mjs` (sayfanın kendi
"Karşılaştır"ı: `idle`, `scroll`, `chart_ticks`, `drag`, evre 8 s, 4 adım/sn), görüntüleyici
kipi (varsayılan), sentetik pazar 30 enstrüman, DPR 3, 402×874. Ayarlar: **PHONE** (WebKit
26.5, `__XPC_JSC_useJIT=false`), **PHONE_JIT_EST** (aynı, JIT açık), **CDP4** (Chromium başsız
kabuk, CDP 4×; WebGPU adaptörü yok → `gpu` desteklenen yollar arasında değil), **CDP4 tam**
(aynı, tam Chromium `--channel chromium`: `gpu` Metal'de gerçekten çalıştı). Her ayar
**ileri** (ref, canvas, cpu, gpu) ve **ters** sırada **3 koşu**; sayılar koşuların
**ortancası**. Sırayla, paralel değil; 15:34–16:26 arası 24 koşu, 0 hata. Yük (`uptime`
1 dk, koşu başı/sonu): PHONE 4,1–9,2; PHONE_JIT_EST 3,6–13,5; CDP4 4,3–8,1; CDP4 tam 6,1–8,8
(16 çekirdek, makine paylaşımlı). Sayfa 127.0.0.1'den açıldığı için `secure_context: true`
(WebGPU açık); JIT sınaması PHONE'da `likely_off` (38 ms). Ham tablolar (her ayar × sıra,
ayrıntı sütunlarıyla): `rounds/RP3/experiments/S/tables.md` (`tables.py` üretir).

Alanlar: `tick` = veri çerçevesinin çekirdek işi (M14'ten beri **boyamasız**); `pass` =
boyama geçişi; **tik+geçiş** (R4 ile karşılaştırılabilir) = evrenin ortalama tiki + veri
çerçevesi başına geçiş süresi (`counts.tick_plus_pass_ms`). WebKit saati 1 ms adımlı.

### 3.1 PHONE (JIT'siz, Arkin'in ölçüm koşulu) — ileri / ters

| yol | tik p50/p95/p99 | geçiş p50/p95/p99 | yüz p50/p95/p99 (n) | sahne p50/p95/p99 (n) | tik+geçiş idle / chart | WASM sonu (MB) |
|---|---|---|---|---|---|---:|
| ref | 0/1/1 · 1/1/1 | 26/43/122 · 25/42/119 | 32/42/43 · 32/42/45 (22) | 24/29/47 · 24/30/45 (131) | 12,8/19,8 · 12,6/20,3 | 16,1 · 48,2 |
| **canvas** | 1/1/1 · 1/1/1 | 9/17/19 · 9/17/18 | **2/7/7 · 2/10/10** (14) | **7/10/11 · 6/9/10** (122) | **1,6/14,3 · 1,4/13,7** | 16,1 · 48,2 |
| cpu | 1/1/1 · 1/1/1 | 9/28/43 · 8/29/43 | 26/39/39 · 25/38/38 (14) | 7/16/25 · 8/14/27 (125) | 3,2/8,8 · 3,4/9,2 | 48,8 · 48,2 |
| gpu | 1/1/1 · 1/1/1 | 11/17/19 · 9/17/18 | 3/5/5 · 3/4/5 (22) | 8/11/14 · 8/10/15 (130) | 2,3/14,9 · 2,1/14,3 | 50,8 · 8,9 |

Kare aralığı p50/p95/p99 (uzun kare payı = > 1,5 × 16,7 ms), ileri sıra (ters sıra
ayrıntısı `tables.md`'de; aynı tablo, farklar ±1 ms):

| yol | idle | scroll | chart_ticks | drag |
|---|---|---|---|---|
| ref | 17/18/27 (0,011) | 17/18/46 (0,021) | 17/18/40 (0,011) | 17/29/31 (**0,263**) |
| **canvas** | 17/18/19 (0,002) | 17/18/19 (0,002) | 17/18/22 (0,002) | 17/19/21 (0,002) |
| cpu | 17/18/21 (0,006) | 17/18/43 (0,019) | 17/18/19 (0,002) | 17/18/24 (0,008) |
| gpu | 17/18/20 (0) | 17/18/19 (0) | 17/19/22 (0,002) | 17/18/19 (0) |

Evre ayrıntısı (ileri): `chart_ticks` geçiş p50/p95 — ref 16/52, canvas 14/19, cpu 9/12,
gpu 17/19; veri→boyama p95 — ref 61, canvas 33, cpu 24, gpu 31 ms. `scroll` yüz p50/p95 —
ref 33/43, canvas 2/7, cpu 31/39, gpu 3/5. `drag` sahne p50/p95 — ref 24/26, canvas 7/9,
cpu 7/13 (hareket sırasında ölçek 1), gpu 8/11.

### 3.2 PHONE_JIT_EST (WebKit JIT açık) — ileri / ters

| yol | tik p50/p95 | geçiş p50/p95/p99 | yüz p50/p95 | sahne p50/p95 | tik+geçiş idle / chart | kare p95 (her evre) |
|---|---|---|---|---|---|---|
| ref | 0/1 · 0/1 | 3/6/10 · 3/7/12 | 3/5 · 3/5 | 4/5 · 4/6 | 1,58/2,84 · 1,58/2,29 | 17–18, uzun 0–0,002 |
| **canvas** | 0/1 · 0/1 | 1/2/3 · 1/2/3 | 1/3 · 1/3 | 1/1 · 1/1 | 0,48/1,38 · 0,58/1,25 | 18, uzun 0 |
| cpu | 0/1 · 0/1 | 1/3/6 · 1/4/6 | 2/4 · 3/5 | 1/2 · 1/2 | 0,71/1,16 · 0,71/1,29 | 18–19, uzun 0 |
| gpu | 0/0 · 0/1 | 2/6/9 · 2/5/7 | 2/4 · 2/3 | 3/5 · 3/5 | 1,09/1,58 · 1,03/2,55 | 18–19, uzun 0–0,002 |

### 3.3 Chromium CDP 4× (başsız kabuk; `gpu` yok) — ileri / ters

| yol | tik p50/p95/p99 | geçiş p50/p95/p99 | yüz p50/p95 | sahne p50/p95 | tik+geçiş idle / chart | kare p95 |
|---|---|---|---|---|---|---|
| ref | 0,1/0,4/1,7 · 0,1/0,5/0,7 | 8,2/10,6/29,4 · 8,6/10,7/28,4 | 7,6/9,4 · 7,4/9,4 | 7,8/9,5 · 8,2/9,6 | 3,17/3,91 · 3,08/3,89 | 9,2 (120 Hz) |
| **canvas** | 0,1/0,4/0,8 · 0/0,3/0,4 | 1,9/3,3/4 · 1,8/3,3/4,3 | 2,1/3,7 · 2,1/3 | 1,2/1,8 · 1,2/1,9 | 0,49/2,07 · 0,43/2,08 | 9,2 |
| cpu | 0,1/0,4/0,8 · 0,1/0,6/1,8 | 1,7/5,7/7,9 · 1,8/6,1/8,2 | 5,2/6,8 · 5,1/6,6 | 1,2/4,2 · 1,3/4 | 0,76/1,62 · 0,78/1,98 | 9,2 |

**CDP 4× tam Chromium (`gpu` çalıştı, Metal):** ref yüz/sahne p50 8,2/8,5, canvas 1,4/1,4,
cpu 5,0/1,4, **gpu 1,3/1,6**; geçiş p95 ref 11,2, canvas 3,2, cpu 5,6, gpu 3,5; tik+geçiş
chart ref 4,73, canvas 2,51, cpu 1,83, gpu 2,54 (ileri; ters sıra ±%15). Bütün kare p95'leri
9,2 ms (120 Hz), uzun kare payı ≤ 0,004. JS yığını (yolun sonunda, tam Chromium): ref 27–56 MB,
canvas 3,4–4,0, cpu 3,6, gpu 4,8–6,0.

### 3.4 Sıra etkisi ve bellek

- **Süreler sıraya duyarsız:** ileri ve ters sıradaki ortancalar her yol için ±1 ms (PHONE)
  ya da ±%15 (CDP) içinde; M12a'nın açık sorusu ("cpu tik p50'si ref'in ardından yüksek mi")
  bu koşularda görülmedi (PHONE cpu tik p50 1 ms her iki sırada; M14 sonrası tik zaten
  boyamasız).
- **WASM belleği sıraya bağlı** (yalnız büyür; değer o yolun sonundaki): ilk yol hangisiyse
  küçük. `cpu` tutulan yüzeyleriyle WASM'ı ~16 → ~49 MB büyütüyor (ileri: ref 16,1 → canvas
  16,1 → cpu 48,8 → gpu 50,8; ters: gpu 8,9 → cpu 48,2). `canvas` ve `gpu` ref'in üstüne bir
  şey eklemiyor (ileride 16,1 = 16,1). JS yığını Chromium'da `ref` en büyük (her boyamada
  `ImageData` kopyası): tam Chromium'da 27–56 MB ↔ canvas 3–4 MB.

## 4. Görsel fark (yollar ↔ ref), birleşik yapıda yeniden koşu

Yük 4,4–6,3 (1 dk). Testler dalların kendi testleri, değiştirilmeden.

| Yol | Test | Görüntü | Sonuç |
|---|---|---|---|
| canvas | `paint-canvas.mjs` DPR 1 | 16 × 2 motor | > 2 düzey farklı piksel ortanca / en kötü: Chromium %0,27 / %1,06, WebKit %0,22 / %0,85; > 32 düzey en kötü %0,021; en büyük kanal farkı 107 (çizgi kenarı). Tolerans (≤ %1,5 / ≤ %0,1) içinde; kısmi = tam 14/14; yedek 2/2 |
| canvas | aynı, DPR 3 | 16 × 2 | > 2 düzey ortanca / en kötü: Chromium %0,09 / %0,30, WebKit %0,06 / %0,29; > 32 en kötü %0,006; maks 58 / 93 |
| cpu | `cpu-pixels.mjs` | 49 karşılaştırma × 2 motor | **0 farklı piksel** (DPR 1 altınlarla da aynı) |
| gpu | `gpu.mjs` (tam Chromium + WebKit, DPR 3) | 16 × 2 | farklı piksel %0,17–3,72, > 2 düzey en çok %0,308, **düz alanda 0**; maks 72 (MSAA kenarı). Yedek: WebGPU yok / adaptör yok / başsız kabuk → `ref` + `unsupported`; aygıt kaybı → yeni aygıt, `gpu` kalır |

Sayılar dalların notlarındakilerle aynı (görüntüleyici kipinde de): birleştirme hiçbir yolun
pikselini değiştirmedi.

## 5. Paket boyutu (birleşik)

`scripts/demo.sh` yolu (`--profile wasm --no-opt`, `replica`, strip); taban `0512196` aynı
yolla bu makinede derlendi. Bayt.

| Dosya | taban `0512196` ham / gzip / br | `rp3/s-paint` ham / gzip / br | fark (gzip) |
|---|---|---|---|
| `qp_wasm_bg.wasm` | 4.850.046 / 1.433.269 / 947.820 | 4.955.339 / 1.463.230 / 967.701 | +29.961 (+%2,1; br +19.881) |
| `qp_wasm.js` | 68.675 / 9.049 / 7.810 | 85.804 / 11.243 / 9.535 | +2.194 |
| `main.js` | 31.509 / 10.158 / 8.950 | 33.797 / 10.989 / 9.654 | +831 (M14 + S) |
| `paint/*.js` (hepsi her açılışta yüklenir) | 3.492 / 1.498 / 1.269 | 25.999 / 8.870 / 7.745 | +7.372 (canvas 2.599, cpu 1.622, gpu 4.637) |
| JS toplam | 103.676 / 20.192 / 17.406 | 145.600 / 30.513 / 25.972 | +10.321 |
| **Toplam (WASM + JS)** | 1.453.461 gzip / 965.226 br | 1.493.743 gzip / 993.673 br | **+40.282 gzip (+%2,8); br +28.447** |

Telefonun LAN/HTTP kurulumunda gzip gider (M13): +40 KB ≈ 40 Mbit/s'te +8 ms.

## 6. Hedef tablosu (GOAL "Hedefler", üst sınıf telefon): görüntüleyici + `canvas`

"Tik" R4'te boyamayı içeriyordu; karşılaştırılabilir sayı **tik + geçiş**. p95 için üst
sınır: veri çerçevesinin tik p95'i + aynı evrenin geçiş p95'i (`chart_ticks`, sahne açık ve
tik akıyor — Arkin'in koşusunun durumu). PHONE_BIAS (CALIBRATION §4): yüz ×1,0, sahne ×1,85,
sahneli tik ×1,6; tik+geçişe ×1,6 uygulandı. **Önce** = §3'ün koşuları (JSON özetli değişiklik
sınaması; ileri/ters, 3+3 koşu ortancası); **sonra** = §6.2'nin parmak izli değişiklik
sınaması (ileri sıra, 3 koşu ortancası; yollar `ref, canvas`).

| Ölçü | Hedef | PHONE ham önce → sonra | PHONE × PHONE_BIAS önce → sonra | PHONE_JIT_EST önce → sonra | Durum (sonra) |
|---|---:|---:|---:|---:|---|
| Tik p95 (tik + geçiş, `chart_ticks`: grafik açık, tik akıyor) | ≤ 8 ms | 1 + 18–19 ≈ 19–20 → **1 + 10 = 11 ms** (ortalama 14,0 → **7,3**) | ≈ 30–32 → **≈ 18 ms** (ortalama ≈ 22 → **11,7**) | 1 + 2 = 3 → **1 + 1 = 2 ms** (ortalama 1,3 → 0,8) | PHONE ✗ (ortalama ham ✓), JIT ✓ |
| Tik p95, akış ekranı (`idle`) | ≤ 8 ms | 1 + 6–7 = 7–8 → **1 + 5 = 6 ms** (ortalama 1,5 → 1,3) | ≈ 11–13 → **≈ 10** | 1 + 2 = 3 → **1 + 1 = 2** | PHONE ham ✓, yanlılıkla ✗; JIT ✓ |
| Kaydırma karesi p95 | ≤ 16,7 ms | **18 ms** (p99 19–23; uzun pay 0,2–0,4 %) | 18 (yüz ×1,0; geçiş p95 12 → 1 kareye sığar) | 18 (uzun 0) | ✓ kaçan kare yok; 18 = 60 Hz + 1 ms'lik saat (ref de 18; T2 §1a) |
| İlk kapsül, soğuk | ≤ 1,5 s | 0,18 s döngüde + 1,55 MB aktarım (40 Mbit/s: 0,31 s) + ≈ 4 RTT (0,08 s) ≈ **0,57 s** | (yanlılık açılışa uygulanmaz) | — | ✓ (tahmin; WebKit'te ağ kısıtı yok) |
| İlk kapsül, Chromium 4× + 40 Mbit/20 ms | — | **540 ms** (ref ile 564) | | | ✓ |

İlk kapsül ölçümü (önce; değişiklik sınaması açılışı etkilemez): `tests/startup.mjs`
(`QP_JSC`, `QP_VIEWPORT`, `QP_QUERY` eklendi), DPR 3, 402×874, 5 taze bağlam, ortanca;
yük 7,3–8,2.

| Ayar | yol | WASM hazır | bağlandı | akış | ilk kapsül (min–maks) | aktarılan bayt (WASM, kodlama) | istek |
|---|---|---:|---:|---:|---:|---|---:|
| WebKit JIT'siz, döngü | canvas | 52 | 62 | 91 | **177** (161–212) | 1.549.885 (1.463.654, gzip) | 15 |
| aynı | ref | 51 | 60 | 81 | 205 (194–213) | aynı | 15 |
| Chromium 4×, 40 Mbit/20 ms | canvas | 384 | 418 | 475 | **540** (532–547) | 1.051.632 (968.119, br) | 12 |
| aynı | ref | 385 | 416 | 476 | 564 (557–570) | aynı | 12 |

(Arkin'in R4 ölçümü: ilk kapsül 3,9 s, WASM hazır 1,8 s. Kalan büyük kalemler M13 —
sıkıştırma, WebKit'in çift indirmesi — ve M15 — başlangıç çerçevesinde bar yok — ile kalktı.)

### 6.1 Boyama geçişinin dökümü: önce JSON özeti, sonra parmak izi

İlk analizde (`cpu` yoluyla karşılaştırarak) "geçişin ≈ 9 ms'si JSON `scene_digest`
değişiklik sınaması" çıkarımı yapılmıştı. Doğrulamak için çekirdeğe iki süre eklendi:
`scene_build` (açık sahnenin kurulması, `ShellCore::scene`) ve `change_check` (yüz/sahne
teslimindeki "değişti mi" anahtarı, `ShellCore::{face,scene}_changed`). Önce JSON özetiyle
ölçüldü (A), sonra yalnız anahtar değiştirildi (B). PHONE ve PHONE_JIT_EST, ileri sıra, yollar
`ref, canvas`, 3 koşu ortancası; ms; "/çerç." = veri çerçevesi başına toplam
(`rounds/RP3/experiments/S/breakdown.py`). Yük (1 dk): A 7,9–9,5; B PHONE 7,0–11,1, B JIT
6,8–11,2.

| Ayar | yol | evre | geçiş p50/p95 · /çerç. | sahne kurma p50/p95 · /çerç. | değişiklik sınaması p50/p95 · /çerç. | boyama (sahne/yüz) · /çerç. | tik+geçiş |
|---|---|---|---|---|---|---|---|
| PHONE A (JSON) | ref | chart_ticks | 16/51 · 19,0 | 3/5 · 3,0 | **6/11 · 7,0** | sahne 40/48 (n 6) · 8,2 | 19,6 |
| PHONE A (JSON) | canvas | chart_ticks | 14/19 · 13,9 | 3/5 · 3,2 | **8/11 · 7,6** | sahne 8/18 (n 2) · 0,8 | 14,4 |
| PHONE B (parmak izi) | ref | chart_ticks | 7/10 · 8,7 | 4/5 · 3,2 | **1/1 · 0,6** | sahne 36/42 (n 1) · 1,8 | **9,4** |
| PHONE B (parmak izi) | canvas | chart_ticks | 7/10 · 6,6 | 4/5 · 3,2 | **1/1 · 0,7** | sahne 8/9 (n 1) · 0,3 | **7,3** |
| PHONE A | ref | idle | 1/130 · 13,4 | — | 0/1 · 0,1 | yüz 32/37 (n 12) · 12,3 | 13,7 |
| PHONE A | canvas | idle | 0/7 · 0,9 | — | 0/1 · 0,2 | yüz 1/2 (n 4) · 0,2 | 1,3 |
| PHONE B | ref | idle | 1/45 · 5,2 | — | 0/0 · 0 | yüz 33/40 (n 4) · 4,4 | 5,6 |
| PHONE B | canvas | idle | 0/5 · 0,9 | — | 0/1 · 0 | yüz 1/2 (n 4) · 0,1 | 1,3 |
| JIT B | ref | chart_ticks | 1/3 · 1,3 | 0/1 · 0,4 | 0/0 · 0,03 | sahne 7/7 (n 1) · 0,2 | 1,5 |
| JIT B | canvas | chart_ticks | 1/1 · 0,7 | 0/1 · 0,3 | 0/0 · 0,03 | sahne 1/1 (n 2) · 0,03 | 0,8 |

Okuma:
- **Çıkarım doğrulandı:** JIT'siz kipte JSON özeti `chart_ticks`'te veri çerçevesi başına
  7,0–7,6 ms (p50 6–8). Parmak iziyle 0,6–0,7 ms (≈ 11 kat ucuz). `canvas` tik+geçiş
  14,4 → **7,3 ms** (−%49), geçiş p50 14 → 7; `ref` 19,6 → 9,4.
- **Kalan 6,6 ms/çerç. (canvas, PHONE):** sahne kurma 3,2 + sınama 0,7 + boyama 0,3 =
  4,2; geri kalan ≈ 2,4 ms geçişin kabuk tarafı (`core.open()` JSON'u ve çözümlemesi,
  katman metinleri, `renderEvidence`, akış DOM'u). Sıradaki en büyük kalem sahne kurma.
- **`ref`'in akış ekranındaki kazancı** (idle 13,7 → 5,6) başka bir etki: JSON özeti,
  piksellere ulaşmayan alanlar (katman verisi, eşleme) değişince de "değişti" diyordu;
  `ref` her seferinde yüzü baştan tarıyordu (yüz boyaması n 12 → 4; `canvas` zaten n 4, çünkü
  ilk analizdeki koşularda da 4'tü). Pikseller aynı kaldığı için görünen bir şey değişmedi
  (verify `demo` adımı, sahneler = altın görseller; `paint-paths`, `paint-canvas` geçti).
- Hedef: PHONE'da **ortalama** tik+geçiş artık 8 ms'nin altında (7,3), **p95** (1 + 10 =
  11 ms) ve yanlılıklı değer (≈ 18) hâlâ üstünde. JIT'li kipte 2 ms.

Gereken (büyükten küçüğe, ölçülene göre):
1. **Sahneyi hiç kurmamak**: M14 önerisi 1 — derleyici (qp-grammar) okuduğu bar aralığını
   versin; değişen bar aralık dışındaysa kurma yok. Bu evrede 31 çerçevenin 30'unda sahne
   değişmiyor: kurma (3,2) + sınama (0,7) kalkar → tik+geçiş ≈ 3 ms (kabuk payı kalır).
2. **Geçişin kabuk payı** (≈ 2,4 ms): sahne değişmediğinde `open()` JSON'u ve katman metinleri
   yeniden kurulmasın (değişiklik bayraklarına bağlanabilir).
3. **Güvenli bağlam (HTTPS)**: aynı kod JIT'li kipte 2 ms (p95).

## 7. Seçim önerisi (ana oturum kararı, ölçümle)

| | ref | **canvas** | cpu | gpu |
|---|---|---|---|---|
| PHONE yüz / sahne p50 | 32 / 24 | **2 / 6–7** | 25–26 / 7–8 | 3 / 8 |
| PHONE sürükleme uzun kare payı | 0,26 | **0,002** | 0,008 | 0 |
| PHONE tik+geçiş chart (§3, JSON sınaması) | 20 | 14 | **9** | 14–15 |
| PHONE tik+geçiş chart (§6.1, parmak izi) | 9,4 | **7,3** | (ölçülmedi; kendi sınaması zaten SipHash) | (ölçülmedi; aynı sınamayı kullanır) |
| Güvenli bağlam ister | hayır | **hayır** | hayır | **evet** (HTTP/LAN'da `ref`'e düşer) |
| Görsel fark | — | > 2 düzey ≤ %1,06 | **0** | > 2 düzey ≤ %0,31 |
| WASM belleği | taban | **taban** | +~33 MB | taban |
| Paket (gzip) | — | +~12 KB | +~12 KB | +~13 KB |

`canvas` varsayılan önerisi ölçümle tutarlı. `cpu`'nun §3'teki tik+geçiş üstünlüğü yoldan
değil, değişmeyen sahneyi ucuz tanımasından geliyordu; aynı teknik (parmak izi) artık bütün
yolların ortak değişiklik sınaması (§6.1) ve `canvas` 7,3 ms ile `cpu`'nun §3'teki 9'unun altında.
Seçim Arkin'in telefon ölçümüne kadar **öneri** (GOAL ölçüt 11).

## 8. Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo clippy -p
  qp-wasm --features replica --all-targets -D warnings`: temiz.
- `node shells/web/arch-check.mjs`: 0 ihlal (main.js + 6 boyama dosyası), öz-sınama 22/22.
- `QP_VERIFY_BROWSER=1 scripts/verify.sh` (commit `035c76d`, kod bu notla aynı): **18/18 PASS**, 524 s, çıkış 0; `demo` adımı: paint-paths (anlamsal okumalar aynı), canvas tolerans içinde, sahneler = altın görseller; `digest` 7b1e99d40889f83c.
- Altın görseller, dökümler, hesap özeti değişmedi (`fixtures/**` dokunulmadı).

## 9. Yapılmayanlar, açık sorular

- Telefonda ölçülmedi; bütün sayılar kalibre benzetim (PHONE) ve tahmin (PHONE_JIT_EST).
- WebKit'te ağ kısıtı yok; ilk kapsülün telefon tahmini bayttan ve RTT sayısından
  hesaplandı. Playwright WebKit'te HTTP önbelleği çalışmadığı için sıcak açılış ölçülmedi.
- `gpu` Chromium başsız kabukta (verify'ın tarayıcısı) hiç çalışmıyor; verify yalnız yedeği
  sınıyor. Gerçek GPU yolu yalnız `--channel chromium` ve WebKit'te (yerel) sınandı.
- `paint/*.js` dosyalarının hepsi her açılışta yükleniyor (`gpu.js` 4,6 KB gzip);
  `import()` mimari testince yasak (R6), seçilmeyen yolu tembel yüklemek kural değişikliği ister.

## Öneriler (kapsam dışı)

1. `DECISIONS.md`: "Önerilen (RP3): varsayılan boyama yolu `canvas`; `ref` başvuru ve yedek;
   `cpu`, `gpu` `?paint=` ile" (ana akış yazar). K12 için M12c'nin önerisi (vello yerine
   çekirdekte üçgenleme + ince WebGPU) ayrıca.
2. (Yapıldı, §6.1.) Sıradaki: değişmeyen sahnede `open()` JSON'u ve katman metinlerinin
   yeniden kurulmaması (geçişin ≈ 2,4 ms'lik kabuk payı).
3. qp-grammar derlemesinin bar aralığı bağımlılığı (M14 önerisi 1; §6.1-2).
4. Gösterimi HTTPS ile sunmak (iPhone'da JIT + WebGPU) — Arkin'in JIT'li ölçümü için şart.
5. `tests/gpu.mjs`'i (tam Chromium kanalıyla) ve `cpu-pixels.mjs`'i isteğe bağlı bir verify
   adımına bağlamak (bugün ~3–4 dk).
6. `docs/CONCEPTS.md`: "boyama yolu", "boyama geçişi", "komut akışı", "glif atlası".
