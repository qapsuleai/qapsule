# Qapsule için değişiklik önerileri

Bu dosya Qapsule deposuna **uygulanmamış** önerileri tutar (karar X1). İki bölüm:
kuzey yıldızına eklenecekler ve yol haritasında yapılacak değişiklikler. Kaynak: RX1
raporları ve RX2 kararları (`docs/rounds/RX2/DECISIONS.md`). Uygulama zamanı Arkin'de.

Durum sütunu: **kabul** (Arkin onayladı, uygulanmayı bekliyor) · **öneri** (konuşulacak).

## A. Kuzey yıldızına eklenecekler

| # | Ek | Durum |
|---|----|-------|
| E1 | **AI-native'in üç basamağı.** (1) *Ajan anlatır:* motorun bulduğunu okur, yorumlar, haber verir. (2) *Ajan yönlendirir:* neyin hesaplanacağını, neyin hangi eşikle izleneceğini ajan kurar; motor çalıştırır ve kanıtlar. (3) *Ajan kurar:* kime neyin hangi biçimde gösterileceğini seçer. Her basamakta hesap deterministik motorda kalır; yapay zekâ karar verir ve yönlendirir. | kabul (X2) |
| E2 | **Profesyonelin oyun alanı.** Profesyonel kullanıcı kendi çalışma alanını bırakmaz: çizimini kendi yapar, göstergesini kendi koyar, yerleşimini kendi kurar. Sistem bu alanı elinden almaz, üstüne üç şey ekler: (a) *Çizim bir niyettir:* insanın çizdiği çizgi, bölge, seviye yalnız görsel değil; ajan için okunur bir beklenti ifadesidir. (b) *Ajan yorumlar:* kullanıcının çizimlerini ve beklentilerini okur, ne beklendiğini söyler, eksik ya da çelişen yeri gösterir. (c) *İzleme devredilir:* çizimden doğan koşul deterministik bir izleme görevine dönüşür ve arkadaki ajanlara iletilir; eşleşme motorda, haber ajandan. İnsanın çizimi insanındır: ajan ona onaysız dokunmaz; ajanın eklediği her şey kökeniyle ayrı durur. | kabul (X3) |
| E6 | **Ajan sistemi Qapsule'ün içindedir.** İzleme, yorum ve haber verme ajanları Qapsule'ün kendi parçasıdır; dış bir ajan altyapısına bağımlı değildir, ama sözleşmesi taşınabilirdir. | kabul (X6) |
| E7 | **Karne paylaşılır.** Bir stratejinin ya da koşulun benzer koşullardaki tutma sonucu, ileriye bakmadan tutulmuş ve mühürlü kayıt olarak toplulukla paylaşılabilir. | kabul (X25–X28): canlı ve yeniden oynatma kaydı ayrı; sıralama yalnız canlı |
| E8 | **Sessizce yanlış kurmaz: kur ve göster.** Ajan en olası okumayı kurar ve hemen çizimin üstünde gösterir; tek dokunuşla değiştirilir ya da kaldırılır; koşul cümlesini motor yazar. Aynı kalıp düzeltmesiz kabul edildikçe güven kazanılır ve sorulmaz. Yalnız okumalar yön olarak ayrışıyor ve kazanılmış varsayılan yoksa sorar. Notsuz, cümlesiz, pozisyonsuz çizimde hiçbir şey kurmaz: sessiz zil, üstüne gelince açıklama. | kabul (X10; RX3 M1 ve insan testi) |
| E9 | **Geri bildirim kanalı hep açıktır.** Sistem ne yaptığını (ajan neyi okudu, neyi seçti) ve izlemenin sağlığını (izleniyor mu, tetiğe ne kadar kaldı) sürekli gösterir; "yeşil sürekli kazanılır". | kabul (X10, X13) |
| E10 | **Kapsam küreseldir.** Sistem tek ülkeye göre tasarlanmaz; aracı kurum bağlantıları (Alpaca, IBKR vb.) ve yurt dışı yayın hesaba katılır; ülkeye özgü varsayım tasarıma gömülmez. | kabul (X11) |
| E11 | **Tespit zaman içinde yaşar.** Grafikte geri-ileri gidildiğinde o aralığın tespitleri (gerçekleşen, gerçekleşmeyen) ve sembolün o tür tespitte tutma yüzdesi görünür; kaynaklar (haber vb.) üstüne bağlanabilir. | kabul (X12) |
| E12 | **Ortak iş bir kez yapılır.** Herkes için aynı olan analiz (ör. bilanço hazırlığı) bir kez üretilir, kullanıcılar üye olur; üye sayısı da bir değerdir. | kabul (X20) |
| E13 | **Sinyal ile tavsiye ayrıdır.** "Al / sat / tut" bir göstergenin ya da stratejinin sinyali olarak, kaynağı ve karnesiyle söylenir; ajan kendi tavsiyesini vermez. "Yatırım tavsiyesi motoru değiliz" cümlesi bu ayrımla netleşir. | kabul (X21) |
| E14 | **Tutma ihtimali takılabilir bir hesaptır.** Bir fikrin tutma ihtimali, birden çok gösterge ve periyotla tanımlanan durumun geçmişte ölçülmüş tutma sıklığıdır; kanıtına açılır. "Tahmin takılabilir sağlayıcıdır; merkez değildir" cümlesiyle uyumlu. | kabul (X18); tasarım açık |
| E2 inceltme | Çizimden doğan koşul kendi fiyatını taşımaz, çizime başvurur; eşleşme yalnız kesin barda; çizim değişince okuma aynıysa modelsiz yeniden kurulur. Çizimin adı, notu ve yapay zekâ özeti şemada kayıtlıdır (X22). | kabul (X13, X22) |
| E5 inceltme | Soru 7'ye ek: "…ve ajan sessizce yanlış kurmuyor mu?" | öneri |
| E15 | **Ajanın yazdığı her hesap tipli ara gösterime iner; motor koşar ve kanıtlar; sinyal doğumda sonuç kuralı taşır.** Basamak 2'nin somut biçimi (E4'ü kesinleştirir). | kabul (X30; RX4) |
| E16 | **Yapı da kabul ölçütüdür.** Kod düzeni, katman sınırları, parametre ve profil yönetimi kuralla ve makine denetimiyle korunur. | kabul (X31) |
| E3 | **Alışılmış grafik dili alfabe sayılır.** Sağ fiyat ekseni, OHLCV lejantı, son değer etiketi, imleç ve tarih hapı, altta hacim: bunlar "TradingView kopyası" değil, alışkın kullanıcının alfabesidir. Farkımız bunların üstüne gelen anlam katmanıdır. "Ne değiliz" bölümü buna göre netleşir. | öneri |
| E4 | **İlke 4'ün somut hali:** "Hesap, ajanın düşünme aracıdır" ancak ajan hesabı *yazabiliyorsa* gerçektir. Ajan (ve profesyonel insan) aynı ifade yüzeyinden hesap ve koşul tanımlar; motor çalıştırır, maliyeti önceden bilir, her değeri kökenine bağlar. | öneri (biçimi RX3 ölçümüyle) |
| E5 | **AI-native testine iki soru:** (6) Ajan neyin hesaplanacağını ve izleneceğini kurabiliyor mu? (7) İnsanın çizimi ajan için okunur bir niyet mi? | öneri |

## B. Yol haritasında yapılacak değişiklikler

| # | Değişiklik | Dayanak | Durum |
|---|------------|---------|-------|
| Y1 | **R5B olduğu gibi koşar** (basamak 1 + niyet). Tek not, round başlatılırken bir cümle olarak verilebilir: *önerinin onayı ve reddi yalnız insan kökenli komutla olur; ajan MCP üzerinden kendi önerisini onaylayamaz* (testi olsun). | `TRANSPLANT.md` T4; dsh-trading dersi (onay aracı ajana açılınca kalıp çöktü) | kabul (X4); not öneri |
| Y2 | **İç içe gösterge protokolden yazılabilir olsun** (K63 adresi bugün yalnız testlerden çağrılıyor). Küçük iş; R5B içinde ya da hemen sonrasında. | `QAPSULE-GAP.md`, `LANGUAGE.md` §4 | öneri |
| Y3 | **RD2'nin ölçütleri genişler:** `VISUAL.md` §5 P1–P15; karşılaştırma aynı DPR ve aynı veriyle; alışılmış grafik öğeleri; renk yalnız veride; telefon ve masaüstü için ayrı görünüş profili (13 px / 44 px eşikleri yalnız telefonda); "profesyonel ve güvenilir" ölçütü düzeneğe girer. | `VISUAL.md` §3, §5 | öneri |
| Y4 | **R7 "Profesyonelin oyun alanı" olarak yeniden tanımlanır ve öne çekilir.** Kapsam: çizim nesneleri (şema: veri uzayında çapa + köken + olgu bağı); insanın çizim araçları; çizim → deterministik koşul → olay → tekilleştirilmiş ajan tetiklemesi; ajanın çizimi ve beklentiyi yorumlaması; izleme görevinin arka plan ajanına devri; çalışma alanı kalıpları (panel, bağlama grubu, komut çubuğu) otoritedeki dokümanın parçası olarak. | X3; `TRANSPLANT.md` T3, T7, T8, T11; OpenChart `server/alert/drawing-alerts.ts` | öneri; sıra X7: beklentiler → RD2 → R7 |
| Y5 | **Yeni round: ajan yazımı hesap** (basamak 2). Biçim (metin dil / veri akışı ara gösterimi + JSON / Pine alt kümesi) RX3 deneyiyle seçilir; sonra Qapsule round'u olarak girer. | `LANGUAGE.md` §6–§7; `TRANSPLANT.md` T9, T10 | öneri |
| Y6 | **Basamak 3 (ajan kurar)** kişiselleştirme round'una (R9) bağlanır: kişiye göre kapsül seçimi ve görünüm kurma. | E1 | öneri |
| Y7 | **Küçük sağlamlaştırmalar** (uygun round'a eklenir): araç sınırında saat ve kimlik otoriteden gelir (T5); karne denetim izi, hash zinciri (T12); doğrulayıcıda ret + tek onarım turu ölçümü (T2; R5B M30 bunu zaten ölçüyor). | `TRANSPLANT.md` | öneri |

| Y8 | **Oyun alanı beklentileri şartnamesi RD2'den önce yazılır** (bu kanalda, RX3): profesyonel ne yapar, yapay zekâ onunla ne yapar, izleme görevi ajana nasıl devredilir. RD2 bunu girdi alır. | X7 | kabul |
| Y9 | **Geriye dönük test ve topluluk ayrı bir iş kolu** olarak yol haritasına girer: Matriks veri kanallarıyla strateji sınaması; sonuçların (tutma oranı, koşul) toplulukta paylaşımı; karne ile bağı. Önce ayrı tasarım konuşması. | X8; `OUTLOOK.md` §8 | kabul (kapsam), tasarım açık |
| Y10 | **Ajan yüzeyi dışa da açılır:** Qapsule MCP'si yalnız kendi ajanlarımız için değil, kullanıcının kendi asistanı (Claude, ChatGPT vb.) için de birinci sınıf yüzeydir; kapsül başka asistanların içinde görüntülenebilir (MCP Apps / A2UI). | `OUTLOOK.md` §1, §6 | öneri |
| Y11 | **Çizim yorumcusu: deterministik varsayılanlar + cümleyi okuyan küçük model; motor denetler.** Varsayılanlar: koruyucu izleme (X16), role göre kaynak (kırılımda kapanış; stop, hedef, bölgeye gelişte fitil), tekrar bir kez. Kural tek başına birincil yol değil (yeniden yazılmış cümlelerde kırılgan). Kur-ve-göster sözleşmesiyle aynı düzenekte yeniden ölçülür (tek senaryo = tek çağrı). | RX3 M1; `HUMAN/RESULTS.md` | kabul |
| Y12 | **Ajan sistemi çalışma modeli: karma.** Olayla uyanan durumsuz çağrı + paylaşılabilir işte toplu çağrı; deterministik yönlendirici; model tik/bar başına çağrılmaz; nöbetçinin varsayılan haber cümlesi modelsiz şablon. | RX3 M2 | kabul |
| Y13 | **Bildirim süzgeci varsayılanı:** kişinin elindekiler + önem + tekrar bastırma; önemli olay muafiyeti bekleme süresinden önce. R5B M31 ile tek süzgeç. | RX3 M3 | öneri |
| Y14 | **Çizim → uyarı modeli OpenChart kalıbıyla** (temiz oda, Rust): kural çizime başvurur; kesin bar; olay olgularıyla kayıt; olay ↔ eylem ayrı ve tekilleştirilmiş; sağlık göstergesi; yanında ajan koşu olayları akışı (ne yaptığını gösteren). | X13, X10; OpenChart `docs/architecture/alert-trigger.md` | kabul |
| Y15 | **İki aşamalı kırılım izleme:** kırıldı → tutundu / geri döndü; yön açık; sonuç karneye yazılır. Koşul şemasına sonuç izleme eklenir. | X14 | kabul |
| Y16 | **Periyot, izleme koşulunun birinci sınıf alanıdır.** Ara periyot seçimi (günlük çizimde 2/4 saatlik, günlükte teyit); ajan periyodu her zaman yazar; bayatlık bar sayısıyla değil zamanla. | X17, X12; insan testi H11, H18 | kabul |
| Y17 | **Tutma ihtimali hesabı:** çok göstergeli, çok periyotlu durum tanımı → geçmiş tutma sıklığı; kalibrasyon ölçümü ("%75" gerçekten dörtte üç mü); izleme süresince güncellenir. İleride sinyal basamağı (eşik aşılırsa pozisyon aç) onay basamaklarının en üstünde ve aracı kurum bağlantısına bağlı. Bağımlı: karne, Y9 (geriye dönük test), RX4 (ajan yazımı hesap). | X18 | kabul (kapsam); tasarım açık |
| Y18 | **Hazırlık görevi:** tarihi ya da aralığı bilinen olaylarda (bilanço, genel kurul, temettü, faiz kararı…) alarm yerine olay öncesi dosya: beklentiler, spekülasyonlar, dönem içi bildirimler, haberler; olay yaklaştıkça güncellenir. | X19 | kabul |
| Y19 | **Ortak analiz ve üyelik:** paylaşılabilir iş tek kez üretilir; kullanıcı var olan analize üye olur; üye sayacı görünür; kişiye özel katman (pozisyonu, çizimi) ayrı. Maliyet modeli buna göre güncellenir (RX3 maliyetinin büyük kısmı paylaşılabilir işlerde). | X20 | kabul |
| Y20 | **Kullanıcı parametreleri ve kullanım hakları temel yapı olarak baştan kurulur:** soru adedi, yapay zekâ kullanımı, kontör; üyelik ve ek paketlerle satılabilir; profilde ayarlanabilir; bütün varsayılanlar (ör. tutunma = 3 bar, günde 3 soru) parametredir. | X23 | kabul |
| Y21 | **Zaman gezintisi katmanı:** geçmişe gidildiğinde o aralığın tespitleri, sonuçları, sembol × tespit türü tutma yüzdesi ve haber katmanı (OpenChart "Chart Explain" kalıbı: aralık seç → olgu özeti deterministik → açıklama kaynaklı). Kullanıcının kendi çizim izlemeleri de karneye girer. | X12 | kabul |
| Y22 | **Uyar ve öner** (koşul kurulamıyor ya da anlamsız: zaten doğru, çok uzak, periyot uyuşmuyor, çelişen çizim) ve **koruyucu izleme varsayılanı** ajan sözleşmesine girer. | X15, X16 | kabul |
| Y23 | **Küresel açılım:** aracı kurum bağlantıları (Alpaca, IBKR), çoklu piyasa ve veri sağlayıcı, çoklu düzenleme; emir yolu onay basamaklarının üstünde. Sıra ve kapsam ayrıca konuşulacak. | X11 | kabul (yön); sıra açık |
| Y24 | **Sinyal dili:** kapsül ve ajan dil taramaları "al/sat"ı bugün tümden reddediyor; kaynağı adlandırılmış sinyal ("X stratejisi SAT sinyali verdi") için ayrım tanımlanmalı. Sinyal üreten strateji gelene kadar R5B'yi etkilemez. | X21 | öneri |
| Y25 | **Sicilin genişlemesi:** kapsül karnesi tek sicile dönüşür; iddia kaynakları kapsül, çizim izlemesi, strateji sinyali, tutma ihtimali beyanı; her iddia sonuç kuralını doğumda taşır; canlı ↔ yeniden oynatma etiketi; as-of sorgu. Tasarım `docs/design/SICIL.md`. | X25, X26 | kabul |
| Y26 | **Zaman aralığı okumasının altyapısı:** as-of sicil, olay dizini (haber, bildirim, bilanço, rapor; zaman × sembol), deterministik aralık özeti; model yalnız açıklama ve kaynak eşleştirmede. | X28, X12 | kabul |
| Y27 | **Paylaşım katmanı:** ortak analiz nesnesi (sürüm, üyelik, üye sayacı; önce yalnız sistem yayımlar), sonra topluluk (tanım + canlı sicil), sinyal aboneliği; rızaya dayalı kimliksiz toplu istatistik. Düzenleme ülke bazında incelenecek. | X20, X27, X28 | kabul (yön); sıra açık |
| Y28 | **Dil round'u (R5B'den sonra):** ara gösterim + `add_program` / `check_program` + metin dil ayrıştırıcısı + MCP aracı; R7 izlemeleri ara gösterim programı olarak koşar; sicil `signal` kaydını programdan alır. Şartname taslağı `reports/COMPUTE-LANGUAGE.md` §8; kararlar X30. Y5'in yerini alır. | RX4; X30 | kabul |
| Y29 | **Geliştirme kuralları Qapsule'e girer:** `docs/ENGINEERING.md`, `CLAUDE.md`'de işaret, depo geneli `arch-check` `verify.sh`'ta; her round'un GOAL'ünde yapı ölçütü; denetçi yapıyı da denetler. Kaynak `docs/design/ENGINEERING-RULES.md`. | X31 | kabul |
| Y30 | **Yönetişim turu (R5B'den hemen sonra, yeni özellik round'undan önce):** kurallar ve `arch-check` Qapsule'e girer (Y29) **ve** mevcut kod yeniden düzenlenir; davranış değişmez (altınlar, özetler, uygunluk dökümleri bayt eşit). Borç listesi ENGINEERING-RULES §11; parametre kayıt defterinin iskeleti (Y31) bu turda açılır. Paket ana akışta, R5B raporu görüldükten sonra yazılır. | X31, X32; ölçüm 2026-11-21 | kabul |
| Y31 | **Parametre kayıt defteri ve profil yapısı** (Y20'yi kesinleştirir): tek kayıt defteri, kapsam ve çözümleme sırası, haklar ve kullanım kaydı, profilin ayrı bölümleri (kimlik, plan/haklar, lisanslar, tercihler, kazanılmış varsayılanlar, rızalar, kişisel nesneler). ENGINEERING-RULES §6–§7. | X23, X31 | kabul |

Yol haritasına **girmeyen** (karar bekliyor): ajanın emir göndermesi (R5B kapsam dışı sayıyor).

## C. RX3 için aday deneyler (bu kanalda, Qapsule'e dokunmadan)

| # | Deney | Soru |
|---|-------|------|
| D1 | Tek ara gösterim, üç yüzey (metin / JSON / Pine alt kümesi) | Ajan hangisiyle doğru hesabı ilk denemede yazıyor; köken ve maliyet sınırı tutuyor mu? |
| D2 | Çizimden niyete | Kullanıcının çizimi (trend çizgisi, bölge, seviye) verildiğinde ajan beklentiyi doğru yorumlayıp deterministik izleme koşuluna çevirebiliyor mu; yanlış ve çift tetik 0 mı? |
| D0 | Oyun alanı beklentileri şartnamesi (belge işi; D2 ve D3'ün girdisi, RD2'nin girdisi) | Profesyonel hangi araçları, çizimleri, yerleşimi bekler; her birinde yapay zekânın rolü ne? |
| D3 | İzleme görevinin devri | Çizimden doğan görev arka plan ajanına nasıl bir sözleşmeyle iletilir (şema, yaşam döngüsü, bildirim süzgeci)? |
| D4 | ~~Çizim yorumcusunun yeniden ölçümü~~ atlandı (X24); ilgili Qapsule round'unda zaten ölçülecek | — |
| D5 | Tutma ihtimali: durum tanımı + geçmiş sıklık + kalibrasyon (geriye dönük test iş koluyla birlikte) | Söylenen ihtimal gerçekleşen sıklıkla örtüşüyor mu? |

## D. RX3'ten gelen küçük ekler (uygun round'a)

- Y1 ek: karar yalnız insan yazarla; özet kilitli onay; onay yüzeyindeki koşul cümlesini motor kurar.
- Y3 ek: RD2 girdisi `reports/PLAYGROUND.md` §5 (Y01–Y30 yüzey) + sık kullanılan netleştirme yüzeyi, motor varsayılanlarının ayrı gösterimi, "koşul zaten doğru" durumu; sessiz zil ve üstüne gelince açıklama.
- Y4 ek: `Drawing`, `WatchCondition`, `WatchTask` şema taslağı ve eşleştirici davranış başvurusu `experiments/rx3/`.
- Y10 notu: dış asistan da aynı olay zarfını ve onay kuralını kullanır.
- R5B'ye not: gömülü talimat sınamasına çizim/kullanıcı notu sınıfı; süzgeçte "elindekiler" kolu.
- Qapsule ARCHITECTURE "ajan ortamı ayrı sistemdir" cümlesi X6 ile gergin; netleştirilmeli.
