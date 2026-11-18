# M17 alternatifi `selvi` — "konuşur gibi tek cümle"

Dal `r5/m17-selvi`. Ürün modülü `crates/qp-capsule/src/voice/selvi.rs` (+ `selvi_glossary.rs`),
düzenek bağdaştırıcısı `rounds/R5/experiments/harness/lang/tool/src/adapters/selvi.rs`.

## İlke

Başlık, bir insanın bir arkadaşına söyleyeceği **tek doğal cümledir**: *kim*, *ne yaptı*, *hangi
somut çizgiye göre* (adı ve değeri), *ne zaman*. Geçerlilik kısa ikinci parçadır ("Hâlâ geçerli.").
Karne ayrı satırda, doğal sıklıkla ve beklentinin kendisiyle konuşur. Ayrıntı aynı sesle sürer:
iç dil yok (bar sayısı, "teyit", "aile", "sinyal" yok); süre takvimle söylenir ("10 işlem günü
içinde", "dün 15:00'te"). Ekranda geçen her teknik sözcük (RSI, bant, hacim, kurum) ve karnenin
"benzer"i dokununca sözlükten açılır.

> SYN03 dün son 20 günün en yükseği olan 148,90'ı aştı. Hâlâ geçerli.
> Geçmişte 74 benzerinin 36'sında fiyat 10 işlem günü içinde yükseldi.

## Giriş noktası

```rust
qp_capsule::voice::selvi::speak(&VoiceInput {
    sentence: &SentenceInput { .. },   // ürünün cümle girdisi (hikâye, öncü/son olgu, şimdi, still_holds, current, redacted)
    parts: &parts(..),                 // yüzün yapı taşları (qp_capsule::realize::parts): when / moved / validity / ending / ledger
    calendar: &ExchangeCalendar,
    period: Period,
}) -> Voice { tr: Surfaces, en: Surfaces }
// Surfaces { headline, detail, ledger, glossary: Vec<Gloss { term, text }>, trust }
```

İki dil **birlikte** yazılır: bir yüzeyin kısa yedek biçimi seçilirse iki dilde aynı biçim seçilir;
bu yüzden TR ve EN her zaman aynı sayıları, tarihleri, saatleri ve geçerliliği taşır. `trust`
ek bir yüz çipidir ("Bu cihazda sınandı" / "Sunucu doğruladı"; bütünleştirme yüze koyabilir).
Yüzün `HeadlineParts`'ı da aynı `Parts`'tan doldurulur; ses `Parts`'ı doğrudan okur (qp-capsule
qp-grammar'a bağımlı değil).

## Yüzeyler

| yüzey | yapı | sınır |
|---|---|---|
| başlık | `{kim} {ne zaman} {çizginin adı ve değeriyle ne yaptı}.` + `{geçerlilik}.` | ≤ 80 karakter |
| ayrıntı | (1) çizgi tam haliyle: ad + değer + parametre; (2) doğduktan sonra ne değişti (son olgu, kendi rakamıyla, takvim zamanıyla); (3) ne zamana kadar geçerli ve sonuç en geç ne zaman belli — ya da nasıl kapandı | ≤ 200 karakter, ≤ 3 cümle |
| karne | doğal sıklık + süre + yön: "Geçmişte N benzerinin R'sinde fiyat H içinde yükseldi." / "Az örnek: …" / "Henüz benzeri yok, H içinde yükselirse tutmuş sayılacak." | ≤ 70 |
| sözlük | ekrandaki terimler + "benzer" (kimler sayıldı, neyin "tuttu" sayıldığı: ortalama günlük salınımın k katı, `outcome::base_params`) | ≤ 160 |

Her dedektörün çizgisi (adı, değeri, parametresi dedektörün kendi sabitinden):

| dedektör | başlık (TR) |
|---|---|
| `breakout.range` | SYN30 bugün son 20 günün en düşüğü olan 6,17'nin altına indi. |
| `extreme.rsi` | SYN25 kısa sürede çok yükseldi: RSI bugün 70'in üstüne çıktı. |
| `continuation.ma_stack` | SYN15 bugün 20 günlük ortalamasının (116,07) üstüne yerleşti. |
| `reversal.rsi_divergence` | SYN26 bugün 110,03 ile daha derin dip yaptı, RSI yukarıda kaldı. |
| `squeeze.bandwidth` | SYN07 bugün 120 günün en dar aralığına sıkıştı: bant 7,78–8,12. |
| `flow.volume_surge` | SYN05 bugün 20 günlük ortalamanın 3,1 katı hacimle yükseldi. |
| `flow.institutional` | Kurumlar bugün SYN19'da ortalamanın 2 katından fazla net aldı. |

## Örnekler (düzenek dökümünden, sentetik veri; `out/selvi/dump.json`)

**Kırılım aşağı, doğdu, geçerli, az örnek** (s100)
- TR başlık: SYN30 bugün son 20 günün en düşüğü olan 6,17'nin altına indi. Hâlâ geçerli.
- TR ayrıntı: Bugün kapanış 6,16 oldu, önceki 20 işlem gününün en düşüğü 6,17 idi. Kapanış 6,17'nin altında kaldıkça geçerli, sonuç en geç 10 işlem günü içinde belli olur.
- TR karne: Az örnek: 2 benzerinin 1'inde fiyat 10 işlem günü içinde düştü.
- EN: SYN30 broke below its prior 20-day low of 6.17 today. Still holds. / Today, it closed at 6.16, below the prior 20-day low of 6.17. It holds while the close stays below 6.17, and the result is due within 10 trading days. / Few past cases: the price fell within 10 trading days in 1 of 2.

**RSI aşırılığı, artık geçerli değil** (s040)
- TR: SYN29'da RSI cuma günü 70'in üstüne çıktı. Artık geçerli değil. / Cuma günü RSI 70,1 oldu ve 70 çizgisini geçti. Bugün RSI 70'in altına döndü. / Az örnek: 2 benzerinin 2'sinde fiyat 10 işlem günü içinde yükseldi.
- EN: SYN29's RSI went above 70 on Friday. No longer holds. / On Friday, RSI reached 70.1, past the 70 line. RSI went back below 70 today. / Few past cases: the price rose within 10 trading days in 2 of 2.
- Sözlük: RSI — Göreceli güç endeksi: son 14 işlem günündeki kazançları kayıplarla tartar, 0 ile 100 arası. 70 üstü hızlı yükseliş, 30 altı hızlı düşüş sayılır.

**Ortalama dizilimi, kapandı: gerçekleşti** (s172)
- TR: SYN20 11 işlem günü önce 20 günlük ortalamasının üstüne yerleşti. Gerçekleşti. / 11 işlem günü önce fiyat 238,02, 20 günlük ortalama 227,15, 50 günlük ortalama 212,13 idi: sıra yukarı doğru. Bugün fiyat yeterince yükseldi ve gerçekleşti. / Henüz benzeri yok, 20 işlem günü içinde yükselirse tutmuş sayılacak.
- EN: SYN20 settled above its 20-day average 11 trading days ago. Played out. / 11 trading days ago, the price was 238.02, with the 20-day average at 227.15 and the 50-day at 212.13, lined up upward. The price rose far enough today, so it played out. / No past cases yet, it counts if it rises within 20 trading days.

**Uyumsuzluk, doğdu** (s058)
- TR: SYN26 bugün 110,03 ile daha derin dip yaptı, RSI yukarıda kaldı. Hâlâ geçerli. / Önceki dipte fiyat 110,23 ve RSI 40,3 idi, yeni dipte fiyat 110,03 ve RSI 42,3: düşüş hız kesiyor. Sonuç en geç 15 işlem günü içinde belli olur.
- EN: SYN26 made a lower low at 110.03 today, but RSI stayed higher. Still holds. / At the previous low the price was 110.23 with RSI at 40.3, at the new low 110.03 with RSI at 42.3: the fall is losing pace. The result is due within 15 trading days.

**Sıkışma** (s223)
- TR: SYN07 bugün 120 günün en dar aralığına sıkıştı: bant 7,78–8,12. Hâlâ geçerli. / Bugün bant 7,78 ile 8,12 arasına daraldı, bu son 120 günün en darı. Bant yeniden açılana kadar geçerli, sonuç en geç 20 işlem günü içinde belli olur. / Henüz benzeri yok, 20 işlem günü içinde bir yöne kırılırsa tutacak.
- EN: SYN07 hit its tightest band in 120 days today: 7.78–8.12. Still holds. / Today, the band narrowed to 7.78–8.12, its narrowest in 120 days. It holds until the band widens again, and the result is due within 20 trading days. / No past cases yet; counts if it breaks out within 20 trading days.

**Hacim** (s052)
- TR: SYN05 bugün 20 günlük ortalamanın 3,1 katı hacimle yükseldi. Hâlâ geçerli. / Bugün 1.321.184.727 adet hisse el değiştirdi, 20 günlük ortalama 422.606.743 adet, eşik bunun 2,5 katı. Sonuç en geç 10 işlem günü içinde belli olur.
- EN: SYN05 rose on 3.1 times its 20-day average volume today. Still holds. / Today, 1,321,184,727 shares changed hands against a 20-day average of 422,606,743, and the mark is 2.5 times that. The result is due within 10 trading days.

**Kurumsal akış, yetkili okur** (s034)
- TR: Kurumlar bugün SYN07'de ortalamanın 2 katından fazla net sattı. Hâlâ geçerli. / Bugün en çok net işlem yapan kurumun net satışı 6.565.421 TL oldu, son 5 günün ortalaması 2.457.846 TL, eşik bunun 2 katı. Sonuç en geç 10 işlem günü içinde belli olur. Bunu sunucu doğruladı. / Geçmişte 21 benzerinin 4'ünde fiyat 10 işlem günü içinde düştü.
- EN: Institutions net-sold SYN07 at over 2× their average today. Still holds. / Today, the top institution's net selling was TRY 6,565,421 against a 5-day average of TRY 2,457,846, and the mark is 2 times that. The result is due within 10 trading days. The server verified this. / The price fell within 10 trading days in 4 of 21 past cases.

**Kurumsal akış, yetkisiz okur (rakamsız, A32)** (s004)
- TR: Kurumlar bugün SYN19'da her zamankinden çok net aldı. Hâlâ geçerli. / En çok net işlem yapan kurumun net alımı son günlerdeki olağan düzeyinin çok üstündeydi. Sonuç önümüzdeki günlerde belli olur. Rakamlar kurum akışı verisi yetkisiyle açılır, bunu sunucu doğruladı. / Henüz benzeri yok, fiyat yükselirse tutmuş sayılacak.
- EN: Institutions net-bought SYN19 far more than usual today. Still holds. / The top institution's net buying was far beyond its recent usual level. The result is due in the coming days. The figures open with a broker-flow data entitlement, and the server verified it. / No past cases yet, it counts if the price rises.

**Saatlik, kapandı: süresi doldu, ek olgu** (s351)
- TR: SYN01 2 işlem günü önce son 20 saatin en düşüğünün altına indi. Süresi doldu. / Önceki 20 işlem saatinin en düşüğü 34,52, kapanış 34,26. Dün 15:00'te fiyat da önceki 20 saatin en düşüğünün (34,05) altına indi. 10 işlem saati içinde yeterli hareket olmadı, az önce süresi doldu.
- EN: SYN01 broke below its prior 20-hour low 2 trading days ago. Ran out of time. / Prior 20-hour low 34.52, close 34.26. The price also broke below its prior 20-hour low (34.05) yesterday at 15:00. No large enough move came within 10 trading hours, so it ran out of time just now.

## Bilinçli seçimler

1. **Çizgi başlıkta, sayı yerinde.** "Önemli bir eşik" yerine çizginin kendisi: "son 20 günün en
   yükseği olan 148,90". Başlığın ilk sözcüğü kim (hisse ya da kurumlar), sonra zaman, sonra fiil
   ve çizgi — insanın söyleyiş sırası.
2. **Uzunluk sınırı sabit, cümle esner.** Her dedektörün başlığı en ayrıntılıdan en kısaya 3–4
   biçim taşır; ilk sığan seçilir (iki dilde birlikte). Kapanmış kapsülde "Kapandı: tutmadı." sığmazsa
   kısa biçim "Tutmadı." gelir (geçerlilik yine söylenir). Ayrıntıda önce "ne değişti" cümlesi
   korunur, çizgi ve koşul cümleleri kısa biçimlerine iner. Değeri başlığa sığmayan durumlarda (ör.
   saatlik "dün 15:00'te" ya da kapanmış akışlar) değer ayrıntıdadır; çizginin adı, değeri ve
   parametresi ekranda (başlık + ayrıntı) her durumda vardır.
3. **Karnede beklentinin kendisi.** "Beklendiği gibi" yok; neyin tutmuş sayıldığı yazılır: süre
   ("10 işlem günü içinde") ve yön ("yükseldi", yönsüzde "bir yöne kırıldı"). Doğal sıklık ("74
   benzerinin 36'sında"); az örnekte "Az örnek:" öne konur; karnesizde ölçüt söylenir. Ölçütün
   tam tanımı (ortalama günlük salınımın k katı, önce ters yöne dönmeden) sözlükte "benzer"
   girdisindedir; k ürünün sonuç kuralından (`outcome::base_params`) okunur, uydurulmaz.
4. **Geçerlilik tek kaynaktan.** Başlığın ikinci parçası ve ayrıntının durum cümlesi yalnız
   `Parts::validity` ve `Parts::ending`'den yazılır; ayrıntı aynı geçerlilik cümlesini tekrarlamaz,
   nedenini söyler ("Kapanış 148,90'ın üstünde kaldıkça geçerli", "Bugün RSI 70'in altına döndü").
5. **Doğduktan sonra ne oldu, kendi rakamıyla.** Güçlenme/teyit/zayıflama iç adlarıyla değil, son
   olgunun kendisiyle anlatılır: "Bugün hacim de 20 günlük ortalamanın 11,3 katına çıktı.", "Dün
   bunun tersine kurumlar her zamankinden çok net sattı." Lisanslı bir son olgunun rakamı yalnız
   okur öncü olgu için aynı yetkiye sahipse yazılır.
6. **Yetkisiz okur (K33/A32).** Değerler hiç okunmaz (`Cx::ev` yetkisizde `None`); zaman rakamsız
   süre ifadesiyle, geçerlilik aynen, karne nitel ("çoğunda", "yaklaşık yarısında", "azında"; az
   örnek ve karnesiz ayrı). Ayrıntı "Rakamlar kurum akışı verisi yetkisiyle açılır, bunu sunucu
   doğruladı." der: neyin, hangi yetkiyle açıldığı ve kimin doğruladığı.
7. **Sözlük az ve gerekli.** Yalnız ekranda geçen teknik terimler + "benzer" (ekranların %22'sinde
   tek girdi, %73'ünde iki, %6'sında üç). Sözlük metinleri de iç terim ve tavsiye taramasından geçer.
8. **TR ek kuralı.** Sayı ve sembol ekleri ürünün `morph` modülüyle (okunuşa göre): "148,90'ı",
   "70'in", "30'un", "36'sında", "SYN03'te" (sondaki rakamlar sayı gibi okunur: "üç" → "-te").

## Kendi taraması (düzenek, 531 durum × 2 dil)

| vague | advice | repeat | internal | unglossed | length | threshold | ledger_expect | contradiction | equivalence | redacted_digit | determinism | empty |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

Uzunluk (ortalama / en çok): başlık TR 73 / 80, EN 71 / 80; ayrıntı TR 159 / 200, EN 165 / 200;
karne TR 66 / 70, EN 63 / 67. Başlıkta çizginin bir sayısı (değer ya da parametre) bulunan yetkili
durumlar (EN): kırılım 102/108, RSI 58/58, ortalama 76/97, hacim 48/53, sıkışma 19/25, uyumsuzluk
18/60, kurumsal 22/66; kalanlarda sayı ayrıntıdadır.

## Sınırlar ve açık noktalar

- Başlık ortalaması sınıra yakın (73/80): tek bakış için uzun olabilir; kör okuyucu ölçüsü bunu
  söyleyecek. Kısa biçim sırası deterministiktir ama hangi biçimin seçildiği zaman ifadesinin
  uzunluğuna bağlıdır (aynı kapsül "bugün" iken değerli, "2 işlem günü önce" iken değersiz başlık
  alabilir).
- Hacim ve kurumsal akış birimleri sentetik akışta büyük sayılar (milyonlar); gerçek veride
  "lot"/"TL" ve kısaltma ("6,6 milyon TL") düşünülebilir — sayı eşleşmesi kuralı nedeniyle bu sürümde
  tam sayı yazılır.
- "Beklenen" sonucun eşiği (k × ortalama salınım) yalnız sözlükte sayıyla; ayrıntıdaki kapanış
  cümlesi "yeterince yükseldi" der. Hedef fiyatı yazmak için doğuştaki ATR'nin ürün tarafından
  hikâyeye eklenmesi gerekir (bkz. rapor önerileri).
- Yönsüz hacim (açılış = kapanış) "yatay kapandı" der; sentetik veride nadir.
