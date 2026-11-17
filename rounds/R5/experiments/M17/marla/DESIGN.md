# M17 alternatifi `marla` — beş sorunun sırası ve bağlam

Kod: `crates/qp-capsule/src/voice/marla.rs` (giriş noktası `speak` / `speak_both`),
sözlük `crates/qp-capsule/src/voice/marla_glossary.rs`, düzenek bağdaştırıcısı
`rounds/R5/experiments/harness/lang/tool/src/adapters/marla.rs`. Deterministik şablon
dilbilgisi; dil modeli, saat, rastgelelik yok. Var olan cümleler (`r3`, `vadrel`) ve dökümler
değişmedi; dil adıyla seçilen yeni bir modül.

## İlke

`NORTH_STAR.md`'deki beş soru dilin iskeleti. Her yüzey bir (en çok iki) soruya cevap verir,
hep aynı sırada, bir başkasını tekrar etmeden:

| Yüzey | Soru | Ne söyler |
|---|---|---|
| Başlık | **Ne değişti?** (+ ne zaman, hâlâ geçerli mi) | sade değişim + eşiğin adı ve değeri; iki küçük ek: zaman, geçerlilik |
| Ayrıntı | **Neden?** (+ şimdi ne durumda) | arkasındaki sayılar, her biri okurun tutabileceği bir başvuruya göre |
| Karne | **Ne kadar emin? Daha önce oldu mu?** | beklentinin kendisi, sayılmış: "Benzer 20 durumun 11'inde fiyat 10 işlem günü içinde yükseldi." |
| Sözlük | — | ekrandaki her terim; "tuttu"nun ne demek olduğu ve yazı tura taban oranı dahil |

"Beni ilgilendiriyor mu?" okurun elindekileri bilmeyi gerektirir; dil bunu tahmin etmez (öneri
aşağıda).

**Bağlam kuralı:** her sayı bir başvuruyla söylenir, göreli ve mutlak birlikte:
eşiğe uzaklık (değer + yüzde: "kapanış 150,20, eşiğin %0,9 üstünde"), neyin uç noktası
olduğu ("son 20 günün en yüksek fiyatı"), neyin katı olduğu ("20 günlük ortalamanın 2,5 katı"),
şimdiki değer ile eşik ("şimdi 151,00, eşiğin %1,4 üstünde"), sonucun ne zaman belli olacağı
("sonuç en geç 7 işlem günü içinde belli olacak"). Süre her yerde takvimden gelen süre ifadesi;
bar sayısı yok. Karne yüzde değil doğal sıklık söyler ("20 durumun 11'inde"; RESEARCH F3).

## Örnekler (sentetik veri; düzenek dökümünden)

**Kırılım yukarı, doğdu, geçerli** (`s117`)
- TR: SYN20 son 20 günün en yüksek fiyatını (227,08) aştı · bugün, hâlâ geçerli
  / O gün kapanış 229,81 oldu, eşiğin %1,2 üstünde. Sonuç en geç 10 işlem günü içinde belli olacak.
  / Benzer 20 durumun 11'inde fiyat 10 işlem günü içinde yükseldi.
- EN: SYN20 rose above its 20-day high (227.08) · today, still holds
  / That day it closed at 229.81, 1.2% above the high. The outcome is due within 10 trading days.
  / In 11 of 20 similar cases the price rose within 10 trading days.

**Kırılım yukarı, artık geçerli değil** (`s119`)
- TR: … · dün, artık geçerli değil / O gün kapanış 65,43 oldu, eşiğin %0,5 üstünde. Şimdi 64,63
  (eşiğin %0,7 altında), sonuç en geç 9 işlem günü içinde belli olacak.
- EN: … · yesterday, no longer holds / … Now 64.63 (0.7% below it), the outcome is due within 9 trading days.

**RSI aşırılığı, zayıfladı, artık geçerli değil** (saatlik, `s355`)
- TR: SYN09 hızla yükseldi: RSI 70'i aştı · bir saat önce, artık geçerli değil / RSI o saat
  68,7'den 70,1'e çıktı. Sonuç en geç 9 işlem saati içinde belli olacak. Az önce zayıfladı: RSI
  62,6 ile 70'in altına indi. / Az örnek: 1 benzerin 1'inde 10 işlem saatinde daha da yükseldi.
- EN: SYN09 rose fast: RSI went above 70 · an hour ago, no longer holds / That hour RSI went from
  68.7 to 70.1. The outcome is due within 9 trading hours. Weakened just now: RSI moved back below
  70 at 62.6. / Few cases: it rose further in 1 of 1 within 10 trading hours.

**Uyumsuzluk (dönüş) yukarı, kapandı: gerçekleşti** (`s072`)
- TR: SYN26 yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı · bugün gerçekleşti / Fiyat
  önceki dibin (110,23) altına, 110,03'e indi, RSI ise 40,3'ten 42,3'e çıktı. 2 işlem günü önce
  başladı, fiyat 15 işlem günü dolmadan yukarı döndü.
- EN: SYN26 could turn up: new price low, not matched by RSI · played out today / The price fell
  below the previous low (110.23) to 110.03, while RSI rose from 40.3 to 42.3. It began 2 trading
  days ago, and the price turned up within 15 trading days.

**Ortalama dizilimi yukarı, ikinci veri türü** (`s147`)
- TR: SYN15 20 günlük ortalamanın üstünde yükselişe geçti · bugün, hâlâ geçerli / Kapanış
  (120,40) 20 günlük ortalamanın (116,07), o da 50 günlüğün (101,08) üstündeydi. Bugün başka bir
  ölçü de aynı yönü gösterdi: kurumsal net alım ortalamanın iki katını aştı.
- EN: SYN15 turned up above its 20-day average · today, still holds / The close (120.40) was above
  the 20-day average (116.07), itself above the 50-day (101.08). Today, a second measure pointed
  the same way: institutional net buying topped two times its average.

**Sıkışma, zayıfladı** (`s225`)
- TR: SYN30: 20 günlük bant 120 günün en darı · 7 işlem günü önce, artık geçerli değil / Bant
  4,11 ile 4,29 arasında (genişliği fiyatın %4,3 kadarı). Sonuç en geç 13 işlem günü içinde belli
  olacak. Bugün zayıfladı: bant yeniden genişledi.
- EN: SYN30: 20-day band at a 120-day low · 7 trading days ago, no longer holds / The band runs
  from 4.11 to 4.29 (4.3% of the price wide). The outcome is due within 13 trading days. Weakened
  today: the band widened again.

**Hacim, kapandı: gerçekleşti** (saatlik, `s328`)
- TR: SYN01: hacim ortalamanın 2,5 katını aştı, fiyat düştü · az önce gerçekleşti / Hacim
  8.090.474, son 20 saatin ortalaması 2.981.328: 2,7 katı. 2 saat önce başladı, fiyat 10 işlem
  saati dolmadan düştü.
- EN: SYN01: volume over 2.5x average, price fell · played out just now / Volume was 8,090,474
  against a 20-hour average of 2,981,328: 2.7 times. It began 2 hours ago, and the price fell
  within 10 trading hours.

**Kurumsal akış, yetkili okur** (`s266`)
- TR: SYN05: kurumsal satım ortalamayı 2 kat aştı · bir saat önce, hâlâ geçerli / Net satım
  1.289.130, son 5 saatin ortalaması 248.355: 5,2 katı (sunucu doğruladı). Az önce güçlendi:
  kurumsal net satım ortalamanın 2,6 katına çıktı.
- EN: SYN05: institutions sold over 2x average · an hour ago, still holds / Net selling was
  1,289,130 against a 5-hour average of 248,355: 5.2 times (verified by the server). …

**Kurumsal akış, yetkisiz okur (rakamsız, A32)** (`s014`)
- TR: SYN09: kurumsal net satım beş günlük ortalamanın iki katını aştı · bugün tutmadı /
  Tutarlar lisanslı veridir: sunucu hesapladı ve doğruladı, kurumsal akış veri yetkisiyle
  görünür. Birkaç işlem günü önce başladı, fiyat on işlem günü dolmadan ters yöne aynı ölçüde gitti.
  / Henüz kapanmış benzer durum yok, ne sıklıkla tuttuğu bilinmiyor.
- EN: SYN09: institutions sold twice the usual · did not play out today / The amounts are
  licensed, computed and verified by the server, shown with institutional flow access. It began a
  few trading days ago, and the price moved as far the other way within ten trading days.

**Sözlük örneği** (`benzer durum`): "Aynı tür ve yönde, bundan önce kapanmış iddialar. Fiyat
süresinde olağan oynaklığı kadar o yöne gittiyse tuttu sayılır; rastgele bir gün de yarı yarıya
tutar." — karnenin sayısını okuyacak taban oranı (yazı tura) burada.

## Bilinçli seçimler

1. **Eşik başlıkta, adıyla ve değeriyle.** Bildirimde (yüz: karne + başlık) "hangi eşik, kaç
   lira" sorusu ayrıntı açılmadan cevaplanır (K49). Değeri olmayan dedektörlerde (uyumsuzluk)
   adı başlıkta, değerleri ayrıntıda.
2. **Sade fiil önce, terim sonra.** "kısa sürede çok yükseldi: RSI 70'i aştı" — teknik analiz
   bilmeyen okur ilk yarıyı, alışkın okur ikinci yarıyı okur. Teknik terimler (RSI, bant,
   hacim, kurumsal) sözlükte; "kırılım, destek, direnç, uyumsuzluk, sıkışma, karne" hiç
   kullanılmaz. İç terim (bar, teyit, aile, olgu, sinyal…) hiçbir yüzeyde yok.
3. **Yüz ile ayrıntı aynı yapı taşlarından.** Geçerlilik, "ne zaman", son hareket ve sonuç
   yalnız `Parts`'tan (yüzün yapı taşları) okunur; ayrıntı yüzle çelişemez (A33). Şimdiki değer
   eşiğe göre söylenirken taraf geçerlilikle uyuşmazsa cümle hiç kurulmaz (ortalama gibi kayan
   eşiklerde "şimdi" değeri eşiğe göre karşılaştırılmaz).
4. **Kapanmış kapsülde başlık kapanışı söyler** ("· bugün gerçekleşti", "played out today"):
   yeni haber kapanıştır; doğuşun zamanı ayrıntıya geçer ("2 işlem günü önce başladı, …").
5. **Karne beklentinin kendisidir:** sayı + süre + yön ("10 işlem günü içinde yükseldi";
   dönüşte "yukarı döndü", aşırılıkta "daha da yükseldi", sıkışmada "dar aralıktan çıktı").
   "beklendiği gibi" yok. Az örnekte "Az örnek:" önde; karne yoksa "henüz kapanmış benzer durum
   yok, ne sıklıkla tuttuğu bilinmiyor".
6. **"Tuttu"nun tanımı ve taban oranı sözlükte** (oynaklık bariyeri simetrik: rastgele bir gün
   de yarı yarıya tutar). Karne sayısını okumak için gereken bağlam budur; karne satırına
   sığmaz (70 karakter).
7. **Ayrıntı "şimdi"yi ve sonucun ne zaman belli olacağını söyler** ("sonuç en geç 7 işlem günü
   içinde belli olacak"): "artık geçerli değil" ile "kapandı" arasındaki fark böylece açık.
8. **Uzunluk sınırları iki dilde birlikte** tutulur: en zengin biçimden başlayıp sığana kadar
   kısaltılır (başlıkta 6 katman; ayrıntıda 8 bileşim; karnede 3). Başlıkta diller ayrı katman
   seçebilir ama yalnız aynı sayıları ve ikisi de zamanı taşıyorsa; aksi halde ortak katman.
   "Zaman" düşürmek son çare (düzenek durumlarında hiç gerekmedi); kısa geçerlilik ("sürüyor")
   uzun olandan ("hâlâ geçerli") sonra gelir (531 durumun 26'sında).
9. **Yetkisiz okur (A32):** aynı iskelet, tek rakam yok. Dedektörün kamuya açık kuralı sözcükle
   ("beş günlük ortalamanın iki katı"), zaman belirsiz süre ifadesiyle, karne nitel; ayrıntı
   tutarların hangi yetkiyle açıldığını ve sunucunun hesaplayıp doğruladığını söyler.
10. **Güven ayrımı:** sunucu olgusunda ayrıntı "(sunucu doğruladı)" der; sözlük "bu cihaz
    lisanslı veriyi kendisi sınayamaz" der. Lisanslı bir ikinci olgunun sayıları, okurun o
    lisansı taşıdığı bilinmedikçe (lider olgu aynı lisansta değilse) sözcükle söylenir.
11. **Tavsiye yok:** fiiller gözlem; "olabilir" yalnız dönüş iddiasında (dedektörün kendisi bir
    olasılık söyler), al/sat/fırsat yok.

## Kendi sınamam

- Düzenek `langh dump --lang marla`: 531 durum × 2 dil, 13 taramanın 13'ü **0**
  (bu klasörde `SCAN.md`, `scan.json`, dedektör × aşama × dil tablosu `TABLE.md`;
  başvuru `r4`: 13 taramanın 9'unda bulgu, toplam 9266).
- Uzunluk (en çok / ortalama karakter): başlık TR 80 / 73,4, EN 80 / 69,7; ayrıntı TR 199 / 146,7,
  EN 200 / 155,1; karne TR 70 / 63,3, EN 70 / 57,8. Her başlık zamanı taşıyor (531/531).
- `cargo test -p qp-capsule`: modülün birim sınamaları + motor üzerinde uçtan uca sınama
  (`engine_tests`: sentetik pazar + lisanslı akış, her hikâye olayı yetkili/yetkisiz; uzunluk,
  iç terim, yetkiside rakam yok, TR/EN aynı sayılar, sözlük sayısı eşit, deterministik).

## Açık kalanlar ve öneriler

- "Beni ilgilendiriyor mu?" için portföy bilgisi gerekir (K42 olumsuz geçiş); dil bunu
  söyleyecek bir yer bırakır (başlıkta üçüncü ek bilgi değil, akışta işaret) — R5 kapsamı dışı.
- Kurumsal akışta "ortalama" mutlak net akışların ortalamasıdır; ayrıntı bunu ayrıca söylemiyor
  (uzunluk). Sözlüğe eklenebilir.
- Oynaklık bariyerinin sayısal büyüklüğü (k·ATR) karnede söylenmiyor; sözlük kuralı sözle
  anlatıyor. İstenirse kanıt ekranına (M20) bir satır.
- Kavram önerisi (sözlüğe): "Sonucun belli olacağı süre" (ufuktan kalan süre) ayrıntı cümlesinde
  kullanılıyor; `docs/CONCEPTS.md`'de adı yok.
