# Qapsule için anlamı: kavram eşlemesi, R6 seçenekleri, riskler (RI1 / T6)

Durum: RI1 keşfi, 2026-11-19. Bu belge karar vermez; seçenekleri ve gerekçeli öneriyi
yazar. Karar R6 paketi tasarlanırken ana akış oturumunda verilir (CLAUDE.md "Round
kapanışı" 5).

Dayandığı belgeler (her iddianın kaynak satırı orada): `CONNECTIVITY.md` (A),
`MESSAGES.md` (B), `NATS.md` (C), `INDICATORS.md` (D), `ANOMALY.md` (E). Bu belgede
olgusal iddialar ya o belgelerin bölümüne ya da doğrudan `services/...:satır` biçiminde
`quantex` kaynağına bağlanır; Qapsule tarafı `docs/...` ya da `crates/...` ile.
"Varsayım:" ve "Öneri:" ayrı işaretlidir.

---

## 1. Beş cümlede durum

1. **Ham veri tek kaynaktan, Matriks MQTT'den gelir** ve uç nokta başına bir bağlantıyla
   alınır; aynı kimlik ile aynı uç noktaya ikinci bağlantı birincisini düşürür, bu bir
   sunucu kuralıdır (`CONNECTIVITY.md` §2.4; `services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:5-7`).
   Belirteç servis hesabı + `hardwareId` ile alınır; `hardwareId` varsayılan olarak pod adından
   gelir, yani pod başına ayrıdır — ortak ortam dosyasında sabitlenmişse paylaşılabilir, bu
   bilinmiyor (`CONNECTIVITY.md` §1.2, §2.5, soru 8).
2. **Matriks mesajlarında sıra numarası yok, QoS 0**; "önce en geniş hal, sonra değişen
   alanlar" yalnız market (`mx/symbol`, `mx/derivative`) için geçerli; işlemde ilk durum yok,
   emir defterinde her ileti tam tablo, derinlikte ilk tablo + satır işlemi (`MESSAGES.md` §3.1).
3. **Quantex'in iç NATS yayını kalıcı değil** (JetStream yazılmıyor), iç şemaya çevrilmiş ve
   ölçekli tam sayılı; `seq` boşluğu kayıp anlamına gelmiyor; borsa açıklığı iç yayına
   geçmiyor (`NATS.md` §0, §2.3, §2.5).
4. **Gösterge servisleri Qapsule'ün hesap tanımlarına referans olamaz:** formüllerin
   çoğu aynı, ama bar hizası UTC epoch'a (K14'e aykırı), ısınmada 0 (K13'e aykırı), 300 bar
   tavanı, `bandwidth` ×100, tik zamanı varış anı (`INDICATORS.md` Özet, T4a §3–§4).
5. **Anomali motoru deterministik değil ve kanıt taşımıyor;** çıktısı olduğu gibi "sunucu
   olgusu" sayılamaz; ama kuralları (özellikle PGC_ACCEL, PGC_DIVERGENCE, AGENT_SHIFT,
   BLOCK_TRADE, OFI) Qapsule dedektörü olarak deterministik biçimde yeniden yazılabilir
   (`ANOMALY.md` §4, §7).

---

## 2. Arkin'in beş bilgisi: hüküm tablosu

| # | Bilgi | Hüküm | Kanıt |
|---|---|---|---|
| 1 | Belirteç alma yöntemi sisteme özel; Quantex belirteci yalnız o sistemin kimliğini kurar | **Kısmen.** Belirteç servis hesabı (HTTP Basic) + `typeCode` + `hardwareId` ile alınıyor. Varsayım: kimlik (`sub` = `QTX-<hardwareId>`) `hardwareId`'ye, yani **örneğe (pod'a)** bağlı — tek bir kod yorumuna dayanıyor (`CONNECTIVITY.md` §1.2). Hesabın ya da `typeCode`'un sisteme kayıtlı olup olmadığı koddan anlaşılmıyor. | `CONNECTIVITY.md` §1.1–1.2, §6; `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:73-81`; `services/quantex_indicators_price_cpp/src/services/token_service.cpp:180-190` |
| 2 | Yayın tipi başına bir bağlantı; abonelikler onun üzerinden; ikinci bağlantı birincisini düşürür | **Doğrulandı.** Bağlantılar uç nokta yoluna göre (`/market`, `/trade`, `/depth`, `/news`…); kural kodda yorum olarak yazılı ve Matriks `DISCONNECT`/`MULTICONNECT` bildirimi tanınıyor; istemci kimliği çakışması değil. Matriks belgesi de "aynı belirteçle aynı adrese ikinci bağlantı ilkini sonlandırır" diyor (`CONNECTIVITY.md` §2.4 madde 0). Uyarı: Quantex'in kendisinde "yanlış bir şey" olabilecek iki yer (paylaşılan belirteç, ortak `hardwareId` olasılığı). | `CONNECTIVITY.md` §2.4–2.5; `userPool.ts:5-7`; `services/quantex-anomaly-engine/src/main.cpp:1792-1793` |
| 3 | Mesajlar önce en geniş haliyle, sonra güncelleme olarak gelir | **Kısmen.** Market iletileri için doğru (Matriks belgesi); işlem iletisinde ilk durum yok, emir defteri her seferinde tam, derinlik ilk tablo + satır işlemi. Protoda ilk durum / güncelleme bayrağı yok (`publishReason` var, kullanılmıyor). | `MESSAGES.md` §3.1, §8 |
| 4 | Borsa açıklığı zaman damgasıyla gelir; bu bağlantı Türkiye saatini ve açıklık durumunu verir | **Kısmen.** `mx/timestamp` (`TimeMessage`): Unix epoch µs + `isBistPPOpen` + `isBistPPHoliday`, yalnız `source = MX` iken anlamlı, yalnız Pay Piyasası. Zaman UTC epoch'tur, Türkiye saati +3 ile türetilir. Seans evresi bu iletide değil, sembol başına `stockStatus` kodlarında. İç NATS yayınına geçmiyor. | `MESSAGES.md` §2.7, §4.2; `CONNECTIVITY.md` §5; `NATS.md` §2.5 |
| 5 | Gösterge servislerine gösterge ve çapraz gösterge aboneliğiyle dinlenir; anomali motoru anomali yakalar; hepsi NATS'tan yayınlar | **Kısmen.** Gösterge aboneliği NATS `{price,flow,depth}.cmd.*` ile var; ama motorlar NATS'a değil iç Mosquitto'ya yazar, bağlayıcılar köprüler. "Çapraz gösterge" adıyla ayrı abonelik yok: iç içe gösterge ve sinyal ifadeleri var. Anomali NATS'ta, ama abonelik yok (bütün semboller için sürekli) ve tür başına değil sembol başına birleşik olay. | `NATS.md` §5, §6, §9; `INDICATORS.md` T4a "Arkin'in 5. bilgisi"; `ANOMALY.md` §6 |

---

## 3. Kavram eşlemesi (Qapsule ↔ Quantex/Matriks)

| Qapsule kavramı (`docs/CONCEPTS.md`) | Bu yapıda karşılığı | Boşluk / not |
|---|---|---|
| Seri kaynağı `SeriesSource` / veri sağlayıcı `SourceProvider` | Matriks MQTT market iletisi (`SymbolMessage`, fiyat kartı) + Matriks REST bar uçları (geçmiş) | Akışta **bar yok** (`MESSAGES.md` §6). Canlı barı Qapsule kendisi üretmeli (K14 hizasıyla); Quantex'in bar üreticisi K14'e aykırı (`INDICATORS.md` T4a §4). |
| Veri olayı `DataEvent` (`bar`/`patch`, `seq`) | Market iletisi alan düzeyinde güncelleme | Matriks `seq` vermez; PROTOCOL §18.4 zaten "kaynak numara taşımıyorsa barındırıcı yerel, kesintisiz numara üretir" diyor (`docs/PROTOCOL.md` §18.4). Çözücü **alan varlığını korumalı** (proto2 `has_`): eksik alan "değişmedi"dir, 0 değil (`MESSAGES.md` §3.2). |
| Bağ durumu `stale` | Bağlantı kopması, yeniden bağlanma (temiz oturum: abonelikler yeniden kurulur), Matriks `MULTICONNECT` bildirimi | Kopukluk sonrası yeniden oynatma yok; yeniden çözüm REST'ten ("kesin bar çekme") yapılmalı. NATS `seq` boşluğu kayıp işareti değildir (`NATS.md` §2.3). |
| Veri düzlemi / anlam düzlemi (K23) | Matriks protobuf akışı veri düzlemidir; NATS da veri düzlemidir | Uyumlu: Qapsule protokolüne bar girmez, kaynaktan `SourceProvider` ile girer. |
| Veri yetkisi (entitlement) | Matriks belirtecinin lisans kapsamı; aracı lisanssız aboneliği SUBACK 128 ile, REST 403 ile reddeder | Quantex kodunda veri sınıfı denetimi **yok**; geçitteki kapsamlar Quantex ürün kapsamı, K33 lisans sınıfı değil (`MESSAGES.md` §7.2; `NATS.md` §7). "Gösterimli / gösterimsiz" ayrımı hiçbir yerde yok. Kişi başına yetki ancak kullanıcı yolu (`matriks-user-adapter`) ile mümkün görünüyor. |
| Lisans sınıfı `public`/`licensed` | Belgede: anlık/gecikmeli, KD1 / PD1P-PD2-PD2P / AKDE işlem düzeyleri, derinlik 10/25 kademe, AKDE (PGC, kurum bazlı), takas | Eşleme tablosu §6'da. |
| Sunucu olgusu (K33, K38) ve mühür | PGC (AKD/AKDE) kökenli olgular; anomali motorunun akış türleri | Anomali motorunun çıktısı mühür için gereken iki şeyi vermiyor: bar başına ham işaretli büyüklük ve deterministik yeniden üretim (`ANOMALY.md` §5 başı). |
| Otorite oturumu (`qp-session`) | Karşılığı yok. Quantex'te belge/otorite kavramı yok; en yakın şey geçidin abone başına durum tutması | Qapsule otoritesi sunucuda ayrı bir süreç olarak koşmalı. |
| Görüntüleyici (K48) | Quantex geçidi (WebSocket) istemciye hazır veri iter | Cihaz Matriks'e **doğrudan bağlanmamalı**: kullanıcının kendi Matriks uygulamasıyla aynı kimlik ve uç noktada çakışma olasılığı açık soru (`CONNECTIVITY.md` §3.2, soru 7). Açık enstrümanın verisi barındırıcıdan gelmeli. |
| Olgu dedektörü (K29: sunucuda, bütün semboller) | Anomali motoru (25 anomali dedektörü + rejim, sembol başına birleşik olay) | Farklı model: Qapsule olgusu tipli, bar hizalı, deterministik; motor olayı ağırlıklı karışım, saniye ölçekli. Doğrudan yerine geçmez (`ANOMALY.md` §5.1 son satır). |
| Karne (K42) | `anomaly_outcomes` (ClickHouse, 180 gün, saniye pencereli) | Yalnız dış karşılaştırma; tanım farklı (`ANOMALY.md` §3.3, §7). |
| Yeniden oynatma (K44) | Yok: NATS Core, kalıcılık yok; Quantex yeniden oynatma sunmaz | Qapsule'ün kendi kaydı gerekir: ham Matriks iletilerini `fixtures/local/` altına kaydedip `qp-source::replay` ile oynatmak. |
| Takvim (`qp-calendar`, K14) | Matriks REST seans saatleri ve tatil uçları; `mx/timestamp` açıklık; sembol başına `stockStatus` | Kaynak var; Quantex bunları kısmen ve duvar saatiyle kullanıyor. Qapsule `qp-calendar` tablosunu bu uçlarla karşılaştırabilir. |
| Fiyat adımı (A8 ile ilgili) | Canlı `priceStep` alanı; derinlik servisinde sabit tablo | İki kaynak birbiriyle ve Matriks belgesiyle çelişiyor (`MESSAGES.md` §5; `INDICATORS.md` T4c §4). Qapsule canlı alanı esas almalı (Öneri). |
| Kanıt ailesi `EvidenceFamily` | Derinlik/emir defteri | **Qapsule'de aile yok.** Derinlik kökenli olgu yazılacaksa önce sözlüğe karar (`ANOMALY.md` §5, soru 4). |
| Hesap `Study` (sunucu yaygın olanı yayınlar, çekirdek gerisini üretir; ARCHITECTURE "Hesap nerede yapılır") | Quantex gösterge servisleri | "Bir değer hem akıştan hem yerel hesaptan geliyorsa tanım tektir ve test ile karşılaştırılır" kuralı bu servislerle sağlanamaz (`INDICATORS.md` Özet). |

**Karşılığı olmayanlar:** otorite ve günlük, doküman ve komut yolu, anlam türü, işaret,
kapsül yaşam döngüsü, kanıt başvurusu ve veri özeti, yansı/görüntüleyici tutarlılık
kuralları. Bunlar Qapsule'ün kendi katmanıdır; Quantex'te eşi yoktur.

---

## 4. R6 için seçenekler

Ortak ön koşul (her seçenekte): Matriks ile **yeni bir kimlik** (ayrı `hardwareId`, tercihen
ayrı servis hesabı ya da `typeCode`) — Quantex'in kimliğiyle bağlanmak çalışan sistemi
düşürebilir (`CONNECTIVITY.md` §3). Kim verir: Matriks veri ekibi + Arkin.

### (a) Qapsule doğrudan Matriks MQTT'ye kendi kimliğiyle bağlanır

Qapsule barındırıcısı (sunucu) kendi belirteciyle `market` (+ `mx/timestamp`), gerekirse
`trade`, `depth` uç noktalarına birer bağlantı açar; protobuf'u kendisi çözer; canlı barı
K14'e göre kendisi üretir; geçmişi REST'ten alır (`docs/DATA_ACCESS.md`).

- **Artı:** tek bağımlılık Matriks; alan düzeyinde güncelleme (K23) ve alan varlığı korunur;
  borsa zamanı ve `stockStatus` doğrudan elde; bar hizası, sıra numarası, `stale` kuralları
  tamamen Qapsule'ün; Quantex'teki çözme kusurları (eksik alanın 0 olması, `MESSAGES.md` §3.2)
  devralınmaz. Lisans denetimi Matriks aracısında (SUBACK 128), kimliğin kapsamı belli.
- **Eksi:** Protobuf çözücü, MQTT/WebSocket istemcisi, belirteç yenileme, yeniden bağlanma,
  konu kuralları (`@lvl2`, sembol tipine göre `symbol`/`derivative`) Qapsule'de yeniden
  yazılır. Gösterge ve anomali servislerinden yararlanılmaz.
- **Risk:** kimlik yanlışlıkla Quantex'inkiyle aynı olursa çalışan sistem düşer; sembol
  sayısı arttıkça abonelik sınırı bilinmiyor (`CONNECTIVITY.md` soru 5); belirteç süresi
  dolunca açık bağlantının davranışı bilinmiyor (soru 4).
- **Lisans denetiminin yeri:** Matriks (kimliğin kapsamı) + Qapsule otoritesi (okuyucuya
  göre izdüşüm, K33). Kişi başına yetki ayrıca çözülmeli (kişinin kendi Matriks yetkisi).
- **Gecikme:** en kısa yol (Matriks → Qapsule).
- **İşletme bağımlılığı:** yalnız Matriks.
- **Çalışan sistemi etkilemeden deneme:** (1) önce Matriks'in **test / gecikmeli** uç noktası
  ve **ayrı test kimliği** (`CONNECTIVITY.md` soru 13); (2) tek uç nokta, birkaç sembol;
  (3) bağlantı öncesi kimliğin Quantex pod adlarından farklı olduğu kodla denetlenir;
  (4) `MULTICONNECT` bildirimi gelirse bağlantı hemen kapanır ve yeniden denenmez.
- **Kayıt ve yeniden oynatma:** ham iletiler (bayt + alış zamanı) `fixtures/local/` altına
  kaydedilir; aynı çözücü kaydı oynatır → K44 ile tam uyum, aynı girdi aynı çıktı.

### (b) Qapsule Quantex'in iç NATS yayınını tüketir

Qapsule barındırıcısı NATS'a salt dinleyici olarak bağlanır: `market.BIST.*` (fiyat kartı),
`anomaly.BIST.*`, isterse `price|flow|depth.BIST.*.data.*`.

- **Artı:** Matriks'e yeni bağlantı yok (çoklu bağlantı riski sıfır); çözme ve abonelik
  Quantex'te; anomali ve gösterge sonuçları hazır.
- **Eksi:** Kalıcılık yok (yeniden oynatma Qapsule'ün kendi kaydına kalır); `seq` boşluk
  algılamaya yaramaz; Matriks'in göndermediği alanlar 0 olarak delta'ya girebiliyor
  (`NATS.md` §2.4); borsa açıklığı iç yayında yok (§2.5); ölçekli tam sayıdan `f64`'e çevirme;
  ilk durum istek/yanıtla ve o istek salt okuma değil (Matriks aboneliği ekler, herkese
  yayın üretir: `NATS.md` §3.3) — kullanılırsa çalışan sistemi etkiler. Gösterge değerleri
  K13/K14'e aykırı (`INDICATORS.md`), anomali deterministik değil.
- **Risk:** NATS'ta hesap/izin tanımı depoda yok (`NATS.md` §7): Qapsule yanlış konuya
  yazarsa (`*.cmd.*`, `matriks.control.u.*`) motorlara iş yükler ya da kullanıcı
  bağlantısına sembol ekletir. Geçidin kuyruk grubu adı kullanılırsa geçidin mesajları
  bölüşülür (`NATS.md` §8). Quantex'in iç şeması değişirse Qapsule kırılır.
- **Lisans denetiminin yeri:** yok (Quantex'te veri sınıfı denetimi yok); hepsi Qapsule
  otoritesine kalır, ama verinin hangi kimliğin lisansıyla alındığı Quantex'in kimliğidir —
  Qapsule kullanıcılarına bu veriyi göstermenin lisans durumu **bilinmiyor** (Arkin + Matriks).
- **Gecikme:** Matriks → matriks-adapter → NATS: bir atlama fazla; ölçüm yok.
- **İşletme bağımlılığı:** Quantex kümesi, NATS, matriks-adapter ve dört bağlayıcı.
- **Çalışan sistemi etkilemeden deneme:** ayrı NATS kullanıcısı ve yalnız okuma izni
  (`market.>`, `anomaly.>`) istenir (`NATS.md` soru 2); kuyruk grubu kullanılmaz;
  `streamx.symbol.refresh` ve `*.cmd.*` hiç çağrılmaz; yalnız dinlenen semboller
  kullanılır. Daha güvenlisi: Quantex'in yerel yığını (`services/quantex-stream/docker/`)
  ayrı bir makinede, kendi test kimliğiyle — ama bu (a)'nın kimlik ihtiyacını geri getirir.
- **Kayıt ve yeniden oynatma:** NATS iletileri kaydedilip oynatılabilir; ama girdi Quantex'in
  dönüştürülmüş şemasıdır, Matriks'in kendisi değil.

### (c) Karma: ham veri Matriks'ten (a), gösterge ve anomali Quantex NATS'tan (b)

- **Artı:** Qapsule'ün hesap ve dedektörleri kendi verisiyle deterministik kalır; anomali
  olayları **bağlam** olarak (önem, "ayrıca motor şunu gördü") eklenebilir.
- **Eksi:** iki bağımlılık; iki kimlik (Matriks ve NATS) gerekir; anomali olayları Qapsule
  olgusu olamayacağı için (`ANOMALY.md` §7) kapsül iddiasına giremez, yalnız ek bilgi olur.
- **Risk:** (a) ve (b)'nin riskleri toplamı; aynı sembolün iki kaynaktan gelen zamanlarının
  hizası (anomali µs/ms tutarsızlığı, `ANOMALY.md` soru 7).
- **Çalışan sistemi etkilemeden deneme:** (a) ve (b)'nin yolları birlikte; NATS tarafı
  yalnız `anomaly.>` dinleme.
- **Kayıt:** ham Matriks kaydı (a) belirleyici; anomali kaydı ayrı ve belirleyici değil.

### (d) İncelemenin gösterdiği başka yollar

- **(d1) Kayıt-önce (R6'nın ilk yarısı).** Canlı gösterim yerine önce (a)'nın bağlantısını
  yalnız **kayıt** için kurmak: bir seans boyunca birkaç sembolün ham iletileri
  `fixtures/local/` altına; Qapsule geliştirmesi kayıt üzerinde (K44). Canlı yol kayıtla
  aynı çözücüyü kullanır. Artı: geliştirme piyasa saatine ve canlı bağlantıya bağlı değil;
  bağlantı süresi ve sembol sayısı küçük tutulur. Eksi: canlı davranış (yeniden bağlanma,
  belirteç yenileme) ayrıca sınanmalı.
  **Çalışan sistemi etkilemeden deneme:** (a)'nın yolunun en dar hali: ayrı test kimliği,
  test ya da gecikmeli uç nokta varsa o; yalnız `market` uç noktası ve 3–5 sembol; tek seans;
  bağlantıyı açan betik kimliğin Quantex'inkinden farklı olduğunu önce denetler ve
  `MULTICONNECT` iletisinde kapanır; kayıt bittikten sonra bağlantı kalıcı açık tutulmaz.
- **(d2) Kullanıcı yolu (`matriks-user-adapter` modeli).** Her kullanıcının kendi Matriks
  belirteciyle, sunucuda kullanıcı başına bağlantı; veri `matriks.u.<id>.*` gibi aynen
  geçer (`CONNECTIVITY.md` §3.1; `NATS.md` §1.5). Artı: K33'teki "kişinin kendi veri
  yetkisi" doğrudan Matriks tarafından uygulanır. Eksi: kullanıcı sayısı kadar bağlantı;
  kullanıcının kendi uygulamasıyla çakışma bilinmiyor; belirteci kim alıyor (mqlaw, ayrı
  depo) belirsiz. K48 ile birlikte düşünülmeli: açık kapsülün verisi kişinin yetkisiyle.
  **Çalışan sistemi etkilemeden deneme:** bugün güvenli bir yol **bilinmiyor**: kullanıcı
  adına bağlantı, o kullanıcının kendi Matriks uygulamasını düşürebilir (`CONNECTIVITY.md`
  §3.2). Önce Matriks veri ekibine sorulur (soru 7); olumluysa yalnız bir test kullanıcısıyla
  (Arkin'in kendi hesabı değil), kendi uygulaması kapalıyken, tek uç nokta ve birkaç sembol.
  Kime: Matriks veri ekibi + Arkin (mqlaw sahibi için Quantex ekibi).
- **(d3) Quantex içinde Qapsule'e ayrılmış bağlayıcı.** Quantex ekibi, mevcut
  `matriks-adapter`'e ham baytları aynen geçiren bir konu (kullanıcı yolundaki gibi
  `mx-format` başlıklı) ekler; Qapsule yalnız onu dinler. Artı: tek Matriks bağlantısı
  korunur, ham veri bozulmaz. Eksi: Quantex ekibine iş; kalıcılık yine yok; lisans soru
  işareti (b) ile aynı.
  **Çalışan sistemi etkilemeden deneme:** yeni konu Quantex'in test ortamında (ya da yerel
  yığında, `services/quantex-stream/docker/`) ve ayrı bir test kimliğiyle açılır; üretimde
  yalnız mevcut bağlantıya ek konu yayını olarak, yeni Matriks bağlantısı açmadan; Qapsule
  yalnız okuma izinli NATS kullanıcısıyla dinler. Kime: Quantex ekibi.

### Karşılaştırma tablosu

| | (a) Doğrudan Matriks | (b) Quantex NATS | (c) Karma | (d1) Kayıt-önce | (d2) Kullanıcı yolu | (d3) Ham geçit |
|---|---|---|---|---|---|---|
| Çoklu bağlantı riski | Orta (yeni kimlikle düşük) | Yok | Orta | Düşük (kısa, küçük) | Yüksek (kullanıcı uygulamasıyla) | Yok |
| Ham veri doğruluğu (alan varlığı) | Tam | Bozulabilir (0 yazma) | Tam | Tam | Tam | Tam |
| Determinizm / yeniden oynatma | Tam (kendi kaydı) | Kısmi | Ham için tam | Tam | Tam | Tam (kendi kaydı) |
| Lisans denetimi | Matriks + Qapsule otoritesi | Bilinmiyor | (a) gibi | (a) gibi | Kişi başına, Matriks | Bilinmiyor |
| Gecikme | En düşük | +1 atlama | (a) + bağlam | — | Düşük | +1 atlama |
| Geliştirme yükü | Yüksek | Orta | Yüksek | Orta (a'nın alt kümesi) | Yüksek | Düşük (Qapsule), orta (Quantex) |
| İşletme bağımlılığı | Matriks | Quantex kümesi | İkisi | Matriks | Matriks + belirteç sağlayıcı | Quantex kümesi |
| Deneme yolu (çalışan sistemi etkilemeden) | Test/gecikmeli uç + ayrı kimlik | Ayrı salt okuma NATS kullanıcısı, `refresh`/`cmd` yok | İkisi | Ayrı kimlik, tek uç nokta, birkaç sembol, kısa kayıt | Bilinmiyor; önce Matriks'e sorulur, sonra yalnız test kullanıcısı | Quantex test ortamı ya da yerel yığın + ayrı kimlik |

### Öneri (karar değil)

**(d1) → (a), anomaliler için (c)'nin bağlam kolu sonraya.** Gerekçe:

1. Qapsule'ün değişmezleri (aynı girdi aynı çıktı, K13/K14 bar hizası, alan düzeyinde veri
   olayı ve `stale`) ham Matriks iletisiyle sağlanabiliyor; Quantex'in iç yayını bunları
   bozuyor (eksik alan 0, UTC hiza, varış zamanı) — `MESSAGES.md` §3.2, `INDICATORS.md` T4a §4.
2. K44: kayıt-önce yaklaşımı canlı bağlantı süresini küçük tutar ve geliştirmeyi piyasa
   saatinden ayırır; R4'ten beri kurulu yeniden oynatma yolu aynen kullanılır.
3. Çoklu bağlantı riski ayrı kimlikle yönetilebilir; Quantex NATS yolu ise lisans durumu
   bilinmeyen (Quantex kimliğiyle alınmış) veriyi başka kullanıcılara göstermek demek.
4. Anomali motorunun kuralları değerli ama çıktısı olgu değil; önce Qapsule dedektörü olarak
   deterministik yeniden yazılması (PGC_ACCEL → `flow.institutional`'ın genişlemesi gibi)
   daha uygun. NATS'tan anomali okumak yalnız "bağlam" olarak ve sonraki bir round'da.

Bu önerinin ön koşulu Matriks'ten ayrı kimlik (ve mümkünse test uç noktası). O gelmezse
(b) salt okuma NATS kullanıcısıyla yalnız `market.>` dinleyerek ikinci en iyi yoldur; o
durumda çözücü `0` değerlerine karşı savunmalı yazılmalı ve lisans sorusu önce cevaplanmalı.

---

## 5. Tespit nerede yapılmalı (K29, K48)

| Seçenek | Ne demek | Sonuç |
|---|---|---|
| T1. Qapsule dedektörleri sunucuda, bütün semboller | Qapsule otoritesi (barındırıcı) her kesinleşen barda yedi dedektörü koşar; cihaz yalnız açık enstrümanı sınar (K48) | Bugünkü mimariyle aynı; veri yolu (a)/(d1). Sembol sayısı (BIST ~500+ hisse; Varsayım) × periyot sunucu yükü R6'da ölçülmeli. |
| T2. Anomali motoru genişler, Qapsule onu tüketir | Qapsule olguları Quantex motorunda üretilir | K3 ve K34'e aykırı (motor deterministik değil, kanıt taşımıyor; `ANOMALY.md` §4). Önerilmez. |
| T3. İkisi birlikte: Qapsule dedektörleri olgu, motor olayları bağlam | Motor olayları kapsülün önemine ya da "başka kaynak da gördü" ek bilgisine girer, iddiaya girmez | Uygulanabilir; ama "teyit = farklı kanıt ailesi" kuralına (K30) motorun birleşik puanı karışmamalı. |
| T4. Motor kurallarını Qapsule'e taşımak | PGC_ACCEL, PGC_DIVERGENCE, AGENT_SHIFT, BLOCK_TRADE, OFI gibi kurallar bar hizalı, deterministik Qapsule dedektörü olarak yazılır | Akış ailesini genişletir; lisanslı olanlar sunucu olgusu ve mühürle (K33, K38). Derinlik kökenli olanlar için önce kanıt ailesi kararı. |

Öneri: T1 + T4 (R6 sonrası, R6'da yalnız T1). T3 yalnız (c) seçilirse.

---

## 6. Lisans sınıfı eşlemesi (bilinen ve bilinmeyen)

| Veri | Matriks sınıfı (belge/koddan) | Qapsule `LicenseClass` önerisi | Kaynak |
|---|---|---|---|
| Fiyat kartı (son fiyat, gün içi yüksek/düşük, hacim) | anlık (`rt`) ya da gecikmeli (`dl`); KD1'de gecikmeli adet/hacim | `public` (kullanıcının kendi bar yetkisiyle) | `MESSAGES.md` §7.1 |
| İşlem tiki (alıcı/satıcısız) | KD1 → lvl0 … | Varsayım: `public` ama yansıda yok (K48) → pratikte sunucu | `MESSAGES.md` §7.1; `ANOMALY.md` §5 |
| İşlem tiki (alıcı/satıcı kurumlu) | AKDE → lvl2 | `licensed` | `MESSAGES.md` §7.1 |
| Derinlik | ilk 10 / kalan 15 kademe | `licensed` (kademe düzeyine göre ayrı yetki adı) | `MESSAGES.md` §7.1 |
| PGC (kurumsal para akışı) | AKDE | `licensed` → sunucu olgusu, mühür | `MESSAGES.md` §7.1; `INDICATORS.md` T4b §5 |
| Takas | takas yetkisi | `licensed` | `MESSAGES.md` §7.1 |
| Haber | AINEWS (belge ile kod yorumu çelişiyor) | bilinmiyor | `MESSAGES.md` §7.1, §7.3 |
| "Gösterimsiz kullanımdan yorum" hakkı | **hiçbir yerde yok** | K33'ün temeli; bilinmiyor | `MESSAGES.md` Özet 9 |

Bilinmeyenler ve kime: `MESSAGES.md` §7.3 ve §9; özellikle gösterimsiz kullanım hakkı
(Matriks lisans / hukuk + Arkin).

---

## 7. Riskler (R6 için)

| # | Risk | Etki | Azaltma |
|---|---|---|---|
| 1 | Aynı kimlikle ikinci bağlantı | Quantex'in canlı akışı düşer | Ayrı `hardwareId`/hesap; bağlanmadan önce denetim; `MULTICONNECT` gelince dur |
| 2 | Eksik alanı 0 sanmak | Sahte fiyat / sahte bar | Çözücü alan varlığını korur; birim testi |
| 3 | Bar hizası (UTC epoch, kapanış seansı) | K14'e aykırı barlar | Barı Qapsule kendisi üretir (`qp-calendar`), Quantex barı kullanılmaz |
| 4 | Hacim birimi karışması (canlı adet, REST TL) | Hacim tabanlı dedektör yanlış | Canlıda TL tutarı alanı seçilir ya da birim açıkça çevrilir (`MESSAGES.md` §6.2; soru) |
| 5 | Düzeltilmiş geçmiş + düzeltilmemiş canlı | Seri ek yerinde sıçrama | Bağda `divide` seçimi açık; A8 ile birlikte karar |
| 6 | Sıra numarası yokluğu, QoS 0 | Kayıp ileti fark edilmez | Barındırıcı yerel `seq` + kopma = `stale` + REST'ten yeniden çözüm (PROTOCOL §18.4) |
| 7 | Zaman alanlarının birimi ve kökeni (epoch µs / gece yarısından bu yana) | Yanlış bar | Alan başına birim testi kayıt üzerinde (`MESSAGES.md` §4.1) |
| 8 | Lisans: gösterimsiz kullanım hakkının belirsizliği | K33 tasarımı dayanaksız kalabilir | R6 öncesi Matriks lisans cevabı |
| 9 | Belirteç yenileme açık bağlantıya geçmiyor (C++) | Uzun bağlantıda kopma | Qapsule yenilemede kontrollü yeniden bağlanır; tek bağlantı korunur |
| 10 | NATS'ta izin yok | Yanlış konuya yazmak motorlara iş yükler | (b) seçilirse yalnız okuma izni zorunlu |

---

## 8. R6 kapsam taslağı (öneri)

Bu taslak `rounds/RI1/REPORT.md` içinde de özetlenir; R6 paketi ana akışta yazılır.

1. **Ön koşullar (round başlamadan):** Matriks'ten Qapsule'e ayrı kimlik (hesap / `typeCode` /
   `hardwareId`) ve mümkünse test ya da gecikmeli uç nokta; lisans cevabı (gösterimsiz kullanım,
   AKDE); hangi sembol evreni.
2. **T1 — Matriks çözücü (`qp-source`):** proto tiplerini derleme sırasında Matriks'in
   sağladığı `.proto`'dan üretmek (kopya değil, Matriks'ten resmi alınmış dosya; lisansı
   sorulur); alan varlığını koruyan `SymbolMessage` → `DataEvent::patch` eşlemesi;
   `TimeMessage` → saat ve açıklık; `stockStatus` → seans evresi.
3. **T2 — Bar üretici:** fiyat kartından K14 hizalı canlı bar (kapanış seansı birleşimi
   dahil); REST geçmişle ek yeri sınaması (aynı gün iki kaynaktan bar eşit mi).
4. **T3 — Bağlantı:** belirteç alma ve yenileme, uç nokta başına tek bağlantı, `MULTICONNECT`
   ve kopmada `stale` + REST'ten yeniden çözüm, yerel `seq`.
5. **T4 — Kayıt ve oynatma:** ham ileti kaydı `fixtures/local/` altında; `qp-source::replay`
   bu kaydı oynatır; aynı kayıt iki kez → bayt eşit çıktı.
6. **T5 — Canlı deneme:** önce kayıt (d1), sonra kısa canlı gösterim; birkaç sembol;
   piyasa saatinde; kapsüllerin canlı doğduğu gösterim (ROADMAP R6 ölçütü).
7. **T6 — Lisans:** okuyucuya göre izdüşüm (K33) gerçek yetki adlarıyla (`MESSAGES.md` §7).
8. **Kapsam dışı (öneri):** Quantex gösterge servislerinin tüketimi; anomali motoru bağlamı;
   derinlik kökenli olgular (kanıt ailesi kararından sonra).

---

## 9. Bu belgeden doğan, sözlüğe ya da karara gidecek adlar (öneri, yapılmadı)

- "Emir defteri / derinlik" kanıt ailesi (`ANOMALY.md` soru 4). Not: CONCEPTS'te
  "Derinlik/`Depth`" anlam zoom seviyesi adıdır; ad çakışması (`INDICATORS.md` T4c §5.3).
- "Seans evresi" (`stockStatus`) kavramı `qp-calendar` için.
- "Matriks kimliği" (servis hesabı + `hardwareId`) ile Qapsule `Principal` ayrımı.
