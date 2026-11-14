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
| K14 | 2026-11-14 | Bar hizası enstrümanın işlem gördüğü borsanın takvimine ve saat dilimine göre yapılır (BIST → İstanbul, NYSE/Nasdaq → New York). Gösterim saat dilimi ayrıdır ve kullanıcıya göre seçilir. | Günlük bar borsanın seansıdır; kullanıcının bulunduğu ülkeye göre hizalamak aynı enstrümanda farklı barlar üretir.  Arkin onayladı (2026-11-14). |
| K15 | 2026-11-14 | Depo geçici olarak Arkin'in açacağı özel bir GitHub organizasyonunda tutulur; sonra Matriks'e taşınır. Linux doğrulaması GitHub Actions üzerinde yapılır; ayrı sunucu şimdilik kiralanmaz. | CI'ın barındırılan Linux x86_64 koşucuları determinizm ölçümü için yeterli. |
| K16 | 2026-11-14 | **Saat dilimi verisi (R1/T6).** Takvim hesabı (`qp-calendar`) için saat dilimleri, kullanılan bölgelerin UTC kayma geçiş tabloları olarak koda gömülür (`crates/qp-calendar/src/tzdata.rs`; şimdilik `America/New_York`, `Europe/Istanbul`, `UTC`; 1900–2100). Tablolar jiff'in paketli tzdb'sinden (jiff-tzdb 0.1.9, `Cargo.lock` ile sabit) bir üretici örnekle üretilir. jiff yalnızca geliştirme bağımlılığıdır; testler tabloyu jiff'e karşı her geçişte, 6 saatte bir ve yerel→UTC (boşluk/katlanma) için doğrular. Çalışma anında yalnızca tamsayı aritmetiği var; sonuç her hedefte aynı. Yeni bölge, üretici listesine eklenip yeniden üretilir. Gösterim (`display_tz`) için gömülü olmayan bölgeleri kabuklar biçimlendirir; bu hesaba girmez. | WASM ölçümü (wasm32, opt-level s, fat LTO; boş modüle göre fark, gzip -9): gömülü tablo **+10,4 KB** (ham +25 KB); jiff `static` (2 bölge) +71 KB (ham +241 KB, çoğu kütüphane kodu); chrono-tz (tam tzdb) +82 KB (ham +528 KB); jiff + tam paketli tzdb +153 KB (ham +504 KB); elle yazılmış kural +0,3 KB ama geçmiş veri yok (Türkiye 2016'ya kadar yaz saati uyguluyordu, 2015'te geçiş seçim nedeniyle ertelendi; testte var). Düzenek: `tools/tz-size-probe/measure.sh`. |
| K17 | 2026-11-14 | Hız hedefleri cihaz sınıfına bağlıdır. **Dizüstü sınıfı:** 10.000 bar × 1.000 varyasyon, tek çekirdek, yerel < 50 ms, WASM < 150 ms. **Telefon sınıfı:** ayrı hedef, R4'te gerçek cihazda ölçülerek konur. CI koşucusundaki ölçüm bilgi amaçlıdır, gerileme izlemek için kullanılır. | R1'de hedef hangi donanıma göre olduğu tanımlanmadan konmuştu. |
| K18 | 2026-11-14 | Çekirdekte bilinen yöntemlerle sınırlı kalınmaz; gerektiğinde araştırma round'ları (RP*) açılır ve yeni mühendislik yaklaşımları denenir. Değişmez: sonuç deterministik ve tek tek çalıştırmayla bit-eşit kalır. | Arkin: performans için yeni yöntemler geliştirilebilir. |
| K19 | 2026-11-14 | RP1 sonuçları çekirdeğe alındı: (1) tarama çekirdekleri yapı-dizisi şeritlerle, şerit sayısı hedef mimariye göre; (2) ızgarada ortak alt hesap paylaşımı; (3) zincirli hesaplar önek paylaşan hesap grafiğiyle; (4) `Study::extend` ile geçmişe bağlanma; (5) çok iş parçacığı yalnızca ek API, K17 hedefleri tek çekirdek kalır. Deneysel kalanlar: kanıtlı geç başlangıç (`late`) ve içerik adresli önbellek. Alınmayanlar: FMA, karşılıklı çarpma, önek toplamı, f32, GPU hesap (bit-eşitliği bozuyor). | `rounds/RP1/RESEARCH.md`, 21 yaklaşım. K17 dizüstü hedefleri üç farklı CI işlemcisinde tutuyor; hesap özeti değişmedi (`7b1e99d40889f83c`). |
| K20 | 2026-11-14 | K13'ün kapsamı netleşti: K13 bir **hizalama** kuralıdır. Dönen her değer bar sırasıyla hizalıdır ve ısınma boştur; ama her çağrının tüm seriyi döndürmesi gerekmez. Aralık, tek çıktı ya da indirgeme isteyen sorgular K13'e uygundur. | RP1 açık soru 3; bir sonraki büyük kazanç çıktı yazımında. Sorgu biçimi protokol tasarımında (RD1) belirlenir. |
| K21 | 2026-11-14 | Motor, arayüz ve ajanlar arasında bir **alan protokolü** tasarlanır; gramer bu protokolün kelime dağarcığıdır. Çalışma kararları: (1) dil modeli çizim döngüsünde yer almaz; ajan anlam düzeyinde konuşur, motordaki deterministik sunucu gramere çevirir; (2) belge başına tek otorite ve sıralı işlem günlüğü; (3) motor hem sunucuda hem cihazda çalışır, doküman devredilebilir; (4) arayüz ve ajan tavizsiz aynı protokolü kullanır; (5) ajanlar varsayılan olarak öneri yazar, yetkiyle doğrudan yazar; (6) protokol zaman çizelgesi ve dikkat yönlendirmesi taşır. (1), (2), (3) ve kodlama biçimi RD1'de alternatifleriyle ölçülerek sınanır; kanıt aksini gösterirse karar değişir. (4) ve (5) kesindir. | Arkin: konu yalnızca JSON değil, arayüz ile motor arasında bir iletişim protokolü. Çelişkili konularda 2026–2030 bakışıyla yenilikçi araştırma ve alternatif ölçüm istenir. |

## Açık kararlar

| # | Konu | Ne zaman | Not |
|---|------|----------|-----|
| A1 | Çizim yığını — **KAPANDI** | R0 | K12'ye taşındı; kanıt `crates/qp-render/SPIKE.md`. |
| A2 | Qapsule adı için marka ön taraması (TÜRKPATENT, EUIPO, USPTO) ve `qapsule.com` alımı | R1 öncesi | Alan adı 2026-11-14 itibarıyla Afternic'te satılık, hemen al fiyatı 4.995 USD. Önce tarama, sonra alım. |
| A3 | Ajanın yazacağı hesap/görünüm dili (kendi küçük dilimiz mi, sandbox'ta WASM mı) | R5 öncesi | Pine uyumluluğu hedef değil. |
| A4 | Yatırım tavsiyesi sınırı ve denetim izi gereksinimleri | R3 öncesi | Hukuk ekibiyle netleştirilecek. |
| A5 | Ajan çalışma ortamı mevcut QODI/MCP altyapısı üzerine mi kurulacak | R5 öncesi | Qapsule kapsamı dışında ama arayüzü etkiler. |
| A6 | Mobil kabuk yaklaşımı (yerel bağlama mı, gömülü web görünümü mü) | R4 sonrası | |
| A7 | x86'da geniş SIMD için `unsafe` yasağına istisna | Gerçek yük görülünce (R3 sonrası) | Öneri: şimdilik hayır. Hedefler istisnasız tutuyor; istisna denetim yükü getirir. |
| A8 | Tamsayı fiyat adımı (kuruş) toplamı: SMA/Bollinger'de daha doğru ve 1,4–2,4× hızlı, ama R1 tanımını değiştirir | RD1 / R3 öncesi | Enstrümana fiyat adımı alanı gerekir. Düzeltilmiş (bölünme/temettü) fiyatlar adımın katı değildir; bu yüzden seçmeli bir yol olmalı. |
| A9 | Önbellek bellek politikası (bütçe, tahliye, canlı seride geçersizleşme) | R3 | Öneri: deneysel kalsın; çalışma ortamı kararı. |

## Önerilen (RD1) — ana akış onayı bekliyor

RD1 bu satırları **önerir**; K21'deki "sınanacak" kararlar round içinde değiştirilmedi.
Kanıt `rounds/RD1/REPORT.md` ve `rounds/RD1/experiments/`.

| # | Önerilen karar | Kanıt |
|---|----------------|-------|
| Ö1 | **K21-1 korunur:** dil modeli çizim döngüsünde yer almaz; ajan anlam düzeyinde yazar (`show_finding`), deterministik sunucu görünümü kurar. Ajanın grameri doğrudan yazması (`put_view`) yeni görünüm türleri için açık kalır; bulgu başına bir kez yazılır ve belgede saklanır. | T6e: sunucu 125 ns / 6,6 µs, tek çıktı; 10 model örneği 10/10 farklı, ~9 s, olguya dayanmayan öğeler; ölçekte Sonnet ~10–25 bin $/gün. |
| Ö2 | **K21-2 korunur:** belge başına tek otorite ve sıralı günlük. İnceltmeler: katmanlara kimlik ve katman başına çakışma denetimi; alan düzeyinde `base`; gönderenin kendi girdileri çakışma sayılmaz; yeniden bağlanma değişim akışı ya da anlık görüntüyle. | T6a: 30 koşuda otorite 0 anlamsal bozulma; Automerge/Loro 1.000 işlemlik çevrimdışı pencerede 33–50 sessiz bozulma; Loro geri alması başka yazarın değerini 10k işlemde 7–8 kez ezdi. CRDT'ler yeniden bağlanma boyutu ve geçmiş boyutunda üstün. |
| Ö3 | **K21-3 inceltilir:** motor sunucuda ve cihazda aynı kodla çalışır (bit-eşit); **varsayılan** otorite sunucuda, cihaz hesapları kendisi yapan bir yansıdır. Tam devir yalnız başka yazar ve canlı ajan yokken, süreli kira ile. | T6d: yerel = WASM özetleri; B+ 15 KB/s, 0,24 µs/tik sunucu işi; epoch kuralı bölünmeyi birleştirmiyor. |
| Ö4 | **Kodlama: JSON kanonik**, `float_roundtrip` zorunlu; ikili biçim eklenmez. Sıcak yolda akış sıkıştırması. | T6b: deflate ile tik 33–35 B tüm biçimlerde; ikinci serde biçimi WASM'ı ~2× büyütüyor; `float_roundtrip` kapalıyken MACD değerlerinin %11'i bozuluyor. |
| Ö5 | **Değişim biçimi anlamsal:** istek `Op`, günlük ve tel `ChangeOp`; JSON Patch yalnız dışa aktarım/hata ayıklama. | T6c: diff hesabı ~2.400× pahalı; eşzamanlılıkta 0/18 yanlış (`base` ile) vs 10/18. |
| Ö6 | Protokol mesaj tipleri `qp-schema::protocol`'da (tek şema kaynağı); referans otorite `qp-session`, MCP uyarlayıcısı `qp-mcp` (araçlar şemadan üretilir). | RD1 uygulaması; `verify.sh` 13/13. |
| Ö7 | Katmanlar R2'de kararlı kimlik taşır; katman işlemleri kimlikle. | T6c: `base`'siz yanlış sonuçların hepsi indeksli `remove_layer`'dan. |
| Ö8 | Anlamsal referans (kimlik yerine anlamla başvuru) ve `welcome`'da doküman özeti protokolün bir sonraki tasarım adımıdır. | Ajan yazabilirlik v2r: ilk deneme 6/20 (kimlik keşfi), düzeltme 13/14. |
| Ö9 | Bar verisi için JSON içinde sütunlu biçim değerlendirilir (doküman şemasını ve `qp-render`'ı etkiler; ayrı karar). | T6b: gzip'te ikili biçimlerden küçük, çözme 2,6× hızlı. |

| # | Açık karar | Ne zaman | Not |
|---|------------|----------|-----|
| A10 | Devir protokolünün tamamı: kira, fencing, `(epoch, seq)` sürekliliği, parçalı anlık görüntü | R4 öncesi (cihaz) | T6d "Öneriler". |
| A11 | Öğe başına `base` (çevrimdışı biriken komutların kendi kendisiyle çakışmaması) | `qp/1` | T6d: tek `base` ile 20 komuttan 1'i, öğe başına 19'u uygulandı. |
| A12 | MCP olay kanalı: `notifications/message` (2026-07-28'de kullanımdan kalktı) yerine `subscriptions/listen` + `log` | MCP barındırıcısıyla ilk gerçek kullanımda | `crates/qp-mcp/NOTES.md`. |
