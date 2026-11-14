# RD1 Araştırma: 2026–2030 Varsayımları ve Sıcak Yol Kodlamaları

Tarih: 2026-11-14. Kaynaklar bu tarihte (WebFetch/WebSearch ile) okunmuştur. Okunamayan veya yalnızca ikincil kaynaktan gelen bilgi açıkça "doğrulanmadı" diye işaretlidir. Bellekten yazılan hiçbir sayı yoktur; belleğe dayanan çıkarımlar "çıkarım" diye etiketlidir.

Genel uyarı: Arama sonuçlarının bir kısmı blog/derleme sitesidir (bazıları görünür biçimde LLM üretimi). Güven sırası: birincil kaynak (Google/Apple/MCP/Linux Foundation/RustSec/crates.io/GitHub) > haber > blog. Her satırda bu ayrım korunmuştur.

---

## BÖLÜM A — 2026–2030 varsayımları

Format: kanıt (kaynak, tarih) → değerlendirme → varsayım tutmazsa protokolde ne değişir.

### A1. Modeller cihaza iniyor (telefon/saat üzerinde küçük LM)

Kanıt:
- Apple, WWDC26'da (Haziran 2026) Foundation Models çerçevesini genişletti. İkincil kaynaklara göre çerçeve artık cihaz içi modele, Private Cloud Compute'a veya başka bir sunucu modeline yönlenebiliyor; görüntü girdisi var; en güçlü cihaz içi model 12 GB bellek istiyor; yeni "Core AI" çerçevesi özel modeller için. Apple'ın kendi oturum sayfası yalnızca kısa bir tanım veriyor, ayrıntılar ikincil (bkz. mer.vin, appbot.co, callstack.com, macrumors.com 2026-06-09). Model sayısı ve Gemini rolü kaynaklar arasında çelişiyor (doğrulanmadı).
- Google: Gemini Nano, AICore üzerinden Prompt API ile erişiliyor. ML Kit GenAI sayfası Pixel 9 ve Pixel 10 ailesini listeliyor; Gemini Nano 4, I/O 2026'da önizlendi, "bu yıl amiral gemisi cihazlara" denmiş (9to5google.com 2026-04-02, developer.android.com I/O'26 yazısı, developers.google.com/ml-kit/genai). Wear OS üzerinde Gemini Nano kanıtı bulunamadı.
- Cihaz kapsamı dar: üst düzey telefonlar. Saat/gözlükte cihaz içi LM kanıtı yok; gözlükler telefona/buluta bağlı.

Değerlendirme: Telefon sınıfında olası ve hızlanıyor; saat/gözlük sınıfında 2030'a kadar belirsiz (çıkarım: pil ve bellek sınırı). Cihaz içi model kalitesi "niyet ayrıştırma + kısa özet" için yeterli, "uzun planlı çok adımlı analiz" için değil.

Tutmazsa değişen:
- Tutmazsa (cihaz içi LM yaygınlaşmazsa): "cihaz içi ajan" profili kapasite müzakeresinden silinir; protokolün yalnızca iki ajan konumu kalır (sunucu ajan, harici ajan). Çevrimdışı doğal dil komutu desteği düşer; cihazlar yalnızca deterministik komut/sorgu konuşur.
- Tersine, saat/gözlükte de olursa: cihaz içi küçük ajanın tüketeceği "dar komut alt kümesi" (tool şeması boyutu tavanı, ör. 8 komut) bir kapasite alanı olmak zorunda; büyük şema küçük modele sığmaz.

### A2. Ajanlar arayüz üretiyor (generative UI)

Kanıt:
- A2UI 1.0 kararlı sürüm, 2026-11-11 (developers.googleblog.com). Bildirimsel bileşen açıklaması, istemci kendi yerel widget'ıyla çiziyor, ajan istemcide kod çalıştırmıyor. React/Angular/Lit/Flutter/SwiftUI render'ları; Compose 0.9.1. Atlassian, Alation, Bloomberg, ByteDance "kullanıyor/geliştiriyor" (Google'ın kendi beyanı; bağımsız doğrulama yok). v0.9 Nisan 2026'da şemayı sistem istemine taşıdı (copilotkit.ai).
- MCP Apps: 2025-11-21 SEP-1865 önerisi, 2026-01-26 resmi ilk MCP uzantısı; tool `_meta.ui.resourceUri` ile `ui://` kaynağı gösterir, host sandbox iframe'de çizer; Claude, ChatGPT, Goose, VS Code Insiders (blog.modelcontextprotocol.io 2026-01-26). 2026-07-28 spec sürümü uzantı çerçevesine kattı (ikincil, doğrulanmadı).
- Vercel AI SDK: RSC tabanlı `streamUI` "deneysel, üretim için önerilmez" (ai-sdk.dev belgeleri); 2026 durumu doğrulanamadı.
- Claude artifacts: bu oturumun kendi aracı kanıtı (HTML sayfa yayınlama).

Değerlendirme: Eğilim güçlü ve iki rakip model var: (a) bildirimsel (A2UI: istemci çizer), (b) iframe/HTML (MCP Apps: host çizer). Qapsule'ın durumu farklı: grafik çizimi çekirdekte (Rust). Yani bizim "UI"ımız ya bir A2UI/MCP Apps içinde gömülü canvas'tır ya da kendi kabuğudur. Her ikisinde de protokolün işi: çekirdeğin ürettiği grafik durumunu ve eylemleri dışarı vermek, ajanın ürettiği arayüzü içeri almak değil.

Tutmazsa değişen:
- Generative UI standartlaşmazsa/parçalanırsa: protokolün "sunum önerisi" (proposal) katmanı kendi bileşen sözlüğünü taşımak zorunda kalır (A2UI'a yaslanamayız). Yani A2UI/MCP Apps köprü katmanı ("sunucu tarafında ince adaptör") yazılmaz, yerine kendi ince bileşen şeması protokole girer.
- Tutarsa: protokol yalnızca grafik canvas'ını ve bağlama verisini sunar, kabuk ajanın ürettiği çerçeveyi (kart, form) A2UI/MCP Apps'e bırakır; protokolün UI şeması küçülür.

### A3. Ekranlar çeşitleniyor (saat, gözlük, ses, araba, ortam)

Kanıt:
- IDC (2026-09-16): 2026 Q2'de XR başlık+gözlük sevkiyatı yıllık %35,3 arttı; gözlükler ~%85. Ekransız "audio" gözlükler %70,3, ekranlı gözlükler %14,3. Tahmin: 2026 %26,3, 2027 %40,5, 2028 %26,4 büyüme (idc.com, mixed-news.com). Counterpoint: H1 2026 AI gözlük %263 artış ama %96'sı ekransız (mixed-news.com). Meta payı kaynaklara göre %69–94, kapsam farklı. Samsung/Google'ın ilk gözlükleri ekransız, Gemini entegrasyonlu.
- Önemli sonuç: büyüyen kategori ekransız (ses+kamera). Ekranlı gözlük küçük.

Değerlendirme: "Ekransız/sesli" büyüme kanıtlı; "çok ekranlı" gözlük ekranı belirsiz. Bu, "ekranı olmayan ajan ve ses" yüzeylerinin hesaba katılmasını destekler (NORTH_STAR ile uyumlu), ekranlı gözlük için ise erken.

Tutmazsa değişen:
- Ekranlar çeşitlenmez (telefon+masaüstü hâkim kalır): kapasite müzakeresi iki profile çöker ("tam ekran" ve "ekransız ajan"); saat/gözlük profilleri ve "dikkat bütçesi" alanları kaldırılabilir.
- Ekransız ses baskın olursa: protokolün birinci sınıf çıktısı "konuşulabilir özet" (metin+yapı) olur; çizim tabanlı sorgu sonuçlarının her birinin ekransız eşleniği zorunlu olur.

### A4. Çok ajanlı çalışma (bir belgede birden çok ajan, A2A)

Kanıt:
- A2A v1.0 2026-03-12; Linux Foundation barındırıyor, 150+ kurum (linuxfoundation.org 2026-04-09). A2A'nın Agentic AI Foundation'a katıldığı iddiası tek kaynaklı (doğrulanmadı).
- Başarısızlık verisi: Berkeley/CMU 1.642 iz analizi, çok ajanlı çerçevelerde %41–87 hata, ~%37 koordinasyon hatası (ikincil yazılar; makale arXiv 2503.13657 açılmadı). Paylaşılan kaynakta 5 ajanlı çekişme kıyaslaması: %25–90 kilitlenme (blog, doğrulanmadı). Belge-spesifik eşzamanlı düzenleme kıyaslaması bulunamadı.

Değerlendirme: Çok ajanlı kullanım artıyor ama güvenilirliği kanıtlı biçimde düşük; koordinasyon hataları doğal dil mesajlarının şemasızlığına bağlanıyor. Bu, bizim şemalı değişiklik (change) + sürüm/çakışma modelimizi destekleyen bir kanıt (çıkarım).

Tutmazsa değişen:
- Çok ajan çalışması yaygınlaşmazsa (tek ajan + insan): çakışma çözümü, kilit/lease ve "ajan kimliği başına dikkat" alanları basitleşir; protokol tek yazar + önerici modeline iner (proposal tek yönlü).
- Yaygınlaşırsa: ajan başına kimlik, kapsam (scope) ve sıralı sürüm vektörü zorunlu; protokol A2A ile ilişki kurmalı (A2A görev nesnesini ya da kartı protokolün "attention/proposal" yüzeyine eşleyen bir bağlama).

### A5. LLM gecikme/maliyet eğilimleri ("LLM görsel dilbilgisini doğrudan mı yazsın, deterministik sunucu mu?")

Kanıt:
- Fiyat: Epoch AI sabit performans düzeyi için fiyat düşüşü medyan ~50x/yıl, Ocak 2024'ten beri ~200x/yıl (2025 analizi, ikincil sitelerden). Epoch Eylül 2026 "~%47/çeyrek" (ikincil, doğrulanmadı). arXiv 2603.28576 (Mart 2026): ~600 kat düşüş. OpenAI GPT-5.6 Luna katmanı Temmuz 2026'da %80 indirimle $0,20/Mtok girdi, $1,20/Mtok çıktı (ikincil, doğrulanmadı).
- Hız: Net bir 2026 gecikme eğilimi kurulamadı. Ölçümler çelişkili (Gemini 3.1 Flash-Lite ~365 tok/s; Claude Haiku 5.5 ~238; bir sitede Opus 5.5 249, diğerinde ~111, bir diğerinde ~58; standardcompute.com 2026-10-13, boredom-at-work.com, hermes-ai.net, llmbase.ai). Artificial Analysis başlık metriğini "görev başına süre"ye (prefill, gizli muhakeme, kuyruk dahil) kaydırmış (hermes-ai.net, ikincil; artificialanalysis.ai makalesi 2026-06-24).
- Sonuç: Birim token fiyatı çok hızlı düşüyor; ama muhakeme modellerinde görev başına süre/token sayısı artıyor, yani "çıktı token'ı ucuzlar, ama etkileşim gecikmesi aynı oranda düşmez".

Değerlendirme (çıkarım, kanıtla desteklenmiş ama karar bizim): Maliyet "LLM görsel dilbilgisini yazsın" lehine ilerliyor; gecikme ise etkileşim döngüsü (sürükle, yakınlaştır, 16 ms çerçeve) için LLM'i uygunsuz bırakıyor. Maliyet düşse bile LLM çağrısı saniye mertebesinde, deterministik sunucu (presenter) mikro/milisaniye. Dolayısıyla doğru ayrım ucuzluk değil, gecikme sınıfı ve tekrarlanabilirlik (CLAUDE.md: hesap kodunda LLM yok). LLM "niyet+öneri" katmanında kalır; çizim durumu deterministik kalır.

Tutmazsa değişen:
- Gecikme 100 ms altına inmez (büyük olasılık): bugünkü ayrım korunur, protokolde "presenter" (deterministik sunum) komutları şart.
- Gecikme+maliyet çok düşer ve LLM görsel dilbilgisini güvenilir yazarsa: protokolde "presenter" soyutlaması küçülür ve LLM çıktısı doğrudan görsel dilbilgisi (komut) olarak kabul edilir; yine de doğrulama/şema kontrolü ve geri alma korunur. Tersine, fiyatlar yükselirse (arz kısıtı): cihaz içi ve önbelleklenebilir deterministik yol zorunlu hale gelir, protokolün "önbelleğe alınabilir sunum çıktısı" alanı büyür.

### A6. Finansta yapay zekâ için düzenleme/denetim

Kanıt:
- AB AI Act: yüksek riskli sistem yükümlülüklerinin (kredi puanlama dahil) ertelenmesi — Digital Omnibus. Komisyon önerisi Kasım 2025: Annex III için Ağustos 2026 → Aralık 2027; Annex I için Ağustos 2028. Yasa koyucuların anlaştığı bildirildi (hlc.com, winstontaylor.com, dlapiper.com, bakerbotts.com). Nihai kabul/Resmi Gazete tarihi doğrulanamadı. Madde 50 şeffaflık yükümlülükleri Ağustos 2026'dan geçerli (ikincil, thesciencetalk.com).
- Önemli: Bir yatırım grafiği aracı kendi başına Annex III "yüksek risk" değil (çıkarım: kredi/sigorta sınıfları listeli). Ama müşterisi (aracı kurum) için MiFID kapsamında dolaylı gereklilik doğar.
- ESMA kamu açıklaması 2024-05-30 (esma.europa.eu): yönetim organı yapay zekâ kullanımından sorumlu; açıklanabilirlik eksikliği hizmet kalitesi riski; kayıt tutma; üçüncü taraf yapay zekâ araçları için dış kaynak (outsourcing) kuralları. 2026'ya ait yeni ESMA metni bulunamadı.
- Türkiye: SPK'nın 2026-02-28'de yürürlüğe giren uzaktan kimlik tespiti değişikliği yapay zekâ kullanımına izin veriyor (MASAK esaslı); açıklanabilirlik hükmü yok (gzt.com haberi). SPK'ya özgü bir yapay zekâ açıklanabilirlik taslağı doğrulanamadı.

Değerlendirme: 2026–2030 için kesin olan: kayıt ve izlenebilirlik beklentisi (ESMA) ve şeffaflık; kesin olmayan: yüksek risk yükümlülüğünün tarihi ve kapsamı. Denetim izi (audit trail) için güvenli varsayım: "her değişiklik kimden, hangi ajan/model sürümüyle, hangi gerekçeyle, hangi girdilerle".

Tutmazsa değişen:
- Düzenleme gevşer/uzar: değişikliklerdeki "provenance/gerekçe" alanları isteğe bağlı kalabilir; yine de ucuz olduğu için zorunlu bırakmak daha güvenli (ek maliyet küçük, sonradan eklemek yıkıcı).
- Düzenleme sıkılaşırsa: protokolde değişmez (append-only) imzalı olay günlüğü, model sürümü ve istem özeti alanları ve "insan onayı" (proposal kabulü) çağrısı zorunlu yüzey haline gelir; replay (aynı girdi, aynı çıktı) denetim aracı olarak protokole girer.

### A7. Local-first / senkron motorlar

Kanıt:
- 2026 genel yazılar CRDT çekirdekleri (Yjs, Automerge 3, Loro) ile senkron motorları (Zero, Electric, PowerSync, Replicache, Jazz, Convex, Triplit) ayırıyor; Linear ve Figma örnek (youngju.dev, letsdatascience.com 2026). Electric'in ajan durumuna yöneldiği anlatılıyor. Bağımsız benimseme metriği bulunamadı; bir yazı benimsemenin ekonomik/kültürel engellerle sınırlı kaldığını savunuyor (alexcloudstar.com). Kaynakların çoğu blog/derleme.

Değerlendirme: Teknik olgunluk var, benimseme belirsiz. Bizim durumumuzda belge "tek yazar çekirdek + sürümlü değişiklik günlüğü"; CRDT zorunlu değil (çıkarım). Protokol kendi senkronunu belirleyecek kadar küçük tutulabilir.

Tutmazsa değişen:
- Local-first yaygınlaşırsa: protokol CRDT/sürüm vektörü uyumlu değişiklik (change) kimlikleri ve birleştirme semantiği tanımlamak zorunda; "sunucu sıralı tek günlük" varsayımı gevşer.
- Yaygınlaşmazsa: sunucu sıralı tek günlük + istemci tahmini (optimistic) yeterli; protokolde birleştirme alanları sadeleşir.

### A8. WebGPU/WASM her yerde

Kanıt:
- web.dev "WebGPU is now supported in major browsers": Chrome/Edge (113, 2023), Safari 26 (macOS/iOS/iPadOS/visionOS 26), Firefox Windows 141 (Temmuz 2025), macOS Apple Silicon sonraki sürümler; Linux/Android Firefox devam ediyor (web.dev; sürüm numaraları kaynaklar arasında çelişiyor: Firefox 145 vs 147). Enterno tahmini: küresel sayfa görüntülemelerinin ~%84'ü tam destekli (enterno.io; caniuse sayfası okunamadı). WebGPU/WGSL ekosistemi 2026 derlemeleri: wgpu, naga, three.js, WebLLM (youngju.dev, ikincil).
- WASM: wasm32 hedefi kararlı; arrow-rs belgeleri wasm32-unknown-unknown ve wasi'ye derleme belirtir, bazı özellikleri kapatmak gerekir (docs.rs/arrow 60.0.0). Mevcut sürümle arrow-ipc'nin temiz derlendiği doğrulanmadı.

Değerlendirme: Büyük olasılıkla doğru; WebGPU %84 + fallback (WebGL/yazılım) gerekir. Çekirdeğin WASM'de çalışması zaten mimari seçim.

Tutmazsa değişen:
- WebGPU yaygınlaşmazsa: istemci tarafı çizim yerine "sunucuda çiz, kare/dilim yayınla" profili korunmalı; protokolde piksel/komut akışı varyantı kalır.
- WASM kısıtlıysa (bellek/iş parçacığı): çekirdeği tarayıcıda koşmak yerine uzaktan bağlan; protokol tamamen uzak kullanıma uygun (gecikme toleranslı) olmalı. Yani tarayıcıda yerel çekirdek varsayımı protokolden çıkarılmalı.

### A9. Ajan-araç standardizasyonu MCP üzerinde birleşiyor

Kanıt:
- Linux Foundation, 2025-12-09'da Agentic AI Foundation'ı kurdu; Anthropic MCP'yi bağışladı, Block ve OpenAI kurucu; Google, Microsoft, AWS, Cloudflare, Bloomberg destekçi (sdtimes.com). ~97 milyon aylık SDK kurulumu (Mart 2026) ve 10.000+ sunucu iddiası çok sayıda blogda, birincil kaynağa bağlantısız (doğrulanmadı). 2026 Ocak–Şubat'ta MCP sunucu/istemci/altyapıya 30+ CVE iddiası (tek kaynak, doğrulanmadı). MCP Apps resmi uzantı (yukarıda). A2A ile yan yana (A4).
- Bir güvenlik ve sürüm riski: MCP'nin 2026-07-28 sürümü uzantı çerçevesi getirdi (ikincil). Yani protokol hâlâ hareket ediyor.

Değerlendirme: MCP fiili standart, ancak kapsamı "araç çağrısı + kaynak + istem" (+UI uzantısı). Abonelik, akış ve yüksek frekanslı veri yolu için tasarlanmış değil (çıkarım: JSON-RPC metin tabanlı). Bizim protokolümüzün çekirdeği (komut, değişiklik, sorgu, abonelik, girdi) MCP'nin üstünde veya yanında olabilir.

Tutmazsa değişen:
- MCP başka bir şeye evrilirse/parçalanırsa: protokol MCP'ye gömülü değil, MCP'ye "adaptör" olarak yazılmış olmalı. Adaptör MCP tool/resource eşlemesi ince bir katman kalırsa değişim ucuz. Protokol çekirdeği MCP kavramlarını (tool şeması gibi) sızdırıyorsa pahalı.
- MCP kalıcı olursa: "komut"lar MCP tool, "sorgu" MCP resource olarak otomatik görünür; sıcak yol (abonelik/akış) MCP dışı ikili kanal olarak kalır. Bu A-bölümü ile B-bölümünü bağlar: JSON-RPC/MCP kontrol düzlemi, ikili sıcak yol veri düzlemi.

### A özeti (en yüksek etkili varsayımlar)

| # | Varsayım | Kanıt gücü | Tutmazsa |
|---|---|---|---|
| 1 | Cihaz içi LM | Telefonda orta, saat/gözlükte zayıf | "cihaz içi ajan" profili silinir, iki ajan konumu kalır |
| 2 | Üretken UI | Güçlü (A2UI 1.0, MCP Apps) ama iki model | Kendi ince bileşen şeması protokole girer |
| 3 | Ekran çeşitliliği | Ekransız güçlü, ekranlı zayıf | Müzakere iki profile çöker |
| 4 | Çok ajan | Standart var (A2A 1.0), güvenilirlik düşük | Tek yazar+önerici modeline iner |
| 5 | LLM maliyet/gecikme | Maliyet çok güçlü, gecikme belirsiz | Presenter küçülür veya şart olur |
| 6 | Düzenleme | Tarih belirsiz, yön net | Provenance alanları isteğe bağlı/zorunlu |
| 7 | Local-first | Zayıf-orta | Birleştirme semantiği sadeleşir |
| 8 | WebGPU/WASM | Güçlü (~%84) | Uzak çizim profili korunur |
| 9 | MCP birleşmesi | Güçlü, ama sıcak yol dışında | MCP adaptör katmanı sökülür |

Doğrulanamayanlar: Apple model sayısı ve Gemini rolü, A2A+AAIF birleşmesi, Epoch %47/çeyrek, GPT-5.6 Luna indirimi, MCP 97M/10K, SPK yapay zekâ taslağı, 2026 ESMA güncellemesi, Digital Omnibus nihai kabul tarihi.

---

## BÖLÜM B — Sıcak yol kodlamaları

### B0. Kıyaslama kaynağı

rust_serialization_benchmark (djkoloski), sayfa "Last updated: 2026-11-05", rustc 1.101.0-nightly, LLVM 23.1.5, x86_64 Linux, Intel Xeon 6975P-C, 4 vCPU KVM altında. https://github.com/djkoloski/rust_serialization_benchmark. Sürümler: serde_json 1.0.154, rmp-serde 1.3.1, ciborium 0.2.2, minicbor 2.3.1, postcard 1.1.3, flatbuffers 25.12.19, capnp 0.27.3, prost 0.14.4. Sayfada bincode yok; bitcode 0.6.9 listelenmiş. Arrow IPC bu kıyasta yok.

Uyarı: Bu veri setleri (log, mesh, minecraft_savedata, mk48) Qapsule yükünü (f64 OHLCV sütunları, küçük komut/değişiklik mesajları, sürekli abonelik akışı) temsil etmez. Sayılar yön göstericidir; karar için kendi ölçümümüz gerekir. Sayfadaki "capnp *" ve "prost *" ikinci değerleri tooltip varyantıdır, anlamı metin olarak doğrulanamadı.

Aşağıda JSON'a göre oranlar (serde_json = 1,0; ser = serileştirme hızı kat, de = çözme hızı kat, boyut = JSON'un oranı; hesaplamalar bu belgeyi yazan tarafından yukarıdaki ham sayılardan yapıldı).

Ham sayılar, "log" veri seti (JSON 1.83 MB):

| Crate | Ser | De | Boyut (B) | Zlib (B) |
|---|---|---|---|---|
| serde_json | 2,329 ms | 4,599 ms | 1.827.461 | 470.560 |
| rmp-serde | 1,058 ms | 2,527 ms | 784.997 | 325.384 |
| ciborium | 1,714 ms | 7,366 ms | 1.407.835 | 403.440 |
| minicbor | 314 µs | 2,368 ms | 817.830 | 332.671 |
| postcard | 256 µs | 1,948 ms | 724.953 | 302.399 |
| flatbuffers | 701 µs | (sıfır kopya) | 1.276.368 | 468.539 |
| capnp | 461 µs | (sıfır kopya) | 1.443.216 | 513.986 |
| prost | 768 µs | 2,727 ms | 884.628 | 363.130 |

Oranlar (log): postcard ser 9,1x, de 2,4x, boyut 0,40; minicbor ser 7,4x, de 1,9x, boyut 0,45; rmp-serde ser 2,2x, de 1,8x, boyut 0,43; prost ser 3,0x, de 1,7x, boyut 0,48; ciborium ser 1,4x, de 0,62x (JSON'dan yavaş), boyut 0,77; flatbuffers ser 3,3x, boyut 0,70; capnp ser 5,1x, boyut 0,79.

"mesh" (sayısal dizi ağırlıklı; JSON 26,2 MB): postcard ser 146x, de 65x, boyut 0,23; minicbor ser 26x, de 8,1x, boyut 0,31; rmp-serde ser 5,1x, de 8,2x, boyut 0,31; flatbuffers ser 97x, boyut 0,23; capnp ser 8,2x, boyut 0,53; prost ser 16x, de 7,2x, boyut 0,33; ciborium ser 1,6x, de 0,89x. (Not: mesh içeriğinin f32 mi f64 mü olduğu sayfada metin olarak doğrulanamadı; çıkarım olarak f32 üçgen köşeleri.)

Zlib sonrası fark çok daralıyor (log: JSON 470 KB, postcard 302 KB, yani 0,64; mesh: JSON 9,6 MB, postcard 5,4 MB). Ağ taşıması sıkıştırmalıysa boyut kazancı yarıya iner; CPU kazancı aynen kalır.

### B1. Crate bazında özet

Aşağıdaki "f64 tam gidiş-dönüş" sütunu bit düzeyinde birebir demektir.

| Kodlama / crate | Sürüm, bakım (crates.io) | WASM | Şema evrimi | f64 tam gidiş-dönüş |
|---|---|---|---|---|
| JSON / serde_json | 1.0.154, 2026-10-11, aktif | Evet (saf Rust) | Alan adı tabanlı; serde `default`/`skip` ile ekleme/çıkarma kolay; tip güvenliği zayıf | Serileştirme ryu ile en kısa gidiş-dönüş basar; AYRICA çözmede varsayılan "en iyi çaba" kesinliği, bir ULP sapma olabilir. `float_roundtrip` özelliği gerekir (yaklaşık 2x yavaş çözme, Cargo.toml yorumu). NaN/Inf JSON'da temsil edilemez. |
| MessagePack / rmp-serde | 1.3.1, 2025-12-23, bakımda (yavaş tempo) | Evet (saf Rust) | serde yapı-harita modunda ada göre, dizi modunda konuma göre; ek alan eklemek harita modunda güvenli | Evet (float64 0xcb 8 bayt; çıkarım, spec bilgisi) |
| CBOR / ciborium | 0.2.2, son sürüm 2024-01-24 (iki yıldan eski), yaygın kullanım (300M indirme) ama yeni sürüm yok | Evet (çıkarım, belge doğrulanamadı) | Self-describing, ada göre | Evet (IEEE 754; çıkarım). Benchmarkta çözme JSON'dan yavaş. |
| CBOR / minicbor | 2.3.1, 2026-10-18, aktif | no_std dostu (çıkarım, belge doğrulanamadı) | `#[n(0)]` indeks tabanlı derive; alan numarasıyla ekleme güvenli (bilgi: README, doğrulanmadı) | Evet (çıkarım) |
| Postcard | 1.1.3, 2025-07-24, kararlı (wire format v1.0 sabit) | Evet, no_std odaklı | YOK: self-describing değil, alanlar tanım sırasında. Belge: uyumluluk "kullanıcıya ait". Alan ekleme ancak sürümlü şemayla | Evet: f64 bit olarak u64, 8 bayt little-endian, varint değil (postcard.jamesmunns.com/wire-format) |
| bincode | 3.0.0 (2025-12-16) bir MEZAR TAŞI sürümü: yalnızca README ve derleme hatası veren lib.rs. RUSTSEC-2025-0141: bakımsız, geliştirme kalıcı olarak durdu (taciz olayı), 1.3.3 "tamam" sayılıyor, yama yok; önerilen alternatifler wincode, postcard, bitcode, rkyv | 1.3.3 ve 2.x çalışır | Postcard gibi şemasız konumsal | Evet (ham 8 bayt; çıkarım) |
| FlatBuffers | 25.12.19, 2025-12-19 | Evet (flatbuffers crate no_std); bu belgede derleme doğrulanmadı | Güçlü: yalnızca sona alan ekle, silme yerine `deprecated`; `flatc --conform` ile otomatik kontrol (flatbuffers.dev/evolution) | Evet (double alan 8 bayt) |
| Cap'n Proto / capnp | 0.27.3, 2026-10-27, aktif | Evet (çıkarım; doğrulanamadı) | Güçlü: yüksek numaralı alan ekle; tip/varsayılan değiştirme güvenli değil (capnproto.org/language.html) | Evet; varsayılan değerlere göre XOR kodlama (bu belgede doğrulanamadı, bilgi), yine de bit düzeyi birebir |
| Arrow IPC / arrow-rs | arrow 60.1.0, 2026-10-28, çok aktif | arrow-rs belgesi wasm32 derlemeyi belirtir, özellik kapatma ve getrandom `js` gerekir; ipc özelliğinin güncel sürümde temiz derlendiği doğrulanmadı | Mesaj başına sürüm numarası; şema+meta veri taşınır; evrim kuralları ayrı sürümleme belgesinde (doğrulanamadı) | Evet (Float64 sütun 8 bayt; geçerlilik bitmap'i ayrı) |
| Protobuf / prost | 0.14.6, 2026-11-12, aktif | Evet (saf Rust) | En olgun: alan numarası, `optional`, `reserved`, bilinmeyen alanı koru | Evet (`double`, 8 bayt little-endian; çıkarım). Protobuf `double` NaN payload'ını bit düzeyinde korumayı garantilemez (çıkarım, doğrulanmadı) |

Arrow IPC özel notları (arrow.apache.org/docs/format/Columnar.html):
- Akış biçimi: şema, ardından RecordBatch'ler; her mesaj 8 bayt önek + FlatBuffers meta + 8 bayta hizalama dolgusu. Küçük mesajlarda sabit maliyet baskın (çıkarım).
- Sıfır kopya: "pointer swizzling" gerektirmeyen yerleşim, paylaşımlı bellekte gerçek sıfır kopya.
- Sütunlu: OHLCV gibi sütunlu f64 verisi için doğal; tek tek komut/olay mesajı için ağır.
- Bu belgede arrow-ipc için kıyas sayıları yok (rust_serialization_benchmark'ta yok).

Sıfır kopya notu: flatbuffers/capnp için çözme "†" (yok) çünkü doğrulamasız erişim ölçülüyor; yüzey sıfır kopya ancak güvenilmeyen veride doğrulama (verifier) maliyeti gerekir. Bu maliyet sayfadaki zero-copy tablosunda ayrı, o tabloyu bu belgede aktarmadım (çıkarım: ağdan gelen veri için doğrulama zorunlu).

### B2. Değerlendirme: bizim için ne önemli

1. f64 birebirlik: Fiyat/hesap değerleri f64 ve "aynı girdi, aynı çıktı". Metin biçimleri (JSON) yalnızca `float_roundtrip` ile ve NaN/Inf hariç güvenli; ikili biçimlerin hepsi ham 8 bayt basıyor. Bu, JSON'un karşılaştırma tabanı kalması için `float_roundtrip` açık olarak ölçülmeli demektir (yoksa JSON'u haksız yere hızlı gösteririz).
2. Şema evrimi ile 2030 ömrü: Postcard ve bincode şemasız; protokol sürümü alan eklemeyi kaldıramaz. Tek kaynak qp-schema Rust tipleri; ek alanlar için ya sürüm numarası + eşleme gerekir ya da evrim dostu biçim (protobuf, flatbuffers, capnp, CBOR indeksli).
3. Bakım riski: bincode kalıcı olarak bırakıldı (RUSTSEC); ciborium'un 2 yıldır yeni sürümü yok (aktif geliştirme rozeti README'de, tek başına kanıt değil); postcard 1.1.3 Temmuz 2025'ten beri; kararlı wire format v1.0 garantili.
4. Yaygın araç desteği: Protobuf ve CBOR dış dünyada (JS, Swift, Kotlin, Python) en olgun; postcard fiilen yalnızca Rust (çıkarım). Kabuklar ince ve çok dilli olduğu için karşı tarafta çözücü yazma maliyeti belirleyici (çıkarım).
5. İki ayrı yük: (a) kontrol düzlemi (komut, değişiklik, öneri; küçük, seyrek, insan/LLM de okuyabilmeli; JSON/JSON-RPC, MCP ile uyumlu) ve (b) veri düzlemi (sorgu sonucu sütunları, abonelik akışı; büyük, sıcak, makine). Tek kodlama her ikisine uymayabilir.

### B3. Öneri: JSON'a karşı ölçülecek 1–2 ikili aday

Aday 1 — CBOR (minicbor, indeks tabanlı derive; karşılaştırma için ciborium da) — "evrim ve ömür adayı":
- Neden: IETF standardı (RFC 8949, bilgi), self-describing (JSON ile aynı veri modeli, kolay hata ayıklama, çıkarım: JSON'a köprü kolay), indeksli alanlarla evrim, kabuk dillerinde olgun çözücüler, kimi biçimlerde deterministik/kanonik kodlama tanımlı (imzalı denetim günlüğü için işe yarar, çıkarım). minicbor 2.3.1 aktif (2026-10-18). Kıyasta JSON'a göre ser 7,4x, de 1,9x, boyut 0,45 (log); mesh'te ser 26x, de 8x, boyut 0,31. ciborium ise çözmede JSON'dan yavaş çıktı (log 0,62x, mesh 0,89x), o yüzden ciborium aday değil, yalnızca referans.
- Risk: minicbor derive'ı serde'den ayrı bir yol (qp-schema'nın serde tipleriyle ikinci derive bakımı; "şemanın tek kaynağı Rust tipleri" kuralıyla uyumlu ama iki derive kümesi); serde köprüsü varsa ona bakılmalı (doğrulanamadı).

Aday 2 — Postcard (veri düzlemi, aynı sürüm sabitli uçlar için) veya Arrow IPC (sütunlu veri düzlemi için) — "hız ve boyut adayı":
- Postcard: kıyasta en hızlı serde-uyumlu biçimlerden: log ser 9,1x, de 2,4x, boyut 0,40; mesh ser 146x, de 65x, boyut 0,23. f64 ham 8 bayt. Ama şemasız: alan ekleme/sürüm yönetimi bize kalır; yalnızca "protokol sürümü eşleşen" uçlar arası (çekirdek <-> aynı sürümlü kabuk, WASM <-> yerel) kullanılabilir. Wire format v1.0 kararlı (belgeli).
- Arrow IPC: yalnızca sütunlu toplu veri (OHLCV serisi, gösterge çıktısı) için; tarayıcıda ve Python/analiz araçlarında doğal; sıfır kopya. Ancak küçük mesajlarda ağır ve wasm derlemesi doğrulanmadı. Kıyas verisi yok, kendi ölçümümüzde ayrı yük (ör. 10.000 mumluk f64 sütun paketi) olarak denenmeli.

Önerilen ölçüm matrisi (öneri; round kapsamını belirler ana akış):
- Yükler: (1) küçük komut/değişiklik (100–500 B JSON eşdeğeri), (2) 10k mumluk OHLCV f64 sütun paketi, (3) abonelikte 1 kHz küçük "tick" güncellemesi.
- Kodlamalar: serde_json (`float_roundtrip` açık ve kapalı), minicbor, postcard; ikinci sırada rmp-serde (kolay: serde ile tak-çalıştır, boyut 0,43x, hız ~2x) ve sütun yükü için arrow-ipc.
- Ölçüt: ser/de süresi (yerel ve wasm32), boyut ve zlib/zstd sonrası boyut, f64 bit düzeyinde gidiş-dönüş testi (NaN payload, -0.0, subnormal), şema evrimi testi (yeni alan eski okuyucu / eski alan yeni okuyucu), wasm paket boyutu.
- Elenenler: bincode (bırakıldı; RUSTSEC-2025-0141), ciborium (hız ve bakım), flatbuffers/capnp (JSON'dan 1,3–1,8x büyük, ikinci şema dili ve ikinci kod üretimi; "şemanın tek kaynağı Rust tipleri" kuralını ihlal eder, çıkarım), prost (aynı gerekçe: .proto ikinci kaynak olur; olgun ama kural gereği değil).

Not: Flatbuffers/capnp'nin sıfır kopya avantajı, "yüzlerce MB okuma" yüklerinde anlamlı; bizim kontrol/sorgu mesajlarımız küçük olduğundan bu kazanç çıkarım olarak önemsiz, ölçülmeden kesin söylenemez.

---

## Açık noktalar ve doğrulanamayanlar

- Sayfadaki "*" işaretli ikinci değerlerin (capnp, prost) anlamı.
- minicbor, ciborium, capnp, flatbuffers için `wasm32-unknown-unknown` derleme sonucu bu araştırmada denenmedi; kendi küçük denememiz gerekir (CLAUDE.md: kütüphane davranışını varsayma).
- arrow-ipc güncel sürümün wasm32 derlemesi.
- `float_roundtrip` davranışı belge yorumundan okundu; sıfırdan test (ULP) yapılmadı.
- Digital Omnibus'ın nihai kabul tarihi, 2026 ESMA/SPK metinleri.
- Gecikme (tok/s) eğilimi için güvenilir bir zaman serisi bulunamadı.

## Kaynak URL'leri (okunan)

Kıyas ve crate:
- https://github.com/djkoloski/rust_serialization_benchmark
- https://crates.io/api/v1/crates/rmp-serde , /ciborium , /minicbor , /postcard , /bincode , /flatbuffers , /capnp , /arrow , /prost , /serde_json
- https://rustsec.org/advisories/RUSTSEC-2025-0141
- https://postcard.jamesmunns.com/wire-format
- https://github.com/serde-rs/json/blob/master/Cargo.toml
- https://arrow.apache.org/docs/format/Columnar.html
- https://flatbuffers.dev/evolution/
- https://capnproto.org/language.html
- https://github.com/enarx/ciborium

Varsayımlar:
- https://developers.googleblog.com/a2ui-10-build-delightful-fast-reliable-agentic-experiences/ (2026-11-11)
- https://blog.modelcontextprotocol.io/posts/2026-01-26-mcp-apps/ (2026-01-26)
- Arama sonuçlarından (okunmadan, yalnızca özet): macrumors.com/2026/06/09/apple-outlines-major-ai-and-developer-tool-updates, callstack.com/blog/on-device-ai-after-wwdc-2026-whats-new, 9to5google.com/2026/04/02/gemini-nano-4-android, developers.google.com/ml-kit/genai, linuxfoundation.org (A2A 150+ kurum, 2026-04-09), sdtimes.com (Agentic AI Foundation), idc.com ve mixed-news.com (gözlük sevkiyatı), web.dev/blog/webgpu-supported-major-browsers, enterno.io, esma.europa.eu (2024-05-30 açıklaması), winstontaylor.com / dlapiper.com / hlc.com (Digital Omnibus), gzt.com (SPK 2026-02-28), artificialanalysis.ai/methodology, arxiv.org/abs/2603.28576, copilotkit.ai (A2UI v0.9), ai-sdk.dev (RSC), docs.rs/crate/arrow/latest.
