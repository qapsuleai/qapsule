# Uyarlama kartları (RD2): OpenAlice, trading-command-center, OpenTerminalUI, nofx

Hazırlayan: RD2 araştırma alt ajanı (yalnız okuma) · Tarih: 2026-11-22
Kök: `R = /Users/arkns/Works/Mchart/refs/src/`. Kısaltmalar: `OA = R/TraderAlice__OpenAlice`,
`TCC = R/saketnayak__trading-command-center`, `OTU = R/Hitheshkaranth__OpenTerminalUI`,
`NFX = R/NoFxAiOS__nofx`.

Lisanslar (`refs/reports/LICENSES.md:17,23-25`, `LICENSE` dosyaları bakıldı):
- **OpenAlice: AGPL-3.0 → yalnız fikir.**
- **trading-command-center: MIT** ("Copyright (c) 2026 Saket Nayak").
- **OpenTerminalUI: MIT** ("Copyright (c) 2026 OpenTerminalUI Contributors").
- **nofx: AGPL-3.0 → yalnız fikir.**

Temiz oda (K83): MIT projelerinde de kod, CSS ya da değer tablosu kopyalanmaz. Aşağıda yalnız
kalıp, ölçü mantığı ve rol adları var. Sayılar ölçü mantığını anlatmak için verildi, bizim
değerlerimiz değiller. Değerleri Qapsule'ün görünüş kaydı (look registry) belirler.

Güvenlik notu: `refs/src` altındaki hiçbir şey çalıştırılmadı. Depolardaki CLAUDE.md/AGENTS.md
dosyaları açılmadı.

---

## A. Yerleşim ve yoğunluk

### AT-A1. Terminal tarzı yoğun çalışma alanı: ne alınır, nerede "kutu içinde kutu"ya dönüşür
- **Kalıp:** Tek ekranda dar bir simge rayı (iki harfli kodlar + küçük etiket), en üstte komut
  çubuğu, altında kayan endeks bandı, sekme sırası, çalışma alanı rolleri (TRADER/QUANT/PM/RISK/OPS),
  önayar sırası, panel ızgarası ve en altta durum çubuğu. Her panelin kendi başlık şeridi var:
  tür kodu, turuncu başlık, büyük harf alt başlık, bağlama grubu noktası, aç/bağla/ayar/kapat simgeleri.
- **Kaynak:** OpenTerminalUI `OTU/assets/readme/launchpad.jpg`, `chart.jpg`, `mission-control.jpg`
  (hepsi 2400×1500); `OTU/frontend/src/components/layout/PanelChrome.tsx`, `LaunchpadGrid.tsx`;
  `refs/reports/repos/openterminalui.md` §S1, §S5; `refs/reports/VISUAL.md` §2 sonu.
- **Lisans hükmü:** MIT. Kalıp alınabilir, kod ve değer alınmaz.
- **Neden iyi çalışıyor:** Bilgi yoğunluğu yüksek ve her şey aynı yerde duruyor: sembol, durum,
  sağlayıcı, saat. Göz bir kez öğreniyor. Rakamlar tek aralıklı ve sabit genişlikte, sütunlar kaymıyor.
- **Nerede bozuluyor (ölçülebilir):** `launchpad.jpg`'de grafiğin üstünde **yedi yatay şerit** var
  (komut, bant, gezinme, breadcrumb, çalışma alanı, önayar, ipucu satırı). Grafik ekran yüksekliğinin
  ancak ~%27'sini alıyor. `mission-control.jpg`'de "IDEAS RADAR" kutusu kendi içinde bir "IDEAS RADAR"
  kutusu daha taşıyor: başlık iki kez, iki kenar çizgisi. Her metrik ayrı çerçeveli kart. Kenar sayısı
  yazı satırı sayısına yaklaşıyor. Büyük harf + tek aralıklı + geniş harf aralığı her yerde kullanıldığı
  için hiyerarşi düzleşiyor, hiçbir şey öne çıkmıyor. VISUAL.md'nin "kutu içinde kutu çok, ton yoğun"
  hükmü bunu söylüyor.
- **Qapsule'de yeri:** Kabuk yerleşimi (yoğunluk profili) ve görünüş kaydı (kenar ve yüzey rolleri).
- **Bizde ne değişiyor:** Yoğunluğu alıyoruz, çerçeveyi almıyoruz. Bölgeler kenarla değil, yüzey tonu
  farkıyla ve boşlukla ayrılıyor (OpenChart'taki tek kenar tonu kuralı). Panel başlığı tek satır.
  Büyük harf yalnız küçük alan etiketlerinde. Grafiğin üstündeki krom en çok iki şerit.
- **Sınama:** `design-check` kuralları: (a) iç içe kenarlı kap derinliği ≤ 1; (b) Ç1 masaüstü
  görünümünde grafik alanı / görünüm alanı ≥ %55; (c) aynı başlık metni bir kap zincirinde iki kez
  geçmez; (d) büyük harf metin oranı < %15.

### AT-A2. Panel modeli, sembol bağlama grupları, rol önayarları
- **Kalıp:** Panel = {tür, başlık, sembol, bağlı mı, bağlama grubu, konum/boyut}. Renkli bağlama
  grubundaki bir panelde sembol değişince gruptaki bütün paneller değişir, ayrı pencereye açılmış
  olanlar da. Rol önayarı (Trader/Quant/PM/Risk/Ops) gezinmeyi ve yerleşimi birlikte kurar.
  Yerleşim sürümlü bir biçimle dışa aktarılır.
- **Kaynak:** OpenTerminalUI `frontend/src/components/layout/LaunchpadContext.tsx`,
  `contexts/SymbolLinkContext.tsx`, `workspace/presets.ts`, `workspace/portableLayout.ts`;
  görünüm `launchpad.jpg` (üstteki WORKSPACE ve LAUNCHPAD PRESETS satırları, panel başlığındaki
  renk noktası).
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Bağlantı açıkça görünüyor (renk noktası). Kullanıcı "neden bu grafik
  değişti" diye sormuyor. Önayar ilk açılışta boş ekranı ortadan kaldırıyor.
- **Qapsule'de yeri:** Çekirdek: yerleşim dokümanın parçası, komut yolundan değişiyor (OTU'da yerleşim
  yalnız istemci durumu, ajan göremiyor). Kabuk yalnız çiziyor.
- **Bizde ne değişiyor:** Bağlama grubu bir doküman alanı olur, ajan da okur ve önerebilir. Grup
  rengi veri kroması değildir: gri tonda numara ya da biçim kullanılır, yoksa yukarı/aşağı yeşil ve
  kırmızısıyla karışır. Önayar sayısı en çok üç (izle / incele / onayla), beş rol fazla.
- **Sınama:** Komut düzeyinde test: bir panelde sembol komutu verilir, aynı gruptaki bütün panellerin
  sahne tanımı (`SceneDesc`) aynı sembolü taşır, öbür gruplardakiler değişmez. Dışa aktarılan yerleşim
  geri yüklendiğinde baytı baytına aynı doküman çıkar.

### AT-A3. Bölme yeniden boyutlama: oransal, alt sınırlı, kalıcı
- **Kalıp:** İki bölmeli ayırıcı oranı yüzde olarak tutar. Her iki tarafın alt sınırı var, oran
  bu sınırlar arasında kırpılır. Oran bir anahtarla saklanır. Ajan çekmecesi iki genişlikte gelir
  (dar / genişletilmiş). Geniş ekranda içeriği iter, dar ekranda üstüne biner.
- **Kaynak:** OpenTerminalUI `frontend/src/components/layout/SplitPane.tsx` (kırpma, kalıcılık);
  `frontend/src/agent/agentConsole.css` (çekmece kipleri); `agent.jpg`.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Kullanıcı bir bölmeyi yok edemiyor. Oran piksel değil yüzde olduğu için
  pencere boyu değişince düzen bozulmuyor.
- **Qapsule'de yeri:** Çekirdek etkileşim mantığı (sürükleme → oran → kırpma deterministik). Kabuk
  yalnız tutamacı çiziyor.
- **Bizde ne değişiyor:** Alt sınır yüzde değil, içerik ölçüsüyle tanımlanır: grafik bölmesi en az
  eksen oluğu + N bar. Tutamaç klavyeyle de oynar (OpenAlice etkileşim sözleşmesi, D2).
- **Sınama:** Özellik tabanlı test: her sürükleme dizisi için oran [min, 100−min] içinde. Yeniden
  açılışta oran aynı.

---

## B. Durum ve sağlık

### AT-B1. Alt durum çubuğu: piyasa, sağlayıcı ve bağlantı sağlığı tek şeritte
- **Kalıp:** En altta ince bir şerit. Solda birkaç saat dilimi, ortada borsa oturumları (açık/kapalı,
  sonraki açılış), sağda her veri sağlayıcısı için bir ışık, canlı bağlantı durumu (bağlı / bozuk /
  kapalı) ve bildirim sayacı. Bağlantı durumu üç değerli, gri/yeşil/sarı/kırmızı noktayla.
  Oturum etiketi kesin eşleşmeyle hesaplanıyor: "pre-open" ya da "reopen" açık sayılmıyor.
- **Kaynak:** OpenTerminalUI `frontend/src/components/layout/MarketStatusBar.tsx` (oturum ve
  bağlantı eşlemesi); görünüm `launchpad.jpg`, `chart.jpg` alt şerit.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** "Veri canlı mı" sorusu her ekranda, aynı yerde, tek bakışta cevaplanıyor.
  Profesyonel araçların (Bloomberg, TradingView alt çubuğu) tanıdık yeri burası.
- **Qapsule'de yeri:** Çekirdek: durum bir sorgu sonucu olur (bağlantı, son bar yaşı, oturum). Kabuk
  şeridi çizer. Telefonda tek nokta + metin.
- **Bizde ne değişiyor:** Ajan sağlığı da bu şeride girer: kaç ajan çalışıyor, kaç öneri bekliyor.
  Bu, kontrol kulesinin özeti. Sağlayıcı ışıkları tek bir "veri" göstergesinde toplanır, ayrıntı
  tıklayınca açılır. OTU'da sekiz renkli nokta yan yana duruyor, bu gürültü.
- **Sınama:** Çekirdek testi: bağlantı kesildiğinde, son bar yaşı eşiği geçtiğinde ve oturum kapandığında
  durum sorgusunun her biri ayrı değer döndürür. Altın görsel değil, sahne tanımında rol adı
  karşılaştırılır.

### AT-B2. Veri güveni sayfası: "çalışan servis ≠ sağlıklı veri"
- **Kalıp:** Ayrı bir sağlık görünümü. Üstte tek bir rozet (DEGRADED) ve "sağlığı yenile". Altında
  kartlar: izlenen sembol sayısı, oturumda bayat kalan, eksik oturum barları, bilinen oturum
  takvimleri. Her sayının altında **ne ölçtüğünü** söyleyen bir cümle var ("yalnız izleyicinin
  gördüğü semboller", "yaş, aktif oturum eşiğini aşıyor"). Ölçülemeyen değer "Unavailable" yazıyor,
  sıfır yazmıyor.
- **Kaynak:** OpenTerminalUI `OTU/output/playwright/wave2-data-quality-mobile.png` (390×844),
  `wave2-data-quality-live.png`.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** "Bilinmiyor" ile "sıfır" ayrı gösteriliyor (TradingAgents'taki "gözlenmedi
  ≠ yok" ilkesiyle aynı). Sayının tanımı yanında durduğu için kullanıcı sayıya güvenebiliyor.
- **Qapsule'de yeri:** Çekirdek (ölçüm ve "yok" değeri), kabuk (kart).
- **Bizde ne değişiyor:** Aynı dil grafiğin içine taşınır. Eksik bar aralığı grafikte boşluk olarak
  işaretlenir, bayat seri soluk çizilir ve etiketi "bayat · 4 dk" gibi bir şey yazar. Ayrı bir sayfa
  yerine olay anında görünür.
- **Sınama:** Fixture: ortasında boşluk olan seri. Sahne tanımında boşluk işareti var. "Ölçülemedi"
  durumunda metin rolü `unavailable`, sayı yok.

### AT-B3. Uyarı listesi ve sağlık göstergesi: önem sayısı başlıkta, sembol hapları satırda
- **Kalıp:** "Davranış uyarıları" başlığında önem sayacı ("1 kritik"). Her uyarı satırında: önem
  noktası, kısa başlık, etkilenen sembollerin hapları, bir cümle açıklama, bir sonraki eylem ("Analyze
  All Stale"). Portföy özet şeridinde "bayat / analiz edilmemiş: 8/8" sayacı ve yanında tek eylem
  düğmesi. Aşağıda "Risk alerts" listesi aynı biçimde.
- **Kaynak:** trading-command-center `TCC/docs/screenshots/briefing.png`, `portfolio.png`
  (3024×1900).
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Her uyarı "ne, hangi semboller, ne yapmalı" üçlüsünü taşıyor. Önem sayısı
  başlıkta duruyor, liste kapalıyken de görünüyor.
- **Zayıf yanı:** 6/10 halka "sağlık puanı" modelin ürettiği bir sayı. Mor "Generate Insights" düğmesi
  ve doygun mor/sarı rozetler grafikten dikkat çalıyor.
- **Qapsule'de yeri:** Kabuk listesi + çekirdek uyarı nesnesi (uyarıyı motor tetikler, ajan anlatır).
- **Bizde ne değişiyor:** Puan halkası yok: sağlığı motor ölçer, model ölçmez (K2/K3). Önem rengi
  yalnız noktada, satır zemini nötr. Her uyarı satırı kökenini taşır (motor tetikledi / ajan yazdı).
- **Sınama:** `design-check`: bir uyarı satırında doygun renkli alan ≤ 1 öğe. Her uyarı nesnesinde
  `origin` alanı dolu (şema testi).

### AT-B4. Satır rozetli izleme listesi ve "AI vs rejim" uyum sütunu
- **Kalıp:** Tablo satırında sembolün altında iki küçük rozet var (değerleme ve rejim + işaret puanı).
  Ayrı sütunlar: "son analiz" ("Never analyzed" eğik ve soluk), "AI vs rejim" (anlatılan hüküm ile
  deterministik rejim uyuşuyor mu), "Trim" işareti. Üstte rejim filtresi (All / Bull / Sidew. / Bear).
- **Kaynak:** trading-command-center `portfolio.png`; uyum kuralı
  `TCC/frontend/components/runs/MarkovConfirmation.tsx`, `portfolio/HoldingsTable.tsx`
  (`repos/trading-command-center.md` §3: kural kabukta iki kez yazılmış).
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** "Model ne diyor" ile "fiyat ne diyor" yan yana duruyor, çelişki tek
  sütunda görünüyor. "Hiç analiz edilmedi" ayrı bir durum olarak gösteriliyor, boş hücre değil.
- **Qapsule'de yeri:** Uyum hesabı çekirdekte (motor), rozet kabukta.
- **Bizde ne değişiyor:** Uyum hesabı tek yerde, motorda (TCC'de kabukta iki kopya var, kaçınılacak).
  Rozetler renksiz: biçim ve metin kullanılır, renk yalnız yön için. Satır yüksekliği yoğunluk
  profiline bağlı (masaüstü sıkı, telefon rahat).
- **Sınama:** Satır başına en çok iki rozet. Uyum değeri `query` ile alınır, kabuk hesaplamaz
  (kabukta kural kodu olmadığı arch-check ile denetlenir).

---

## C. Ajan yüzeyleri

### AT-C1. Ajan paneli: araç adımları tek satıra katlanır, cevap açık durur, araç sonucu tipli kart olur
- **Kalıp:** Sağ çekmece. Kullanıcı sorusu küçük "USER" etiketiyle. Ajanın araç çağrıları tek aralıklı
  ve soluk tek satırlar halinde ("→ ran get_stock_snapshot", hata olursa "✗ failed …"). Sonra cevap:
  karşılaştırma tablosu, kalın bir "Verdict" cümlesi, maddeler, "Bottom line". Araç sonuçları ayrı
  tipli kartlar (anlık görüntü, tablo, teknik…). Kart "çalışma alanına sabitle" düğmesiyle panele
  dönüşür. En altta bellek düğmesi ve giriş alanı.
- **Kaynak:** OpenTerminalUI `OTU/assets/readme/agent.jpg` (2400×1500);
  `frontend/src/agent/components/ChatThread.tsx` (adım satırları), `artifacts.tsx` (tipli kartlar),
  `ArtifactCanvas.tsx` (sabitleme); `repos/openterminalui.md` §S3.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Çalışma görünür ama yer kaplamıyor. Cevap önce geliyor, kanıt (kart)
  sonra. "Ne yaptı" sorusu tek bakışta cevaplanıyor.
- **Zayıf yanı:** Turuncu büyük harf başlıklar ve kutulu kartlar cevabı ana yüzeyle yarıştırıyor.
  Panel 560 px ve ekranın ~%35'ini alıyor.
- **Qapsule'de yeri:** Kabuk paneli. Adım ve kart türleri çekirdek olay tiplerinden gelir.
- **Bizde ne değişiyor:** Ajan cevabı grafiğe bağlanır: kart "sabitle" ile panel olmaz, grafikte
  köken işaretli bir işarete ya da kapsüle dönüşür ve onaya düşer. Panel krom tonunda, vurgu rengi yok.
  Adım satırları varsayılan kapalı, sayıyla özetlenir ("4 adım").
- **Sınama:** Ç1 maketinde panel genişliği ≤ %30. Adım satırları kapalıyken cevabın ilk satırı
  panelin üst üçte birinde. Turuncu ya da doygun başlık rengi yok (`design-check` krom kroma eşiği).

### AT-C2. Ajan sayısı doğrulama rozetleri (grounding): verified / mismatch / unsourced
- **Kalıp:** Ajan metnindeki her sayı araç sonuçlarıyla eşleştirilir. Ön yüzde notun altında özet
  ve sayı başına rozet var: doğrulandı / uyuşmuyor / kaynaksız, ayrıca kaynaklar arası çelişki.
- **Kaynak:** OpenTerminalUI `frontend/src/agent/components/Grounding.tsx`,
  `backend/agent/grounding.py`; `refs/reports/PATTERNS.md` P5.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Güven sayı düzeyinde gösteriliyor. "Güvenir miyim" (P15 testi) sorusunun
  doğrudan görsel karşılığı.
- **Qapsule'de yeri:** Çekirdek: doğrulama motorda. Görünüş: üç durum için rol (işaret biçimi).
- **Bizde ne değişiyor:** R5B ilkesine göre doğrulanmayan yorum yayımlanmaz. Bu yüzden "uyuşmuyor"
  rozeti son kullanıcıya değil, onay kartına düşer (insan görür, yayından önce). Rozet renk değil,
  biçimdir (✓ / ≠ / ?), yeşil ve kırmızı fiyat yönüne ayrılmıştır.
- **Sınama:** Fixture: bir sayısı yanlış olan ajan notu. Yayın komutu reddedilir, onay kartında
  `mismatch` işareti görünür.

### AT-C3. Onay kartı: tür etiketi + özet + yük + gerekçe + tek eylem + durum izi
- **Kalıp:** Ajanın her yazma eylemi bir öneri kartı. Başlıkta tür etiketi ve öneri kimliği. Gövdede
  özet cümlesi, türe göre yük (emir: yön/miktar/sembol; yeniden dengeleme: mevcut → hedef tablosu ve
  değerleme zamanı; not: metin; izleme listesi: semboller), soluk gerekçe satırı ve "yalnız kâğıt
  portföy" gibi kapsam uyarısı. Onay düğmesi **eylemin adını** taşıyor ("Add to watchlist",
  "Create alert"), "OK" yazmıyor. Yanında nötr "Reject". Bekleyen öneri süresi dolabiliyor.
  Durumlar: pending → executing → confirmed / failed, rejected, expired. Karar verildikten sonra
  düğmeler kalkıyor, yerine durum satırı geliyor. Fiyat belirli bir eşikten fazla oynarsa
  "yeni öneri gerekir" deniyor.
- **Kaynak:** OpenTerminalUI `frontend/src/agent/components/ProposalCard.tsx`,
  `backend/agent/proposals.py`. Kilit fikri: OpenAlice `ui/src/components/PushApprovalPanel.tsx`
  (onay ve ret ekranda görülen kısa hash'i taşır, farklıysa çakışma; hedef başına tek bekleyen öneri;
  "user-rejected" ile "rejected" ayrı) ve nofx (onayda planı yeniden derleyip hash karşılaştırma,
  `repos/nofx.md` §S2).
- **Lisans hükmü:** OTU MIT, kalıp. **OpenAlice ve nofx AGPL: yalnız fikir** (hash kilidi, tek
  bekleyen öneri, onayda yeniden üretim).
- **Neden iyi çalışıyor:** İnsan neyi onayladığını görüyor. Düğme metni sonucu söylüyor. Bayat
  ekrandan yanlış onay engelleniyor (hash). Karar izi kartta kalıyor.
- **Qapsule'de yeri:** Çekirdek: öneri nesnesi, durum makinesi, beklenen özet alanı (`qp/0` öneri kipi,
  PROTOCOL §4.7). Kabuk: kart. Telefonda aynı kart tam ekran.
- **Bizde ne değişiyor:** Kart grafikteki önizlemeyle eşleşir: önerilen işaret grafikte ajan rolünde
  ve kesikli görünür, onaylanınca insan onaylı köken alır. Onay yalnız insan kökenli komutla verilebilir.
  Kartta "neye dayanıyor" (olgu bağı) satırı var. Süre dolumu görünür bir geri sayım değil, durum
  metnidir (sakin ton).
- **Sınama:** Protokol testi: beklenen özet uyuşmazsa onay `conflict` döner. Ajan principal'ı onay
  veremez. Kart maketinde birincil düğme metni bir fiil + nesne. Durum geçişlerinin hepsi için birer
  sahne fixture'ı.

### AT-C4. Ajan etkinlik akışı: canlı satırlar, dürüst süre, katlanan geçmiş
- **Kalıp:** "Live activity" bölümü. Her adım bir satır: durum simgesi (dönen / ✓ / !), kalın adım
  adı, tek satır ayrıntı (hesap ayrıntısı üzerine gelince görünür), geçen saniye. Çalışma sürerken
  bütün satırlar görünür ve liste aşağı takip eder. Kullanıcı yukarı kaydırırsa takip durur. İş
  bitince yalnız son 4 satır kalır, "All steps" ile açılır. Başlıkta "N tamamlandı · streaming".
  Erişilebilirlik için canlı bölge kullanılıyor. İlerleme yüzdesi uydurulmuyor, yalnız gerçek olaylar.
- **Kaynak:** nofx `NFX/web/src/components/planning/PlanningActivity.tsx`; olay türleri
  `planning/harness/harness.go` (`repos/nofx.md` §S2 madde 5).
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Neden iyi çalışıyor:** Ajanın çalıştığı görülüyor ama ekranı doldurmuyor. Bitince kendiliğinden
  sakinleşiyor. Takip ve durdurma davranışı okuyan kullanıcıyı rahatsız etmiyor.
- **Qapsule'de yeri:** Olaylar çekirdek günlüğünden gelir (sıralı `notify`). Kabuk satırları çizer.
  Telefonda "ajanlarım" ekranının satır biçimi.
- **Bizde ne değişiyor:** Satır = günlük girdisi, sıra numarası günlükten gelir (TCC'deki sırasız canlı
  çerçeve sorunu bizde yok). Süre motor saatinden. Bitmiş iş tek satırlık özete katlanır ve sonuç
  bağlantısını taşır (onay kartı ya da kapsül).
- **Sınama:** Yeniden oynatma (`replay`) ile aynı satırlar aynı sırada çıkar. İş bitince görünen satır
  sayısı ≤ 4 + özet.

### AT-C5. Karar günlüğü: döngü kartı, "Input / Chain of Thought" kapalı, sonuç satırları açık
- **Kalıp:** "Recent Decisions" sütununda her ajan döngüsü bir kart: döngü no, zaman damgası, durum
  rozeti (Success). İçinde "Input Prompt" ve "AI Chain of Thought" varsayılan kapalı başlıklar, her
  sembol için hüküm hapı (hold / wait), en altta tek aralıklı yürütme özeti (model süresi, "✓ BTCUSDT
  hold succeeded"). Kaç döngü gösterileceği seçilebilir.
- **Kaynak:** nofx `NFX/screenshots/dashboard-page.png` (3784×1990),
  `dashboard-market-chart.png` (3830×1980).
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Neden iyi çalışıyor:** Gerekçe her zaman erişilebilir ama kapalı. Sonuç (ne yapıldı) her zaman açık.
  Döngü numarası denetim izi veriyor.
- **Zayıf yanı:** Açılınca ham ve uzun model metni bölmeyi kaplıyor. Beyin emojisi, mavi/sarı başlık
  bağlantıları ve kalın sarı vurgular krom kromasız kuralını bozuyor.
- **Qapsule'de yeri:** Kabuk listesi. Kayıt çekirdek günlüğünde (sicil).
- **Bizde ne değişiyor:** Ham düşünce zinciri gösterilmez. Gerekçe yapılandırılmış kısa nedenlerdir ve
  olgulara bağlanır. Karar kartı grafikte o anın zaman çizgisine bağlanır ("bu karar burada verildi").
  Durum metni nötr, yalnız başarısızlık renkli.
- **Sınama:** Karar kartında kapalıyken en çok 3 satır. Her neden satırı bir olgu kimliğine bağlı
  (şema testi).

### AT-C6. Ajanın gelen kutusu: kaynak etiketli güncelleme + aynı yerden cevap
- **Kalıp:** Üç sütun. Solda iş akışına göre gruplanmış güncellemeler (konu, okunmamış sayısı, göreli
  zaman). Ortada seçili güncelleme: başlık, "from claude" köken satırı, tarih, bulgu ("Key finding"),
  ajanın sonraki adım sorusu, ek dosya, en altta "Reply to claude here without leaving Inbox"
  (cevap aynı ajan oturumuna gider). Sol rayda gelen kutusu sayacı.
- **Kaynak:** OpenAlice `OA/docs/images/inbox.jpg` (1440×940); `repos/openalice.md` §5.
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Neden iyi çalışıyor:** Ajanın arka planda yaptığı iş tek yerde, sakin bir okuma yüzeyinde toplanıyor.
  Köken satırı kimin yazdığını söylüyor. Cevap bağlamdan çıkmadan veriliyor.
- **Qapsule'de yeri:** Kabuk yüzeyi. Telefon kontrol kulesinin ana ekranı için en iyi aday.
- **Bizde ne değişiyor:** Güncelleme bir kapsüle ya da öneriye bağlanır. Açınca grafik o kapsülle açılır.
  Cevap bir komut olur (`qp/0`), sohbet metni değil.
- **Sınama:** Ç3 telefon maketinde ilk ekranda: bekleyen öneri sayısı, son 3 güncelleme, her birinin
  kökeni. Tek dokunuşla ilgili grafik.

### AT-C7. Zamanlı ajan işleri tahtası: durum rozetleri ve "sonraki koşu"
- **Kalıp:** Gruplu liste (In Progress / Todo / Backlog / Done, katlanabilir başlık + sayı). Satırda:
  iş adı, iş akışı + etiket (tek aralıklı, soluk), sahibi (@human), durum rozeti (Running mavi metin,
  Healthy yeşil nokta, Blocked kırmızı zeminli, Interrupted sarı zeminli + "11h ago"), zamanlama
  ("Every weekday 08:30 · in 5h").
- **Kaynak:** OpenAlice `OA/docs/images/issue-board.jpg` (1440×940).
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Neden iyi çalışıyor:** Renk yalnız sorunlu durumlarda zemin alıyor. Sağlıklı işler nokta + metinle
  sakin kalıyor. Gözün dikkati doğrudan Blocked ve Interrupted'a gidiyor. "Sonraki koşu" bilgisi
  kullanıcıya "ne zaman tekrar bakmalıyım" sorusunun cevabını veriyor.
- **Qapsule'de yeri:** Kabuk (izleme / niyet listesi). Telefonda "ajanlarım" listesi.
- **Bizde ne değişiyor:** Satırlar Qapsule kavramlarıyla adlanır (niyet, izleme). Durum motor
  ölçümünden gelir. Zeminli rozet yalnız eylem gerektiren durumda.
- **Sınama:** `design-check`: zeminli (dolgulu) durum rozeti yalnız `blocked` / `needs_attention`
  rolünde. Sağlıklı durumlarda yalnız nokta.

### AT-C8. Ajan çizimleri: ayrı katman, kilitli, kesikli, bütçeli, aç-kapa
- **Kalıp:** Ajanın grafiğe çizdikleri ayrı listede tutulur. Kilitli, seçilemez, kaydedilmez, geri
  alınmaz. İnce, kesikli çizgi ve çok saydam dolguyla çizilir, fiyat ve etiket gösterilmez. Renk
  anlamdan gelir (destek / direnç / diğer). En çok 10 aday gelir, ama dengeli bir seçimle (en çok 2
  destek + 2 direnç) yalnız 4'ü gösterilir. Gerisi "genişlet" ile açılır. Ajan katmanı tek düğmeyle
  gizlenebilir. Plan karşılaştırması açıkken otomatik gizlenir. Yeni istek eskisini geçersiz kılar
  (sürüm sayacı), bayat ajan çizimi ekrana düşmez.
- **Kaynak:** nofx `NFX/web/public/planning/chart.js` (çizim, seçim, durum fonksiyonları);
  `repos/nofx.md` §S3.
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Neden iyi çalışıyor:** İnsan çizimi ile ajan önerisi göz ucuyla ayrılıyor (düz / kesikli, etiketli /
  etiketsiz). Görsel bütçe ajanın grafiği boğmasını engelliyor. Bayatlık koruması yanlış zamana
  çizilmiş işareti önlüyor.
- **Zayıf yanı:** Fiyatı model veriyor, motor değil. Ajan çizimi geçici, onaylanıp kalıcı olamıyor.
- **Qapsule'de yeri:** **Çekirdek görünüş grameri**: köken → çizgi biçimi rolü (insan düz, ajan kesikli,
  motor ince noktalı ya da başka bir ayrım). Görsel bütçe sahne kuruluşunda uygulanır.
- **Bizde ne değişiyor:** Köken zaten şemada zorunlu (`no_origin`). Ajan işareti onaylanınca köken
  izi korunarak kalıcı olur. Fiyatı motor bağlar, ajan yalnız olguya işaret eder. Bütçe görünüş
  kaydında yoğunluk profiline göre (telefon daha az).
- **Sınama:** Sahne testi: 10 ajan işareti verilir, görünür sayısı bütçeyi aşmaz. Ajan kökenli her
  öğe `stroke_role = agent`. Altın görselde insan ve ajan çizgisi gri tonlamada da ayrılır
  (renk körlüğü testi: yalnız biçimle ayrım).

### AT-C9. Ajan işlem hattı ve çalışma sırasında dürüst durum (yarı olumsuz örnek)
- **Kalıp:** Sol sütunda koşu künyesi (sembol, "running" hapı, hüküm "—", analistler, zamanlar, "Abort
  Run") ve "Pipeline" listesi: her ajan rolü için nokta + "done / waiting". Sağda "Live Event Feed"
  ve olay sayacı. Sonuç ekranında büyük hüküm rozeti (BUY), gerekçe metni, "Re-run / Compare /
  Download".
- **Kaynak:** trading-command-center `TCC/docs/screenshots/analysis.png`, `verdict.png`
  (3024×1900); `TCC/docs/demo.gif`.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Hat listesi çok ajanlı bir işin nerede olduğunu tek sütunda gösteriyor.
  Henüz hüküm yokken "—" yazıyor.
- **Ne alınmaz:** Canlı akış ham sistem istemini döküyor (aynı paragraf satır satır tekrarlanıyor).
  Okunmaz ve güven vermez. Büyük yeşil BUY rozeti modelin hükmünü en görünür öğe yapıyor.
- **Qapsule'de yeri:** Kabuk (ajan koşusu ayrıntısı).
- **Bizde ne değişiyor:** Hat listesi alınır. Akışta yalnız tipli ve özetlenmiş olaylar görünür. Hüküm
  rozeti yok: ajan önerisi onay kartıdır, karar insanındır.
- **Sınama:** Olay satırında ham istem metni olmaz (en çok 1 satırlık özet). Tekrarlanan olay
  satırları tek satıra birleşir.

---

## D. Etkileşim

### AT-D1. Komut çubuğu: `<SEMBOL> <İŞLEV> [değiştirici]` sözdizimi + palet
- **Kalıp:** Üstte her zaman görünen komut alanı ("Type ticker, command, or search… (Ctrl+G)").
  Bloomberg tarzı işlev kodları ve takma adlar. Ayrıştırıcı beş biçim tanır (işlev; sembol; işlev +
  değiştirici; sembol + işlev; çoklu sembol + işlev), tanımadığını doğal dil sayar. Ayrıca Ctrl/Cmd+K
  paleti: yakın sayfalar, bulanık eşleşme, kapsamlara göre gruplu kısayol tablosu ve **kısayol çakışma
  bulucu**.
- **Kaynak:** OpenTerminalUI `frontend/src/components/layout/commanding.ts`, `CommandPalette.tsx`,
  `CommandBar.tsx`; `repos/openterminalui.md` §S2.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Uzman kullanıcı fareye dokunmadan çalışıyor. Sözdizimi profesyonel
  terminallerden tanıdık. Ayrıştırıcının sonucu tipli olduğu için yanlış yorum görünür kalıyor.
- **Qapsule'de yeri:** Çekirdek: ayrıştırıcı ve kısayol tablosu Rust'ta, çıktısı `qp/0` komutu. Kabuk
  yalnız metin alanı.
- **Bizde ne değişiyor:** Komut gezinme değil, doküman komutudur. İnsan ve ajan aynı komutu kullanır.
  Doğal dil girişi ajana gider ve ajan bir öneri döndürür, doğrudan uygulanmaz. Komut alanı
  sürekli bir şerit değil, palet olarak açılır (A1'deki şerit sayısı sınırı).
- **Sınama:** Ayrıştırıcı birim testleri (beş biçim + belirsizlik). Kısayol çakışma testi CI'da.
  Her palet eylemi bir komut adına eşlenir.

### AT-D2. Etkileşim ve hareket sözleşmesi: yoğun konsolda hareket yalnız anlam için
- **Kalıp:** Kısa bir belge: hareket yalnız olanak, süreklilik, durum değişimi ya da gerçek etkinlik
  için kullanılır. Her denetimin dinlenme / üzerinde / basılı / odak / devre dışı / yükleniyor durumu
  açıkça tanımlı. Gerekli bilgi yalnız hover ile gösterilmez. Yükleme ve hata, isteği yapan öğenin
  yanında gösterilir ve yeniden dene sunar. Üç hareket süresi rolü (hızlı / standart / yavaş), giriş
  mesafeleri küçük, süre < 300 ms. Azaltılmış hareket tercihi her yerde uygulanır. "Hiçbir iş akışı
  bir animasyonu izlemeyi gerektirmez." "Cilalı boş fixture kullanılabilirliği kanıtlamaz: uzun adlar,
  hatalar, finansal değerlerle sına." Duyarlı tasarım "küçültülmüş masaüstü ekran görüntüsü değil,
  bilgi hiyerarşisi değişimidir".
- **Kaynak:** OpenAlice `OA/docs/architecture/ui-interaction-and-motion.md`.
- **Lisans hükmü:** **AGPL: yalnız fikir** (ilke düzeyi; belge metni ve süre değerleri alınmaz).
- **Neden iyi çalışıyor:** "Sakin profesyonel araç" hedefinin yazılı tanımı. Ölçülebilir kurallara
  dönüşüyor.
- **Qapsule'de yeri:** Tasarım sözleşmesi (`CONTRACT.md`) + görünüş kaydında hareket rolleri. Çekirdek
  etkileşim durum makinesi.
- **Bizde ne değişiyor:** Hareket rolleri görünüş kaydından CSS özel özelliklerine üretilir, elle
  yazılmaz. Grafik içinde hareket yok (canlı bar güncellemesi dışında).
- **Sınama:** `design-check`: hareket süresi rolleri dışında süre yok. Azaltılmış hareket kipinde bütün
  süreler 0. Hata ve yükleme metni tetikleyen öğenin kabında.

---

## E. Telefon (kontrol kulesi)

### AT-E1. Telefon kabuğu: alt sekme çubuğu + yüzen ajan düğmesi
- **Kalıp:** 390×844 ekranda altta beş sekme (Home, Watch, Chart, Scan, Port), simge + küçük etiket.
  Sağ altta, sekmelerin üstünde yüzen turuncu "Agent" hapı. Ajan paneli mobilde tam yükseklik açılır,
  sekmeleri ve giriş alanını korur.
- **Kaynak:** OpenTerminalUI `OTU/assets/readme/research-mobile.png` (390×844),
  `OTU/output/playwright/agent-risk-limits-mobile.png`, `wave2-data-quality-mobile.png`;
  `frontend/src/components/layout/MobileBottomNav.tsx`.
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Başparmak bölgesinde gezinme var ve ajan her ekrandan tek dokunuşla açılıyor.
- **Nerede bozuluyor:** Masaüstü kromu telefona taşınmış: komut alanı, endeks bandı, yatay kayan
  gezinme, breadcrumb ve çalışma alanı satırı. İçerik ancak ekranın ~%25'inden sonra başlıyor.
  "Küçültülmüş masaüstü" hatasının tam örneği (D2'deki ilke).
- **Qapsule'de yeri:** Kabuk yerleşimi (telefon yoğunluk profili).
- **Bizde ne değişiyor:** Telefonda üstte yalnız bağlam başlığı (sembol / ajan adı) ve durum noktası.
  Sekmeler kontrol kulesine göre: Ajanlar · Onay · Grafik · İzleme. Onay sekmesinde sayaç. Ajan
  düğmesi turuncu değil, nötr. Bekleyen öneri varsa yalnız sayaç rengi değişir.
- **Sınama:** Ç3 maketinde ilk içerik satırı üstten ≤ 15% yükseklikte. Dokunma hedefleri ≥ 44 px.
  Sekme sayısı ≤ 4.

### AT-E2. Ekransız eşdeğerlik ve ikinci istemci (fikir)
- **Kalıp:** Onay dahil bütün iş arayüz olmadan da yapılabiliyor (CLI, MCP, sohbet uygulaması paneli).
  Grafik referansı ajan metninde bir bağlantı olarak duruyor. Masaüstü istemcide etkileşimli grafik,
  mesajlaşma istemcisinde sunucunun çizdiği PNG oluyor. Kaynak uyuşmazlığında çizim reddediliyor.
- **Kaynak:** OpenAlice `repos/openalice.md` §5 (Telegram bağlayıcısı, grafik referansı).
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Neden iyi çalışıyor:** Telefon "kontrol kulesi" bir uygulama olmak zorunda değil: bildirimden
  gelen onay kartı ve tek PNG grafik yetebiliyor.
- **Qapsule'de yeri:** Çekirdek (`qp-render` → PNG, aynı `SceneDesc`).
- **Bizde ne değişiyor:** PNG ve etkileşimli görünüm aynı çekirdek sahneden çıkar (bizde zaten
  deterministik). Onay kartı her yüzeyde aynı alanları taşır.
- **Sınama:** Aynı öneri için web kartı ve PNG kart aynı alan kümesini gösterir (sahne tanımı
  karşılaştırması).

---

## F. Boş, yükleme, hata, bağlantı yok durumları

### AT-F1. Boş durum: neden boş + bir sonraki adım, panel içinde tek satır
- **Kalıp:** Boş panellerde kısa ve somut cümleler: "No watchlist rows.", "No active alerts.", "No
  analyst ratings from the current data source." (boşluğun nedeni veri kaynağı), "No catalyst notes
  ingested for MSFT yet." İzleme listesinin üstünde besleme durumu ("NASDAQ FEED: CONNECTED · 0
  SYMBOLS"). OpenAlice'te boş izleme listesi nasıl doldurulacağını söylüyor ("Pin assets here from
  a detail page."). Ajan paneli boşken "Tool outputs will appear here.". Yeni sohbet ekranı tek soru
  soruyor ("What should Alice work on?") ve hazır bir örnek istem sunuyor.
- **Kaynak:** OpenTerminalUI `launchpad.jpg`, `agent.jpg`, `agent-risk-limits-mobile.png`;
  OpenAlice `market.jpg`, `ask-alice.jpg` (1440×940).
- **Lisans hükmü:** OTU MIT kalıp. OpenAlice AGPL, fikir.
- **Neden iyi çalışıyor:** Boşluğun nedeni ayrılıyor: veri yok / kaynak desteklemiyor / henüz
  işlenmedi. Kullanıcı hatayla boşluğu karıştırmıyor.
- **Qapsule'de yeri:** Çekirdek: "boş" durumunun nedeni bir enum. Kabuk: metin ve yer.
- **Bizde ne değişiyor:** Boş durum metni dil dosyasından ve neden kodundan gelir. Grafikte boş seri
  ekseni ve ızgarayı korur, ortada tek satır gösterir.
- **Sınama:** Her panel türü için dört durum fixture'ı (boş, yükleniyor, hata, bağlantı yok) ve her
  birinde neden metni + en çok bir eylem.

### AT-F2. Yükleme, bozulma ve bağlantı kesilmesi: yerinde ve dürüst
- **Kalıp:** Öneri kartında ayrıntı gelene kadar "Loading action details…" (durum rolüyle), düğmeler
  devre dışı. Uygulanırken düğme metni "Applying…" olur. Bağlantı durumu üç değerli (bağlı / bozuk /
  kapalı). Sağlık sayfası "DEGRADED" diyor. Nofx emir engelleyicisi hesap gözleminin 30 sn'den taze
  olmasını istiyor, değilse "needs_attention" diyor ve yeniden denetle düğmesi sunuyor. Etkinlik
  akışı ilerleme yüzdesi uydurmuyor.
- **Kaynak:** OTU `ProposalCard.tsx`, `MarketStatusBar.tsx`, `wave2-data-quality-mobile.png`; nofx
  `web/src/components/planning/FrontierOrderBlocker.tsx`, `PlanningActivity.tsx` (AGPL, fikir).
- **Lisans hükmü:** OTU MIT kalıp. nofx AGPL, fikir.
- **Neden iyi çalışıyor:** Belirsizlik açıkça adlandırılıyor. Eylem, bayat bilgiyle yapılamıyor.
- **Qapsule'de yeri:** Çekirdek (tazelik eşiği, durum makinesi). Kabuk (metin).
- **Bizde ne değişiyor:** Tazelik eşiği onay komutuna bağlanır: bayat sahneden onay reddedilir (C3'teki
  hash kilidiyle birlikte). Bağlantı yokken grafik son bilinen veriyi soluk tonla ve "son veri: 14:32"
  etiketiyle gösterir.
- **Sınama:** Bağlantı kesme fixture'ı: sahne tanımında `stale` rolü ve zaman etiketi. Bayat sahneden
  onay `conflict` döner.

---

## G. Renk, tipografi, yoğunluk

### AT-G1. Nötr krom, renk yalnız durumda ve veride (OpenAlice en temiz örnek)
- **Kalıp:** OpenAlice ekranlarında krom renksiz: koyu gri yüzeyler, beyaz-gri metin, ince ayırıcılar.
  Renk yalnız durum rozetinde (Running mavi metin, Healthy yeşil nokta, Blocked kırmızı zemin,
  Interrupted sarı zemin), okunmamış sayacında, mum ve son fiyat etiketinde var. Ayrıca semantik
  renk değişkenleri ve palet kartları var (palet ekseni ayrı bir özellik).
- **Kaynak:** OpenAlice `issue-board.jpg`, `inbox.jpg`, `market.jpg`; `ui/src/theme/palette.css`
  (yalnız varlığına ve yapısına bakıldı); `ui-interaction-and-motion.md` ("vurgu rengi seçim ve eylem
  anlamını korur, genel durum rengi olmaz").
- **Karşı örnekler:** OTU (turuncu vurgu her başlıkta, camgöbeği ve yeşil ışıklar), nofx (sarı vurgu,
  emoji, mor "beyin" simgesi), TCC (mor marka rengi düğmede, sekmede ve sembol adında).
- **Lisans hükmü:** OpenAlice AGPL, ilke düzeyinde fikir (ilke zaten OpenChart'ta MIT olarak var,
  VISUAL.md §1.4).
- **Neden iyi çalışıyor:** Göz önce sorunlu duruma ve grafiğe gidiyor. "Sakin" algısının temeli bu.
- **Qapsule'de yeri:** Görünüş kaydı rolleri (çekirdek) → kabuklara CSS özel özellikleri.
- **Bizde ne değişiyor:** Vurgu rolü yalnız seçim ve odak için. Durum rolleri: nokta (sağlıklı),
  zemin (eylem gerekiyor). Marka rengi kromda yok.
- **Sınama:** `design-check` krom kroma eşiği (krom rollerinde oklch kroma ≈ 0). Doygun renk piksel
  oranı ölçümü Ç1'de grafik dışı alanda < %3.

### AT-G2. Köken renk ve biçim rolleri (insan / ajan / motor)
- **Kalıp (dört projenin toplamı):** nofx'te ajan çizimi biçimle ayrılıyor (kesikli, etiketsiz,
  kilitli). OpenAlice'te gelen kutusunda "from claude" köken satırı ve sahip etiketi "@human".
  OTU'da araç adımları tek aralıklı soluk satır, insan sorusu "USER" etiketi. TCC'de köken gösterimi
  yok (rozetler kimin hesapladığını söylemiyor).
- **Kaynak:** C8, C6, C1, B4 kartlarındaki yollar.
- **Lisans hükmü:** Karışık. Bileşim Qapsule'e özgü (özgün katman), tek tek fikirler yukarıdaki
  lisanslarla.
- **Neden iyi çalışıyor:** Biçimle köken ayrımı renk körlüğüne dayanıklı ve veri kromasıyla
  yarışmıyor.
- **Qapsule'de yeri:** **Çekirdek görünüş grameri** (köken → çizgi biçimi, etiket öneki, metin tonu).
- **Bizde ne değişiyor:** Üç köken üç biçimde gösterilir. Örnek bir öneri: insan düz çizgi, ajan kesikli,
  motor ince sürekli ve kilit simgeli. Ajan onay bekliyorsa daha soluk. Renk ajanın "ai" vurgusuna
  ayrılmaz. Panel ve kartlarda köken ön satırı ("ajan · nöbetçi", "motor", "sen") her zaman görünür.
- **Sınama:** Gri tonlamalı altın görselde üç köken ayırt edilir (insan denetimi + biçim rolü testi).
  Her sahne öğesinde `origin` → `stroke_role` eşlemesi tam.

### AT-G3. Tipografi: arayüz grotesk, rakam tek aralıklı ya da tabular; büyük harf az
- **Kalıp:** OTU iki aileyi rolle ayırıyor: veri için tek aralıklı, arayüz için grotesk. Ama uygulamada
  tek aralıklı her yere sızmış (başlıklar, gezinme), harf aralığı geniş. OpenAlice arayüzü grotesk,
  rakamlar tabular, tek aralıklı yalnız etiket ve kimliklerde (`#morning-scan`, `codex override`).
  nofx'te tek aralıklı büyük rakamlar kart başlığında iyi duruyor, düşünce zinciri metninde kötü.
- **Kaynak:** OTU `frontend/src/styles/terminal-theme.css` (rol yapısı; değerler alınmadı), ekranlar;
  OpenAlice `issue-board.jpg`, `market.jpg`; nofx `dashboard-page.png`.
- **Lisans hükmü:** OTU MIT (rol yapısı), diğerleri gözlem.
- **Neden iyi çalışıyor (OpenAlice biçimi):** Okuma metni rahat, rakamlar hizalı, kimlikler ayırt
  edilebilir.
- **Qapsule'de yeri:** Görünüş kaydı tipografi rolleri.
- **Bizde ne değişiyor:** Üç rol: arayüz (grotesk), sayı (tabular rakamlı grotesk ya da tek aralıklı),
  kimlik/kod (tek aralıklı, küçük, soluk). Büyük harf + geniş aralık yalnız alan etiketlerinde.
- **Sınama:** `design-check`: tek aralıklı aile yalnız `numeric` ve `code` rollerinde. Tabloda sayı
  sütunları sağa hizalı ve tabular.

### AT-G4. Yoğunluk profilleri: aynı veri, üç yoğunluk
- **Kalıp:** Dört proje dört farklı yoğunlukta: OTU çok sıkı (11–13 px, kutulu), nofx gevşek ama büyük
  rakamlı, TCC gevşek tablo (satır ~100 px, iki rozetli), OpenAlice orta (liste satırı ~70 px,
  14 px gövde).
- **Kaynak:** Yukarıdaki ekran görüntüleri.
- **Lisans hükmü:** Gözlem.
- **Qapsule'de yeri:** Görünüş kaydında yoğunluk profilleri (masaüstü-sıkı, masaüstü-rahat, telefon).
- **Bizde ne değişiyor:** Profil değeri tek yerde. Satır yüksekliği, yazı ölçeği ve boşluk birlikte
  değişir.
- **Sınama:** Aynı fixture'ın üç profilde render'ı. Masaüstü-sıkı profilde Ç1'de 20+ izleme listesi
  satırı görünür.

---

## H. Diğer güçlü kalıplar

### AT-H1. Grafik üstü katman anahtarları kısa kodlarla, oturum hapları
- **Kalıp:** Grafiğin sağ üstünde kısa kodlu aç-kapa haplar (lejant, istatistik, oturum, olay, eylem,
  piyasa, temel veri), dışa aktarma (PNG/CSV), ölçek seçici. Grafiğin ortasında "MKT RTH / SESS RTH"
  oturum hapları, solda tekrar oynatma tarih seçici.
- **Kaynak:** OpenTerminalUI `chart.jpg` (2400×1500).
- **Lisans hükmü:** MIT, kalıp.
- **Neden iyi çalışıyor:** Katmanlar bir menüye gömülmüyor, durumları tek bakışta görünüyor.
- **Zayıf yanı:** Turuncu kenarlı çok sayıda hap grafiğin üstünde gürültü yapıyor.
- **Qapsule'de yeri:** Çekirdek (katman görünürlüğü doküman alanı), kabuk (hap).
- **Bizde ne değişiyor:** Haplar OpenChart'taki gibi yalnız üzerine gelince görünen yüzen çubukta.
  Açık katman nötr dolguyla, kapalı yalnız metinle gösterilir. Ajan katmanı anahtarı burada durur (C8).
- **Sınama:** Dinlenme durumunda grafik üstünde görünür hap sayısı ≤ 3.

### AT-H2. "Neyi ölçtüğünü söyleyen" metrik kartı
- **Kalıp:** B2'deki her sayı kartında, sayının altında tanımı yazıyor. nofx karar kartında model
  süresi ve "✓ … succeeded" satırları sayısal izi veriyor.
- **Kaynak:** OTU `wave2-data-quality-mobile.png`; nofx `dashboard-market-chart.png`.
- **Lisans hükmü:** OTU MIT kalıp. nofx fikir.
- **Qapsule'de yeri:** Kabuk, ama tanım metni motor ölçümünün tanımından (sözlük) gelir.
- **Bizde ne değişiyor:** Tanımlar `docs/CONCEPTS.md` adlarıyla yazılır.
- **Sınama:** Her ölçüm kartının tanım alanı dolu.

### AT-H3. İlişki grafiği görünümü (izlenenler, konular, notlar, işler)
- **Kalıp:** İzlenen varlıklar, konular, notlar ve işler düğüm-bağ grafiğinde. Seçili düğümün kısa
  kartı köşede, "Open details" düğmesi. Detay/Graph arasında geçiş. Lejant dört türle.
- **Kaynak:** OpenAlice `OA/docs/images/tracked.jpg` (1440×940).
- **Lisans hükmü:** **AGPL: yalnız fikir.**
- **Qapsule'de yeri:** Kapsam dışı (öneri). Kapsül ve olgu ilişkileri için ileride bir görünüm.
- **Bizde ne değişiyor:** RD2'de alınmaz. REPORT "Öneriler"e yazılabilir.
- **Sınama:** —

### Bulunmayan (not present)
- **Çizim araç çubuğu:** dört projede de kendi tasarımı yok. Hepsi TradingView `lightweight-charts`
  ya da widget varsayılanını kullanıyor (OTU `chart.jpg`, OpenAlice `market.jpg`, nofx
  `dashboard-page.png` üstte TV araç çubuğu). Kaynak OpenChart (VISUAL.md §1.2).
- **Eksen ve lejant düzeni:** kütüphane varsayılanı. Özgün bir kalıp yok. Dikkat: OpenAlice zaman
  ekseninde yerel ay adları ("4月") karışıyor, OTU'da TV logosu ve hacim etiketleri mumla çakışıyor
  (`launchpad.jpg`, sol alt): VISUAL.md P6 çakışma testinin karşı örneği.
- **Telefona özgü kontrol kulesi ekranı:** hiçbir projede "ajanlarımı izle" telefon ekranı yok. OTU'nun
  telefon görünümleri küçültülmüş masaüstü. E1 ve C6/C7 kartlarından bileşim yapılmalı (Qapsule'e özgü).
- **Ajan ile insan işaretini ayıran renk rolü:** yalnız nofx'te biçim ayrımı var (C8). Renk rolü olarak
  hiçbirinde yok.

---

## Başvuru ekranları

| Çerçeve | Dosya (mutlak yol) | Boyut | Neden |
|---|---|---|---|
| **Ç1 masaüstü çalışma alanı (birincil)** | `/Users/arkns/Works/Mchart/refs/src/TraderAlice__OpenAlice/docs/images/market.jpg` | 1440×940 | Sakin, nötr krom. Sol ray + ikinci gezinme + grafik + metrik kartları. "Tanıdık, sakin profesyonel" hedefe en yakın (AGPL: yalnız görsel başvuru). |
| Ç1 (yoğunluk karşılaştırması) | `/Users/arkns/Works/Mchart/refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/launchpad.jpg` | 2400×1500 | Terminal yoğunluğu, durum çubuğu, panel başlığı ve bağlama grubu. Aynı zamanda "kutu içinde kutu" karşı örneği. |
| Ç1 (ajan paneli açık) | `/Users/arkns/Works/Mchart/refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/agent.jpg` | 2400×1500 | Katlı araç adımları, açık cevap, tipli kart. |
| Ç1 (ajan işleri) | `/Users/arkns/Works/Mchart/refs/src/TraderAlice__OpenAlice/docs/images/issue-board.jpg` | 1440×940 | Durum rozetinde renk disiplini. |
| **Ç2 grafik yakın plan** | `/Users/arkns/Works/Mchart/refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/chart.jpg` | 2400×1500 | Tam genişlik grafik, OHLC lejantı, son fiyat etiketi, katman hapları, oturum hapları. |
| Ç2 (alternatif) | `/Users/arkns/Works/Mchart/refs/src/NoFxAiOS__nofx/screenshots/dashboard-page.png` | 3784×1990 | Grafik + yanında karar günlüğü (ajan ile grafiğin yan yana durumu). |
| **Ç3 telefon (okuma / gelen kutusu yerine)** | `/Users/arkns/Works/Mchart/refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/research-mobile.png` | 390×844 | Alt sekme + yüzen ajan düğmesi. Üst kromun fazla olduğunun örneği. |
| **Ç4 telefon (sağlık / durum)** | `/Users/arkns/Works/Mchart/refs/src/Hitheshkaranth__OpenTerminalUI/output/playwright/wave2-data-quality-mobile.png` | 390×844 | "Bilinmiyor ≠ sıfır", tanımlı metrik kartları, DEGRADED rozeti. |
| Ç4 (ajan paneli telefonda) | `/Users/arkns/Works/Mchart/refs/src/Hitheshkaranth__OpenTerminalUI/output/playwright/agent-risk-limits-mobile.png` | 390×844 | Ajan paneli tam yükseklik, boş durum metni ("Tool outputs will appear here."). |
| Ç3/Ç4 masaüstü kaynaklı fikir | `/Users/arkns/Works/Mchart/refs/src/TraderAlice__OpenAlice/docs/images/inbox.jpg` | 1440×940 | Telefonda kontrol kulesinin ana ekranı için içerik modeli (köken satırı, bulgu, cevap). Telefon görseli değil. |

Not: Dört projede de gerçek bir telefon kontrol kulesi ekranı yok. Ç3/Ç4 için başvuru ancak kısmi.
Telefon için asıl başvuru OpenAlice inbox içeriğinin E1 kabuğuna oturtulmuş hali olmalı.
