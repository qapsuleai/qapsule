# Uygulama yönergesi (yürütücü için) — insan testi v2

Amaç: gösterim (`scripts/demo.sh`) üzerinden, telefon boyutunda ve gerçek hızda beş
karşılaştırmayı gerçek insanlarla yapmak (GOAL ölçüt 12). Persona ajanları (M8–M11) ön elemeydi;
**persona ajanı insan değildir**, son söz bu testtedir (`DECISION.md`).

| Görev | Konu | Karşılaştırılan (A / B) | Ölçüm kökü |
|---|---|---|---|
| G5 | işaret alfabesi | `siluet` / `rozet` | A21, Ö32 |
| G1 | kapsülün kapalı yüzü, ilk bakış (5 sn) | `Y1` / `Y2` (M8'in ilk iki sırası) | M8 |
| G3 | açılma: kapsül → grafik | `zumlak` / `tutamir` | M10 |
| G4 | kapanmış kapsülde "gerçekleşti / tutmadı" | `tg5` / `r3` | M11 |
| G2 | akış: son bakıştan beri ne değişti | `elikov` / `vakiro` | M9 |

A = persona ölçümünün ön seçimi, B = ikinci. Adlar katılımcıya hiçbir zaman söylenmez.
**Yüz adları tek yerde:** `plan.json` → `"faces": {"Y1": …, "Y2": …}`. Doldurulunca
`node prepare.mjs key` yeniden koşulur; bütün bağlantılar ve raporlar oradan okur.

## Özet

| | |
|---|---|
| Süre | Kişi başına **yaklaşık 20 dakika** (aşağıdaki akış tablosu) |
| Kişi | 5–10; iki grup: **alışkın** (grafiğe düzenli bakan / işlem yapan) ve **genç** (yaklaşık 18–30, işlem yapmayan ya da yeni başlayan; teknik analiz bilmeyen). Mümkünse eşit sayı. |
| Kod | Alışkın **A01, A02 …**, genç **G01, G02 …** — geliş sırasıyla, grup içinde ayrı sayılır. Ad, yaş, iletişim bilgisi yazılmaz. |
| Sıra | Görev sırası sabit (G5 → G1 → G3 → G4 → G2). Her görevde iki set var; 1. set hep önce gelir, hangi tasarımın 1. sette gösterileceği koddan çıkar (aşağıdaki **Sıra** tablosu). |
| Malzeme | Bilgisayar (gösterim), aynı ağda bir telefon, `form.html` çıktısı (katılımcı başına), yanıt kartı (form.html 1. sayfa, masada), `key.md` çıktısı (yürütücüde, katılımcı görmez), **metin örtüsü** (aşağıda), kronometre. |

## Hazırlık (bir kez)

1. Gösterimi aç: `scripts/demo.sh --synthetic --paused --lan`. Konsol ve **Ayarlar** sekmesi
   "telefondan aç: http://192.168.x.y:8390/" yazar; telefon aynı ağda olmalı. `--lan` yerel
   ağdaki cihazlara açar; sentetik veriyle sorun yok. **Varsayılan sentetik pazardır.** Yerel
   gerçek veri varsa yürütücü seçebilir, ama anahtar (`key.json`) sentetik pazar için çıkarıldı:
   gerçek veriyle anahtar geçersizdir, `plan.json` yeniden kurulmalıdır (`prepare.mjs scan`).
   Gerçek veriyle ekran görüntüsü alınmaz, hiçbir şey commit edilmez.
2. Anahtarı çıkar: `cd rounds/R4/human-test && node prepare.mjs key` (gereken:
   `cargo build --release -p qp-cli`, `shells/web/pkg` (bir kez `scripts/demo.sh`),
   `cd shells/web && npm ci`). Betik oturumu başsız tarayıcıda yürütücünün sırasıyla oynar ve
   `key.json`, `key.md`, `links.html` yazar. Uyarı basarsa (satır kaydı, akışta görünmeyen olumsuz
   haber) oturum yapılmaz; `plan.json` düzeltilir. Depodaki `key.json` ile bayt eşit çıkmalı
   (aynı plan + aynı gösterim → aynı anahtar).
3. Bağlantı sayfası: `python3 -m http.server 8391 --bind 0.0.0.0` (bu klasörde), telefonda
   `http://192.168.x.y:8391/links.html`. Gösterimin adresini bir kez yazın. Kodlar (U0–U10)
   tarafsızdır; adlar yalnız URL'dedir ve bu sayfa katılımcıya gösterilmez.
4. **Son bakış yer imi** (G2 için): telefonun tarayıcısına adı `sb`, adresi
   `javascript:qp.seen();void(0)` olan bir yer imi ekleyin (links.html'de yazar). Gösterimde
   "son bakış"ı koyan bir düğme yok (açık soru, `README.md`). Çalışmazsa G2'yi masaüstü
   tarayıcıda telefon görünümüyle (390×844) yapın, konsolda `qp.seen()`; formda gösterim "masaüstü".
5. **Metin örtüsü** (G5, G4): yüzün yalnız işaretini açıkta bırakan opak bir kâğıt şerit
   (ör. katlanmış A4). Telefon masada durur; örtü işaretin sağındaki/altındaki bütün metni ve
   enstrüman adını kapatır. Örtüyü `Y1` yüzüne göre bir kez kesin; deneme oturumunda denetleyin.
6. Telefon: tarayıcı yakınlaştırması %100, parlaklık normal, gece modu kapalı; ekran kilidi
   uzun. Katılımcı **iki parmakla büyütmez** (G4 için şart). Adres çubuğu görünmesin: sayfayı
   hafif kaydırınca mobil tarayıcı çubuğu gizler; tasarım adı geçen URL'yi katılımcı görmemeli.
7. Bir deneme oturumu kendiniz yapın; süreleri tabloyla karşılaştırın.

### Gösterimde gezinme (yürütücü, telefon katılımcıda değilken)

- **Ayar değiştirme:** links.html'den kodu açın (`?face= &feed= &transition= &outcome= &alphabet=`
  URL'si). URL ile verilen ayar sayfa yenilense de, oynatma geri sarılsa da kalır. Ayarlar
  sayfası da çalışır, ama **oynatmayı geri sarınca Ayarlar sayfasındaki seçim kaybolur** (URL'nin
  değerine döner) ve seçenek adları ekranda görünür; bu yüzden URL kullanın.
- **Adıma gitme:** alttaki kutuya adım numarasını yazıp **Git**. Geri gitmek otoriteyi baştan
  kurar (yarım saniye); izleme listesi ve pozisyonlar silinir. G2 bu yüzden en sondadır.
- **Satır:** `key.md`'deki "satır" akışın yukarıdan kaçıncı kapsülüdür (0 = en üstteki; grup
  başlıkları sayılmaz). G1/G3'te o kapsülü ekranın en üstüne getirin; enstrüman adı ve cümle
  `key.md`'de yazar. "Son üç" = akışın en altındaki üç kapsül (sona kaydırın).
- Oynatma her zaman **duraklatılmış** kalır (▶'ye basmayın).

## Onam ve gizlilik (giriş; sözlü, aynen)

> "Bu yaklaşık yirmi dakikalık bir deneme. Bir uygulamanın ekranlarının ne kadar anlaşıldığını
> ölçüyoruz; ölçülen siz değilsiniz, ekranlar. Doğru ya da yanlış cevap sizi değerlendirmez.
> Adınızı ya da sizi tanıtacak bir bilgiyi almıyoruz; formda yalnız bir kod ve grubunuz yazacak.
> Ekrandaki piyasa verisi uydurmadır, gerçek bir hisse değildir. Uygulama tavsiye vermez.
> İstediğiniz an bırakabilirsiniz; o ana kadarki cevapları ister kullanırız ister sileriz.
> Başlayalım mı?"

"Evet" yoksa oturum yapılmaz. Ekran görüntüsü, ses, video kaydı alınmaz. Katılımcının sağlık
(renk görme, gözlük) ya da kişisel bilgi vermesi hiçbir yere yazılmaz.

## Akış ve söylenecekler

Süreler yönlendiricidir; süre dolunca "Tahmin edip ilerleyelim". Süre (saniye) formda istenen
yerlerde kronometreyle tutulur. Her cevapta katılımcı yanıt kartından seçeneğin **numarasını**
söyler ya da gösterir; yürütücü daire içine alır. Katılımcı yardım isterse yalnız **"Ekranda ne
görüyorsanız o; emin değilseniz tahmin edin."** denir; buna rağmen ipucu verildiyse formda
"ipucu" işaretlenir (o cevap yanlış sayılır).

| # | Adım | Süre | Ayar / adım | Yürütücünün söylediği (aynen) |
|---|---|---:|---|---|
| 0 | Giriş, onam, kod | 1:30 | — | Onam metni. Grubu birlikte işaretleyin. |
| P | Alıştırma (puanlanmaz) | 1:00 | U0, adım 1, satır 0 | "Telefonu beş saniye göstereceğim; en üstteki kutuya bakın. Sonra telefonu alıp kartta sorular soracağım." 5 sn → kartla S1–S5'i birlikte gezin. Cevap tartışılmaz. |
| G5 | İşaret alfabesi | 3:00 | 1. set: sıra tablosu (U7/U8), adım 1, son üç; 2. set: öteki kod, adım 201, son üç | Metin örtüsü yerleştirilir. "Şimdi yazılar kapalı; yalnız küçük işaretler açık. Yukarıdan aşağı üç işaret var. Her biri için kartta üç soru: yön, ne oluyor, hâlâ geçerli mi. Telefona dokunmayın, büyütmeyin." Süre: örtü kalkınca başlar, üçüncü işaretin son cevabında biter. |
| G1 | Yüz, ilk bakış | 6:00 | 1. set (U1/U2): 1a adım 1 satır 1, 1b adım 46 satır 2, 1c adım 64 satır 2; 2. set öteki kod: 2a adım 100 satır 7, 2b adım 163 satır 1, 2c adım 334 satır 4 | Her kapsülde: hedefi en üste getir, telefonu ver: "Beş saniye; en üstteki kutuya bakın." 5 sn sonra telefonu alın. "Kartta altı soru: ne oldu (yön, ne), ne zaman, hâlâ geçerli mi, ne kadar güvenilir, size ne yapmanızı söylüyor." Süre: telefon alındıktan son cevaba kadar. |
| G3 | Açılma | 3:00 | 1. set (U3/U4): 1a adım 334 satır 5, 1b adım 334 satır 7; 2. set öteki kod: 2a adım 469 satır 7, 2b adım 469 satır 6 | Hedef en üstte, telefon katılımcıda: "Bu kutuya dokunun. Bir daha dokunun." Grafik açılınca kronometre başlar: "Az önceki kutunun söylediği şey bu grafikte nerede? Parmağınızla gösterin." Gösterince durur. Sonra kartta: "Az önceki kutuyla aynı şeye mi bakıyorsunuz?", "Hâlâ geçerli mi?", "Açılırken nerede olduğunuzu kaybettiniz mi (1 hiç … 5 tamamen)?" ‹ Geri iki kez. |
| G4 | Sonuç işareti | 2:30 | 1. set (U5/U6): adım 76, son üç; 2. set öteki kod: adım 351, son üç | Metin örtüsü. "Bu üç kutu kapanmış ya da açık olabilir. Yalnız işarete bakarak: gerçekleşti mi, tutmadı mı, hâlâ açık mı?" Süre: örtü kalkınca → üçüncü cevap. |
| G2 | Akış | 3:00 | 1. set (U9/U10): adım 600'de kitle (aşağıda), `sb`, adım 609; 2. set öteki kod: adım 882'de 2. kitle, `sb`, adım 891 | Telefon katılımcıda, akış en üstte: "Bu sizin uygulamanız. En son dün baktınız. Elinizde **{elde: key.md}** var; izleme listenizde birkaç hisse daha. Kaydırabilirsiniz." Üç soru, her biri için süre: (a) "Son baktığınızdan beri ne değişti?" (b) "Elinizdeki hisselerde olumsuz bir şey var mı? Hangisinde, ne?" (c) "En güvenilir kutu hangisi? Gösterin." Kaydırma sayısını not edin. |
| T | Tercihler | 1:00 | — | Her görev için: "İlk gördüğünüz mü ikinci gördüğünüz mü daha kolaydı, yoksa fark yok mu?" (G5, G1, G3, G4, G2 sırasıyla; "işaretler", "kutular", "açılış", "kapanmış kutuların işareti", "akış" diye anın). Sonra: "Eklemek istediğiniz bir şey?" |
| K | Kapanış | 0:15 | adım 1'e dön | "Teşekkürler. Doğru cevapları şimdi konuşmayalım; sonraki katılımcıları etkilemesin." |

Toplam yaklaşık 21 dakika; G1'in kapsül arası hazırlığı (Git + kaydırma) katılımcı soruları
cevaplarken yapılır.

**G2 kitlesi (Ayarlar sekmesi, telefon yürütücüdeyken):** Kullanıcı = Alışkın, Hassasiyet =
Dengeli; izleme listesi ve pozisyonlar `key.md` "G2-1"/"G2-2" başlığında (`plan.json`
`G2.audience` ve `G2.sets[1].audience`): 1. pencere izleme SYN02 SYN03 SYN04 SYN05 SYN22 SYN25
SYN26 SYN28, pozisyon (long) SYN09 SYN12; 2. pencere aynı izleme listesi, pozisyon SYN19 SYN01
(SYN09 ve SYN12 boşa). **Uygula**, sonra Akış sekmesi. Kitle sayfa yenilenince (kod değişince)
kalır, geri sarınca silinir.

## Sıra (karşı dengeleme)

Her hücre: 1. sette açılacak kod → 2. sette açılacak kod (`python3 -I score.py --sequence 10`
üretir; adlar `links.html` ve `plan.json`'da). Her karşılaştırma, bir gruptaki her iki
katılımcıda bir dengelenir. Kodlar U0, U1, U3, U5, U7 aynı adresi açar (taban ayarlar).

| kod | G5 | G1 | G3 | G4 | G2 |
|---|---|---|---|---|---|
| A01 / G01 | U7 → U8 | U1 → U2 | U3 → U4 | U5 → U6 | U10 → U9 |
| A02 / G02 | U8 → U7 | U2 → U1 | U4 → U3 | U6 → U5 | U9 → U10 |
| A03 / G03 | U7 → U8 | U2 → U1 | U3 → U4 | U6 → U5 | U9 → U10 |
| A04 / G04 | U8 → U7 | U1 → U2 | U4 → U3 | U5 → U6 | U10 → U9 |
| A05 / G05 | U7 → U8 | U1 → U2 | U4 → U3 | U6 → U5 | U10 → U9 |
| A06 / G06 | U8 → U7 | U2 → U1 | U3 → U4 | U5 → U6 | U9 → U10 |
| A07 / G07 | U7 → U8 | U2 → U1 | U4 → U3 | U5 → U6 | U9 → U10 |
| A08 / G08 | U8 → U7 | U1 → U2 | U3 → U4 | U6 → U5 | U10 → U9 |
| A09 / G09 | U7 → U8 | U1 → U2 | U3 → U4 | U5 → U6 | U10 → U9 |
| A10 / G10 | U8 → U7 | U2 → U1 | U4 → U3 | U6 → U5 | U9 → U10 |

## Söylenmeyecekler

- Tasarım adları (`siluet`, `rozet`, yüz adları, `elikov`, `vakiro`, `zumlak`, `tutamir`,
  `tg5`, `r3`), hangisinin "yeni", "bizim seçtiğimiz" ya da yapay zekânın önerdiği olduğu.
- Bir cevabın doğru ya da yanlış olduğu; baş sallama, "hmm" de yok. "Doğru cevap" diye bir şeyin
  var olduğu bile söylenmez.
- İşaretin, rengin, çizginin, grafiğin ne anlama geldiği; "bar", "kırılım" gibi terimler.
- Piyasa yorumu, "al/sat" konuşması; uygulamanın tavsiye vermediği girişte bir kez söylenir, sonra
  S5'te hatırlatılmaz.
- Önceki katılımcıların cevapları; bir tasarımı övmek ya da yermek.

## Kayıt

1. Oturum biter bitmez formu denetleyin (kod, grup, gösterim türü, boş satır), günlüğe dikkat
   çeken şeyi yazın (telefonu büyütme, örtüyü kaldırma, adres çubuğunu görme, yer imi sorunu).
2. Bütün oturumlar bitince formları `FORM.md` biçiminde `answers.csv`'ye geçirin; geçirirken
   `key.md`'ye bakmayın.
3. `python3 -I score.py answers.csv > results.md`. Uyarıları formdan denetleyin. Karar raporun
   "Karar" bölümündedir (`DECISION.md` kuralı).
4. Form ve CSV kişisel veri içermez; yine de depoya eklenmeden önce Arkin'e sorulur.

## Sık karşılaşılan durumlar

- **Kapsül beklenen satırda değil:** `node prepare.mjs key` uyarısız geçtiyse aynı adım aynı akışı
  verir; büyük olasılıkla yanlış kod (feed) açık ya da kitle duruyor (geri sarıp yeniden gelin).
- **Yer imi çalışmadı:** G2'yi masaüstü telefon görünümüyle yapın; gösterim türünü not edin.
- **Katılımcı telefonu büyüttü / örtüyü kaldırdı:** o set için not yazın; cevap kaydedilir,
  "not" sütununa yazılır.
- **Zaman taştı:** G2'nin (c) sorusu atlanabilir (boş = yanlış değil, eksik; raporda görünür).
