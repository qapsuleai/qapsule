# RI1 — Keşif: Matriks Bağlantısı ve Quantex Servisleri

## Neden

R6 Qapsule'ü gerçek canlı veriye bağlayacak. Bunun nasıl yapılacağı bugün başka bir
depoda, çalışan kodda yazılı: `quantex`. Orada Matriks'e nasıl belirteç alındığı, MQTT
kanallarına nasıl bağlanıldığı, hangi istek için hangi protonun kullanıldığı ve verinin
içeride NATS ile nasıl yeniden yayınlandığı kurulu. Ayrıca gerçek zamanlı gösterge ve
anomali hesaplayan C++ servisleri var (K5: bunlar ortak hesap katmanıdır, grafikten ayrıdır).

R6 paketini tahminle değil, bu kodun gerçekte ne yaptığını bilerek yazmak istiyoruz (K59).
Bu round **yalnızca okur ve belgeler**. Qapsule koduna dokunmaz.

## İncelenecek yerler (hepsi salt okunur)

Kök: `/Users/arkns/LocalGit/quantex/services/`

| Ne | Klasör |
|---|---|
| Matriks bağlayıcısı (belirteç, MQTT, proto eşlemesi; içeride NATS'a geçit) | `quantex-stream/matriks-adapter` |
| Fiyat göstergeleri (C++) | `quantex_indicators_price_cpp` |
| Akış göstergeleri (C++) | `quantex_indicators_flow_cpp` |
| Derinlik göstergeleri (C++) | `quantex_indicators_depth_cpp` |
| Anomali motoru | `quantex-anomaly-engine` |
| NATS bağlayıcıları | `quantex-stream/price-indicator-adapter`, `flow-indicator-adapter`, `depth-indicator-adapter`, `anomaly-adapter` |

Gerekirse bunların bağımlı olduğu ortak kitaplıklar, proto klasörleri ve dağıtım
tanımları da okunur; depo kökünün dışına çıkılmaz.

## Arkin'in verdiği bilgiler (kodla doğrulanacak)

1. Belirteç alma yöntemi sisteme özeldir: Quantex için üretilen belirteç yalnız o sistemin
   kimliğini kurar.
2. MQTT tarafında **yayın tipi başına bir bağlantı** alınır; abonelikler o bağlantı
   üzerinden yapılır. Aynı uç noktaya ikinci bağlantı isteği birincisini düşürür.
   "Düşme çoklu bağlantıdan kaynaklanıyorsa yanlış bir şey yapıyoruz demektir."
3. Mesaj içerikleri protolardadır. Mesajlar tipine göre **ilk seferde en geniş haliyle**,
   sonra **güncelleme** olarak gelir (K23 ile uyumlu: alan düzeyinde güncelleme).
4. **Borsanın açık olup olmadığı** zaman damgasıyla birlikte gelir; bu bağlantı hem
   Türkiye saatini hem borsa açıklık durumunu verir.
5. Gösterge servislerine gösterge ve çapraz gösterge aboneliğiyle dinleme yapılabilir;
   anomali motoru piyasa anomalilerini yakalar; hepsi NATS üzerinden yayınlar.

## Kesin sınırlar

- **Salt okunur.** `quantex` deposuna hiçbir şey yazılmaz; orada derleme, test, betik
  çalıştırılmaz (derleme çıktı üretir). Yalnız okuma ve arama.
- **Canlı bağlantı yok.** Bu round'da Matriks'e, MQTT'ye, NATS'a ya da herhangi bir iç
  servise ağ bağlantısı kurulmaz; belirteç istenmez. Gerekçe: aynı kimlikle ikinci bağlantı
  çalışan sistemin bağlantısını düşürebilir.
- **Gizli bilgi okunmaz, taşınmaz.** `.env*`, anahtar, sertifika, belirteç, parola
  dosyaları açılmaz. Kodun içinde gömülü bir gizli değer görülürse değeri hiçbir yere
  yazılmaz; yalnız "şu dosyada gömülü gizli değer var" diye yeri bildirilir.
- **Kod ve proto kopyalanmaz.** Qapsule deposuna `quantex`'ten dosya, proto tanımı ya da
  uzun kod parçası alınmaz. Belgeler kendi cümleleriyle anlatır; kanıt olarak dosya yolu
  ve satır numarası verir; gerekirse birkaç satırlık alıntı (alan adı, konu adı kalıbı).
- **Gerçek piyasa verisi taşınmaz.** Depoda kayıtlı örnek mesaj ya da veri varsa içeriği
  kopyalanmaz; yalnız yapısı anlatılır.
- İç adresler (sunucu adı, IP, port) belgede genelleştirilir ("MQTT uç noktası, yapılandırmadan");
  tam değerleri yazılmaz.
- Varsayım ile kanıt ayrılır: her iddia ya bir kaynak satırına dayanır ya da açıkça
  "varsayım" / "bilinmiyor" diye işaretlenir.

## Cevaplanacak sorular

### A. Bağlantı ve kimlik
- Belirteç nasıl alınıyor (uç nokta türü, istek biçimi, kimliğin neye bağlı olduğu),
  ömrü, yenilenmesi, süresi dolunca davranış?
- MQTT: kaç bağlantı, hangi yayın tipi için; istemci kimliği nasıl kuruluyor; "ikinci
  bağlantı birincisini düşürür" kuralı kodda nasıl görünüyor (istemci kimliği çakışması mı,
  sunucu kuralı mı); yeniden bağlanma, geri çekilme, oturum sürekliliği, hizmet kalitesi düzeyi.
- Qapsule ayrı bir sistem olarak bağlanırsa ne gerekir: ayrı kimlik mi, ayrı belirteç
  yöntemi mi? Bir kullanıcının kendi oturumu ile bir sunucu servisi aynı anda bağlanabilir mi?
- Abonelik modeli: konu adı kalıpları, sembol başına mı toplu mu, abonelik sınırları.

### B. Mesajlar
- Proto kataloğu: hangi istek ve yayın tipi için hangi mesaj; alanlar ve anlamları
  (fiyat, hacim, derinlik, işlem, bar, takas, haber, kurumsal akış …).
- İlk durum (en geniş hali) ile güncelleme nasıl ayrılıyor; eksik alan "değişmedi" mi demek;
  sıra numarası, boşluk algılama, yeniden eşitleme var mı?
- Zaman: hangi alan hangi zamanı taşıyor (borsa zamanı, yayın zamanı); saat dilimi; borsa
  açıklık durumu mesajı ve durumları (açılış seansı, sürekli işlem, kapanış, tatil …).
- Sembol evreni ve üst veri (pazar, para birimi, fiyat adımı, lot, seans takvimi) nereden geliyor?
- Bar verisi akıştan mı üretiliyor, hazır mı geliyor; geçmiş için REST ile ilişkisi
  (`docs/DATA_ACCESS.md`).
- Hangi veri hangi **lisans sınıfına** giriyor (K33: gösterimli / gösterimsiz, AKDE,
  kurumsal akış, derinlik düzeyleri)? Kodda yetki denetimi nerede yapılıyor?

### C. İç NATS yayını
- Konu adları ve hiyerarşisi; yük biçimi (proto mu, JSON mu); kalıcılık ve yeniden oynatma
  (JetStream ya da eşdeğeri) var mı; geç katılan tüketici ilk durumu nasıl alıyor?
- Bağlayıcının geçit olarak yaptığı dönüşümler: birleştirme, normalleştirme, zenginleştirme.

### D. Gösterge servisleri (fiyat, akış, derinlik)
- Hangi göstergeler; parametreleri; hangi periyotlarda; hangi semboller için; ısınma ve
  ilk değer davranışı; bar kapanışında mı her tikte mi yayın.
- Abonelik: gösterge ve çapraz gösterge aboneliği nasıl isteniyor (istek biçimi, konu);
  isteğe bağlı parametreyle hesap yapılabiliyor mu, yoksa sabit bir küme mi?
- Sayısal tanımlar Qapsule'ün `qp-compute` tanımlarıyla aynı mı (ör. RSI yumuşatması,
  EMA başlangıcı, Bollinger sapma tanımı, `f64` mü)? K13/K14 (bar hizası, borsa takvimi)
  ile uyum. Farklar tablosu; "aynı ad, farklı sonuç" riski.
- Performans ve ölçek: kodda ya da belgede yazılı sınırlar, sembol sayısı, gecikme.

### E. Anomali motoru
- Hangi anomali türleri; her birinin kuralı, eşiği, penceresi, girdileri; yaşam döngüsü
  (başladı, sürüyor, bitti) var mı; güven ya da şiddet değeri; tekrar bastırma.
- Yük biçimi ve konu adları; geçmişe dönük kayıt tutuluyor mu (karne için gerekir, K42).
- Deterministik mi (aynı girdi aynı çıktı; K3)? Dil modeli ya da rastgelelik var mı?
- Qapsule'e eşleme: her anomali türü hangi **anlam türüne** (devam, dönüş, kırılım,
  sıkışma, aşırılık, akış; K30) ve hangi kanıt ailesine düşer; Qapsule'ün yedi dedektörüyle
  örtüşen, tamamlayan ve çelişenler; "sunucu olgusu" (K33, K38) olarak taşınabilirliği:
  kanıtına açılabiliyor mu (K34), mühür için ne gerekir?

### F. Qapsule için anlamı
- Kavram eşlemesi: `SeriesSource`, veri bağı (K23), veri düzlemi / anlam düzlemi ayrımı,
  sunucu olgusu, otorite oturumu, görüntüleyici (K48), yeniden oynatma (K44) bu yapıda
  neye karşılık gelir; karşılığı olmayanlar.
- **R6 için seçenekler**, artıları ve eksileriyle (karar verilmez):
  (a) Qapsule doğrudan Matriks MQTT'ye kendi kimliğiyle bağlanır;
  (b) Qapsule Quantex'in iç NATS yayınını tüketir;
  (c) karma (ör. ham veri bir yoldan, gösterge ve anomali öteki yoldan);
  (d) incelemenin gösterdiği başka bir yol.
  Her seçenek için: kimlik ve çoklu bağlantı riski, lisans denetiminin yeri, gecikme,
  işletme bağımlılığı, geliştirme ortamında nasıl denenir (çalışan sistemi etkilemeden),
  kayıt ve yeniden oynatma olanağı.
- Tespit nerede yapılmalı (K29, K48): Qapsule dedektörleri sunucuda mı koşar, anomali
  motoru mu genişler, ikisi birlikte mi? Seçenekler ve sonuçları.
- R6 için önerilen kapsam taslağı ve Arkin'e / Matriks ekiplerine sorular listesi.

## Çıktılar

Hepsi Qapsule deposunda, `docs/matriks/` altında (yeni klasör):

| Dosya | İçerik |
|---|---|
| `CONNECTIVITY.md` | A |
| `MESSAGES.md` | B (proto kataloğu tablo olarak; alan anlamları) |
| `NATS.md` | C |
| `INDICATORS.md` | D (gösterge listesi ve `qp-compute` ile farklar tablosu) |
| `ANOMALY.md` | E (tür tablosu ve Qapsule eşlemesi) |
| `INTEGRATION.md` | F (kavram eşlemesi, R6 seçenekleri, riskler) |

Ve `rounds/RI1/REPORT.md` (özet, cevaplanamayan sorular, Arkin'e sorular, R6 kapsam
taslağı), `rounds/RI1/AUDIT.md`.

## Kabul ölçütleri

| # | Ölçüt | Kanıt |
|---|-------|-------|
| 1 | Altı belge yazıldı; A–F'deki her soru ya kaynak satırıyla cevaplandı ya da "bilinmiyor / koddan anlaşılmıyor" diye işaretlendi ve kime sorulacağı yazıldı. | ✅ Altı belge `docs/matriks/`; A–F soruları her belgenin bölümlerinde, cevapsızlar "bilinmiyor + kime" ile her belgenin "Açık sorular" tablosunda (CONNECTIVITY §7: 13, MESSAGES §9: 14, NATS §10: 8, INDICATORS: 23, ANOMALY §8: 9). Denetçinin kapsam tablosu: eksik soru yok (`AUDIT.md` "Kapsam tablosu"). |
| 2 | Her olgusal iddia dosya yolu ve satır numarasıyla; varsayımlar ayrı işaretli. Arkin'in beş bilgisi kodla karşılaştırıldı: doğrulandı / kısmen / kodda görünmüyor. | ✅ Kanıtlar `services/...:satır` biçiminde; "Varsayım:" ayrı. Beş bilgi: `INTEGRATION.md` §2 (1 kısmen, 2 doğrulandı, 3 kısmen, 4 kısmen, 5 kısmen), ayrıntı CONNECTIVITY §6, MESSAGES §8, NATS §9, INDICATORS, ANOMALY §6. |
| 3 | Proto kataloğu: mesaj → kullanım (hangi istek ya da yayın) → ilk durum mu güncelleme mi → önemli alanlar. Lisans sınıfı bilinen her veri türü için yazıldı; bilinmeyenler listelendi. | ✅ `MESSAGES.md` §1 (22 proto: mesaj → konu/uç nokta → ilk durum/güncelleme → çözen servis → alanlar), §2 alan tabloları, §7 lisans sınıfları ve §7.3 bilinmeyenler; Qapsule eşlemesi `INTEGRATION.md` §6. |
| 4 | Gösterge listesi ve `qp-compute` ile sayısal tanım farkları tablosu (en az Qapsule'ün beş göstergesi için: SMA, EMA, RSI, MACD, Bollinger). | ✅ `INDICATORS.md` Özet (kısa fark tablosu) ve T4a §3 (SMA, EMA, RSI, MACD, Bollinger, ATR, Donchian; formül, dosya:satır iki tarafta, risk) + girdi farkları; akış ve derinlik için Qapsule'de karşılık yok (T4b §4, T4c §5). |
| 5 | Anomali türleri tablosu ve altı anlam türüne eşleme; Qapsule dedektörleriyle örtüşme; kanıta açılabilirlik ve determinizm değerlendirmesi. | ✅ `ANOMALY.md` §2 (tür tablosu), §5.1 (tür → anlam türü, aile, lisans, dedektör ilişkisi, taşınabilirlik), §5.2–5.3, §4 (determinizm: hayır; kanıta açılma: kısmen). |
| 6 | R6 seçenekleri karşılaştırmalı tabloyla; çalışan sistemi etkilemeden deneme yolu her seçenek için yazılı; karar verilmedi, öneri gerekçeli. | ✅ `INTEGRATION.md` §4: (a), (b), (c), (d1–d3) her biri artı/eksi/risk/lisans/gecikme/işletme/deneme yolu/kayıt; karşılaştırma tablosu; öneri "karar değil" diye gerekçeli. (d2) deneme yolu denetimden sonra eklendi. |
| 7 | Sınırlara uyuldu: `quantex` deposunda değişiklik yok (`git status` önce ve sonra aynı); ağ bağlantısı kurulmadı; belgelerde gizli değer, tam iç adres, kopyalanmış proto ya da gerçek veri yok (denetçi tarar). Gömülü gizli değer bulunduysa yalnız yeri bildirildi. | ✅ quantex `git status --porcelain` önce/sonra aynı (`experiments/quantex_status_before.txt`; ana akış ve denetçi karşılaştırdı); ağ bağlantısı kurulmadı (yalnız dosya okuma); tarama 0 bulgu (`AUDIT.md` "Sınır taraması", düzeltme sonrası yeniden). Gömülü gizli değer yerleri yalnız yerle `REPORT.md` ve `CONNECTIVITY.md` Ek'te. |
| 8 | Bağımsız denetçi alt ajan belgelerden rastgele en az 40 iddiayı kaynağında doğruladı; yanlış çıkanlar düzeltildi; sonuç `AUDIT.md`'de. | ✅ Bağımsız denetçi 54 iddia: 48 doğru, 6 kısmen, 0 yanlış; altısı düzeltildi; koşullar karşılandı (`AUDIT.md` "Düzeltmeler"). |
| 9 | `REPORT.md`, `STATUS.md`, `ROADMAP.md` güncel; Qapsule kodunda ve altınlarda değişiklik yok; `scripts/verify.sh` çalıştırmaya gerek yok (kod değişmedi), ama gizli bilgi taraması temiz. | ✅ `REPORT.md`, `STATUS.md`, `ROADMAP.md` güncel; Qapsule'de kod/altın değişmedi (`git diff --stat` yalnız `docs/matriks/`, `rounds/RI1/`, `STATUS.md`, `ROADMAP.md`); gizli tarama temiz. |
