# RP3 — Bağımsız son denetim (görev A)

Denetçi: bağımsız alt ajan; RP3'ün hiçbir alt görevini yapmadı. Tarih 2026-11-17.
İncelenen: `main` @ `81215d0` (round'un son durumu), başvuru `6877bda` (R4 kodu).
Kod değiştirilmedi, birleştirme yapılmadı; bu dosya commit edilmedi.
Portlar: 8500–8507 (verify'ın kendi demo portları 8396–8398). R4 derlemesi
scratchpad'de ayrı bir worktree'de yapıldı ve iş sonunda silindi.

Makine notu: paylaşımlı makine, yük (1 dk) 4–13 arası; bir ara makine pil gücünde
uykuya geçti (`pmset -g log`: 20:31 "Maintenance Sleep", 20:40 uyanma) ve o sırada
yük 37–132'ye çıktı. O pencerede alınan üç PHONE koşusu ve iki paint-paths koşusu
zaman aşımına düştü ("page did not settle", `waitForFunction 200000ms`); uyanmadan sonra
aynı komutlar ilk denemede geçti. Bu başarısızlıklar ölçüme sayılmadı; aşağıdaki
bütün sayılar uyanma sonrası ya da uyku öncesi koşulardandır.

## Karar: **KOŞULLU KABUL**

Round'un iddiaları büyük ölçüde yeniden üretildi: verify 18/18, boyama yolları ve
yedekleri iki motorda açılıyor, anlamsal okumalar yoldan bağımsız, otomatik karşılaştırma
başsız ve elle hareketsiz bitiyor, seçilen yolun PHONE sayıları raporla ±%20 içinde,
sıkıştırma, bellek ve görüntüleyici bağımsızlığı doğrulandı, kabukta alan mantığı yok,
var olan altınlar/dökümler/hesap özeti değişmedi. İki ⚠️ ölçüt dürüstçe raporlanmış.
Koşul olan tek önemli bulgu: **kalibrasyonun tik sayısı bugün yeniden üretilmiyor**
(B1). Ölçüt 1'in durumu (⚠️) bundan değişmiyor, ama `CALIBRATION.md`/`REPORT.md`'deki
"tik 12 ms, ×0,16" sayısı düzeltilmeli ya da yeniden ölçülmeli.

### Koşullar

1. **K1 (B1):** `CALIBRATION.md` §3–4 ve `REPORT.md` §2 satır 1'deki PHONE tik p50'si
   (12 ms, ×0,16) bu denetimde iki ayrı koşuda 29 ve 28 ms (×0,39 / ×0,38) çıktı; yüz ve
   sahne birebir tuttu. Ya fark açıklanmalı (ör. ortam değişikliği) ya da sayı her iki
   ölçümle birlikte yazılmalı. Ölçütün sonucu (⚠️, ±%30 dışı) değişmez.
2. **K2:** CI, `main` gönderildikten sonra ana oturumda koşulmalı (`main`, `origin/main`'in
   64 commit önünde; son CI koşusu R4'e ait). VERIFY 2'nin CI kısmı bu denetimde
   **bekliyor**.

## VERIFY satırları

| # | Komut / kontrol | Denetçinin kanıtı | Sonuç |
|---|---|---|---|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | 17:43:47 → 17:50:32, `real 404.40`, `EXIT=0`; 18/18 PASS (fmt, clippy, test 298 s, golden "byte-identical to golden", views 12×2, digest `arm64, x86_64, wasm32: 7b1e99d40889f83c`, conform 59 döküm × 3, replica "viewer (lag 0, 25)", shell, demo 76 s "paint paths (semantic reads identical); canvas within tolerance; … scenes = goldens"). Yük 4,4 → 7,9 | ✅ |
| 2 | `scripts/preflight.sh`; CI | `preflight: TAMAM` (origin, ssh, qgh hesabı, yazma izni, e-posta PASS). CI: `main` gönderilmedi (`ahead 64`); `scripts/qgh run list` son koşu R4 (başarılı). | preflight ✅, CI **bekliyor** (K2) |
| 3 | Kalibre benzetim, R4 kodunda | `6877bda` scratch worktree'de derlendi (wasm-pack `--profile wasm --no-opt --features replica`, strip; `qp` release); T1'in `calibrate.mjs`'i (`0d37f3c`). `node tests/calibrate.mjs <r4 qp> --config phone --port 8500` (yük 7,4 → 5,8; 1 dk 55 s, 3 koşu) ve `--port 8501 --raw` (yük 5,6 → 8,2). Sonuç (ikinci koşu parantezde): **yüz 31 (31) ms ×0,97 ✓**, **sahne 39 (39) ms ×0,55 ✗**, **tik 29 (28) ms ×0,39 (×0,38) ✗**; sahne/tik 0,253, açılan kapsül `cap.SYN16.breakout.up.200` (rapordakiyle aynı). Rapor: yüz 31, sahne 38, tik 12. Koşu başına grafik tiki ortalaması 27,9 / 29,7 / 29,1 ms. | Yüz ve sahne yeniden üretildi; ölçüt yalnız yüzde tutuyor (raporla aynı). **Tik raporla tutmuyor** (B1) |
| 4 | Aynı benzetim, seçilen yolla | `QP_DEMO_PACK_ONLY=1 scripts/demo.sh` (WASM gzip 1.463.647 B), `RUNS=3 ORDERS=forward PATHS=ref,canvas rounds/RP3/experiments/S/measure.sh … phone` (20:45–20:48, yük 7,2–13,3; her koşu 68,1 s, 0 hata). `tables.py` ortancaları — **canvas:** yüz p50 1 (rapor 2), sahne p50 4 (rapor 7), tik+geçiş akış/grafik **1,34 / 7,06** (rapor 1,3 / 7,3), grafik geçişi p95 10 → tik+geçiş p95 1+10 = **11** (rapor 11), sürükleme karesi p95 18, uzun pay 0,000 (rapor 19 / 0,002). **ref:** yüz 34 (32), sahne 23 (24), tik+geçiş 5,29 / 8,61 (5,6 / 9,4), sürükleme p95 27, uzun 0,237 (29 / 0,263). | ✅ ±%20 içinde; yalnız canvas sahne p50 (4 ↔ 7) dışarıda ve iyi yönde (B4) |
| 5 | Her yol `?paint=` ile açılır; WebGPU yokken yedek | `node tests/paint-paths.mjs … --browsers webkit --port 8504` (19,7 s): ref, canvas, cpu, gpu kendi yolları (`fallback:null`), varsayılan `canvas`, "paint-paths: ok". Aynısı `--browsers chromium` (11,6 s): `gpu` → `{"fallback":"unsupported","used":"ref"}` (başsız kabukta adaptör yok), diğerleri kendi yolu. verify `demo` adımı da aynı sonucu verdi. | ✅ |
| 6 | Otomatik karşılaştırma başsız | `node tests/compare.mjs <qp> --port 8505 --out …` (20:40:56 → 20:44:56, yük 36,7 → 6,7; uyanma sonrası düşen yük): **Chromium 101,7 s** (120 Hz, 3 yol: ref, canvas, cpu), **WebKit 135,6 s** (60 Hz, 4 yol: + gpu). Her tarayıcıda tek özet (`compare-<b>.json`: `paths`, her yolda 4 evre `idle/scroll/chart_ticks/drag`, `mode` {jit, secure_context, webgpu}), betik yalnız düğmeye dokunuyor. | ✅ (her biri < 180 s). Not: raporun "139 s (denetçi 140 s)" ifadesi B6 |
| 7 | Seçilen yol ↔ referans görsel fark | verify günlüğü (`paint-canvas.mjs`, Chromium DPR 1, 16 görüntü: akış yüzleri, kapsül ve grafik sahneleri, kapalı/rozet/fotolay görünümleri): "> 2 düzey" en kötü **%1,062** (`feed-tr-dark-fotolay.face-1`), "> 32" en kötü **%0,021**, maks kanal farkı 107; kısmi = tam 7/7; `FontFace` yokken `ref`. Rapor: %1,06 (DPR 1), > 32 %0,021, maks 107. | ✅ fark raporla aynı |
| 8 | `screen` ve nokta sorgusu yoldan bağımsız | paint-paths (yukarıda), iki motorda: "canvas/cpu/gpu = ref: digests, feed, open, 10 authority answers (tap_at/hit, screen) byte-identical". | ✅ |
| 9 | Mimari testi + denetçinin taraması | verify `shell` PASS. Kendi taramam aşağıda (§Mimari). | ✅ |
| 10 | 5.000 adım bellek | `node rounds/RP3/experiments/T4/mem.mjs target/release/qp --browser chromium --variant feed --steps 5000 --every 1000 --port 8507` — **görüntüleyici kipi (artık varsayılan)**: WASM 6,95 → 7,14 → 7,14 → 7,34 → 7,34 → **7,34 MB** (adım 0 → 5.020), JS yığını 1,9–2,6 MB, tuval 41 sabit, düğüm 623–1.589 arası oynuyor (büyümüyor). Raporun 17,5 → 102 MB'ı tam yansı kipine aittir (rapor da böyle diyor; T4-verify o kipi yeniden üretmiş); tam yansıyı bu denetimde yeniden koşmadım. | ✅ sınırsız büyüme yok (görüntüleyici); tam yansı büyümesi A34'te yazılı |
| 11 | Sıkıştırılmış paket | `qp demo --synthetic --port 8500 --web shells/web`; `curl -D -`: AE yok → kodlamasız 4.958.369 B; `gzip` → `Content-Encoding: gzip`, 1.463.647 B; `br` ve `gzip, deflate, br` → `br`, 966.645 B; her cevapta `Vary: Accept-Encoding` ve kodlamaya özgü `ETag`; yanlış `Host` → 403. | ✅ (M13'ün önce/sonra tablosuyla uyumlu: WASM gzip 1,46 MB) |
| 12 | 60 Hz uzun kare; uzun açık sayfada açılış | `metrics_t2` testleri verify'da 10/10 ok; `a_60hz_integer_rounded_series_has_no_long_frames` (16/17/17 dizisi → `long` 0, 60 Hz algılanıyor), `the_opening_timeline_survives_a_later_start_frame` (836.034 ms senaryosu → ilk değer korunuyor, akla yatmayan işaret `open_flags`'te). Gerçek koşu: WebKit 60 Hz karşılaştırmasında uzun pay 0–0,006 (R4 kodunun aynı motordaki `long_16_7`'si benim kalibrasyon koşumda 0,56). | ✅ |
| 13 | Altınlar, dökümler, hesap özeti | `git diff 6877bda..HEAD --stat -- fixtures/`: **yalnız** `fixtures/protocol/conformance/59-viewer.json` (+999). Altınlar bayt eşit ve özet `7b1e99d40889f83c` (verify 6, 7, 12, 18). | ✅ |
| 14 | Gizli bilgi; `fixtures/local` | `git ls-files fixtures/local` boş; izlenen dosyalarda `.env`/anahtar adı yalnız `scripts/qapsule-gh.env` (R4'ten beri, bu round değişmedi). `git diff 6877bda..HEAD` eklenen satırlarda belirteç/anahtar kalıbı taraması (ghp_, github_pat_, sk-, AKIA, özel anahtar başlığı, xox, Bearer, api_key/password atamaları): eşleşme yok. Eklenen ham dosyalar (`T1/results/*.json`, `T4/raw/*.jsonl`) yalnız süre/bellek/yük; örnek: `wk-nojit-dpr3.json` (cfg, load, span özetleri), `after-chart.jsonl` (wasm, js_heap, nodes). Görsel ya da bar verisi eklenmedi (diffte png/csv/parquet yok). Notlardaki "THYAO/AKBNK" yalnız test adları. | ✅ |
| 15 | Evren 30/300(/3.000) | `node tests/viewer-measure.mjs <qp> 30|300 … --reps 2 --modes viewer --port 8506`: akış iş/adım 0,28–0,43 (30) ↔ 0,22–0,27 ms (300); açık 0,21 ↔ 0,16; WASM 9,0–9,4 ↔ 9,0 MB; JS yığını 2,75 ↔ 2,75 MB; bayt/adım 7.193 ↔ 7.325. 3.000 koşulmadı (isteğe bağlı). | ✅ evrenden bağımsız (WASM ve yığın ±%5; ms alt-milisaniye ve büyümüyor). Mutlak sayılar M15 tablosundan küçük (B5) |
| 16 | Güven işareti; tutarlılık | Metinler çekirdekte (`qp-session/src/viewer.rs:516,520`: "Sunucu doğruladı" / "Bu cihazda sınandı"), kabuk yalnız `textContent` yazıyor (`trustOf`). viewer-measure: açık kapsülde `trust_open` = "Bu cihazda sınandı", sınama `state: device`. verify `replica`: `qp replica-tape --viewer` gecikme 0 ve 25 PASS (ihlal 0, diğer enstrümana abonelik yok). | ✅ |
| 17 | Abonelikler | viewer-measure `subs_feed`: `data: []`, `bars: 0`; `subs_open`: tek besleme (`SYN21` / `SYN110`, `1d`), 121 bar. | ✅ |

## GOAL ölçütleri

| # | Ölçüt | Raporun durumu | Denetçinin görüşü ve kanıtı |
|---|---|---|---|
| 0 | İş bölümü (K48) | ✅ | ✅ VERIFY 15–17; PROTOCOL §22 mevcut (§22.1–22.6); yeni döküm 59, eski 58 değişmedi; describe kaynaklı iki test sabiti değişikliği M15-verify'da gerekçeli. |
| 1 | Kalibrasyon | ⚠️ | ⚠️ **doğru etiket.** Yüz (31/32) ve sahne (39/71, ×0,55) yeniden üretildi. Tik bugün 28–29 ms (×0,38–0,39), raporun 12 ms'si değil (B1). Nedenlerin ayrıştırılması (`CALIBRATION.md` §5) belgeli; `DEVICE.md` düzeltmesi var. Ayrıştırmayı yeniden koşmadım. |
| 2 | Üç boyama yolu | ✅ | ✅ Üç ayrı dal ve not (`M12a/b/c.md`); `S.md` §4 görsel fark; `cpu`/`gpu` piksel testlerini (`cpu-pixels`, `gpu.mjs`) yeniden koşmadım; `canvas` farkı verify'da yeniden üretildi. Körlük iddiasını süreç olarak denetleyemem (ajan yönergeleri depoda değil). |
| 3 | Hedefler, kalibre benzetimde | ⚠️ | ⚠️ **doğru etiket.** Grafik açıkken tik+geçiş p95 1+10 = 11 ms > 8 (benim ölçümüm aynı); ortalama 7,06. Kaydırma p95 18 (60 Hz + 1 ms saat; kaçan kare yok, uzun pay 0). Nedenin dökümü `S.md` §6.1'de ölçülü. İlk kapsül ≈0,57 s bir **tahmindir** (rapor da öyle diyor). |
| 4 | Varsayılan ve adres parametresi | ✅ | ✅ `DEFAULT_PAINT = 'canvas'`, `?paint=` seçimi, bilinmeyen/desteklenmeyen → `ref` + neden (`paint/index.js`); paint-paths iki motorda. |
| 5 | Açılış | ✅ | ✅ VERIFY 11; önce/sonra tablosu M13-verify'da bağımsız yeniden üretilmiş. Önce/sonra zamanlamasını yeniden koşmadım. |
| 6 | Güncelleme sıklığı | ✅ | ✅ WebKit karşılaştırma özetinde `passes_per_data_frame` 1,03, `faces_skipped_hidden` > 0 (ör. idle 108), `frame_to_paint` p95 ≤ 18 ms (≤ 1 kare). |
| 7 | Bellek | ✅ | ✅ VERIFY 10 (görüntüleyici sınırlı); tam yansı büyümesi kaynaklarıyla yazılı ve A34'e açık karar olarak bırakılmış. |
| 8 | Otomatik karşılaştırma | ✅ | ✅ VERIFY 6. |
| 9 | Ölçüm ekranı hataları | ✅ | ✅ VERIFY 12; DPR uyarısı `zoom_and_scale_warnings` testinde. |
| 10 | Değişmeyenler | ✅ | ✅ VERIFY 1, 7, 8, 9, 13. |
| 11 | Arkin için yönerge | ✅ | ✅ küçük düzeltmeyle (B3): §8 çalıştırılabilir; düğme adları (`Ölçüm`, `Karşılaştır`, `Karşılaştırmayı kopyala`, `Kopyalandı`), `demo.sh --lan`'ın yazdığı satır (`telefondan aç / open on your phone (same network): …`), `qp demo --https-host` (boş ad reddediliyor: `qp: --https-host: empty name`, çıkış 2) ve özetin `mode` alanı (`secure_context`, `jit.state` `likely_on/likely_off`) kodla örtüşüyor. Seçim `DECISIONS.md` "Önerilen (RP3) — kesinleşmedi" altında Ö52–Ö60, A34–A38. |
| 12 | Belgeler | ✅ | ✅ (bu dosyayla) `REPORT`, `RESEARCH`, `CALIBRATION`, `STATUS`, `ROADMAP` commit'li (`81215d0`). |

## Mimari (VERIFY 9) — denetçinin kendi taraması

`shells/web/main.js` (6877bda'dan beri eklenenler), `paint/{index,ref,canvas,cpu,gpu,fixture}.js`
okundu.

- `canvas.js`: komut akışını yineleyiciyle okuyup Canvas 2D çağrılarına birebir eşliyor;
  koordinat, renk, yazı tipi ve metin dizgileri çekirdekten (`canvas_strings`) geliyor.
  Aritmetik yok; tek sabit `ctx.arc(…, 0, n())`'deki başlangıç açısı. `ops[5] === CLIP`
  yalnız ölçüm etiketi için.
- `gpu.js`: köşe/renk/atlas baytları ve çizim listesi çekirdekten; kabuk yalnız yükleyip
  verilen sırayla çiziyor. **WGSL dizgesi** (`SHADER`): `o.pos = vec4f(p,0,1)`,
  `textureLoad(atlas, vec2i(floor(i.uv)))`, `i.color * a` — genel bir doku × renk
  karışımı; alan değeri (fiyat, zaman, yerleşim) içermiyor. ARCH.md'de not olarak yazılı.
  Mimari testinin dizge içeriğine bakmadığı doğru; bu dizge alan mantığı değil.
- `cpu.js`: çekirdeğin döndürdüğü dizi alanları (`r[0..10]`) doğrudan `putImageData`
  argümanı; hesap yok. `ref.js`: R4 ile aynı.
- `main.js`: `trustOf` yalnız çekirdek metnini yazıyor; karşılaştırma sürücüsü
  (`bench_start/tick/rows/summary`) çekirdekte, kabuk yalnız `scrollTop` atıyor, sekme
  değiştiriyor ve tabloyu çekirdeğin satırlarından kuruyor. Kabuktaki metin sözlüğü
  (`compare: 'Karşılaştır'`, `b_hint`) arayüz etiketi, piyasa verisinden metin kurma değil.
- Kabuğun değerlendirdiği, çekirdekten gelen dizgeler: `JSON.parse` (değişiklikler,
  akış, tablo, `canvas_strings`) ve `ctx.font/fillStyle` atamaları — kod olarak
  değerlendirilen dizge yok (`eval`/`Function` yok; dinamik `import()` mimari testinde yasak).

Sonuç: kabukta alan mantığı bulmadım.

## Süreç kuralları

- **Ayrı denetçi notu birleştirmeden önce:** birinci ebeveyn geçmişinde her birleştirmeden
  önce bir denetim notu commit'i var: M13 (`77794d7` → `a7c09fb`), T2 (`8a6856c` → `ea0dd7f`),
  T4 (`f45da6b` → `53d2fb7`), T1 (`c4939f1` → `8d78c16`), M15 (`f70e32a` → `0512196`),
  S + M12a/b/c + M14 (`7bda6e3` → `4c7868a`), HTTPS vekili (S-verify §g → `40290cc`).
  Notlar: `experiments/{T1,T2,T4,M13,M15,S}-verify.md`.
- `8952d25` birleştirmeden sonra doğrudan `main`'de küçük bir kod değişikliği (boş
  `--https-host` reddi, 3 satır) — S denetiminin önerisi; ayrı denetçiden geçmedi (B7).
  Davranışını doğruladım (çıkış 2, "empty name").
- `NORTH_STAR.md`: `git diff 6877bda..HEAD -- NORTH_STAR.md` boş.
- "Önerilen (RP3)" öneri olarak yazılmış ("kesinleşmedi"), kanıt bağlantılarıyla.
- Paylaşımlı ağaç temiz (`git status` boş).

## Bulgular

| # | Önem | Bulgu |
|---|---|---|
| B1 | **önemli** | PHONE tik p50'si R4 kodunda yeniden üretilmiyor: rapor 12 ms (×0,16; T1-verify o gün 11,5), denetim 29 / 28 ms (×0,39 / ×0,38), aynı açık kapsül ve aynı sahne/tik oranı (0,253); yüz (31) ve sahne (39 ↔ 38) aynı. Koşu başına ortalama 28–30 ms; boyamasız grafik tiki raporun 10 ms'sinden pahalı görünüyor. Neden bilinmiyor (ortam/sürüm farkı olabilir). Ölçüt 1'in sonucu değişmez; `CALIBRATION.md`'nin "tik kalıntısı = sahne kalıntısı + karışım" yorumu ve `PHONE_BIAS.tick_with_scene` bu sayıya dayanıyor. → K1 |
| B2 | önemli (süreç) | CI henüz koşmadı; `main` gönderilmedi. → K2 |
| B3 | küçük | REPORT §8 A.3 "Alt sekmelerden **Ölçüm**'e geçin" diyor; sekmeler başlıkta (üstte, `header > nav#tabs`). http'de `navigator.clipboard` yok: "Kopyalandı" bildirimi çıkmaz, `execCommand('copy')` denenir ve metin seçili kalır — rapor bunu "pano izni yoksa metin seçili gelir" diye kabaca söylüyor. |
| B4 | küçük | REPORT §4 tablosunda `canvas` sahne p50 7 ms, `HEAD`'de 4 ms (yüz 2 → 1). Dipnot bu sütunların eski (JSON özetli) koşulardan geldiğini söylüyor; sayı güncel kodu temsil etmiyor (iyi yönde). |
| B5 | küçük | M15 tablosunun mutlak sayıları (akış tik işi 1,66 ms/adım, WASM 13,8 MB) M14/S öncesi koda ait; `viewer-measure.mjs` bugün 0,2–0,4 ms/adım ve 9 MB veriyor, çünkü M14'ten beri `tick` boyamayı içermiyor (betik başlığı hâlâ "both including the paints they cause" diyor). Bağımsızlık iddiası etkilenmiyor. |
| B6 | küçük | REPORT ölçüt 8: "Chromium + WebKit başsız: 139 s (denetçi 140 s)". 140 s T2-verify'ın iki yollu (ref, fixture) koşusunun iki tarayıcı toplamıydı; bugünkü dört yollu düzenekte tarayıcı başına 101,7 s (Chromium) ve 135,6 s (WebKit), toplam 237 s. Ölçüt tarayıcı başına < 3 dk ve tutuyor; ifade karışık. |
| B7 | küçük | `8952d25` (kod, 3 satır) birleştirme sonrası doğrudan `main`'de, ayrı dal ve ayrı denetim olmadan (CLAUDE.md "her alt görev kendi dalında"). Etkisi küçük, verify sonrası değil öncesi (verify bu HEAD'de yeşil). |
| B8 | küçük | Paylaşımlı makinede ölçüm düzeneğinin uykuya/aşırı yüke karşı korumasız olduğu görüldü: `compare.mjs` ve `lib.mjs settle` zaman aşımında yalnız "TimeoutError" yazıyor; yük ve uyku bilgisi günlükte yok. Öneri: başarısızlıkta `uptime` ve hangi evrede kalındığını yazmak. |

## Yeniden üretilmeyen / koşulmayanlar

- Kalibrasyonun ayrıştırma tabloları (§5: probe, DPR 1/2, CDP 4×/16×/20×, SIMD, wasm-opt).
- PHONE_JIT_EST, CDP4 ve `cpu`/`gpu` yollarının PHONE ölçümleri; `cpu-pixels.mjs`, `gpu.mjs`;
  `paint-canvas.mjs` WebKit ve DPR 3.
- Tam yansı kipinde 5.000 adım belleği; 3.000 enstrümanlı görüntüleyici koşusu.
- M13 önce/sonra açılış zamanlamaları (M13-verify bağımsız üretmiş).

## Zaman ve yük kaydı (`uptime`, 1 dk ortalama)

| Ölçüm | Saat | Yük önce → sonra |
|---|---|---|
| verify.sh (404 s) | 17:43–17:50 | 4,42 → 7,95 |
| calibrate phone #1 (R4) | 17:50–17:52 | 7,42 → 5,62 |
| calibrate phone #2 (R4, --raw) | 17:53–17:55 | 5,78 → 8,16 |
| measure.sh phone (ilk deneme, başarısız: makine uykusu) | 17:56–18:26 | 6,18 → 108 |
| paint-paths chromium+webkit (başarısız, uyku) | 20:14–20:30 | 58,8 → 54,4 |
| paint-paths webkit | 20:30–20:31 | 41,7 → 33,6 |
| paint-paths chromium | 20:40 | 46,0 → 39,5 |
| compare.mjs chromium+webkit | 20:40:56–20:44:56 | 36,7 → 6,7 |
| measure.sh phone ref,canvas ×3 | 20:45–20:48 | 7,9 → 8,7 |
| viewer-measure 30, 300 | 20:51–20:52 | 8,2 → 8,1 |
| mem.mjs 5.000 adım (80 s) | 20:52–20:53 | 7,9 → 7,6 |
