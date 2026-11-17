# RP3 — Kalibrasyon (T1)

Dal `rp3/t1-calib`, temel commit `6877bda` (R4 kodu, iyileştirme öncesi; VERIFY 3).
Amaç (GOAL ölçüt 1): masaüstü benzetimi Arkin'in iPhone ölçümünü (tik, yüz, sahne p50)
±%30 içinde yeniden üretsin; masaüstü → telefon farkının (≈30 kat) nedenleri ayrıştırılsın.

## 0. Özet

- **Farkın ana nedeni JIT'in kapalı olması.** iOS 27, güvenli bağlam olmayan sayfalarda
  (`http://`, yerel ağ IP'si) JS'i ve WASM'ı JIT'siz, yorumlayıcıyla çalıştırıyor. WebKit
  bunu kasıtlı bir "Enhanced Security" politikası olarak tanımlıyor ([WebKit 324968][b1]:
  iOS 27.0'da hem Safari hem Chrome-on-iOS (WKWebView) için; aynı sayfa `https://`
  üzerinden tam hızda). Arkin'in ölçümü tam bu koşulda alındı (`scripts/demo.sh --lan`,
  `http://192.168.x.y:8390/`, iOS 27.0.1, CriOS 157).
- Playwright WebKit 26.5'te JIT kapatılınca (`__XPC_JSC_useJIT=false`) **yüz boyama
  telefonla aynı çıkıyor (31 / 32 ms, ×0,97)**, sahne boyama telefonun 0,54'ü
  (38 / 71 ms). Telefonun tik p50'si (74 ms) tik başına sahne yeniden boyama oranına bağlı
  (aşağıda §4); sahneli tiklerde benzetim 45 ms (×0,61).
- Payları (sahne, log ölçekte): **JIT kapalı ≈ %55**, **piksel (DPR 1 → 3) ≈ %34**,
  açıklanamayan cihaz farkı ≈ %14, motor (V8 → JSC, JIT açık) ≈ %0 (−%3). Kopya ve
  `putImageData` < %3 (JIT kapalıyken), WASM derleme kipi (wasm-opt −%5, SIMD −%9)
  farkın nedeni değil, küçük birer kazanç.
- **R4'ün "4× ≈ orta sınıf" eşlemesi yanlıştı, ama yalnız "3 kat hafif" olduğu için değil:**
  CDP kısıtı her işi eşit yavaşlatır; yorumlayıcı WASM'ı JS'ten çok daha fazla yavaşlatır
  (basit döngüde WASM 23×, JS 4×). Sayısal olarak CDP 16× yüz ve sahneyi ±%30'a
  sokuyor (1,15 / 0,71), ama yanlış mekanizmayla: JS ağırlıklı bir boyama yolunu telefona
  göre yaklaşık 4–5 kat kötü gösterir.
- **Seçilen ayar (`PHONE`, `shells/web/tests/calibrate.mjs`):** WebKit 26.5, JIT kapalı,
  DPR 3, demo WASM'ı (SIMD yok, wasm-opt yok). Tek bir ayar üç ölçüyü birden ±%30'a
  sokmuyor; bilinen sapma: sahne yolu ×1,85 hafif, tik karışıma bağlı (§4).
- **Doğrulanması gereken en önemli şey:** aynı iPhone'un HTTPS (güvenli bağlam, JIT açık)
  üzerinden sayıları. Masaüstü WebKit'in JIT'li ölçümüne göre telefonda yüz ≈ 2–4 ms,
  sahne ≈ 3–6 ms beklenir — bugünkü kodla bile ≈ 10–15 kat daha iyi. RP3'ün boyama
  kararları iki kipte de ölçülmeli (Öneriler).

[b1]: https://bugs.webkit.org/show_bug.cgi?id=324968

## 1. Senaryo (`shells/web/tests/calibrate.mjs`)

Arkin'in telefon koşusu (`rounds/R4/experiments/D/phone-iphone-ios27-default.json`):
sentetik piyasa, 30 enstrüman, 4 adım/sn, adım 1017'de kopyalandı; tik n=309, sahne
n=213, yüz n=13, sürükleme kare n=104. Grafik koşunun çoğunda açıktı.

Otomatik karşılığı (bir koşu, taze `qp demo` ve taze tarayıcı bağlamı; tek yapılandırma
3 koşu ≈ 3–4 dk, tek koşu < 1,5 dk):

1. `qp demo --synthetic --paused` (hız 4 adım/sn), görünüm 402×874 CSS px, DPR yapılandırmadan.
2. Açılış → sayfanın kendi "Git" denetimiyle adım 700'e (telefon koşusunun penceresi
   ≈ 708–1017; `--seek`).
3. **Akış evresi** 8 sn: oynat, 4 sn dokunmadan, 4 sn tekerlekle kaydır → tik, yüz.
4. **Grafik evresi** 20 sn: durdur, akıştaki ilk sahneli kapsül grafik derinliğine açılır
   (`qp.openCapsule` + sahneye dokunma), oynat → tik, sahne.
5. Her evrenin başında seriler sıfırlanır, sonunda `window.qp.metrics()` okunur.

Kalibrasyon sayıları: **tik ve sahne grafik evresinden, yüz akış evresinden**. Çıktı: JSON
(koşuların medyanı, yük) ve Arkin'in sayılarına oranlar. Örnek:

```
node shells/web/tests/calibrate.mjs target/release/qp --config phone          # kalibre ayar
node shells/web/tests/calibrate.mjs target/release/qp --engine chromium --dpr 3 --rate 16
node shells/web/tests/calibrate.mjs target/release/qp --web <dizin> --jsc useJIT=false
```

Sonraki ajanlar için: `import { PHONE, PHONE_BIAS, launchPhone, runCalibrated } from
'./calibrate.mjs'`. `--web` başka bir WASM derlemesini ya da enstrümanlı sayfayı sunar.

**Masaüstü kontrolü** (`--config desktop`: Chromium, DPR 1, 1280×900): yüz 0,7 ms
(Arkin 0,9; ×0,78), akış tiki 1,1 ms (Arkin 2,8), sahne 0,7 ms (Arkin 2,3; n=8, açılış ve
geçiş anlarında). Yüz tutuyor; tik ve sahne Arkin'in masaüstü koşusunda daha yüksek
(başsız Chromium 151 / gerçek Chrome 156, ekranlı birleştirme, Arkin'in koşusunda akış
çoğunlukla görünür ve 1.258 tikte 567 yüz boyandı). Masaüstü sayıları yalnız payda olarak
kullanıldığı için oranlar §5'te benzetimin kendi masaüstü sayısıyla kuruldu.

## 2. Denenen düğmeler ve doğrulamaları

| Düğme | Ne yapıyor (doğrulandı) | Sonuç |
|---|---|---|
| Chromium CDP `setCPUThrottlingRate` | Ana iş parçacığını eşit oranda yavaşlatır | 4×, 16×, 20× denendi |
| DPR 1 / 2 / 3 | Çekirdek yuvarlanmış ölçekte boyar (piksel ×1/×4/×9) | sahne 390×300 → 1170×900 |
| Playwright WebKit 26.5 | JavaScriptCore; zamanlayıcı 1 ms adımlı (telefonla aynı), 60 Hz kare | — |
| **JSC JIT kapalı** | Düz `JSC_*` ortam değişkenleri WebContent sürecine ulaşmıyor; `__XPC_JSC_useJIT=false` ulaşıyor (`experiments/T1/jit.mjs`) | WASM döngüsü 28 → 643 ms (23×), JS 147 → 597 ms (4×) |
| JSC `useOMGJIT=false` (yalnız BBQ) | `__XPC_` ile | WASM 2,3× yavaş |
| `taskpolicy -b -p <WebContent>` | PRIO_DARWIN_BG; verimlilik çekirdekleri (`experiments/T1/oscpu.mjs`) | JIT açık 3,6×, kapalı 6× yavaş; yüklü makinede oynak |
| `taskpolicy -t 1…5 -p` | Etkisiz (ölçüldü: aynı süre) | — |
| WASM SIMD (`--features simd`, `+simd128`) | Derlemede `v128` var (wasm-opt doğrulaması) | JIT kapalı −%9, V8 −%2 |
| wasm-opt `-O3` | wasm-pack önbelleğindeki binaryen 117 (indirme yok) | JIT kapalı −%5; ham −%8, gzip +%4 |

Yorumlayıcı oranı WebKit hata kaydındaki telefon ölçümleriyle uyumlu: orada `http://`
/ `https://` oranı JS döngüsünde 4–5,6×, WASM kütüphanesinde (h3-js) ≈ 26×.

## 3. Ölçümler (3 koşu medyanı)

Makine: Apple M4 Max (12 P + 4 E çekirdek), macOS 27, Node 24.3, Playwright 1.62
(Chromium 151 başsız, WebKit 26.5). Makine başka ajanlarla paylaşımlı; her satırın
yükü (1 dk ortalaması, önce → sonra) tabloda. Yük > 12 iken 3 dk beklendi, düşmezse
koşuldu. WebKit süreleri 1 ms adımlı.

Oranlar Arkin'in iPhone p50'sine göre (tik 74, yüz 32, sahne 71; 1,0 = aynı).
"sahne/tik": grafik açıkken tik başına sahne boyama (açık kapsüle bağlı; bu senaryoda
belirleyici, 0,25).

| yapılandırma | yük | tik p50 / p95 | yüz p50 / p95 | sahne p50 / p95 | sahne/tik | akış tik p50 / p95 | oran tik · yüz · sahne |
|---|---|---:|---:|---:|---:|---:|---:|
| **wk-nojit-dpr3 (PHONE)** | 10,2 → 7,9 | 12 / 47 | **31 / 36** | **38 / 41** | 0,25 | 14 / 135 | 0,16 · **0,97** · **0,54** |
| wk-nojit-dpr3, adım 1000 (1 koşu) | 10,5 → 10,7 | 10 / 49 | 30 / 34 | 37 / 45 | 0,15 | — | 0,14 · 0,94 · 0,52 |
| wk-nojit-dpr2 | 8,6 → 7,7 | 12 / 31 | 18 / 20 | 22 / 24 | 0,25 | 14 / 95 | 0,16 · 0,56 · 0,31 |
| wk-nojit-dpr1 | 7,7 → 53 | 12 / 19 | 9 / 10 | 11 / 12 | 0,25 | 14 / 50 | 0,16 · 0,28 · 0,15 |
| wk-nojit-dpr3 + SIMD | 17,7 → 18,7 | 12 / 49 | 31 / 35 | 38 / 42 | 0,25 | 14 / 136 | 0,16 · 0,97 · 0,54 |
| wk-nojit-dpr3 + wasm-opt | 11,0 → 8,7 | 11 / 46 | 30 / 34 | 36 / 40 | 0,25 | 13 / 133 | 0,15 · 0,94 · 0,51 |
| wk-nojit-dpr3 + bg (E çekirdek) | 11,9 → 8,7 | 44 / 236 | 140 / 225 | 181 / 352 | 0,20 | 66 / 736 | 0,59 · 4,38 · 2,55 |
| wk-jit-dpr3 | 8,7 → 33,5 | 1 / 4 | 2 / 3 | 3 / 4 | 0,25 | 1 / 9 | 0,01 · 0,06 · 0,04 |
| wk-jit-dpr3 + SIMD | 35,0 → 31,6 | 1 / 4 | 2 / 3 | 3 / 4 | 0,25 | 1 / 11 | 0,01 · 0,06 · 0,04 |
| wk-jit-dpr3 + bg | 12,1 → 22,7 | 2 / 12 | 5 / 13 | 7 / 13 | 0,25 | 3 / 35 | 0,03 · 0,16 · 0,10 |
| cr-dpr3 1× | 50,0 → 45,8 | 1,0 / 4,4 | 2,4 / 2,9 | 3,4 / 3,9 | 0,25 | 1,2 / 10,6 | 0,01 · 0,07 · 0,05 |
| cr-dpr3 4× (R4 "orta sınıf") | 21,0 → 35,2 | 3,6 / 14,5 | 8,7 / 10,4 | 12,0 / 12,8 | 0,25 | 4,2 / 37,2 | 0,05 · 0,27 · 0,17 |
| **cr-dpr3 16× (CDP16)** | 10,9 → 10,0 | 14,9 / 60,0 | 36,8 / 41,9 | 50,4 / 53,8 | 0,25 | — | 0,20 · **1,15** · **0,71** |
| cr-dpr3 20× | 55,5 → 16,9 | 19,7 / 79,6 | 48,2 / 57,0 | 65,0 / 70,3 | 0,25 | 28,9 / 242 | 0,27 · 1,51 · 0,92 |
| desktop (cr, DPR 1) | 28,6 → 29,1 | 0,9 / 1,4 | 0,7 / 0,8 | 0,7 / 0,9 | 0,25 | 1,1 / 2,8 | — |

Ham sonuçlar: `rounds/RP3/experiments/T1/results/*.json` (yalnız süre; sentetik veri),
tablo `python3 -I rounds/RP3/experiments/T1/tables.py rounds/RP3/experiments/T1/results`.

Diğer okumalar (wk-nojit-dpr3): açılış WASM hazır 50 ms, ilk kapsül 685 ms (yerel döngü;
telefonda 1,8 / 3,9 s — Wi-Fi'de 4,4 MB sıkıştırılmamış WASM ve JIT'siz derleme/başlatma;
bu turda ölçülmedi); kare p95 akış durağan 23 ms, kaydırma 37 ms (p99 48), grafik açık
p99 56 ms (telefon: durağan p95 87, kaydırma p95 50 / p99 179). WASM belleği 64 MB
(telefon 56 MB). Oturum uzunluğu (adım 700 → 1000) tik başına işi değiştirmedi.

## 4. Seçilen ayar ve kalıntılar

**`PHONE` = Playwright WebKit 26.5, `__XPC_JSC_useJIT=false`, DPR 3, 402×874, demo WASM'ı
(`scripts/demo.sh` derlemesi: profil `wasm`, SIMD yok, wasm-opt yok), işletim sistemi
düğmesi yok.** Seçim gerekçesi: telefonun ölçüldüğü koşulun mekanizmasını (JIT'siz
JavaScriptCore) aynen kuran tek ayar; yüzü birebir tutturuyor; boyama yolları arasındaki
seçimde belirleyici olan JS / WASM maliyet oranını doğru veriyor (CDP bunu bozar).

| p50 (ms) | iPhone | PHONE | oran | ±%30 | CDP16 | oran |
|---|---:|---:|---:|:-:|---:|---:|
| Yüz | 32 | 31 | 0,97 | ✓ | 36,8 | 1,15 ✓ |
| Sahne | 71 | 38 | 0,54 | ✗ | 50,4 | 0,71 ✓ |
| Tik (bu senaryonun karışımı, sahne/tik 0,25; §4a: aynı derlemeyle 12–29) | 74 | 12 | 0,16 | ✗ | 14,9 | 0,20 ✗ |
| Tik, sahne boyanan tikler | 74 | 45 | 0,61 | ✗ | ≈ 61 (tahmin) | ≈ 0,8 |
| p95: yüz / sahne / tik | 72 / 92 / 118 | 36 / 41 / 47 | 0,50 / 0,45 / 0,40 | ✗ | 42 / 54 / 60 | 0,58 / 0,58 / 0,51 |

**Tek ayar üç ölçüyü birden ±%30'a sokmuyor.** Nedenleri:

1. **Tik, yeniden boyama karışımına bağlı.** Bu senaryoda grafik açıkken tiklerin %25'i
   sahneyi yeniden boyuyor (açık kapsüle göre %15–50; `experiments/T1/repaint-700.json`,
   8 kapsül); sahne değişmeyen tik JIT'siz 10 ms, sahneli tik 45 ms. Telefonun tik
   medyanı (74) sahne medyanına (71) neredeyse eşit; bu ancak telefon tiklerinin yarıdan
   fazlası sahne boyadıysa olur (213 sahne / 309 tik; bir kısmı sürükleme cevabı). Arkin'in
   hangi kapsülü açtığı bilinmiyor; benzetimin sahneli tik sayısı (45) telefonla sahnenin
   aynı oranında (×0,61). Yani tik kalıntısı, sahne kalıntısı + karışım farkıdır.
2. **Sahne yolu telefonda yüzden orantısız pahalı.** Telefonda sahne/yüz = 2,2,
   benzetimde 1,2–1,4 (her motorda, her kısıtta). Piksel oranı 1,6. Telefonda büyük
   tampon (1170×900, 4,2 MB) için piksel başına daha pahalı bir şey var (bellek bant
   genişliği / önbellek, hızlandırılmış tuvalin yüklenmesi, termal); masaüstünde hiçbir
   düğme bunu ayrı üretmiyor. Telefon yüz örneği küçük (n=13) — bu da bir belirsizlik.
3. **p95 telefonda daha geniş** (×2 dağılım): telefonda arka plan işi, çöp toplama,
   termal; benzetim dar dağılım veriyor. p95'ler kalibre değildir.

**Önerilen kullanım:** iyileştirmeler `PHONE` ile ölçülür; sayılar telefona şu çarpanlarla
okunur (`PHONE_BIAS`): yüz ×1,0, sahne ×1,85, sahneli tik ×1,6. Sıralama kararları
(hangi yol daha hızlı) çarpansız ham sayılarla verilir. CDP16 yalnız sayısal yedek:
Chromium'a özgü bir yolu (ör. WebGPU) telefonla aynı mertebede görmek için; JS ve WASM
paylarını karşılaştıran bir kararda kullanılmaz.

## 4a. Tik ortancasının kararsızlığı (son denetim sonrası düzeltme)

Son denetim (`AUDIT.md` B1/K1) PHONE tik p50'sini R4 kodunda (`6877bda`, aynı derleme
bayrakları) iki koşuda **29 ve 28 ms** buldu. Ana oturum aynı gün T1'in **kendi derlemesiyle**
(`T1/web-base`, aynı `qp`, aynı `calibrate.mjs`, WebKit 26.5, prizde) tekrarladı:

| koşu | saat | yük önce → sonra | tik p50 / p95 | yüz p50 / p95 | sahne p50 / p95 | sahne/tik | oran tik · yüz · sahne |
|---|---|---|---:|---:|---:|---:|---:|
| T1 (§3 tablosu) | 13:50 | 10,2 → 7,9 | 12 / 47 | 31 / 36 | 38 / 41 | 0,253 | 0,16 · 0,97 · 0,54 |
| T1 denetimi | öğleden sonra | — | 11,5 / 47 | 31 / 36 | 38 / 41 | 0,252 | 0,16 · 0,97 · 0,53 |
| son denetim #1, #2 (R4 `6877bda`) | 17:50–17:55 | 7,4 → 8,2 | 29 (28) / — | 31 (31) | 39 (39) | 0,253 | 0,39 (0,38) · 0,97 · 0,55 |
| ana oturum (T1 `web-base`) | 20:56–20:58 | 5,6 → 5,7 | 28 / 73 | 31 / 36 | 39 / 58 | 0,253 | 0,38 · 0,97 · 0,55 |

Aynı derleme, aynı motor sürümü, aynı açık kapsül (`cap.SYN16.breakout.up.200`) ve aynı
sahne/tik oranıyla yüz ve sahne birebir tekrarlanıyor; **tik ortancası 12'den 28 ms'ye
kayıyor**. Tik, ¾'ü boyamasız (≈10–14 ms), ¼'ü sahneli (≈45 ms) iki kümeden oluşuyor;
ortanca bu karışımda boyamasız kümenin üst ucuna düşüyor ve boyamasız tikin maliyetindeki
gün içi değişime (makinenin o anki durumu; neden bu turda ayrıştırılmadı) çok duyarlı.
Sonuç:

- Tik oranı **×0,16–0,38** aralığında; her iki uçta da ±%30 dışında. Ölçüt 1'in sonucu (⚠️,
  yalnız yüz tutuyor) değişmez.
- "Tik kalıntısı = sahne kalıntısı + karışım" yorumu nitelik olarak geçerli, ama 12 ms'ye
  dayanan nicel kısmı (boyamasız tik ×14) tek bir koşudan gelir; güvenilir değildir.
- `PHONE_BIAS.tick_with_scene` (1,6) sahneli tiklerin 45 ms'sinden türetildi; bu küme bu
  koşularda ayrıca yeniden ölçülmedi. Tahmin olarak kalır, kalibre sayı değildir.
- Bundan sonra tik için ortanca yerine sahneli / boyamasız tikler ayrı raporlanmalı (REPORT
  "Öneriler").

## 5. Ayrıştırma

### 5.1 Bir boyamanın parçaları

Ölçüm: enstrümanlı sayfa (`experiments/T1/main-probe.js`) ve sınama çekirdeği
(`experiments/T1/probe/`: R4 bağlamasının birebir kopyası + `probe_build`, `probe_raster`,
`probe_take`). Replay duruyor, grafik açık; her parça 8 (Chromium SIMD: 20) kez döngüde,
ortalama; 3 koşu medyanı (SIMD: 1 koşu). Sütunlar: kurma = sahne tarifinin derlenmesi
(gramer, çekirdek); tarama = tiny-skia + demultiply (WASM içinde); dışarı kopya = `Vec<u8>`'ın
JS'e kopyası; ImageData; put = `putImageData` (boyut değişmeden); boyut+put = tuval
boyutu yeniden atanıp put (bugünkü yol); put+okuma = put + 1 piksel geri okuma (ertelenmiş
yükleme varsa onu da öder); tam yol = `scene_rgba(true)`/`face_rgba` + bugünkü put (özet
karşılaştırması dahil).

| motor | ölçek | tür | piksel | kurma | tarama | kopya | put | boyut+put | put+okuma | tam yol |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| WebKit JIT'siz | 1 | sahne | 390×300 | 2,50 | 7,38 | 0,00 | 0,12 | 0,00 | 0,50 | 11,00 |
| WebKit JIT'siz | 2 | sahne | 780×600 | 2,50 | 19,88 | 0,25 | 0,12 | 0,12 | 0,75 | 24,75 |
| WebKit JIT'siz | 3 | sahne | 1170×900 | 2,75 | 38,12 | 0,25 | 0,38 | 0,25 | 1,25 | 43,50 |
| WebKit JIT'siz | 3 | yüz | 1170×564 | 1,38 | 29,25 | 0,25 | 0,25 | 0,12 | 1,25 | 30,75 |
| WebKit JIT'siz + SIMD | 3 | sahne | 1170×900 | 2,25 | 34,62 | 0,25 | 0,25 | 0,25 | 1,12 | 39,25 |
| WebKit JIT'li | 3 | sahne | 1170×900 | 0,12 | 2,00 | 0,38 | 0,25 | 0,25 | 1,12 | 2,75 |
| WebKit JIT'li | 3 | yüz | 1170×564 | 0,00 | 1,38 | 0,25 | 0,25 | 0,25 | 1,00 | 1,75 |
| Chromium (V8) | 1 | sahne | 390×300 | 0,07 | 0,36 | 0,04 | 0,01 | 0,02 | 0,04 | 0,55 |
| Chromium (V8) | 3 | sahne | 1170×900 | 0,07 | 1,83 | 0,40 | 0,19 | 0,15 | 0,15 | 2,59 |
| Chromium (V8) | 3 | yüz | 1170×564 | 0,04 | 1,19 | 0,20 | 0,12 | 0,09 | 0,09 | 1,53 |
| Chromium + SIMD | 3 | sahne | 1170×900 | 0,06 | 1,80 | 0,36 | 0,17 | 0,14 | 0,15 | 2,38 |

(ImageData oluşturma her yerde ≈ 0: bellek görünümü, kopya yok. WebKit'in JIT'li satırları
1 ms adımlı zamanlayıcı yüzünden ±0,2 ms belirsiz.)

Okuma:
- **JIT'siz yolda tarama her şeydir:** sahne 43,5 ms'nin 38,1'i (%88) tarama, 2,75'i
  (%6) kurma, ≈ 2 ms özet ve sarmalayıcı; kopya + put + boyut < 1 ms (%2–3). Geri okumalı
  put bile 1,25 ms.
- **JIT'li yolda kopya görünür hale gelir:** V8'de sahne 2,59 ms'nin 1,83'ü tarama
  (%71), dışarı kopya 0,40 + put 0,19 (%23).
- **Piksel ölçeklemesi:** tarama ≈ a + b·piksel; JIT'siz a ≈ 3,5 ms, b ≈ 33 ns/piksel; V8'de
  a ≈ 0,18 ms, b ≈ 1,6 ns/piksel (piksel başına 21 kat). DPR 1 → 3 (9 kat piksel) sahneyi
  JIT'siz 3,95, V8'de 4,7 kat pahalılaştırıyor.
- **SIMD:** JIT'siz −%9, V8'de −%2; wasm-opt JIT'siz −%5. Derleme kipi farkın nedeni değil.

### 5.2 Tik başına iş ve yeniden boyama sıklığı (grafik açıkken / akışta)

| | WebKit JIT'siz | WebKit JIT'li | Chromium |
|---|---:|---:|---:|
| `on_frame` (yansı, boyamasız) p50 | 4 ms | ≈ 0,2 ms | 0,2 ms |
| Boyamasız tik, grafik açık (yansı + değişmemiş sahnenin yeniden kurulup özetinin karşılaştırılması) | 10 ms | 1 ms | 0,7 ms |
| Boyamasız tik, akış | 14 ms | 1 ms | 0,9 ms |
| Sahneli tik p50 | 45 ms | 3 ms | 3,1 ms |
| Yüzlü tik p50 (yeni kapsül: görünen yüzlerin hepsi yeniden boyanır) | 162 ms | 8 ms | 8 ms |
| Grafik açık: tik başına sahne boyama | 0,23 (açık kapsüle göre 0,15–0,50) | | |
| Akış: tik başına yüz boyama (ortalama) | 0,53; tiklerin 1/8'inde 4 yüz birden | | |

Grafik açıkken her tik, sahne değişmese de sahne tarifini baştan kurup özetini
karşılaştırıyor (JIT'siz ≈ 6 ms, tikin %60'ı); yansının kendi işi 4 ms. Akışta yeni bir
kapsül gelince görünen 4 yüz baştan boyanıyor: o tik JIT'siz 162 ms (akış tiki p95 135).

### 5.3 Masaüstü → telefon farkının payları

Zincir, benzetimin kendi ölçümleriyle (span p50, ms); son halka telefona kalan kalıntı.
Pay = halkanın log çarpanı / toplam log çarpan.

| halka | yüz | çarpan | pay | sahne | çarpan | pay |
|---|---:|---:|---:|---:|---:|---:|
| masaüstü (Chromium, ölçek 1) | 0,7 | | | 0,7 | | |
| piksel: DPR 1 → 3 (Chromium) | 2,4 | ×3,4 | %32 | 3,4 | ×4,9 | %34 |
| motor: V8 → JSC, JIT açık | 2,0 | ×0,83 | −%5 | 3,0 | ×0,88 | −%3 |
| **JIT kapalı (JSC yorumlayıcı)** | 31 | ×15,5 | **%72** | 38 | ×12,7 | **%55** |
| kalıntı: telefon / benzetim | 32 | ×1,03 | %1 | 71 | ×1,87 | %14 |
| **toplam** | | ×46 | | | ×101 | |

Tik: boyamasız tik 0,7–0,9 → 10–14 ms (JIT kapalı ×14); telefonun tiki ayrıca **yeniden
boyama sıklığına** bağlı: benzetim karışımında (sahne/tik 0,25) medyan boyamasız tike
düşüyor (12 ms), telefonda sahneli tike (74 ms; benzetim 45, ×1,6). Bu halkanın payı
ölçülebilir değil, çünkü telefonun karışımı bilinmiyor.

Kopya + `putImageData`: JIT kapalıyken < %3, JIT açıkken ≈ %23 (yukarıda). WASM derleme
kipi: wasm-opt −%5, SIMD −%9 (JIT kapalı); farkın nedeni değil.

## 6. Tahminler (kalibre değil)

- **Aynı iPhone, JIT açık (HTTPS):** `PHONE_JIT_EST` = WebKit JIT'li, DPR 3: yüz 2, sahne 3,
  tik 1 (p95 4), yeni kapsüllü akış tiki ≈ 8–9 ms. Telefon/Mac oranı JIT'li kipte WebKit
  hata kaydına göre ≈ 1,3 (JS döngüsü 98 / 75 ms); kalıntıyla (×1,0–1,85) telefonda yüz
  ≈ 2–4 ms, sahne ≈ 3–6 ms, akış tiki p95 ≈ 10–20 ms beklenir. **Doğrulanmadı.**
- **Orta sınıf Android (4–6 yaşında, Chrome / V8, JIT açık):** Android'de bu politika yok.
  Tek çekirdek oranı (M4 sınıfı ≈ 3.900 Geekbench 6 tek çekirdek; Snapdragon 778G ≈ 1.100,
  Helio G99 ≈ 750) ≈ 3,5–5,5 kat → `MID_ANDROID_EST` = Chromium CDP 5×, DPR 3. 4× ölçümüne
  göre yüz ≈ 11, sahne ≈ 15 ms, akış tiki p95 ≈ 45 ms. **Tahmin; telefon ölçümü yok.**
  R4'ün 4–6× aralığı bu sınıf için kaba olarak yerinde olabilir; yanlış olan iPhone'a
  uygulanmasıydı (JIT'siz kip).

## 7. Boyama yolları için sonuçlar

1. **İki kip ayrı ölçülmeli.** JIT'siz kipte (Arkin'in ölçümü; iOS'ta her `http://` sayfa
   ve Kilit Modu) WASM'ın her komutu ≈ 20 kat, JS ≈ 4–6 kat pahalı: WASM'da yazılımla tarama
   en kötü seçenek; tarayıcının kendi (yerel kodla çalışan) tuval ilkelleri JS'ten
   çağrılınca yorumlayıcı cezası yalnız çağrı sayısı kadar. JIT'li kipte bugünkü yol
   telefonda büyük olasılıkla bütçenin içinde (§6) ve kopya + put payı öne çıkar.
2. **Saldırılması gereken maliyetler, büyükten küçüğe (JIT'siz):** (a) piksel başına
   tarama (sahnenin %88'i; DPR 3'te 9 kat piksel) — daha az piksel taramak (hareket
   sırasında düşük ölçek, kirli bölge, önbellekli katmanlar) ya da taramayı yerel koda
   bırakmak (Canvas 2D / GPU); (b) değişmeyen sahnenin her tikte yeniden kurulup
   özetlenmesi (grafik açıkken tikin %60'ı, 6 ms) — değişmeyi veriden bilmek;
   (c) yeni kapsülde görünen bütün yüzlerin yeniden boyanması (tek tikte 162 ms) —
   değişmeyen yüzü yeniden boyamamak, kare başına bir boyama (M14); (d) yansının tik
   başına işi (4 ms; K48 görüntüleyici biçimiyle evrenden bağımsız hale gelir).
3. Kopya ve `putImageData` JIT'siz kipte önemsiz; JIT'li kipte ≈ %23 — orada tek kopyalı
   yol (WASM belleğinden doğrudan ImageData, tuvali her seferinde yeniden boyutlamamak)
   değerlendirilebilir.
4. SIMD ve wasm-opt küçük (%2–9) ama bedava; açılış boyutuna etkisiyle birlikte (M13)
   tartılmalı (wasm-opt gzip'i %4 büyüttü).

## 8. Yapılamayanlar ve açık sorular

- Gerçek telefon yok: kalıntının (sahne ×1,85) kaynağı (bellek, tuval yüklemesi, termal)
  ayrıştırılamadı. iPhone'un JIT'li sayıları ölçülmeden §6 bir tahmindir.
- Arkin'in açtığı kapsül ve sürükleme/tik oranı bilinmiyor; tik kalıntısının bir kısmı bu.
- WebKit'in `performance.now()`'ı 1 ms adımlı: JIT'li WebKit satırları kaba; 8–20
  yinelemeli döngülerle hafifletildi.
- Makine yükü koşular boyunca 7,7–55 arası (1 dk ortalaması). Yüksek yüklü satırlar
  (cr-dpr3 1×, cr 20×, wk-jit-simd) aynı motorun düşük yüklü satırlarıyla tutarlı; yine
  de tek ölçüm olarak okunmamalı.
- Açılış (ilk kapsül 3,9 s) kalibre edilmedi: Wi-Fi, sıkıştırmasız paket ve JIT'siz
  başlatma ayrı ölçülmeli (M13).
- `taskpolicy -b` ile E çekirdeğine itme çalışıyor (3,6–6×) ama başka ajanların yükünde
  oynak; kalibre ayarda kullanılmadı.

## 9. Öneriler (kapsam dışı; yapılmadı)

- **Telefon ölçümünü HTTPS üzerinden de almak.** Gösterim barındırıcısına güvenli bağlam
  (yerel bir sertifika otoritesiyle HTTPS ya da bir tünel) eklenmeden iPhone'da JIT'li sayı
  alınamaz. Bu, RP3'ün "üst sınıf telefon" hedeflerinin hangi kipte ölçüleceğini belirler
  (ana akış kararı). WebKit, "Enable EnhancedSecurity Heuristic" bayrağının kapatılmasını
  da önermiş (Safari geliştirici ayarları; telefonda denenmeli).
- **Ölçüm özetine "JIT açık mı" göstergesi (T2):** ölçüm ekranı küçük bir WASM döngüsünü
  zamanlayıp özete yazsın (masaüstünde JIT açık ≈ 0,3 ms / 1M yineleme, kapalı ≈ 6 ms);
  her telefon özeti hangi kipte alındığını kendisi söylesin.
- Kilit Modu açık iPhone kullanıcıları JIT'siz kipte kalır; ürün hedefi için JIT'siz kipte
  bir taban hedef de yazılmalı (K17'ye girdi).
- Orta sınıf bir Android'de bir ölçüm (Chrome, JIT açık) `MID_ANDROID_EST`'i kalibre eder.

## 10. Dosyalar

- `shells/web/tests/calibrate.mjs` — senaryo, yapılandırmalar (`PHONE`, `PHONE_BIAS`,
  `PHONE_JIT_EST`, `MID_ANDROID_EST`, `DESKTOP`, `R4_MID`, `CDP16`), `launchPhone()`,
  `runCalibrated()`.
- `rounds/RP3/experiments/T1/setup.sh` — derleme çeşitleri (`web-base`, `web-simd`,
  `web-opt`, `web-probe`, `web-probe-simd`; üretilir, commit edilmez).
- `rounds/RP3/experiments/T1/matrix.sh` — §3 tablosunun koşusu; `tables.py` tabloları basar.
- `rounds/RP3/experiments/T1/probe/`, `main-probe.js` — ayrıştırma çekirdeği ve sayfası
  (yalnız deney; kabuk ve çekirdek değişmedi).
- `rounds/RP3/experiments/T1/jit.mjs`, `oscpu.mjs` (`results-oscpu.json`), `repaint.mjs`
  (`repaint-700.json`) — düğme doğrulamaları ve yeniden boyama sıklığı.
- `rounds/R4/DEVICE.md` sonuna "RP3 düzeltmesi".
