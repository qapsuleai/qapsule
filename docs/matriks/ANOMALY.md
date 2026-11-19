# Anomali motoru (RI1 / GOAL §E)

Kaynak: `quantex` deposu, salt okunur inceleme (derleme, çalıştırma, canlı bağlantı yok).
Bütün yollar aksi yazılmadıkça `services/quantex-anomaly-engine/` köküne göredir; `src/...`
= `services/quantex-anomaly-engine/src/...`. Satır numaraları inceleme anındaki ağaca göredir.
"Varsayım:" işaretli cümleler koddan çıkmaz; "bilinmiyor" diyenler için kime sorulacağı yazılıdır.
Dağıtımdaki ortam değişkenlerinin değerleri depoda yok (`cpp-anomaly-engine-deploy.yml:47-70`
yalnız adlarını listeler); aşağıdaki eşikler **koddaki varsayılanlardır**.

## 0. Bir bakışta

- Motor C++; fabrikada 26 kayıt: 25 anomali dedektörü + `REGIME` (`src/detectors/anomaly_detector_factory.hpp:110-715`, `REGIME` `:705`). Her
  sembol için **her kayıtlı dedektör** boş parametreyle kurulur, yani fabrika varsayılanları
  geçerlidir (`src/main.cpp:454`, `:1097-1099`). `DETECTORS_ENABLED` ayarı okunur ama
  kullanılmaz (`src/config/config.hpp:66`, `:526`; `main.cpp` içinde `get_enabled_detectors`
  çağrısı yok).
- Dışarıya **tür başına anomali gitmez.** Dedektörler sürekli bir puan seviyesi tutar
  (0–1); bir **birleşik motor** (`src/engine/composite_engine.hpp:240-544`) bunları sembol
  başına ağırlıklı ortalamaya çevirir, eşik/kalıcılık/soğuma geçerse **tek bir birleşik
  anomali olayı** yayınlar. Olayın içinde katkı veren dedektörlerin listesi
  (`contributors`) ve her birinin bağlam sözlüğü (`context.detectors`) vardır.
- Yayın MQTT'ye (istemci aracısı) yapılır: `quantex/anomaly/{data,outcome,regime,health}/…`.
  NATS'a ayrı bir bağlayıcı (`services/quantex-stream/anomaly-adapter`) köprüler:
  `anomaly.BIST.<sembol>.{data,outcome,regime}` ve `anomaly.health.<motor>`.
- Geçmiş kaydı **Postgres değil ClickHouse**'tadır (`migrations/001_baseline_anomaly_tables.sql`).
  Sonuç (outcome) izlenir: olay başına, katkı veren dedektör başına, saniye cinsinden bir
  pencere sonunda fiyat yönüyle "doğrulandı/doğrulanmadı" yazılır.
- **Deterministik değil** (§3): olay kimliği rastgele, tarih ve seans durumu duvar saatinden,
  akışlar ayrı iş parçacıklarında, çapraz varlık durumu duvar saatiyle çalışan bir işçiden,
  eşikler/ağırlıklar dışarıdan (öğrenme ve yapay zekâ geri bildirimi) çalışırken değişebilir.
  Motorun kendisinde dil modeli çağrısı yok; yapay zekâ servisi (LLM kullanan ayrı servis)
  geri bildirimle eşiği yükseltebilir.

## 1. Birleşik motor: anomali nasıl doğar

Akış (her derinlik, işlem, PGC, fiyat ve gösterge olayında; `src/main.cpp:1512`, `:1701`,
`:1752`, `:1779`):

1. **Kısma:** sembol başına olay zamanıyla 500 ms (`main.cpp:904`; `composite_engine.hpp:252-258`).
2. **Sönüm:** her dedektörün puanı `exp(-oran·Δt)` ile azaltılır; Δt olay zamanı farkıdır
   (`base_anomaly_detector.hpp:51-58`, `composite_engine.hpp:819-831`). Oran varsayılan
   0,05/sn (`src/core/types.hpp:275`).
3. **Ağırlıklı toplam:** puanı ≥ 0,15 (`min_significance`, `config.hpp:128`) olan dedektörler
   `puan × ağırlık` ile toplanır, ağırlık toplamına bölünür (`composite_engine.hpp:278-358`).
   Ağırlık = dedektör dosyasındaki `meta().default_weight` (fabrika kaydındaki değil; ikisi
   SPOOF, ICEBERG, AGENT_SHIFT, ENTROPY_RATE için farklı) → öğrenilmiş ağırlık varsa o →
   seans çarpanı (açılışta derinlik ×0,5, kapanışta işlem ×1,2) → rejim çarpanı
   (`composite_engine.hpp:645-667`).
4. **Güçlendirme:** son 5 sn içinde puanı ≥ 0,5 olan farklı dedektör sayısı `n` ise
   `1 + (n−1)·0,15`; öğrenilmiş "çift" çarpanı [0,85, 1,25]; gösterge kuralı çarpanı;
   yapay zekâ öncelik çarpanı [1, 2] (`composite_engine.hpp:362-392`, `:799`;
   `src/engine/pair_boost.hpp:9-10`; `src/engine/priority_boost.hpp:35`). Sonuç 1'de kesilir.
5. **Eşik:** sürekli işlemde 0,70, açılış müzayedesinde 0,80, kapanışta 0,72
   (`config.hpp:71-73`) × rejim eşik çarpanı [0,5, 1,5] + yapay zekâ "yanlış pozitif" artışı
   (≤ +0,20, en çok 0,95) (`composite_engine.hpp:395-401`; `src/engine/adaptive_threshold.hpp:33-70`).
6. **Kalıcılık:** eşik art arda 3 değerlendirmede aşılmalı (`config.hpp:76`;
   `composite_engine.hpp:411-415`). Aşılmayan değerlendirme sayacı sıfırlar.
7. **Yön:** boğa katkıları toplamı − ayı katkıları toplamı ≥ 0,1 ise o yön, değilse nötr
   (`composite_engine.hpp:418-423`).
8. **Şiddet:** puan ≥ 0,90 `critical`, ≥ 0,75 `high`, ≥ 0,60 `medium`, değilse `low`
   (`src/core/types.hpp:319-324`).
9. **Soğuma (tekrar bastırma):** sembol başına, tür ayırmadan: critical 10 sn, high 30 sn,
   medium 60 sn, low 120 sn. Soğuma sırasında daha yüksek şiddet gelirse kalan süre yarıya
   iner ("tırmanma") (`src/engine/cooldown_manager.hpp:11-66`).
10. **Güven:** `0,35·ağırlıklı dedektör güveni + 0,25·yön uyumu + 0,20·etkin dedektör oranı
    + 0,20·puan tutarlılığı` (`composite_engine.hpp:455-506`).
11. **Uyum katmanı (SAL):** etkin/boğa/ayı sayısı, uyum oranı, çelişki, en güçlü çift ve
    kademe (`single|aligned|mixed|conflict`) hesaplanır; yayını kapılamaz, kanıt olarak
    eklenir (`src/engine/signal_agreement.hpp:24-37`, `composite_engine.hpp:509-523`).

**Yaşam döngüsü:** yayınlanan olayda başladı/sürüyor/bitti yoktur. Her olay bağımsız bir
"ateşleme"dir; aynı durum sürdükçe soğuma bitince yeniden ateşlenebilir. Dedektörlerin
çoğu da kenar değil **seviye** tutar (puan her olayda yeniden hesaplanır ya da söner).
Bitiş olayı yalnız rejim tarafında dolaylı vardır (eksen geçişi, §2.6).

**Gösterge girdisi:** motor `quantex/indicator/data/#` aboneliğiyle gösterge değerleri alır
(`main.cpp:1812`; `src/ingestion/indicator_ingester.hpp:17-21`). Dikkat (denetimde bulundu):
fiyat motoru göstergeleri `quantex/price/indicator/update/<key>` konusuna yazıyor
(`docs/matriks/INDICATORS.md` T4a §0); bu iki konu ağacı eşleşmediği için gösterge girdisi
büyük olasılıkla **boş** kalıyor (koddan çıkarım; kime: Quantex anomali motoru geliştiricisi). Bunlar dedektör değil, yalnız
birleşik puanı çarpan dört kuralda kullanılır (`config/indicator_config.json`, kural adları:
`rsi_pgc_divergence`, `rsi_overbought_pgc`, `bb_depth_breakout`, `adx_agent_shift`). Üç kural
dedektör adını `PGC_DIVERGENCE` / `AGENT_SHIFT` diye yazar, motor ise tam ad eşitliği arar
(`composite_engine.hpp:750`; dedektör adları `…_DETECTOR`). Bu üç kural koddan okunduğu
haliyle **hiç tutmaz**; yalnız `bb_depth_breakout` (BB %B > 1 ve bir derinlik dedektörü
≥ 0,4 → ×1,20) çalışabilir.

## 2. Anomali türleri tablosu

Ortak notlar (tabloda tekrar edilmez):

- **Taban (baseline):** "z" yazan her yerde taban, sürecin başından beri biriken Welford
  ortalama/sapmasıdır (pencere yok; `src/core/types.hpp:328-356`). Güncel değer z'si
  alınmadan tabana eklenir. Seans açılışında sıfırlama yöntemi var (`base_anomaly_detector.hpp:69`)
  ama hiçbir yerden çağrılmaz (`main.cpp` içinde `on_session_open` yok). Yani taban sürecin
  yeniden başlatıldığı ana bağlıdır.
- **Sönüm:** aksi yazılmadıkça 0,05/sn; 0,01 altı sıfır.
- `fire_threshold_` (0,65; öğrenme ile değişebilir) çoğu dedektörde yalnız açıklama ve bağlam
  doldurulup doldurulmayacağını belirler; puanı kapılamaz. "bağlam ≥ 0,65" = bağlam yalnız
  puan 0,65'i geçince dolar.
- Girdi kısaltmaları: **D1** derinlik en iyi kademe, **D2** derinlik defteri (çok kademe),
  **Dİ** derinlik ekle/sil olayları, **B** emir bazlı defter (booklet), **İ** işlem (fiyat,
  adet, aktif taraf), **İK** işlemde alıcı/satıcı kurum kodu, **F** fiyat tiki, **P** PGC,
  **S** sektör eşleri (paylaşılan durum), **H** haber, **R** dış REST verisi.

### 2.1 Derinlik (kategori `depth`)

| Tür (ad) | Kural (kendi cümlemle) | Eşik / pencere | Girdi | Puan · güven | Yön | Bastırma | Bağlam (kanıt) |
|---|---|---|---|---|---|---|---|
| BAAM_DETECTOR | En iyi alış kuyruğunun (alış+satış) içindeki payının z'si; değişim hızı ve son 10 örnekte uç kalma oranı ile birleşir | z ≥ 2,5; ısınma 20 örnek; tarih halkası 60 (`src/detectors/depth/baam_detector.hpp:13-20`, `:202-251`) | D1, F (orta fiyat) | `0,40·z + 0,35·hız + 0,25·kalıcılık`; güven veri/istatistik/kararlılık (`:228`, `:281-302`) | z işareti; orta fiyat iddianın tersine 5 bp gitmişse veto (`:236-246`, `:266-273`) | soğuma 15 sn; puan < 0,70 → 0 (`:248-258`) | **yok** (yalnız motorun eklediği `trigger_price`) |
| OFI_DETECTOR | Cont tarzı emir akışı dengesizliği: en iyi kademe değişimlerinin son 20 olaydaki toplamının z'si | z ≥ 2,5; 20 olay; ısınma 30 (`ofi_detector.hpp:29-36`, `:193-205`) | D1 | `min(|z|/3,1)`; güven `0,4·veri + 0,6·(|z|−2,5)/2` (`:212-215`, `:258-267`) | z işareti (`:217-227`) | soğuma 10 sn; < 0,55 → 0 (`:228-236`) | `e_n, ofi_cum, z_score, cum_window, best_bid, best_ask, best_bid_qty, best_ask_qty` (`:240-249`); eşik yok |
| SPOOF_DETECTOR | Büyük ekleme (≥ 10.000 lot) aynı fiyat ve tarafta 3 sn içinde silinir, az dolar; 60 sn'de ≥ 2 kez; yakınlık, çok kademe ve iptal sonrası karşı taraf saldırısı çarpanları | `spoof_detector.hpp:33-46`; açılışta lot ×2, kapanışta ömür ×1,5 (`:341-355`) | Dİ, B, İ, F | `(0,4·boy + 0,3·hız + 0,3·dolmama)·yakınlık × çarpanlar` (`:299-309`, `:392-420`) | aday alışta → ayı, satışta → boğa (`:433-435`) | **soğuma yok**; < 0,55 → 0; aday 60 sn yaşar | `side, price, lot_size, lifetime_us, fill_ratio, spoof_count, unique_levels, …boost, proximity` (`:443-455`); eşik yok |
| ICEBERG_DETECTOR | Büyük seviye işlemle dolduktan sonra 5 sn içinde tepe miktarın ≥ %70'ine yeniden dolar; ≥ 3 kez | `iceberg_detector.hpp:34-40`; açılışta atlanır (`:81`) | Dİ, B, İ | `0,5·dolum sayısı + 0,3·oran + 0,2·hacim` (`:263-291`) | hep nötr (`:294`) | soğuma ve en az puan **tanımlı ama kullanılmıyor** | **yok** |
| OB_GRADIENT_DETECTOR | Defter eğimi: her taraf için `Σ miktar/|fiyat−orta|`, iki tarafın farkının oranı (ΔS); z'si; uzaklığa göre ağırlıklı dengesizlik aynı yönde olmalı; şekil sapması yalnız güçlendirir | z ≥ 2,0; 10 kademe; ısınma 25 (`ob_gradient_detector.hpp:32-41`) | D2 | `min(|z|/4,1) + şekil (≤ 0,20)`; güven (`:132-138`, `:337-343`) | z işareti, dengesizlik uyuşursa (`:121-140`) | soğuma 10 sn; < 0,55 → 0 (`:142-151`) | `delta_s, z, weighted_i, s_bid, s_ask, bid_div, ask_div`; bağlam ≥ 0,65 (`:174-182`) |
| VELOCITY_FIELD_DETECTOR | Kademe başına miktar değişim hızının büyüklüğü (iki taraf, karekök toplam) olağandışı yüksek | tek yönlü z ≥ 2,5; ısınma 20 (`velocity_field_detector.hpp:28-31`) | D2 | `min(1,(z−2,5)/3 + 0,5)` (`:98-101`) | hep nötr | soğuma 15 sn (`:103-109`) | `magnitude, z_score, mean, std_dev`; bağlam ≥ 0,65 (`:125-133`) |
| CROSS_DEPTH_CORR_DETECTOR | Sembolün defter dengesizliği − sektör eşlerinin ortalaması (her 10 derinlik olayında) | |z| > 2,5; ≥ 2 eş; ısınma 20 (`cross_depth_corr_detector.hpp:28-32`, `:80-103`) | D2, S | `min(1,(|z|−2,5)/3 + 0,5)` (`:118-120`) | z işareti | soğuma 15 sn | `imbalance, deviation, z_score, sector`; bağlam ≥ 0,65 (`:153-158`) |

### 2.2 İşlem (kategori `trade`)

| Tür | Kural | Eşik / pencere | Girdi | Puan · güven | Yön | Bastırma | Bağlam |
|---|---|---|---|---|---|---|---|
| AGENT_SHIFT_DETECTOR | Son 5 dk'daki (500'lük halka) işlemleri adetçe ikiye böl; bir kurumun net pozisyonu ilk yarıdan ikinciye işaret değiştirir, değişim büyük ve kurum ilk 5'te | ısınma 100 işlem; hacim ≥ max(1 M TL, OGH·0,001); oran ≥ 0,75 (açılışta 0,975) (`agent_shift_detector.hpp:22-36`, `:204-247`) | İ, İK, F, R (takas geçmişi), OGH | `oran × hacim ağırlığı × 1,15`; takas eğilimi uyarsa ×1,15, uymazsa ×0,55, ≥ 5 gün ters eğilimde veto (`:249-262`, `:427-447`) | ikinci yarı net > 0 → boğa (`:267`) | aynı kurum için 30 sn; < 0,75 → 0; orta fiyat vetosu 5 bp; sönüm 0,10 (`:292-302`, `:356-370`) | `agent, agent_name, net_before, net_after, agent_volume_tl, window_trades, shift_direction, mid_change_bps, settlement_trend{…}` (`:327-347`); oran ve eşik yok |
| INFORMED_TRADE_DETECTOR | Olağandışı büyük (z) işlem; hızlı geliş ve sık işlem yapan kurum ek puan; habere yakınsa çarpan | tutar ≥ max(100 bin TL, OGH·0,0005); z > 2,5; ısınma 20 (`informed_trade_detector.hpp:25`, `:94-98`, `:140-144`) | İ, İK, H, OGH | `0,4·boy + 0,3·zamanlama + 0,3·kurum sırası`; haber öncesi ×1,2, haberle ×0,85 (`:171-172`, `:207-213`) | aktif taraf alış → boğa, satış → ayı (`:173-175`) | soğuma 15 sn; < 0,55 → 0 (`:182-190`) | `size_z, size_factor, timing_factor, agent, agent_name, agent_factor, news_context{…}` (`:220-235`) |
| TRADE_CLUSTER_DETECTOR | 10 sn'deki işlem sayısı patlaması + kısa/uzun hız oranı + bir tarafın ağır basması | z > 4,5 (fabrika; dosyada 3,0); ≥ 15 işlem; tutarlılık ≥ 0,65, ≥ 8 taraflı işlem (`anomaly_detector_factory.hpp:327-335`; `trade_cluster_detector.hpp:19-25`) | İ | `0,5·küme + 0,3·ivme + 0,2·taraf` (`:184-186`) | ağır basan taraf | soğuma 15 sn; < 0,55 → 0; fiyat vetosu 5 bp (`:199-211`, `:300-307`) | `window_count, z_score, mean, std_dev, short_rate, long_rate, acceleration, …, buy_qty, sell_qty` (`:231-245`) |
| KYLE_LAMBDA_DETECTOR | Orta fiyat değişiminin işaretli TL hacme regresyon eğimi (son 128 gözlem) olağandışı yüksek | tek yönlü z ≥ 2; ısınma ~60 işlem (`kyle_lambda_detector.hpp:122-133`, `:170-200`) | İ, F | `z/3` (en az 0,667) | hep nötr | soğuma 15 sn | `lambda, z_score, n_obs`; bağlam ≥ 0,65 (`:224-230`) |
| VPIN_DETECTOR | Eşit hacimli TL kovalarında |alış−satış|/hacim ortalaması (son 50 kova) olağandışı yüksek | z ≥ 2; kova max(100 bin TL, OGH·0,0005) ya da 500 bin TL (`vpin_detector.hpp:133-139`, `:174-194`) | İ, OGH | `z/3` | hep nötr | soğuma 15 sn | `vpin, z_score, buckets, bucket_volume_tl`; bağlam ≥ 0,65 (`:218-225`) |
| AGGRESSIVENESS_DETECTOR | 60 sn'lik pencerede (alış−satış)/(alış+satış) TL dengesizliğinin z'si | |z| ≥ 2,5; pencere ≥ 100 bin TL; ısınma 30 pencere (`aggressiveness_detector.hpp:24-27`, `:114-139`) | İ | `|z|/5` | **z'nin işareti** (dengesizliğin değil) (`:138`) | soğuma 15 sn; < 0,55 → 0 | `imbalance, z_score, mean, std_dev, buy_vol, sell_vol`; bağlam ≥ 0,65 (`:168-177`) |
| BLOCK_TRADE_DETECTOR | (a) TradeEx (toptan) işlem OGH'nin ≥ %5'i; (b) sürekli işlemde tutar z ≥ 3 ve OGH'nin ≥ %2'si | `block_trade_detector.hpp:171-214`; varsayılan OGH 10 M TL (`:67-69`) | İ (+TradeEx konusu), İK (bağlamda), OGH | (a) `min(1,5·oran)`, (b) `z/5`; oran > %5 ise ×1,3 | aktif taraf | ortak soğuma 15 sn; < 0,55 → 0; sönüm 0,08 | `source, block_volume, daily_avg_volume, volume_ratio, size_z, buyer, seller, is_trade_ex, block_trade_count` (`:155-165`) — **kurum kodları çıktıda** |
| ENTROPY_RATE_DETECTOR | Son 500 işlemin adet dağılımının (log2 kovalar) Shannon entropisi, her 20 işlemde | |z| > 2 (iki yönlü); ısınma ~100 işlem (`entropy_rate_detector.hpp:31`, `:57-84`) | İ | `|z|/5` | yalnız z < −2 iken alış/satış sayısı 2 kat baskınsa (`:86-87`, `:173-177`) | soğuma yok, en az puan yok | **yok** |

Not: OGH = sembol profilinden ortalama günlük hacim; ClickHouse'taki son 20 günden kurulur
(`src/services/profile_bootstrap.hpp:17-24`; `main.cpp:1114-1132`).

### 2.3 Akış / PGC (kategori `flow`)

PGC alanları `pgc5, pgc8, pgc10` (`src/core/types.hpp:176-182`). Anlamları (hangi kurum
kümesi, birikimli mi) koddan anlaşılmıyor — **bilinmiyor; Matriks veri ekibine / Quantex
geliştiricisine sorulur.** Motor PGC verisi 5 dk gelmezse üç PGC dedektörünü kapatır,
açılışta hiç gelmezse "AKDE lisansı gerekebilir" uyarısı yazar (`main.cpp:2029-2066`); yani
PGC lisanslı kurum dağılımı verisidir.

| Tür | Kural | Eşik / pencere | Girdi | Puan · güven | Yön | Bastırma | Bağlam |
|---|---|---|---|---|---|---|---|
| PGC_ACCEL_DETECTOR | pgc10'un ardışık iki olay arası hızının ivmesinin z'si | |z| ≥ 3; ısınma 20; hız < 1 M ise puan yarıya (`pgc_acceleration_detector.hpp:25-30`, `:81-106`) | P | `min(|z|/6,1)` | z işareti | soğuma 15 sn; < 0,55 → 0 | `acceleration, velocity, z_score, dt_us, pgc10` (`:141-147`) |
| PGC_DIVERGENCE_DETECTOR | Son 5 dk'da fiyat değişimi (≥ %0,3) ile pgc10 değişimi (≥ 5·10⁷) zıt işaretli | sabit mutlak eşikler (`pgc_divergence_detector.hpp:24-28`, `:144-157`) | P, F | fiyat ve pgc büyüklüklerinin ortalaması (`:159-164`) | **pgc10 değişiminin işareti** (`:167-171`) | soğuma 15 sn; < 0,55 → 0 | `price_change_pct, pgc10_change, window_us, direction` (`:209-214`) |
| PGC_CONCENTRATION_DETECTOR | 60 sn'lik pgc10 hızının z'si + kurum yoğunlaşması (HHI, REST'ten) + pgc5/pgc10 farkı | |z| ≥ 2,5 ya da HHI çarpanı ≥ 0,5 (`pgc_concentration_detector.hpp:33-40`, `:147-156`) | P, R (HHI) | `0,5·z + 0,3·HHI + 0,2·fark` | hep nötr (`:179`) | soğuma 15 sn; < 0,55 → 0 | `pgc_velocity, pgc_velocity_zscore, hhi_factor, divergence_score, pgc5, pgc10, elevated_z, high_hhi` (`:185-194`) |

### 2.4 Mikro yapı (kategori `microstructure`)

| Tür | Kural | Eşik | Girdi | Puan | Yön | Bastırma | Bağlam |
|---|---|---|---|---|---|---|---|
| HAWKES_INTENSITY_DETECTOR | Ekle/sil/işlem olaylarından 3 tipli üstel Hawkes yoğunluğu; toplam yoğunluk/taban oranının z'si | z ≥ 1,5 (yalnız yukarı); ısınma 40+40 (`hawkes_intensity_detector.hpp:29-35`, `:154-203`) | Dİ, İ | `|z|/3` | nötr | yok | `lambda_insert, lambda_cancel, lambda_trade, excess_ratio, z_score, event_count`; bağlam ≥ 0,65 |
| INTRADAY_VOL_SURFACE_DETECTOR | ~1 dk'lık kovalarda tik getirilerinin sapması; z | |z| > 2,5; ısınma 10 kova (`intraday_vol_surface_detector.hpp:27-30`, `:147-188`) | F | `|z|/6` | sıçramada nötr; **düşüşte** yukarı/aşağı tik baskınlığına göre | yok | **yok** |
| QUOTE_ARRIVAL_RATE_DETECTOR | Son 1 sn'deki derinlik olayı sayısının z'si (sıçrama ya da kuruma) | |z| > 2,5 (`quote_arrival_rate_detector.hpp:26-28`, `:57-76`) | Dİ | `|z|/6` | nötr | yok | **yok** |
| TICK_TO_TRADE_DETECTOR | Son 30 sn'de derinlik olayı / işlem oranının z'si | z > 2,5 ya da z < −2 (`tick_to_trade_detector.hpp:26-29`, `:114-135`) | Dİ, İ | `z/6` ya da `|z|/5` | nötr | sn'de en çok bir hesap | **yok** |

### 2.5 Çapraz varlık (kategori `cross_asset`)

Üçü de sektör eşlerinin paylaşılan durumunu (`src/state/cross_asset_state.hpp`) okur; bu
durum her sembolün fiyat iş parçacığından ve **duvar saatiyle 1 sn'de bir çalışan** bir
işçiden yazılır (`main.cpp:1991-2005`).

| Tür | Kural | Eşik | Puan | Yön | Bastırma | Bağlam |
|---|---|---|---|---|---|---|
| LEAD_LAG_SHIFT_DETECTOR | Sembol bir çiftte izleyici ise lider ile kendi 60 tiklik getirisi arasındaki farkın z'si | |z| > 2; ısınma 10 (`lead_lag_shift_detector.hpp:54-74`) | `min(1,(|z|−2)/2 + 0,5)` | liderin getirisinin işareti (`:77-79`) | tik başına ×0,95 | `leader, gap, gap_z_score` |
| BASIS_VOL_OF_VOL_DETECTOR | Kendi tik getiri sapması − sektör ortalama "oynaklık" alanı; adı vadeli baz içermiyor, iki taraf farklı birimde (`basis_vol_of_vol_detector.hpp:52-61`; `cross_asset_state.hpp:245-256`) | z > 2,5 | `min(1,(z−2,5)/2,5 + 0,5)` | nötr | ×0,95 | `symbol_vol, sector_avg_vol, vol_diff, z_score` |
| SECTOR_ENTROPY_DETECTOR | Sektör üyelerinin 5 dk getirilerinin entropisi; düşük = sürü, yüksek = dağılma | z < −2 (sürü) / z > 2,5 (dağılma); ≥ 3 üye (`sector_entropy_detector.hpp:56-80`) | `min(1,(|z|−eşik)/2 + 0,5)` | sürüde sektör ortalamasının işareti, dağılmada nötr | yalnız sönüm | `sector, entropy, z_score, sector_volume, sector_symbol_count, anomaly_type` |

Lider-izleyici "korelasyonu" iki noktadan (1 dk ve 5 dk getiri) hesaplanır
(`cross_asset_state.hpp:320-330`); etkin sembol süzgeci duvar saatini olay zamanıyla
karşılaştırır (`:145`).

### 2.6 Rejim (REGIME_DETECTOR, kategori `composite`, ağırlık 0)

Rejim bir anomali türü değil, **bağlamdır**; birleşik puana doğrudan katılmaz
(`regime_detector.hpp:31`), ağırlıkları ve eşiği çarpar.

- **Eksenler** (`src/core/regime_types.hpp:18-39`): seans (duvar saatiyle, §3), oynaklık
  (LOW_VOL / NORMAL / HIGH_VOL / TRENDING / MEAN_REVERTING), likidite (DEEP / NORMAL / THIN),
  toksisite (QUIET / NORMAL / ELEVATED / TOXIC). Anahtar `seans|oyn|lik|toks`.
- **Karar:** oynaklık son 60 tik getirisinin sapmasının z'si ve 1 gecikmeli öz ilinti
  (z > 2 yüksek, < −1,5 düşük, ilinti > 0,35 trend, < −0,35 ortalamaya dönüş);
  likidite fark (bp) z, defter derinliği z ve kotasyon varış z'sinden; toksisite VPIN,
  Kyle-λ ve Hawkes z'lerinden (`src/engine/regime_context.hpp:96-266`, `:554-671`). Her eksende
  değişim için en az 30 sn olay zamanı bekleme (`:36`, `:587-589`); histerezis bandı yok.
- **Etkisi:** kategori ağırlık çarpanı [0,20, 1,50] ve eşik çarpanı [0,5, 1,5]
  (`regime_context.hpp:328-355`, tablolar `:446-551`; `composite_engine.hpp:395-398`, `:663-664`).
- **Yayın:** eksen geçişinde `quantex/anomaly/regime/<sembol>` (`src/dispatch/regime_publisher.hpp:47-104`).
  Likidite ve toksisite için `from` = `to` ve güven sabit 1,0 (`:69-74`); gerçek geçiş
  bilgisi yalnız oynaklık ekseninde.
- **HMM:** 3 durumlu (QUIET/NORMAL/TOXIC), 7 gözlemli, köşegen Gauss salınımlı bir gizli
  Markov modeli var; yalnız ileri süzgeç, çevrimiçi öğrenme yok
  (`src/engine/regime_latent_model.hpp:17-29`, `src/engine/regime_latent_filter.hpp:33-100`).
  **Varsayılan kapalı** (`config.hpp:138-139`) ve açılsa da **gölge**: yalnız ClickHouse
  özellik JSON'una olasılık ekler; puan, eşik ve yayına dokunmaz (`regime_latent_filter.hpp:138-146`).
  Parametreleri çevrim dışı bir Python betiği üretir (`scripts/train_regime_hmm.py`, sabit
  tohum 42). Parametre dosyası (`config/hmm_params.fixture.json`) yapısı: sürüm, model kimliği,
  3 durum adı, 7 gözlem adı, başlangıç olasılıkları (3), geçiş matrisi (3×3), ortalamalar
  (3×7), köşegen varyanslar (3×7), eğitim zamanı ve penceresi. Sayılar aktarılmadı.

## 3. Yük biçimi, konular, geçmiş kaydı, öğrenme

### 3.1 MQTT (motorun yayınladığı ve dinlediği)

| Konu | Yön | İçerik | Kanıt |
|---|---|---|---|
| `quantex/anomaly/data/<sembol>` | yayın | Birleşik olay (JSON): `type="anomaly", event_id, symbol, timestamp (µs), composite_score, confidence, direction, severity, contributors[{name, category, score, weight, weighted_contribution, explanation, direction}], context{price, bid, ask, volume, imbalance, spread, pgc5, pgc8, pgc10, volatility_regime, regime_key, liquidity_regime, toxicity_regime, detectors{<ad>: {...}}, signal_agreement{...}, escalation_count?}, engine_id, partition_id` | `src/dispatch/signal_dispatcher.hpp:65-80`, `:274-304`; `composite_engine.hpp:586-626` |
| aynı konu, ikinci biçim | yayın | Yapay zekâ için: `event_type="anomaly_detected", version, data{…aynı alanlar…, market_context}, metadata{engine_id, partition_id, processing_latency_ms}`. Yalnız puan ≥ 0,70, şiddet ≥ medium, açılış müzayedesi dışı, sembol başına 300 sn'de bir | `signal_dispatcher.hpp:116-145`, `:162-219`; `config.hpp:150-152` |
| `quantex/anomaly/outcome/<sembol>` | yayın | `event_type="anomaly_outcome", event_id, symbol, detector_name, outcome_confirmed, predicted_direction, trigger_price, final_price, price_change_pct, eval_ts, outcome_source, regime_axis, regime_confirm_rule, proxy0, proxy1, outcome_features?` | `src/main.cpp:837-868` |
| `quantex/anomaly/regime/<sembol>` | yayın | `type="regime_transition", event_id, symbol, timestamp, session_regime, primary_axis, axes{…}, features{13 değer}, score, direction="NEUTRAL", explanation, regime_key, engine_id, partition_id` | `regime_publisher.hpp:47-94` |
| `quantex/anomaly/health/<motor>` | yayın | sağlık | `main.cpp:2574` |
| `quantex/indicator/data/#` | dinleme | gösterge değerleri (§1) | `main.cpp:1812` |
| `quantex/anomaly/feedback/#`, `quantex/anomaly/priority/#` | dinleme | yapay zekâ servisinden "yanlış pozitif" (eşik artışı) ve "öncelik" (ağırlık çarpanı) | `src/feedback/ai_feedback_consumer.hpp:23-26`, `main.cpp:1846-1847` |
| `quantex/anomaly/lesson/#` (`…/parameters`) | dinleme | öğrenilmiş ağırlık, eşik, çift çarpanı | `main.cpp:1852-1895` |

Olayın zaman damgası mikrosaniyedir (`types.hpp:126`); MQTT'de açık bir sürüm alanı yok
(yapay zekâ biçimindeki `version="1.0"` dışında).

### 3.2 NATS (bağlayıcı `services/quantex-stream/anomaly-adapter`)

- MQTT'de `quantex/anomaly/{data,outcome,regime,health}/#` abonesi (`anomaly-adapter/src/mqtt/mqtt.subscriber.ts:9-14`),
  NATS konusu `anomaly.<borsa>.<sembol>.data|outcome|regime` ve `anomaly.health.<motor>`;
  borsa varsayılanı `BIST` (`anomaly-adapter/src/nats/nats.publisher.ts:73-99`; `src/config/env.ts:47`).
- Yük, ortak zarf (`Envelope`) protobuf'u: `anomaly_event = 30`, `anomaly_outcome = 31`,
  `regime_event = 46` (`services/quantex-stream/shared/proto/envelope.proto:52-53`, `:78`).
  `AnomalyEvent` alanları: kimlik, sembol, zaman, birleşik puan, güven, yön, şiddet,
  katkıcılar (JSON bayt), bağlam (JSON bayt), motor, bölüm (`shared/proto/anomaly.proto:15-26`).
  `AnomalyOutcome` 9 alan (`:30-39`), `RegimeEvent` 14 alan (`:44-58`).
- Bağlayıcı dönüşümleri (`anomaly-adapter/src/transform/anomaly.transform.ts`):
  - Yapay zekâ biçimli olaylar **atılır** (`:42-43`); NATS'a yalnız standart biçim gider.
  - Katkıcılar ve bağlam JSON olarak bayta çevrilir; kanıt sözlüğü korunur (`:63-64`).
  - Sonuç iletisinden `outcome_source`, `regime_axis`, `regime_confirm_rule`, `proxy0/1`,
    `outcome_features` **düşer** (`:85-95`).
  - Birim tutarsızlığı: proto `AnomalyEvent.timestamp` ve `AnomalyOutcome.eval_ts` için
    "ms" der (`anomaly.proto:18`, `:39`), motor µs gönderir, bağlayıcı çevirmeden geçirir; zarfın
    `tsEvent`'i de anomali olayında µs değerle dolar (`anomaly.transform.ts:52`). Rejim olayında
    ise /1000 ile ms'ye çevrilir (`:129-131`).

### 3.3 Geçmiş kaydı (ClickHouse) ve sonuç izleme

Yazma yalnız `CLICKHOUSE_ENABLED` açıkken (varsayılan kapalı, `config.hpp:114`); dağıtımdaki
değeri bilinmiyor (dağıtım env dosyası depoda yok — Quantex işletmesine sorulur).

| Tablo | Alanlar (özet) | Saklama | Kanıt |
|---|---|---|---|
| `anomaly_events` | olay kimliği, sembol, zaman, birleşik puan, güven, yön, şiddet, katkıcı adları/puanları/ağırlıkları (dizi), bağlam (JSON metni), motor, bölüm; SAL alanları (`sa_*`, kademe) | 90 gün | `migrations/001…sql` (tablo 1), `005…sql`, `006…sql` |
| `anomaly_scores` | sembol, dedektör, zaman, puan, güven, yön, açıklama, bağlam | 30 gün | `001…sql` |
| `anomaly_outcomes` | olay kimliği, sembol, dedektör, beklenen yön, tetik fiyatı, son fiyat, % değişim, tetik ve değerlendirme zamanı, doğrulandı, yapay zekâ geri bildirimi (4 alan), sonuç kaynağı; rejim kanıtı (`regime_axis, regime_confirm_rule, proxy0, proxy1, outcome_features_json`) | 180 gün | `001…sql`, `004…sql` |
| `audit_log` | her değerlendirme kararı (`FIRE`, `SUPPRESS_THRESHOLD`, `SUPPRESS_COOLDOWN`), puan, eşik, kalıcılık, şiddet, etkin dedektörler | 7 gün | `001…sql`; `composite_engine.hpp:853-879` |
| `anomaly_regime_transition`, `anomaly_regime_snapshot` | rejim geçişi ve 5 sn'lik anlık görüntüler, özellikler, çarpan dökümü | 90 / 30 gün | `001…sql`, `002…sql`, `003…sql` |
| diğer | `symbol_trade_profile`, `detector_accuracy_profile`, `settlement_cache`, `news_impact`; öğrenme tabloları `anomaly_parameter_state`, `anomaly_parameter_history`, `anomaly_lesson_runs` | 180–365 gün | `001…sql`; `lesson-learner/src/services/migrations.ts:13-57` |

**Sonuç kuralı** (`src/engine/outcome_tracker.hpp`):

- Kayıt: birleşik puan ≥ 0,6 olan olayın **her katkıcısı** için ayrı bekleyen satır; yön =
  o dedektörün kendi yönü; tetik fiyatı = bağlamdaki son fiyat (`main.cpp:804-829`).
- Pencere saniye cinsinden, kategoriye göre: derinlik 120, işlem 300, akış 900, çapraz 1800,
  mikro 180, rejim 600, birleşik 300; OFI 30 (`types.hpp:57-80`).
- Değerlendirme: pencere dolduktan **sonra gelen ilk fiyat** ile (`outcome_tracker.hpp:136-160`).
  Yönlü: hareket ≥ dinamik eşik (sembol oynaklığı ×0,4, %0,075–0,45 arasına kıstırılır;
  `:424-425`) yönünde; **nötr** iddia hareket eşiğin altında kalırsa doğru sayılır (`:270-277`).
  Yapısal (SPOOF, ICEBERG, BLOCK_TRADE, PGC_CONCENTRATION, VELOCITY_FIELD, CROSS_DEPTH_CORR):
  herhangi bir yönde eşik üstü hareket (`types.hpp:27-55`; `outcome_tracker.hpp:252-268`).
  Rejim türü: fiyat değil, özellik anlık görüntüsü (kayıt ve pencere sonu) ile
  (`src/engine/regime_outcome_policy.hpp:61-164`).
- Bellekte en çok 5000 bekleyen; taşanlar ve 30 dk'dan eskiler "doğrulanmadı" diye kapanır
  (`outcome_tracker.hpp:121-127`, `:214-231`).
- Okuma API'si (iç anahtarla korunur, `api/src/middleware/internal-auth.ts:9-16`):
  `/api/v1/outcomes/stats`, `/scorecard`, `/api/v1/outcomes`, `/api/v1/events/:id/outcome`
  (`api/src/routes/outcomes.ts:64`, `:140`, `:341`, `:427`).

### 3.4 Öğrenme (`lesson-learner`) ve yapay zekâ geri bildirimi

- **Ne öğreniyor:** `anomaly_outcomes` üzerinden dedektör başına isabet (son 1 gün saatlik,
  7 gün günlük; en az 30 sonuç) (`lesson-learner/src/analysis/detector-performance.ts:30-48`;
  `config/env.ts:27-31`). Beta öncül ile sonsal isabet / genel isabet oranında **ağırlığı**
  çarpar (en çok ±%25, [0,01, 0,40] içinde); isabet < %20 ise `fire_threshold` +0,05, > %60 ise
  −0,03 (en çok ±%15) (`analysis/bayesian-optimizer.ts:60-168`). Dedektör çifti başına güçlendirme
  çarpanı da önerir (`analysis/pair-performance.ts`).
- **Eşikleri değiştiriyor mu:** öneri her zaman tabloya yazılır; MQTT ile motora uygulanması
  `AUTO_APPLY` / `PAIR_BOOST_AUTO_APPLY` açıksa olur (ikisi de varsayılan kapalı,
  `config/env.ts:34-36`; `analysis/lesson-runner.ts:66-93`), ya da API'den elle onayla
  (`api/src/routes/lessons.ts:242`). Motor bunları çalışırken uygular ve açılışta tablodan okur
  (`main.cpp:1852-1895`; `src/services/lesson_bootstrap.hpp:62-66`). Ağırlık birleşik puanı
  doğrudan değiştirir; `fire_threshold` ise çoğu dedektörde yalnız bağlamın dolup dolmadığını
  etkiler (§2 ortak notlar).
- Öğrenicinin varsayılan tablosu fabrika ile senkron değil (OB_GRADIENT 0,10 ↔ dosyada 0,15;
  PGC_DIVERGENCE 0,05 ↔ 0,10; OFI, KYLE_LAMBDA, VPIN, HAWKES tabloda yok)
  (`bayesian-optimizer.ts:12-35`).
- **Deterministik mi:** hesap kuralı deterministik, ama çalışma zamanı duvar saatiyle (cron,
  `now() - INTERVAL`) ve tablonun o anki içeriğiyle belirlenir; çalışma kimliği rastgele
  (`lesson-runner.ts:23`; `detector-performance.ts:46`; `scheduler/cron.ts:37-43`).
- **Yapay zekâ geri bildirimi** (varsayılan açık, `config.hpp:150`): ayrı `quantex-ai-insights`
  servisi, kendi içgörü isabeti düşük kalınca sembol için "yanlış pozitif" gönderir; motor
  eşiği 120 dk boyunca en çok +0,15 yükseltir (`services/quantex-ai-insights/src/engine/accuracy/FeedbackLoop.ts:159-182`;
  `adaptive_threshold.hpp:27-51`). Öncelik iletisi dedektör ağırlığını ×2'ye kadar çarpar
  (`priority_boost.hpp:28-36`). Bu servis bir LLM ağ geçidi istemcisi içerir
  (`services/quantex-ai-insights/src/engine/llm/GatewayClient.ts`). Varsayım: isabet kayıtları
  LLM'in ürettiği içgörülerin değerlendirmesidir; bu yüzden motorun eşiği dolaylı olarak LLM
  çıktısına bağlıdır. Doğrulanmadı — `quantex-ai-insights` sahibine sorulur.

## 4. Determinizm (K3) ve kanıta açılabilirlik (K34)

### 4.1 Aynı girdi aynı çıktıyı verir mi? Hayır.

| Kaynak | Etki | Kanıt |
|---|---|---|
| Olay kimliği = duvar saati ms + `random_device` tohumlu rastgele sayı | kimlik her koşuda farklı; Qapsule'ün belirleyici olgu kimliği (`fx.…`) ile eşleşmez | `signal_dispatcher.hpp:147-160`; `regime_publisher.hpp:107-121` |
| Matriks gün içi zaman damgaları **bugünün** gece yarısına eklenir (duvar saati) | geçmiş bir günü yeniden oynatınca olaylar bugünün tarihini alır | `src/utils/timestamp_utils.hpp:29-62`; `ingestion/*_ingester.hpp` (ör. `trade_ingester.hpp:112`) |
| Seans durumu (açılış/kapanış müzayedesi) duvar saatinden (+ Matriks açık/kapalı iletisi) | eşik (0,80/0,72/0,70), ağırlık ve SPOOF/ICEBERG/AGENT_SHIFT kuralları buna bağlı | `src/services/market_clock.hpp:249-327`; `composite_engine.hpp:395`, `:659-662` |
| Akış başına ayrı iş parçacığı (derinlik, işlem, piyasa, haber, istemci) | aynı sembolde akışlar arası sıra koşudan koşuya değişir; SPOOF/ICEBERG gibi işlem–derinlik sırasına duyarlı kurallar etkilenir | `src/services/mqtt_manager.hpp:97-101` |
| Çapraz varlık işçisi duvar saatiyle 1 sn/5 sn; eş durumu başka sembollerin iş parçacıklarından | CROSS_DEPTH_CORR, LEAD_LAG, BASIS_VOL, SECTOR_ENTROPY zamanlamaya bağlı; çift seçimi `unordered_map` sırasına bağlı | `main.cpp:1991-2005`; `cross_asset_state.hpp:144-153` |
| Tabanlar süreç başından birikimli, seans sıfırlaması yok | çıktı sürecin ne zaman başlatıldığına bağlı | `base_anomaly_detector.hpp:69` (çağrısız); `types.hpp:328-356` |
| Dış veri: takas geçmişi (REST, duvar saati tarih aralığı), HHI (REST, ayrı iş parçacığı), sembol profili (ClickHouse), haber | aynı tik akışı farklı dış durumla farklı sonuç verir | `src/services/settlement_service.hpp:337-350`; `src/state/institutional_state.hpp:53-88`; `profile_bootstrap.hpp:17-24` |
| Çalışırken değişen parametre: öğrenilmiş ağırlık/eşik/çift çarpanı, yapay zekâ eşik artışı ve ağırlık çarpanı (TTL `steady_clock`) | aynı girdi gün içinde farklı eşikle değerlendirilir | `composite_engine.hpp:117-187`; `adaptive_threshold.hpp:35-70`; `priority_boost.hpp:38-60` |
| Dil modeli | motor kodunda `llm/openai/anthropic/gpt` yok (arama boş); dolaylı etki §3.4 | `grep` (src, api/src, lesson-learner/src) |

Rastgele sayı yalnız kimlikte kullanılır; puan hesabında rastgelelik yok.

### 4.2 Kanıta açılabilir mi?

Kısmen. Olay, katkıcı başına puan, ağırlık, yön ve açıklama ile birlikte
`context.detectors.<ad>` altında dedektörün kendi bağlamını taşır (`composite_engine.hpp:291-302`,
`:609-623`). Ama:

- **Eşik değerleri hiçbir bağlamda yok.** Gözlenen değer ve z çoğunlukla var; taban
  ortalama/sapma yalnız birkaçında (VELOCITY_FIELD, TRADE_CLUSTER, AGGRESSIVENESS).
- Bağlamı **hiç olmayanlar:** BAAM, ICEBERG, ENTROPY_RATE, INTRADAY_VOL_SURFACE,
  QUOTE_ARRIVAL_RATE, TICK_TO_TRADE. Bunlar yalnız etiket + açıklama cümlesidir.
- Birçoğunda bağlam yalnız puan > 0,65 iken dolar, katkı ise 0,15'ten başlar (§2).
- Bağlam 4096 baytı aşarsa 512 baytı aşan girdiler `_truncated` olur (`composite_engine.hpp:49`, `:609-623`).
- Pencere bilgisi az sayıda dedektörde (OFI `cum_window`, PGC_DIVERGENCE `window_us`).
- Tabanlar birikimli olduğundan z'yi bağımsız olarak yeniden üretmek için sürecin başından
  beri bütün akış gerekir; tek bir olaydan doğrulanamaz.

## 5. Qapsule'e eşleme

Qapsule tarafı: altı anlam türü ve beş kanıt ailesi (`docs/CAPSULE.md` §4; K30), lisans sınıfı
ve sunucu olgusu (§7, §9.7; K33, K38), mühür `qp_capsule::trust::Muhur`
(`crates/qp-capsule/src/trust.rs:1-36`), yedi dedektör (`crates/qp-facts/src/*.rs`).

**Lisans dayanağı (Quantex tarafı):** kurum bazlı akış AKD/AKDE (ve vadeli için VAKD)
lisansı ister (`services/quantex-api/src/routes/flow.routes.ts:23-26`, `:162`); alıcı/satıcılı
işlem verisi AKDE ister (`services/quantex-api/FRONTEND_INTEGRATION.md:521`); ayrı bir
"derinlik lisansı" anılır (`flow.routes.ts:366`). Varsayım: derinlik ve emir bazlı defter
verisi de lisanslıdır. Varsayım: alıcı/satıcısız işlem tiki gerçek zamanlı veri yetkisine
tabidir; ama yansı tik almadığı (K48) için tikten türeyen her olgu pratikte **sunucu olgusudur**.

**Kanıt ailesi açığı:** Qapsule'ün beş ailesinde (fiyat, hacim, akış, türev, haber/temel;
`crates/qp-schema/src/capsule.rs:79-87`) **emir defteri / derinlik** yok; `Flow` "takas, kurumsal
akış" diye tanımlı. Aşağıda derinlik kökenli türler geçici olarak "akış (derinlik)" yazıldı.
Bu bir öneridir; yeni aile gerekiyorsa önce `docs/CONCEPTS.md` sözlüğüne eklenmeli.

**Mühür için ortak gereksinim:** `Muhur` yalnız tek bir ham değeri (`net`) okur, gelir gelmez
işaret + ikili büyüklük basamağına (`floor(log2|x|)`) indirger, ileti yalnız "olağan
düzeyinin belirgin biçimde üzerinde" + yön der; denetim, yetkili bir denetçinin aynı lisanslı
veriden bütün iletileri **yeniden türetmesine** dayanır (`trust.rs:6-28`). Anomali motorunun
çıktısını mühürlemek için bu yüzden (a) bar başına **ham, işaretli bir büyüklük** (ör. bar
içi Δpgc10, bar içi net kurum akışı, bar içi OFI toplamı) ve (b) deterministik yeniden üretim
gerekir. Motor (a)'yı bar başına vermez (yalnız ateşleme anındaki bağlamda), (b)'yi hiç
sağlamaz (§4.1). Motorun z ve puanı zaten normalize edilmiş, işaret+eşik biçiminde bir
çıktıdır; Varsayım: sızıntı sınıfı mühürle benzer (ateşleme anı + yön) ama denetlenebilirlik
olmadan "sunucu tarafından doğrulandı" iddiası (K38) taşınamaz.

### 5.1 Tür → Qapsule

Sütunlar: anlam türü · yön · kanıt ailesi · lisans · Qapsule dedektörüyle ilişki ·
sunucu olgusu olarak taşınabilirlik (K34 açılabilirlik; mühür için gereken).

| Motor türü | Anlam türü (K30) | Aile | Lisans | Qapsule ilişkisi | Taşınabilirlik |
|---|---|---|---|---|---|
| BAAM | akış (yönlü kuyruk baskısı) | akış (derinlik) | lisanslı (varsayım) | yok; tamamlar | bağlam yok → açılamaz; mühür için bar başına en iyi kademe dengesizliği gerekir |
| OFI | akış | akış (derinlik) | lisanslı (varsayım) | yok; tamamlar | gözlenen değer var, eşik yok; ham `ofi_cum` bar başına mühüre uygun aday |
| SPOOF | — (altı türde yok: manipülasyon/yapısal uyarı) | akış (derinlik) | lisanslı | yok | yönü "karşı taraf" yorumu; Qapsule dili "gözlem" (§5) ile riskli; Varsayım: kapsül değil, ayrı uyarı |
| ICEBERG | akış (yönsüz, gizli büyük emir) | akış (derinlik) | lisanslı | yok | bağlam yok; taşınamaz |
| OB_GRADIENT | akış | akış (derinlik) | lisanslı | yok | kısmen (≥ 0,65); `delta_s` mühüre aday |
| VELOCITY_FIELD | aşırılık (yönsüz) | akış (derinlik) | lisanslı | yok | kısmen; yönsüz |
| CROSS_DEPTH_CORR | akış (sektöre göre sapma) | akış (derinlik) | lisanslı | yok | kısmen; eş durumu deterministik değil |
| AGENT_SHIFT | akış (kurum yön değiştirdi; dönüş çağrışımlı) | akış | lisanslı (AKDE) | **flow.institutional'ı tamamlar** (o: bar net > 2× son 5 ortalama; bu: tek kurumun işaret dönüşü) | kurum adı ve net rakamları bağlamda → yetkisize gidemez; mühür için kurum başına bar net değeri |
| INFORMED_TRADE | akış | akış (+haber) | lisanslı (AKDE) | yok | kısmen; haber eşleşmesi duvar saatine bağlı |
| TRADE_CLUSTER | akış (işlem patlaması) | hacim | tik (varsayım: açık ama yansıda yok) | volume_surge'e yakın; o bar hacmi, bu 10 sn işlem sayısı | bağlam iyi (gözlem + taban) |
| KYLE_LAMBDA | — (rejim: fiyat etkisi) | hacim/fiyat | tik | yok | rejim girdisi; kapsül değil |
| VPIN | — (rejim: toksisite) | hacim | tik | yok | rejim girdisi |
| AGGRESSIVENESS | akış | hacim | tik | volume_surge ile kısmen; yön kuralı farklı (§5.3) | bağlam ≥ 0,65 |
| BLOCK_TRADE | akış | hacim | tik; TradeEx ve kurum kodları lisanslı | **volume_surge ile örtüşür** (tek işlem vs bar) | bağlamda kurum kodu → yetkisize gidemez |
| ENTROPY_RATE | — (rejim) | hacim | tik | yok | bağlam yok |
| PGC_ACCEL | akış | akış | lisanslı (AKD/AKDE) | **flow.institutional ile örtüşür** (ikisi de lisanslı kurum akışı; o bar net büyüklüğü, bu pgc10 ivmesi z'si) | gözlenen değer var; mühüre en uygun aday (ham pgc10 değişimi işaretli büyüklük) |
| PGC_DIVERGENCE | dönüş (fiyat–para akışı uyumsuzluğu) | akış + fiyat | lisanslı | rsi_divergence'ı **tamamlar** (o fiyat–RSI, bu fiyat–akış); yön akışın işareti, fiyatın tersi | pencere var, eşik yok; Qapsule'de iki aileli teyit gibi okunabilir |
| PGC_CONCENTRATION | akış (yönsüz yoğunlaşma) | akış | lisanslı | flow.institutional'ı tamamlar | yönsüz |
| HAWKES_INTENSITY | — (rejim) | akış (derinlik) | lisanslı (varsayım) | yok | rejim girdisi |
| INTRADAY_VOL_SURFACE | sıçrama: aşırılık (yönsüz); **düşüş: sıkışma** | fiyat | tik | düşüş kolu squeeze.bandwidth'ı **tamamlar** (o bar Bollinger bant genişliği 120 bar en düşüğü, bu 1 dk tik sapması) | bağlam yok |
| QUOTE_ARRIVAL_RATE | — (rejim/likidite) | akış (derinlik) | lisanslı (varsayım) | yok | bağlam yok |
| TICK_TO_TRADE | — (rejim) | akış (derinlik) + hacim | lisanslı (varsayım) | yok | bağlam yok |
| LEAD_LAG_SHIFT | Varsayım: devam (izleyici liderin yönüne) | fiyat | tik | yok | deterministik değil; 2 noktalı korelasyon |
| BASIS_VOL_OF_VOL | aşırılık (yönsüz) | fiyat | tik | yok | birim tutarsız; taşınması önerilmez |
| SECTOR_ENTROPY | sürü: devam; dağılma: — | fiyat | tik | yok | deterministik değil |
| REGIME (bağlam) | — (olgu değil) | fiyat/derinlik/hacim | karma | yok | kapsül bağlamı (önem/eşik) olabilir, olgu olamaz |
| **Birleşik olay** | yönlü ise "akış ağırlıklı karma", nötr ise — | karışık (çok aile) | karma | Qapsule'ün "teyit = farklı aile sayısı" kuralının yerine geçemez: ağırlıklı ortalama aileleri karıştırır | olgu değil, **bulgu**/özet; Qapsule kendi hikâyesini olgulardan kurmalı |

### 5.2 Altı anlam türünün motorda karşılığı

| Anlam türü | Motorda |
|---|---|
| Devam | yok denecek kadar az (SECTOR_ENTROPY sürü kolu, LEAD_LAG varsayımla) |
| Dönüş | PGC_DIVERGENCE; AGENT_SHIFT çağrışımla |
| Kırılım | **yok** (fiyat seviyesi kırılımı aranmaz; yalnız `bb_depth_breakout` gösterge çarpanı) |
| Sıkışma | INTRADAY_VOL_SURFACE düşüş kolu; rejim LOW_VOL |
| Aşırılık | VELOCITY_FIELD, INTRADAY_VOL sıçrama kolu, BASIS_VOL_OF_VOL (yönsüz); RSI aşırılığı yalnız ölü gösterge kuralında |
| Akış | ağırlık burada: derinlik, işlem, PGC türlerinin çoğu |

### 5.3 Qapsule'ün yedi dedektörü → motor (ters yön)

| Qapsule dedektörü | Tür · aile · lisans (Qapsule) | Motorda karşılık | İlişki |
|---|---|---|---|
| `breakout.range` (`crates/qp-facts/src/breakout_range.rs:1-11`) | kırılım · fiyat · açık | yok | Qapsule tamamlar |
| `extreme.rsi` (`extreme_rsi.rs:1-14`) | aşırılık · fiyat · açık | dedektör yok; RSI < 30 / > 70 yalnız birleşik puan çarpanı kuralında ve o kural ad uyuşmazlığı yüzünden tutmuyor (§1) | Qapsule tamamlar |
| `flow.institutional` (`flow_institutional.rs:1-15`) | akış · akış · **lisanslı** (sunucu olgusu) | PGC_ACCEL (örtüşür), PGC_CONCENTRATION, AGENT_SHIFT (tamamlar) | aynı lisanslı veri ailesi; Qapsule bar başına `net` ister — motorun girdisi `pgc5/8/10` ile `net`'in ilişkisi **bilinmiyor** (Matriks veri ekibine) |
| `continuation.ma_stack` (`ma_stack.rs:1-12`) | devam · fiyat · açık | yok (ADX kuralı ölü) | Qapsule tamamlar |
| `reversal.rsi_divergence` (`rsi_divergence.rs:1-14`) | dönüş · fiyat · açık | PGC_DIVERGENCE (benzer anlam, farklı aile) | tamamlayıcı; birlikte iki aileli teyit |
| `squeeze.bandwidth` (`squeeze_bandwidth.rs:1-15`) | sıkışma · fiyat · açık | INTRADAY_VOL_SURFACE düşüş kolu, rejim LOW_VOL | kısmi örtüşme, farklı ölçek (bar vs dakika) |
| `flow.volume_surge` (`volume_surge.rs:1-9`) | akış · hacim · açık | BLOCK_TRADE, TRADE_CLUSTER, AGGRESSIVENESS | örtüşür; **yön çelişebilir**: Qapsule mum gövdesine (kapanış − açılış), BLOCK_TRADE aktif tarafa, AGGRESSIVENESS z'nin işaretine (net satışta bile boğa olabilir) bakar |

## 6. Arkin'in 5. bilgisi — anomali yanı

> "… anomali motoru piyasa anomalilerini yakalar; hepsi NATS üzerinden yayınlar."

**Hüküm: kısmen doğrulandı.**

- Doğru: motor piyasa mikro yapısı anomalilerini (derinlik, işlem, kurum akışı, mikro yapı,
  sektör) yakalar (§2) ve sonuçları NATS'ta `anomaly.BIST.<sembol>.{data,outcome,regime}`
  konularında bulunur (`anomaly-adapter/src/nats/nats.publisher.ts:83-95`).
- Ama motor **NATS'a doğrudan yayınlamaz**; MQTT'ye (`quantex/anomaly/…`) yayınlar, ayrı bir
  bağlayıcı köprüler (`signal_dispatcher.hpp:71`; `anomaly-adapter/src/mqtt/mqtt.subscriber.ts:9-14`).
  Köprüde yapay zekâ biçimli olaylar atılır, sonuç iletisinin rejim kanıtı düşer, zaman
  birimi tutarsızdır (§3.2).
- Yayınlanan şey tür başına anomali değil **sembol başına birleşik olaydır**; türler
  katkıcı listesinde durur (§1).
- Motor da gösterge servisini MQTT aboneliğiyle (`quantex/indicator/data/#`) dinler, NATS'tan
  değil (`main.cpp:1812`).

## 7. Qapsule için sonuç (karar değil, gözlem)

- Motorun çıktısı K3/K34/K38 ile olduğu gibi **sunucu olgusu** sayılamaz: deterministik
  değil, eşik ve pencere taşımıyor, kimliği rastgele, denetçi yeniden türetemez.
- Kullanılabilir olan: (1) türlerin **kuralları** (Qapsule dedektörü olarak deterministik,
  bar hizalı yeniden yazılabilir: özellikle PGC_ACCEL / PGC_DIVERGENCE / AGENT_SHIFT /
  BLOCK_TRADE / OFI); (2) `anomaly_outcomes` tablosu, Qapsule karnesine (K42) **dış karşılaştırma**
  olarak (saniye pencereli, yönlü/yapısal ayrımlı; Qapsule'ünki bar ufuklu ve yön anahtarlı);
  (3) rejim, kapsül önemine bağlam olarak.
- Derinlik kökenli olgular için kanıt ailesi ve lisans sınıfı tanımı gerekiyor (§5 başı).

## 8. Açık sorular

| # | Soru | Kime |
|---|---|---|
| 1 | Dağıtımdaki ortam değerleri: `CLICKHOUSE_ENABLED`, `AI_ENRICHMENT_ENABLED`, `COMPOSITE_*`, `REGIME_LATENT_ENABLED`, `WATCH_SYMBOLS`; lesson-learner `AUTO_APPLY` açık mı? | Quantex işletmesi / Arkin |
| 2 | `pgc5`, `pgc8`, `pgc10` ne demek (ilk 5/8/10 kurumun net akışı mı, birikimli mi, birim)? Qapsule `flow.institutional`'ın `net`'i bunlardan nasıl türetilir? | Matriks veri ekibi |
| 3 | Derinlik (`DepthTable`, booklet) ve alıcı/satıcısız işlem tiki hangi lisans sınıfında? Gösterimsiz yorum (K33) serbest mi? | Matriks lisans / hukuk |
| 4 | Derinlik/emir defteri için Qapsule'de yeni kanıt ailesi mi, yoksa "akış" içinde mi? | Arkin (sözlük kararı, CONCEPTS) |
| 5 | Gösterge kurallarındaki ad uyuşmazlığı (`PGC_DIVERGENCE` ↔ `PGC_DIVERGENCE_DETECTOR`) bilinçli mi; üç kural hiç çalışmıyor mu? | Quantex anomali motoru geliştiricisi |
| 6 | `quantex-ai-insights` isabet kayıtları LLM çıktısının değerlendirmesi mi; "yanlış pozitif" eşik artışları canlıda etkin mi? | `quantex-ai-insights` sahibi |
| 7 | Proto'daki "ms" ile motorun µs zaman damgası tutarsızlığı tüketicilerde nasıl ele alınıyor? | quantex-stream geliştiricisi |
| 8 | `anomaly_outcomes` geçmişi (180 gün) Qapsule'e karşılaştırma için okunabilir mi; erişim yolu ve lisans? | Arkin / Quantex işletmesi |
| 9 | Dosya içi ağırlıklar (SPOOF/ICEBERG 0,20, AGENT_SHIFT 0,15, ENTROPY 0,07) ile fabrika kayıtları (0,15/0,15/0,08/0,05) farkı bilinçli mi? Motor dosya içini kullanıyor. | Quantex anomali motoru geliştiricisi |

### Ek: yan bulgular (Qapsule'e etkisi yok, Quantex için not)

Yerleri verildi, değerler yazılmadı: takas servisi, üretim dışında TLS doğrulamasını kapatır
ve bir hata ayıklama günlüğünde belirtecin ilk karakterlerini yazar
(`src/services/settlement_service.hpp:413`, `:426-429`). Haber etkisi servisinin temizliğinde
indeks kayması (`src/services/news_impact_service.hpp:130-143`). Takas günlük yenilemesi yalnız
`_total` anahtarını yazar, kurum bazlı eğilim açılıştan sonra yenilenmez (`settlement_service.hpp:306-313`).
