# Kavram Sözlüğü

Projede herkes (insan ve ajan) aynı kelimeyi aynı anlamda kullanır. Kodda İngilizce
karşılık kullanılır; dokümanda Türkçesi.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Seri | `Series` | Zamana bağlı sayısal veri. Bar, indikatör çıktısı, akış. |
| Seri kaynağı | `SeriesSource` | Serinin geldiği yer: dosya, Matriks yayını, NATS, yerel hesap. Çekirdek kaynağın türünü bilmez. |
| Hesap | `Study` | Seriden seri üreten deterministik fonksiyon. Akışkan, çok çıktılı, parametreli. |
| Olgu | `Fact` | Motorun tespit ettiği, tartışmasız, sayısal olay. "RSI(14) 70'i yukarı kesti, bar #1842." Kanıt verisini taşır. |
| Bulgu | `Finding` | Olgulardan çıkarılan iddia. İddia + dayandığı olgular + güven + geçersizlik koşulu + yazar (ajan/insan). |
| Kapsül | `Capsule` | Tek bir bulgunun insana sunulan, canlı, açılabilir, taşınabilir hali. |
| Anlam işareti | `Glyph` | Kapsülün tek bakışlık görseli. Veriyi değil anlamı çizer. Bir kez öğrenilir, her yerde aynı okunur. |
| Görünüm | `View` | Görsel gramerle yazılmış gösterim tarifi: işaret, ölçek, katman, kodlama. |
| Görsel gramer | `grammar` | Görünümleri kuran yapı taşları kümesi. |
| Derinlik | `Depth` | Anlam zoom'unun seviyesi: 0 cümle, 1 kapsül, 2 işaretli grafik, 3 tam çalışma alanı. |
| Doküman | `Document` | Seri bağları, görünümler, bulgular, kapsüller, çizimler ve niyetlerin tamamı. |
| Komut | `Command` | Dokümanı değiştiren tek yol. İnsan ve ajan aynı komutları kullanır. |
| Niyet | `Intent` | Kullanıcının bıraktığı kalıcı istek. "Yükseliş bozulursa haber ver." |
| Karne | `Ledger` | Bulguların sonradan gerçekle karşılaştırılmış kaydı. |
| Enstrüman | `Instrument` | İşlem gören varlık. Hangi borsada, hangi takvimle işlem gördüğünü taşır. |
| Takvim | `Calendar` | Borsanın saat dilimi, seans saatleri ve tatilleri. Bar hizası buna göre yapılır. |
| Gösterim saat dilimi | `display_tz` | Zamanın kullanıcıya hangi saat diliminde gösterileceği. Bar hizasını etkilemez. |
| Isınma | `warmup` | Bir hesabın ilk geçerli değerini üretmesi için gereken bar sayısı. Bu barlarda çıktı boştur (`null`); çıktı uzunluğu her zaman bar sayısına eşittir. |
| Hesap çıktısı | `StudyOutput` | Bir hesabın bir bar serisi üzerindeki sonucu: bar zamanları ve adlandırılmış çıktılar, her biri bar sayısı uzunluğunda; ısınma `null`. |
| Bar olayı | `BarEvent` | Seri kaynağının ürettiği adım: yeni bar ya da son barın düzeltilmesi (canlı bar). Hesabın akışkan ve "son barı düzelt" kiplerine birebir karşılık gelir. |
| Parametre taraması | `sweep` | Bir hesabın tek çağrıda çok sayıda parametre kümesiyle çalıştırılması; her sonuç tek tek çalıştırmayla bit-eşittir. |
| Köken | `Provenance` | Bir öğeyi kimin, neye dayanarak, ne zaman ürettiği. |
| Alan protokolü | `qp/0` | Motor, arayüzler ve ajanlar arasındaki konuşma; arayüz ve ajan aynı mesajları kullanır (`docs/PROTOCOL.md`). |
| Otorite | `authority` | Bir dokümanın tek yazma noktası: komutları sıraya koyar, günlüğe yazar, değişimi yayar. Belge başına tektir. |
| Kimlik | `Principal` | Konuşan kişi, ajan ya da besleme; rolü otoritenin yapılandırmasındadır. |
| Rol | `Role` | Kimliğin yetkisini belirler: `owner`, `editor`, `proposer`, `viewer`, `feed`, `host`. |
| Bağlantı | `connection` | Bir istemcinin otoriteyle konuştuğu kanal; `hello` ile başlar. |
| Yüzey | `Surface` | Bağlantının sunum ortamı: masaüstü, telefon, saat, ses, ekransız. |
| Profil | `Profile` | Otoritenin yüzeye göre seçtiği sunum derinliği ve veri sınırları (yetenek uzlaşması). |
| İşlem | `Op` | Komuttaki istek, anlam düzeyinde ("RSI ekle", "bulguyu reddet"). |
| Değişim | `ChangeOp` | İşlemin otoritede açıldığı ilkel, tam ve yeniden oynatılabilir hali. |
| Günlük | `LogEntry` / log | Uygulanmış girdilerin sıralı, boşluksuz listesi; her girdide yazar, onaylayan, istek, değişim ve ters değişim. Denetim izidir. |
| Öneri | `Proposal` | Onay bekleyen yazı: `bekliyor → kabul | ret | süresi doldu`. Dokümanın içinde durur. |
| Sorgu | `Query` | Dokümanı değiştirmeyen anlamsal okuma. |
| Abonelik | `Topic` | Bir bulgu, koşul, öneri, dikkat ya da değişim için haber isteği. |
| Girdi | `Gesture` | Dokunma, kaydırma, yakınlaştırma; motor yorumlar, dokümanı değiştirmez. |
| Varlık durumu | presence | Kalıcı olmayan oturum durumu: bakılan aralık, odak, dikkat yönergeleri. Günlüğe girmez. |
| Dikkat yönergesi | `AttentionDirective` | Bir kişinin nereye, hangi sırayla bakmasının istendiği. |
| Anlatım | `Narrative` | Bir bulgu hakkında sıralı, zamanlı adımlar; her adım bir şeyi gösterir ve bir cümle söyler. Dokümanda yaşar. |
| Sunucu | presenter | Bulgudan görünümü deterministik olarak kuran motor parçası; ajan anlam düzeyinde konuşur, sunucu gramere çevirir (K21-1). |
| Devir | hand-over / `epoch` | Otoritenin bir yerden başka yere (sunucu ↔ cihaz) taşınması; her devirde `epoch` artar. |
| Uygunluk dökümü | `Transcript` | Dilden ve taşımadan bağımsız "girdi mesajları → beklenen çıktı mesajları" senaryosu. |

## Kapsülün yaşam döngüsü

`taslak → canlı → (güncellendi)* → geçersiz | süresi doldu | onaylandı | reddedildi`

- **Canlı:** dayandığı seriler değiştikçe kendini günceller.
- **Geçersiz:** geçersizlik koşulu gerçekleşti; kapsül bunu kendisi ilan eder.
- **Onay / ret:** insan girdisi; aynı dokümana yazılır, karneye işlenir.

## Anlam işaretleri (ilk aday küme)

Kesin liste R3'te tasarlanır. Başlangıç adayları:

| İşaret | Cevapladığı soru |
|--------|------------------|
| Yön | Nereye gidiyor? |
| Güç | Hareket ne kadar kuvvetli? |
| Belirsizlik | Ne kadar emin? |
| Aşırılık | Olağandışı mı? |
| Benzerlik | Daha önce oldu mu, sonra ne oldu? |
| Değişim | Son bakıştan beri ne değişti? |
