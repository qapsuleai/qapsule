# RP4 — Gün İçi Kalibrasyon, İşlem Akışı ve Sağlamlaştırma: Rapor

Tarih: 2026-11-20 23:34 → 2026-11-21 (tek oturum). Borsa **kapalıydı** (Cuma gece başlandı, hafta
sonu): canlı bağlantı **açılmadı**; REST geçmişinden (işlem ve bar) hızlandırılmış benzetim kuruldu,
canlı doğrulamalar "İlk iş günü" listesinde (§13). Gerçek veri, adres, belirteç depoya, günlüğe,
rapora girmedi; burada yalnız toplu sayılar var. Her ağ işlemi `experiments/LIVE-LOG.md`'de.

## 1. Özet

- **Veri:** 1.933 REST veri isteği (sıralı, ölçülü, önbellekli): BIST100'ün 2026-10-05 → 11-20
  işlemleri (64,4 M işlem, 34 gün), 5 dk barları 2025-11-03'ten (241 kalibrasyon + 24 sınama günü),
  1 sa, 1 gün (2018→), BIST30 düzeltilmemiş barlar (M27), BISTTÜM'ün 2026-11-20 işlemleri (580 sembol,
  ölçek). Hata: 15 istek aynı veriyi ikinci kez çekti (fark edilip durduruldu; §2).
- **M25 (çıkış kuralı):** önceden yazılı düzenek ve puanlama, beş alternatif ayrı ajanlarda. Üç aday;
  önceden yazılı sıralama **`yukdus`**'u seçti (gün içi; günlük `r3` kalır). Sınamada 5 dk erken
  kapanma **0,156 → 0,065 (0,42×; hedef ≤ 0,5× tuttu)**, tek bar duyarlılığı 0,198 → 0,097; bedeli
  süresi dolan 0,11 → 0,30 ve küçük bir kalibrasyon kötüleşmesi (+0,007). Arkin'in gördüğü durum
  benzetimde ve düzenekte yeniden üretildi: 2026-11-20'de 22 durum; `rp4` ile 7'si gerçekleşti, 7'si
  yine tutmadı (daha geç, geniş eşikte), 5'i süresi doldu (§5).
- **M26:** gün içinde 5 dk/15 dk'da aday yok; 1 sa'te `mevsim` (sınırda). **Gün içinde hiçbir dedektör
  0,55'e ulaşmıyor** (kalibrasyon havuzu 5 dk 0,41, 1 sa 0,47; günlük 0,51); periyot sınırı önerisi.
- **M27 (bar kaynağı):** işlemden bar 1 dk/5 dk/1 sa'te REST'le fiyat ve lot hacimde **≥ %99,999
  bit eşit** (hedef ≥ %99 tuttu); günlükte %88,6 → kalan fark özel işlemlerdi (`trade_ex`, yalnız
  günlüğe girer). Fiyat kartı yolu (R6) gün içinde %83–85.
- **Benzetim:** canlı yolun kendisi (aynı `LiveFeed`/`LiveRuntime`/otorite; test), bayt eşit, 1 gün ×
  100 sembol × 5 dk `max` hızda **42–49 s** (hedef ≤ 2 dk). `scripts/demo.sh --sim`; `--lan` reddi.
- **Sağlamlaştırma:** A52 durma kuralı (10 dk'da 2 açıklamasız sunucu kapatması → dur), insan onaylı
  `qp live reset`; geç katılan tam kopya sorunu yeniden üretilemedi (üç düzeyde bayt eşit; düzeltme
  gerekmedi). Canlı işlem ucu (`/trade`, kendi kilidi, düzey 2 yok) ve `qp demo --live --trades`
  sahte sunucuda hazır; gerçek bağlantı ilk iş günü.
- **Ölçek:** 580 sembol, tek süreç, benzetimde 1× hız: işlemci %11, RSS 376–418 MB, tik→çerçeve p95 23 ms (hedefler ≤ %30 / ≤ 450 MB / ≤ 150 ms tuttu; R6 canlı: %69 / 0,9 GB / 835 ms).
- **Küçük işler:** satırlarda saat (gösterimde), kapanış seansı satırı (A48).
- Sabitler sürümlü (`r3`, `rp4`); protokol varsayılanı `r3`; var olan dökümler, altınlar ve hesap özeti
  aynı. Önerilen kararlar Ö77–Ö90, açık A53–A56 (`DECISIONS.md`).

## 2. Ne yapıldı (alt görev → dal → birleşme)

| Görev | Dal | Ajan | Özet |
|---|---|---|---|
| T0 | main | orkestratör | Ana akış commit'i (`c92b011`); deneme alt ajanı (başladı); `SPLIT.md` (`cc476de`) **veri çekilmeden önce**. |
| T1 | `rp4/t1-trades` | orkestratör | `RestClient::trades_csv`/`trades_ex_csv` (yalnız düzey 1 sütunlar; alıcı/satıcı sütunu gelirse atılır), `qp live trades` (≤ 15 günlük pencere, önbellek, istek günlüğü), `AuthError::HttpCode`. Küçük olduğu ve ağ yalnız orkestratörde olduğu için alt ajana verilmedi (sapma). |
| D1 | — | orkestratör | `scripts/rp4-fetch.sh`; sayılar LIVE-LOG. |
| T3 | `rp4/t3-trade` | alt ajan | `TradeMessage` çözümü (alıcı/satıcı tanımsız prost tipiyle atılır), işlem ucu (`<kip>-trade` kilidi), `@lvl2` işlem konusu yerelde reddedilir, `BarMethod::Trades`, `LiveFeed::with_trades`, boşluk/onarım, sahte sunucu testleri, `parse_topach`. |
| T5 | `rp4/t5-halt` | alt ajan | A52 (`halt.rs`, kayan pencere, enjekte saat), `qp live reset`, geç katılan istemci (3 düzeyde bayt eşit; düzeltme yok). |
| T7 | `rp4/t7-time` | alt ajan | Satır saati (`clock` seçeneği, gösterim açık), A48; v1/v2 altınlar bayt eşit, yeni `p4-*` altınlar. |
| H | `rp4/h-harness` | alt ajan | Düzenek (`prepare`, `m25`, `m26`, `report`, `ticks`); SPEC (`eb1a8c3`) alternatiflerden önce. |
| T4 | `rp4/t4-sim` | alt ajan | `qp sim`, `qp demo --sim`, `scripts/demo.sh --sim`. |
| T2 | `rp4/t2-m27` | alt ajan | `qp live compare-trades`, `export-trade-bars`, `price-steps`; `SpecialTrades::DayOnly`. |
| M25a–e | `rp4/m25a-taban`, `m25b-teyit2`, `m25c-yukdus`, `m25d-olcek`, `m25e-savunu` | 5 alt ajan | Yalnız kalibrasyon kopyası; tam veri klasörü bu evrede okunamaz (`chmod 000`). |
| M26b–c | `rp4/m26b-olcek`, `rp4/m26c-mevsim` | 2 alt ajan | Aynı. |
| eval | `rp4/eval` | orkestratör | Alternatiflerin birleşimi, `--reference` (aday−aday eşli aralık), bütün sayıların yeniden üretimi. |
| S | `rp4/s-integrate` | alt ajan | `HighLow`, `Rp4`, `rp4_detectors`, `EngineConfig::named`, `SessionInit.capsules.engine`, `--rule`; deneylerle bayt eşit. |
| L-prep | `rp4/l-prep` | alt ajan | `qp live record --trades`, `inspect` işlem bölümü, `RestTradeHistory`, `qp demo --live --trades`, T4'ün yeniden tohumlama sorunu düzeltildi. |
| T6 | `rp4/t6-scale` | alt ajan | §8. |
| A | — | bağımsız alt ajan | `AUDIT.md`. |

Her dal birleştirilmeden önce fmt + clippy + test yeşildi (alt ajanların koşusu ve birleşmeden sonra
main'de ayrıca). Birleşen her görevin worktree'si hemen kaldırıldı. Güvenlik incelemesi iki bulgu
verdi, ikisi de düzeltildi: `@lvl2` reddinin olay metni maskelenmiyordu; işlem ucunda konusu
tanınmayan bir yük temizlenmeden kaydedilebilirdi (`1821a6f`: işlem ucunda her yük temizlenir).

**Süreç notları (dürüstlük):** (1) Round başında Arkin'in `qp demo --live` süreci kimlik kilidini
tutuyordu; Arkin'in onayıyla durduruldu (LIVE-LOG N0). (2) VPN kopukken 4 belirteç isteği 403 aldı
(N1). (3) Alternatif ajanları aynı oturum scratchpad'ini paylaştı; iki ajan birbirinin kalibrasyon
sonuç dosyalarını gördüğünü bildirdi (sınama verisi değil; seçimlerini etkilemediğini yazdılar). (4)
Ölçek ölçümü başka ajanların derlemeleriyle aynı anda yapıldı (yük §8'de).

## 3. Kabul ölçütleri

| # | Durum | Kanıt |
|---|---|---|
| 1 | ✅ | `qp live trades` (14 günlük pencereler, önbellek, `requests.jsonl`), 2026-10-05→11-20, BIST100 + 480 sembol (1 gün), 1.933 REST veri isteği (LIVE-LOG); veri yalnız `fixtures/local/` (izlenmiyor). Düzey 2: `trade_csv_level1` (test `level2_columns_never_survive`), `trade_bs` hiç çağrılmıyor, `@lvl2` işlem konusu kurulamaz/yerelde reddedilir (testler), ham kayıtta ve hub'da her işlem yükü temizlenir; kod taraması: `trade_bs` yalnız bir yorumda, `@lvl2` işlem konusu yalnız ayrıştırıcı testinin girdisinde. |
| 2 | ✅ | `experiments/M27/RESULTS.md`: 30 sembol, 34 gün; 1 dk/5 dk/1 sa ≥ %99,999 bit eşit (o/h/l/c/lot); 1 gün %88,6 → `trade_ex` eksikliği (tam `trade_ex` çekildi, D1f). Zaman `timestamp_ns` = gün başından ns (44,86 M satırın hepsi [0, 1 gün)); açılış eşleşmesi ilk bara, kapanış seansı son bara (REST 1 dk 18:04 barı 300/300); özel işlemler yalnız günlüğe — kalibrasyon günlerinde ölçülerek. |
| 3 | ✅ (canlı: ilk iş günü) | T3 + L-prep: kod, kendi kilidi, sahte sunucu testleri (ilk durum yok; kopmada bayat; REST işlem geçmişiyle onarım bit eşit; piyasa ucuyla birlikte her uçta tek bağlantı). Gerçek bağlantı: §13 adım 2–7. |
| 4 | ✅ | `qp sim` canlı yolun aynısı (test `simulation_path_is_the_live_path`); iki koşu aynı özet (§7); 1 gün × 100 sembol × 5 dk: 42–49 s (≤ 2 dk); `scripts/demo.sh --sim --lan` reddi (`tests/demo_sim.rs`). |
| 5 | ✅ | SPLIT (`cc476de`) → SPEC (`eb1a8c3`) → alternatif dalları → seçim (`8bb7571`) → sınama. Beş alternatif; dört periyot tabloları (`M25/RESULTS.md`, `S/RESULTS.md`); erken kapanma önce/sonra; duyarlılık; öneri gerekçeli (Ö77, A53). Arkin'in durumu: §5. |
| 6 | ✅ | `M26/RESULTS.md`: gün içinde anlamsız dedektörler dürüstçe; periyot sınırı önerisi (A55); açılış/kapanış doğumları sayıldı (5 dk'da daha isabetli, 1 sa'te yanlış tetikleme). |
| 7 | ✅ | `rp4` günlükte `r3` ile bayt eşit sonuç; R3 başvurusu tablosu `S/RESULTS.md` §1/§5 (kalibrasyonda R3'ü yeniden üretiyor: 0,087 ↔ 0,088). |
| 8 | ✅ | `EngineConfig::named("r3"|"rp4")`, `detectors_named`; protokol varsayılanı `r3`; değişen döküm 0, değişen altın 0 (T7'nin yeni `p4-*` altınları ayrı adlarla, gerekçesi `experiments/T7.md`); hesap özeti aynı (verify). |
| 9 | ✅ (canlı davranış: ilk iş günü) | `halt.rs` + 4 sahte sunucu testi + 2 süreç testi; `qp live reset` notsuz reddeder, uçbirimsiz `--i-am-human` ister. Geç katılan: üç düzeyde bayt eşit (sorun yeniden üretilemedi). |
| 10 | ✅ (benzetimde) | §8: artımlı görüntüleyici öğeleri; 580 sembol önce/sonra; üç hedef tuttu; 100 sembolde gerileme yok; özellik testleri. |
| 11 | ✅ | Satır saati (`clock`, gösterimde açık), A48 "Kapanış seansında"; var olan altınlar ve dökümler bayt eşit; yeni `p4-*` altınlar (T7.md). |
| 12 | ✅ | §13 (komut, beklenen sonuç, süre). Round iş gününe denk gelmedi; LIVE-LOG'da canlı deneme yok. |
| 13 | §9 | |
| 14 | §10 | |
| 15 | §11 | |

## 4. M27 — gün içi bar kaynağı

`experiments/M27/RESULTS.md` (yeniden üretim: `qp live compare-trades`). BIST30, 34 gün, REST
fiyatları 0,001'e yuvarlanarak (REST tek duyarlıklı sayıyı fazla basamakla yazıyor: fiyatların %5,8'i).

| Periyot | Bar | Tümü tam (lot) | o/h/l/c | lot | TL hacim ≤ 1e-6 |
|---|---|---|---|---|---|
| 1 dk | 464.890 | %99,9998 | 100 | 100 | 100 |
| 5 dk | 95.871 | %99,999 | 100 | 100 | 100 |
| 1 sa | 8.010 | %100 | 100 | 100 | 100 |
| 1 gün | 1.020 | %88,6 (kalibrasyon günlerinde %100) | 100 | 88,6 | 88,5 |

Kaynak karşılaştırması (R6 kayıt günleri): fiyat kartı 1 dk %84,8, işlem %100, karma %100; günlükte
kart %100 (özel işlemleri içeriyor). İki kaynaktan doğan kapsüllerin farkı (sınama, 100 sembol): 5 dk
doğumların %2,4'ü yalnız bir kaynakta (çoğu REST'te eksik barlar), 1 sa %1,0 (`S/RESULTS.md` §2).
Fiyat adımı: 581 sembolün 3.583 kart iletisinde `priceStep` BIST tablosuyla %100 uyumlu; işlemlerin
%99,9998'i adım ızgarasında (harness'in `≤` sınır yorumu tablo sınırında 22 durumda farklı: Öneriler).

## 5. M25 — çıkış kuralı

Düzenek ve kural: `experiments/harness/SPEC.md`; kalibrasyon ve seçim `M25/RESULTS.md`; sınama
`S/RESULTS.md`. Veri: 100 sembol; gün içi 241 kalibrasyon + 24 sınama günü; günlük 1.751 + 473.

**Kalibrasyon, 5 dk (yönlü):**

| Kural | ECR_own | ECR_ref | SENS | p_real / p_inv / p_exp | aday |
|---|---|---|---|---|---|
| `r3` (başvuru) | 0,148 | 0,148 | 0,212 | 0,415 / 0,467 / 0,118 | — |
| `taban` (8 adım tabanı) | 0,069 | 0,087 | 0,193 | 0,284 / 0,290 / 0,426 | hayır (süresi dolan) |
| `teyit2` (iki kapanış) | 0,061 | 0,071 | 0,132 | 0,315 / 0,344 / 0,341 | evet |
| `yukdus` (yüksek/düşük) | 0,066 | 0,115 | **0,099** | 0,342 / 0,342 / 0,316 | **evet, seçildi** |
| `olcek` (periyoda göre) | 0,065 | 0,090 | 0,279 | — / — / 0,315 | hayır (SENS) |
| `umut` (daralan durdurma) | **0,026** | 0,036 | 0,199 | 0,367 / 0,482 / 0,151 | evet |

**Sınama (yönlü, REST barları; işlem barlarında aynı):**

| Periyot | `r3` ECR / SENS / p_exp | `rp4` ECR / SENS / p_exp | ECR oranı |
|---|---|---|---|
| 5 dk | 0,156 / 0,198 / 0,106 | **0,065** / 0,097 / 0,304 | **0,42** |
| 15 dk | 0,147 / 0,141 / 0,109 | 0,070 / 0,040 / 0,283 | 0,48 |
| 1 sa | 0,121 / 0,074 / 0,065 | 0,040 / 0,011 / 0,216 | 0,33 |
| 1 gün | `rp4` = `r3` (bayt eşit) | | |

Kalibrasyon: Brier her kuralda kendi tabanına eşit (gün içinde karne öngörü becerisi taşımıyor, r3'te
de); 5 dk kalibrasyon farkı +0,007 [+0,001, +0,013] (sınır +0,03 içinde, küçük ama gerçek).

**Arkin'in gördüğü durum** (2026-11-20, 5 dk, BIST100, `harness cases`, REST ve işlem barlarında aynı):
`r3`'ün ertesi barda "tutmadı" dediği 79 düşüş iddiasından **22**'si ufuk içinde hedefe ulaştı. Aynı
hikâyeler `rp4` ile: **7 gerçekleşti, 7 tutmadı, 5 süresi doldu, 2 açık, 1 doğmadı.** Örnek (sembolsüz):
16:25 doğum, ertesi kapanış +1,24 ATR (r3 eşiği 0,95) → r3 tutmadı; sonra −1,66 ATR'ye indi; `rp4` (eşik
1,9 ATR) süresi doldu. Ayrıntı yalnız yerelde (`fixtures/local/rp4/cases/…local.jsonl`).

**Dürüst sonuç:** önceden yazılı kural `yukdus`'u seçti ve hedef tuttu; ama (1) `yukdus`'un düşük SENS'i
kısmen yapısal (kapanış oynatılır, fitil değil), (2) Arkin'in desenini en iyi `umut` çözüyor (22'nin 13'ü
gerçekleşti, süresi dolan 0,14), (3) bütün adaylarda kazancın çoğu bariyerin genişlemesinden. Karar
Arkin'in (Ö77, A53).

## 6. M26 — dedektör eşikleri

`M26/RESULTS.md`. Kalibrasyonda havuz isabeti 5 dk 0,413, 15 dk 0,437, 1 sa 0,470, günlük 0,509; sembol·gün
başına doğum 18,2 / 6,4 / 1,5 / 0,20. Açılış ve kapanış barlarında doğanlar 5 dk'da geri kalandan daha
isabetli (`ft` −0,050), 15 dk'da fark yok, 1 sa'te daha kötü (+0,032). `olcek26` (ölçekli pencereler)
aday değil; `mevsim` 1 sa'te aday (sınırda) → `rp4` dedektör seti. Sınamada 1 sa `ft` 0,061 → 0,043.

## 7. Benzetim

`docs/matriks/LIVE.md` §8, `experiments/T4.md`. 1 gün (2026-11-18), 100 sembol, 5 dk, `max`: 1.764.375
işlem, 9.683 kesin bar, duvar süresi 41–49 s (yükleme ~12 s + koşu ~30 s), bellek tepe 1,7–2,0 GB.
2026-11-20 benzetiminin barları `export-trade-bars` dosyalarıyla 9.588/9.588 bit eşit. İki koşu aynı
özet: §9 (VERIFY 7).

## 8. Ölçek (T6)

`experiments/T6.md`. Düzenek: `qp demo --sim --sim-at 11:00 --sim-speed 1 --interval 5m`, 2026-11-20,
**1× hız**, 21,5 dk pencere, başsız tarayıcıda görüntüleyici açık (telefon görünümü, ilk kapsül
açılıyor). 580 sembolün 578'i başladı (2'sinin 11:00'den önce işlemi yok). Tik→çerçeve: işlemin
gönderim anından görüntüleyicinin çerçevesine (`LiveIn::Arrival`).

| | İşlemci | RSS | tik→çerçeve p50 / p95 |
|---|---|---|---|
| R6 L5 (canlı, 580, başvuru) | %69 | 864–897 MB | 159 / 835 ms |
| 580 önce → sonra (yan yana, aynı yük) | %18,9 → **%11,2** | 1.060–1.142 → **376–418 MB** | 0,70 / 46,8 → **0,61 / 23,2 ms** |
| 100 önce → sonra (yan yana) | %14,7 → %8,5 | 608–653 → 287–331 MB | 0,32 / 40,7 → 0,29 / 18,4 ms |
| **Hedef (580)** | ≤ %30 ✅ | ≤ 450 MB ✅ | p95 ≤ 150 ms ✅ |

Ne değişti: (1) görüntüleyicinin `capsules` öğesi yalnız girdisi (bayt düzeyinde) değişince yeniden
yazılır (öğelerin %94'ü yeniden kullanıldı; görüntüleyici hesabı bir çekirdeğin ~%17'sinden ~%6'sına);
(2) kayıtlı veri olayları sıkışık (~200 → ~70 B); (3) işlem tekilleştirme anahtarı sayı; (4) benzetimin
kendi girdisi tek kopya (bu, R6'nın canlı ölçümünde yoktu). Artımlı = baştan (iki özellik testi),
akış özetleri ve verify 18/18 aynı. **Sınırlar:** ölçüm canlı değil benzetimde (canlı 580 ölçümü ilk iş
günü listesine eklenebilir); işlemci yüzdesi koşudan koşuya %9–14 oynadı (araç ortamı `nice 5`;
verimlilik çekirdekleri olabilir, doğrulanmadı) — bu yüzden önce/sonra yan yana ölçüldü; ilk "önce"
koşusunda makine yükü medyanı 16,8 (başka ajanların işleri), yan yana koşularda 3,2 / 4,6; bir koşu
yük sıçramasında geçersiz sayılıp yinelendi. Tam seans (8 sa) bellek büyümesi ölçülmedi (~2–2,5 MB/dk).
100 sembolde gerileme yok.

## 9. Doğrulama

(verify, CI, taramalar, quantex.)

## 10. Disk

(kapanış temizliği.)

## 11. Bağımsız denetim

(AUDIT.)

## 12. Öneriler (kapsam dışı, yapılmadı)

- Fitil gürültüsünü de oynatan bir SENS ile M25 adaylarını yeniden ölçmek; `umut` ile `yukdus`'u
  Arkin'in deseninde karşılaştırmak (A53). Ufku eşikle birlikte ölçekleyen bir kural (`OutcomeRule::horizon`
  hikâye başına olmalı; `taban` ve `olcek` savunucularının önerisi).
- SPEC §4 koşul 3'ü "Brier − taban" olarak okumak (taban kayınca Brier mekanik değişiyor; `umut`
  savunucusunun önerisi).
- Harness fiyat adımı tablosunda sınır `[alt, üst)` (borsa böyle; SPEC `≤` dedi; 22 durum).
- REST kapanış barının K14 eşlemesi her yerde "son seans barı" (Ö83): `fetch-bars.sh`, harness `prepare`.
- REST'te boş gelen 3 dosya (1 dk, 10-19→11-01) ve 11-16'da kısa 4 gün yeniden çekilmeli mi.
- Durma (halt) bir uçta olunca öteki ucu da kapatmak (L-prep notu).
- Mevsim dedektörleri XIST'e sabit; başka borsa gelirse takvimden.
- Karne oturumlar arası saklanırsa anahtara kayıt adı (`r3`/`rp4`).
- `scripts/qapsule-gh.env` izlenmesi (eski öneri).

## 13. İlk iş günü yapılacaklar (Arkin ya da orkestratör; tek süreç, sırayla)

Hepsi depo kökünden, anlık kipte (`MATRIKS_MQTT_MODE=realtime`), VPN bağlıyken. Aynı makinede başka
bir `qp` canlı süreci (ör. açık kalmış `qp demo --live`) olmasın: kimlik kilidi ikinciyi reddeder. Her
adımı `rounds/RP4/experiments/LIVE-LOG.md`'ye bir satır olarak yazın (yalnız sayılar).

| # | Ne (ölçüt) | Komut | Beklenen | Süre |
|---|---|---|---|---|
| 0 | Hazırlık | `scripts/live-env.sh` · `ls fixtures/local/*.halted` | `live-env: OK`, `mode: realtime`; `.halted` dosyası yok | 1 dk |
| 1 | **Anlık kip, piyasa ucu (A49)**, seans içinde | `scripts/live.sh record --symbols THYAO,GARAN,ASELS --seconds 120` ardından `scripts/live.sh inspect fixtures/local/live/raw/<son>.qpraw` | ilk durum 3/3; `updateDate` ile varış farkı p50 < 1 sn (gecikmeli kipte ~900 sn idi); `mx/timestamp` kabul ya da ret (yaz); çoklu bağlantı bildirimi 0 | 3 dk |
| 2 | **Canlı işlem ucu (ölçüt 3)** | `scripts/live.sh record --trades --symbols THYAO,GARAN,ASELS --seconds 120` | ilk satır `endpoint realtime-trade, 3 symbols, 3 topics`; son satır `level-2 fields on the wire (dropped, not kept): buyer B, seller S; unreadable 0`. `subscription refused mx/trade/…` görürseniz durun (lisans/konu: Topach). | 3 dk |
| 3 | İşlem kaydının denetimi | `scripts/live.sh inspect fixtures/local/live/raw/<son>-trade.qpraw` | `buyer: 0`, `seller: 0` (kayıtta); `valid as ns since local midnight: N of N`, `Auto` seçimi `ns_from_midnight`, gecikme p50 < 1000 ms; `activeBidOrAsk` değer kümesi (yazın: REST'te 98/97/null) | 1 dk |
| 4 | **İki uç birlikte, gösterim** | `scripts/demo.sh --live --trades --rule rp4 --symbols THYAO,GARAN,ASELS --interval 1m --history 120` → `http://localhost:8390/`; `curl -s -H "X-Qp-Live-Control: <konsoldaki sır>" localhost:8390/live/stats` | `trade_mode/market_connected/trade_connected: true`, `link: live`, `trade_fills[0].failed 0`, `feed.trades.far_from_arrival ≈ 0`, olay sayısı artıyor | 5 dk |
| 5 | İşlem ucu kopması ve onarım | `curl -s -X POST -H "X-Qp-Live-Control: <sır>" 'localhost:8390/live/drop?secs=20&endpoint=trade'` | `link` stale → repairing → live; `trade_fills[1].after_loss true`; piyasa ucu bağlı kalır; bayatken doğan kapsül 0 | 2 dk |
| 6 | Piyasa ucu kopması | aynı, `endpoint=market` | `link` live kalır, bar onarımı yok | 2 dk |
| 7 | Kapatma | `curl -s -X POST -H "X-Qp-Live-Control: <sır>" localhost:8390/live/quit` | iki bağlantı DISCONNECT ile kapanır; kilit dosyaları serbest | 1 dk |
| 8 | **Durma kuralının gerçek davranışı (A52)** | 4–7 boyunca ve 9'da konsol ve `/live/stats` | açıklamasız sunucu kapatması sayısı (beklenen 0); olursa `HALTED (unexplained server close)` + `.halted` işareti → **yeniden denemeyin**, Arkin'e bildirin; açmak yalnız `scripts/live.sh reset --endpoint <uç> --note "<kim, neyi denetledi>"` (uçbirimde onay sorar) | gözlem |
| 9 | **Tam seans kaydı (R6 ölçüt 4) + belirteç ömrü (A50)** | 09:50'de `scripts/demo.sh --live --trades --rule rp4 --symbols @fixtures/local/live/u30.txt --interval 5m`, 18:40'ta `/live/quit`. Ertesi gün: `scripts/live.sh bars --symbols @fixtures/local/live/u30.txt --period 1min --start <gün> --unadjusted` (1min, 5min, 1hour, 1day için), sonra `qp live compare --raw <piyasa kaydı>` ve `qp live compare-trades --symbols @fixtures/local/live/u30.txt --from <gün> --to <gün> --raw <piyasa kaydı> --rest-price mill` | Belirteç yenileme(ler)i: `/live/stats` ya da konsolda yenileme sayısı > 0 ve kopma yok (yenileme öncekini kapatıp yeniyi açar, tek bağlantı); işlem barı ↔ REST: 1 dk/5 dk/1 sa ≥ %99,9, günlük ≥ %99 (özel işlemler için ertesi gün `scripts/live.sh trades --ex --symbols @fixtures/local/live/u30.txt --start <gün> --end <gün>`); kart ↔ REST R6 tablosu tam seansla | 9 sa (gözetimsiz) + 30 dk |
| 10 | 18:00–18:10 kapanış seansı satırı (A48) | 9 açıkken 18:00–18:10 arasında ekran | üst satır "Kapanış seansında" | 10 dk |

Sonuçlar ve sorunlar LIVE-LOG'a; `fixtures/local/live/raw/` saklama kuralı (30 gün / 2 GB) geçerli.

## 14. Hızlandırılmış benzetimi nasıl izlerim

Borsa kapalıyken gerçek geçmişi canlı yolun aynısından, hızlandırarak izlemek için (ağ yok; yalnız
`fixtures/local/` önbelleği; ayrıntı `docs/matriks/LIVE.md` §8).

1. Önbellek: işlemler `fixtures/local/trades/` (BIST100, 2026-10-05→11-20; BISTTÜM 2026-11-20), barlar
   `fixtures/local/live/rest/`. Başka gün için önce (iş günü, VPN, tek süreç):
   `scripts/live.sh trades --symbols @fixtures/local/live/u30.txt --start <gün> --end <gün>`.
2. Tarayıcıda izle (30 sembol, iki gün, seansta 60×: 5 dk'lık bar 5 sn'de, kapalı saatler 2 sn):
   ```sh
   scripts/demo.sh --sim --rule rp4 --symbols @fixtures/local/live/u30.txt --start 2026-11-19 --end 2026-11-20 --speed 60
   ```
   `http://localhost:8390/` (yalnız bu makine; `--lan` reddedilir). Arkin'in gördüğü günü izlemek
   için `--start 2026-11-20`; aynı günü `--rule r3` ile açıp iki kuralı yan yana karşılaştırabilirsiniz
   (iki ayrı süreç sırayla; aynı anda iki `qp demo` aynı kapıyı kullanamaz). Hangi sembol/saat olduğu
   yerel dosyada: `fixtures/local/rp4/cases/cases.rp4.5m.2026-11-20.local.jsonl` (`born_local`, `symbol`).
3. Sayılar: konsolda basılan denetim sırrıyla `curl -s -H "X-Qp-Live-Control: <sır>" localhost:8390/live/stats`.
4. Tarayıcısız, en hızlı: `target/release/qp sim --start 2026-11-18 --rule rp4` (1 gün × 100 sembol ≈ 45 sn;
   yalnız toplu sayılar ve özet).
5. Durdurmak: Ctrl-C.
