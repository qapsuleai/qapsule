# Uygulama yönergesi (yürütücü için) — insan testi v3

Amaç: R5'in tasarım adaylarını (her konuda ajan ön elemesinin ilk ikisi) gerçek insanlarla,
telefonda ve gerçek hızda karşılaştırmak; açık kararlara (`DECISION.md`) insan kanıtı getirmek.
**Ajan okuyucu insan değildir**; ajan sonucuyla bu test ayrışırsa bu test geçerlidir.

## Özet

| | |
|---|---|
| Süre | Kişi başına yaklaşık **20 dakika** (`TASKS.md` "Akış ve süre") |
| Kişi | **5–10**; iki persona, mümkünse eşit sayı (aşağıda) |
| Kod | Genç **G01, G02 …**, alışkın **A01, A02 …** — geliş sırasıyla, persona içinde ayrı sayılır. Ad, yaş, iletişim bilgisi yazılmaz |
| Sıra | Görev sırası sabit (G0 → G8 → T); her karşılaştırmada hangi adayın önce geleceği koddan çıkar (`TASKS.md` "Karşı dengeleme") |
| Malzeme | Bilgisayar (gösterim), aynı ağda bir telefon (katılımcının değil, yürütücünün), yanıt kartı (masada; `TASKS.md` kart soruları basılır, numaralı), gözlem formu (katılımcı başına; `FORM.md`), `key.md` (yürütücüde, katılımcı görmez), metin örtüsü (G3), kronometre |

## Katılımcı profili

| Persona | Kim | Kod |
|---|---|---|
| **Genç** (geleceğin yatırımcısı) | Yaklaşık 18–30; telefondan yatırım yapan ya da yeni başlayan; teknik analiz bilmiyor (mum, indikatör, direnç sözcükleri ona bir şey ifade etmiyor) | G |
| **Alışkın** | 40 yaş ve üstü; yıllardır işlem yapıyor; grafik ve terminal ekranlarına alışkın, ayrıntıya güvenir | A |

- Persona katılımcıya sorulan iki soruyla belirlenir: "Teknik analiz terimlerini (mum, direnç,
  indikatör) kullanır mısınız?" ve "Kaç yıldır yatırım yapıyorsunuz?" Cevaplar yazılmaz; yalnız
  persona kodu yazılır. Hiçbirine uymayan kişi alınmaz.
- Proje ekibinden, tasarımları önceden görmüş ya da galeriye bakmış kişi alınmaz.
- **Renk görme:** en az bir katılımcının renk görme farkı olması (ör. kırmızı-yeşil ayrımı)
  istenir; anlam renklerinin biçimle de okunması (GOAL "renk tek başına anlam taşımaz") ancak
  böyle sınanır. Bu yalnız **gönüllü beyanla** öğrenilir ("Renkleri ayırt etmekte bildiğiniz bir
  güçlük var mı? İsterseniz söylemeyebilirsiniz."); formda tek kutu (var / yok / söylemek
  istemiyor). Tanı, ayrıntı, gözlük vb. yazılmaz. Raporlama kuralı `SCORING.md` §6.
- Katılımcıya ödeme ya da hediye varsa herkese aynı ve cevaplardan bağımsız.

## Hazırlık (bir kez)

1. **Gösterim:** `scripts/demo.sh --synthetic --paused --lan`. Konsol ve Ayarlar sekmesi
   "telefondan aç: http://192.168.x.y:8390/" yazar; telefon aynı ağda olmalı. `--lan` yerel ağdaki
   cihazlara açar; sentetik veriyle sorun yok. Ağ kısıtlıysa ya da pano/güvenli bağlam gerekirse
   önüne bir HTTPS vekili konup `--https-host AD` verilir (`shells/web/README.md`). **Gerçek veri
   kullanılmaz:** anahtar sentetik pazar için çıkarılır; gerçek veriyle ekran görüntüsü de alınmaz.
2. **Galeri (isteğe bağlı, yürütücünün kendi provası için):** `scripts/demo.sh --gallery --lan`
   → telefonda `http://192.168.x.y:8390/gallery.html`. Adayları aynı içerikte alt alta gösterir;
   yürütücü adayların farkını önceden bilsin diye. **Galeri katılımcıya gösterilmez** (adaylar
   adlarıyla ve yan yana durur; bu test ise sırayla, adsız karşılaştırır).
3. **Yer tutucular dolu mu:** `grep -n '<GÖRÜNÜŞ\|<OPEN\|<MOTION' rounds/R5/human-test/*.md` boş
   dönmeli (README "Doldurulacak yer tutucular"). Dolmamış konu varsa `DECISION.md` "Eksik konu".
4. **Ayarlar seçilebiliyor mu:** taban bağlantıyı (U00) ve her kodu (`TASKS.md` "Bağlantı kartı")
   bir kez açın; Ayarlar sekmesinde ilgili ayarın URL'deki değeri gösterdiğini denetleyin.
   Bilinmeyen ad sessizce yok sayılabilir (R4 dersi): her kodda gerçekten farklı görünümün
   geldiğini gözle doğrulayın.
5. **Prova ve cevap anahtarı** (aşağıda) → `key.md`.
6. **Metin örtüsü (G3):** yalnız işaretleri açıkta bırakan opak kâğıt. Taban görünüşte işaretin
   yeri provada telefonun kendisinde ölçülür; görünüş adayları işareti farklı yere koyuyorsa örtü
   **G3'ün taban görünüşüne** göre kesilir (G3'te görünüş değişmez). İşaret yazının üstünde ya da
   resmin içindeyse örtü işe yaramaz; o zaman G3 metnin gizlendiği bir ayarla (varsa) yapılır ya da
   A21 bu testte kapanamaz (`DECISION.md` A21).
7. **Telefon:** yakınlaştırma %100, parlaklık normal, gece modu kapalı, ekran kilidi uzun.
   Katılımcı iki parmakla büyütmez (G1, G3). Adres çubuğu gizli (sayfa hafif kaydırılır).
8. **Bir deneme oturumu** yapın (ekipten biriyle; sonuçları sayılmaz); süreleri tabloyla
   karşılaştırın, taşan görevi not edin.

## Prova ve cevap anahtarı (oturumlardan önce, bir kez)

Her görevin içerik anı (`<ADIM-…>`, `<SATIR-…>`, `<KİTLE-…>`) aşağıdaki **önceden yazılı
ölçütlere** göre seçilir ve `key.md`'ye yazılır. Anahtar motorun durumundan okunur (Ayarlar'daki
durum, `window.qp.feed()` ve kabuğun aldığı protokol iletileri; R4 `prepare.mjs` aynı işi yapıyordu,
uyarlanabilir), tasarımdan değil. Bir anı seçerken adaylardan birinde "daha iyi görünüyor" diye
seçim yapılmaz; ölçüte uyan ilk an alınır (adım sırasıyla tarayın).

| Yer tutucu | Ölçüt |
|---|---|
| `<ADIM-P>` | herhangi bir an; puanlanmaz |
| `<ADIM-G1-1a/1b/2a/2b>` | dört kapsül: en az biri canlı, biri kapanmış (gerçekleşti ya da tutmadı), biri "artık geçerli değil"; en az iki farklı tür ve iki yön; en az biri karnesi az örnekli (n < 20), en az biri n ≥ 20. Set 1 ile set 2 aynı zorluk karışımında (her sette bir canlı, bir kapanmış ya da geçersiz) |
| `<ADIM-G2-1/2>` | eşiği adıyla ve değeriyle söyleyen bir kapsül (iki dilde de); en az bir sözlük terimi; karnesi n ≥ 20; iki set farklı dedektör |
| `<ADIM-G3-1/2>` | görünen ilk üç kapsülde iki farklı tür, iki yön, en az bir kapanmış |
| `<ADIM-G4-1/2>`, `<ADIM-G4-V1/V2>` | G4 metnindeki ölçüt; set 1 "sunucu doğruladı", set 2 "bu cihazda sınandı"; yetkisiz anlar aynı kapsüller |
| `<ADIM-G5-1/2>` | G5 metnindeki ölçüt (≥ 10 hikâye, ≥ 1 canlı, ≥ 3 kapanmış, karışık sonuç); `c_evid` için gösterilecek kapanmış işaret seçili olandan farklı |
| `<ADIM-G6-n-T0/T1>`, `<KİTLE-…>` | G6 metnindeki ölçüt; iki sırada da olumsuz geçiş akışta görünmeli (görünmüyorsa pencere değişir; R4 v2 "Açık sorular 2") |
| `<ADIM-G7-1/2>` | G7 metnindeki ölçüt |
| `<ADIM-G7-BOŞ>`, `<KİTLE-BOŞ>` | `rate55` ile akışın boş olduğu bir an (az enstrümanlı kitleyle) — boş akış ekranı görünmeli |
| `<ADIM-G8-1/2/3>` | bir sonraki adımda görünen alanda bir kapsülün durumu değişiyor (doğdu, güçlendi ya da geçersiz oldu); üç sette üç ayrı an, değişim türleri karışık |

`key.md` her madde için: adım, satır (akışta yukarıdan 0 tabanlı), kapsül kimliği, enstrüman,
doğru cevaplar (TASKS.md soru kodlarıyla), G2 için eşik adı ve değeri ve toleransı (son basamağın
yarısı), sözlük teriminin metni, G4 için kanıt aileleri ve güven iddiası. Anahtar, adayların
hepsinde aynı kapsülün aynı satırda olduğunu denetleyerek yazılır (akış sırası G6 dışında sabit;
G6'da iki sıra ayrı ayrı oynanır, doğru cevaplar aynı olmalı). Denetim geçmezse an değiştirilir.

`key.md` sentetik pazardan çıkar ve kişisel veri içermez; prova bitince commit edilebilir. Gerçek
veriyle anahtar çıkarılmaz.

## Onam (giriş; sözlü, aynen)

> "Bu yaklaşık yirmi dakikalık bir deneme. Bir uygulamanın ekranlarının ne kadar anlaşıldığını
> ölçüyoruz; ölçülen siz değilsiniz, ekranlar. Cevaplarınız sizi değerlendirmez. Adınızı ya da
> sizi tanıtacak bir bilgiyi almıyoruz; formda yalnız bir kod yazacak. Ekrandaki piyasa verisi
> uydurmadır, gerçek bir hisse değildir. Uygulama tavsiye vermez. Ses, görüntü ya da ekran kaydı
> almıyoruz; yalnız ben not tutuyorum. Düşündüklerinizi yüksek sesle söylerseniz bize yardımcı
> olur, ama zorunlu değil. İstediğiniz an bırakabilirsiniz; o ana kadarki cevaplarınızı isterseniz
> sileriz. Başlayalım mı?"

"Evet" yoksa oturum yapılmaz. İngilizce isteyen katılımcı için `TASKS.md` "English annex".

## Akış

`TASKS.md` görev görev uygulanır. Genel kurallar:

- Süreler yönlendiricidir; süre dolunca **"Tahmin edip ilerleyelim."**
- Kronometre her görevde TASKS.md'nin yazdığı anda başlar ve biter; formda saniye.
- Dokunma sayısı: ekrana her ayrı dokunuş (kaydırma bir dokunuş sayılır; FORM.md).
- Kararsızlık: katılımcı cevabı değiştirirse, 5 sn'den uzun duraksarsa ya da "emin değilim" derse
  formda işaretlenir.
- **Yüksek sesle düşünme:** katılımcının kendiliğinden söyledikleri aynen not edilir; yürütücü
  sormaz, yorumlamaz. Uzun sessizlikte yalnız bir kez: **"Aklınızdan geçeni söyleyebilirsiniz."**
- İpucu: "ekranda ne görüyorsanız o" dışında bir şey söylendiyse formda "ipucu" (o cevap yanlış
  sayılır).
- Kapsül arası hazırlık (adıma gitme, kaydırma, kod değiştirme) katılımcı kart sorularını
  cevaplarken ya da telefon yürütücüdeyken yapılır; katılımcı ayar adlarını ve Ayarlar sekmesini
  görmez.
- Oynatma her zaman duraklatılmış kalır; yalnız G8'de **+1** basılır.

## Söylenmeyecekler

- Tasarım adları (`marla`, `selvi`, `pavka`, `korda`, `grouped`, `live_new`, `rate55`, `none`,
  `siluet`, `rozet`, görünüş, açılmış kapsül ve hareket adayları), hangisinin "yeni", "seçilen" ya
  da bir yapay zekânın önerdiği olduğu.
- Bir cevabın doğru ya da yanlış olduğu; baş sallama, onaylayan ses de yok.
- İşaretin, rengin, çizginin ne anlama geldiği; "kapsül", "bar", "kırılım", "karne", "hikâye"
  gibi terimler (yürütücü "kutu", "olay", "işaret" der).
- Piyasa yorumu ya da alım-satım konuşması; "tavsiye vermez" yalnız onamda bir kez söylenir.
- Önceki katılımcıların cevapları; bir tasarımı övmek ya da yermek.

## Kayıt ve verinin depoya girişi

1. Oturum biter bitmez form denetlenir (kod, persona, dil, gösterim türü, boş satır). Dikkat
   çeken olay (büyütme, örtüyü kaldırma, adres çubuğunu görme, ağ kopması) "not"a yazılır.
2. Bütün oturumlar bitince kâğıt formlar `FORM.md` biçiminde **`answers.csv`**'ye geçirilir
   (geçiren `key.md`'ye bakmaz; puanlama ayrı adımdır). Açık uçlu sözler aynen ama **kişiyi
   tanıtabilecek her şey çıkarılarak** (ad, iş yeri, şehir, kendi yatırımları, sağlık) yazılır.
3. Puanlama `SCORING.md`'ye göre yapılır → **`results.md`** (toplu: persona başına ve toplam
   sayılar, oranlar, ortancalar, karar). Kişi başına satır `results.md`'ye girmez; katılımcı
   kodları yalnız `answers.csv`'de durur.
4. **Depoya ne girer:** `results.md` (toplu) ve `key.md`. `answers.csv` kod-anonimdir ve kişisel
   veri içermemelidir; yine de **Arkin'in onayıyla** commit edilir, onay yoksa depo dışında
   (yürütücüde) kalır ve `results.md` yalnız toplu sayıları taşır. Kâğıt formlar CSV'ye geçirilip
   denetlendikten sonra imha edilir. Renk görme beyanı yalnız `SCORING.md` §6'daki toplu biçimde
   raporlanır.
5. Sonuç `DECISION.md` kuralıyla değerlendirilir; karar satırlarını orkestratör / ana akış
   `DECISIONS.md`'ye yazar (bu paket yazmaz).

## Sık karşılaşılan durumlar

- **Kapsül beklenen satırda değil:** yanlış kod açık ya da kitle duruyor; adıma yeniden gidin,
  kodu denetleyin.
- **Ayar değişmedi** (iki set aynı görünüyor): kodun URL'si yanlış ya da ad bilinmiyor; o set
  "geçersiz" işaretlenir, katılımcının o karşılaştırması sayılmaz.
- **Katılımcı büyüttü / örtüyü kaldırdı:** cevap kaydedilir, "not"a yazılır; puanlamada ayrıca
  raporlanır.
- **Ağ koptu, sayfa yenilendi:** URL'deki ayarlar ve kitle geri gelir (R4/S1); adıma yeniden gidin.
- **Zaman taştı:** `TASKS.md`'deki atlama sırası.
