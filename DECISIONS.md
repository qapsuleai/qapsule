# Karar Günlüğü

## Verilmiş kararlar

| # | Tarih | Karar | Gerekçe |
|---|-------|-------|---------|
| K1 | 2026-11-14 | Kuzey yıldızı `NORTH_STAR.md` içindeki iki cümledir. | Arkin onayı. |
| K2 | 2026-11-14 | AI-ready araç değil, AI-native sistem. Varsayılan yazar ajan. | TradingView benzeri "ben söyleyeyim AI çizsin" modeli farklılaştırmıyor. |
| K3 | 2026-11-14 | Tüm hesaplar LLM'siz ve deterministik. | Model sayı konusunda güvenilmez; ispat gerekir. |
| K4 | 2026-11-14 | Çekirdekte hızlı, istek anında hesap kabiliyeti vardır. | Sunucu tüm sembol × periyot × indikatör × parametre uzayını önceden hesaplayamaz. |
| K5 | 2026-11-14 | Grafik ve ortak hesap/yayın katmanları ayrıdır; mevcut C++ indicator-engine ve anomaly-engine ana yapı kurulduktan sonra incelenir (R6). | Hedef dağılmasın. |
| K6 | 2026-11-14 | En küçük birim kapsüldür; profesyonel grafik onun tam açılmış halidir. | İki kullanıcı kitlesi tek motorla karşılanır. |
| K7 | 2026-11-14 | Anlam işaretleri için özgün bir görsel alfabe tasarlanır. | Küçültülmüş mum grafiği anlam taşımaz. |
| K8 | 2026-11-14 | Mimari sıfırdan kurulur; eski prototipten parçalar taşınır. | Eski yapıda çizim, etkileşim ve veri TS'te; doküman modeli yok. |
| K9 | 2026-11-14 | Önce kapsül, sonra profesyonel çalışma alanı. | Çekirdeği en az arayüzle sınar; boş alanda başlar. |
| K10 | 2026-11-14 | Geliştirme Claude Code'da round'lar halinde yürür; ana akış ve round tasarımı ayrı oturumda takip edilir. Süreklilik bu repodaki dosyalardadır. | Oturumlar arası hafıza yok. |
| K11 | 2026-11-14 | Proje adı **Qapsule**; paket öneki `qp-`, komut satırı `qp`. Birim adı dokümanda "kapsül", kodda `Capsule`. | Q ailesine uyumlu, uluslararası kullanıma uygun, icadın kendisini adlandırıyor. Marka ön taraması ve alan adı alımı bekliyor; sonuç olumsuzsa ad değişir. |
| K12 | 2026-11-14 | **A1 kapandı — çizim yığını.** (1) Referans/headless çizici: tiny-skia 0.12 + fontdue 0.9 + tamsayı metin birleştirici + gömülü OFL font (IBM Plex Mono); `qp render`, altın görseller ve tarayıcıdaki wasm aynı kod. (2) Etkileşimli GPU yolu: vello 0.12 (wgpu) + WebGPU, **yalnızca MSAA** kipinde, R2'de etkileşimle birlikte eklenir; GPU çıktısı referansa karşı toleransla test edilir, bayt eşitliğiyle değil. (3) Yedek zinciri: WebGPU → CPU wasm; ayrı WebGL2 katmanı yok. (4) vello_cpu alınmadı. | Ölçüm (R0/T3, `crates/qp-render/SPIKE.md`, Apple M4 Max): A yolu aarch64, x86_64 (Rosetta; SSE2 ve AVX2/FMA), wasm32 skaler/simd128'de bit-eşit, tarayıcı canvas ↔ altın 0 px, PNG baytları aynı; 100k bar yerel 38 ms / tarayıcı 80 ms; wasm 363 KB gzip. vello_cpu 100k bar 16,7 ms ama mimariler arası ±1 kayma (stres sahnesi 8 px). vello GPU 100k bar 8–9 ms yerel / 37 ms tarayıcı, ama Area AA çalıştırmalar arası deterministik değil (31'de 3 farklı çıktı), MSAA8/16 deterministik ve Metal ↔ WebGPU 0 px; soğuk boru hattı ~1,5 sn. Açık risk: gerçek x86/Linux donanımında ilk ölçüm CI'da yapılacak. |
| K13 | 2026-11-14 | Hesap çıktısı her zaman bar serisiyle aynı uzunlukta ve aynı hizadadır. Isınma dönemindeki değerler boştur (`null`); kırpma yapılmaz. | Barlar ve indikatörler senkron hareket etmeli (Arkin). |
| K14 | 2026-11-14 | Bar hizası enstrümanın işlem gördüğü borsanın takvimine ve saat dilimine göre yapılır (BIST → İstanbul, NYSE/Nasdaq → New York). Gösterim saat dilimi ayrıdır ve kullanıcıya göre seçilir. | Günlük bar borsanın seansıdır; kullanıcının bulunduğu ülkeye göre hizalamak aynı enstrümanda farklı barlar üretir. Arkin'in onayı bekleniyor (aşağıda not). |
| K15 | 2026-11-14 | Depo geçici olarak Arkin'in açacağı özel bir GitHub organizasyonunda tutulur; sonra Matriks'e taşınır. Linux doğrulaması GitHub Actions üzerinde yapılır; ayrı sunucu şimdilik kiralanmaz. | CI'ın barındırılan Linux x86_64 koşucuları determinizm ölçümü için yeterli. |
| K16 | 2026-11-14 | **Saat dilimi verisi (R1/T6).** Takvim hesabı (`qp-calendar`) için saat dilimleri, kullanılan bölgelerin UTC kayma geçiş tabloları olarak koda gömülür (`crates/qp-calendar/src/tzdata.rs`; şimdilik `America/New_York`, `Europe/Istanbul`, `UTC`; 1900–2100). Tablolar jiff'in paketli tzdb'sinden (jiff-tzdb 0.1.9, `Cargo.lock` ile sabit) bir üretici örnekle üretilir. jiff yalnızca geliştirme bağımlılığıdır; testler tabloyu jiff'e karşı her geçişte, 6 saatte bir ve yerel→UTC (boşluk/katlanma) için doğrular. Çalışma anında yalnızca tamsayı aritmetiği var; sonuç her hedefte aynı. Yeni bölge, üretici listesine eklenip yeniden üretilir. Gösterim (`display_tz`) için gömülü olmayan bölgeleri kabuklar biçimlendirir; bu hesaba girmez. | WASM ölçümü (wasm32, opt-level s, fat LTO; boş modüle göre fark, gzip -9): gömülü tablo **+10,4 KB** (ham +25 KB); jiff `static` (2 bölge) +71 KB (ham +241 KB, çoğu kütüphane kodu); chrono-tz (tam tzdb) +82 KB (ham +528 KB); jiff + tam paketli tzdb +153 KB (ham +504 KB); elle yazılmış kural +0,3 KB ama geçmiş veri yok (Türkiye 2016'ya kadar yaz saati uyguluyordu, 2015'te geçiş seçim nedeniyle ertelendi; testte var). Düzenek: `tools/tz-size-probe/measure.sh`. |

## Açık kararlar

| # | Konu | Ne zaman | Not |
|---|------|----------|-----|
| A1 | Çizim yığını — **KAPANDI** | R0 | K12'ye taşındı; kanıt `crates/qp-render/SPIKE.md`. |
| A2 | Qapsule adı için marka ön taraması (TÜRKPATENT, EUIPO, USPTO) ve `qapsule.com` alımı | R1 öncesi | Alan adı 2026-11-14 itibarıyla Afternic'te satılık, hemen al fiyatı 4.995 USD. Önce tarama, sonra alım. |
| A3 | Ajanın yazacağı hesap/görünüm dili (kendi küçük dilimiz mi, sandbox'ta WASM mı) | R5 öncesi | Pine uyumluluğu hedef değil. |
| A4 | Yatırım tavsiyesi sınırı ve denetim izi gereksinimleri | R3 öncesi | Hukuk ekibiyle netleştirilecek. |
| A5 | Ajan çalışma ortamı mevcut QODI/MCP altyapısı üzerine mi kurulacak | R5 öncesi | Qapsule kapsamı dışında ama arayüzü etkiler. |
| A6 | Mobil kabuk yaklaşımı (yerel bağlama mı, gömülü web görünümü mü) | R4 sonrası | |
