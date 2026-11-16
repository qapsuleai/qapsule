# R4 — Ölçüm düzenekleri şartnamesi (M8–M11)

Yazan: orkestratör, 2026-11-16, **hiçbir alternatif yazılmadan önce** (TASKS "düzenek ve
puanlama ölçütü önce"; K39). Bu dosyanın commit'i her alternatif dalının ilk commit'inden
öncedir (VERIFY 14). Düzeneğin uygulaması (betikler, sayfa üretici, puanlayıcı) Evre 1
bütünleştikten sonra ve alternatiflerden önce ayrı commit'lerde yapılır. Alternatif ajanlar
düzeneği değiştirmez; yalnız arayüzü (gramer kuralı + ayar adı) uygular. **Seçim kuralları
sonuç görülmeden burada yazılıdır**; sonradan değişirse değişiklik ve nedeni RESULTS'ta açıkça
yazılır (R3 sapma 1 tekrarlanmasın).

## 0. Ortak ilkeler

1. **Tasarım gramerde.** Her alternatif bir gramer/derinlik kuralı (`qp-grammar`) ve gerekirse
   şablon cümle kuralıdır (`qp-capsule::realize`); bir ayar adıyla seçilir (ör. bakış
   durumunda `face: "<ad>"`). Kabukta alternatife özel çizim kodu yok: kabuk aynı sahne
   boyama yolunu kullanır. Bir tasarım yalnız kabukta özel kodla mümkünse elenir ya da
   gramer genişletilir (ek, gerekçeli).
2. **Görüntüler gösterimden.** Persona ve eleştirmenlerin gördüğü ekranlar, gösterimin
   (`qp demo`) **sentetik** veriyle, yeniden oynatmanın **sabit anlarında** başsız tarayıcıda
   alınan ekran görüntüleridir (telefon bakışı 390×844 CSS px, DPR 2, açık tema; ayrıca koyu
   tema ve gri tonlamalı kopya). Aynı alternatif + aynı an + aynı tohum → bayt eşit görüntü.
   Görüntüler `rounds/R4/experiments/<M>/raw/screens/` altına, alternatif adı **yerine
   tohumlu karma kodla** adlandırılır; eşleme `raw/key.json`'dadır ve okuyucu dizinine
   kopyalanmaz.
3. **Körleme.** Persona ve eleştirmen ajanları tasarımın adını, kimin yaptığını, kaç
   alternatif olduğunu (eleştirmen hariç) ve önceki turların sonucunu görmez. Taze süreç:
   `claude -p --model sonnet --restricted --strict-mcp-config --permission-mode dontAsk
   --no-session-persistence --output-format json`, yalnız `Read` aracı, çalışma dizini
   yalnız görüntüleri içeren yeni geçici dizin (R3 `run_readers.sh` ile aynı yöntem). İstem
   metni, tohum, ajan kimliği, model, ham cevap ve maliyet `raw/<M>/readers/…`'de. İstemde
   alternatif adı geçmediği düzenek testiyle denetlenir (`grep` yasak sözcük listesi: dal
   adları, ajan adları, `alt`, `variant`, alternatiflerin ayar adları).
4. **Belirleyicilik.** Sayfa seçimi, sıra, etiketler tohumludur; yeniden puanlama ham
   cevaptan aynı sonucu verir. Puanlayıcı çoktan seçmeli cevapları anahtarla karşılaştırır;
   serbest metin puanlanmaz (yalnız kayıt).
5. **Sınır.** Persona ajanı insan değildir; model küçük pikseli ve beş saniyelik bakışı
   insan gibi deneyimlemez. Bu ölçümler **ön elemedir**; son söz insan testi v2'dedir
   (`rounds/R4/human-test/`). Her RESULTS bu cümleyi taşır.
6. Ölçüm başına model `sonnet` (`claude-sonnet-5-5`), en çok 5 eşzamanlı ajan.

## 1. Personalar

| Kod | Persona | İstemdeki tarif (birebir) |
|-----|---------|---------------------------|
| `F` | Geleceğin yatırımcısı | "Yirmi yaşındasın. Hisse senedi alıp satıyorsun ama teknik analiz bilmiyorsun; grafik, indikatör, mum gibi terimler sana bir şey ifade etmiyor. Telefonuna bir bildirim geldi; ekrana yalnızca birkaç saniye bakacaksın. İlk bakışta ne anladığını cevapla; uzun uzun inceleme." |
| `H` | Alışkın kullanıcı | "Yıllardır borsada işlem yapıyorsun; teknik analize, mum grafiklerine ve indikatörlere alışkınsın. Telefonunda bir piyasa uygulamasına bakıyorsun. Gördüğünü okuyup cevapla." |

İki persona da şunu bilir (istemde): "Bu bir piyasa bilgi uygulamasının ekran görüntüsü.
Uygulama tavsiye vermez." Bu son cümle **verilmez** (M8 S5 bunu ölçer).

## 2. M8 — Kapsülün kapalı yüzü

**Alternatifler:** en az üç; birbirini görmeyen ayrı alt ajanlar, ayrı dallar. Her biri
`docs/CONCEPTS.md` "Kapsülün yüzü" ve GOAL "tasarım sınırları"na uyar (tek bakış: bir başlık
cümlesi, bir işaret, en çok iki ek bilgi; gündelik dil; geçerlilik görünür; dürüstlük; TR/EN;
süre ifadesi takvime göre). Her alternatif kendi işaret yerleşimini, başlık şablonunu ve ek
bilgi seçimini yapar; işaret alfabesi `siluet` (varsayılan) ve `rozet` ile çalışmalıdır.

**Durum kümesi `S8`** (düzenek kurar; yeniden oynatmanın sentetik verisinde aranıp bulunan
anlar, bulunamayan durum düzenek dökümünden belge içi seriyle kurulur; 12 durum):

| # | Durum |
|---|-------|
| 1 | Olay, yukarı kırılım, bugün, geçerli |
| 2 | Olay, aşağı kırılım, 3 işlem günü önce (hafta sonu aşan), geçerli |
| 3 | Durum, aşırılık yukarı, **artık geçerli değil** (`no_longer`), hikâye açık |
| 4 | Durum, sıkışma, sürüyor |
| 5 | Kapandı: gerçekleşti |
| 6 | Kapandı: tutmadı (geçersiz) |
| 7 | Kapandı: süresi doldu |
| 8 | Geri çekildi |
| 9 | Lisanslı akış, yetkisiz okuyucu (rakamsız, sunucu doğruladı) |
| 10 | Az örnekli karne (n < 5) |
| 11 | 60 dakikalık periyot, "2 saat önce" |
| 12 | Dönüş aşağı, teyit edilmiş (iki aile), dün |

Her durumun **anahtarı** (ne oldu: tür+yön sade dilde; ne zaman: kova; geçerlilik; güven
kovası; tavsiye: yok) düzenekte motorun durumundan üretilir, alternatiften değil.

**Sayfa.** Kapalı yüzlerden 6'sı, telefon akışında alt alta (gösterimin akış ekranı; akış
düzeni bu deneyde sabit "zaman sırası" başvurusudur), tek ekran görüntüsü; iki sayfa (tohum
1, 2) `S8`'in 12 durumunu kapsar. Etiketler (A–F) kabuğun düzen öğesi olarak yüzün yanına
düzenek ayarıyla basılır.

**(a) Persona okuma testi.** Persona × alternatif × sayfa × 3 okuyucu (3 × 2 × 2 × 3 = 36
koşu). Her yüz için çoktan seçmeli JSON:

| Soru | Seçenekler | Doğru |
|------|------------|-------|
| S1 Ne oldu? (yön) | yukarı / aşağı / belirsiz / anlamadım | anahtar yön |
| S1b Ne oldu? (ne tür) | "fiyat önemli bir seviyeyi geçti" / "yön değişiyor" / "hareket sürüyor" / "fiyat sıkıştı, hareketsiz" / "aşırı gitti" / "büyük alım ya da satım var" / anlamadım | anahtar tür (sade karşılık tablosu düzenekte) |
| S2 Ne zaman? | bugün / dün / birkaç gün önce / bir haftadan önce / birkaç saat önce / bilmiyorum | süre kovası |
| S3 Hâlâ geçerli mi? | evet, sürüyor / hayır, artık geçerli değil / kapandı: gerçekleşti / kapandı: tutmadı / kapandı: süresi doldu ya da geri çekildi / bilmiyorum | geçerlilik + aşama |
| S4 Ne kadar güvenilir? | geçmişte çoğunlukla tuttu / yarı yarıya / çoğunlukla tutmadı / az örnek var, bilinmiyor / hiçbir bilgi göremedim | karne kovası (≥ %60, %40–60, < %40, n < 5) |
| S5 Bana ne yapmamı söylüyor? | al / sat / bekle / hiçbir şey, yalnız bilgi veriyor / bilmiyorum | "hiçbir şey" |

Ölçü: soru başına doğruluk; yüz başına **tam doğru** (S1–S5 hepsi); persona başına ayrı.
Şans düzeyi yazılır.

**(b) Hesapla** (yüzün sahne tarifinden, `SceneDesc`; tarayıcıdan değil): başlık uzunluğu
(karakter, sözcük; TR ve EN; 12 durumda en çok ve ortalama), metin yoğunluğu (yüzdeki metin
öğesi sayısı ve metin kutularının alanı / yüz alanı), işaretin alanı (px² ve yüz alanına
oranı), renk karşıtlığı (her metin ve işaret öğesinin zeminine WCAG oranı; açık ve koyu
tema), en küçük yazı boyu (CSS px). **Sert eşikler:** metin karşıtlığı ≥ 4,5:1 (14 px ve
üstü kalın / 18 px ve üstü için ≥ 3:1), işaret ve geçerlilik işareti ≥ 3:1 (WCAG 1.4.11),
en küçük yazı ≥ 11 CSS px, başlık en çok 80 karakter (TR ve EN), ek bilgi ≤ 2, gri tonlamada
yön ve geçerlilik şekille ayırt edilir (eleştirmen ölçütü 6 ve §6).

**(c) Eleştirmenler.** 3 taze ajan; her biri bütün alternatiflerin aynı iki sayfasını görür
(etiketler her ajana farklı rastgele harf: "Tasarım K/L/M", sıra tohumlu). Aşağıdaki ölçütle
her tasarıma 1–5 puan ve genel sıralama; gerekçe kayıt için (puanlanmaz):

1. Tek bakış: kapalı yüz tek bir mesaj veriyor mu?
2. Hiyerarşi: önce ne okunacağı belli mi?
3. Geçerlilik: sürüyor / artık geçerli değil / kapandı ilk bakışta ayrılıyor mu; işaret ve
   cümle aynı şeyi mi söylüyor?
4. Dürüstlük: güvenilirlik (karne, az örnek) görünür mü, kısaltılmış ama gizlenmemiş mi?
5. Gündelik dil: iç terim, jargon, "bar" yok mu; zaman gündelik mi?
6. Erişilebilirlik: renk tek başına anlam taşımıyor mu (gri kopya verilir)?
7. Tavsiye değil: ekran bir eylem önermiyor mu?
8. Estetik ve özgünlük: finans uygulaması kalıbının kopyası değil, "tek bakışta anlam"
   hedefinden mi?

**Seçim kuralı (M8, önceden):**
1. (b)'nin sert eşiklerinden birini geçemeyen alternatif elenir (hepsi geçemezse kural 1
   uygulanmaz ve bu yazılır).
2. Birincil ölçü: persona tam doğru oranı, `F` ağırlığı 2, `H` ağırlığı 1.
3. Birincil ölçüde fark < 0,05 ise eleştirmen ortalama sırası; o da eşitse daha kısa ortalama
   başlık (TR+EN).
4. İlk iki sıra gösterimde ayardan seçilebilir kalır ve insan testi v2'ye gider (GOAL 12).

## 3. M9 — Akışın düzeni

**Alternatifler:** en az iki (ör. zaman sırası; enstrümana göre gruplu; öneme göre). Sıra ve
gruplama çekirdekte (gramer/akış kuralı); kabuk yalnız alt alta dizer. "Son baktığımdan beri"
işareti gerekiyorsa çekirdek verir (bakış durumu).

**Senaryo.** Sentetik yeniden oynatma; kişi kitlesi: izleme listesi 8 enstrüman, pozisyon 2
enstrüman (biri olumsuz geçiş yaşayan), hassasiyet `balanced`. "Son bakış" anı `t0`, şimdiki
an `t1` (aradaki geçişler bilinir). Yüz tasarımı sabit: M8 seçilmeden önce koşulursa
düzeneğin başvuru yüzü (gösterimin R3 kartının en sade kısaltması), seçildikten sonra M8
birincisi; hangisi kullanıldıysa yazılır.

**Sayfa.** Akışın ardışık ekran görüntüleri (ekran 1, 2, …, en çok 6), sırayla verilir.

**Görevler** (persona × alternatif × 3 okuyucu):

| Görev | Soru | Doğru |
|-------|------|-------|
| G1 | Son baktığından beri ne değişti? Değişen enstrümanları ve değişimin yönünü listele (seçenek listesinden çoklu seçim). | `t0`–`t1` arası geçişler (anahtar) |
| G2 | Elindeki hisseler: X ve Y. Bunlarda olumsuz bir şey var mı? Hangisinde, ne? | olumsuz geçiş yaşayan pozisyon |
| G3 | En güvenilir kapsül hangisi? (etiket) | karne kovası en yüksek ve n ≥ 5 olan |

Her görevde ajan ayrıca "cevabı hangi ekranda buldun" (ekran numaraları) ve "cevap için bir
kapsülü açman gerekir miydi" (etiketler) yazar.

**Ölçüler:** doğruluk (G1: kesinlik ve duyarlılık, F1; G2, G3: doğru/yanlış); **gereken
kaydırma** = ajanın dayandığı en son ekranın sırası − 1 (doğru cevaplarda; ayrıca anahtardan
hesaplanan en az kaydırma); **gereken dokunma** = "açmam gerekirdi" dediği kapsül sayısı +
düzenin kendisinin gerektirdiği (ör. grup açma).

**Seçim kuralı (M9):** birincil: üç görevin ortalama doğruluğu (`F` 2, `H` 1); fark < 0,05
ise ortalama kaydırma + dokunma (az olan); sonra eleştirmen yok, alfabetik değil, **daha az
kural** (çekirdekte daha az parametre) olan.

## 4. M10 — Açılma geçişi (cümle → kapsül → grafik)

**Alternatifler:** en az iki. Geçişin "ortak öğeleri" çekirdekte belirlenir (T4'ün açılma
cevabı); alternatif hangi öğelerin kalacağını, nereye taşınacağını ve grafiğin nasıl
işaretleneceğini gramerle tanımlar; kabuk ara kareleri yalnız iki sahne arasında
konum/opaklık arası değerlemesiyle boyar (alternatife özel kabuk kodu yok).

**Durumlar:** `S8`'den 4 durum (1, 3, 6, 12).

**(a) Hesapla:** geçişten önceki ve sonraki sahnede ortak kalan öğe sayısı ve bunların
ekran alanı payı (sahne tariflerinden); ortak öğelerin konum kayması (px); akıştan grafiğe
gereken dokunma sayısı (protokol mesajı sayısı).

**(b) Persona:** ajan önce kapsül ekranını, sonra geçişin 2 ara karesini, sonra grafik
ekranını görür. Sorular: G1 "Grafikte işaretli yer az önceki kapsülün söylediği şeyin
neresi?" — grafik üzerine düzenekçe basılmış A–D bölge etiketlerinden biri (doğru: kapsülün
olgusunun barı/seviyesi); G2 "Grafik ekranında az önceki kapsülle aynı şeye mi bakıyorsun?"
(evet / hayır / emin değilim; doğru: evet); G3 "Grafikte kapsülün iddiası hâlâ geçerli mi?"
(S3 seçenekleri).

**Seçim kuralı (M10):** birincil G1 doğruluğu (`F` 2, `H` 1); fark < 0,05 ise G2+G3; sonra
ortak öğe alan payı (yüksek olan); sonra daha az dokunma.

## 5. M11 — "Gerçekleşti" ile "tutmadı"nın işarette ayrımı

**Alternatifler:** en az iki (R3'teki birkaç piksellik onay/çarpı başvurudur ve ayrıca
ölçülür). Sonuç işareti işaret alfabesinin kanalıdır (`qp-grammar::glyph`); hem `siluet` hem
`rozet` ile çalışmalıdır. Var olan `g1-glyphs` altınları değişmez: yeni çizim yeni ayar adıyla
seçilir; varsayılan değişikliği bütünleştirmede (S) yeni altınla yapılır.

**(a) Hesapla:** R3 M1 düzeneğinin ayırt edilebilirlik ölçüsü (`rounds/R3/experiments/
harness/` m1 measure yöntemi: CIELAB ΔE76, σ = 0,5 px bulanıklık, ±1 px kaydırma) yalnız
`realized` ↔ `failed` çiftlerinde (6 tür × 3 yön), 16/24/32 px, açık/koyu, normal +
döteranopi + protanopi + tritanopi + gri. Ana ölçü: en kötü koşulda en zayıf çift.

**(b) Persona:** 4×4 ızgarada 16 işaret (32 px, 4× en yakın komşu büyütme; sınır yazılır),
karışık sonuçlu (gerçekleşti, tutmadı, açık); her etiket için "gerçekleşti / tutmadı / hâlâ
açık / bilmiyorum". İki koşul: açıklamasız ve tek cümlelik açıklamalı (alternatif yazar, en
çok 30 sözcük). Persona × alternatif × koşul × 3 okuyucu.

**Seçim kuralı (M11):** en kötü koşul ΔE'si başvurunun (R3 onay/çarpı) altında kalan elenir;
birincil: açıklamasız persona doğruluğu (`F` 2, `H` 1); fark < 0,05 ise en kötü koşul ΔE'si.

## 6. Ortak hesap ölçüleri: renksiz okunabilirlik

Her seçilen tasarım için gri tonlamalı ekran görüntüsünde (L* kanalı) yön (yukarı/aşağı) ve
geçerlilik (sürüyor/artık değil/kapandı) çiftleri arasındaki ΔE (§5 yöntemi) ve M8 eleştirmen
ölçütü 6. Eşik: en zayıf çift ΔE ≥ başvuru (R3 `siluet` en kötü koşulu 4,60) ya da şekil
farkı (çiftler gri kopyada eleştirmenlerce ayırt ediliyor: 3/3).

## 7. Kayıt düzeni

```
rounds/R4/experiments/
  harness/            bu şartname, betikler (run_readers.sh, screens.mjs, score.py), README
  M8/ M9/ M10/ M11/
    <alt>/            yalnız alternatifin notu (DESIGN.md) — kod alternatif dalında
    raw/
      key.json        kod → alternatif eşlemesi (okuyucuya gitmez)
      screens/        sentetik veriyle ekran görüntüleri (gerçek veri yok)
      readers/<persona>_<sayfa>_r<k>/{prompt.txt,answer.json,stderr.txt,meta.json}
      critics/c<k>/{…}
      computed.json
    RESULTS.md        ölçüler, seçim kuralının uygulanışı, seçim, sınır cümlesi
```

## 8. Ekler (alternatiflerden önce; 2026-11-16, orkestratör)

Düzenek kurulurken (H-M8910) çıkan sapmalar; hiçbir alternatif yazılmadan ve okunmadan karara
bağlandı.

- **E1 (S4 kovası).** "Az örnek" kovası ürünün eşiğidir: karne `n < GLYPH_MIN_LEDGER_N` (20).
  Gerekçe: anahtar ürünün kullanıcıya söylediğini ölçmeli; ürün `n < 20`'yi "henüz az örnekli"
  diye yazar, okuyucunun buna uyması yanlış sayılmamalı. §2 tablosundaki "n < 5" bu eşikle
  değişir; S8 #10 durumu (n < 5) aynı kalır (yeni eşiğin içinde).
- **E2 (S8 #8 geri çekildi).** Sentetik oynatma geri çekme üretmez (§20.7: yalnız itiraz/veri
  düzeltmesi); bu durum M8'de yoktur, sayfa 2 beş yüzle koşar. Sonuç notunda yazılır.
- **E3 (saatlik dünya).** #1, #11, #12 düzeneğin saatlik dünyasından; M10 #1/#12 yerine gösterim
  yedekleri (#101, #112) kullanır. Sonuç notunda yazılır.
- **E4 (sayfa boyu).** M8 sayfası telefon genişliğinde (390 CSS px, DPR 2) tam boy alınır;
  "tek ekran" şartı 844 px'e sığmayan yüzlerde kaydırmalı tek görüntü demektir. Yüz yüksekliği
  hesap ölçüsü olarak kaydedilir (sert eşik değil).
- **E5 (başlık şablonu).** Her M8 alternatifi kendi başlık şablonunu `realize::parts` yapı
  taşlarından kurar (çekirdekte; arayüz H-FP ile açılır). Başvuru `r3` bugünkü kapsül cümlesini
  kullanır.
- **E6 (M8 kural 1 eşik başına; 2026-11-16, ilk alternatif tesliminden sonra, hiçbir okuma ve
  puan yokken).** Kural 1 eşik başına uygulanır. Tasarımdan bağımsız bir nedenle (alfabe ya da
  kabuk; başvuru `r3` de aynı değeri alır) **hiçbir** alternatifin geçemediği eşik o eşik için
  uygulanmaz ve sonuç notunda yazılır; öteki eşikler elemeye devam eder. Gerekçe: kapanmış
  işaretin soluk gri çizimi alfabenin kuralıdır (karşıtlık 2,06, `r3` dahil); yazılı kural bütün
  alternatifleri eler ve başlık/metin eşiklerini de boşa çıkarırdı.
- **E7 (#9 başlık ölçüsü).** Yetkisiz okuyucunun yüzü bütün tasarımlarda kabuğun düz metnidir
  (yüz şablonu çizilmez); başlık uzunluğu sert eşiği #9 dışındaki durumlarla hesaplanır, #9 ayrıca
  yazılır.
