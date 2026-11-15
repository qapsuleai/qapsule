# M6 adayı `coarse` — "Kaba ilişki + süreklilik"

Crate: `rounds/R3/experiments/M6/coarse/` (`m6_coarse`), giriş `trust_path()` (her çağrıda
taze durum), düzenekte `("coarse", m6_coarse::trust_path)`. Ham ve özet:
`rounds/R3/experiments/M6/raw/coarse/`.

## Fikir

Yetkisiz kullanıcıya barın kurumsal net akışının **sayı değil, kaba bir durumu** gider:
"olağan" / "olağanın belirgin biçimde üzerinde (alış)" / "… (satış)". İleti yalnız durum
değişince çıkar (giriş, yön dönüşü, olağana dönüş), bir barlık gecikmeyle gösterilir.
Sızıntıyı azaltan üç ilke:

1. **Ölçü çubuğu ölçülenden ayrı.** "Olağan" düzeyi `U`, raporlanan büyüklükten (`net`)
   değil, en büyük kurumun aynı barlardaki **iki yönlü brüt işlemi** `min(buy, sell)`'den
   (net'e girmeyen ciro) hesaplanır: son 40 gözlemin medyanı, şimdiki bar hariç, 20
   gözlemde bir yenilenir, aşağı doğru 2'nin kuvvetine yuvarlanır. Hiçbir `net` değeri
   `U`'ya girmez; bu yüzden bir barın `net`'i sonraki barların eşiğini **hiç**
   oynatmaz ve sonraki iletiler o değeri ancak kendi barının kaba durumu kadar ele verir
   (test: `a_past_net_reaches_later_bars_only_through_its_own_state`).
2. **Kaba nicemleme + histerezis.** `x = |net| / U`. Giriş `x ≥ 0,45`, çıkış `x < 0,15`
   ya da işaret dönerse. Bir barın kendi iletisi değerin yalnız geniş bir aralıkta
   olduğunu söyler: `[0,45·U, ∞)`, `[0,15·U, 0,9·U)` gibi; aralık kenarları kaba ızgaradaki
   `U`'ya bağlıdır.
3. **Olgu tetikleyici değil.** `flow.institutional` olgusu (`|net| > 2·avg5`) ileti
   tetiklemez. `avg5` son beş `|net|`'e sürekli bağlıdır; olguyu yankılamak, sonraki beş
   barın olgu var/yok bilgisiyle hedef değeri iki yandan kıstırır (`echo` control'ün en
   kötü hedeflerinde 7–10 bit bu yüzden).

### Durum makinesi

| Önceki | Bar | Sonraki | İleti |
|---|---|---|---|
| Olağan | `x ≥ enter` | Yoğun(işaret) | giriş (`flow_well_above_usual`, `net_buy`/`net_sell`) |
| Olağan | `x < enter` | Olağan | — |
| Yoğun(d) | `x ≥ enter`, ters işaret | Yoğun(ters) | yön dönüşü (`reversal`) |
| Yoğun(d) | ters işaret, `net = 0` ya da `x < exit` | Olağan | olağana dönüş (`flow_back_to_usual`) |
| Yoğun(d) | aynı işaret, `x ≥ renew` | Yoğun(d) | yenilendi (`renewed`) |
| Yoğun(d) | aynı işaret, `exit ≤ x < renew` | Yoğun(d) | 3. barda bir kez "sürüyor" (`persisting`), sonra — |

"Sürüyor" notu yeni bilgi taşımaz (çıkış iletisinin yokluğu aynı şeyi söyler); kullanıcıya
sürekliliği açıkça söylemek için var. `net` yoksa ya da sonlu değilse bar sessizdir ve
durumu değiştirmez; `buy`/`sell` yoksa `U` penceresine örnek girmez.

## Parametreler ve neden

| Parametre | Değer | Neden |
|---|---|---|
| Ölçü çubuğu | brüt `min(buy, sell)` medyanı | `net`'ten bağımsız; ayarlamada en kötü hedef ~4,7 → ~2,8 bit |
| Pencere / ilk | 40 / 10 gözlem | "olağan" ≈ son iki ay (günlük); 10 gözlemle ısınma (olguların ilki 6. gözlemde) |
| Yenileme | 20 gözlemde bir | bir barın brütü en çok iki yenilemeye girer; 10'a göre daha az eşik oynaması |
| Izgara | 2'nin kuvvetleri (aşağı) | √2 ızgarasına göre daha az eşik geçişi; ayarlamada en kötü bit 4,7 → 3,4 (`net` çubuğuyla) |
| `enter` / `exit` | 0,45 / 0,15 | `flow.institutional` olgularının çoğunu yakalayan en yüksek giriş; geniş histerezis bandı orta kovayı genişletir |
| `renew` | 0,9 (= 2·enter) | yoğun durumda gelen güçlü barı (çoğu yeni bir olgu) yeniden söyler; fayda +0,1, kova `[exit, renew)` geniş kalır |
| `sustain` | 3 bar | "birkaç mumdur sürüyor" bilgisi |
| Gecikme | 1 bar | bar kapandıktan sonra yayım (gerçek zamanlı değil); kâhin ölçüsünü değiştirmez, lisans açısından "gecikmeli yorum" |

Rakam yok; metinde `birkaç` gibi sözcükler bile sayı bildirmez. Ekler sabittir
(`basis = licensed_institutional_flow`, `entitlement = institutional_flow`).

### Ayarlama (tohum 1, ilk 16 enstrüman, gerçekçi saldırganda 100 aday)

"Bit" sütunları ızgarası **tamamen** tutulan hedefler hariç (bkz. Zayıflıklar: bu hedeflerde
"sızıntı" düzeneğin ızgara kısalığından doğar).

| Çeşit | Fayda | İleti | Bit p90 | p99 | en çok | > 3 bit |
|---|---|---|---|---|---|---|
| `net` medyanı, √2, yenileme 10, giriş 2 / çıkış 1 | 0,744 | 3.556 | 1,21 | 3,09 | 4,72 | 16 |
| … yenileme 20 | 0,730 | 3.506 | 1,15 | 2,53 | 4,40 | 3 |
| … ızgara 2 (yenileme 10) | 0,749 | 3.889 | 1,19 | 2,57 | 3,43 | 4 |
| … giriş 2,5 | 0,696 | 3.045 | 1,28 | 2,83 | 4,95 | 10 |
| … `renew` = giriş | 0,860 | 4.094 | 1,26 | 3,33 | 4,85 | 20 |
| brüt, 0,5 / 0,25, `renew` yok | 0,736 | 3.464 | 1,20 | 1,96 | 2,23 | 0 |
| brüt, 0,4 / 0,15, `renew` yok | 0,752 | 3.864 | 1,22 | 2,07 | 2,81 | 0 |
| brüt, 0,7 / 0,35, `renew` 0,7 | 0,727 | 2.993 | 0,98 | 1,59 | 3,15 | 1 |
| brüt, 0,5 / 0,25, `renew` 0,5 | 0,843 | 3.931 | 1,20 | 2,08 | 4,02 | 6 |
| brüt, 0,45 / 0,15, `renew` 0,45 | 0,865 | 4.203 | 1,21 | 2,17 | 3,75 | 5 |
| brüt, 0,4 / 0,2, `renew` 0,4 | 0,894 | 4.557 | 1,24 | 2,15 | 4,34 | 4 |
| brüt, 0,4 / 0,15, `renew` 0,8 | 0,879 | 4.215 | 1,38 | 2,34 | 3,00 | 1 |
| **brüt, 0,45 / 0,15, `renew` 0,9 (seçilen)** | **0,849** | 3.916 | 1,35 | 2,13 | **2,80** | **0** |

Seçim ölçütü: en kötü hedefte 3 bitin altında kalan çeşitler içinde en yüksek fayda.
`renew` olmadan fayda ~0,75'te kalıyor; `renew` eşiği girişle aynı olunca orta kova
daralıp en kötü bit 3,7–4,3'e çıkıyor; `renew = 2·enter` ikisini dengeliyor.

## Sonuçlar (tam düzenek, 3 tohum)

`harness-sim m6 run --alt coarse --data synthetic` (tohum 1, 2, 3; ~1.230 s). Tam tablo:
`rounds/R3/experiments/M6/raw/coarse/summary-synthetic.md`. Karşılaştırma control `echo`.

| Ölçü (havuz, 8.406 hedef) | `coarse` | control `echo` |
|---|---|---|
| Kâhin genişlik/yayılım medyan · p10 · en kötü | 1,69 · 0,40 · 0,0007 | 1,17 · 0,20 · 0,0007 |
| Olgulu / olgusuz hedef medyan | 1,75 / 0,30 | 1,22 / 0,36 |
| Bit medyan · p90 · en çok (düzenek) | 0 · 1,33 · 10,41 | 0 · 2,34 · 10,41 |
| Izgarası tamamen tutulan hedef | 149 | 99 |
| Bit, bu hedefler hariç: p90 · p99 · en çok | **1,29 · 2,20 · 3,98** | 2,30 · 4,42 · 8,04 |
| > 2 bit · > 3 bit · > 4 bit hedef | **151 · 4 · 0** | 1.035 · 432 · 146 |
| Çok parçalı · gerçek tutulmadı | 0 · 0 | 56 · 0 |
| Gerçekçi: tutulan aday medyan · ≥ 20 tutulan | 33 · 4.834 | 120 · 6.432 |
| Gerçekçi: sonsal / önsel medyan · kapsama | 0,586 / 0,549 · 0,835 | 1,083 / 0,549 · 0,889 |
| Gecikme | hepsi 1 bar | hepsi 0 |
| trust = server · metinde/kodda/ekte rakam · TR/EN ifade | 1 · 0/0/0 · 1/1 | 1 · 0/0/0 · 1/1 |
| İleti · olgu · **fayda** | 21.901 · 8.136 · **0,843** (0,847 / 0,846 / 0,838) | 8.136 · 8.136 · 1 |

Okuma:
- Düzeneğin "en kötü" ve "bit en çok" sütunları iki yolda da aynı (0,0007; 10,41):
  çok küçük `|net|`'li hedeflerde ±%300 ızgara yayılımın küçük bir parçasıdır, ızgaranın
  tamamı tutulsa da bit sayılır. Bu hedefler ayrılınca `coarse`'ta en kötü hedef 3,98 bit
  (8.257 hedeften 4'ü 3 bitin üstünde), `echo`'da 8,04 bit (432'si 3 bitin üstünde).
- Hiçbir hedefte aralık parçalanmıyor (çok parçalı 0): durum makinesi değerin kendisi
  dışında hiçbir şeye bağlı değil, kova tek aralık.
- Olgusuz (sıradan) hedeflerde medyan oran 0,30: "olağan" kovası `|net| < 0,45·U`
  (önceki durum yoğunsa `< 0,15·U` ya da ters işaret) bilgisini verir.
- Gerçekçi saldırgan: `coarse` iletileri daha çok (bar başına ~0,5) olduğundan ±3 bar
  penceresi aday sayısını azaltıyor (33'e karşı 120) ve sonsal aralık olguya koşullu
  `echo`'dakinden dar; önselle aynı mertebede (0,59 / 0,55).

## Örnek iletiler

Her iletinin sonuna dayanak cümlesi eklenir:
TR "Dayanak: lisanslı kurumsal akış verisi; rakamları görmek için kurumsal akış yetkisi
gerekir." · EN "Based on licensed institutional flow data; the figures require the
institutional flow entitlement."

| Kod | TR | EN |
|---|---|---|
| `flow_well_above_usual`, `net_buy` | Sunucu doğruladı: bu mumda kurumsal net alış, bu sembolde olağan kurumsal işlem hacmine göre belirgin biçimde yüksek. | Server-verified: institutional net buying on this bar is well above usual for the normal institutional trading activity in this symbol. |
| … `net_sell`, `reversal` | Sunucu doğruladı: kurumsal akış yön değiştirdi; bu mumda net satış, olağan kurumsal işlem hacmine göre belirgin biçimde yüksek. | Server-verified: institutional flow reversed; net selling on this bar is well above usual for the normal institutional trading activity. |
| `flow_above_usual`, `net_buy`, `persisting` | Sunucu doğruladı: olağanın üzerindeki kurumsal net alış birkaç mumdur sürüyor. | Server-verified: above-usual institutional net buying has persisted for several bars. |
| `flow_well_above_usual`, `net_sell`, `renewed` | Sunucu doğruladı: kurumsal net satış bu mumda yeniden olağanın belirgin biçimde üzerinde. | Server-verified: institutional net selling is again well above usual on this bar. |
| `flow_back_to_usual` | Sunucu doğruladı: kurumsal net akış bu sembolde yeniden olağan ölçülerde. | Server-verified: institutional net flow in this symbol is back to usual proportions. |

K34 (vazgeçilmez): ileti kanıtın var olduğunu (sunucu doğruladı), neye dayandığını
(lisanslı kurumsal akış) ve hangi yetkiyi gerektirdiğini (kurumsal akış yetkisi) söyler.

## Zayıflıklar (dürüstçe)

- **Bir barın kendi kovası sızar.** Kâhin saldırganı `[0,15·U, 0,9·U)` gibi bir kovayı
  öğrenir; yayılım (`|net|` p5–p95) `U`'nun birkaç katıysa bu 2–4 bit eder (tam
  koşuda p99 2,2, en kötü 3,98 bit). Bu,
  nitel ilişkinin kendisinin bedelidir; daha az bit için daha az kova (ör. `renew` yok:
  en kötü ~2,2 bit) ama daha az fayda (~0,74).
- **Brüt değerler hâlâ eşiği oynatır.** `U` brütten gelir; düzeneğin saldırganı yalnız
  `net`'i bilinmez sayar (brüt tutarlı kaydırmada sabit). Bir barın brütü bilinmez
  sayılsaydı o değer sonraki eşikleri (medyan + kaba ızgara + seyrek yenileme
  sayesinde yalnız tek yönlü, basamaklı) etkilerdi; ölçülmedi. Brüt de lisanslıdır.
- **"Olağan"ın anlamı değişti.** "Sembolün olağan net akışı" değil, "olağan kurumsal
  işlem hacmine göre" net akış. Metin bunu açıkça söyler; ama gerçek veride en büyük
  kurumun brütü ciroyla sentetikteki kadar orantılı olmayabilir. Gerçek akışla
  doğrulanmadı (yerelde lisanslı akış yok). Sentetik üreteçte brüt = ciro × U(0,03; 0,12)
  gürültülü bir çarpandır; 40 gözlemlik medyan bunu yumuşatır.
- **Fayda %100 değil** (~0,85): olgu tanımı (`2·avg5`, kısa pencere) ile "olağan
  hacme göre yüksek" farklı şeyler; olgusu olup `x < 0,45` kalan barlar ve yoğun
  durumda `x < 0,9` kalan olgulu barlar iletisiz kalır. Bunlar için olgunun kendisi
  yetkisize hiç ulaşmaz (sessizlik bilinçli).
- **Konuşkan:** yaklaşık her iki bardan birinde ileti (giriş/çıkış/yenileme). Ürün
  tarafında kapsül yalnız girişleri öne çıkarıp diğerlerini zaman çizelgesinde
  tutabilir; bu M6'nın dışında.
- **Gerçekçi saldırgan** daha çok ileti gördüğü için `echo`'ya göre daha az aday tutuyor;
  sonsal medyanı önselle aynı mertebede (0,59 / 0,55) ama p10'da 0,21'e iniyor; kapsama
  0,835 (%90 hedefinin altında: az aday tutulan hedeflerde yüzdelik kaba).
- **Düzenek kapsamı:** 16 enstrüman / 1 tohumla ayarlandı, 30 enstrüman / 3 tohumla
  ölçüldü; parametreler aynı veri üretecine göre seçildi (aşırı uyum riski). Ayarlama
  tablosundaki bit sütunları yalnız tohum 1'in ilk 16 enstrümanıdır.
- **Gecikme** kâhin ölçüsünde hiçbir şey kazandırmaz; yalnız gerçek zamanlı lisans
  değerini korur. Toplu gönderim (ör. gün sonu paketi) de aynı nedenle sızıntıyı
  azaltmaz; seçilmedi.
- Izgarası tamamen tutulan hedeflerde (çok küçük `|net|`) düzenek bit sayar
  (`log2(yayılım / 6·|net|)`); bu ölçü yapısıdır, gerçek sızıntı değildir (`echo`'da
  da aynı hedefler en kötü satırda).
