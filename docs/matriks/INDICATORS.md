# Quantex gösterge servisleri (RI1/T4)

Durum: RI1 keşfi, salt okunur kod incelemesi (2026-11-19). Kaynak depo `quantex`
(`/Users/arkns/LocalGit/quantex`, HEAD `f3d95aea`); kanıt yolları `services/` ile başlar.
Hiçbir servis çalıştırılmadı, derlenmedi, canlı bağlantı kurulmadı. "Koddan çıkarım"
diye işaretli davranışlar kod izlenerek bulundu, çalıştırılarak doğrulanmadı.

Bu belge üç alt görevin (T4a fiyat, T4b akış, T4c derinlik) çıktılarının birleşimidir;
her bölüm kendi kısaltmalarını başında tanımlar. Bağlantı ayrıntısı `CONNECTIVITY.md`,
NATS konuları `NATS.md`, anomali motoru `ANOMALY.md`, Qapsule için anlamı
`INTEGRATION.md`.

## Özet

| Servis | Göstergeler | Abonelik | Yayın | Qapsule için hüküm |
|---|---|---|---|---|
| Fiyat (`quantex_indicators_price_cpp`) | 30 sınıf (36 ad): SMA, EMA, RSI, MACD, BB, ATR, STOCH, ADX, Ichimoku, Supertrend, VWAP… Parametre serbest (sınırlar içinde), iç içe gösterge var; 44 hazır sinyal, 27 formasyon | NATS `price.cmd.*` → bağlayıcı → iç MQTT → motor | Her birleştirilmiş tikte (100 ms), iç MQTT'den bağlayıcıyla NATS'a | Tanımlar SMA/EMA/RSI/ATR'de aynı; ama bar hizası UTC epoch'a (K14'e aykırı), 300 bar tavanı, ısınmada 0 (K13'e aykırı), `bandwidth` ×100, parametreler tam sayıya kesiliyor. **Referans ya da yerine koyma için güvenilmez.** |
| Akış (`quantex_indicators_flow_cpp`) | 6: CUMULATIVE_DELTA, BUY_SELL_RATIO, TRADE_FLOW, VWAP_TICK, PGC_MOMENTUM, PGC_TREND; 12 sinyal | NATS `flow.cmd.*` | İşlem/PGC iletisi başına, sembol başına en çok 100 ms'de bir | Qapsule'de karşılığı yok (VWAP, işlem tabanlı hesap yok); PGC (kurumsal para akışı, AKD) lisanslı veri; sinyallerin yarısı yer tutucular çözülmediği için tetiklenmiyor (koddan çıkarım). |
| Derinlik (`quantex_indicators_depth_cpp`) | 5: ORDER_BOOK_IMBALANCE, BID_ASK_SPREAD, DEPTH_PRESSURE, WEIGHTED_MID_PRICE, CUMULATIVE_DEPTH; 7 sinyal | NATS `depth.cmd.*` | Derinlik iletisi başına, sembol başına en çok 100 ms'de bir | Qapsule'de derinlik hesabı yok; her ileti tam tablo sayılıyor (artımlı güncelleme alanları okunmuyor); sinyallerin çoğu çalışmıyor görünüyor (koddan çıkarım). |

Ortak bulgular (üç serviste aynı kod iskeleti):

- Motorlar Matriks MQTT'ye **kendileri** bağlanır; sonuçlarını iç bir MQTT aracısına JSON
  olarak yazar; `*-indicator-adapter` bunları NATS'a protobuf zarfla köprüler (`NATS.md`).
- Hesap sahipliği Redis'te tek sahipli anahtarla kopyalar arasında paylaşılır; Redis'e
  gösterge değeri yazılmaz.
- Hesapta LLM ve rastgelelik yok; ama çıktı duvar saatine (tik zamanı = alış anı, 100 ms
  kısma) bağlı, yeniden oynatmada aynı üretilemez (K3/K44 açısından referans değildir).
- Lisans/paket denetimi yok; JWT varsayılan kapalı, açıkken yalnız istek sıklığı katmanını
  seçiyor (`MESSAGES.md` §lisans, `INTEGRATION.md`).
- "Çapraz gösterge" adıyla ayrı bir abonelik yok: fiyat servisinde iç içe gösterge
  (`SMA[EMA[RSI:14]:20]:20` biçimi) ve sinyal ifadeleri, akış servisinde fiyat motoruna
  köprü (bugün beslenmiyor) var.

### Qapsule `qp-compute` ile sayısal fark tablosu (kısa)

Ayrıntı ve satır kanıtları aşağıda, T4a bölümünün 3. kısmında ("Sayısal tanım karşılaştırması").

| Gösterge | Tanım aynı mı | Fark | Risk |
|---|---|---|---|
| SMA | Evet | Toplama sırası (yeniden toplama ↔ kayan toplam): ulp; ısınmada 0.0 ↔ `null` | Düşük / ısınmada orta |
| EMA | Evet (α=2/(p+1), SMA tohumu) | Özyineleme ifadesi ulp; 300 bar penceresi uzun p'de tohum etkisini sürdürür | Düşük / p ≳ 100'de yüksek |
| RSI | Evet (Wilder, ilk p değişimin ortalaması) | Hiç hareket yokken 100 ↔ 50; girdi tam 0 ise "ilk bar" gözcüsü | Orta |
| MACD | Evet | Isınmada Quantex sinyal=0, histogram=macd yayınlar; Qapsule `null` | Isınmada yüksek |
| Bollinger | σ ikisinde de popülasyon (/N) | `bandwidth` Quantex'te ×100 (yüzde), Qapsule oran; `k` tam sayıya kesilir (2.5 → 2) | Yüksek |
| ATR | Evet | Isınmada TR yayınlanır | Düşük / orta |

---

## RI1/T4a — Fiyat gösterge servisi (`quantex_indicators_price_cpp`)

Kaynak: `/Users/arkns/LocalGit/quantex` (salt okunur; yalnız okuma ve `grep`/`sed -n`).
O depoda derleme, test, betik çalıştırılmadı; canlı bağlantı kurulmadı. Aşağıdaki her
davranış koddan okunmuştur; "koddan çıkarım" diye işaretlenenler kodun izlenmesiyle
bulunmuş, çalıştırılarak doğrulanmamıştır.

Kısaltmalar (dosya:satır kanıtlarında):

- `P/` = `services/quantex_indicators_price_cpp/`
- `A/` = `services/quantex-stream/price-indicator-adapter/src/`
- `S/` = `services/quantex-stream/shared/proto/`
- `Q/` = Qapsule `crates/qp-compute/src/` (bu depo)

Okunanlar: `P/README.md`, `P/CHANGELOG.md` (yalnız arama), `P/DEVAM_DOKUMANI.md`,
`P/src/main.cpp`, `P/src/core/types.hpp`, `P/src/core/ring_buffer.hpp`,
`P/src/indicators/**` (SMA, EMA, RSI, MACD, BB, ATR, Donchian, StdDev satır satır; diğerleri
parametre, çıktı ve ısınma için), `P/src/engine/*`, `P/src/signals/*`,
`P/src/patterns/pattern_factory.hpp`, `P/src/services/{market_clock,mqtt_manager,redis_service,matriks_client}.cpp`,
`P/src/security/validation.cpp`, `P/src/config/config.hpp`, `P/proto/Symbol.proto`,
`P/tests/*` (ne test ettiği), `A/**`, `S/indicator.proto`. Qapsule tarafında
`Q/studies/*.rs` ve `Q/lib.rs` satır satır okundu.

---

### 0. Mimari özet (bu belgenin geri kalanı için)

```
Matriks MQTT (mx/symbol/<SYM>, mx/timestamp)            Matriks REST (geçmiş bar, seans saati, tatil)
        │ protobuf SymbolMessage / TimeMessage                    │
        ▼                                                         ▼
  C++ fiyat motoru (her replika kendi Matriks MQTT bağlantısını açar)
  Tick → TickMerger → BarAggregator(sembol:periyot) → Gösterge(ler) → yayın
        │ JSON, MQTT QoS 0, iç Mosquitto
        ▼
  quantex/price/indicator/update/<key>, quantex/price/signal/..., quantex/price/pattern/data/...
        │
        ▼
  price-indicator-adapter (TS): MQTT → protobuf Envelope → NATS  (price.BIST.<SYM>.data.<key> ...)
  ters yön: NATS price.cmd.* / price.signal.cmd.* (protobuf) → MQTT JSON komut
```

- Motor, Matriks'e kendisi bağlanır: istemci kimliği `quantex-ind-mx-<engine_id>-<zaman>`
  (`P/src/services/mqtt_manager.cpp:85`), sembol konusu `mx/symbol/<SYM>`
  (`mqtt_manager.cpp:392-394`), piyasa saati konusu `mx/timestamp` (`mqtt_manager.cpp:123-126`).
  Bu, Arkin'in 2. bilgisi (yayın tipi başına tek bağlantı) açısından T1'in konusudur: fiyat
  motorunun her replikası (ve akış/derinlik motorları) ayrı Matriks MQTT bağlantısı açıyor
  görünüyor. Bilinmiyor — aynı kimlik bilgisiyle kaç bağlantı açıldığı dağıtım
  yapılandırmasına bağlı; T1 / Quantex ekibine sorulur.
- Motor NATS'a doğrudan yayın yapmaz; NATS köprüsü bağlayıcıdadır (`A/nats/nats.publisher.ts:58-146`).

---

### 1. Gösterge listesi

#### 1.1 Genel davranış (bütün göstergeler için geçerli)

| Konu | Davranış | Kanıt |
|---|---|---|
| Kayıtlı adlar | 36 ad (takma adlarla): RSI, STOCH/STOCHASTIC, SMA, EMA, WMA, MACD, ATR, BB/BOLLINGER/BOLLINGERBANDS, CCI, ROC, DEMA, OBV, VWAP, CMF, WLR/WILLIAMSR, MFI, TEMA, ADX, KELTNER, DONCHIAN, STDDEV, SUPERTREND, ICHIMOKU, PIVOT/PIVOTPOINTS, AROON, HMA, KAMA, MOST, PSAR/PARABOLICSAR, VORTEX. 30 sınıf. | `P/src/engine/indicator_factory.hpp` `register_indicator(...)` çağrıları; `P/src/indicators/**` |
| Belge çelişkisi | `DEVAM_DOKUMANI.md:43` "31 indikatör, 25 formasyon" diyor; kodda 30 sınıf ve 27 formasyon kimliği var. Kod esas. | `P/src/patterns/pattern_factory.hpp` |
| Periyotlar (doğrulayıcının kabul ettiği) | `1m, 5m, 10m, 15m, 30m, 1h, 2h, 4h, 1d, 1w, 1M` | `P/src/security/validation.cpp:112-116` |
| Periyotlar (gerçekte hesaplanan) | `Period::parse` yalnız `1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w` tanıyor; tanımadığını **5m** yapıyor. Geçmiş isteği de tanımadığını `5min` yapıyor. Yani `10m`, `2h`, `1M` aboneliği kabul edilir, anahtarında `10m` yazar, ama **5 dakikalık barlarla** hesaplanır (koddan çıkarım). | `P/src/core/types.hpp:222-235` (`:235` varsayılan 5m), `:241-254` (`:254` "5min") |
| Semboller | Sabit liste yok; abonelikle. Sembol `^[A-Z0-9]{2,10}$`. İlk abonelikte o sembol:periyot için bir BarSeries kurulur, Matriks'ten geçmiş çekilir ve `mx/symbol/<SYM>`'e abone olunur. Abone kalmayınca boşta temizlik (varsayılan 5 dk). | `validation.cpp:118`; `calculation_registry.cpp:1288-1387`; `config.hpp:55`; `calculation_registry.cpp:649-716` |
| Fiyat girdisi | Tick fiyatı `last`; yoksa `dayClose` (proto açıklaması: "bir önceki günün kapanış fiyatı"). Hacim `incrementalQuantity`, yoksa `volume` ("işlem hacmi"). Barın O/H/L/C'si yalnız tick fiyatından kurulur (mesajdaki gün içi high/low kullanılmaz). | `mqtt_manager.cpp:539-553`; `P/proto/Symbol.proto:16-22,82`; `types.hpp:52-63`; `bar_aggregator.cpp:354-364` |
| Bar geçmişi | Abonelikte Matriks REST'ten `count=BAR_HISTORY_SIZE` (varsayılan 300) bar; bar halkası derleme zamanında **300**'e sabit (`RingBuffer<T, MaxSize = 300>`), ortam değişkeni artırsa da 300'ü geçmez (koddan çıkarım). | `config.hpp:54,199`; `matriks_client.cpp:25-46`; `ring_buffer.hpp:19`; `bar_aggregator.hpp:145` |
| Isınma | Gösterge, abonelik anında son ≤300 bar üzerinde baştan koşturulur (`update` her bar). | `calculation_registry.cpp:940-950` |
| Isınmada çıktı | `null` yok. Canlı yayında ısınmamış gösterge için `get_value()` gönderilir: SMA/EMA/RSI için **0.0**, MACD/BB için sıfır haritası, ATR için son TR, Stoch için 50/50, Vortex 1/1, Aroon 50/50/0. Anlık görüntü (snapshot) geçmişine ise çoğu gösterge yalnız ısındıktan sonra kayıt yazar; istisnalar aşağıda. | `calculation_registry.cpp:1626-1643` (özellikle `:1642`); `sma.hpp:74`, `ema.hpp:86`, `rsi.hpp:110`, `macd.hpp:178`, `bollinger_bands.hpp:128`, `atr.hpp:43-44`, `stochastic.hpp:175`, `vortex.hpp:140`, `aroon.hpp:38` |
| Yayın sıklığı | **Her birleştirilmiş tikte** (bar kapanışını beklemez). TickMerger: fiyat değişince hemen; aynı fiyatsa 100 ms pencere dolunca bir sonraki tik geldiğinde. Zamanlayıcı yok (`flush()` hiç çağrılmıyor). | `calculation_registry.cpp:1649-1652`; `tick_merger.cpp` (`add_tick`); `config.hpp:53`; `grep tick_merger->flush` sonuçsuz |
| `is_closed` alanı | Temel göstergelerin canlı yayınında `is_closed = (events.back().type == CLOSED)`; ama `BarAggregator::update` her durumda son olayı `UPDATED` yapıyor. Sonuç: canlı yayında `is_closed` **hep false**; kapanan barın son değeri canlı akışta ayrı gönderilmez, yalnız snapshot geçmişinde görünür (koddan çıkarım). | `calculation_registry.cpp:1650`; `bar_aggregator.cpp:136-212` (her dal `UPDATED` ile biter) |
| Türetilmiş (iç içe) gösterge yayını | Kapanışta `is_closed=true` ve `time=closed_time`, ama değer canlı değerdir; kapanış dışı tiklerde `time = source_result.closed_time` = **0** (koddan çıkarım). | `calculation_registry.cpp:1790-1819` |
| Piyasa kapanışı | MX kaynaklı `mx/timestamp` "kapandı" geçişinde bütün BarSeries'lerin açık barı kapatılır ve göstergelere işlenir; **abonelere yayın yapılmaz**, sinyal/formasyon motoruna bildirilmez. | `market_clock.cpp:132-158`; `calculation_registry.cpp:1947-1991` |
| Sayı tipi | Her yerde `double` (f64). `float` yok. | `types.hpp:19-45,196`; gösterge sınıfları |

#### 1.2 Gösterge tablosu

Parametreler `get_param_bounds()` ile `{min, max, varsayılan}`; "Gerekli bar" `get_required_bars()`
(yalnız bilgi amaçlı, hesapta kullanılmıyor). "Snapshot ısınma" = geçmişe ilk kayıt ne zaman.

| Ad | Parametreler (min–max, vars.) | Çıktılar | Gerekli bar | Snapshot ısınma / ilk değer | Kanıt |
|---|---|---|---|---|---|
| SMA | length 1–500, 20 | tek | p | bar p-1'den itibaren | `trend/sma.hpp:21-68` |
| EMA | length 1–500, 20 | tek | p | bar p-1 (SMA tohumu) | `trend/ema.hpp:28-77` |
| WMA | length 1–500, 20 | tek | p | pencere dolunca | `trend/wma.hpp:24-96` |
| DEMA | length 1–500, 20 | tek | 2p | — | `trend/dema.hpp` |
| TEMA | length 1–500, 20 | tek | 3p | — | `trend/tema.hpp` |
| HMA | length 4–500, 20 | tek | p+√p | — | `trend/hma.hpp` |
| KAMA | length 2–100 (10), fast 1–50 (2), slow 5–200 (30) | tek | p+1 | — | `trend/kama.hpp` |
| MACD | fast 2–100 (12), slow 2–200 (26), signal 2–100 (9) | macd, signal, histogram | s+g | **bar s-1'den itibaren kısmi kayıt**: signal=0, histogram=macd (g-1 bar boyunca) | `trend/macd.hpp:85-89` |
| ADX | length 2–100, 14 | adx, plusDI, minusDI, dx | 2p+1 | — | `trend/adx.hpp:84-183` |
| AROON | length 2–200, 25 | aroonUp, aroonDown, oscillator | p+1 | — | `trend/aroon.hpp:107-109` |
| MOST | length 1–100 (3), percent 0.1–20 (2.0) | most, exMov, trend | p+10 | — | `trend/most.hpp:11-22,159` |
| PSAR | start 0.01–0.1 (0.02), increment 0.01–0.1 (0.02), max 0.1–0.5 (0.2) | tek | 2 | — | `indicator_factory.hpp:659-672` |
| VORTEX | length 2–100, 14 | plusVI, minusVI | p+1 | — | `trend/vortex.hpp:128-129` |
| RSI | length 2–200, 14 | tek | p+1 | bar p (p değişim sonrası) | `momentum/rsi.hpp:24-102` |
| STOCH | kPeriod 2–100 (14), kSmoothing 1–10 (1), dPeriod 1–50 (3) | k, d | k+s+d | — | `momentum/stochastic.hpp:30-162` |
| CCI | length 2–200, 20 | tek | p | — | `momentum/cci.hpp` |
| ROC | length 1–200, 12 | tek | p+1 | — | `momentum/roc.hpp` |
| WLR | length 2–200, 14 | tek | p | — | `momentum/williams_r.hpp` |
| MFI | length 2–200, 14 | tek | p+1 | — | `momentum/mfi.hpp` |
| ATR | length 1–200, 14 | tek | p+1 | bar p-1 | `volatility/atr.hpp:29-83` |
| BB | length 2–200 (20), stdDev 0.5–5.0 (2.0) | upper, middle, lower, bandwidth, percentB | p | bar p-1 | `volatility/bollinger_bands.hpp:34-121` |
| KELTNER | length 2–200 (20), atrPeriod 1–100 (10), multiplier 0.5–5.0 (2.0) | upper, middle, lower | max(p, atr) | — | `volatility/keltner.hpp` |
| DONCHIAN | length 2–200, 20 | upper, middle, lower | p | **bar 0'dan itibaren kısmi pencere** kaydı | `volatility/donchian.hpp:34-51` |
| STDDEV | length 2–200, 20 | tek | p | bar p-1 (popülasyon /N) | `volatility/stddev.hpp:24-77` |
| SUPERTREND | length 2–100 (10), multiplier 0.5–10 (3.0) | value, trend, upperBand, lowerBand | p+1 | — | `composite/supertrend.hpp:153-156` |
| ICHIMOKU | conversion 2–50 (9), base 5–100 (26), span 10–200 (52) | tenkanSen, kijunSen, senkouSpanA/B, chikouSpan, cloudTop, cloudBottom | span | — | `composite/ichimoku.hpp:133-139` |
| PIVOT | `type` dizgesi (standard/fibonacci/woodie/camarilla), sınır yok | pivot, r1–r3, s1–s3 | 1 | — | `composite/pivot_points.hpp:35-39,108`; `indicator_factory.hpp:582-588` |
| OBV | — | tek | 1 | bar 0 (birikimli) | `volume/obv.hpp:34-42` |
| VWAP | — | vwap, upper, lower | 1 | bar 0 (birikimli) | `volume/vwap.hpp:29-52` |
| CMF | length 2–100, 21 | tek | p | — | `volume/cmf.hpp` |

"—": bu görevde satır satır izlenmedi; ilk kayıt zamanı dosyadan okunmalı.

#### 1.3 Göstergelere özgü bulgular (koddan çıkarım)

1. **300 bar tavanı uzun periyotları imkânsız kılıyor.** SMA/EMA/WMA/DEMA/TEMA/HMA 500'e kadar
   kabul ediliyor ama bar halkası 300. `SMA:400`, `TEMA:101+` vb. hiç ısınmaz ve canlı akışta
   sürekli 0.0 yayınlar. EMA(200) ısınır ama yalnız 100 bar sonrası değer üretir ve tohumun
   etkisi `(1-2/201)^100 ≈ 0.37` oranında hâlâ durur: aynı veride uzun geçmişle hesaplayan
   bir motordan (Qapsule) belirgin farklı sonuç verir. (`ring_buffer.hpp:19`, `sma.hpp:68`,
   `ema.hpp:77`, `calculation_registry.cpp:946-948`.)
2. **`reset()` hiçbir göstergede ezilmemiş.** Taban sınıf "türetilmiş sınıflar MUTLAKA ezmeli"
   diyor (`base_indicator.hpp:60-79`), ama 30 dosyanın hiçbiri `reset() override` içermiyor.
   Boşluk doldurma (gap-fill) sonrası "yeniden kurma" `reset()` + son 300 barı yeniden besleme
   yapıyor (`calculation_registry.cpp:1893-1900`): iç durum (RSI ortalamaları ve
   `last_close_`, EMA, MACD tamponları, OBV/VWAP birikimleri) silinmeden aynı barlar ikinci kez
   işleniyor. OBV ve VWAP birikimli olduğu için seviye kayar; RSI'ye en yeni kapanıştan en
   eski kapanışa sahte bir "değişim" girer. Ayrıca snapshot geçmişine zamanı geriye giden
   kayıtlar eklenir (`base_indicator.hpp:175-182` yalnız son kayıtla aynı zamanı eler).
3. **VWAP oturum başında sıfırlanmıyor.** `reset_session()` tanımlı (`vwap.hpp:89`) ama hiçbir
   yerden çağrılmıyor (`grep reset_session` yalnız tanımı buluyor). VWAP, BarSeries'in tüm
   ömrü boyunca birikir; adıyla beklenen "günlük VWAP" değildir.
4. **Çıktı adları üst veri ile uyuşmuyor.** Çıktı seçici doğrulaması üst veriye bakıyor
   (`validation.cpp:61`), değer çıkarma ise gerçek haritaya; bulunamazsa haritanın
   **alfabetik ilk** alanını döndürüyor (`base_indicator.hpp:152-171`).
   - AROON üst verisi `up, down` (`indicator_factory.hpp:734`), gerçek alanlar
     `aroonUp, aroonDown, oscillator`. `AROON(up)` kabul edilir ve `aroonDown` döner.
   - VORTEX üst verisi `plus, minus` (`:736`), gerçek `plusVI, minusVI`; `VORTEX(plus)` →
     `minusVI`.
   - PIVOT üst verisi `pp, ...` (`:740`), gerçek `pivot, ...`.
   - VWAP üst verisi tek çıktı (`:742`), gerçek üç alanlı harita.
5. **Kesirli parametreler tam sayıya kesiliyor** (doğrudan istemci aboneliğinde):
   `validate_params` her parametreyi `static_cast<int>` ile yazıyor (`validation.cpp:301`) ve
   sınırdaki bütün parametreleri varsayılanıyla doldurduğu için PSAR'ın `start=0.02,
   increment=0.02, max=0.2` varsayılanları **0**'a iner; `BB stdDev 2.5` → 2, `0.5` → 0,
   `MOST percent 1.5` → 1, `KELTNER/SUPERTREND multiplier` aynı şekilde. Anahtar da tam sayıyla
   kurulur (`calculation_registry.cpp:2084-2086`). Sinyal motorunun iç abonelik yolu ise
   konumsal parametreleri kesmeden geçirir (`calculation_registry.cpp:1169-1219, 2477`), ama
   anahtarı yine tam sayıyla kurar: iç `BB:20:2.5` ile dış `stdDev=2` **aynı anahtarı**
   (`...:BB:20:2`) paylaşır ve hangisi önce kurulduysa onun hesabı herkese gider.
6. **Doğrudan abonelikte dizge içindeki konumsal parametreler yok sayılır.** `indicator="RSI:7"`
   ve `params={}` gönderilirse doğrulayıcı `length=14` varsayılanını koyar, anahtar
   `...:RSI:14` olur (`validation.cpp:225-235`; `calculation_registry.cpp:2046-2048` yorumu
   "konumsal parametreyi kullanma" diyor). Konumsal parametre yalnız iç içe göstergenin
   **iç** kısmında kullanılır (`calculation_registry.cpp:1027`). README örneği (`P/README.md`
   "Subscribe Command") doğru biçimi gösteriyor: ad + `params` JSON.
7. **MACD'de `fast < slow` denetimi yok** (`macd.hpp:25-31,142-147`); Qapsule hata verir
   (`Q/studies/macd.rs:36-40`).

---

### 2. Abonelik

#### 2.1 Gösterge aboneliği (istek biçimi ve konular)

NATS tarafı (bağlayıcının dinlediği):

| Yön | NATS konusu | Biçim | MQTT karşılığı | Kanıt |
|---|---|---|---|---|
| istemci → motor | `price.cmd.*` | protobuf `IndicatorCommand {action, symbol, period, indicator, params_json, reply_to, client_id}` | `quantex/price/indicator/cmd` (JSON: `action, symbol, period, indicator, params, replyTo, clientId, reassert`) | `A/nats/nats.cmd-listener.ts` (`CMD_SUBJECT`, `handleCommand`); `S/indicator.proto:52-60` |
| motor → istemci (canlı) | `price.<EXCH>.<SYM>.data.<key>` (EXCH varsayılan `BIST`) | Envelope içinde `IndicatorUpdate {key, symbol, indicator, period, engine_type, engine_id, timestamp, time, is_closed, value_json, output}` | `quantex/price/indicator/update/<key>` | `A/nats/nats.publisher.ts:85-90`; `A/transform/price.transform.ts:32-54`; `S/indicator.proto:15-27`; `calculation_registry.cpp:2111-2156` |
| motor → istemci (ilk görüntü) | `price.reply.<clientId>.<key>` | `IndicatorSnapshot {key, symbol, indicator, period, source, depth, output, history_json}` | istemcinin verdiği `replyTo` (bağlayıcı yalnız `quantex/price/indicator/reply/#` dinler) | `nats.publisher.ts:91-96`; `mqtt.subscriber.ts:10-11`; `calculation_registry.cpp:2165-2244` |
| motor durumu | `price.engine.status`, `price.engine.heartbeat` | — | `quantex/engine/status`, `quantex/engine/heartbeat` | `nats.publisher.ts:136-139`; `main.cpp:312-341` |

- `action`: `subscribe` (varsayılan; tanınmayan her eylem SUBSCRIBE sayılır), `unsubscribe`,
  `signal_*`, `pattern_*` (`calculation_registry.cpp:113-150`).
- Anahtar biçimi: `SEMBOL:PERİYOT:AD[iç]:p1:p2...` (`calculation_registry.cpp:2006-2105`),
  ör. `X:5m:RSI:14`, `X:5m:SMA[RSI:14]:20`. Parametre sırası sabit tabloyla.
- İstek alanları sınırlı: JSON en fazla 4096 bayt (`validation.hpp:113`), `replyTo` 200
  karaktere kırpılır (`validation.cpp:243`).
- Gösterge dizgesi sözdizimi: `AD(çıktı)[kaynak]:parametreler`; kaynak OHLCV alanı
  (`open/high/low/close/volume/hl2/hlc3/ohlc4`) ya da başka bir gösterge; en fazla 4 düzey
  iç içe; döngü denetimi var (`indicator_parser.hpp` başlık yorumu; `types.hpp:111-127`;
  `calculation_registry.cpp:1222`).
- Çok kopyalı çalışma: komut bütün replikalara gider; Redis `SET NX EX` ile tek sahip seçilir
  (`calc:<key>`, TTL 30 s, 10 s'de bir yenileme). Ağ geçidi 30 s'de bir `__reassert__`
  istemcisiyle aynı aboneliği yeniden gönderir (`calculation_registry.cpp:755,824`;
  `calculation_registry.hpp` `kClaimTtlSeconds=30`, `kClaimRefreshSeconds=10`;
  `nats.cmd-listener.ts` `REASSERT_CLIENT_ID`).
- **`period` alanı canlı güncellemede boş:** motor `publish_update` içinde `period`
  yazmıyor (`calculation_registry.cpp:2117-2126`); bağlayıcı `json.period` okuyor
  (`price.transform.ts:48`). Tüketici periyodu anahtardan ayrıştırmalı.
- **Belge çelişkisi:** README konuları `quantex/indicator/cmd`, `.../data/{KEY}` ve canlı
  güncellemede `closed` alanı diyor (`P/README.md` "Topics", "Live Update");
  `DEVAM_DOKUMANI.md:262-269` de `quantex/indicator/data/{key}`. Kod
  `quantex/price/indicator/cmd`, `quantex/price/indicator/update/<key>` ve `is_closed`
  kullanıyor (`mqtt_manager.cpp:228`; `calculation_registry.hpp` `data_topic_for`;
  `calculation_registry.cpp:2126`). `S/indicator.proto:58` yorumu `reply_to` için
  `quantex/indicator/reply/...` örneği veriyor; bağlayıcı yalnız `quantex/price/indicator/reply/#`
  dinliyor. Ağ geçidinin gerçekte hangi `reply_to`'yu verdiği bilinmiyor — T3 / Quantex ekibi.

**Hüküm:** isteğe bağlı parametreyle hesap yapılabiliyor (sabit küme değil): her (sembol,
periyot, gösterge dizgesi, parametre) birleşimi için talep anında bir hesap kurulur; aynı
anahtara gelen abonelikler paylaşılır. Sınırlar: parametre aralıkları, 300 bar, tam sayıya
kesme (1.3/5), tanınmayan periyodun 5m'ye düşmesi.

#### 2.2 Çapraz gösterge / sinyal aboneliği

| Yön | NATS | Biçim | MQTT | Kanıt |
|---|---|---|---|---|
| istemci → motor | `price.signal.cmd.*` | protobuf `SignalCommand {action, signal_id, instance_key, params_json, user_id, delivery_json, reply_to, client_id}` | `quantex/price/signal/cmd` (JSON; `user_id` snake_case) | `A/nats/nats.signal-cmd-listener.ts`; `S/indicator.proto:94-103` |
| motor → istemci | `price.signal.<EXCH>.<SYM>.<signalId>`, `price.signal.status.<instanceKey>`, `price.signal.state.<instanceKey>`, `price.signal.reply.<clientId>.<key>` | Envelope | `quantex/price/signal/{data,instance,status,state,reply}/...` | `nats.publisher.ts:97-125`; `signal_registry.cpp:1696-1730` |

- Eylemler: `signal_subscribe, signal_unsubscribe, signal_pause, signal_resume,
  signal_register, signal_unregister, signal_list, signal_stats, signal_instance_detail`
  (`calculation_registry.cpp:113-150`).
- Hazır (core) sinyal: **44** tanım — eşik 5, kesişim 14, formül 18, çoklu zaman dilimi 5,
  hacim 2 (`signal_registry.cpp:864-923`; `P/src/signals/definitions/**`).
- Kullanıcı tanımlı sinyal: `signal_register` ile JSON tanım; alanlar
  `signal_id, name, conditions[], triggers[], params[], eval_on_bar_close, cooldown_bars, ...`
  (`signal_registry.cpp:1806-1976`).
- Koşul türleri: `THRESHOLD`, `CROSSOVER` (gösterge–eşik ya da gösterge–gösterge;
  `CROSSES_ABOVE/BELOW/CROSSES`), `FORMULA` (ifade dili), `FORMATION` (formasyon sonucu),
  `ANOMALY` (anomali motoru `quantex/anomaly/data/<SYM>`) (`signal_types.hpp:71-226`).
- Gösterge referansı şablon: ör. `$SYMBOL:$PERIOD:EMA:$FAST_LEN`
  (`signals/definitions/crossover/ema_cross_9_21.hpp`); örnek başlatıldığında motor iç
  gösterge aboneliği açar (`main.cpp:194-205`; `calculation_registry.cpp:2466-2477`).
- İfade dili: `+ - * / % ^`, karşılaştırma, `&& || !`, değişken `$alias`, geçmiş
  `$alias[-n]`, `abs max min sqrt avg log sign pow` (`signals/expression_parser.hpp:10-45`).
- Örnek anahtarı: `signal_id` + sıralı parametre değerleri (`signal_types.hpp` yorumu;
  `signal_registry.cpp:932`).

Sinyal anlamıyla ilgili bulgular (koddan çıkarım; Qapsule'de "kesişim" kavramıyla aynı adı
taşıyıp farklı sonuç verme riski **yüksek**):

1. **"Önceki değer" önceki tikteki değerdir, önceki barınki değil.** `previous_values` ve
   `$x[-n]` geçmişi her güncellemede (her tik) kaydırılıyor (`signal_registry.cpp:494-504,
   1772-1785`), değerlendirme ise yalnız bar kapanışında (`:1339`, hazır sinyallerin hepsi
   `eval_on_bar_close = true`). Kapanış anında "önceki" değer, aynı barın son canlı tikindeki
   değerdir; bu da kapanış değerine (aynı bar verisinden) eşittir. Bu durumda
   `prev < hedef && şimdiki >= hedef` kesişimi bar kapanışında pratikte hiç sağlanamaz;
   `abs($rsi - $rsi[-5]) > 10` gibi formüller "5 bar önce" değil "5 tik önce"yi okur
   (`signals/definitions/formula/rsi_divergence.hpp:32`).
2. **Bekleme (cooldown) iki göstergeli sinyallerde iki kat hızlı azalır:** her göstergenin
   kapanış güncellemesi ayrı değerlendirme tetikliyor ve her kapanış değerlendirmesinde
   `cooldown_remaining--` (`signal_registry.cpp:1348-1355`; `calculation_registry.cpp:1661-1676`).
3. Piyasa kapalıyken (MX `isBistPPOpen=false`) değerlendirme yapılmaz
   (`signal_registry.cpp:1342`).

#### 2.3 Formasyonlar

27 kimlik (`pattern_factory.hpp`): DOJI, HAMMER, INVERTED_HAMMER, HANGING_MAN, SHOOTING_STAR,
BULLISH_ENGULFING, BEARISH_ENGULFING, MORNING_STAR, EVENING_STAR, THREE_WHITE_SOLDIERS,
THREE_BLACK_CROWS (mum); DOUBLE_TOP, DOUBLE_BOTTOM, HEAD_AND_SHOULDERS,
INVERSE_HEAD_AND_SHOULDERS, ASCENDING/DESCENDING/SYMMETRIC_TRIANGLE, RISING/FALLING_WEDGE,
BULL/BEAR_FLAG, CUP_AND_HANDLE, ASCENDING/DESCENDING_CHANNEL, RECTANGLE,
FIBONACCI_RETRACEMENT (grafik).

- Değerlendirme yalnız bar kapanışında, gösterge ile aynı BarSeries üzerinde
  (`calculation_registry.cpp:1679-1684, 1698-1703`; `pattern_registry.cpp`).
- Sonuç alanları: `detected, confidence, direction, status (forming/detected/confirmed/failed/expired),
  target_price, stop_loss, neckline` (`signal_types.hpp:188-195`; `signal_registry.cpp:1414-1452`).
- Yayın: `quantex/price/pattern/data/<instanceKey>` → NATS `price.pattern.<EXCH>.<SYM>.<instanceKey>`
  (`pattern_registry.cpp:537`; `nats.publisher.ts:126-131`).
- İstek: MQTT `quantex/price/pattern/cmd` ya da gösterge komut konusu, eylem `pattern_subscribe`,
  alanlar `pattern_id, symbol, period, userId, delivery, lookback, replyTo`
  (`calculation_registry.cpp:426-432`; `mqtt_manager.cpp:229-233`). **Bağlayıcıda NATS→MQTT
  formasyon komutu köprüsü yok**: `price.cmd.*` köprüsü `pattern_id` ve `lookback`'i
  taşımıyor (`nats.cmd-listener.ts` `mqttPayload`). NATS üzerinden formasyon aboneliğinin
  nasıl yapıldığı bilinmiyor — Quantex ekibine sorulur. Sinyal koşulu (`FORMATION`) yoluyla
  dolaylı abonelik var (`main.cpp:259-282`).

---

### 3. Sayısal tanım karşılaştırması (Quantex ↔ Qapsule `qp-compute`)

Qapsule'de olan çalışmalar: `sma, ema, rsi, macd, bollinger` (kayıtlı, `Q/lib.rs:240`) ve
olgu dedektörleri için `highest, lowest, atr` (`Q/studies/mod.rs:15-23`). Stochastic, WMA,
ADX vb. Qapsule'de yok; karşılaştırılamaz. İki tarafta da `f64`/`double`; ikisinde de
canlı bar için "durumu değiştirmeden hesapla" + "kapanışta işle" ayrımı var (Quantex
`calculate_live`/`update`, Qapsule `peek1`/`commit1`).

Kapalı barlar için aynı girdi dizisi verildiğinde:

| Gösterge | Quantex formülü (dosya:satır) | Qapsule formülü (dosya:satır) | Fark | "Aynı ad, farklı sonuç" riski |
|---|---|---|---|---|
| SMA | Her bar pencereyi `std::accumulate` ile baştan toplar (eskiden yeniye), `/p` (`trend/sma.hpp:28-35`). İlk değer bar p-1. Isınmada canlı yayın 0.0 (`calculation_registry.cpp:1642`). | Kayan toplam `(sum + x) - çıkan`, değer `(sum + x)/p` (`Q/studies/sma.rs:45-51,61-69`). Isınma p-1, `None`. | Toplama sırası farklı: Quantex her seferinde yeniden, Qapsule kayan. Sonuçlar ulp düzeyinde ayrışabilir (bit-eşit değil). Isınma: 0.0 ↔ `null`. | **Düşük** (değer); **orta** (ısınmada 0.0 gerçek değer gibi görünür). |
| EMA | α=2/(p+1) (`ema.hpp:24`); tohum = ilk p girdinin toplamı/p, bar p-1 (`:34-46`); sonra `(x - ema)·α + ema` (`:49`). | α=2/(p+1) (`Q/studies/ema.rs:29`); tohum aynı, bar p-1 (`:45-54,73-83`); sonra `x·α + ema·(1-α)` (`:52,80`). | Tanım aynı; özyineleme ifadesi cebirsel eşdeğer ama kayan nokta işlem sırası farklı → ulp farkı (EMA daraltıcı, büyümez). Asıl fark **girdi penceresi**: Quantex en fazla 300 bar görür, uzun p'de tohum etkisi sürer (§1.3/1). | **Düşük** (aynı geçmişte); **yüksek** (p ≳ 100 ve farklı geçmiş uzunluğunda). |
| RSI | Wilder. Değişim = x − önceki; kazanç `>0`, kayıp `<0 ? -d : 0` (`rsi.hpp:35-37`); tohum = ilk p değişimin toplamı/p (`:39-49`); sonra `(avg·(p-1) + g)/p` (`:52-53`); `100 − 100/(1+rs)` (`:63-64`). İlk değer bar p. `avg_loss == 0` → **100** (`:60-61`). İlk bar algılama `last_close_ == 0.0` gözcüsüyle (`:30-33`). | Wilder; aynı tohum ve özyineleme (`Q/studies/rsi.rs:159-176`, `:146-148`); aynı RSI ifadesi (`:40-49`). İlk değer bar p (`:187-189`). `avg_loss == 0` → `avg_gain == 0` ise **50**, değilse 100 (`:43`). İlk bar sayaçla (`n`). | (1) Hiç hareket yokken (kazanç=kayıp=0) Quantex 100, Qapsule 50. (2) Girdi tam 0.0 olursa Quantex onu "ilk bar" sayar (fiyatta olmaz; iç içe kullanımda — ör. `RSI[MACD...]` — MACD tam 0 olduğunda olur). (3) İşlem sırası aynı görünüyor → aynı girdide bit-eşit olması beklenir. Varsayım: C++ derleyicisi FMA birleştirmesi yapmıyor (x86-64 varsayılanında yok; derleme bayrakları `P/CMakeLists.txt`'te ayrıca `-march` görünmedi). | **Orta**: düz seride 100↔50; iç içe 0 gözcüsü. Normal fiyat serisinde düşük. |
| MACD | Hızlı EMA bar f-1'de SMA tohumuyla, yavaş bar s-1'de (`macd.hpp:43-78`); macd = hızlı − yavaş; sinyal = macd'nin EMA'sı, tohum ilk g macd değerinin ortalaması (`:91-102`); histogram = macd − sinyal (`:105`). EMA ifadesi `(x-e)·α+e`. | Aynı yapı: üç `Ema`, sinyal macd'nin ilk g değerinin ortalamasıyla tohumlanır (`Q/studies/macd.rs:51-80`); ısınma `[s-1, s+g-2, s+g-2]` (`:95-99`). | Tanım aynı. Isınmada fark büyük: Quantex macd ilk değerinden itibaren g-1 bar boyunca `signal = 0`, `histogram = macd` olarak **kaydediyor ve yayınlıyor** (`:85-89`); Qapsule `None`. `fast < slow` denetimi yalnız Qapsule'de. | **Yüksek** (ısınma bölgesinde histogram/sinyal tamamen farklı); kararlı bölgede düşük (ulp). |
| Bollinger | Orta = pencere toplamı/p; σ = √(Σ(x−orta)²/p) **popülasyon**, iki geçişli (`bollinger_bands.hpp:93-101`); bant = orta ± k·σ (`:104-105`); **bandwidth = (üst−alt)/orta × 100**, orta ≤ 0 ise 0 (`:108`); ek `percentB` (`:109`). İlk değer bar p-1. | Orta = `sma(p)`; σ popülasyon (/p), pencereli Welford M2 (`Q/studies/bollinger.rs:8-21, 91-107, 147-160`); düz pencerede tam 0 (`:100-104`); **bandwidth = 2kσ/orta** (oran), orta == 0 ise 0 (`:121-135`). Isınma p-1. | σ tanımı aynı (/N). Hesap yolu farklı (iki geçiş ↔ Welford) → ulp farkı. **bandwidth 100 kat farklı** (yüzde ↔ oran) ve sıfır koruması farklı (≤0 ↔ ==0). `k` Quantex doğrudan abonelikte tam sayıya kesilir (§1.3/5). Alan sırası/adları farklı (`upper,middle,lower,bandwidth,percentB` ↔ `middle,upper,lower,bandwidth`). | **Yüksek** (`bandwidth` aynı ad, 100× fark; `k=2.5` → 2). Bantlar düşük. |
| ATR | TR₀ = h−l (ilk bar, `prev_close_ == 0` gözcüsü) (`atr.hpp:90-101`); tohum = ilk p TR'nin ortalaması, bar p-1 (`:47-55`); Wilder `(atr·(p-1)+tr)/p` (`:58`). Isınmada `value_ = tr` canlı yayınlanır (`:42-44`), canlı ısınma `h−l` (`:65-66`). | TR₀ = h−l, sonra `max(h−l, |h−pc|, |l−pc|)` (`Q/studies/atr.rs:12-25`); tohum ilk p TR ortalaması bar p-1; Wilder aynı (`:88-100`). | Kapalı barlarda tanım ve işlem sırası aynı → aynı girdide bit-eşit beklenir. Fark: ısınmada Quantex sayı (TR) yayınlar, Qapsule `None`. | **Düşük** (kapalı bar); **orta** (ısınma). |
| Highest/Lowest ↔ DONCHIAN | upper = son p yükseğin en büyüğü, lower = son p düşüğün en küçüğü, middle = (u+l)/2; **pencere dolmadan kısmi kayıt** bar 0'dan (`donchian.hpp:37-50`). | `highest(p)`/`lowest(p)` tek girdi, monoton kuyruk; ısınma p-1, `None` (`Q/studies/extremum.rs`; `mod.rs:21-22`). | Değerler pencere dolunca eşit (min/max tam). Isınmada Quantex kısmi pencere değeri. Qapsule'de `middle` yok. | **Düşük** (dolu pencere); **orta** (ısınma). |
| StdDev | Popülasyon /N, iki geçişli (`stddev.hpp:66-77`). | Ayrı çalışma yok; Bollinger içindeki σ aynı tanım. | — | Düşük. |

Girdi serisinden gelen farklar (tanımlar aynı olsa da sonuçları ayırır):

| Konu | Quantex | Qapsule | Risk |
|---|---|---|---|
| Geçmiş uzunluğu | ≤300 bar, her abonelikte son 300 (`ring_buffer.hpp:19`) | tüm seri | EMA/MACD (uzun p) için yüksek |
| Boş periyot | Seans içindeki işlemsiz periyotlar için O=H=L=C=önceki kapanış, V=0 bar eklenir (`bar_aggregator.cpp:151-174, 215-266`) | Bilinmiyor bu görevde — Qapsule bar kaynağı (`docs/DATA_ACCESS.md`) boş periyot üretmiyor görünüyor; ana akış doğrulamalı | SMA/RSI'de orta (sahte "0 değişim" barları RSI'yi yumuşatır) |
| Hacim birimi | Geçmiş: REST `volume` (TL tutarı; `matriks_client.cpp:91`, Qapsule `docs/DATA_ACCESS.md:113` aynı alanı "TL işlem tutarı" diye tanımlıyor). Canlı: `incrementalQuantity` (Varsayım: adet) (`mqtt_manager.cpp:550-553`) | `v` = TL tutarı | Hacimli göstergelerde (OBV, MFI, CMF, VWAP) **yüksek**: aynı seride iki birim |
| Fiyat | `last`; yoksa önceki günün kapanışı `dayClose` | bar kapanışı | Kısmi güncellemede `last` yoksa önceki gün fiyatıyla sahte tik — orta |

---

### 4. Bar hizası (K13 / K14 uyumu)

| Konu | Quantex | Kanıt | K13/K14 ile |
|---|---|---|---|
| Bar başlangıcı | `floor(epoch_ms / periyot_ms) · periyot_ms` — **UTC epoch'a** hizalı, borsa takvimine değil | `bar_aggregator.cpp:350-352` | Dakika ve 1h barlar İstanbul (UTC+3) saatleriyle örtüşür. **4h**: UTC 0/4/8/12/16 → İstanbul 03/07/11/15/19; seans 10:00'da başlarken ilk bar 07:00 etiketli. **1d**: İstanbul 03:00 etiketli (Qapsule K14: 10:00 İstanbul = 07:00 UTC). **1w**: epoch Perşembe başladığı için **Perşembe 03:00** hizalı haftalar. K14'e uymuyor. |
| Tick zamanı | Mesaj alındığı andaki **duvar saati** `now_ms()`; Matriks'in `updateDate`'i ya da `mx/timestamp` kullanılmıyor | `mqtt_manager.cpp:533`; `Symbol.proto:11`; `market_clock.cpp` (`exchange_timestamp_us_` yalnız saklanıyor) | Bar ataması ağ gecikmesine ve sunucu saatine bağlı. |
| Bar kapanışı | Yeni periyoda düşen ilk tik gelince; zamanlayıcı yok. Seans kapanışında MX "kapandı" olayıyla açık bar kapatılır | `bar_aggregator.cpp:143-190`; `calculation_registry.cpp:1947-1991` | İşlem görmeyen sembolde bar geç kapanır. Seans kapanışında **bütün** periyotların açık barı kapanır: 1w (ve kısmen 4h) barı her gün kapanıp ertesi gün **aynı zaman damgasıyla** yeniden açılır (koddan çıkarım: `close_current_bar` `current_bar_`'ı sıfırlar, sonraki tik "ilk tik" dalından aynı `bar_start` ile yeni bar kurar, `bar_aggregator.cpp:136-140, 316-326`). |
| Kapanış seansı | Seans pencereleri `/v1/session-hours` yanıtındaki `BISTPP` dizisinden (en fazla iki pencere); yalnız boşluk doldurmada kullanılıyor. Kapanış seansı tikleri kendi saatlerinin barına düşer, son sürekli işlem barına **birleştirilmez** | `market_clock.cpp:178-212`; `bar_aggregator.cpp:59-83` | K14 (kapanış seansı son bara birleşir) ile **uyumsuz**. |
| Tatil / hafta sonu | Matriks tatil API'sinden önceki–sonraki yıl aralığı bir kez yüklenir (hata olursa boş küme, yeniden denenmez); hafta sonu sabit Cmt/Paz | `calculation_registry.cpp:1259-1283`; `bar_aggregator.cpp:66-74` | Yalnız boşluk doldurmayı etkiler. Yarım gün bilgisi yok. |
| Saat dilimi | `bar_aggregator`: sabit UTC+3. `market_clock`: `localtime_r` (kapsayıcının yerel saat dilimi). Geçmiş tarih dizgesi `mktime` (yerel saat dilimi) | `bar_aggregator.hpp:154`; `market_clock.cpp:42,317`; `matriks_client.cpp:320-344` | Kapsayıcı UTC'de çalışırsa seans saati yedek denetimi ve dizge tarih ayrıştırması 3 saat kayar (Varsayım: kapsayıcı TZ ayarı bilinmiyor). |
| Geçmiş barların zaman etiketi | Matriks REST'in `time`/`date` alanı olduğu gibi alınır; canlı hizalamayla aynı noktaya düştüğü denetlenmiyor | `matriks_client.cpp:109-121` | Bilinmiyor: REST'in gün içi/günlük barı hangi ana etiketlediği (Qapsule `docs/DATA_ACCESS.md:104-110` "gün içi `time` = bar başlangıcı" diyor; günlük için ayrı kural uyguluyor). Uyuşmazsa geçmiş ile canlı arasında kayık bar ya da çift gün oluşur. |
| Yinelenen geçmiş bar | `std::sort` (kararlı değil) + `std::unique`: yorum "sonuncuyu tut" diyor, kod **ilkini** tutar; eşitlerden hangisinin önde olduğu belirsiz | `matriks_client.cpp:131-139` | Belirlenimsizlik kaynağı. |
| K13 (aynı uzunluk, ısınma `null`) | Çıktı bar serisiyle aynı uzunlukta değil; snapshot yalnız kaydedilen noktaları (çoğunda ısınma sonrası) zaman damgasıyla verir; canlı akışta ısınma 0 ya da kısmi değer | §1.1, §3 | Qapsule'e alınırken **zamana göre hizalama** ve ısınmanın `null`'a çevrilmesi gerekir; 0.0 değerleri ısınma ile ayırt edilemez. |

---

### 5. Performans ve ölçek

| Konu | Bulgu | Kanıt |
|---|---|---|
| Belgede yazan hedefler | "< 1 ms tick işleme", "< 0.1 ms gösterge/bar", "~50 KB/hesap", "10.000+ eşzamanlı hesap", "100.000+ tick/s" | `P/README.md` "Performance" |
| Ölçüm kanıtı | Kodda ya da depoda bu sayıları destekleyen ölçüm sonucu yok. `tests/go-load/README.md` bir yük testi aracını ve **örnek** çıktı bloğunu gösteriyor; kaynağı/tarihi belirtilmemiş. Prometheus histogramları var (tick süresi, hesap süresi) — değerler canlı sistemde. | `P/tests/go-load/README.md`; `calculation_registry.cpp:1629-1636, 1720-1726` |
| Birim testleri | `BUILD_TESTS` seçeneği `tests/test_ring_buffer.cpp` ve `tests/test_bar_aggregator.cpp` istiyor; bu dosyalar depoda yok. Gösterge sayısal doğruluğu için birim testi yok; `tests/*.py` canlı MQTT üzerinden "yanıt geliyor mu" denetleyen el betikleri. | `P/CMakeLists.txt:196-209`; `ls P/tests`; `P/tests/test_indicators.py:1-20` |
| İş parçacığı modeli | Tek **tick işçisi** bütün sembolleri sırayla işler; tek komut işçisi; temizlik ve sahiplik yenileme işçileri; MQTT yayın işçisi; Matriks bekçisi; boşluk doldurma her istek için ayrı kopuk (`detach`) iş parçacığı. Tüm haritalar tek `data_mutex_` altında. | `calculation_registry.cpp:156-159, 543-558, 1707-1714`; `mqtt_manager.cpp:31-41` |
| Kuyruklar | Yayın kuyruğu en fazla 10.000 ileti; dolunca yeni ileti **atılır**. MQTT QoS 0. | `config.hpp:66`; `mqtt_manager.cpp:286-297, 321` |
| Bellek sınırı | Bar 300, gösterge geçmişi 300, formasyon geçmişi 50. MACD'nin `price_buffer_`/`macd_buffer_` tamponları hiç kırpılmıyor (sınırsız büyür); VWAP `tp_buffer_` için kırpma görülmedi (yalnız `update` gövdesi okundu). | `ring_buffer.hpp:19`; `patterns/base_pattern.hpp:330`; `macd.hpp:40,82,175-176` |
| Ölçek modeli | Çok kopya; her hesap anahtarı Redis'te tek sahipli (`calc:<key>`), abone sayacı `subs:<key>`, motor sıra numarası `engine:seq:price`. | `redis_service.cpp:100-300`; `main.cpp:90-109` |
| Redis'e ne yazılıyor | Yalnız koordinasyon: sahiplik, abone sayacı, motor sırası, hız sınırı (`zadd`). **Gösterge değeri ya da geçmiş yazılmıyor.** | `redis_service.cpp` (bütün yazma çağrıları); `grep redis_service_\.` (engine/security) |
| Hız sınırı | Katman başına dakikada 10/30/100 istek (README); kodda varsayılan açık. JWT varsayılan kapalı. | `config.hpp:78,88`; `P/README.md` "Security" |
| Sembol sayısı sınırı | Kodda sabit üst sınır yok; pratik sınır tek tick işçisi ve tek kilit. | — |

---

### 6. Determinizm

| Kaynak | Durum | Kanıt |
|---|---|---|
| LLM / ağdan hesap | Yok. Hesap kodu saf C++. | `P/src/indicators/**`, `P/src/signals/**` |
| Rastgelelik | Yalnız yapılandırmada cihaz kimliği üretimi için `mt19937`; hesapta yok. | `config.hpp:163-180` |
| Duvar saati | **Bar ataması duvar saatine bağlı** (tik zamanı alış anı). Bar kapanışı sonraki tikin gelişine bağlı. Tick birleştirme `steady_clock` 100 ms penceresine bağlı → hangi tikin yayınlandığı ve birikmiş hacmin hangi bara düştüğü zamanlamaya bağlı. | `mqtt_manager.cpp:533`; `tick_merger.cpp` |
| Geçmiş penceresi | Aynı sembolde gösterge değeri, aboneliğin **ne zaman** kurulduğuna (hangi 300 bar) ve sonradan boşluk doldurma olup olmadığına bağlı (`reset` eksikliği, §1.3/2). Yeniden başlatma sonrası EMA/MACD değerleri kayabilir. | `calculation_registry.cpp:940-950, 1842-1944` |
| Sıralama | Göstergeler `unordered_set`/`unordered_map` sırasıyla işlenir; değerleri etkilemez, ama sinyal değerlendirmesinin hangi göstergenin güncellemesiyle tetiklendiğini etkiler (§2.2/2). | `calculation_registry.cpp:1609-1647, 1661-1676` |
| Sonuç | Aynı tik akışı iki kez oynatılsa bile aynı bar serisi garanti değil. Qapsule'ün "aynı girdi → aynı çıktı" değişmeziyle (CLAUDE.md) bu servisin çıktısı **referans olarak kullanılamaz**; yalnız canlı gösterim amaçlı tüketilebilir. | — |

---

### Arkin'in 5. bilgisi (gösterge ve çapraz gösterge aboneliği) — fiyat servisi yanı

**Kısmen doğrulandı.**

- Doğru: fiyat göstergelerine abonelikle dinleme yapılabiliyor. İstek NATS `price.cmd.*`
  (protobuf `IndicatorCommand`), canlı veri NATS `price.BIST.<SYM>.data.<key>`, ilk görüntü
  `price.reply.<clientId>.<key>`. Parametre serbest (aralıklar içinde), iç içe gösterge ve
  OHLCV kaynağı seçilebiliyor.
- Doğru: "çapraz gösterge" sinyal motoru olarak var: NATS `price.signal.cmd.*` ile 44 hazır
  ya da kullanıcı tanımlı sinyal, kesişim/eşik/formül/formasyon/anomali koşulları; sonuçlar
  `price.signal.BIST.<SYM>.<signalId>`.
- Düzeltme: motor NATS'a **doğrudan yayın yapmıyor**; iç Mosquitto MQTT'ye JSON yayınlıyor,
  `price-indicator-adapter` NATS'a köprülüyor. Formasyon aboneliği için NATS köprüsü yok.
- Uyarı (Qapsule için): sayısal tanımlar SMA/EMA/RSI/ATR'de büyük ölçüde aynı, ama bar
  hizası (UTC epoch, duvar saati, kapanış seansı birleşmemesi), 300 bar tavanı, ısınmada 0
  yayını, `bandwidth` ölçeği, parametrelerin tam sayıya kesilmesi, desteklenmeyen
  periyotların sessizce 5m olması ve kesişim sinyallerinin tik-düzeyi "önceki" değeri
  nedeniyle bu servisin değerleri Qapsule'ün hesaplarıyla **karşılaştırma ya da yerine koyma
  için güvenilir değil**. Qapsule göstergeleri kendi çekirdeğinde hesaplamaya devam etmeli;
  bu servis en fazla "Quantex'te ne görünüyor" karşılaştırması için okunabilir.

### Açık sorular

| # | Soru | Kime |
|---|---|---|
| 1 | Ağ geçidi `IndicatorCommand.reply_to` için hangi konuyu veriyor (`quantex/price/indicator/reply/...` mi, proto yorumundaki `quantex/indicator/reply/...` mi)? | Quantex ekibi / T3 |
| 2 | `10m`, `2h`, `1M` periyotları üretimde isteniyor mu; 5m'ye düşme biliniyor mu? | Quantex ekibi |
| 3 | Her fiyat motoru replikası ayrı Matriks MQTT bağlantısı açıyor; Arkin'in "tek bağlantı" kuralıyla nasıl bağdaşıyor (ayrı kimlik bilgisi mi)? | Arkin / T1 |
| 4 | Matriks REST geçmiş barlarının zaman etiketi (bar başlangıcı mı, bitişi mi; günlük bar hangi saat) canlı hizalamayla örtüşüyor mu? | Matriks / T2 |
| 5 | `incrementalQuantity` adet mi, TL mi? Geçmişte `volume` TL ise canlı ile birim karışıyor mu? | Matriks / T2 |
| 6 | Kesişim sinyalleri üretimde gerçekten tetikleniyor mu (bulgu §2.2/1 koddan çıkarım; canlı kayıt ile doğrulanmalı)? | Quantex ekibi |
| 7 | NATS üzerinden formasyon aboneliği hangi yoldan yapılıyor? | Quantex ekibi |
| 8 | Kapsayıcı saat dilimi (UTC mi İstanbul mu)? `localtime_r`/`mktime` kullanan yerleri etkiler. | Quantex ekibi (dağıtım) |

---

## RI1/T4b — Akış gösterge servisi (`quantex_indicators_flow_cpp`)

Kaynak: `/Users/arkns/LocalGit/quantex` (salt okunur; yalnız okuma ve arama). Aşağıdaki
yollar, aksi yazılmadıkça `services/quantex_indicators_flow_cpp/src/` altındadır; bağlayıcı
için `services/quantex-stream/flow-indicator-adapter/src/`. Hiçbir şey derlenmedi ya da
çalıştırılmadı: "çalışmaz" diye işaretlenen her şey **kod okumasından çıkarımdır**,
canlıda doğrulanmadı. Gizli değer içeren dosya açılmadı; formüller kendi gösterimimle
yazıldı.

### 0. Özet

- Servis iki Matriks yayınından beslenir: `mx/trade/{sembol}` (tek tek işlemler) ve
  `mx/pgc/{sembol}` (Para Giriş Çıkışı, aracı kurum dağılımından türetilmiş kurumsal akış).
  Ayrıca `mx/timestamp`'i borsa açık/kapalı durumu için dinler
  (`services/mqtt_manager.cpp:125`, `:397`).
- **Altı gösterge** (dördü işlem, ikisi PGC) ve **on iki çekirdek sinyal** (yedi akış, beş
  çapraz) var. Gösterge yok bar; her şey tik ya da PGC güncellemesi başına, sembol başına
  100 ms kısmayla yayınlanır.
- Abonelik istek-cevap biçimindedir; parametre kümesi küçüktür (`period`, `pgcType`) ve
  sınırları **denetlenmez**. Özel sinyal tanımı için komut yolu yoktur.
- Sinyal motorunda kod okumasıyla görülen ciddi kusurlar var: on iki sinyalden altısının
  formülü parametre değişkenlerini çözemediği için hiç tetiklenmez; soğuma (cooldown) hiç
  bitmez; tek yönlü kesişim sinyalleri iki yönü birden yayınlar; çapraz sinyallerin fiyat
  tarafı anahtar uyuşmazlığı nedeniyle beslenmez (§2.3).
- Lisans/yetki denetimi servis içinde **yok**: JWT varsayılan kapalı, açıkken de yalnız
  kimlik ve "tier" okunur; PGC (AKD/AKDE lisanslı) ile açık işlem verisi ayrılmaz (§5).
- Qapsule'ün `flow.institutional` dedektörü kavram olarak PGC göstergelerine en yakın olan;
  ama tanımlar farklı (bar hizalı, gözlem penceresi ortalamasına oran vs. ardışık PGC farkının
  mutlak eşiği). `qp-compute`'ta VWAP ya da işlem tabanlı hesap yok (§4).

### 1. Göstergeler

#### 1.1 Girdiler

| Girdi | Konu | Alanlar (proto) | Not |
|---|---|---|---|
| İşlem (`TradeMessage`) | `mx/trade/{sembol}` | `symbol`, `orderNo`, `price` (**float32**), `quantity` (int64), `activeBidOrAsk` (dize), `timestamp` (int64), `buyer`, `seller`, `isTradeEx` | `proto/Trade.proto:8-18`. `buyer`/`seller` (aracı kurum kodları olduğu varsayılır) okunur ama hiçbir göstergede kullanılmaz (`engine/trade_processor.cpp:32-33`). |
| PGC (`PgcMessage`) | `mx/pgc/{sembol}` | `timestamp`, `pgc5`, `pgc8`, `pgc10` (double), `symbol` | `proto/Pgc.proto:7-13`. Matriks belgesine göre PGC "AKD lisansı olan kullanıcıların görebildiği Aracı Kurum Dağılımı verilerinden oluşturulan Para Giriş Çıkışı"dır, N kurum için hesaplanır ve **günlük** veridir (`docs/working-docs/quantex/services/quantex-indicators/08-MATRIKS-INDICATOR.md:3156-3168`). Yani **kurumsal/aracı kurum akışı**dır. |
| Borsa saati | `mx/timestamp` | `source`, `isBistPPOpen`, `isBistPPHoliday`, `timestamp` (µs) | Açık/kapalı yalnız `source == "MX"` mesajlarından alınır (`services/market_clock.cpp:127-142`). |

**Alış/satış yönü:** işlemdeki taraf alanından (`activeBidOrAsk`'in ilk karakteri) gelir;
`'b'` alıcı tetikli, `'a'` satıcı tetikli (`core/types.hpp:19`, `engine/trade_processor.cpp:36-38`).
Tik kuralı yok. Taraf alanı `b`/`a` dışında olan ya da fiyatı/miktarı sıfır olan işlem
**tümüyle atılır** (`core/types.hpp:25-28`, `trade_processor.cpp:40-42`) — VWAP ve toplam
hacim de bu işlemleri içermez. Varsayım: açılış/kapanış seansı eşleşmeleri tarafsız gelirse
VWAP'tan düşerler (Matriks'e sorulmalı).

**İşlem birikimi (sembol başına, her geçerli işlemde)** — `engine/trade_processor.cpp:50-74`:
`B += q` ve `nB += 1` (alıcı tetikli) ya da `S += q`, `nS += 1`; `PV += p·q`; `V += q`;
`son_fiyat = p`; zaman penceresine `(t, q)` eklenir ve `t − W·1000`'den eski girdiler
atılır (`W = TRADE_FLOW_DEFAULT_PERIOD`, varsayılan 60 s; `config/config.hpp:54`, `main.cpp:197`).
Pencere zamanı işlemin kendi `timestamp`'idir (yoksa duvar saati, `trade_processor.cpp:31`);
milisaniye varsayılır (`* 1000`, `:71`) — birim bilinmiyor, bkz. Açık sorular.

**PGC geçmişi (sembol başına, her PGC mesajında)** — `engine/pgc_processor.cpp:45-62`:
seçili tür `x = pgc{5|8|10}` (servis düzeyinde `PGC_DEFAULT_TYPE`, varsayılan `pgc10`;
`config/config.hpp:55`); `önceki = geçmişin son değeri` (ilk mesajda `x`'in kendisi);
`geçmiş` son `PGC_HISTORY_SIZE` (varsayılan 20) değer; `x > 0` ise `ardışık_giriş += 1`,
`ardışık_çıkış = 0`; `x < 0` ise tersi; `x == 0` sayaçlara dokunmaz.

#### 1.2 Gösterge tablosu

Kayıt: `engine/flow_indicator_factory.hpp:100-158`. Hepsi `f64` (`double`) üretir; çıktılar
anahtar-değer haritasıdır, varsayılan çıktı son sütunda.

| Ad | Girdi | Formül (kendi gösterimim) | Parametre | Pencere | Isınma | Sıfırlama | Kaynak |
|---|---|---|---|---|---|---|---|
| `CUMULATIVE_DELTA` | İşlem | `delta = B − S`; `buyvolume = B`; `sellvolume = S`; `netvolume = B + S` (adı "net" ama toplam); `deltapercent = 100·delta/(B+S)` (payda 0 ise 0). Varsayılan çıktı `delta`. | yok | Seans başından (kümülatif) | Yok; ilk işlemden sonra hazır | Borsa açılış olayında (§1.3) | `indicators/trade/cumulative_delta.hpp:17-24` |
| `BUY_SELL_RATIO` | İşlem | `ratio = B/S`; `S = 0` iken `B > 0` ise `+∞`, `B = 0` ise 1. Ayrıca `buycount = nB`, `sellcount = nS`, `avgbuysize = B/nB`, `avgsellsize = S/nS`. Varsayılan `ratio`. | yok | Kümülatif | Yok | Açılışta | `indicators/trade/buy_sell_ratio.hpp:18-32` |
| `TRADE_FLOW` | İşlem | Penceredeki işlem sayısı `n`, hacim `Vw`; `tradespersecond = n/P`, `volumepersecond = Vw/P`, `avgtradesize = Vw/n`, `tradecount = n`, `windowvolume = Vw`. Varsayılan `tradespersecond`. | `period` P saniye, sınır 10–600, varsayılan 60 | **Zaman** penceresi (kayan, işlem zamanına göre) | Yok; ilk P saniyede oran düşük çıkar | Açılışta pencere boşaltılır | `indicators/trade/trade_flow.hpp:19-29`, `:47-49` |
| `VWAP_TICK` | İşlem | `vwap = PV/V`; `deviation = 100·(son_fiyat − vwap)/vwap`; ayrıca `totalvolume`, `totalvalue`, `lastprice`. Varsayılan `vwap`. Fiyat float32'den gelir. | yok | Kümülatif (seans) | Yok | Açılışta | `indicators/trade/vwap_tick.hpp:16-21`, `proto/Trade.proto:11` |
| `PGC_MOMENTUM` | PGC | `momentum = x_şimdi − önceki`; `direction = sign(x_şimdi)` (giriş/çıkış); `strength = 3` eğer `|momentum| > 1 000 000`, `2` eğer `> 100 000`, aksi 1. Varsayılan `momentum`. | `pgcType` (`pgc5`/`pgc8`/`pgc10`, varsayılan `pgc10`) | Ardışık iki PGC mesajı | Yok (ilk mesajda momentum 0) | **Sıfırlanmaz** (yalnız yeniden başlatmada) | `indicators/pgc/pgc_momentum.hpp:16-42` |
| `PGC_TREND` | PGC | `avgpgc` = geçmişin son `period` değerinin aritmetik ortalaması; `trenddirection = +1` eğer `avgpgc > 0` ve `x > 0`, `−1` ikisi de `< 0`, aksi 0; `trendstrength = round(50·min(|x|/10⁷, 1) + 50·min(max(ardışık_giriş, ardışık_çıkış)/10, 1))` (0–100). Ayrıca ham `pgc5`, `pgc8`, `pgc10`, iki ardışık sayaç. Varsayılan `trendstrength`. | `period` 3–50, varsayılan 10; `pgcType` | Son `period` **PGC mesajı** (zaman değil, bar değil) | Yok; geçmiş kısaysa olanların ortalaması | Sıfırlanmaz | `indicators/pgc/pgc_trend.hpp:27-53`, `:75-77` |

Gözlemler (hepsi kod okumasından):

1. **`TRADE_FLOW`'un `period`'u pencereyi değiştirmez.** Pencere işlemcide tek ve servis
   genelidir (`W`, varsayılan 60 s; `trade_processor.cpp:71`); `period` yalnız bölende
   kullanılır (`trade_flow.hpp:26-28`). `period=120` istenirse "son 60 s'deki işlem sayısı / 120"
   döner — adıyla uyuşmayan sonuç.
2. **`pgcType` geçmişle tutarsız.** Geçmiş ve `önceki` servisin varsayılan türüyle (`pgc10`)
   tutulur (`pgc_processor.cpp:46-48`); `PGC_MOMENTUM(pgcType=pgc5)` güncel `pgc5`'ten önceki
   `pgc10`'u çıkarır. Ayrıca hesap anahtarı `pgcType`'ı içermez (`engine/flow_calculation_registry.cpp:212-215`):
   aynı sembolde farklı `pgcType` isteyen ikinci abone birincinin örneğini paylaşır.
3. `PgcData::get_pgc` bilinmeyen türde sessizce `pgc10` döner (`core/types.hpp:72-76`).
   Matriks belgesi PGC5/8/**15** der (`08-MATRIKS-INDICATOR.md:3163-3165`), proto ise
   `pgc10` taşır — hangisinin doğru olduğu bilinmiyor.
4. `BUY_SELL_RATIO` `+∞` üretebilir; Varsayım: nlohmann JSON bunu `null` olarak yazar,
   çapraz köprü ise sonsuz değerli mesajı tümden atar (`engine/cross_engine_bridge.cpp:290`, `:297`).
5. `PGC_MOMENTUM` eşikleri (10⁵, 10⁶) ve `PGC_TREND` ölçeği (10⁷) sembolden bağımsız sabit
   para birimi eşikleridir; küçük ve büyük hisse için aynı.
6. Parametre sınırları (`get_param_bounds`) yalnız varsayılan atamak için kullanılır
   (`flow_indicator_factory.hpp:89-98`); `InputValidator::validate_params` tanımlı
   (`security/validation.cpp:120`) ama hiçbir yerden çağrılmaz (arama: yalnız
   `is_valid_symbol` ve `is_json_size_valid` kullanılıyor, `flow_calculation_registry.cpp:204`,
   `services/mqtt_manager.cpp:606`). `period` dize gelirse `get<int>()` istisna atar ve
   komut sessizce düşer (`flow_calculation_registry.cpp:213-214`, `:104-106`).

#### 1.3 Yayın sıklığı, sıfırlama, zaman

- **Yayın:** birikim her işlemde güncellenir, ama göstergeler yalnız sembol başına
  kısmadan geçen mesajda hesaplanıp yayınlanır: son yayından bu yana `FLOW_THROTTLE_MS`
  (varsayılan 100 ms, 10–10000) geçtiyse (`trade_processor.cpp:77-81`, `:118-135`;
  `pgc_processor.cpp:65-69`; `config/config.hpp:53`). Kısma `steady_clock` (duvar saati
  benzeri, makineye bağlı) ile ölçülür ve işlem/PGC için ayrı ama sembol içindeki bütün
  göstergeler için ortaktır. Arada kalan son işlem bir sonraki işleme kadar yayınlanmaz
  (zamanlayıcıyla "son durumu bas" yok).
- Yayın konusu `quantex/flow/indicator/update/{anahtar}`; yük JSON: `type`, `key`, `symbol`,
  `indicator`, `engine_type`, `engine_id`, `timestamp`, `values` (harita) ya da `value`
  (`flow_calculation_registry.cpp:359-375`). **`timestamp` işlem zamanı değil, yayın anındaki
  duvar saatidir** (`now_ms()`, `:355`, `:411`; `core/types.hpp:124-128`).
- Borsa kapalıyken (`is_market_open() == false`) hesap ve yayın yapılmaz
  (`flow_calculation_registry.cpp:334-336`, `:390-392`); ama işlem birikimi kapalıyken de
  sürer (`trade_processor.cpp` saat denetimi yapmaz).
- **Sıfırlama:** yalnız işlem birikimi ve pencereler, `mx/timestamp` "MX" kaynağında
  kapalı→açık geçişi görüldüğünde sıfırlanır (`main.cpp:228-232`, `trade_processor.cpp:90-103`,
  `market_clock.cpp:145-156`). PGC geçmişi sıfırlanmaz. Saat mesajı gelmemişse
  oturum saatleri REST'ten alınır ve sunucunun **yerel saat dilimine** göre karşılaştırılır
  (`market_clock.cpp:31-58`, `localtime_r`); hiçbiri yoksa "kapalı" sayılır (`:33-35`).
- Servis yeniden başlarsa birikim sıfırdan başlar; geçmişi yeniden oynatma ya da
  anlık görüntü yok. Gösterge başına son 50 değer bir halka tamponda tutulur
  (`flow_calculation_registry.hpp:49`) ama hiçbir komutla sunulmaz (abone cevabında
  `history` yok; bağlayıcı `history ?? data` bekliyor, `flow.transform.ts:100`).

### 2. Sinyaller

#### 2.1 Sinyal motoru (ortak kurallar)

- Tanımlar `engine/signal_registry.cpp:729-748`'de yüklenir; yalnız bu 12 çekirdek sinyal
  var. `parse_signal_definition_json` (`:1607`) olsa da komut işleyicide özel sinyal
  kaydetme eylemi yok (`main.cpp:234-300`: `subscribe`, `unsubscribe`, `signal_subscribe`,
  `signal_unsubscribe`, `signal_list`, `signal_stats`).
- Bir örnek (instance) = sinyal + çözülmüş parametreler (`:166-203`). Örnek, koşullarının
  göstergelerine iç abonelik açar; iç abonelik parametre almaz, göstergeler hep varsayılan
  `period`/`pgcType` ile kurulur (`flow_calculation_registry.cpp:473-493`).
- Değerlendirme her gösterge güncellemesinde (tik/PGC kısmasıyla) yapılır; bar kapanışı
  kavramı yok (`signal_registry.cpp:369`: `is_closed = false` sabit).
- **Bütün koşullar VE ile bağlanır** (`:1231-1239`); sonra her tetik için, tetiğin
  `condition_indices` listesi boşsa doğrudan, doluysa o koşullar yeniden sağlanırsa
  ateşlenir (`:1242-1258`).
- Formüller sinyal kaydında **ham metinle** ayrıştırılıp önbelleğe alınır (`:67-79`).
  Değerlendirme bağlamı yalnız takma adların son değerleri ve geçmişidir (`:1588-1600`);
  `$THRESHOLD_HIGH`, `$MULTIPLIER` gibi parametre yer tutucuları formüle hiç yerleştirilmez
  (yer tutucu değiştirme yalnız gösterge referansları ve ileti şablonu için yapılır, `:995`,
  `:1401`). Bilinmeyen değişken `UNKNOWN_VARIABLE` hatası atar
  (`signals/expression_evaluator.cpp:266-274`) ve koşul **yanlış** sayılır (`signal_registry.cpp:1294-1299`).
- `avg(a, b, …)` argümanlarının ortalamasıdır (`expression_evaluator.cpp:240-247`); geçmiş
  ortalaması değil.
- **Soğuma:** ateşlemeden sonra örnek `COOLDOWN`'a girer (`:1261-1266`); soğuma sayacı
  yalnız `is_closed` doğruyken azalır (`:1214-1222`), ama `is_closed` hep yanlış (`:369`).
  Çıkarım: örnek ilk ateşlemeden sonra bir daha ateşlemez (yalnız IDLE→RUNNING geçişi
  durumu düzeltir, `:229-235`).
- Kesişim: önceki ve şimdiki değer, sabit eşiği ya da öteki çizgiyi geçerse doğru
  (`:1304-1351`). "Önceki değer" önceki *gösterge yayını*dır (100 ms kısmalı), bar değil.
- Ateşleme yayını: `quantex/flow/signal/data/{signal_id}/{sembol}/{period}` ve
  `quantex/flow/signal/instance/{instance_key}` (`:1497-1508`); güven 0–100 tamsayı
  (bağlayıcı 0–1'e böler, `flow.transform.ts:33-37`).

#### 2.2 Sinyal tablosu

"Durum" sütunu kod okumasından çıkarımdır.

| Sinyal | Sınıf | Kural (tanımdaki niyet) | Eşik / varsayılan | Pencere | Girdiler | Durum | Kaynak |
|---|---|---|---|---|---|---|---|
| `DELTA_REVERSAL` | akış | `delta` 0'ı keserse; aşağıdan yukarı yükseliş, tersi düşüş | 0 | ardışık iki yayın | `CUMULATIVE_DELTA.delta` | Çalışır, **ama** iki tetik de koşulsuz olduğu için her kesişimde hem BULLISH hem BEARISH olayı çıkar; sonra soğumada kalır | `signals/definitions/flow/delta_reversal.hpp:18-39` |
| `BUY_SELL_EXTREME` | akış | `ratio > threshold_high` (yükseliş) ya da `ratio < threshold_low` (düşüş) | 2.0 / 0.5 | kümülatif | `BUY_SELL_RATIO.ratio` | **Tetiklenmez**: iki koşul VE ile bağlı (aynı anda >2 ve <0.5 olamaz) ve `$THRESHOLD_*` çözülmez | `flow/buy_sell_extreme.hpp:14-50` |
| `VOLUME_ACCELERATION` | akış | `tps > avg(tps, 60) · multiplier` | 3.0 | 60 s pencere | `TRADE_FLOW.tradespersecond` | **Tetiklenmez**: `$MULTIPLIER` çözülmez; ayrıca `avg($tps,60)` = `(tps+60)/2` | `flow/volume_acceleration.hpp:14-31` |
| `LARGE_TRADE_DETECT` | akış | anomali motorunun `LARGE_TRADE` dedektörü skoru ≥ 0.75 | 0.75 | — | anomali olayları (`quantex/anomaly/data/#`) | **Tetiklenmez**: anomali aboneliği `main.cpp`'de bağlanmıyor (`set_on_anomaly`/`subscribe_anomaly_topics` hiç çağrılmaz; `services/mqtt_manager.cpp:360-373`), durum boş kalır (`signal_registry.cpp:1807-1808`) | `flow/large_trade_detect.hpp:18-33` |
| `PGC_INFLOW_SIGNAL` | akış | `momentum > 0` ve `strength ≥ 3` (yani PGC farkı > +10⁶) | sabit | ardışık iki PGC | `PGC_MOMENTUM` | Çalışır (formül yalnız takma ad kullanır); ilk ateşlemeden sonra soğumada kalır | `flow/pgc_inflow_signal.hpp:18-47` |
| `PGC_OUTFLOW_SIGNAL` | akış | `momentum < 0` ve `strength ≥ 3` | sabit | aynı | `PGC_MOMENTUM` | Çalışır; aynı soğuma sorunu | `flow/pgc_outflow_signal.hpp:18-47` |
| `PGC_TREND_CHANGE` | akış | `trenddirection` 0'ı keserse | 0 | son 10 PGC | `PGC_TREND.trenddirection` | Çalışır; `DELTA_REVERSAL` gibi her kesişimde iki yönü birden yayınlar. Not: −1→0 ve 0→+1 de kesişim sayılır | `flow/pgc_trend_change.hpp:18-39` |
| `DELTA_DIVERGENCE` | çapraz | fiyat eğimi > 0 ve delta eğimi < 0 (düşüş), ya da tersi | `period` 5m, `lookback` 5 (kullanılmıyor) | — | fiyat `CLOSE`, `CUMULATIVE_DELTA` | **Tetiklenmez**: `$close_slope`, `$delta_slope` hiçbir yerde üretilmiyor; fiyat tarafı için §2.3 | `cross/delta_divergence.hpp:14-41` |
| `FLOW_RSI_DELTA_COMBO` | çapraz | `RSI < rsi_threshold` ve `delta > 0` | 5m, RSI 14, eşik 30 | — | fiyat `RSI:14`, `CUMULATIVE_DELTA` | **Tetiklenmez**: `$RSI_THRESHOLD` çözülmez; §2.3 | `cross/flow_rsi_delta_combo.hpp:14-42` |
| `PGC_BREAKOUT_COMBO` | çapraz | `close > BB_üst` ve `pgc_momentum > pgc_threshold`, ya da `close < BB_alt` ve `< −eşik` | 5m, BB 20/2.0, eşik 0.5 | — | fiyat `BB`, `CLOSE`, `PGC_MOMENTUM` | **Tetiklenmez**: `$PGC_THRESHOLD`, `$bb_lower` çözülmez; ayrıca VE'li `pgc_momentum > 0` koşulu düşüş kolunu zaten keser | `cross/pgc_breakout_combo.hpp:14-52` |
| `PGC_PRICE_DIVERGENCE` | çapraz | `pgc_momentum > eşik` ve fiyat eğimi < 0, ya da tersi | 5m, eşik 0.5 | — | fiyat `CLOSE`, `PGC_MOMENTUM` | **Tetiklenmez**: `$PGC_THRESHOLD`, `$close_slope` çözülmez. Eşik 0.5 para birimi biriminde anlamsız küçük | `cross/pgc_price_divergence.hpp:14-41` |
| `SMART_MONEY_SIGNAL` | çapraz | `pgc_momentum > eşik` ve `close < vwap` ve `tps > tps_avg · oran` | 5m, 0.5, 2.0 | — | fiyat `CLOSE`, `VWAP_TICK.vwap`, `PGC_MOMENTUM`, `TRADE_FLOW` | **Tetiklenmez**: `$tps_avg`, `$PGC_THRESHOLD`, `$VOLUME_SPIKE_RATIO` çözülmez; çıktı adı `trades_per_second` yanlış (gösterge `tradespersecond` üretir; yedek olarak haritanın ilk alanı alınır, `signal_registry.cpp:424-428`) | `cross/smart_money_signal.hpp:14-60` |

Not: `cooldown_bars` adı bar dese de akış motorunda bar yok; değerlendirme sayısı da değil
(§2.1 soğuma).

#### 2.3 Çapraz motor köprüsü (`engine/cross_engine_bridge.cpp`)

- **Keşif:** `quantex/registry/engine/+/+` (tutulan/retained kayıt iletileri) ve
  `quantex/engine/heartbeat` dinlenir (`:16-24`). Kayıttan `command_topic`,
  `indicator_update_prefix`, gösterge listesi alınır (`:250-262`). 60 s kalp atışı gelmezse
  motor "stale" sayılır (`cross_engine_bridge.hpp:108`, `cross_engine_bridge.cpp:327-349`);
  geri gelince bütün dış abonelikler yeniden gönderilir (`:352-370`).
- **Abonelik:** sinyal koşulundaki `price:SEMBOL:PERİYOT:GÖSTERGE:PARAM` referansı ayrıştırılır
  (`signal_registry.cpp:859-904`, `:906-942`); kaynak `flow` değilse köprüye gider (`:1009-1022`).
  Köprü fiyat motorunun komut konusuna `{action: subscribe, symbol, indicator, period, params:{length}, replyTo}`
  yayınlar (`cross_engine_bridge.cpp:79-114`; `RSI:14` → `indicator=RSI`, `length=14`, `:88-104`)
  ve `{prefix}/{SEMBOL}:{PERİYOT}:{GÖSTERGE:PARAM}` güncelleme konusuna abone olur (`:107`, `:116-124`).
  Fiyat motoru kayıtlı değilse sinyal örneği hiç kurulmaz (`signal_registry.cpp:1017-1022`).
- **Geri besleme:** gelen güncellemenin anahtarı `engine_type + ":" + yük.key` olarak sinyal
  motoruna verilir (`cross_engine_bridge.cpp:306-309`). Ama sinyal örneği takma adı
  `build_subscription_key` = `price:SEMBOL:GÖSTERGE` anahtarına bağlar (`:56`, `:400-405`;
  `signal_registry.cpp:1036-1039`) — periyot yok. Fiyat motoru anahtarı `SEMBOL:PERİYOT:…`
  biçiminde kurar (`services/quantex_indicators_price_cpp/src/engine/calculation_registry.cpp:2005-2010`).
  Çıkarım: anahtarlar eşleşmez, fiyat değerleri sinyale ulaşmaz.
- Ayrıca `CLOSE` adlı bir fiyat göstergesi fiyat motorunda kayıtlı görünmüyor (fiyat
  `src/` içinde `"CLOSE"` araması boş; kayıtlı hacim göstergeleri `OBV`, `VWAP`,
  `engine/indicator_factory.hpp:416`, `:427`). T4a teyit etmeli.
- Yeniden abonelikte komut `symbol`/`indicator` yerine yalnız `key` taşır (`:361-366`); fiyat
  motorunun bunu kabul edip etmediği bilinmiyor (T4a).

### 3. Abonelik

Zincir: istemci → ağ geçidi → NATS → bağlayıcı → MQTT (Mosquitto) → motor; dönüş tersi.

| Adım | Gösterge | Sinyal |
|---|---|---|
| NATS istek konusu | `flow.cmd.*`, protobuf `FlowCommand{action, symbol, indicator, params_json, reply_to, client_id}` (`shared/proto/flow.proto:50-57`; `nats/nats.cmd-listener.ts:30`) | `flow.signal.cmd.*`, `FlowSignalCommand` (`nats/nats.signal-cmd-listener.ts:27`) |
| Bağlayıcının MQTT'ye geçirdiği | `quantex/flow/indicator/cmd`, JSON `{action, symbol, indicator, params, replyTo, clientId, reassert}` (`nats.cmd-listener.ts:34`, `:105-116`) | `quantex/flow/signal/cmd`, JSON `{action, signal_id, instance_key, params, userId, delivery, replyTo, clientId, reassert}` (`nats.signal-cmd-listener.ts:31`, `:108-122`) |
| Motorun dinlediği | Genel `quantex/flow/{indicator,signal}/cmd` + motora özel `quantex/engine/{id}/{indicator,signal}/cmd` (`services/mqtt_manager.cpp:226-238`) | aynı |
| Sahiplik | Her bölme (pod) komutu alır; Redis `SET NX EX` iddiasını kazanan hesaplar, kaybeden sessiz kalır; TTL 30 s, yenileme 10 s (`flow_calculation_registry.cpp:251-257`, `flow_calculation_registry.hpp:119-120`) | Aynı model, `signal:{instance_key}` (`signal_registry.cpp:248-256`, `main.cpp:178-191`) |
| Hesap anahtarı | `SEMBOL:GÖSTERGE[:period]` (`flow_calculation_registry.cpp:212-215`) | `signal_id` + sıralı parametre değerleri (`signal_registry.cpp:760-767`) |
| Cevap | `replyTo`'ya `{type: subscribed, key, data_topic, engine_id}` (`:236-246`, `:286-296`); bağlayıcı `quantex/flow/indicator/reply/{clientId}/{key}` → NATS `flow.reply.{clientId}.{key}` | `{type: signal_subscribed|signal_error, instance_key, …}` → NATS `flow.signal.reply.{clientId}.{key}` |
| Veri | `quantex/flow/indicator/update/{key}` → NATS `flow.{BORSA}.{sembol}.data.{key}`, protobuf `Envelope{marketType: FLOW, dataType: flow_update, flowUpdate{…, valueJson}}` (`nats/nats.publisher.ts:86-91`; `transform/flow.transform.ts:46-67`) | `quantex/flow/signal/{data,instance}/…` → ikisi de `flow.signal.{BORSA}.{sembol}.{signalId}` (`nats.publisher.ts:98-107`): **her ateşleme NATS'a iki kez** düşer; durum `flow.signal.status.{instanceKey}` |

- **İsteğe bağlı parametre:** kısmen. Gösterge isteği `params` taşır, ama yalnız `period`
  (`TRADE_FLOW`, `PGC_TREND`) ve `pgcType` (PGC göstergeleri) okunur
  (`flow_indicator_factory.hpp:117-157`); sınırlar denetlenmez; `pgcType` anahtara girmez
  (§1.2). Sinyal parametreleri anahtara girer ama formüllerde etkisizdir (§2.1); iç
  gösterge abonelikleri parametresizdir. Yani pratikte sabit küme.
- **Çapraz gösterge:** istemci doğrudan "fiyat × akış" göstergesi isteyemez; çapraz yalnız
  çapraz sinyaller içinden, köprüyle kurulur (§2.3).
- Sembol biçimi `^[A-Z0-9]{2,10}$` (`security/validation.cpp:28`); komut JSON'u en çok 4 KB
  (`security/validation.hpp:113`). Bir sembole ilk hesap açılınca Matriks'te **hem**
  `mx/trade/{s}` **hem** `mx/pgc/{s}` abonesi olunur, gösterge türünden bağımsız
  (`services/mqtt_manager.cpp:395-404`).
- Gösterge listesi kayıt iletisinde ilan edilir: `quantex/registry/engine/flow/{id}`
  (tutulan), yetenekler `tick_based`, `signals`, `cross_engine` (`main.cpp:327-341`).
- Abonelik bırakıldığında hesap hemen silinmez; 5 dk boşta kalınca temizlenir
  (`flow_calculation_registry.cpp:110-128`, `config/config.hpp:58`).

### 4. Qapsule ile karşılaştırma

#### 4.1 `qp-compute`

`crates/qp-compute/src/studies/` yalnız `atr`, `bollinger`, `ema`, `extremum`, `macd`, `rsi`,
`sma` içerir; **VWAP, kümülatif delta ya da işlem tabanlı hesap yok**. Hacim yalnız bar alanı
olarak var (`crates/qp-compute/src/lib.rs:62-90`). Doğrudan formül farkı çıkarılacak eş yok.
Karşılaştırma için: Quantex'in **fiyat** motorundaki `VWAP` bar tabanlıdır ve tipik fiyat
`(H+L+C)/3` ile ağırlıklandırır (`services/quantex_indicators_price_cpp/src/indicators/volume/vwap.hpp:32-38`),
akış motorunun `VWAP_TICK`'i ise işlem fiyatı × miktar (`vwap_tick.hpp:16-20`). **Aynı ad
("VWAP"), farklı sonuç** riski Quantex'in kendi içinde var; Qapsule'e VWAP eklenirse hangisi
olduğu adda yazılmalı.

#### 4.2 `qp-facts` dedektörleri ve Quantex akışı

| Qapsule | Girdi | Kural | Quantex'teki en yakın | Aynı kavram mı? |
|---|---|---|---|---|
| `flow.institutional` v1 (`crates/qp-facts/src/flow_institutional.rs`) | Bar hizalı lisanslı `FlowPoint.values["net"]` (`:4-6`, `crates/qp-facts/src/lib.rs:54-63`) | Son 5 gözlemin `|net|` ortalaması `avg5`; `net ≠ 0` ve `|net| > 2·avg5` → `net_buy`/`net_sell`, güç `(|net|/avg5 − 2)/2` (`:8-12`, `:108-137`); lisans `Licensed`, yetki `flow/*` (`:91-96`), anlam türü akış, kanıt ailesi akış | `PGC_MOMENTUM` + `PGC_INFLOW/OUTFLOW_SIGNAL` | **Kavram yakın, tanım farklı.** (a) Qapsule'de girdi bar başına net akış (bir kurumun ya da toplamın); Quantex'te PGC N kurumun kümülatif (günlük) değeri, "momentum" iki ardışık mesajın farkı — Varsayım: bu fark, bar başına net akışa denk düşer ancak PGC mesajları bar sınırında örneklenirse. (b) Eşik Qapsule'de göreli (kendi ortalamasının 2 katı), Quantex'te mutlak (10⁶ para birimi). (c) Qapsule bar kapanışında, Quantex mesaj başına (100 ms). (d) Quantex sinyalinde soğuma hatası (§2.1). |
| `flow.institutional` | — | — | `PGC_TREND` (`trenddirection`) + `PGC_TREND_CHANGE` | Qapsule'de karşılığı yok: "akış yönü dönüşü" (anlam türü dönüş + kanıt ailesi akış) olabilir. |
| `flow.volume_surge` v1 (`crates/qp-facts/src/volume_surge.rs`) | Bar hacmi (açık veri) | Önceki 20 barın SMA'sı `vol_avg`; `v > 2.5·vol_avg` → `surge`, yön `C − O` işaretinden, güç `(v/vol_avg − 2.5)/2.5` (`:3-9`, `:77-107`); lisans açık (varsayılan `Public`, `crates/qp-facts/src/lib.rs:122-125`), kanıt ailesi hacim | `VOLUME_ACCELERATION` (amaçlanan), `TRADE_FLOW` | **Kavram yakın, tanım farklı.** Quantex işlem **hızını** (işlem/s, 60 s kayan zaman penceresi) kendi "ortalamasının" 3 katıyla kıyaslamayı amaçlar; Qapsule bar **hacmini** 20 bar ortalamasının 2.5 katıyla. Quantex'te yön yok (NEUTRAL); Qapsule mumun yönünü verir. Quantex sinyali bugün tetiklenmiyor. |
| — | — | — | `CUMULATIVE_DELTA` + `DELTA_REVERSAL` | Qapsule'de eşi yok. Alıcı/satıcı tetikli hacim ayrımı açık veri mi lisanslı mı bilinmiyor (§5). Anlam türü: dönüş, kanıt ailesi: akış (Varsayım). |
| — | — | — | `BUY_SELL_RATIO` + `BUY_SELL_EXTREME` | Qapsule'de eşi yok; anlam türü aşırılık olurdu. |
| — | — | — | `LARGE_TRADE_DETECT` | Anomali motoruna bağlı (T5); Qapsule'de eşi yok. |
| — | — | — | `SMART_MONEY_SIGNAL`, `PGC_PRICE_DIVERGENCE`, `DELTA_DIVERGENCE` | Qapsule'de çoklu-aile birleşik olgu yok; Qapsule bunu dedektörde değil kapsül düzeyinde (birden çok kanıt ailesinin teyidi) yapar. Bu sinyaller bugün çalışmıyor. |

Sonuç: Quantex akış sinyallerini **sunucu olgusu** olarak doğrudan almak bugün mümkün değil
(çoğu ateşlemiyor, ateşleyenler bir kez ateşliyor ve iki yönü birden veriyor; zaman damgası
yayın saati; bar hizası yok, K13/K14). Taşınabilir olan ham girdi: PGC değerleri (lisanslı)
ve taraf işaretli işlem akışı. Qapsule'ün `FlowPoint`'i bunları bar hizasında kendisi
toplamalı (ör. bar başına `net = Δpgc`, `buy = B`, `sell = S`).

### 5. Lisans ve yetki

- **PGC lisanslıdır:** Matriks belgesi "AKD lisansı olan kullanıcıların görebildiği"
  der (`08-MATRIKS-INDICATOR.md:3156-3158`); Quantex API test raporları para giriş-çıkışı
  uçları için **AKDE** ya da **AKDE/AKD && VAKD** yetkisi ister
  (`services/quantex-api/Api-Test-Results/401_UNAUTHORIZED_ENDPOINTS_SUMMARY.md:73`, `:138`).
  Qapsule'de `flow.institutional`'ın `Licensed` sınıfına denk düşer (K33).
- **İşlem verisi:** fiyat/miktar açık piyasa verisi lisansına, `buyer`/`seller` (kurum kodları)
  ise Varsayım: AKD sınıfına girer; servis bu alanları kullanmıyor. `activeBidOrAsk`
  (alıcı/satıcı tetikli) hangi lisans sınıfında, bilinmiyor — Matriks'e sorulmalı.
- **Servis içinde lisans denetimi yok.** Komut yolunda:
  - JWT yalnız `JWT_AUTH_ENABLED` doğruysa çalışır, varsayılan **kapalı**
    (`config/config.hpp:90`, `services/mqtt_manager.cpp:616-633`). Anahtar tanımlı değilse
    her belirteci geçerli sayar ve `tier = premium` verir (`security/jwt_validator.cpp:37-43`).
    Okunan iddialar `sub`, `tier`, `exp`, `iat`, `permissions` (`:80-99`); `permissions`
    hiçbir yerde kullanılmaz (arama boş). Bağlayıcı komuta `token` koymaz
    (`nats.cmd-listener.ts:105-113`): JWT açılsa bu yoldan gelen her komut reddedilir.
  - Hız sınırı `tier`'e göre dakikada 10/30/100 istek (`config/config.hpp:81-87`,
    `security/rate_limiter.cpp:12-16`); bu yetki değil, kota.
  - `validation`: sembol biçimi ve 4 KB boyut (§3); paket/veri yetkisi denetimi değil.
- Sonuç: kim hangi sembole PGC göstergesi açabilir sorusu motorda değil, ağ geçidinde
  çözülüyor olmalı (Varsayım; T3/T6 ağ geçidini doğrulamalı). Motorun yayını
  `quantex/flow/indicator/update/#` ve NATS `flow.*.data.*` olarak herkese açık konuda;
  lisanslı ve açık göstergeler aynı konu ağacında, ayrım yok.

### 6. Performans, ölçek, determinizm

**Yazılı sınırlar** (belge yok; yalnız kod/yapılandırma):

| Konu | Değer | Kaynak |
|---|---|---|
| Yayın kısması | sembol başına 100 ms (10–10000) | `config/config.hpp:53`, `:259-261` |
| İşlem penceresi | 60 s (10–600) | `config/config.hpp:54`, `:262-264` |
| PGC geçmişi | 20 (5–100) | `config/config.hpp:56`, `:268-270` |
| Boşta temizlik | 5 dk | `config/config.hpp:58` |
| MQTT yayın kuyruğu | 10 000 ileti, QoS 0 | `config/config.hpp:69`, `services/mqtt_manager.cpp:281-293`, `:321-322` |
| Sahiplik iddiası | TTL 30 s, yenileme 10 s | `flow_calculation_registry.hpp:119-120` |
| Kaynak | 2 CPU / 512 MiB sınır; `scaling.enabled: false` | `flow-indicator-deploy.yml` (kaynak ve ölçekleme blokları) |
| Sembol sayısı, gecikme hedefi | yazılı değil | — |

Ölçek gözlemleri: her gösterge güncellemesi tek bir komut iş parçacığında işlenir
(`flow_calculation_registry.cpp:81-108`) ve her kısmalı işlemde işlem penceresi (deque)
kopyalanarak kuyruğa konur (`:44-55`) — yoğun sembolde 60 s'lik pencere kopyası maliyetli
olabilir (ölçülmedi). Prometheus etiketleri sembol başınadır (`trade_processor.cpp:44-45`).
Her bölme kendi Matriks MQTT bağlantısını açar (`main.cpp:123-129`; donanım kimliği
`HOSTNAME`'den, `config/config.hpp:157-170`) — T1'in "yayın tipi başına bir bağlantı" sorusuna
girdi.

**Determinizm (K3 karşısında):**

- Hesap formülleri saf aritmetiktir; dil modeli, rastgelelik yok. Rastgelelik yalnız
  kimlik üretiminde (`config/config.hpp:162-169`, `:174-180`).
- Ama yayımlanan değer dizisi **duvar saatine bağlıdır**: hangi ara durumun yayımlanacağını
  `steady_clock` kısması seçer (`trade_processor.cpp:118-135`); yük zaman damgası yayın anı
  (`flow_calculation_registry.cpp:355`); mesajda zaman yoksa işlem/PGC zamanı da duvar saati
  (`trade_processor.cpp:31`, `pgc_processor.cpp:30`); borsa açık/kapalı yedeği sunucunun yerel
  saat dilimi (`market_clock.cpp:39-42`); sıfırlama saat mesajının geliş anına bağlı.
- Sinyal kesişimleri "önceki yayın"a göre ölçüldüğünden, aynı işlem akışı farklı zamanlamayla
  farklı sinyal üretebilir.
- Yeniden oynatma yok; yeniden başlatma birikimi siler. Aynı girdi → aynı çıktı ancak
  "aynı girdi" işlem sırası **ve** zamanlaması olarak tanımlanırsa sağlanır.
- Fiyat proto'da float32 (`proto/Trade.proto:11`); VWAP `double`'da birikir ama girdi
  hassasiyeti 7 basamak civarı.

### 7. Arkin'in 5. bilgisi — akış servisi yanı

> "Gösterge servislerine gösterge ve çapraz gösterge aboneliğiyle dinleme yapılabilir;
> anomali motoru piyasa anomalilerini yakalar; hepsi NATS üzerinden yayınlar."

**Hüküm: kısmen doğrulandı.**

- *Gösterge aboneliği:* **doğrulandı.** NATS `flow.cmd.*` → MQTT → motor; veri NATS
  `flow.{BORSA}.{sembol}.data.{key}` (§3).
- *NATS üzerinden yayın:* **doğrulandı**, ama motor kendisi NATS'a değil Mosquitto'ya yazar;
  NATS'a geçiren bağlayıcıdır (`mqtt/mqtt.subscriber.ts:10-18`, `nats/nats.publisher.ts:58-72`).
- *Çapraz gösterge aboneliği:* **kodda bu biçimde görünmüyor.** İstemcinin isteyebileceği
  bir "çapraz gösterge" yok; çapraz yalnız beş çapraz **sinyalin** içinde köprüyle kurulur ve
  kod okumasına göre bugün fiyat tarafı beslenmiyor (anahtar uyuşmazlığı, `CLOSE` yok,
  çözülmeyen formül değişkenleri; §2.2–2.3).
- *Anomali bağlantısı (akış tarafı):* akış motorunda anomali olayı tüketimi yazılmış ama
  bağlanmamış (`LARGE_TRADE_DETECT` ölü; §2.2). Anomali motorunun kendisi T5'te.
- *İsteğe bağlı parametre:* yalnız `period`/`pgcType`, denetimsiz ve kısmen yanlış uygulanmış
  (§1.2, §3).

### 8. Açık sorular

| # | Soru | Kime |
|---|---|---|
| 1 | Bu servis canlıda çalışıyor mu, sinyalleri tüketen var mı? Kod okumasına göre 12 sinyalden 6'sı hiç, 4'ü bir kez ateşlemeli; gözlenen davranış bu mu? | Quantex ekibi (servis sahibi) |
| 2 | `mx/trade` `timestamp` birimi (ms mi µs mü) ve saat dilimi? Pencere ms varsayıyor (`trade_processor.cpp:71`); `mx/timestamp` µs (`market_clock.cpp:122-124`). | Matriks / T2 |
| 3 | PGC: `pgc10` mu `pgc15` mi? Değer günlük kümülatif mi, yoksa son dönemin akışı mı? Para birimi TL mi? Hangi sıklıkta gelir? | Matriks |
| 4 | `activeBidOrAsk` hangi değerleri alır (yalnız `b`/`a` mı; açılış/kapanış seansı eşleşmelerinde boş mu)? Bu alan ve `buyer`/`seller` hangi lisans sınıfında (açık, AKD, AKDE)? | Matriks |
| 5 | PGC göstergesi açan istemcinin AKD/AKDE yetkisi nerede denetleniyor (ağ geçidi mi, hiç mi)? | Quantex ekibi / T3, T6 |
| 6 | Fiyat motorunda `CLOSE` göstergesi var mı; dış abonelik `{action: subscribe, key}` biçimini kabul ediyor mu; güncelleme yükündeki `key` biçimi nedir? | T4a |
| 7 | Seans içi kapanış→açılış geçişleri (ör. kapanış seansı, devre kesici) birikimi sıfırlıyor mu, bu istenen bir şey mi? | Quantex ekibi |
| 8 | Akış motoru kaç bölmeyle koşuyor (deploy `scaling.enabled: false`, kod çok bölmeye hazır)? Her bölme ayrı Matriks bağlantısı açıyor mu? | Quantex ekibi / T1 |

---

## RI1 / T4c — Derinlik göstergeleri servisi (`quantex_indicators_depth_cpp`)

Kapsam: GOAL §D'nin derinlik servisi yanı. Bütün yollar `quantex` deposuna göredir
(`services/...`); Qapsule yolları `qapsule:` önekiyle. Kaynak yalnız okundu; derleme, test,
ağ bağlantısı yapılmadı. Formüller kendi gösterimimle yazıldı. Koddan çıkmayan her şey
"bilinmiyor" ya da "Varsayım:" olarak işaretli.

Kısaltmalar: `B_i`, `A_i` = i'nci alış/satış kademesinin miktarı (`quantity`, int64 → f64);
`pB_i`, `pA_i` = kademe fiyatı; `N` = `levels` parametresi; `mid = (pB_1 + pA_1) / 2`.

---

### 0. Özet (en önemli bulgular)

1. **Beş gösterge, hepsi anlık (durumsuz) defter fonksiyonu**: pencere yok, ısınma yok, ilk
   geçerli defterde hazır. Tek parametre `levels` (1–20, varsayılan 5); iki göstergede hiç
   parametre yok (`services/quantex_indicators_depth_cpp/src/engine/depth_indicator_factory.hpp:112-159`).
2. **Defter tam tablo olarak alınıyor, artımlı güncelleme uygulanmıyor.** Proto'daki
   `action`/`bidAsk`/`row`/`actionPrice`… alanları (`proto/DepthTable.proto:20-34`) kodda hiç
   okunmuyor (src altında `action()`, `row()`, `bidask` araması boş). Her mesaj kendi
   `bids`/`asks` listesiyle yeni bir anlık görüntü sayılıyor
   (`src/engine/depth_processor.cpp:24-58`); iki taraftan biri boşsa mesaj atılıyor (`:50-53`,
   `src/core/types.hpp:26-29`). Qapsule belgesine göre Matriks derinliği "ilk mesaj tam defter,
   sonrası artımlı" gönderiyor (`qapsule:docs/MATRIKS_PROTOCOL.md:362`). Bu iki bilgi
   çelişiyor; servisin abone olduğu konu `mx/depth/{SYMBOL}` (soneksiz,
   `src/services/mqtt_manager.cpp:392`) ve MQTT yolu `/depth` (`src/config/config.hpp:44`);
   anomali motoru ise `mx/depth/{SYMBOL}@lvl2` kullanıyor
   (`services/quantex-anomaly-engine/src/main.cpp:623`). Soneksiz konunun tam tablo mu
   gönderdiği **bilinmiyor — Matriks veri ekibine / Quantex servis sahibine sorulmalı**.
3. **Yayın her derinlik mesajında, ama sembol başına 100 ms kısma ile**: kısma penceresinde
   gelen görüntüler hesaplanmadan düşüyor (ertelenmiyor; `depth_processor.cpp:62-66`,
   `:69-86`). Pencerenin son görüntüsü bir sonraki mesaj gelene kadar yayımlanmaz.
4. **Yedi çekirdek sinyalin en az beşi kod okumasına göre hiç tetiklenemez, kalan ikisi bir
   kez tetiklenip kalıcı bekleme (cooldown) durumunda kalır** (ayrıntı §2.2). Bu bir çalışma
   gözlemi değil, okumadan çıkarımdır; çalıştırarak doğrulanmadı.
5. **Lisans/paket denetimi yok.** JWT yalnız kimlik, süre ve `tier` (free/standard/premium)
   çıkarıyor; `tier` yalnız istek sıklığı sınırını seçiyor. JWT varsayılan kapalı
   (`config.hpp:88`). Kademe sayısı ve derinlik lisans sınıfı kodda hiçbir yerde geçmiyor.
6. **Qapsule'de derinlik tabanlı hesap ya da dedektör yok; fiyat adımı tablosu da yok.**
   Qapsule'de "Derinlik/`Depth`" adı başka bir kavrama (sunum derinliği) ayrılmış — ad
   çakışması (§5.3).

---

### 1. Gösterge listesi

#### 1.1 Tablo

Hepsi `BaseDepthIndicator` alt sınıfı; `update(DepthSnapshot)` → çok alanlı değer (map)
(`src/indicators/base_depth_indicator.hpp:10-25`). Girdi her durumda `DepthTable` (Matriks
`DepthTableMessage`); `DepthStats` proto'su depoda var (`proto/DepthStats.proto:7-12`) ama
kodda hiç kullanılmıyor (src araması boş).

| Ad (kayıt adı) | Girdi; kademe | Formül (kendi gösterimim) | Çıktı alanları (varsayılan) | Parametre | Pencere / ısınma | Kanıt |
|---|---|---|---|---|---|---|
| `ORDER_BOOK_IMBALANCE` | DepthTable; ilk `N` kademe iki taraf (eldeki kademe azsa o kadar) | `SB = Σ_{i≤N} B_i`, `SA = Σ_{i≤N} A_i`; `imbalance = (SB − SA)/(SB + SA)`, toplam 0 ise 0 | `imbalance`*, `bidvolume`=SB, `askvolume`=SA | `levels` 1–20, vars. 5 | Yok / ilk geçerli defterde hazır | `src/indicators/depth/order_book_imbalance.hpp:17-35`; kayıt `depth_indicator_factory.hpp:112-121` |
| `BID_ASK_SPREAD` | DepthTable; yalnız 1. kademe | `spread = pA_1 − pB_1`; `spreadpercent = 100·spread/mid`; `spreadticks = round(spread / adım(mid))` (adım §4) | `spreadpercent`*, `spreadticks`, `bestbid`, `bestask`, `midprice` | Yok | Yok | `src/indicators/depth/bid_ask_spread.hpp:16-32`; kayıt `factory:123-129` |
| `DEPTH_PRESSURE` | DepthTable; ilk `N` kademe | Kademe ağırlığı `w = 1 / (1 + d/mid)`, `d` = kademe fiyatının `mid`'e uzaklığı (d ≤ 0 ise w = 1); `BP = Σ B_i·w_i`, `AP = Σ A_i·w_i`; `pressure = (BP − AP)/(BP + AP)`, toplam 0 ise 0 | `pressure`*, `bidpressure`, `askpressure` | `levels` 1–20, vars. 5 | Yok | `src/indicators/depth/depth_pressure.hpp:17-44`; kayıt `factory:131-140` |
| `WEIGHTED_MID_PRICE` | DepthTable; yalnız 1. kademe | `qB = B_1`, `qA = A_1` (≤0 ise 1 alınır); `wmid = (pB_1·qA + pA_1·qB)/(qA + qB)`; `deviation = 100·(wmid − mid)/mid` | `weightedmid`*, `simplemid`, `deviation` | Yok | Yok | `src/indicators/depth/weighted_mid_price.hpp:15-33`; kayıt `factory:142-148` |
| `CUMULATIVE_DEPTH` | DepthTable; ilk `N` kademe | `cumulativebid = Σ B_i`, `cumulativeask = Σ A_i`, `bidordercount = Σ orderCount`, `askordercount` benzeri; `ratio = SB/SA` (SA = 0 ise SB > 0 → +∞, değilse 1) | `ratio`*, `cumulativebid`, `cumulativeask`, `bidordercount`, `askordercount` | `levels` 1–20, vars. 5 | Yok | `src/indicators/depth/cumulative_depth.hpp:18-43`; kayıt `factory:150-159` |

`*` = varsayılan çıktı (`IndicatorMeta.default_output`). Hiçbiri iç içe kullanılamaz
(`nestable = false`, `src/core/types.hpp:47`, kayıtlarda `false`).

Notlar:
- `ratio = +∞` JSON'a yazılırken ne olduğu (nlohmann `null` üretir) — Varsayım: kütüphanenin
  sonsuzluğu `null` yazdığı biliniyor; burada denenmedi.
- `levels` sınır dışıysa kırpılır (`apply_defaults`, `factory:91-106`); sayısal olmayan
  `levels` (`get<double>`) istisnaya düşer ve komut işçisinde günlüğe yazılıp yutulur
  (`depth_calculation_registry.cpp:101-105`).
- Ortam değişkenleri `DEPTH_DEFAULT_LEVELS` ve `DEPTH_HISTORY_SIZE` okunuyor ve doğrulanıyor
  (`config.hpp:192-194`, `:258-263`) ama hiçbir yerde kullanılmıyor (varsayılan 5 kayıtta sabit,
  geçmiş tamponu 50'ye sabit: `depth_calculation_registry.hpp:44`).
- Gösterge başına 50 noktalık geçmiş tamponu dolduruluyor (`depth_calculation_registry.cpp:374`)
  ama hiçbir yerde okunmuyor; abonelik onayı geçmiş göndermiyor (`:254-264`, `:304-314`).
  Bağlayıcı `history` alanını bekliyor ama boş geçiyor
  (`services/quantex-stream/depth-indicator-adapter/src/transform/depth.transform.ts:100`).
- Sayılar `double` (f64); miktarlar int64 olarak gelip `double`'a çevriliyor.

#### 1.2 Defter nasıl tutuluyor, ne sıklıkla yayın

- **Tutma:** Sembol başına yalnız son anlık görüntü (`latest_snapshots_`,
  `depth_processor.cpp:55-58`); kademe güncellemesi uygulanmıyor (bkz. §0/2).
  `DepthLevel` alanları: fiyat, miktar, emir sayısı, zaman damgası (`src/core/types.hpp:13-18`);
  alış azalan, satış artan sıralı varsayılıyor (yorum, `types.hpp:22-23`); sıralama kodda
  denetlenmiyor.
- **Zaman:** Görüntü zamanı = mesajdaki `timestamp`, yoksa sunucu duvar saati (`now_ms`)
  (`depth_processor.cpp:26`, `types.hpp:76-80`). Birimi (ms/µs) kodda belirtilmiyor; Qapsule
  belgesi `TradeMessage`/`TimeMessage` için µs diyor (`qapsule:docs/MATRIKS_PROTOCOL.md`
  §10). Derinlik zaman damgasının birimi **bilinmiyor — Matriks veri ekibine sorulmalı**;
  bağlayıcı aynı değeri `tsEvent` olarak geçiriyor (`depth.transform.ts:55`).
- **Sıklık:** her geçerli derinlik mesajı → kısma denetimi (sembol başına ≥ `DEPTH_THROTTLE_MS`,
  varsayılan 100 ms, aralık 10–10000; `config.hpp:53`, `:255-257`) → geçerse o sembolün bütün
  göstergeleri güncellenir ve **her biri ayrı ileti olarak** yayımlanır
  (`depth_calculation_registry.cpp:352-385`). Kısmada `steady_clock` kullanılıyor
  (`depth_processor.cpp:70`).
- **Seans kapısı:** Piyasa kapalıyken hesap ve yayın yapılmaz (`depth_calculation_registry.cpp:354-356`;
  sinyaller de `signal_registry.cpp:1137-1139`). Açık/kapalı bilgisi `mx/timestamp`
  iletisinin yalnız `source == "MX"` olanından (`src/services/market_clock.cpp:115-140`);
  hiç ileti gelmediyse REST'ten alınan seans saatleri ve **sunucunun yerel saat dilimi**
  (`localtime_r`, `market_clock.cpp:25-62`); o da yoksa "kapalı".
- **Yayın yükü:** konu `quantex/{engine_type}/indicator/update/{key}` (vars. `engine_type=depth`;
  `depth_calculation_registry.hpp:109-111`), JSON: `type="indicator_update"`, `key`,
  `indicator`, `symbol`, `engine_type`, `engine_id`, `timestamp`, `values{...}`
  (`depth_calculation_registry.cpp:403-430`). Anahtar: `SEMBOL:GÖSTERGE[:levels]`
  (`:387-401`).

---

### 2. Sinyaller

#### 2.1 Tanım tablosu

Yükleme: `signal_registry.cpp:723-733`. Hepsi `eval_on_bar_close = false`; değerlendirme
her gösterge güncellemesinde (`signal_registry.cpp:357-447`). Koşullar VE ile bağlanır
(`:1161-1171`).

| Sinyal | Kural (kendi gösterimim) | Eşik / parametre (vars.) | Pencere | Girdi | Yön / önem / güven | Bastırma (`cooldown_bars`) | Kanıt |
|---|---|---|---|---|---|---|---|
| `IMBALANCE_EXTREME` | `|imbalance| > THRESHOLD` | `threshold` 0.7 | 1 ölçüm | OBI(5) | iki tetik: BULLISH ve BEARISH, HIGH, 70 | 3 | `src/signals/definitions/depth/imbalance_extreme.hpp:14-43` |
| `IMBALANCE_CROSS` | `imbalance` 0'ı yukarı ya da aşağı keser (önceki < 0 ≤ şimdiki ya da önceki > 0 ≥ şimdiki) | 0 (sabit) | 2 ardışık ölçüm | OBI(5) | NEUTRAL, MEDIUM, 60 | 3 | `imbalance_cross.hpp:18-35`; kesişim `signal_registry.cpp:1236-1284` |
| `DEPTH_TREND` | son 5 ölçümün hepsi > 0 ya da hepsi < 0 | `lookback` 5 (parametre formülde **kullanılmıyor**; 5 sabit) | 5 ölçüm | OBI(5) | NEUTRAL, MEDIUM, 65 | 5 | `depth_trend.hpp:14-35` |
| `DEPTH_PRESSURE_EXTREME` | `|pressure| > THRESHOLD` | `threshold` 0.6 | 1 ölçüm | DEPTH_PRESSURE(5) | iki tetik: BULLISH ve BEARISH, HIGH, 70 | 3 | `depth_pressure_extreme.hpp:14-42` |
| `SPREAD_SPIKE` | `spreadpercent > MULTIPLIER × ortalama(önceki 4 spreadpercent)` | `multiplier` 3.0 | 5 ölçüm | BID_ASK_SPREAD | NEUTRAL, HIGH, 75 | 5 | `spread_spike.hpp:14-35` |
| `LIQUIDITY_DRY` | `spreadpercent > spread_threshold` VE `cumbid + cumask < VOLUME_THRESHOLD` VE `cumask > 0` | `spread_threshold` 0.5, `volume_threshold` 1000 | 1 ölçüm | BID_ASK_SPREAD, CUMULATIVE_DEPTH(5) | BEARISH, HIGH, 70 | 10 | `liquidity_dry.hpp:14-54` |
| `BOOK_WALL_DETECT` | dış anomali olayında `BOOK_WALL` katkıcısının puanı ≥ 0.8 | `wall_multiplier` 5.0 (**kullanılmıyor**) | — | anomali motoru olayı (MQTT `quantex/anomaly/data/#`) | NEUTRAL, HIGH, 80 | 10 | `book_wall_detect.hpp:14-38`; değerlendirme `signal_registry.cpp:1735-1814` |

"Ölçüm" = kısmadan geçen bir gösterge güncellemesi. Pencere uzunluğu zamana değil ileti
sayısına bağlı: 5 ölçüm en az ~0,4 s (100 ms kısmayla), üst sınırı yok (seyrek defterde
dakikalar). Isınma: geçmiş yetersizse ifade "geçmiş aralık dışı" hatası verir ve koşul
yanlış sayılır (`src/signals/expression_evaluator.cpp:277-300`, yakalama
`signal_registry.cpp:1214-1234`).

Sinyal yayını: `quantex/depth/signal/data/{signal_id}/{symbol}/{period}` **ve**
`quantex/depth/signal/instance/{instance_key}` (aynı yük iki kez, `signal_registry.cpp:1430-1442`);
durum değişimi `quantex/depth/signal/status/{instance_key}` (`:1444-1462`). Yük alanları:
`signal_id`, `instance_key`, `direction`, `severity`, `confidence` (0–100 tamsayı), `timestamp`,
`price` (derinlik motorunda doldurulmuyor), `message`, `trigger` (değerler + koşul ayrıntısı),
`fire_count`, `subscriber_count` (`src/signals/signal_types.hpp:613-627`;
`signal_registry.cpp:1304-1428`). Bağlayıcı güveni 0–1'e çeviriyor
(`depth.transform.ts:33-37`).

#### 2.2 Kod okumasından çıkan işleyiş kusurları (çalıştırılarak doğrulanmadı)

| # | Bulgu | Sonuç | Kanıt |
|---|---|---|---|
| S1 | Formüllerdeki `$THRESHOLD`, `$MULTIPLIER`, `$VOLUME_THRESHOLD` değişkenleri yalnız takma ad değerleri (`latest_values`) içinde aranıyor; abonelik parametreleri değerlendirme bağlamına konmuyor; formül tanım anında tek kez ayrıştırılıp önbellekleniyor | "Bilinmeyen değişken" hatası → koşul her zaman yanlış: `IMBALANCE_EXTREME`, `DEPTH_PRESSURE_EXTREME`, `SPREAD_SPIKE`, `LIQUIDITY_DRY` tetiklenmez | `expression_evaluator.cpp:266-275`; `signal_registry.cpp:1521-1535`, `:44-76`, `:1467-1487`; parametre yalnız `threshold_param` yoluyla THRESHOLD/CROSSOVER koşullarına çözülüyor (`:1020-1037`) |
| S2 | Bekleme sayacı yalnız `is_closed` doğruyken azalıyor; derinlik motorunda `is_closed` her zaman `false` | İlk tetiklemeden sonra örnek kalıcı olarak COOLDOWN'da; yalnız bütün aboneler ayrılıp biri yeniden gelirse (IDLE → RUNNING) çıkar. `IMBALANCE_CROSS` ve `DEPTH_TREND` örnek başına bir kez tetiklenir | `signal_registry.cpp:361`, `:1146-1154`, `:1192-1197`, `:305-311`, `:223-229` |
| S3 | `on_anomaly_event` hiçbir yerden çağrılmıyor; `MqttManager::set_on_anomaly` `main.cpp`'de bağlanmıyor; `subscribe_anomaly_topics` çağrılmıyor | `BOOK_WALL_DETECT` hiç tetiklenmez | `signal_registry.cpp:1711`; `src/services/mqtt_manager.cpp:359-372`, `:541`; `src/main.cpp` (çağrı yok) |
| S4 | İki tetikli sinyallerde (BULLISH + BEARISH) tetiklerin `condition_indices` alanı boş; yön seçimi yok | Koşul sağlansaydı aynı anda hem alış hem satış olayı yayımlanırdı (S1 yüzünden şu an etkisiz) | `signal_registry.cpp:1173-1189`; `imbalance_extreme.hpp:27-39` |
| S5 | `DEPTH_TREND.lookback`, `BOOK_WALL_DETECT.wall_multiplier` örnek anahtarına giriyor ama hesaba girmiyor | Farklı parametre = aynı hesap, ayrı örnek | `depth_trend.hpp:16,24`; `book_wall_detect.hpp:16`; anahtar `signal_registry.cpp:745-780` |
| S6 | Her sinyal olayı iki MQTT konusuna gidiyor; bağlayıcı ikisini de aynı NATS konusuna eşliyor | NATS'ta her olay iki kez | `signal_registry.cpp:1433-1441`; `depth-indicator-adapter/src/nats/nats.publisher.ts:98-107`; `mqtt/mqtt.subscriber.ts:12-13` |
| S7 | Boşta temizliği `last_activity`'ye bakıyor ama `last_activity` her derinlik güncellemesinde yenileniyor | Abonesi kalmamış gösterge, sembol işlem gördükçe silinmez | `depth_calculation_registry.cpp:111-139`, `:369` |
| S8 | `SignalRegistry::cleanup_idle_instances`, `pause_instance`, `resume_instance` hiç çağrılmıyor | IDLE sinyal örnekleri ve iç göstergeleri bellekte kalır | tanımlar `signal_registry.cpp:317,337,690`; çağıran yok |

Bu kusurların gerçekte gözlenip gözlenmediği **bilinmiyor — Quantex servis sahibine
sorulmalı** (çalışan sistemin sinyal sayaçları `signal_fires_total` ölçütünde görünür,
`signal_registry.cpp:1424-1426`).

---

### 3. Abonelik

#### 3.1 Uçtan uca yol

İstemci → (ağ geçidi) → NATS `depth.cmd.*` (protobuf `DepthCommand`) → bağlayıcı → MQTT
`quantex/depth/indicator/cmd` (JSON) → her derinlik podu; yalnız Redis sahiplik kilidini
(`calc:{key}`, SET NX EX, TTL 30 s, 10 s'de bir yenileme) kazanan pod hesaplar ve yanıtlar
(`depth-indicator-adapter/src/nats/nats.cmd-listener.ts:30-35`, `:109-121`;
`depth_calculation_registry.cpp:269-275`; TTL `depth_calculation_registry.hpp:100-101`;
anahtar biçimleri `src/services/redis_service.cpp:105,160,208`).
Sinyal için aynı yol: NATS `depth.signal.cmd.*` → MQTT `quantex/depth/signal/cmd`
(`nats.signal-cmd-listener.ts:27-30`).

Pod ayrıca kendine özgü konuları da dinliyor: `quantex/engine/{engine_id}/indicator/cmd`,
`.../signal/cmd` (`mqtt_manager.cpp:204-209`, `:230-236`). Pod kimliği Redis sayacından
(`engine:seq:depth`) otomatik (`src/main.cpp:70-86`).

#### 3.2 İstek biçimi (JSON, MQTT)

| `action` | Alanlar | Yanıt (`replyTo` konusuna) | Kanıt |
|---|---|---|---|
| `subscribe` | `symbol` (`^[A-Z0-9]{2,10}$`), `indicator` (büyük/küçük harf duyarsız), `params` (`{levels}`), `replyTo`, isteğe bağlı `reassert` | `{type:"subscribed", key, indicator, symbol, engine_id, data_topic, timestamp}` ya da `{type:"error", code, message}` | `main.cpp:199-200`; `depth_calculation_registry.cpp:202-317`; regex `src/security/validation.cpp:29` |
| `unsubscribe` | `key`, `replyTo` | `{type:"unsubscribed", key}` | `depth_calculation_registry.cpp:319-350` |
| `signal_subscribe` | `signal_id`, `params`, `userId`, `delivery`, `replyTo`, `reassert` | `{type:"signal_subscribed"\|"signal_error", instance_key, status, engine_id}` | `main.cpp:203-231` |
| `signal_unsubscribe` | `instance_key`, `userId`, `replyTo` | `{type:"signal_unsubscribed", ...}` | `main.cpp:232-245` |
| `signal_list`, `signal_stats` | `replyTo` | sinyal kimlikleri / istatistik | `main.cpp:246-259` |

Bilinmeyen gösterge adı sessizce yok sayılır, yanıt gitmez (`depth_calculation_registry.cpp:225-228`).
Aynı anahtara ikinci abone mevcut hesabı paylaşır (`:235-267`). Ham derinliğe (Matriks
`mx/depth/{SYMBOL}`) abonelik ilk gösterge için açılır, sembolün son göstergesi silinince
kapanır (`:296`, `:453-459`; `mqtt_manager.cpp:380-435`).

#### 3.3 İsteğe bağlı parametre ve çapraz gösterge

- **Parametre:** yalnız `levels` (tamsayı 1–20; sınır dışı kırpılır). Kapalı bir kümeden
  (5 gösterge) seçilir; yeni formül istenemez.
- **Çapraz gösterge:** derinlik göstergeleri iç içe geçemez (`nestable=false`). Sinyal
  katmanında gösterge-gösterge kesişimi (CROSSOVER, `alias_b`) altyapısı var
  (`signal_registry.cpp:984-1014`, `:1252-1267`) ve JSON'dan sinyal tanımı ayrıştırıcısı var
  (`parse_signal_definition_json`, `:1540`), ama `main.cpp`'de özel sinyal kaydeden bir komut
  yok; yalnız yedi çekirdek sinyal abone olunabilir.

#### 3.4 Konu kalıpları

| Yön | MQTT (motor) | NATS (bağlayıcı) | Kanıt |
|---|---|---|---|
| Gösterge güncellemesi | `quantex/depth/indicator/update/{key}` | `depth.{exchange}.{symbol}.data.{key}` | `nats.publisher.ts:86-91` |
| Gösterge yanıtı | `{replyTo}` (bağlayıcı `quantex/depth/indicator/reply/#` dinliyor) | `depth.reply.{clientId}.{key}` | `nats.publisher.ts:92-97`; `mqtt.subscriber.ts:11` |
| Sinyal olayı | `quantex/depth/signal/data/{signal_id}/{symbol}/{period}` + `.../instance/{instance_key}` | `depth.signal.{exchange}.{symbol}.{signal_id}` | `nats.publisher.ts:98-107` |
| Sinyal durumu | `quantex/depth/signal/status/{instance_key}` | `depth.signal.status.{instance_key}` | `nats.publisher.ts:108-112` |
| Motor durumu / nabız (30 s) | `quantex/engine/status` (kalıcı), `quantex/engine/heartbeat` | `depth.engine.status`, `depth.engine.heartbeat` | `main.cpp:285`, `:329-341`; `nats.publisher.ts:119-122` |
| Motor kaydı | `quantex/registry/engine/depth/{engine_id}` (kalıcı; gösterge listesi, komut konuları) | — (bağlayıcı içinde kayıt) | `main.cpp:287-301`; `nats/engine-registry.ts` |

Not: depodaki deneme betiği güncellemeleri `quantex/engine/{id}/depth/indicator/update/#`
konusunda bekliyor (`tests/test_depth_subscribe.py:160`); motor bu konuya yazmıyor.

---

### 4. Fiyat adımı (`src/utils/bist_tick_size.hpp`)

#### 4.1 Koddaki tablo (`bist_tick_size.hpp:5-14`)

Fiyat `p` için adım; her eşik **dahil üst sınır** (`p ≤ eşik`):

| Fiyat aralığı (kod) | Adım (TL) |
|---|---|
| p ≤ 20 | 0,01 |
| 20 < p ≤ 50 | 0,02 |
| 50 < p ≤ 100 | 0,05 |
| 100 < p ≤ 250 | 0,10 |
| 250 < p ≤ 500 | 0,25 |
| 500 < p ≤ 1000 | 0,50 |
| 1000 < p ≤ 2500 | 1,00 |
| p > 2500 | 2,50 |

- Tek kullanım yeri: `BID_ASK_SPREAD.spreadticks`, adım **orta fiyata** göre seçiliyor
  (`bid_ask_spread.hpp:28-29`). Başka hiçbir yerde kullanılmıyor (src araması).
- Tablo bütün sembollere aynı uygulanıyor; enstrüman türü (pay, varant, ETF, VİOP) ayrımı
  yok; adım bilgisi Matriks'ten okunmuyor.
- Varsayım: BIST pay piyasası kuralında bantların alt sınırı dahildir (ör. 20,00 TL'de adım
  0,02). Doğruysa kod tam eşik fiyatlarında bir alt adımı veriyor (ör. p = 20 → 0,01) ve
  orta fiyat iki bandın sınırında olduğunda `spreadticks` bir bant kayabilir. Doğrulama:
  Borsa İstanbul pay piyasası işlem esasları (kamuya açık) ya da Matriks meta veri uç noktası
  (`qapsule:docs/DATA_ACCESS.md:159` "fiyat adımı").

#### 4.2 Qapsule karşılığı

- `crates/qp-calendar`, `crates/qp-schema`, `qp-source`, `qp-render` içinde `tick_size`,
  `price_step`, fiyat adımı yok (`.rs` araması boş; `tick` geçenler sentetik akışın
  sayaçları: `qapsule:crates/qp-source/src/synthetic.rs:97-158`).
- Konu açık karar olarak duruyor: A8 "Tamsayı fiyat adımı (kuruş) … Enstrümana fiyat adımı
  alanı gerekir" (`qapsule:DECISIONS.md:78`). Quantex tablosu bu karar için kaynak olamaz
  (sabit, sınır anlamı belirsiz); Matriks meta verisinden enstrüman başına adım okumak
  daha güvenli (öneri).

---

### 5. Qapsule ile karşılaştırma

#### 5.1 Hesap/dedektör karşılığı

`crates/` altında derinlik, emir defteri, spread, dengesizlik hesabı ya da dedektörü
**yok**: `qp-compute/src/studies/` = atr, bollinger, ema, extremum, macd, rsi, sma;
`qp-facts/src/` = breakout_range, extreme_rsi, flow_institutional, ma_stack, rsi_divergence,
squeeze_bandwidth, volume_surge. `.rs` dosyalarında `depth|order_book|imbalance|spread`
araması yalnız ilgisiz eşleşme verdi (sunum derinliği, test yorumları). **Karşılığı yok.**

#### 5.2 Anlam türü ön eşlemesi (K30; ana eşleme anomali belgesinde)

| Quantex sinyali | En yakın anlam türü | Not |
|---|---|---|
| `IMBALANCE_EXTREME`, `DEPTH_PRESSURE_EXTREME` | aşırılık (akış ailesi kanıtı) | Tek ölçümlük eşik; süreklilik yok |
| `IMBALANCE_CROSS` | dönüş | Gürültüye çok açık: 100 ms çözünürlükte sıfır kesişimi |
| `DEPTH_TREND` | devam (akış ailesi) | 5 ileti; zaman ölçeği belirsiz |
| `SPREAD_SPIKE` | yakın tür yok (likidite/oynaklık olayı); en yakın aşırılık | |
| `LIQUIDITY_DRY` | karşılığı yok (likidite rejimi); sıkışma değil (sıkışma fiyat oynaklığı daralmasıdır) | Mutlak hacim eşiği (1000 lot) sembolden bağımsız |
| `BOOK_WALL_DETECT` | karşılığı yok (destek/direnç kanıtı olabilir → kırılım ailesine kanıt) | Bağlı değil (S3) |

Hepsi anlık defter durumuna dayanıyor; Qapsule dedektörleri bar tabanlı. Derinlikten
olgu üretilecekse bar hizası (K13) için bir özetleme kuralı (bar içi ortalama, kapanış
anı görüntüsü vb.) tanımlanmalı; Quantex'te böyle bir kural yok.

#### 5.3 Ad çakışması

Qapsule sözlüğünde **Derinlik = `Depth`** sunum (anlam zoom'u) seviyesidir
(`qapsule:docs/CONCEPTS.md:17`, `:74`); **kademe** de önem kademesi anlamında kullanılıyor
(`CONCEPTS.md:150-151`). Emir defteri derinliği ve defter kademesi için ayrı terim gerekir
(öneri: "emir defteri", "defter kademesi"; sözlüğe eklenmeden kullanılmamalı — CLAUDE.md
kuralı).

---

### 6. Lisans ve yetki

- **Kademe sayısı:** Göstergeler en çok 20 kademe kullanabiliyor (`levels` üst sınırı); kaç
  kademe geldiği gelen mesaja bağlı, kodda sabit yok. Qapsule belgesine göre `@lvl2` ilk 10,
  `@lvl3` kalan 15 kademe (`qapsule:docs/MATRIKS_PROTOCOL.md:264`). Bu servis soneksiz konu
  kullandığı için hangi kademe kümesini aldığı **bilinmiyor** (Matriks veri ekibi). `levels`
  10'dan büyükse ve yalnız lvl2 geliyorsa sessizce eldeki kadar toplanır.
- **Lisans sınıfı izi:** Kodda derinlik lisansı, paket, kademe yetkisi izi yok. `INSUFFICIENT_LICENSE`
  hata kodu tanımlı ama hiçbir yerde üretilmiyor (`src/core/error.hpp:18`, `:102`; başka
  kullanım yok). Matriks'e hizmet hesabıyla (`typeCode` vars. `"H"`, `config.hpp:32`;
  `src/services/token_service.cpp:181`) bağlanılıyor; derinliğin hangi lisansla alındığı
  **bilinmiyor — Arkin / Matriks lisans sorumlusu**.
- **`jwt_validator`:** imza (HS256 ya da RS256), `issuer`, süre (`exp`); talep olarak `sub`,
  `tier`, `iat`, `permissions` okunuyor (`src/security/jwt_validator.cpp:47-99`,
  `jwt_validator.hpp:13-19`). `permissions` hiçbir yerde kullanılmıyor. Anahtar
  yapılandırılmamışsa doğrulama atlanıp `tier=premium` veriliyor (`jwt_validator.cpp:37-43`).
  JWT varsayılan kapalı (`config.hpp:88`); bağlayıcı komutlara `token` koymuyor
  (`nats.cmd-listener.ts:109-117`), yani bağlayıcı yolunda JWT açılırsa komutlar reddedilir —
  pratikte kimlik denetimi ağ geçidinde olmalı (Varsayım; ağ geçidi bu görevin kapsamı
  dışında).
- **`validation`:** sembol biçimi `^[A-Z0-9]{2,10}$`, JSON yükü ≤ 4 KB, dize ≤ 100
  (`src/security/validation.cpp:29-35`; `validation.hpp:109-119`). İçerik/yetki denetimi değil.
- **İstek sıklığı:** Redis tabanlı, 60 s pencerede free 10 / standard 30 / premium 100 komut
  (`config.hpp:78-85`; `main.cpp:88-104`; `mqtt_manager.cpp:591-604`). Kimlik `replyTo`
  konusundan çıkarılıyor (`mqtt_manager.cpp:639-656`).
- **K33'e göre:** derinlik verisinin gösterimi lisanslı olabilir; servis ham defteri yaymıyor
  ama `bestbid`, `bestask`, `cumulativebid/ask`, emir sayıları gibi **ham değerleri doğrudan**
  yayımlıyor (`depth_calculation_registry.cpp:418-427`). Qapsule'ün "sunucu olgusu" ilkesine
  (`qapsule:docs/CAPSULE.md` §7: rakam yok, yalnız nitel ilişki) göre bu çıktılar yetkisiz
  okuyucuya olduğu gibi gidemez.

---

### 7. Performans, ölçek, determinizm

#### 7.1 Performans / ölçek (kodda ve dağıtım dosyasında yazılı olanlar)

| Konu | Değer | Kanıt |
|---|---|---|
| Kısma | sembol başına ≥ 100 ms (10–10000) → en çok ~10 hesap/s/sembol | `config.hpp:53`, `:255-257` |
| İşçi | bütün semboller ve komutlar için **tek** komut iş parçacığı; kuyruk sınırsız | `depth_calculation_registry.cpp:82-109`; `src/core/thread_safe_queue.hpp` (kapasite yok) |
| Yayın kuyruğu | ≤ 10000 ileti (100–1 000 000) | `config.hpp:67`, `:287-289`; `mqtt_manager.cpp:278-301` |
| Kaynak | CPU 0,5–2, bellek 128–512 MiB | `depth-indicator-deploy.yml:71-76`; `docker-compose.yml:24-29` |
| Çoklu pod | Redis kilidiyle anahtar başına tek sahip; kilit kaybında ~30 s devralma | `depth_calculation_registry.hpp:97-101`; `:163-200` |
| Gösterge maliyeti | kademe sayısında doğrusal, O(N ≤ 20) | §1 dosyaları |

Sembol sayısı, gecikme, ölçülmüş verim belgede ya da kodda **yok**; servisin belgesi yok
(`qapsule:rounds/RI1/experiments/MAP.md:16`). **Bilinmiyor — Quantex servis sahibi
(Prometheus ölçütleri: `depth_snapshots_processed`, `depth_throttle_drops`,
`depth_indicator_calculations`).**

#### 7.2 Determinizm (K3)

- Gösterge formülleri saf ve deterministik; rastgelelik ya da dil modeli yok.
- **Hangi görüntülerin hesaplandığı deterministik değil:** kısma geliş anındaki
  `steady_clock`'a bağlı (`depth_processor.cpp:69-86`). Aynı ileti dizisi farklı hızda
  oynatılırsa farklı görüntüler düşer → farklı gösterge dizisi ve farklı sinyaller (sinyal
  pencereleri ileti sayısıyla tanımlı, §2.1).
- Zaman damgası eksikse sunucu saati (`depth_processor.cpp:26`); seans kapısı canlı
  `mx/timestamp` durumuna ya da sunucunun yerel saatine bağlı (`market_clock.cpp:25-62`).
- Rastgelelik yalnız kimlik üretiminde (cihaz/kaynak kimliği, `config.hpp:156-179`; MQTT
  istemci kimliğinde `time(nullptr)`, `mqtt_manager.cpp:85,158`); hesaba girmiyor.
- Sinyal geçmişi ve durum (COOLDOWN vb.) yalnız bellekte; yeniden başlatmada sıfırlanır,
  geçmişe dönük kayıt yok.

Sonuç: Qapsule yeniden oynatması (K44) için bu servisin çıktısı yeniden üretilemez; aynı
formüller Qapsule'de kısma olmadan ileti dizisi üzerinde (ya da bar özetinde) yeniden
yazılırsa deterministik olur.

---

### 8. Arkin'in 5. bilgisi — derinlik servisi yanı

> "Gösterge servislerine gösterge ve çapraz gösterge aboneliğiyle dinleme yapılabilir;
> anomali motoru piyasa anomalilerini yakalar; hepsi NATS üzerinden yayınlar."

**Hüküm: kısmen doğrulandı.**

- Gösterge aboneliği: **doğrulandı** — `subscribe`/`unsubscribe` komutları, anahtar başına
  paylaşılan hesap, yalnız `levels` parametresi (§3).
- Çapraz gösterge aboneliği: **derinlik servisinde kodda görünmüyor** — göstergeler iç içe
  geçemez; sinyal katmanında gösterge-gösterge kesişimi altyapısı var ama çekirdek
  sinyallerden hiçbiri kullanmıyor ve özel sinyal tanımlama komutu yok (§3.3).
- NATS üzerinden yayın: **kısmen** — motor yalnız MQTT'ye yazar; NATS'a taşıyan
  `depth-indicator-adapter` bağlayıcısıdır (MQTT → protobuf zarf → NATS, `nats.publisher.ts:61-122`).
  Komutlar da NATS'tan MQTT'ye aktarılır (`nats.cmd-listener.ts`). Dış istemci açısından
  NATS doğru; servis açısından değil.
- Ek: 3. bilgiyle (ilk mesaj tam, sonra güncelleme) **çelişki olası** — bu servis artımlı
  güncellemeyi uygulamıyor (§0/2). 4. bilgi (açıklık zaman damgasıyla gelir) bu serviste
  **doğrulandı** (`market_clock.cpp:115-140`).

---

### 9. Açık sorular

| # | Soru | Kime |
|---|---|---|
| 1 | `mx/depth/{SYMBOL}` (soneksiz, `/depth` yolu) tam defter mi gönderiyor, artımlı mı? Kaç kademe? `@lvl2/@lvl3` ile farkı ne? | Matriks veri ekibi |
| 2 | `DepthTableMessage.timestamp` birimi (ms mi µs mi)? | Matriks veri ekibi |
| 3 | Derinlik verisi hangi lisans sınıfında alınıyor; türev değerlerin (`bestbid`, `cumulativebid`, emir sayısı) yetkisiz kullanıcıya yayını lisansa uygun mu? | Arkin / Matriks lisans sorumlusu |
| 4 | §2.2'deki S1–S3 canlıda gözleniyor mu (sinyal tetik sayaçları)? Derinlik sinyalleri üretimde kullanılıyor mu? | Quantex servis sahibi |
| 5 | Sembol sayısı, gecikme ve kısma düşüş oranı ölçümleri var mı? | Quantex servis sahibi |
| 6 | BIST fiyat adımı bantlarının alt sınırı dahil mi; varant/ETF için ayrı tablo gerekir mi? Matriks meta verisi enstrüman başına adım veriyor mu? | Matriks veri ekibi (meta veri), borsa kuralı |
| 7 | Qapsule'de emir defteri kavramları için sözlük adı ("emir defteri", "defter kademesi") | Ana akış oturumu (CONCEPTS.md) |

---

