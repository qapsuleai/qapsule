# RP4 / M27 — İşlemden kesin bar ↔ REST (T2)

Yalnız toplu sayılar; sembol adı, fiyat, hacim yok. Veri yalnız bu makinede (`fixtures/local/`).

## Düzenek

- Komut: `qp live compare-trades` (`crates/qp-cli/src/live_trades.rs`). Önbellekteki işlemler
  (`fixtures/local/trades/<SYM>/*.csv.gz`, düzey 1; `trade_ex` → `fixtures/local/trades_ex/`)
  `parse_rest_csv` ile okunur, (yerel gün, işlem no) ile tekilleştirilir, borsa sırasına
  (`Trade::exchange_cmp`) dizilir ve **canlı yolun bar kurucusundan** geçer:
  `qp_source::LiveBars` + `BarMethod::Trades` → `DataEvent` → `FeedStore` (ayrı bar kodu yok).
- REST: `fixtures/local/live/rest/<SYM>.<1min|5min|1hour|1day>.*.unadj.json` (düzeltilmemiş),
  `qp live compare`'in yükleyicisiyle K14'e eşlenir (günlük `t` = 10:00; 18:00+ kapanış barı
  günün barına birleşir).
- Karşılaştırma: R6 M22'nin `compare_bars_with`'i; fiyat toleransı 1e-6 mutlak, hacim %1 göreli.
  Sütunlar M22'ninkiler (tam eşit = bit eşit) ve üç ek hacim sütunu: `v ≤ 1e-6` göreli,
  "fiyatlar bit eşit ve v ≤ 1e-6", `f32(v) = REST`.
- Kapsam: BIST30 (`fixtures/local/live/u30.txt`, 30/30 sembol), **34 işlem günü**
  (2026-10-05 → 2026-11-20; 2026-10-28 yarım gün), 44.859.378 işlem, 57 özel işlem.
- Kalibrasyon günleri (karar): 2026-10-05 → 2026-10-16 (10 gün, `SPLIT.md`). Sınama günleri
  yalnız raporlanır.

### Yeniden üretme (VERIFY 6)

Depo kökünden (veri ana ağaçta):

```
# Ana tablo (bütün aralık, seçilen kurallar)
qp live compare-trades
# Kararlar (yalnız kalibrasyon günleri; alternatifler)
qp live compare-trades --to 2026-10-16 --time ns,date_time --edges session,none \
    --special day_only,exclude,include --rest-price mill
# Fark dökümü (konum / gün)
qp live compare-trades --volume-units quantity --rest-price mill --detail --by-day
# Kapanış barının REST'teki yerleşimi (R3 kuralı)
qp live compare-trades --rest-close prev --volume-units quantity --rest-price mill --detail
# Kaynak tablosu (kart / işlem / karma), R6 kayıt günleri
qp live compare-trades --symbols @fixtures/local/live/u100.txt --from 2026-11-19 --to 2026-11-20 \
    --rest-price mill --raw <fixtures/local/live/raw/*.qpraw, virgülle, sıralı> --card-method mixed
# Fiyat adımı
qp live price-steps --raw <aynı liste> --symbols @fixtures/local/live/u100.txt
```

Varsayılanlar: `--symbols @fixtures/local/live/u30.txt --from 2026-10-05 --to 2026-11-20
--time ns --edges session --special day_only --volume-units value,quantity --rest-price raw,mill
--rest-close last`. Tam tablo ~6 dk (tek çekirdek).

## Ana tablo (M27, işlemden; 30 sembol, 34 gün)

REST fiyatı `mill` (0,001'e yuvarlanmış; aşağıda (e)). Kapanış barı REST'te günün son seans barına.

| Periyot | Hacim | Bar (eşleşen) | Yalnız işlem | Yalnız REST | Tümü tam eşit | Tümü toleransta | o / h / l / c tam % | v tam % | v ≤ %1 % | En büyük göreli fark (fiyat / v) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1m | lot | 464.890 | 13.470 | 0 | **%99,9998** (1 bar) | %99,9998 | 100 / 100 / 100 / 100 | 100 | 100 | 4,6e-4 / 0 |
| 1m | TL | 464.890 | 13.470 | 0 | %25,21 | %99,9998 | 100 / 100 / 100 / 100 | 25,21 | 100 | 4,6e-4 / 7,7e-8 |
| 5m | lot | 95.871 | 0 | 0 | **%99,999** (1 bar) | %99,999 | 100 / 100 / 100 / 100 | 100 | 100 | 4,6e-4 / 0 |
| 5m | TL | 95.871 | 0 | 0 | %24,09 | %99,999 | 100 / 100 / 100 / 100 | 24,09 | 100 | 4,6e-4 / 5,6e-8 |
| 1h | lot | 8.010 | 0 | 0 | **%100** | %100 | 100 / 100 / 100 / 100 | 100 | 100 | 0 / 0 |
| 1h | TL | 8.010 | 0 | 0 | %23,97 | %100 | 100 / 100 / 100 / 100 | 23,97 | 100 | 0 / 3,8e-8 |
| 1d | lot | 1.020 | 0 | 0 | %88,63 | %91,57 | 100 / 100 / 100 / 100 | 88,63 | 91,57 | 0 / 0,44 |
| 1d | TL | 1.020 | 0 | 0 | %19,80 | %91,57 | 100 / 100 / 100 / 100 | 19,80 | 91,57 | 0 / 0,44 |

TL hacim, "fiyatlar bit eşit ve v ≤ 1e-6 göreli": 1m %100, 5m %100, 1h %100, 1d %88,53.

Aynı tablo REST fiyatı **olduğu gibi** (`raw`): tümü tam eşit 1m %88,00, 5m %87,20, 1h %87,02,
1d %78,04 (lot); bütün fark REST'in fiyat yazımından (e).

**Hedef (≥ %99 tam eşit, fiyat ve hacim):**

- Gün içi (1m, 5m, 1h): **tuttu** — fiyat 4 alanda %100 (1m'de 464.890 barın 1'i, 5m'de
  95.871'in 1'i kapanışta farklı), hacim lot olarak %100 bit eşit; TL olarak bit eşitlik %24–25,
  ama göreli fark en çok 7,7e-8 (aşağıda (f)).
- Günlük (1d): **tutmadı, %88,63** (fiyat %100; hacim). Neden ölçüldü: (c) ve (d1).
  Kalibrasyon günlerinde (özel işlemler önbellekte) 1d %100.

## Kararlar (kalibrasyon günleri 2026-10-05 → 10-16; 30 sembol, 143.664 1m bar)

Tablo: tümü tam eşit (lot hacim, REST `mill`).

| Seçenek | 1m | 5m | 1h | 1d |
|---|---|---|---|---|
| **ns + session + day_only (seçilen)** | **%100,00** (143.663/143.664) | **%100** | **%100** | **%100** |
| `date_time` (saniye) + session + day_only | %99,97 | %99,93 | %99,92 | %100 |
| ns + **none** (açılış/kapanış seansı düşer) + day_only | %99,58 | %97,91 | %75,00 | %0 |
| ns + session + **exclude** | %100 | %100 | %100 | %86,33 |
| ns + session + **include** (her periyotta) | %99,97 | %99,83 | %98,00 | %100 |

**(a) Zaman alanı: `timestamp_ns`.** Birim doğrulandı: 44.859.378 satırın hepsinde
`0 ≤ timestamp_ns < 86.400·10⁹` (gün başından nanosaniye) ve `date_time − ⌊timestamp_ns⌋`
saniye: 0 s %98,14, +1 s %0,66, +2 s %1,17, +3 s %0,02 (eşleşme satırları; `date_time`
saniyeye yuvarlı ve geride değil, **ileride**). `date_time` ile bar sınırındaki işlemler kayıyor
(1m %99,97, 1h %99,92); `timestamp_ns` ile %100. Gün = `date_time`'ın tarihi (+ ofset).

**(b) Açılış ve kapanış seansı.** Açılış emir toplama eşleşmesi (`bidask = null`, 09:55
dakikasında) **günün ilk barına** (10:00) girer; kapanış seansı eşleşmesi (18:05) ve kapanış
fiyatından işlemler (18:08–18:09) **günün son barına** girer. Kanıt: REST 1 dakikalıkta kapanış
ayrı bir `18:04` barı olarak gelir ve 300/300 gün-sembolde adedi = 18:05 eşleşmesi + 18:08–18:09
işlemleri (Python, kalibrasyon); düşürülürse (none) 1m %99,58, 1h %75, 1d %0. Canlı kural
(`livebar` kural 1: `pre_open_ms` / `post_close_ms` 30 dk) zaten bu; değişiklik yok.
REST tarafında kapanış barının yeri: bütün aralıkta 3 günde son dakikada sürekli işlem yok; REST
(R3 kuralı, `--rest-close prev`) kapanışı bir önceki REST barına (17:58) koyar, canlı kural
takvimin son barına (17:59). Bu üç bar `prev` ile farklı (1m 3 + 3 yalnız-işlem, 5m 1 + 1),
`last` ile eşit. Ölçümde `last` (canlı kural) kullanıldı; öneri aşağıda.

**(c) Özel işlemler (`trade_ex`): yalnız günlük bara.** 57 özel işlem (10 kalibrasyon günü,
30 sembol; `trade.gz` satırlarıyla çakışma 0). REST gün içi barları onları **içermez** (include:
1m %99,97, 1h %98), REST günlük bar **içerir** (exclude: 1d %86,33, 41 gün-sembolün hacmi
eksik; include/day_only: %100). Fiyatlar ayırt edici değil: 57 işlemin 57'si o günün normal
işlem aralığında; günlük barda fiyatla birlikte katılım ölçüldü (o/h/l/c %100). Ürün yolu
düzeltmesi: `SpecialTrades::DayOnly` (yeni, varsayılan) — özel işlem `1d` barına her işlem gibi
girer, gün içi barlara girmez (`crates/qp-source/src/livebar.rs`, test
`special_trades_join_the_day_bar_only_by_default`; `LiveFeed`'in `TradeFeedConfig` varsayılanı da
bu). Fiyat kartı yolu (R6) değişmedi.

### (d) Kalan farklar (bütün aralık), sayıyla

1. **1d hacim, 116 / 720 gün-sembol (sınama günleri):** özel işlemler yalnız
   2026-10-05..10-18 penceresi için çekildi. 116 farkın 116'sında REST'in adedi bizimkinden
   **büyük** (hiç küçük değil), göreli eksik medyan %1,5, en çok %43,6; kalibrasyon günlerinde
   (özel işlemler var) 0 fark. Kalan pencerelerin `trade_ex`'i (10-19..11-01, 11-02..11-15,
   11-16..11-20; 30 sembol × 3 = 90 istek) çekilince kapanması beklenir — **ölçülmedi**.
   Ayrıca R6 kayıt günü (2026-11-19, 100 sembol) kartın günlük alanları REST 1d ile 100/100
   bit eşit (TL dahil): kartın `dailyVolume`'ü özel işlemleri içeriyor.
2. **Yalnız işlemde 13.470 1m bar:** REST 1 dakikalık dosyalarından 3'ü (10-19..11-01
   penceresi) boş (0 satır) ve 2026-11-16'da 4 sembolün günü kısa (REST'in "eksik 200" yanıtı,
   DATA_ACCESS R3). İşlem tarafında bar var, REST'te yok; yeniden çekim gerekir (ağ yok, yapılmadı).
3. **1 bar (1m ve aynı bar 5m'de), 2026-10-14:** kapanış fiyatı farklı. Barın son 4 işlemi aynı
   milisaniyede (`timestamp_ns` 10 ns adımlı), REST'in kapanışı bizim sondan bir önceki işlemimiz.
   Aynı ms içindeki sıra (işlem no ile) REST'inkinden farklı; tek örnek, kural çıkarılmadı.
4. **Yarım gün 2026-10-28:** fark yok (1m 4.050 bar, 1h 90 bar; kapanış seansı yarım gün
   kapanışından sonra son bara).
5. **Düzeltme / kurumsal işlem:** düzeltilmemiş REST'le karşılaştırıldığı için yok. (Düzeltilmiş
   harness serisiyle karşılaştırmada 1 sembolde var; aşağıda "Harness dışa aktarımı".)

### (e) REST fiyat yazımı

REST JSON'unda fiyatların %5,8'i (132.306 / 2.279.164) ondalık değerinden en çok 1e-5 uzakta
yazılıyor (tek duyarlıklı sayının ondalık yazımı: `…9.89999` = `…9.9`). İşlem fiyatlarının hepsi
en çok 3 ondalıklı (BIST30'da 158 işlem 3 ondalıklı, 2 günde 4 sembol; geri kalanı ≤ 2). `mill`
REST fiyatını 0,001'e yuvarlar (`round(x·1000)/1000`, ondalık metnin ayrıştırıldığı `f64`'e
eşit); 0,01'e yuvarlamak 3 ondalıklı 17 barı bozuyordu, 0,001 hiçbirini. Gerçek fiyat farkı
gizlenmez: yuvarlama mesafesi en çok 1e-5, fiyat adımı en az 0,01.

### (f) TL hacim

Lot hacmi her periyotta bit eşit. TL hacim (Σ fiyat·adet, `f64`, borsa sırası) REST'le bit eşit
değil (%24–27) ama göreli fark en çok 7,7e-8: REST tutarı fiyatın **32 bit** değeriyle hesaplıyor
(örnek 6 sembol × 1 gün, 2.850 bar: Σ f32(fiyat)·adet REST'e 1e-12 göreli içinde %74; ondalık
fiyatla bit eşit %34). Canlı işlem akışı da fiyatı 32 bit gönderir (`widen_price` ondalık yazımla
genişletir), yani REST'in tutarına bit eşit olmak için kasıtlı bir duyarlılık kaybı gerekir; yapılmadı.

## Fiyat adımı (`qp live price-steps`)

- Fiyat kartı kayıtları (12 kayıt, 2026-11-19 ve 2026-11-20; 1.989.383 kayıt): `priceStep`
  taşıyan 3.583 ileti, 581 sembol, 8 farklı adım. BIST tablosuyla (son fiyata, yoksa
  `basePrice`'a göre; bant üst sınırı hariç): **3.583 / 3.583 uyuşuyor** (kart büyük 0, küçük 0);
  581 / 581 sembol her iletisinde uyuşuyor. `basePrice`'a göre 3.489 / 3.583 (bant sınırına
  yakın fiyatlar; adım son fiyatı izliyor).
- İşlemler (BIST100, 34 gün, 64.684.528 işlem): tablonun ızgarasında **%99,9998**
  (64.684.370); dışındaki 158 işlemin hepsi 3 ondalıklı (yukarıda (e)); REST barları da aynı
  fiyatları taşıyor (o günlerde `mill` ile 100 eşit).

## M27 ikinci ölçü için yuva: kaynaklar (R6 kayıt günleri)

Kart kaydı olan barlarda (kesinleşmiş, kopma dışı, aboneliğin ilk barı değil; o gün REST barı
olan semboller): 100 sembol; 1m bar 2026-11-19 16.865, 2026-11-20 10.390. Kart yöntemi `mixed`
(R6 Ö72). Karma = o barda işlemden bar varsa işlem, yoksa kart. REST `mill`.

| Periyot | Kaynak | Bar | Yalnız kaynakta | Tümü tam eşit (lot) | o / h / l / c tam % | v tam (lot) % | v ≤ %1 % |
|---|---|---|---|---|---|---|---|
| 1m | kart | 27.231 | 24 | %84,77 | 94,95 / 98,21 / 97,94 / 97,89 | 89,13 | 94,57 |
| 1m | işlem | 27.231 | 0 | **%100** | 100 / 100 / 100 / 100 | 100 | 100 |
| 1m | karma | 27.231 | 24 | %100 | 100 / 100 / 100 / 100 | 100 | 100 |
| 5m | kart | 4.936 | 0 | %84,50 | 95,08 / 98,18 / 97,61 / 98,12 | 90,38 | 97,33 |
| 5m | işlem | 4.936 | 0 | **%100** | 100 / 100 / 100 / 100 | 100 | 100 |
| 5m | karma | 4.936 | 0 | %100 | 100 / 100 / 100 / 100 | 100 | 100 |
| 1h | kart | 220 | 0 | %82,73 | 94,55 / 98,18 / 95,45 / 98,64 | 90,45 | 100 |
| 1h | işlem | 220 | 0 | **%100** | 100 / 100 / 100 / 100 | 100 | 100 |
| 1h | karma | 220 | 0 | %100 | 100 / 100 / 100 / 100 | 100 | 100 |
| 1d | kart (gün alanları) | 100 | 0 | **%100** (TL dahil bit eşit) | 100 / 100 / 100 / 100 | 100 | 100 |
| 1d | işlem | 100 | 0 | %95 | 100 / 100 / 100 / 100 | 95 | 96 |
| 1d | karma | 100 | 0 | %95 | 100 / 100 / 100 / 100 | 95 | 96 |

- 1d yalnız 2026-11-19 (2026-11-20 kaydı kapanıştan önce bitti). İşlemin 1d farkı (5 sembol)
  bu günün özel işlemlerinin önbellekte olmamasından ((d1)).
- Kartta 24 bar REST'te yok (o dakikada işlem yok, kart bir bar açmış); karma bunları karttan alır.
- `--card-method ticks` ile kart 1m %84,68, 1h %81,82 (aynı sonuç).
- "Aynı günde iki kaynaktan doğan kapsüllerin farkı" bu çalışmada yok (benzetim/orkestratör).

## Harness dışa aktarımı (orkestratör isteği)

`qp live export-trade-bars --check fixtures/local/rp4/full` →
`fixtures/local/rp4/tradebars/full/<5m|15m|1h|1d>/<SYM>.bars` + `manifest.json` (yalnız
sayılar). Biçim harness'inki (`QPRP4BR1`, u64 sayı, 48 baytlık LE kayıt); kurallar M27'nin
(`timestamp_ns`, açılış ilk bara, kapanış seansı son bara, özel işlem yalnız 1d, v = TL);
15m, 5m'nin takvim yeniden örneklemesi (harness `prepare` gibi; düşen 0). BIST100, 34 gün:
5m 319.228, 15m 106.600, 1h 26.700, 1d 3.400 bar; 100/100 sembol.

Sınama aralığı (2026-10-19 → 11-20) barlarının harness REST `full/` (düzeltilmiş) serisiyle
eşitliği (REST `mill`; hacim TL):

| Periyot | Eşleşen bar | Yalnız işlemde | Yalnız REST'te | o / h / l / c tam % | Fiyatlar tam ve v ≤ 1e-6 % | v ≤ %1 % | En büyük göreli fark (fiyat) |
|---|---|---|---|---|---|---|---|
| 5m | 220.783 | 2.561 | 0 | 99,45 / 99,45 / 99,45 / 99,45 | 99,45 | 100 | 1,3e-2 |
| 15m | 73.751 | 849 | 0 | 99,45 / 99,45 / 99,45 / 99,45 | 99,44 | 99,99 | 1,5e-2 |
| 1h | 18.700 | 0 | 0 | 99,34 / 99,34 / 99,34 / 99,34 | 99,34 | 100 | 1,3e-2 |
| 1d | 2.400 | 0 | 0 | 99,33 / 99,33 / 99,33 / 99,33 | 93,67 | 95,62 | 1,3e-2 |

REST fiyatı olduğu gibi (`raw`): fiyatlar tam ve v ≤ 1e-6 5m %90,99, 15m %90,77, 1h %90,54,
1d %85,71. TL hacim bit eşitliği %13–15 ((f)); 15m'de v ≤ 1e-6 dışında kalan birkaç bar var
(%99,99; nedeni ayrıca ölçülmedi).

- Fiyat farklarının hepsi 3 sembolde: birinde sınama aralığındaki bütün barlar sabit 0,9868
  oranıyla farklı (harness REST'i **düzeltilmiş**; o sembolde aralık içinde bir kurumsal işlem
  var, düzeltme geriye doğru), ikisinde birer bar (oran ~0,995; açıklanmadı).
- Yalnız işlemde olan barlar (5m 2.561, 15m 849): harness REST serisinde olmayan barlar
  (REST'in eksik yanıtları).
- 1d hacim farkı (d1) ile aynı neden: sınama günlerinin özel işlemleri önbellekte yok.

## Açık sorular

1. Kalan `trade_ex` pencereleri (90 istek) çekilsin mi? 1d hacmin %100'e çıktığını sınamak için.
2. REST 1 dakikalık boş 3 dosya ve 11-16'daki 4 kısa gün yeniden çekilsin mi?
3. Aynı milisaniyedeki işlemlerin sırası (1 bar): REST hangi sırayı kullanıyor? Örnek az.
4. REST kapanış barının yeri: `fetch-bars.sh` / harness `prepare` / `qp live compare` R3 kuralıyla
   önceki REST barına koyuyor; canlı kural takvimin son barına. Önerim: REST eşlemesi de son
   seans barına (son dakikada işlem yoksa o barı kapanıştan yaratarak). Bütün aralıkta 3 gün-sembol.
5. Özel işlemin fiyatı günlük barın h/l'sini değiştirir mi (57 örneğin hepsi aralık içinde)?
