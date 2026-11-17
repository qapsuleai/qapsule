# RP3 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Kalibrasyon: Arkin'in iki ölçüm dosyasından yola çıkarak benzetimi ayarla; nedenleri tek tek aç-kapa yaparak ayrıştır (DPR 1/2/3, işlemci kısıtı, tampon kopyası, yeniden boyama sıklığı, WASM derleme seçenekleri, SIMD) | `rounds/RP3/CALIBRATION.md`, `shells/web/tests/` | — |
| T2 | Ölçüm ekranı düzeltmeleri ve otomatik karşılaştırma sınaması (alternatiflerden önce; boyama yolu takılabilir arayüz) | `shells/web/`, `crates/qp-wasm/` | — |
| T3 | Araştırma: tarayıcıda 2B çizim yolları, mobil WebKit ve Chromium'un güncel davranışı (güncel belgelerden), benzer ürünlerin yaklaşımı; aday listesi ve beklenen kazanç | `rounds/RP3/RESEARCH.md` | — |
| M15a–b | Cihazdaki iş bölümü (K48): görüntüleyici biçimi ve varsa ara biçim, ayrı dallarda. Protokol eki (görünürlük aboneliği, akış kapsüllerinin güveni, açılışta veri bağı ve doğrulama), 30/300/3.000 enstrümanlı sentetik evren, ölçüm. **Boyama yollarından önce**: boyama, görüntüleyici biçimi üzerinde ölçülür | `crates/qp-session/`, `crates/qp-schema/`, `crates/qp-wasm/`, `shells/web/`, `docs/PROTOCOL.md` | T1, T2 |
| M12a–d | Boyama yolları: her biri ayrı dalda, ayrı alt ajanla, birbirini görmeden | dal başına; `crates/qp-render*/`, `crates/qp-wasm/`, `shells/web/` | T1, T2 |
| M13 | Açılış: sıkıştırma, bölme, derleme | `crates/qp-cli/` (gösterim sunucusu), `crates/qp-wasm/`, `shells/web/` | T1 |
| M14 | Güncelleme sıklığı ve görünürlük | `crates/qp-wasm/`, `shells/web/` | T2 |
| T4 | Bellek ölçümü ve sınırlama | `rounds/RP3/experiments/`, ilgili crate | T1 |
| S | Kazananların birleştirilmesi; birlikte ölçüm (kazançlar toplanmayabilir); toleranslı ekran testleri | bütünleştirme dalı | M12–M14, T4 |
| A | Denetim: bağımsız alt ajan; kalibrasyonu ve ölçümleri yeniden üretir; anlamsal okumanın ve mimari testinin değişmediğini doğrular; otomatik karşılaştırmayı başsız tarayıcıda çalıştırır | `rounds/RP3/AUDIT.md` | hepsi |

## Notlar

- **Sıra:** kalibrasyon → iş bölümü (M15) → boyama (M12). Tam yansı üzerinde boyamayı
  hızlandırmak yanlış sorunu çözer.
- **Güven ayrımı (K48):** akıştaki kapsül otoritenin kesinleşmiş iddiasıdır; R3'ün mühür
  yolundaki "sunucu tarafından doğrulandı" işaretinin genellenmesidir. Açılınca cihaz kendi
  verisinde sınar. RP2'nin "yansı sınamadan göstermez" kuralı yalnızca açık enstrüman
  için geçerli kalır; aday yine yalnız açık enstrümanda, yerel veriden.
- **Önce kalibrasyon.** T1 bitmeden hiçbir iyileştirme ölçülmez. Kalibre edilmemiş
  benzetimde kazanan, telefonda kaybedebilir (R4'ün dersi).
- **Mimari sınır:** boyama yolu sahne tarifini okur; alan mantığı içermez. Tarayıcının
  kendi tuvaliyle boyayan bir yol kabukta yaşayabilir, ama yalnızca sahne tarifindeki
  ilkelleri (çizgi, dikdörtgen, yol, metin, işaret) boyar; yerleşim, metin kurma, sıralama
  yapmaz. Mimari testini buna göre genişlet, gevşetme.
- **Metin:** yazı tipi farkı görsel farkın ana kaynağı olabilir. Aynı yazı tipi dosyasının
  her yolda kullanılması ve farkın ölçülmesi beklenir.
- **Bilinen yöntemlerle sınırlı değilsin (K18).** Karma çözümler (ör. durağan içerik bir
  yolla, hareketli içerik başka yolla; yüzlerin doku olarak önbelleği) serbest.
- **GPU yolu** K12'deki ölçümlere dayanır ama sürümler değişmiş olabilir; güncel
  belgelerden doğrula. iOS'ta WebGPU desteği tarayıcıya ve sürüme bağlıdır; yedeği şart.
- **Gerçek cihaz yok:** ajan telefonda ölçemez. Bu yüzden ölçüt 8'in sınaması Arkin'in
  tek dokunuşla çalıştırabileceği kadar basit olmalı; seçim onun ölçümüne kadar öneridir.
- **Paket boyutu:** her yolun WASM ve JS boyutuna etkisi ölçülür; GPU yolu paketi
  büyütüyorsa ayrı yükleme değerlendirilir.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
