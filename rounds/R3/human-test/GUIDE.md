# Uygulama yönergesi (görevli için)

Amaç: iki işaret alfabesini (`siluet` = **Alfabe M**, `rozet` = **Alfabe N**) ve şablon
cümlelerini gerçek insanlarla sınamak. Yapay zekâ ön elemesi `siluet`'i önerdi
(`experiments/M1/RESULTS.md`); K39 gereği son söz bu testtedir. Katılımcıya alfabelerin
gerçek adları, hangisinin önerildiği ve testin amacı dışında bir şey söylenmez.

## Özet

| | |
|---|---|
| Süre | Kişi başına **yaklaşık 16 dakika** (giriş 1,5 + Bölüm 1 5,5 + Bölüm 2 5 + cümleler 3 + tercih 1) |
| Kişi | 5–10; iki grup: **alışkın kullanıcı** (grafiğe düzenli bakan ya da işlem yapan) ve **genç, işlem yapmayan** (yaklaşık 18–30 yaş). Mümkünse iki grupta eşit sayı. |
| Sıra | Kitapçık **A** (M önce) ve **B** (N önce). Her grupta geliş sırasına göre A, B, A, B … dağıtılır; böylece her grupta iki sıra dengelenir. |
| Malzeme | Kişi başına bir `form.html` çıktısı (3 sayfa), bir kalem; kâğıt gösterimde kişi başına kitapçık (`kitapcik-A.html` ya da `kitapcik-B.html` çıktısı, 9 sayfa, **renkli**); ekran gösteriminde telefon. Kronometre. Oturum günlüğü (bir kâğıt). |
| Görevli | Bir kişi yeter. Oturum birebirdir; aynı anda birden çok katılımcı ise birbirini görmeyecek şekilde oturur. |

## Hazırlık

1. **Katılımcı numarası:** P01, P02, … sırayla. Grup ve kitapçık numarayla birlikte
   günlüğe yazılır. Ad, telefon, e-posta, yaş, iş yeri **yazılmaz**.
2. **Kâğıt gösterim:** kitapçığı tarayıcıda açıp yazdırın: A4, **renkli**, ölçek
   **%100** ("sayfaya sığdır" kapalı), kenar boşlukları varsayılan. Her bölüm ayrı sayfadadır
   (9 sayfa). Yazdırınca etiketin yanındaki gerçek boyutlu işaret yaklaşık 6,4 mm, büyütülmüşü
   12,7 mm çıkar; cetvelle bir işareti ölçüp denetleyin. Formu da (`form.html`, 3 sayfa)
   aynı ayarla yazdırın. Siyah-beyaz baskı **kullanılmaz** (yön rengi de taşır).
3. **Ekran gösterimi (tercih edilen):** telefonda `kitapcik-A.html` ya da `kitapcik-B.html`
   açılır. Dosya `pages/` klasöründeki görselleri göreli yolla yükler; klasörün tamamını
   telefona kopyalayın ya da bilgisayarda `python3 -m http.server` ile sunup aynı ağdaki
   telefondan açın. Tarayıcı yakınlaştırması %100, parlaklık normal, gece modu kapalı.
   Katılımcı telefonu **kol mesafesinde** (yaklaşık 35–45 cm) tutar ve **iki parmakla
   büyütmez**. Bu durumda etiketin yanındaki küçük işaret ürünün gerçek boyutudur (24 CSS piksel).
   Masaüstü ekranda: tarayıcı %100, yaklaşık 60 cm uzaklık.
   Ekran gösteriminde de sözlük sayfasını (kitapçığın 1. sayfası) kâğıda basıp masaya
   koyun; katılımcı ona bakmak için ekranı geri kaydırmasın.
4. Sayfaları **görevli** çevirir ya da kaydırır (ekranda her bölüm bir ekran boyundadır ve
   kesik çizgiyle ayrılır). Katılımcı ileri sayfaya bakmaz.
5. Bir deneme oturumu: ilk katılımcıdan önce kendiniz bir kez baştan sona geçin; süreleri
   aşağıdaki tabloyla karşılaştırın.

## Onam ve gizlilik (giriş; sözlü, aynen okunur)

> "Bu kısa bir deneme; yaklaşık 15 dakika sürüyor. Bir uygulama için tasarlanan küçük
> işaretlerin ne kadar anlaşıldığını ölçüyoruz. Ölçülen siz değilsiniz, işaretler. Adınızı
> ya da sizi tanıtacak bir bilgiyi almıyoruz; formda yalnız bir numara ve iki gruptan
> hangisinde olduğunuz yazacak. Cevaplar yalnız toplu olarak kullanılacak. İstediğiniz an
> bırakabilirsiniz; bıraktığınızda o ana kadarki cevapları ister kullanırız ister
> sileriz, siz seçersiniz. Başlayalım mı?"

Katılımcı "evet" demezse oturum yapılmaz. Bırakan katılımcının tercihi günlüğe yazılır.

## Akış ve söylenecekler

Süreler yönlendiricidir. Süre dolduğunda "Tahmin edip ilerleyelim" denir; katılımcı satırı
bitirince geçilir. Boş kalan satırlar yanlış sayılır. Her bölümün başlangıç ve bitiş
saatini günlüğe yazın.

| # | Adım | Süre | Kitapçık sayfası | Görevlinin söylediği (aynen) |
|---|---|---|---|---|
| 0 | Giriş, onam, başlık | 1:00 | — | Onam metni. Sonra: "Formun üstüne numaranızı ben yazıyorum; grubunuzu ve kitapçık harfini birlikte işaretleyelim." |
| 1 | Sözlük | 0:30 | 1 | "Bu sayfa yalnız sözcüklerin anlamını veriyor: altı tür, yön, güç ve aşama. İşaretlerin nasıl çizildiğini söylemiyor; onu siz tahmin edeceksiniz. Bu sayfa masada açık kalabilir." (Kâğıtta sözlük sayfası kitapçıktan ayrılıp yanda tutulur; ekranda gerekirse görevli sözlüğe geri kaydırır.) |
| 2 | Bölüm 1 · 1. adım (açıklamasız) | 2:00 | 2 | "Burada 16 işaret var, şimdi yalnız üstteki 8'ini, A1'den B4'e kadar cevaplayın. Her işaretin yanında gerçek boyutu, altında büyütülmüşü var; önce küçüğe bakın. Her satırda tür, yön, güç, aşama ve ne kadar emin olduğunuz. Bilmiyorsanız tahmin edin ve eminliğe düşük puan verin." |
| 3 | Bölüm 1 · 2. adım (açıklama) | 0:45 + 1:30 | 3 | "Şimdi bu alfabenin kendi kısa açıklamasını okuyun. Sonra alttaki 8 işareti, C1'den D4'e kadar cevaplayın. Önceki cevaplarınızı değiştirmeyin." |
| 4 | Bölüm 1 · 3. adım (öğrenme) | 0:30 + 0:45 | 4 | "Burada beş örnek var, doğru cevapları yanında yazıyor. Yarım dakika inceleyin. Sonra en alttaki dört yeni işareti, L1'den L4'e kadar cevaplayın." |
| 5 | Bölüm 2 · 1.–3. adım | 5:00 | 5–7 | 2–4. adımlardaki cümlelerin aynısı, "Bölüm 2" diyerek. Başta: "Şimdi başka bir alfabe; kurallarını yeniden tahmin edin." |
| 6 | Bölüm 3 (cümleler) | 3:00 | 8 | "Bunlar uygulamanın yazdığı cümleler. Her biri için formda: ne oldu (tür ve yön), ne zaman oldu (geçmişte, şu an sürüyor ya da bir koşula bağlı) ve hâlâ geçerli mi (evet, hayır ya da cümleden anlaşılmıyorsa bilinmiyor). Geçmişte ise ve cümle söylüyorsa kaç bar önce olduğunu yazın. Bar, grafikteki bir mum, yani bir zaman dilimi." |
| 7 | Bölüm 4 (tercih) | 1:00 | 9 | "Son üç soru: hangi alfabe daha kolay okundu; hangi işaretler karıştı; eklemek istediğiniz bir şey. İki okuma sayfası hatırlatma için burada." |
| 8 | Kapanış | 0:15 | — | "Teşekkürler. Doğru cevapları ve sonuçları şimdi konuşmayalım; sonraki katılımcıları etkilemesin." |

Toplam yaklaşık 16 dakika. İlk katılımcılarda 1. bölüm daha uzun sürebilir; bölüm 2'de
katılımcı biçime alışmıştır (sıra etkisi; A/B dengesi bunu iki alfabe arasında eşitler).

## Görevlinin söylememesi gerekenler

- İşaretlerin kuralı, ipucu ya da yorumu ("şu ok yönü gösteriyor", "gri olanlar bitmiş").
  Soru gelirse yalnız: **"Sözlükte ne yazıyorsa o. Emin değilseniz tahmin edin."**
- Bir cevabın doğru ya da yanlış olduğu (oturum sırasında hiçbir geri bildirim yok;
  baş sallama, "hmm" de yok).
- Alfabelerin gerçek adları (`siluet`, `rozet`), hangisinin "bizim seçtiğimiz" ya da
  "yeni" olduğu, yapay zekâ testinin sonucu.
- Cümleler için: cümlenin anlamı, "bar"ın tanımı dışında bir açıklama, piyasa yorumu.
- Bir alfabeyi övmek ya da yermek; tercih sorusunda yönlendirmek.
- Önceki katılımcıların cevapları.

Katılımcı A–B satırlarına açıklamadan sonra geri dönmeye başlarsa: "Önceki adımın
cevaplarını değiştirmeyelim." denir ve günlüğe not edilir.

## Cevapların kaydı

1. Oturum biter bitmez formun köşesine katılımcı numarası, kitapçık ve gösterimin
   işaretli olduğunu denetleyin; günlüğe süreleri ve dikkat çeken bir şeyi (sorulan
   sorular, geri dönme, telefonun büyütülmesi) yazın.
2. Bütün oturumlar bitince formları `FORM.md`'deki biçimde `answers.csv`'ye geçirin
   (bir satır = bir madde; kodları olduğu gibi yazmak yeterli). Geçirirken `key.json`'a
   bakmayın.
3. Puanlama: `python3 -I score.py answers.csv > results.md` (bkz. `SCORING.md`).
   Uyarılar bölümünü okuyun; tanınmayan cevapları formdan denetleyin.
4. Kâğıt formlar ve CSV kişisel veri içermez, yine de depoya eklenmeden önce Arkin'e
   sorulur. Formlar CSV denetlendikten sonra saklanabilir ya da imha edilebilir.

## Sık karşılaşılan durumlar

- **Katılımcı "aşama"yı anlamadı:** sözlükteki aşama satırlarını birlikte yeniden
  okuyun; başka açıklama yok.
- **Telefonda görsel kaydı / küçüldü:** sayfa 360 CSS piksel genişliğindedir, telefonda
  küçültülmeden sığar. Yatay kaydırma çubuğu çıkarsa tarayıcı yakınlaştırmasını %100'e
  getirin.
- **Zaman taştı:** bitirin, taşmayı günlüğe yazın; sonraki katılımcıda "tahmin edip
  ilerleyelim" uyarısını daha erken yapın.
- **Katılımcı renk görmesi, gözlüğü gibi sağlık bilgisi verirse:** teşekkür edilir, teste
  devam edilir, **hiçbir yere yazılmaz** (gruptan başka kişisel bilgi toplanmaz). Renk
  körlüğü etkisi ayrıca hesapla ölçüldü (M1 (a), döteranopi/protanopi/tritanopi/gri).
