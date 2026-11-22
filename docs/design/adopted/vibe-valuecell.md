# Uyarlama kartları — Vibe-Trading ve valuecell (RD2, K83)

Hazırlayan: RD2 araştırma alt ajanı (salt okuma). Tarih: 2026-11-22.
Kökler: `VT/` = `/Users/arkns/Works/Mchart/refs/src/HKUDS__Vibe-Trading/`,
`VC/` = `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/`.
Lisans (`refs/reports/LICENSES.md:18-19`): Vibe-Trading MIT (ön yüz dahil; yalnız faktör
kütüphanesi ayrı), valuecell Apache-2.0. İkisinde de hüküm: **kalıp uyarlanır**. Temiz oda
gereği aşağıda kod, CSS ya da değer tablosu yok; yalnız kalıp, ölçü mantığı, rol adları ve
kanıt olarak tek tük gözlenen değer var.

Genel gözlem: Vibe-Trading'in arayüz kodu çok olgun (durum, erişilebilirlik ve "yalan
söylememe" yorumları yoğun) ama **gerçek arayüz görseli yok**: `VT/assets/*.png` 1024×1024
çizgi film illüstrasyonları, `VT/assets/Frontend.mp4` izlenemedi. valuecell'in görselleri
gerçek ekran ama açık tema, düşük yoğunluk, 16 px gövde ve orantılı rakamlar; **yapı için
başvuru, "profesyonel görünüm" için değil**. Hiçbirinde profesyonel mum grafiği yok (Vibe:
ECharts varsayılanları; valuecell: TradingView gömme widget'ı).

---

## A. Ajan paneli

### VV-A1. Tek etkinlik satırı: çalışma katlanır, cevap açık
- **Kalıp:** Ajanın bir denemesi (attempt) sohbette tek bir durum satırıdır: durum ikonu +
  "fiil · son araç · geçen süre · adım sayısı" özeti + açma oku. Çalışırken satır açıktır;
  deneme bittikten kısa bir süre sonra (gözlenen: 900 ms) kendiliğinden katlanır. Kullanıcı
  bir kez elle açtı/kapadıysa onun seçimi otomatiği ezer. Bitmiş durumlar ayrı ikon ve ayrı
  özet alır: tamam, durduruldu, zaman aşımı, başarısız; "durduruldu"da *Devam*, "zaman
  aşımı"nda *Yeniden bağlan* eylemi satırın içinde çıkar. Canlıyken satırın altında modelin
  düşünce kuyruğundan sınırlı bir "fısıltı" (3 satır, italik, soluk) görünür, bitince kaybolur.
  Nihai cevap bu satırın **dışında**, normal metin olarak durur. Satır `role=status`,
  `aria-live=polite`. Sayaç sekme gizliyken durur; yeniden bağlanınca süre arka ucun
  başlangıç zamanından sürer (yeniden bağlanma anından değil).
- **Kaynak:** `VT/frontend/src/components/chat/ActivityLine.tsx` (özellikle 60-204),
  `ThinkingTimeline.tsx` (eski kayıtları aynı satıra çeviren uyarlayıcı),
  `VT/frontend/src/pages/Agent.tsx:736-738, 942-961` (süre otoritesi arka uç).
  valuecell karşılığı daha zayıf: "Thinking" ve her araç ayrı katlanır kutu
  (`VC/frontend/src/components/valuecell/renderer/reasoning-renderer.tsx`).
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Göz cevaba gider; iş adımları tek satıra sıkışır ama kaybolmaz.
  Otomatik katlanmanın kullanıcı seçimini ezmemesi "panel kendi kendine zıplıyor" hissini
  önler. Bitiş durumlarının ayrılması (durduruldu ≠ başarısız ≠ zaman aşımı) güven verir.
- **Qapsule'de yeri:** Kabuk paneli (DOM). Durum kümesi ve özet dizgisi çekirdekte
  tanımlanabilir (ajan koşu olayları `notify` akışına eklenirse), görünüş kabukta.
- **Bizde ne değişiyor:** Satır bir *ajan koşusu*nu gösterir, ama her adımın sonucu belgeye
  yazılan bir komutsa (op) adım satırı o komutun kökenine (agent) ve günlük sırasına bağlanır;
  "fısıltı" yalnız model metnidir, sayı içeremez (motor sayısıyla karışmasın). Bitişte
  "N işaret eklendi, M öneri onay bekliyor" gibi belge etkisi özeti eklenir.
- **Sınama:** Maket/oynatma ile: (1) 8 adımlı koşuda satır tek satır yüksekliğinde kalıyor
  mu; (2) bitişten ~1 sn sonra katlanıyor, elle açılmışsa açık kalıyor mu; (3) 4 bitiş
  durumu ekran okuyucuda farklı okunuyor mu; (4) yeniden bağlanmada süre geri sarılmıyor mu.

### VV-A2. Araç çağrısı satırları: argüman özeti, birleştirme, dürüst ilerleme
- **Kalıp:** Açılan etkinlik satırının içinde her araç çağrısı bir satır: durum ikonu,
  yerelleştirilmiş araç adı, **argüman özeti** (önce kullanıcının sorduğu türden argümanlar:
  ad, sembol, sorgu, yol; ~42 karakterde kırpılır), geçen süre. Ardışık **başarılı** aynı araç
  çağrıları tek satıra birleşir (sayı ile). Belirli ilerleme varsa küçük bir halka (12 px)
  çizilir; kalan süre tahmini yalnız güvenilir olduğunda gösterilir (aynı aşamada, en az 3
  birim ve toplamın %10'u geçilmiş, ilerleme geri gitmemiş). Süre biçimi "<0.1s / 1.4s /
  12s / 2m 5s" basamaklı. valuecell'de araç kutusu sonuç gelene dek döner ikonla kilitli,
  sonuç gelince açılabilir hâle gelir.
- **Kaynak:** `VT/frontend/src/components/chat/ToolProgressIndicator.tsx` (ETA mantığı 15-36,
  argüman özeti 46-72, birleştirme ~125+), `VT/frontend/src/pages/Agent.tsx:860` (ilerleme
  olaylarını kare başına birleştirme). `VC/frontend/src/components/valuecell/renderer/tool-call-renderer.tsx`;
  görsel: `VC/assets/product/superagent.png` (iki araç satırı + cevap).
- **Lisans hükmü:** MIT / Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Aynı aracın yedi çağrısı ancak argümanla ayırt edilir; birleştirme
  gürültüyü keser; yanlış ETA göstermemek "tahmin ediyor gibi yapma" ilkesidir.
- **Qapsule'de yeri:** Kabuk paneli. Argüman özeti için "önemli alan" listesi `qp-schema`
  araç şemasından üretilebilir (elle yazılmaz).
- **Bizde ne değişiyor:** Okuma araçları (query_*) ile yazma araçları (op_*) ayrı ikon/ton
  alır; yazma satırına tıklamak grafikte o işareti vurgular (çizimden niyete bağ).
- **Sınama:** 7 ardışık `query_get` + 1 `op_add_finding` koşusunda panel 2 satır mı
  gösteriyor; argüman özeti her satırı ayırıyor mu; ilerleme olmayan araçta ETA hiç
  görünmüyor mu.

### VV-A3. Bileşen türü → çizici eşlemesi; bilinmeyen tür görünür kalır
- **Kalıp:** Ajan akışındaki her öğe bir `component_type` ve `item_id` taşır; ön yüz kapalı bir
  eşleme tablosuyla türü çiziciye bağlar (markdown, reasoning, tool_call, report, scheduled
  task, alt ajan konuşması). Aynı `item_id` gelirse öğe yerinde güncellenir (ekle/değiştir).
  Tanınmayan tür sessizce düşmez: tür adı + ham içerik bir "bilinmeyen bileşen" kutusunda
  gösterilir.
- **Kaynak:** `VC/frontend/src/constants/agent.ts:58-68`, `VC/frontend/src/lib/agent-store.ts:170-270`,
  `VC/frontend/src/components/valuecell/renderer/unknown-renderer.tsx`; Vibe'de SSE tipi →
  kart eşlemesi (`VT/frontend/src/hooks/useSSE.ts`, `refs/reports/repos/vibe-trading.md` §5).
- **Lisans hükmü:** Apache / MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Yeni ajan yetenekleri arayüzü kırmaz; kayıp veri olmaz.
- **Qapsule'de yeri:** Kabuk (çizici tablosu), tür listesi `qp-schema`'dan üretilir.
- **Bizde ne değişiyor:** Tür kümesi kapalı ve şemadan gelir; bilinmeyen tür bir *protokol
  hatası* olarak da günlüğe düşer (yalnız ekranda değil).
- **Sınama:** Şemaya olmayan bir tür enjekte edilince panelde görünür kutu + günlükte hata.

### VV-A4. Ajan çıktısı kartı → yan bölmede ayrıntı; iç içe alt ajan kartı
- **Kalıp:** Uzun ajan çıktısı (rapor) sohbette tek satırlık bir kart olarak durur (ikon,
  başlık, oluşturma zamanı, sağda ok); tıklanınca ekranın sağ yarısında ayrıntı bölmesi açılır,
  başında "Geri" ve başlık. Alt ajana devredilen iş, sohbette ajan adı hapı + sağda durum
  ("Çalışıyor" döner ikonla, bitince "Görüntüle" düğmesi) taşıyan, içinde alt konuşmayı
  sınırlı yükseklikte gösteren bir kartla görünür.
- **Kaynak:** `VC/frontend/src/components/valuecell/renderer/report-renderer.tsx`,
  `VC/frontend/src/provider/multi-section-provider.tsx`,
  `VC/frontend/src/app/agent/components/chat-conversation/chat-multi-section-component.tsx`,
  `VC/frontend/src/components/valuecell/renderer/chat-conversation-renderer.tsx`.
  Görseller: `VC/assets/product/NewPushAgent.png` (sohbet solda, rapor sağda),
  `VC/assets/product/superagent.png` ("Research Agent [View]").
- **Lisans hükmü:** Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Sohbet akışı kısa kalır; okunacak metin geniş bir sütuna taşınır;
  devredilen işin kimde ve hangi durumda olduğu tek bakışta görünür.
- **Qapsule'de yeri:** Kabuk paneli; "ayrıntı bölmesi" Qapsule'de kapsül kartı ya da bulgu
  ayrıntısıdır.
- **Bizde ne değişiyor:** Ayrıntı bölmesi sohbetin değil **grafiğin** yanında açılır; karttaki
  ok grafiği ilgili zamana/işarete kaydırır. Alt ajan kartında ajan adı yerine *rol* (yorumcu,
  nöbetçi) ve köken rozeti.
- **Sınama:** Ç1 maketinde ajan raporu açıkken grafik görünür kalıyor mu (en az %50 genişlik).

### VV-A5. Soru / onay kartı ve ayrıcalıklı onay yolu
- **Kalıp:** Ajan bir **öneri nesnesi** üretir; kart 2-4 seçenek karosunu somut sayılarla
  gösterir (evren, emir başı en çok tutar, günlük sınır, kaldıraç). *Onayla* sohbet iletisi
  değil, ajanın erişemeyeceği ayrı bir uç çağrısıdır; *Düzelt* doğal dil iletisi olarak ajana
  döner ve yeni öneri çizilir. Hesap gibi bağlayıcı seçim **önceden seçili gelmez**. Onaydan
  sonra kart, aynı aileden kısa bir rozete çöker (etkin · sınırlar · bitiş tarihi); yeniden
  yetki isteyen varyant uyarı ikonuyla ayrılır. Daha hafif varyant (zamanlanmış araştırma
  önerisi): tanım listesi (sıklık, saat dilimi, bitiş, teslim) + Vazgeç/Onayla; sonuçlanınca
  tek satırlık makbuza döner ("onaylandı / vazgeçildi / süresi doldu"). Bu kartlar düşünce
  zaman çizelgesine **katlanmaz**, ayrı öğe olarak durur.
- **Kaynak:** `VT/frontend/src/components/chat/MandateProposalCard.tsx:190-330`,
  `VT/frontend/src/components/chat/ScheduledResearchProposalCard.tsx:45-122`,
  `VT/frontend/src/pages/Agent.tsx:170-174`. Karşı örnek: valuecell onayı düz sohbetle alır
  ("please confirm: every 5 minutes" → kullanıcı "yes") ve sonra İptal düğmeli bir görev
  kartı gösterir (`VC/assets/product/NewPushAgent.png`,
  `VC/frontend/src/components/valuecell/renderer/scheduled-task-controller-renderer.tsx`).
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** İnsan neyi onayladığını sayılarla görür; onay kanalı ajandan
  ayrılmış olduğu için ajan kendi önerisini onaylayamaz; sonuçlanan öneri akışı kirletmez.
- **Qapsule'de yeri:** Kabuk paneli (kart) + çekirdek protokol (`qp/0` öneri kipi; onay
  yalnız insan kökenli komutla).
- **Bizde ne değişiyor:** Öneri, grafikte **hayalet önizleme** olarak da görünür (onaylanmamış
  işaret, ajan kökeni, kesikli/soluk); onay kartı önizlemeyi ve etkilediği nesne sayısını anar;
  onay komutu öneri özetine (hash) kilitlenir (PATTERNS P2).
- **Sınama:** (1) Ajan principal'ı onay komutu gönderince red; (2) onaydan sonra kart rozete
  çöküyor, günlükte insan kökenli onay kaydı var; (3) 5 saniyelik bakışta kullanıcı "neyi
  onaylıyorum" sorusunu cevaplayabiliyor mu (insan testi P15'e eklenebilir).

### VV-A6. Ajan kişileri (ünlü yatırımcı avatarları) — kaçınılacak
- **Kalıp:** Her ajan gerçek bir yatırımcının adı ve portresiyle temsil ediliyor (Buffett,
  Munger, Burry …), ajan pazarı kart ızgarası.
- **Kaynak:** `VC/frontend/src/constants/agent.ts:70-98`, `VC/assets/product/agent_market.png`
  (2818×1710, açılmadı).
- **Lisans hükmü:** Apache, ama içerik açısından **alınmaz**.
- **Neden (kötü):** Gerçek kişiyi taklit; güveni otoriteye bağlar, sicile değil.
- **Qapsule'de yeri / değişen:** Ajanlar rol adı + köken rengiyle görünür; güven kapsül
  sicilinden gelir. **Sınama:** Sözleşmede "yapılmayacaklar" maddesi; design-check kişi adı/
  portre kullanan ajan kimliğini reddeder.

---

## B. Durum ve sağlık

### VV-B1. Bağlantı bandı: yalnız gerçek kopuşta, sayaçlı, sonra kalıcı
- **Kalıp:** Bağlantı bandı yalnız *yeniden bağlanma döngüsü* sırasında görünür; açılış ve
  bilinçli sayfa değişimi "kopuş" sayılmaz. Bant içeriğin üstünde ince bir şerit (en az 32 px),
  uyarı tonunda, deneme sayısıyla ("yeniden bağlanıyor, deneme 3"); 5 denemeden sonra kalıcı
  "bağlantı yok" + *Yeniden yükle*. valuecell: arka uç sağlığı hatası 500 ms gecikmeyle
  gösterilir (kısa kesintide titreme yok).
- **Kaynak:** `VT/frontend/src/components/layout/ConnectionBanner.tsx`,
  `VC/frontend/src/components/valuecell/app/backend-health-check.tsx`.
- **Lisans hükmü:** MIT / Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Yanlış alarm yok; kullanıcı ne olduğunu ve ne yapacağını biliyor.
- **Qapsule'de yeri:** Kabuk (DOM şeridi); durum kaynağı çekirdek oturum/protokol durumu.
- **Bizde ne değişiyor:** Kopuşta grafik "donmuş veri" olarak işaretlenir (son bar zamanı +
  "canlı değil" rozeti) ki kullanıcı bayat fiyatı canlı sanmasın.
- **Karşı örnek:** valuecell aynı ekranda **rastgele artan sahte ilerleme çubuğu** kullanıyor
  (ölçülmüş bir şey değil). Qapsule'de yasak: ilerleme yalnız ölçülen değerden; yoksa
  belirsiz gösterge.
- **Sınama:** Sunucu 2 sn kesilince bant çıkıyor, döngü bitince kayboluyor; 300 ms kesintide
  hiç görünmüyor; ilerleme çubuğunda ölçülmemiş değer yok (kod incelemesi).

### VV-B2. Koşucu / nöbetçi sağlık paneli: son kalp atışı ve bitiş geri sayımı
- **Kalıp:** Otonom koşucunun paneli profil başına: çalışıyor mu, **son tick "x sn/dk/sa
  önce"**, etkin yetkinin sınırları ve bitiş geri sayımı ("2g 4s", "45d"; "yakında" ve "süresi
  doldu" ayrı tonlarda). Tek bir yoklayıcı hem acil durdurmayı hem paneli besler. Başlat/
  durdur ayrıcalıklı uç, sohbet değil. Panel kompakt bir anahtara çöker. valuecell strateji
  kartında "Çalışıyor / Durdu" + durma nedeni ipucu.
- **Kaynak:** `VT/frontend/src/components/chat/RunnerStatus.tsx:45-70, 324-334`,
  `VT/frontend/src/components/chat/LiveRuntimePanel.tsx`;
  `VC/frontend/src/app/agent/components/strategy-items/trade-strategy-group.tsx:126-175`.
- **Lisans hükmü:** MIT / Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** "Çalışıyor" yazısı tek başına yalan olabilir; son kalp atışı
  yaşını göstermek bayatlığı görünür kılar.
- **Qapsule'de yeri:** Kabuk paneli; değerler çekirdekten (izleme/niyet durumu, son
  değerlendirme barı).
- **Bizde ne değişiyor:** Qapsule'de "nöbetçi" bir izleme (watch/intent) ve son
  değerlendirdiği **bar zamanı** ile gösterilir (duvar saati değil, piyasa saati); yaş eşiği
  zaman dilimine göre (1d grafikte 2 bar gecikme ≠ 1m grafikte).
- **Sınama:** Değerlendirme durdurulunca rozet eşik sonrası "bayat" tonuna geçiyor mu.

### VV-B3. Sayının kökeni rozeti: tahmin / etiketsiz / ölçülmemiş
- **Kalıp:** Token toplamlarında yalnız sağlayıcının kesin ölçümü rozetsiz; sezgisel sayım
  "tahmini", kaynağı bilinmeyen "etiketsiz" rozeti alır; kısmen tahmini bir toplam asla kesin
  okunamaz. İlk ölçüm gelmeden sayaç **hiç çizilmez** (uydurma sıfır yok). Çalışma raporunda
  strateji başlığı modelden çıkarıldıysa "çıkarıldı" rozeti.
- **Kaynak:** `VT/frontend/src/components/chat/SwarmStatusCard.tsx:120-125`,
  `VT/frontend/src/components/chat/LlmUsageCounter.tsx:9-13`,
  `VT/frontend/src/components/charts/StrategyResearchDashboard.tsx` (`strategyInferred` rozeti).
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Okur sayının ne kadar güvenilir olduğunu sayının yanında görür.
- **Qapsule'de yeri:** Çekirdek gramer (köken: human / agent / engine) + kabuk rozeti.
- **Bizde ne değişiyor:** Qapsule'ün köken modeli bunun genel hâli: motor sayısı rozetsiz
  (varsayılan güvenilir), ajan metnindeki sayı "ajan" rozeti ya da doğrulama durumu taşır;
  ölçülmemiş değer "—" değil, açık "veri yok" durumu.
- **Sınama:** Kapsül kartında her sayı için köken sorgulanabilir (`query_verify_evidence`);
  maket incelemesinde rozetsiz ajan sayısı yok.

---

## C. Strateji/kapsül kartı ve sicil

### VV-C1. Strateji kartı (kapsül kartının en yakın benzeri)
- **Kalıp:** Liste kartı: üst satır ad (kalın) + sağda zaman (soluk); etiket hapları (strateji
  türü, *Sanal/Canlı*); soluk model kimliği; alt satır **kâr/zarar mutlak (yüzde)** yön
  renginde + durum + eylemler (durdur onay diyaloğuyla, ayrıntı, çoğalt, sil). Seçili kart
  ajan vurgu kenarı alır. Liste boşsa kesikli çerçeveli yer tutucu (bkz. E1).
- **Kaynak:** `VC/frontend/src/app/agent/components/strategy-items/trade-strategy-group.tsx:60-330`;
  görsel `VC/assets/product/AutoTradingAgent.png` sol sütun.
- **Lisans hükmü:** Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Kimlik, tür, kim yazdı (model), sonuç ve durum beş satırda;
  *Sanal* etiketi gerçek parayla karışmayı önler.
- **Qapsule'de yeri:** Kabuk paneli (kapsül listesi); alanlar çekirdek kapsül tipinden.
- **Bizde ne değişiyor:** Kâr/zarar yerine **iddia + sicil**: tek cümle iddia, isabet oranı
  ve örneklem (n), son sonuç, köken (ajan/insan); para sonucu yoksa yön rengi kullanılmaz.
  valuecell'in zayıflıkları alınmaz: 16 px+ büyük tip, orantılı rakam, kart başına bol boşluk
  (görselde 3 kart ekran yüksekliğini dolduruyor).
- **Sınama:** Ç1 maketinde 320 px genişlikte en az 6 kapsül kartı görünüyor; rakamlar
  tabular; 5 sn bakışta "hangisi daha güvenilir" sorusu cevaplanıyor.

### VV-C2. Karar döngüsü sicili: gerekçe katlı, sonuç çipli
- **Kalıp:** Zaman sırasının tersinde "Döngü #n" kartları: başlık + zaman damgası; "Yapay zekâ
  gerekçesi" etiketi altında tek satıra kırpılmış gerekçe (açılınca tam metin, ajan vurgu
  kenarı); "İşlem" etiketi altında çip satırı: sembol ikonu, eylem (ör. CLOSE LONG), kaldıraç,
  sağda sonuç yön renginde. Çipin üstüne gelince ayrıntı kartı: zaman, fiyat aralığı, miktar,
  tutma süresi, ücret ve gerekçe.
- **Kaynak:** `VC/frontend/src/app/agent/components/strategy-items/strategy-compose-list.tsx`;
  görsel `VC/assets/product/AutoTradingAgent.png` orta sütun.
- **Lisans hükmü:** Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Ajanın *ne düşündüğü* ile *ne oldu* yan yana; gerekçe varsayılan
  katlı olduğu için sonuçlar taranabilir.
- **Qapsule'de yeri:** Kabuk paneli (sicil listesi); kayıtlar çekirdek defterinden (ledger).
- **Bizde ne değişiyor:** Her kayıt bir kapsül olayıdır: iddia anı → değerlendirme anı →
  sonuç (tuttu / tutmadı / süresi doldu), sonuç **motor** kökenli; gerekçe ajan kökenli ve
  sayısız. Kayda tıklamak grafikte iddia ve değerlendirme barlarını gösterir. İleriye bakmama
  kuralı (CAPSULE §9.4) kayıtta görünür: değerlendirme barı zamanı.
- **Sınama:** Sicilde her satırın sonucu motor kökenli; gerekçede sayı yok (design-check
  ya da doğrulayıcı); satırdan grafiğe gidiş tek tık.

### VV-C3. Çalışma kartı (run card): yeniden üretilebilirlik ve uyarılar önce
- **Kalıp:** Çalışma ayrıntısında bir "run card" sekmesi: üstte 4 sayaç karosu (şema sürümü,
  üretim zamanı, veri kaynakları, **uyarı sayısı** — sıfır değilse uyarı tonu); uyarı varsa
  hemen altında uyarı kutusu; sonra iki sütunlu paneller: geriye dönük test özeti,
  **yeniden üretilebilirlik** (hash'ler, eşaralıklı), metrikler, doğrulama yükü; en altta
  yürütme kayıtları (her araç izi, durum çipi). Boş anahtarlar "kayıt yok" diye açıkça yazılır
  (boş hücre "eksik" okunmasın).
- **Kaynak:** `VT/frontend/src/pages/RunDetail.tsx:65, 160-170, 478-560`,
  `VT/frontend/src/components/run/RunCard.tsx`.
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Güven bilgisi (kaynak, hash, uyarı) sonuçtan ayrı ama aynı yerde;
  uyarı sayısı ilk bakışta.
- **Qapsule'de yeri:** Kabuk (kapsül ayrıntı / kanıt sekmesi); veriler çekirdek
  (`query_verify_evidence`, günlük).
- **Bizde ne değişiyor:** "Kanıt" sekmesi: kapsülün dayandığı olgular, veri kaynağı, günlük
  sırası, doğrulama durumu. Hash gösterimi kısaltılmış + kopyala.
- **Sınama:** Kapsül ayrıntısında uyarı/doğrulanamayan kanıt varsa sekme başlığında sayı.

### VV-C4. Rapor sayfası: numaralı şekiller ve tek cümlelik okuma notu
- **Kalıp:** Çalışma panosu bir araştırma raporu gibi dizilir: üst kimlik bandı (küçük eyebrow,
  başlık, istem özeti, tarih aralığı/motor/kaynak hapları, kısa çalışma kimliği), altında 6'lı
  KPI şeridi (eşaralıklı rakam), sonra "Şekil 01…05" başlıklı bölümler; her şeklin altında
  **ne okunacağını söyleyen tek cümle** (ör. "kayıpların ne zaman derinleştiğini görün").
  Çalışma türü panoya uymuyorsa boş çizgiler yerine "bu tür çalışma burada çizilmez, şu
  sekmeye bak" açıklaması.
- **Kaynak:** `VT/frontend/src/components/charts/StrategyResearchDashboard.tsx:112-120, 211-277`;
  metinler `VT/frontend/src/i18n/locales/tr.json:2229-2235`.
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Okura her şeklin sorusunu verir; "bozuk mu, farklı mı" belirsizliği
  ortadan kalkar.
- **Qapsule'de yeri:** Kabuk (kapsül/hikâye sunum görünümü); "okuma notu" çekirdek anlatı
  (narrative) alanı olabilir.
- **Bizde ne değişiyor:** Okuma notu ajan yazar, ama sayı içeremez; şekiller Qapsule
  çekirdeğinden çizilir.
- **Karşı örnek (alınmaz):** KPI değerleri metriğe göre sabit marka renkleri alıyor (mavi,
  turuncu, yeşil) ve başlık bandında degrade + bulanık daire süsü: kroma krome taşmış. Qapsule
  kuralı: KPI nötr; renk yalnız yön/sonuç anlamı varsa.
- **Sınama:** Kapsül sunum maketinde her şeklin altında ≤1 cümle not; KPI'larda anlamsız renk yok.

### VV-C5. Hedef paneli: kriter başına kanıt sayısı ve doğrulama durumu
- **Kalıp:** Ajan ve insanın ortak yazdığı hedefin paneli: kriter listesi, her kriterde durum
  ve "N kanıt"; kanıt satırında kaynak sağlayıcı + doğrulama durumu; başlıkta toplam kanıt hapı.
  Kanıtı olan ama durumu "açık" kalmış kriter "kapsandı" diye ayrıca işaretlenir.
- **Kaynak:** `VT/frontend/src/components/chat/GoalPanel.tsx:7-50, 138-245`.
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** İddia ile kanıt arasındaki bağ sayılabilir ve denetlenebilir.
- **Qapsule'de yeri:** Kabuk paneli; kapsülün iddia → olgu bağı çekirdekte.
- **Bizde ne değişiyor:** Kriter = kapsül koşulu; kanıt = olgu (fact) kaydı, köken ve
  doğrulama durumuyla.
- **Sınama:** Kapsül kartında koşul başına kanıt sayısı sorgu sonucuyla aynı.

### VV-C6. Model yarışı / sıralama tablosu
- **Kalıp:** Stratejiler (her biri bir model) aynı tabloda: sıra (ilk üç için madalya ikonu),
  kullanıcı/strateji, kâr/zarar, borsa, model, ayrıntı; yanında model başına çok çizgili
  portföy değeri grafiği.
- **Kaynak:** `VC/frontend/src/app/rank/board.tsx`,
  `VC/frontend/src/components/valuecell/charts/model-multi-line.tsx`;
  `refs/reports/repos/valuecell.md` §11.
- **Lisans hükmü:** Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Karar kalitesi sonuçla, yan yana karşılaştırılır.
- **Qapsule'de yeri:** Kabuk (sicil karşılaştırma görünümü).
- **Bizde ne değişiyor:** Sıralama ham kâra değil, örneklem büyüklüğüne duyarlı sicile göre
  (n küçükse sıralanmaz, "yetersiz örneklem"); madalya süsü yok.
- **Sınama:** n < eşik kapsüller sıralamada ayrı grupta.

### VV-C7. İki çalışmayı karşılaştırma: hücre içi oransal çubuk
- **Kalıp:** Metrik tablosunda iki sütun; her hücrenin arkasında iki değerin büyüğüne göre
  oransal ince bir çubuk (lejant rengiyle), kazanan tarafı sayı okumadan gösterir; "yüksek
  iyi / düşük iyi" metrik başına tanımlı; düşüş işaretli kayıp olarak saklanıyor, büyüklük
  olarak karşılaştırılıyor.
- **Kaynak:** `VT/frontend/src/pages/Compare.tsx:76, 420-440`.
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Tabloyu tarama hızında okunur yapar, sayıyı da korur.
- **Qapsule'de yeri:** Kabuk (kapsül karşılaştırma) ya da çekirdek tablo gramerinde bir kodlama.
- **Bizde ne değişiyor:** Çubuk rengi nötr (veri kroması değil), kazanan yalnız kalınlıkla.
- **Sınama:** Renk körlüğü simülasyonunda kazanan yine ayırt ediliyor.

---

## D. Renk, tipografi, yoğunluk

### VV-D1. Yön rengi ile kâr/zarar rengi ayrı roller
- **Kalıp:** Mum/hacim **yön** renkleri yerel ayara göre ters döner (Çin: kırmızı = yükseliş);
  **kâr/zarar** renkleri dönmez (yeşil = kâr her dilde) ve ayrı anlamsal tokenlardan okunur.
  valuecell aynı şeyi kullanıcı ayarı olarak sunar (yeşil-yukarı / kırmızı-yukarı).
- **Kaynak:** `VT/frontend/src/lib/chart-theme.ts:37-39`, `VT/frontend/src/lib/pnl-colors.ts:1-8`;
  `VC/frontend/src/store/settings-store.ts:18-110`.
- **Lisans hükmü:** MIT / Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** İki farklı anlam tek renk çiftine bindirilmez; yerel gelenek kâr
  okumasını bozmaz.
- **Qapsule'de yeri:** Çekirdek görünüş kaydı (look registry): `up/down` ve `gain/loss` ayrı
  roller; kabuklara CSS özel özellikleri olarak çıkar.
- **Bizde ne değişiyor:** Üçüncü rol çifti eklenir: kapsül sonucu (tuttu/tutmadı) — yön değil,
  doğruluk; şekil/simgeyle de taşınır (renk tek kanal değil).
- **Sınama:** Yerel ayar `zh` yapılınca mumlar döner, kâr/zarar ve kapsül sonucu dönmez
  (altın görsel değil, rol tablosu testi).

### VV-D2. Krom nötr, kroma veride — ve karşı örnek
- **Kalıp:** valuecell arayüzü shadcn "neutral" temasında: yüzey ve metin rolleri kromasız
  (gözlenen: koyu temada arka plan kromasız, çok koyu gri), kenar beyazın %10 alfası; kroma yalnız
  fiyat değişimi ve vurguda. Vibe-Trading ise krome sıcak bir umber birincil renk ve sıcak
  tonlu yüzeyler koyuyor; grafik ızgara/metin tokenları da sıcak. Vibe'nin iyi tarafı: koyu
  temada **4 basamaklı yüzey merdiveni** ve "komşu yüzeyler arasında ≥5 açıklık puanı" kuralı,
  kontrast ölçümleri token yorumunda (ör. birincil üstü yazı için ölçülen oran yazılı).
- **Kaynak:** `VC/frontend/src/global.css:42-110`; `VT/frontend/src/index.css:6-80`
  (merdiven kuralı ve kontrast notu koyu tema bloğunun başında).
- **Lisans hükmü:** Apache / MIT → kalıp uyarlanır (değer tablosu kopyalanmaz).
- **Neden iyi çalışıyor:** Nötr krom grafiği öne çıkarır (OpenChart ile aynı sonuç,
  `refs/reports/VISUAL.md` §1.4); yüzey basamak kuralı "panel kayboldu" hatasını ölçülebilir
  kılar; kontrastın token yanında yazılması kararı denetlenebilir yapar.
- **Qapsule'de yeri:** Çekirdek görünüş kaydı; sözleşmede (CONTRACT) kural olarak.
- **Bizde ne değişiyor:** Nötr krom (valuecell/OpenChart yolu) + Vibe'nin ölçü disiplini:
  yüzey basamakları arasında asgari açıklık farkı ve her metin rolünün ölçülmüş kontrastı
  görünüş kaydında test olarak.
- **Sınama:** design-check: krom rolünde kroma > eşik yok; komşu yüzey açıklık farkı ≥ eşik;
  her metin/yüzey çifti kontrast eşiğini geçiyor.

### VV-D3. Ajan vurgu rengi (degrade kenar)
- **Kalıp:** valuecell ajan kökenli öğelere (düşünce, araç çağrısı, rapor kartı, gerekçe
  kutusu, seçili strateji) pembe→mavi degrade 1 px kenar veriyor; düz öğeler nötr kenarlı.
  Sorun: aynı degrade *üzerine gelme* durumunda da çıkıyor, yani "ajan" ile "etkileşim"
  anlamı karışıyor.
- **Kaynak:** `VC/frontend/src/global.css:133-156` (border-gradient yardımcı sınıfı ve
  `data-active` durumu); görsel `VC/assets/product/AutoTradingAgent.png` (açık gerekçe ve
  seçili kart).
- **Lisans hükmü:** Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Ajan yazımı nötr bir arayüzde tek bir imza işaretiyle ayrılıyor;
  büyük yüzey boyamadan.
- **Qapsule'de yeri:** Çekirdek görünüş kaydı: `origin.agent` rolü (panel kenarı + grafikteki
  ajan işaretleri); kabukta CSS özelliği.
- **Bizde ne değişiyor:** Ajan rolü **tek ton** ve yalnız köken anlamında (hover'da kullanılmaz);
  grafikte ajan işareti renk + çizgi deseni/rozetle (renk tek kanal değil); insan işareti
  nötr/kullanıcı rengi, motor işareti ayrı bir soluk rol. Degrade kullanılmaz (tuvalde ve
  küçük boyda okunmaz).
- **Sınama:** Aynı ekranda insan, ajan ve motor işareti renk körlüğü simülasyonunda ve gri
  tonlamada ayırt ediliyor.

### VV-D4. Rakam tipografisi ve yoğunluk
- **Kalıp:** Vibe: metrik karosunda etiket çok küçük, büyük harf, aralıklı, soluk; değer
  eşaralıklı + `tabular-nums` (ön yüzde 102 kullanım); sohbet etkinlik satırı 13 px, satır
  yüksekliği en az 32 px; Inter. valuecell: gövde 16 px, `tabular-nums` yalnız 1 kullanım,
  görsellerde rakamlar orantılı ve kartlar seyrek.
- **Kaynak:** `VT/frontend/src/components/chat/MetricsCard.tsx`, `ActivityLine.tsx`,
  `VT/frontend/tailwind.config.ts:22-23`; `VC/assets/product/AutoTradingAgent.png`, `homepage.png`.
- **Lisans hükmü:** MIT / Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor (Vibe):** Üç ton basamağı (etiket soluk/küçük, değer kalın/eşaralıklı)
  sayıyı öne çıkarır; tabular rakam sütunları hizalar. valuecell bunun neden gerekli olduğunu
  ters yönden gösteriyor: aynı içerik büyük tip ve orantılı rakamla "tüketici uygulaması"
  okunuyor.
- **Qapsule'de yeri:** Çekirdek (grafik içi etiketler, eksen) + kabuk tip ölçeği.
- **Bizde ne değişiyor:** Masaüstü yoğunluk profilinde gövde 13-14 px, sayı her yerde tabular;
  telefon profili ayrı (R5'in 13 px / 44 px eşikleri yalnız telefonda).
- **Sınama:** design-check: sayı içeren her metin rolü tabular; masaüstü maketinde 1440×900'de
  kapsül listesi + grafik + ajan paneli aynı anda.

### VV-D5. Durumu yalnız renkle söyleme
- **Kalıp:** Eşik yargısı olan metriklerde (Sharpe +0,20 kırmızı, +1,50 yeşil) renk yanında
  küçük ▲/▼ simgesi ve ekran okuyucu için "iyi/zayıf" sözcüğü; metrik adının üstüne gelince
  kısa açıklama.
- **Kaynak:** `VT/frontend/src/components/chat/MetricsCard.tsx:6-60`.
- **Lisans hükmü:** MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Renk körü ve yardımcı teknoloji kullanıcıları için aynı bilgi.
- **Qapsule'de yeri:** Çekirdek gramer (yargı = rol + simge) ve kabuk.
- **Bizde ne değişiyor:** Simge kümesi kapsül sonucu ve köken için de tanımlanır.
- **Sınama:** Gri tonlamalı ekran görüntüsünde bütün yargılar okunuyor.

---

## E. Boş, yükleme, hata durumları

### VV-E1. Boş durum: ne olduğunu söyle, tek eylem ver
- **Kalıp:** valuecell boş strateji listesi: kesikli çerçeve, yuvarlak zeminde soluk ikon,
  kalın tek satır başlık, tek cümle açıklama, altında birincil "oluştur" düğmesi. Vibe:
  pano çalışma türüne uymuyorsa boş çizgiler yerine neden boş olduğunu ve doğru sekmeyi
  söyleyen kutu (C4); karşılama ekranında kategorili istem kütüphanesi; ölçülmemiş sayaç hiç
  çizilmez (B3); run card'da boş alan "kayıt yok" yazar (C3).
- **Kaynak:** `VC/frontend/src/app/agent/components/strategy-items/trade-strategy-group.tsx:300-326`;
  `VT/frontend/src/components/charts/StrategyResearchDashboard.tsx:211-224`,
  `VT/frontend/src/components/chat/WelcomeScreen.tsx`.
- **Lisans hükmü:** Apache / MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** "Bozuk mu, boş mu, farklı mı" ayrımı yapılır; kullanıcı ne
  yapacağını bilir.
- **Qapsule'de yeri:** Kabuk; grafikte "veri yok" durumu çekirdek sahnesinde (boş tuval değil).
- **Bizde ne değişiyor:** Boş kapsül listesinde eylem "ajandan iddia iste" ya da "çizimden
  kapsül kur"; grafikte boş durum nedenli (sembol yok / aralıkta bar yok / veri kaynağı kapalı).
- **Sınama:** Her ana yüzeyin üç boş nedeni için ayrı metin; boş tuval altın görselde yok.

### VV-E2. Yükleme: son yerleşimin iskeleti
- **Kalıp:** İskelet (skeleton) son içeriğin geometrisini taklit eder: izleme listesi satırı
  (sembol, ad, mini grafik, fiyat), metrik ızgarası (etiket çizgisi + değer çizgisi), mini
  sermaye eğrisi iskeleti. valuecell'in sohbet "AI düşünüyor…" üç zıplayan nokta göstergesi
  daha zayıf (ne yaptığı belli değil; A1 bunu çözer).
- **Kaynak:** `VC/frontend/src/components/valuecell/skeleton/sparkline-stock-list-skeleton.tsx`,
  `VT/frontend/src/components/common/Skeleton.tsx`, `VT/frontend/src/components/chat/RunCompleteCard.tsx`
  (eğri iskeleti); `VC/frontend/src/app/agent/components/chat-conversation/chat-streaming-indicator.tsx`.
- **Lisans hükmü:** Apache / MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Yükleme bitince yerleşim zıplamaz.
- **Qapsule'de yeri:** Kabuk; grafik için çekirdek bir "yükleniyor" sahnesi (eksenler çizili,
  veri yok).
- **Bizde ne değişiyor:** Grafik iskeleti gerçek eksen geometrisini kullanır (çekirdekten).
- **Sınama:** Yükleme → veri geçişinde düzen kayması (CLS) ≈ 0.

### VV-E3. Hata: satır içi, sınırlı, nedenli
- **Kalıp:** Bileşen düzeyinde hata sınırı: tüm sayfa yerine yalnız o bölüm, ince yıkıcı-ton
  çerçeveli kutuda ikon + hata iletisi. Çalışma kartında uyarılar ayrı kutu (C3). Bilinmeyen
  bileşen ham içerikle görünür (A3).
- **Kaynak:** `VT/frontend/src/components/common/ErrorBoundary.tsx`; `VT/frontend/src/pages/RunDetail.tsx:497-507`;
  `VC/frontend/src/components/valuecell/renderer/unknown-renderer.tsx`.
- **Lisans hükmü:** MIT / Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Bir panelin hatası çalışma alanını düşürmez.
- **Qapsule'de yeri:** Kabuk; protokol hataları (`path`, `expected`, `hint`) kullanıcıya
  sadeleştirilmiş, ajana tam.
- **Bizde ne değişiyor:** Ajan komutu reddedildiğinde ajan panelinde red satırı + nedeni; belge
  değişmez (hata = değişiklik yok).
- **Sınama:** Hatalı op sonrası belge sürümü aynı, panelde red satırı görünür.

---

## F. İzleme listesi, uyarı, arama, mobil

### VV-F1. İzleme listesi (sağ sütun)
- **Kalıp:** Sağ sütunda "İzleme listem": satır başına solda sembol (kalın) + şirket adı
  (soluk, altında), sağda fiyat + değişim yüzdesi (yön renginde, altında); listenin dibinde
  "Hisse ekle". Arama modalında her sonuç satırında durum taşıyan düğme: *İzle* → (dönen
  ikonla) *İzleniyor* → *İzlendi* (pasif). Üstte endeks kartları: ad (soluk, büyük harf),
  değer (büyük), değişim, sağda alan dolgulu mini çizgi.
- **Kaynak:** `VC/frontend/src/app/home/components/stock-list.tsx`,
  `VC/frontend/src/app/home/components/stock-search-modal.tsx:40-80`,
  `VC/frontend/src/components/valuecell/charts/sparkline.tsx`; görsel `VC/assets/product/homepage.png`.
  Vibe-Trading'de izleme listesi **yok**.
- **Lisans hükmü:** Apache → kalıp uyarlanır.
- **Neden iyi çalışıyor:** Klasik iki sütunlu satır taranabilir; eylem durumunun düğmede
  görünmesi çift ekleme ve belirsizliği önler.
- **Qapsule'de yeri:** Kabuk paneli; mini grafik çekirdekten (aynı görünüş kaydı).
- **Bizde ne değişiyor:** Satır yoğun (≈28-32 px), tabular rakam; satırda o sembole bağlı açık
  kapsül/izleme sayısı rozeti (ajan dikkati); valuecell'in 100+ px satır yüksekliği alınmaz.
- **Sınama:** 320 px panelde en az 15 satır; fiyat sütunu hizalı.

### VV-F2. Uyarı / sinyal listesi — doğrudan yok; en yakın: ajan itme kartı
- **Kalıp:** Ana sayfada zamanlanmış ajanın kartı: ajan ikonu + adı, sağda bir sonraki çalışma
  ("8 saat içinde"), altında zaman damgalı son çıktılar (iki satıra kırpılmış). Vibe'de
  zamanlanmış araştırma listesi ve öneri kartı var (A5), ama fiyat/çizim uyarısı listesi yok.
- **Kaynak:** `VC/frontend/src/app/home/components/agent-task-cards.tsx`; görsel
  `VC/assets/product/homepage.png` (orta); `VT/frontend/src/pages/Scheduled.tsx`.
- **Lisans hükmü:** Apache / MIT → kalıp uyarlanır.
- **Neden iyi çalışıyor:** "Bir sonraki ne zaman" bilgisi nöbetçinin canlı olduğunu söyler.
- **Qapsule'de yeri:** Kabuk paneli (dikkat/uyarı listesi), kaynak çekirdek dikkat kuyruğu.
- **Bizde ne değişiyor:** Uyarı satırı = tetiklenen izleme/niyet: zaman (bar zamanı), sembol,
  kısa olay, köken; tıklayınca grafikte tetik noktası. Liste profesyonel uyarı listelerine
  (OpenChart) göre yoğun kurulur; valuecell'in uzun metin kartları değil.
- **Sınama:** Tetiklenen her uyarı grafikte bir noktaya gidiyor; sonraki değerlendirme zamanı
  görünür.

### VV-F3. Komut paleti — yok
- Vibe-Trading'de komut paleti bulunamadı (Esc ile kapanan menüler dışında kısayol yok,
  `VT/frontend/src/components/layout/Layout.tsx:49, 396`). valuecell'de cmdk bileşeni kurulu
  ama yalnız çoklu seçimde kullanılıyor (`VC/frontend/src/components/valuecell/multi-select.tsx`);
  en yakın şey 300 ms gecikmeli sembol arama modalı (F1). Kaynak olarak OpenChart'a bakılmalı.

### VV-F4. Mobil yerleşim — yok
- Vibe-Trading: 768 px altında kenar çubuğu 48 px ikon rayına iner, metinler gizlenir
  (`VT/frontend/src/components/layout/Layout.tsx:141-183`); telefon için ayrı yerleşim yok.
  valuecell: Tauri masaüstü uygulaması; yalnız shadcn kenar çubuğunun 768 px kırılımı ve sol
  sohbet listesi çekmecesi (`VC/frontend/src/hooks/use-mobile.ts`,
  `VC/frontend/src/components/valuecell/app/app-conversation-sheet.tsx`). Ç3/Ç4 için bu iki
  projeden başvuru yok.

---

## Başvuru ekranları

Vibe-Trading'de gerçek arayüz görseli yok (illüstrasyonlar başvuru olamaz):
`VT/assets/feature-*.png` 1024×1024 (çizgi film), `VT/wiki/assets/alpha-library-og.png`
1200×630 (tanıtım). `VT/assets/Frontend.mp4` izlenmedi.

| Yol | Piksel | Ne gösteriyor | Kullanım |
|---|---|---|---|
| `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/assets/product/AutoTradingAgent.png` | 2826×1722 | 3 sütun: strateji kartları · karar döngüsü sicili (gerekçe + işlem çipi) · portföy değeri çizgisi + pozisyon tablosu; açık tema | **Ç1 kısmi** (yan panel + sicil düzeni; mum grafiği ve ajan sohbeti yok). C1/C2 için en iyi başvuru. |
| `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/assets/product/homepage.png` | 2796×1532 | endeks kartları + mini grafik, ajan itme kartı, sağda izleme listesi, altta sohbet girişi | Ç1 için sağ izleme sütunu (F1, F2); yoğunluk açısından **karşı örnek**. |
| `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/assets/product/NewPushAgent.png` | 3022×1656 | sohbet solda, ajan raporu sağ ayrıntı bölmesinde; düz metin onay + zamanlanmış görev kartı | Ç1 ajan paneli ↔ ayrıntı bölmesi ayrımı (A4); onay için karşı örnek (A5). |
| `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/assets/product/superagent.png` | 2824×1614 | iç içe alt ajan kartı ("Research Agent · View"), iki katlı araç satırı, atıflı cevap | Ajan paneli içeriği (A2, A4). |
| `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/assets/product/agent_market.png` | 2818×1710 | ajan pazarı (açılmadı) | A6 karşı örnek. |
| `/Users/arkns/Works/Mchart/refs/src/ValueCell-ai__valuecell/assets/product/Model_Configuration.png` | 2818×1710 | model ayarları (açılmadı) | — |

- **Ç2 (grafik yakın plan):** iki projede de yok. valuecell fiyat grafiği TradingView gömme
  widget'ı (görseli yok); Vibe ECharts varsayılanları (görseli yok). OpenChart/Vela'ya bakılmalı.
- **Ç3/Ç4 (telefon):** yok.
- Bütün valuecell görselleri açık tema, retina, düşük yoğunluk; "profesyonel" yargısı için
  OpenChart görselleriyle yan yana kullanılmalı, tek başına değil.
