# Matriks akışının `DataEvent`'e eşlenmesi (değerlendirme, uygulanmadan)

Durum: RP2 / T3, kabul ölçütü 3. **Kod yok**; gerçek bağlayıcı R6'nın işi. Kaynaklar:
`docs/MATRIKS_PROTOCOL.md` §6, §9–§12 (oradaki güven etiketleri geçerli), `docs/PROTOCOL.md`
§18.4, `rounds/RP2/RESEARCH.md` §1.2, §2.4, §5. Bu belgedeki her yargı **[çıkarım]**dır;
R6'da gerçek mesajla doğrulanmalıdır. Bilinmeyenler "R6-Sn" diye işaretli ve §9'da toplu.

## 1. İlke: barı bağlayıcı kurar, motor yalnız `DataEvent` görür

```
Matriks REST (tarihsel)  ─┐
Matriks MQTT (protobuf)  ─┼─► bağlayıcı (barındırıcı tarafı, R6) ─► DataEvent ─► FeedStore / oturum
TimeMessage (saat)       ─┘        bar kurma kuralı + yerel seq
```

- Motorun girdisi yalnızca `DataEvent`'tir (`bar` / `patch`, `seq`). Matriks mesajı motora
  hiç girmez; protobuf çözümü, sembol kimliği eşlemesi, bar sınırı ataması bağlayıcıdadır.
- **Bar kurma kuralı** (hangi mesaj, hangi alan, hangi birim, f32→f64) beslemenin
  (`FeedKey.feed`, ör. `matriks`) sabit bir parçasıdır: aynı beslemeye bağlanan iki taraf
  aynı kuralla kurulmuş barları görür. Kural değişirse besleme adı değişir
  (ör. `matriks-trade`), çünkü aynı adla iki kural kalıcı bit farkı üretir
  (RESEARCH §2.4).
- Belirleyicilik: motorun çıktısı aynı olaylar aynı sırayla geldiğinde aynıdır (§18.4).
  Matriks'ten olaya geçişin kendisi (tiklerin hangi bara düştüğü, hangisinin kaybolduğu)
  belirleyici değildir; bu yüzden tutarlılık, olayların içeriğine (`DataPosition`, özet)
  dayanır, bağlayıcının davranışına değil.

## 2. Kaynaklar ve rolleri

| Matriks kaynağı | Rolü | `DataEvent` karşılığı |
|-----------------|------|------------------------|
| REST `temel-bar` / `türetilmiş-bar` (§6) | İlk çözüm (pencerenin barları); kesinleşmiş barlarla düzeltme | `Resolution::Ready{bars, seq}`; sonradan fark → `patch` (düzeltme) |
| `SymbolMessage` `REFRESH` (§9) | Abonelik / yeniden bağlanma sonrası tam anlık görüntü | Canlı barın farklı alanları → `patch`; gerekirse yeni `bar` |
| `SymbolMessage` `UPDATE` (§9) | Alan düzeyinde delta: yalnızca değişen alanlar | Canlı bar → `patch` (yalnız değişen OHLCV alanları); bar sınırı geçilince `bar` |
| `TradeMessage` (§9) | Tek tek işlemler (f32 fiyat, µs zaman, anlık görüntü yok) | v0'da bar kaynağı **değil** (§4); isteğe bağlı ayrı besleme |
| `TimeMessage` (§9) | Borsa saati, seans açık/kapalı | Bar sınırı ve bar kapanışı için saat; kendisi olay üretmez |

`UPDATE`'in alan düzeyindeki yapısı `BarPatch` ile doğrudan örtüşür: Matriks "yalnızca
değişen alanlar" gönderir, `patch` "yalnızca değişen alanlar" taşır. Bu, K23'ün "alan
düzeyinde" seçiminin ana gerekçesiydi ve eşleme bunu doğruluyor.

## 3. Bar kurma: `SymbolMessage`'tan (önerilen v0 kuralı)

### 3.1 Günlük bar (`1d`)

`SymbolMessage` günlük değerleri zaten taşır; bar doğrudan alanlardan okunur:

| Bar alanı | Matriks alanı | Not |
|-----------|---------------|-----|
| `t` | işlem gününün açılışı, `qp_calendar` ile (BIST, İstanbul) | `updateDate` biçimi belgesiz (R6-S1); gün TimeMessage saatinden |
| `o` | `open` (47) | |
| `h` | `dailyHigh` (13) | (`high` (8) ile farkı belgesiz, R6-S2) |
| `l` | `dailyLow` (12) | |
| `c` | `last` (9) | |
| `v` | `quantity` (14, lot) | REST'te karşılığı `totalQuantity` (lot). TL hacmi (`volume` 15 / REST `volume`) seçilirse iki uçta da o seçilmeli; aksi halde her bar "düzeltilmiş" görünür |

`UPDATE` → `patch`: mesajda bulunan alanlardan yalnız yukarıdakiler ve yalnız değeri
gerçekten değişenler (bit düzeyinde) `fields`'e girer. Örnek: `UPDATE{last: 302.25,
quantity: 1200}` → `{"kind":"patch","t":<bugün>,"fields":{"c":302.25,"v":1200}}`.
`dailyHigh` gelmediyse `h` yoktur; motor `h >= c` kuralını denetler (FeedStore
`InconsistentOhlc`), yani `last` yeni zirveyse Matriks'in `dailyHigh`'ı aynı mesajda
göndermesi gerekir — göndermiyorsa bağlayıcı `h = max(h, c)` yazar (R6-S3).

### 3.2 Gün içi bar (`1m`…`240m`)

Gün içi barın hazır alanı yoktur; bağlayıcı `last` ve kümülatif miktardan kurar:

| Bar alanı | Kural |
|-----------|-------|
| `t` | Tikin zamanının `qp_calendar` ile hizalanmış bar açılışı (K14). Tik zamanı: `UPDATE`'te güvenilir zaman yok (R6-S1) → `TimeMessage` saatine göre alış zamanı |
| `o` | Bar içindeki ilk `last` |
| `h` / `l` | Bar içindeki `last` değerlerinin en büyüğü / en küçüğü; **yalnız aşıldığında** `patch`'e girer |
| `c` | Son `last` |
| `v` | `dailyQuantity`(48) (ya da `quantity`) − bar başındaki kümülatif değer |

Yeni bar: tikin hizalanmış `t`'si son barınkinden büyükse `bar` olayı
`{o=h=l=c=last, v=Δ}`. Tik gelmeyen periyot için bar üretilmez (boş bar yok; REST'in
davranışıyla aynı olmalı, R6-S4).

Bu, `SyntheticEvents`'in ürettiği biçimin ta kendisidir: açılışta `bar`, sonra `c` her
değişimde, `h`/`l` yalnız aşıldığında, `v` büyüdükçe.

## 4. `TradeMessage` neden v0'da bar kaynağı değil

1. **f32 fiyat.** `price` 32 bit `float`. `f64::from(f32)` kesindir ama f32'nin temsil
   hatasını taşır: ör. `301.55f32` → `301.549987792…`. Aynı fiyat `SymbolMessage`'ta
   `double` olarak `301.55`'e en yakın f64'tür; iki kaynaktan kurulan barlar **kalıcı**
   bit farkı verir (MATRIKS_PROTOCOL §12/5).
   **Kural önerisi (trade kullanılırsa):** fiyat, sembolün `fractionCount`(11) basamağına
   ondalık yuvarlanır: `p64 = (f64::from(p32) * 10^k).round() / 10^k`. Tik büyüklüğü
   f32'nin ondalık çözünürlüğünden büyük olduğu sürece (BIST fiyat aralıklarında öyle
   görünüyor, R6-S5) bu, `double` kaynağın vereceği değeri geri kazanır. Yuvarlama
   belirleyicidir (yalnız IEEE temel işlemleri). Kural beslemenin parçasıdır.
2. **Anlık görüntü yok.** Abonelikten önceki işlemler gelmez; o anki barın `o`/`h`/`l`'si
   bilinmez. REST ya da `REFRESH` ile tamamlanması gerekir, yani yine iki kaynak.
3. **Zaman birimi µs** (§10'daki belge/kod çelişkisiyle, R6-S6). Hizalama için
   `t_ms = ts_us.div_euclid(1000)` ve sonra `qp_calendar`; çelişki çözülmeden
   kullanılmamalı.
4. **Hacim**: `quantity` int64 lot → `f64`, 2^53'e kadar kesin.

Trade akışı ayrı bir besleme olarak (ör. `feed: "matriks-trade"`) değerlendirilebilir;
aynı beslemede `SymbolMessage` ile karıştırılmaz.

## 5. Sıra numarası (`seq`): yerel

- Matriks MQTT QoS 0'dır ve belgelenmiş bir sıra numarası yoktur (RESEARCH §1.2/1,
  R6-S7). `DataEvent.seq` bu yüzden **bağlayıcının yerel sayacıdır**: besleme başına
  1, 2, 3… alış sırasıyla, ürettiği her olaya bir.
- Sonuç: `FeedStore`'un boşluk tespiti (§18.4) **bağlayıcı → motor** arasındaki kaybı
  yakalar (ör. NATS üzerinden dağıtımda), **Matriks → bağlayıcı** arasındaki kaybı
  yakalamaz. Bu bilinçli bir zayıflıktır; üst akıştaki kayıp için §6'daki dolaylı
  dedektör kullanılır.
- Bağlayıcı yeniden başlarsa sayaç kaldığı yerden sürmelidir (kalıcı) ya da beslemeyi
  sıfırlamalıdır (tüketiciler yeniden çözer). Sayaç geri giderse tüketici yeni olayları
  yineleme sanıp yok sayar — bu yüzden "sayaç sıfırlandı" bir yeniden çözme sinyaliyle
  birlikte gitmelidir (R6-S8; CME'nin kanal sıfırlamasında `RptSeq`'in 1'e dönmesiyle aynı
  sorun, RESEARCH §2.4).

## 6. Kayıp tespiti: günlük kümülatif alanlar

`SymbolMessage`'ın günlük kümülatif alanları dolaylı bir kayıp dedektörüdür (düzeltmez):

| Denetim | Kayıp belirtisi |
|---------|-----------------|
| Bugünkü gün içi barların `v` toplamı = `dailyQuantity` (ya da `quantity`) | Toplam küçükse bir `UPDATE` kaçmıştır; ama sonraki kümülatif değer farkı bir sonraki bara yazıldığı için toplam kendini onarır, **dağılım** yanlış kalır |
| `dailyHigh` > bugünkü barların en büyük `h`'si (benzer şekilde `dailyLow`) | Zirve/dip tikini taşıyan güncelleme kaçmıştır; hangi gün içi barda olduğu bilinmez |

Tepki: günlük barda doğrudan `patch` (`h = dailyHigh`). Gün içinde bar belirsiz olduğundan
bağlayıcı o günün barlarını REST'ten yeniden çeker ve farkları düzeltme olarak gönderir
(§8). Kayıp sinyali ayrıca `binding` sorgusunda bir neden olarak görünebilir (T4/T8'in
kararı).

## 7. Yeniden bağlanma ve `REFRESH`

Matriks'te geri doldurma yoktur (§11): bağlantı kopukken gelen tikler kaybolur; yeniden
abonelikte `REFRESH` (tam anlık görüntü) gelir.

Önerilen davranış — **bağlayıcı tarafında uzlaştırma, olay olarak**:

1. `REFRESH` alanları tutulan canlı barla karşılaştırılır. Günlük barda farklı alanlar
   doğrudan `patch` olur. Gün içinde `REFRESH` yalnız son durumu (`last`, kümülatif
   miktar, günlük uçlar) verir; kopukluk süresindeki gün içi barları vermez.
2. Kopukluk bir ya da daha çok gün içi bar sınırını aştıysa bağlayıcı kopukluk aralığını
   REST'ten çeker: eksik barlar `bar` olayı, değişen barlar `patch` (düzeltme) olarak,
   **yerel `seq` kesintisiz** sürer. Motor yolu değişmez; düzeltme birinci sınıf bir
   olaydır ve özet (`DataPosition.digest`) farkı zaten yakalar.
3. REST'e ulaşılamıyorsa ya da aralık eşikten uzunsa (ör. pencerenin büyük kısmı)
   bağlayıcı olay üretmek yerine beslemeyi bayat bildirir: tüketicide `seq` boşluğu
   yaratır ya da barındırıcı yeniden çözmeyi tetikler; `FeedStore` `reset` ile sağlayıcının
   yeni çözümünü alır.

"Yeniden çöz" (reset) ile "olayla düzelt" (patch) arasındaki seçim maliyetle ilgilidir:
düzeltme bar *i*'den sonrasının hesabını ve özetini yeniden yaptırır (`FeedStore`'da
O(n − i), T3 ölçümü), reset bütün pencereyi. Kısa kopuklukta patch daha ucuz, uzun
kopuklukta eşittir ve reset daha basittir.

`symbolId → symbolCode` eşlemesi `REFRESH`'ten tutulur; `UPDATE`'te `symbolCode`
olmayabilir (§9). Eşlemesi bilinmeyen `UPDATE` atılmaz, ilk `REFRESH`'e kadar
bekletilir ya da kayıp sayılır (R6-S9).

## 8. Kesinleşmiş barla düzeltme (REST)

- Seans kapanışından sonra (ve isteğe bağlı olarak bar kapanışından bir süre sonra)
  bağlayıcı o günün barlarını REST'ten alır, tuttuğu barlarla alan alan karşılaştırır,
  farklı alanları `patch` olarak gönderir. `FeedStore` bunları `Correction{index}` olarak
  bildirir; oturum o bardan sonrasını yeniden hesaplar (§18.8).
- Bölünme / temettü ayarı bütün geçmişi ölçekler: binlerce düzeltme yerine yeniden çözme
  (reset) daha uygundur. Eşik barındırıcının kararıdır.
- Koşul: REST ile canlı kural **aynı birimi** kullanmalı (lot ↔ lot, aynı fiyat kaynağı);
  yoksa her bar sahte bir düzeltme üretir. REST `time`'ın bar başlangıcı olup olmadığı
  belgesiz (R6-S10); `t` eşlemesi buna bağlıdır.
- Bu, RESEARCH'teki "doğru = sağlayıcının kesinleşmiş barı" (S3) tanımının veri düzlemindeki
  karşılığıdır: kesinleşme, REST barının `patch`'lerle uygulanmış hali.

## 9. Açık sorular (R6)

| # | Soru |
|---|------|
| R6-S1 | `SymbolMessage.updateDate` biçimi ve saat dilimi; `UPDATE`'te güvenilir bir olay zamanı var mı? Yoksa bar sınırı alış zamanına (TimeMessage saatine) göre atanır ve sınıra yakın tik iki barındırıcıda farklı bara düşebilir. |
| R6-S2 | `high`/`low` (7–8) ile `dailyHigh`/`dailyLow` (12–13) farkı. |
| R6-S3 | `last` yeni zirve olduğunda `dailyHigh` aynı `UPDATE`'te mi geliyor? |
| R6-S4 | REST hiç işlem olmayan periyot için bar döndürüyor mu (boş bar kuralı)? |
| R6-S5 | BIST fiyat adımları ve `fractionCount` her sembolde güvenilir mi; f32 → ondalık yuvarlama her fiyat aralığında `double` değeri geri veriyor mu? |
| R6-S6 | `TradeMessage.timestamp` biçimi (§10 çelişkisi: µs epoch mı, gece yarısından ms + sıra mı?). |
| R6-S7 | Akışta (MQTT ya da Topach) sıra numarası, yeniden gönderim ya da boşluk doldurma var mı? |
| R6-S8 | Bağlayıcı yeniden başlarken yerel `seq` nasıl sürdürülür; tüketiciye "sayaç sıfırlandı" nasıl bildirilir (şema gerektirebilir, bkz. T3.md)? |
| R6-S9 | `symbolId → symbolCode` eşlemesi `REFRESH` öncesi gelen `UPDATE`'ler için nereden? |
| R6-S10 | REST `time` bar başlangıcı mı; `date` hangi saat diliminde? |
| R6-S11 | REST barı ile canlıdan kurulan bar seans sonunda bit düzeyinde aynı mı (aynı birim ve kaynak alanla)? Değilse her gün sistematik düzeltme gelir; ölçülmeli. |
| R6-S12 | Seans dışı / açılış-kapanış seansı (`sessionIsOpen`, `stockStatus`) güncellemeleri hangi bara yazılır? |

## 10. Biçimin yeterliliği (değerlendirme sonucu)

- **Yeterli:** canlı güncelleme (`UPDATE` → `patch`), yeni bar, geçmiş düzeltme (REST →
  `patch`), anlık görüntü uzlaştırma (`REFRESH` → `patch`), kayıp sonrası yeniden çözme
  (`seq` boşluğu → `reset`). Hepsi bugünkü `DataEvent` ile ifade edilebiliyor.
- **Eksik (şema önerisi, T3.md'de):** (a) bar **silme** yok — sağlayıcı hatalı bir barı
  geri çekerse tek yol yeniden çözme; (b) barın **kesinleştiği** işareti yok (bar kapandı /
  REST ile onaylandı) — tutarlılık yaklaşımları (RESEARCH D, "kesinleşme") bunu isteyebilir;
  (c) bağlayıcının "sayaç sıfırlandı / yeniden çöz" sinyali olay olarak yok.
- **Kapsam dışı ve bilerek yok:** VWAP (`weightedAverage`), işlem adedi, derinlik; bunlar
  bar değil, ayrı beslemeler.
