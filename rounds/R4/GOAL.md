# R4 — İlk Kullanılabilir Gösterim

Dokuz round'un çıktısı sabit görsellerdi. Bu round'un sonunda Arkin tek komutla bir sayfa
açar, gerçek geçmiş verinin hızlandırılmış akışında kapsüllerin doğup evrilmesini izler ve
dokunarak cümleden kanıta iner. Aynı sayfa insan testinin de aracıdır.

Girdi: `docs/CAPSULE.md`, `docs/PROTOCOL.md`, `docs/GRAMMAR.md`, `DECISIONS.md` K39,
K41–K45, `rounds/R3/REPORT.md`, `rounds/R3/human-test/`.

## Ölçü

`NORTH_STAR.md`: geleceğin yatırımcısı "hızlıca bir sonucu görüp çıkmak" ister. İlke 5
(kapsül), 7 (anlam zoom'u) ve AI-native testinin 4. ve 5. soruları: kabuk incedir,
kullanıcı girdisi aynı protokolden geçer.

## Kapsam

1. **Yeniden oynatma kaynağı (K44):** gerçek ya da sentetik geçmişin canlı akış gibi verilmesi.
2. **Web kabuğu:** telefon boyutunda; kapsül akışı; anlam zoom'u; kanıta açılma.
3. **Kapsülün yüzü (K43):** yeniden tasarım, alternatiflerle ve ölçümle.
4. **Etkileşim:** dokunma, kaydırma, yakınlaştırma; çekirdekte, protokolden.
5. **K42 kararlarının uygulanması** ve R3'ten kalan ergonomi önerileri.
6. **Cihazda ölçüm:** kare süresi, tik maliyeti, açılış süresi; telefon sınıfı hedefi ve GPU kararı için veri.
7. **İnsan testi v2:** gösterim üzerinden.

## Kapsam dışı

- Gerçek Matriks MQTT bağlantısı (R6). Yerel iOS/Android kabuğu (R8). GPU çizimi (K45).
- Profesyonel çalışma alanı: çizim araçları, çoklu grafik, indikatör ekleme arayüzü (R7).
- Hesap açma, oturum, sunucuya dağıtım. Gösterim tek makinede, yerelde çalışır.
- Dil modeliyle sohbet. Gösterim tamamen deterministik motordan beslenir.

## Değişmezler

- **Kabuk incedir.** Kabukta alan mantığı (hesap, tespit, sıralama, cümle kurma, yerleşim)
  yoktur; kabuk sahne tarifini boyar, girdiyi protokole iletir, protokolden geleni gösterir.
- Kabuğun yaptığı her şey protokol mesajıdır; aynı oturum bir ajan tarafından aynı
  mesajlarla sürülebilir.
- Yeniden oynatma belirleyicidir: aynı veri, aynı hız, aynı girdiler aynı ekranı üretir.
- Gerçek veri makineden çıkmaz: sayfa dış ağa istek yapmaz; veri, türevleri ve ondan
  alınan ekran görüntüleri commit edilmez.
- Var olan altın görseller, dökümler ve hesap özeti değişmez (K42 gereği değişenler
  gerekçeli listelenir).

## Kapsülün yüzü için tasarım sınırları (K43)

- **Tek bakış:** kapsül kapalı halde bir başlık cümlesi, bir işaret ve en çok iki küçük
  ek bilgi (ör. karne, ne zaman) taşır. Ayrıntı dokununca gelir.
- **Gündelik dil:** "6 bar önce" değil, periyoda ve takvime uygun süre ("6 işlem günü
  önce", "2 saat önce"). İç terimler ("aile", "teyit", "bar") başlıkta yer almaz.
- **Geçerlilik görünür:** bugün hâlâ geçerli olan ile artık geçerli olmayan kapsül ilk
  bakışta ayrılır; işaret ve cümle aynı şeyi söyler (K42).
- **Dürüstlük korunur:** karne ve "az örnekli" bilgisi gizlenmez; kısaltılır.
- **İki dil:** Türkçe ve İngilizce.
- Başlığın kısa olması doğruluktan ödün vermez: zaman ve geçerlilik başlıkta kalır.

## Ölçümle karar verilecek konular (K39)

Her biri için düzenek ve puanlama ölçütü alternatiflerden önce yazılır; alternatifler
birbirini görmeyen alt ajanlarca en iyi halleriyle kurulur.

| # | Konu | Alternatif | Ölçüm |
|---|------|-----------|-------|
| M8 | Kapsülün kapalı yüzü | En az üç tasarım | (a) **Persona okuma testi:** görsel okuyan taze alt ajanlar iki personayla (geleceğin yatırımcısı: teknik analiz bilmiyor, beş saniyesi var; alışkın kullanıcı) yalnızca ekran görüntüsünü görür; sorular: ne oldu, ne zaman, hâlâ geçerli mi, ne kadar güvenilir, bana ne yapmamı söylüyor (doğru cevap: hiçbir şey; tavsiye değil). (b) **Hesapla:** başlık uzunluğu, metin yoğunluğu, işaretin alanı, renk karşıtlığı (erişilebilirlik eşiği), en küçük yazı boyu. (c) **Eleştirmen ajanlar:** önceden yazılmış bir tasarım ölçütüyle, tasarımın hangisi olduğunu bilmeden puanlar. |
| M9 | Akışın düzeni | En az iki (ör. zaman sırası; enstrümana göre gruplu; öneme göre) | Persona görevleri: "son baktığımdan beri ne değişti", "elimdeki hissede olumsuz bir şey var mı", "en güvenilir kapsül hangisi"; doğruluk, gereken dokunma ve kaydırma sayısı. |
| M10 | Açılma geçişi (cümle → kapsül → grafik) | En az iki | Kullanıcının "aynı şeye bakıyorum" sürekliliği: geçişten önce ve sonra ekranda ortak kalan öğeler; persona ajanlarının geçiş sonrası "bu grafikteki işaretli yer az önceki kapsülün neresi" sorusuna cevabı; dokunma sayısı. |
| M11 | "Gerçekleşti" ile "tutmadı"nın işarette ayrımı | En az iki (R3'te yalnızca birkaç piksellik onay/çarpı) | 16–32 px'te ayırt edilebilirlik; persona okuma. |

Sınır: persona ajanı insan değildir. Bu ölçümler ön elemedir; son söz insan testi v2'dedir.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Yeniden oynatma kaynağı: `fixtures/local/` verisini (yoksa sentetik pazarı) alan düzeyinde veri olayları olarak verir; hız, duraklat, adım, belirli tarihe git; saat girdisi barındırıcıdan. Aynı koşu iki kez bayt düzeyinde aynı mesaj akışını üretir. Otorite ve yansı yolu RP2/R3'teki gibi çalışır. |
| 2 | `scripts/demo.sh` tek komutla gösterimi açar; yerel veri yoksa sentetikle çalışır ve bunu ekranda söyler. Sayfa dış ağa istek yapmaz (testle). Aynı ağdaki telefondan açılabilir; nasıl açılacağı ekranda ve belgede. |
| 3 | Web kabuğu telefon boyutunda: kapsül akışı; kapsüle dokununca açılma; grafik derinliğinde kaydırma ve yakınlaştırma; kanıta açılma (yetkiliyken değerler, yetkisizken "şu yetkiyi gerektirir"); karne; hassasiyet seviyesi; izleme listesi ve pozisyon işaretleme; Türkçe/İngilizce; açık/koyu tema. |
| 4 | Kabuk ince: mimari testi kabuk kodunda alan mantığı olmadığını denetliyor (hesap, cümle, sıralama, yerleşim çağrısı yok; yalnız sahne boyama, girdi iletme, protokol mesajı gösterme). Aynı oturum yalnızca protokol mesajlarıyla (kabuksuz) sürüldüğünde aynı doküman ve aynı sahne özetleri çıkıyor. |
| 5 | Etkileşim çekirdekte: dokunma, kaydırma, yakınlaştırma `input` mesajı olarak gidiyor; bakış durumu günlüğü şişirmiyor (PROTOCOL §4.6); hareket sırasında kare süresi ölçülüyor. |
| 6 | Kapsülün yüzü M8 ile seçildi; tasarım sınırlarına uyuyor; başlık ve ayrıntı cümleleri şablon dilbilgisinden (belirleyici, iki dilde); süre ifadesi takvime göre. `docs/FACE.md` tasarımı, elenenleri ve gerekçeleri anlatıyor. Yeni altın görseller eklendi. |
| 7 | K42: karne yönü ayırıyor; yalnız olumsuz geçişler hassasiyetten bağımsız; geçerli olmayan kapsül işarette de öyle görünüyor; durum koşulu ile beklenen çözüm ayrı. Değişen dökümler ve görseller gerekçeli listelenmiş; hassasiyet benzetimi yeniden koşulmuş. |
| 8 | M8–M11'in her biri için: düzenek ve puanlama ölçütü (alternatiflerden önce), alternatifler, ham kayıtlar, seçim ve gerekçe `rounds/R4/experiments/` altında. Eleştirmen ve persona ajanlarının tasarımı kimin yaptığını bilmediği kayıtlardan anlaşılır. |
| 9 | Cihaz ölçümü: gösterim sayfasında bir ölçüm ekranı kare süresini (durağan, kaydırırken), tik maliyetini, açılış süresini ve paket boyutunu gösterir ve tek dokunuşla kopyalanabilir bir özet üretir. Masaüstünde ve kısıtlanmış işlemci benzetiminde ölçülmüş. Arkin'in kendi telefonunda ölçmesi için yönerge var. Telefon sınıfı hedefi (K17) ve GPU kararı (K45) için öneri, ölçüme dayalı. |
| 10 | Açılış: 30 enstrümanlık oturuma bağlanma 100 ms'nin altında (R3: 133 ms). Sayfanın ilk kapsülü göstermesine kadar geçen süre ölçülmüş. WASM boyutu ölçülmüş; yansılı paketin ayrı yüklenmesi değerlendirilmiş. |
| 11 | Ekran görüntüsü testleri: yeniden oynatmanın sabit anlarında başsız tarayıcıda alınan ekranlar belirleyici ve sentetik veriyle depoda; gerçek veriyle alınanlar depoda değil. |
| 12 | İnsan testi v2: `rounds/R4/human-test/` altında gösterim üzerinden yapılan görev listesi (yaklaşık yirmi dakika, beş ila on kişi), gözlem formu, puanlama; alfabe finalistleri (`siluet`, `rozet`) ve yüz tasarımının ilk iki sırası gösterimde ayardan değiştirilebiliyor. Sonuçların A21'i ve yüz seçimini nasıl kapatacağı yazılı. |
| 13 | R3 ergonomi önerileri: `capsule` sorgusu seçici kabul ediyor; `capsules` sorgusunda "en son doğan" ve doğum süzgeci; seçicide aşama/açık anahtarı; kitle biçimi şartnamede. Ajan yazabilirlik yeniden ölçülmüş (R3: ilk deneme 12/20). |
| 14 | Dökümler üç taşımada geçiyor; yeni dökümler (yeniden oynatma, girdi, açılma) eklenmiş. |
| 15 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; `docs/FACE.md`; önerilen kararlar `DECISIONS.md`'de ayrı bölümde. |
