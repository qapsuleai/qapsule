# Karar Günlüğü — Önerilen (RP4) — Arkin'in onayını bekliyor (2026-11-21)

<!-- dizin: DECISIONS.md; bu dosyadaki satırlar RG1 bölünmesinde aynen taşındı -->

## Önerilen (RP4) — Arkin'in onayını bekliyor (2026-11-21)

RP4 bu satırları **önerir**; ölçümler `rounds/RP4/REPORT.md`, `experiments/M25|M26|M27|S/RESULTS.md`.
Kalibrasyon ve sınama aralıkları önceden yazıldı (`experiments/SPLIT.md`, cc476de); seçim sınama
açılmadan commit edildi (8bb7571).

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö77 | **Gün içi çıkış kuralı `yukdus` (ürün adı `highlow`, kayıt `rp4`): hedef/durdurmaya barın yüksek/düşüğüyle değme, aynı barda ikisi değerse kapanış tarafı, bariyer yönlüde 2,0×, yönsüzde 1,3× r3. Günlük periyotta `r3` (bugünkü bariyer) kalır.** Seçim önceden yazılı puanlama kuralıyla (SPEC §4: beş koşul, sıra 5 dk SENS); aday−aday farkları ayırt edildi. | Kalibrasyon (241 gün, 100 sembol): 5 dk erken kapanma 0,148 → 0,066, SENS 0,212 → 0,099; sınama (24 gün): 0,156 → 0,065 (**hedef tuttu: 0,42×**), SENS 0,198 → 0,097, iki bar kaynağında aynı. Bedeli: süresi dolan 0,11 → 0,30, karne tabanı kayar (p_real 0,44 → 0,36), kalibrasyon farkı +0,007 [+0,001, +0,013]. Uyarılar: SENS fitil kuralında kısmen yapısal; Arkin'in deseninde (22 durum) yalnız 7'si gerçekleşti'ye döner — `umut` 13 (A53). |
| Ö78 | **Dedektör seti `rp4` = `mevsim`:** 1 sa ve üstü gün içi barlarda hacim patlaması ve kırılım günün aynı saat dilimine göre düzeltilir (sürüm 2); 1 sa altında ve günlükte ürünle bit eşit. 5 dk ve 15 dk'da aday yok: ürün sabitleri kalır. | Kalibrasyon 1 sa: açılış/kapanış ıskaları −%25 / −%19, isabet +0,003; sınama 1 sa: `ft` 0,061 → 0,043, isabet aynı. Sınırda bir aday (üst uç −0,0005). |
| Ö79 | **Kapsül sabitleri adlandırılmış kayıtlarda:** `EngineConfig::named("r3"|"rp4")`, `qp_facts::detectors_named`; `SessionInit.capsules.engine` (yoksa `r3`, yazılmaz; var olan dökümler ve altınlar bayt eşit); `qp sim/demo --rule`. **Protokol varsayılanı `r3` kalır.** Öneri: gün içi gösterimde (`--live`, `--sim`) varsayılan `rp4` — Arkin'in kararı. | `docs/CAPSULE.md` §9.9; verify 18/18; hesap özeti aynı. Karne anahtarı dedektör sürümünü taşır (`breakout.range@2`); oturumlar arası saklanan karnede anahtara kayıt adı da girmeli. |

### T7 (satır zamanı, A48)

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö86 | **A48 kapatılır:** kapanış emir toplamada genel satır "Kapanış seansında" / "Closing auction" der (durum `closed`, kabuk görünümü aynı). Karar fiyat kartlarının evrelerinden: evreli enstrümanların yarıdan çoğu `closing_session(_wo_eq)` ise; takvimin `open`'ı ne derse desin, tatil ve bayat veri önce gelir (`qp_capsule::market::in_closing_auction`). | R6 L6: 18:00'da takvim `open: false`, bütün semboller kapanış evresinde (5) iken ekran "Seans kapalı" dedi. Testler `market.rs`, `qp-wasm/tests/market_r6.rs`. Canlı doğrulama ilk iş gününde 18:00–18:10. |
| Ö87 | **Satır zamanı adlandırılmış seçenek arkasında:** `style` `clock` / `ViewState.clock` (varsayılan kapalı; protokol dökümleri bayt eşit); sunum kabuğu açar (`ShellCore::presented_look`), `?clock=0` kapatır. Zaman: açık hikâyede doğuş, kapanmışta kapanış barının açılış zamanı; gün içinde ve bugünse `SS:DD`, değilse tarih; "şimdi" serinin son barı; gösterim dilimi dokümanın `display_tz`'si, yoksa borsanınki. Ses başlığının başına (ses 80 karakter sınırını zamanla birlikte korur), `korda` şerit etiketinin olaydan sonrasına. Yetkisize yok (K33). | Var olan altınlar (v1, v2) ve dökümler bayt eşit kaldı; yeni görünüm yeni altınlarla (`fixtures/golden/demo/p4-*`). `rounds/RP4/experiments/T7.md`. |

### T4 (hızlandırılmış benzetim)

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö84 | **Benzetim canlı yolun kendisidir; saat çalışma zamanına enjekte edilir.** `LiveRuntime`'ın "şimdi"si duvar saati ya da benzetim saati (`use_sim_clock`, yalnız `LiveIn::Clock` ilerletir); bar kapatma, piyasa durumu ve gecikme tahmini bu saate bağlı (önce `Instant`'a bağlıydı). İşlemler işlem ucunun iletisi olarak (`TradeMessage`, varış = işlem zamanı) `LiveIn::Trade` ile girer; ayrı hat yok. | Aynı önbellek → her hızda bayt eşit akış (ölçüldü: 100 sembol, 1,76 M işlem, iki koşu aynı özet); canlı sahte sunucu + aynı işlemler → bayt eşit veri olayları (`sim::tests::simulation_path_is_the_live_path`). `rounds/RP4/experiments/T4.md`. |
| Ö85 | **Benzetimde ısınma: önbellekteki REST barı, yetmezse önceki günlerin işlemlerinden aynı bar kurucuyla** (sembol başına daha çok bar veren; eşitlikte REST); seans evresi fiyat kartı olmadığından XIST takviminden (`MarketSource::Calendar`). | REST 5 dk önbelleği şimdilik tek gün (2026-11-19); işlem önbelleği 2026-10-05'ten. Hangi kaynağın alındığı sayılır. M27 işlemden barın REST'le eşitliğini ölçer. |

### T2 (M27, işlemden bar)

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö80 | **Gün içi barlar işlemden** (`BarMethod::Trades`): zaman `timestamp_ns` (gün başından ns, borsa saati), açılış emir toplama eşleşmesi günün ilk barına, kapanış seansı eşleşmesi ve kapanış fiyatından işlemler günün son barına (livebar kural 1, değişmedi). | `rounds/RP4/experiments/M27/RESULTS.md`: 30 sembol, 34 gün; 1m/5m/1h fiyat ve lot hacim ≥ %99,999 bit eşit (1m'de 464.890 barın 1'i, 5m'de 95.871'in 1'i farklı). Kalibrasyonda `date_time` %99,97, seanslar düşürülürse 1h %75. |
| Ö81 | **Özel işlemler (`trade_ex`) yalnız günlük bara ve yalnız hacimleriyle** (`SpecialTrades::DayOnly`, yeni varsayılan; `LiveConfig` ve `TradeFeedConfig`; fiyat alanlarını değiştirmez — denetim K1 sonrası ölçüldü: fiyatı alınınca 1.020 günlük barın birinde düşük %3,6 farklı, alınmayınca 1.020/1.020 tam). Fiyat kartı yolu değişmedi. | Kalibrasyon günleri (10-05..10-16): REST 1d özel işlemleri içeriyor (exclude %86,33 → day_only %100), gün içi içermiyor (include 1h %98 → day_only %100). |
| Ö82 | **Günlük bar kartın gün alanlarından ya da işlem + `trade_ex`'ten**; gün içi işlemden. Karma "işlem varsa işlem, yoksa kart" gün içinde işlemle aynı. | R6 kayıt günü: kart 1d 100/100 bit eşit (TL dahil); işlem 1d %95 (o günün `trade_ex`'i önbellekte yok). Gün içi kart %83–85, işlem %100. |
| Ö83 | REST'in kapanış barı (18:00+) K14 eşlemesinde **günün son seans barına** (son dakikada işlem yoksa o bar kapanıştan açılır), önceki REST barına değil: canlı kuralla aynı. `qp live compare-trades --rest-close last` böyle; `fetch-bars.sh`, harness `prepare`, `qp live compare` R3 kuralında kaldı. | 34 günde 3 gün-sembolde fark (1m), `last` ile 0. |

### L-prep (canlı işlem ucu, CLI)

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö88 | **İşlem kipinde barların canlı/bayat durumu işlem ucuna bağlıdır.** Piyasa ucunun kaybı bayatlatmaz ve REST bar onarımı yapmaz (kart yalnız seans evresi, günlük alanlar ve saat verir); işlem ucunun kaybı bütün beslemeleri bayatlatır, yeniden bağlanınca boşluklar REST işlem geçmişiyle dolar (hub dışında bir iş parçacığında; benzetimde eşzamanlı). Eski bayat dönemin sonucu atılır; başarısız boşluk 30 sn'de bir yeniden istenir. | REST bar onarımı işlemle kurulan barları REST barıyla değiştirirdi (M27: kaynaklar her zaman eşit değil). Test: `sim::tests::trade_link_drops_repair_from_rest_bit_equal` (onarım sonrası barlar yalnız REST işlem geçmişinden kurulanla bit eşit). |
| Ö89 | **Yeniden tohumlanan işlem beslemesi işlem durumunu taşır** (`LiveFeed::carry_trades_from`): bağlantı durumu, izlenen semboller; boşluk yeni dikişten. İşlemle kurulmuş bar "başlamış" sayılır. | T4'ün bildirdiği sorun 4; test `sim::tests::reseeded_trade_feed_keeps_watched_symbols_and_trade_link`. |
| Ö90 | **Ham kayıtta ve hub'da işlem yükü yalnız düzey 1** (`keepable_payload_on`; işlem ucunda her yük, konusundan bağımsız — güvenlik incelemesi): yük kayda ve hub'a girmeden yeniden kodlanır; çözülemeyen işlem yükü boş tutulur; alıcı/satıcı alanının telde gelip gelmediği yalnız sayılır. | K66; test `tests/live_record_trades.rs`, mock testleri. |

### T6 (ölçek: artımlı görüntüleyici, bellek)

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö91 | **Görüntüleyici akışı artımlı: `capsules` öğesi tam girdisiyle bellekte** (`capidx::ItemMemo`). Yetkili okurun (projeksiyonsuz) öğesi — cümle, geçerlilik, ses — yalnız girdisi değişince yeniden yazılır; girdi bayt bayt karşılaştırılır: kapsülün kendisi (hikâyesi), hikâyenin olguları, "şimdi"nin okunduğu bar serisi (barları), dil, ses, saat seçeneği, başlık takvimi, gösterim dilimi. Okurun `relevant`, `coin_flip` alanları ve piyasa notu her seferinde uygulanır. Projeksiyonlu okur (yetkisiz) eskisi gibi baştan. | R6 profili: işlemcinin çoğu her partiden sonra bütün okur cümlelerinin (ses dahil) yeniden kurulmasıydı. 580 sembolde öğelerin %94'ü yeniden kullanıldı; örnekleme profilinde görüntüleyici hesabı bir çekirdeğin ~%17'sinden ~%6'sına, süreç işlemcisi yan yana ölçümde %18,9'dan %11,2'ye, tik→çerçeve p95 46,8'den 23,2 ms'ye. Artımlı = baştan: iki eş barındırıcı, biri belleği her girdiden önce unutur; bütün satırlar bayt eşit (`capidx::item_memo_tests`, `livehost::tests::incremental_viewer_equals_from_scratch_on_live_events`). `rounds/RP4/experiments/T6.md`. |
| Ö92 | **Barındırıcının kaydettiği veri olayları sıkışık** (`ReplayHost` içinde besleme numarası + değişiklik; olay başına ≈ 70 bayt, iki dize yerine). `recorded_events()` artık yazılmış bir `Vec` döndürür; yalnız yeniyi gönderen `recorded_len()` / `recorded_since(n)` kullanır. Canlı işlem tekilleştirme penceresi (30 dk) düz ondalık işlem numarasını sayı olarak tutar (`SeenKey`, bire bir). | Gösterim gün boyu her işlemi kaydeder (geç katılan kopya için): önce olay başına ≈ 200 bayt + vektör büyümesi. Bit bit geri yazım testi (`replayhost::recorded_tests`), bire birlik testi (`feed::seen_key_tests`); 580 sembol tam gün benzetim özeti değişmedi. |
| Ö93 | **Benzetimde başlangıç saati ve tek, sıkışık işlem kopyası:** `--at HH:MM` / `--sim-at HH:MM` (o günün önceki işlemleri ısınma); günün işlemleri sembol başına bir kez, 32 baytlık satırlarla (`sim::Packed`), gönderim sırası okunurken birleştirilir; ayrıştırma 4 MB'lık parçalarla. Benzetimde tik→çerçeve, hızlı (N×) gönderimin duvar saatinden ölçülür (`LiveIn::Arrival`; çıktıyı değiştirmez). | Önce işlemler bellekte iki kez (≈ 830 MB, 580 sembol 11:00'den). 580 sembol, 11:00'den tam gün, `max`: akış özeti önce/sonra aynı (`6524d48f…`). 580 sembol 1× penceresinde RSS 1 060–1 142 → 376–418 MB. |

### RP4 açık soruları

| # | Soru | Kim | Not |
|---|------|-----|-----|
| A53 | Çıkış kuralı: önceden yazılı sıralama `yukdus`'u seçti; ama SENS ölçüsü kapanışı oynatır, fitil kuralının fitil gürültüsünü ölçmez; Arkin'in gördüğü desende `umut` (zamanla daralan durdurma, süresi dolan 0,14) daha iyi. Fitil oynatmalı bir SENS ile yeniden ölçüm mü, `umut` mu, `yukdus` mu? **Not (denetim K2):** `umut` gözlemi sınama aralığından (2026-11-20); `umut`'a geçmek ancak 2026-11-20 sonrasındaki yeni bir sınama aralığında doğrulanırsa geleceğe bakmamış olur. | Arkin | `M25/RESULTS.md` okunuş 1–3; `S/RESULTS.md` §3. |
| A54 | Gün içi karne tabanı kuralla değişiyor (süresi dolan ıska sayılır): K53'ün 0,55 yazı tura eşiği gün içinde `rp4` altında yeniden ayarlanmalı mı? | Arkin | 5 dk p_real r3 0,44 → rp4 0,36. |
| A55 | Gün içinde anlamsız dedektörler: 5 dk/15 dk/1 sa'te hiçbir dedektör kalibrasyonda 0,55'e ulaşmıyor (havuz 0,41/0,44/0,47). Periyot sınırı: kırılım, MA dizilimi, RSI aşırılık en kısa 1 gün; sıkışma en kısa 15 dk? (Sınamada 1 sa 0,54–0,58: rejim.) | Arkin | `M26/RESULTS.md`. |
| A56 | Bölünme/kurumsal işlem: işlem barları düzeltilmemiş; gün içi karne bir sembolün geriye dönük düzeltmesinde fiyat sıçraması görür. Canlıda düzeltme olayı nasıl ele alınsın? | Sonraki round | M27 §(d); S tablo 2. |
