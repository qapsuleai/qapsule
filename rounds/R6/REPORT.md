# R6 — Canlı Veri: Rapor

Tarih: 2026-11-19 (tek oturum, borsa açıkken başladı). Kip: **gecikmeli**
(`MATRIKS_MQTT_MODE=delayed`; anlık kip denenmedi — "Arkin'den beklenenler"). Her canlı deneme
`experiments/LIVE-LOG.md`'de (15 satır, eşzamanlı deneme yok). Gerçek veri, ham kayıt, belirteç ve uç
nokta adresi depoya girmedi; burada yalnız toplu sayılar var.

## 1. Özet

Qapsule kendi kimliğiyle Matriks'in gecikmeli market ucuna bağlandı, evreni 3 → 30 → 100 → 580
sembole (bütün pay piyasası) adım adım büyüttü ve `scripts/demo.sh --live` ile gerçek sembollerde
kapsül akışını canlı üretti (L6: 2 sa 13 dk, 761 canlı kapsül olayı). Round boyunca **hiçbir "çoklu
bağlantı" bildirimi alınmadı**; tek bağlantı süreç içi kayıt + makine kilidiyle her denemede korundu.
Canlı yol ile kayıttan oynatma aynı hattı (`qp_matriks::feed::LiveFeed`) kullanıyor; aynı kayıt iki kez
bayt eşit çıktı. Kopma bayat işaretliyor, REST'ten onarılıyor; bayatken üretilen kapsül 0. Günlük canlı
bar REST'e (düzeltilmemiş) 90/100 tam, hacmi 100/100 tam eşit; gün içi barlar kart iletisi birleştirmesi
yüzünden yaklaşık (fiyat %91–95 tam, hacim %95–97 ≤ %1). `verify.sh` 18/18 yeşil; var olan altınlar,
dökümler ve hesap özeti değişmedi.

## 2. Ne yapıldı (alt görev → dal → birleşme)

| Görev | Yer | Özet |
|---|---|---|
| T0 | `scripts/live-env.sh`, `shells/web`, `docs/CONCEPTS.md` | Ana akış commit'i (`e61928f`); değişken varlık denetimi (değer yazdırmaz); gösterim varsayılanı hikâye çizelgesi `korda` (akış sırası `live_new` R5'te vardı), v2 altınları `?story=pavka` ile sabitlendi, altınlar bayt eşit; sözlüğe "Canlı veri (R6)". |
| T1 | `crates/qp-matriks` | 22 proto bayt eşit kopya + köken kaydı (`proto/PROVENANCE.md`, `provenance.json`, SHA-256) + özet testi; tipler derlemede `protox`+`prost-build` ile (protoc yok); alan varlığını koruyan `SymbolBook` (protobuf birleştirme; gelmeyen alan değişmez — test), ilk durum/güncelleme, `MarketClock` (yalnız MX), `classify_notice` (MULTICONNECT), seans evresi tablosu, `rawlog` (CRC'li, kesik son kayıt güvenli). |
| T2 | `crates/qp-live` | Belirteç (bellekte, istekler arası ≥30 sn, üstel geri çekilme, 5 hatada dur), kendi MQTT 3.1/3.1.1 kodlayıcısı, tungstenite + rustls/ring (doğrulama açık), tek bağlantı (süreç içi + `fixtures/local/live-<uç>.lock`), çoklu bağlantıda dur + `.halted`, geri çekilme 2→60 sn / 6 hatada dur, `Redactor` maskeleme, sahte sunucu (`mock`). R6/L1'de `rest::RestClient` (servis belirteciyle REST bar ve meta) eklendi. |
| T6 | `qp_compute::address` | `SEMBOL:PERİYOT:GÖSTERGE[İÇ:param]:param` ayrıştırıcı ve zincir çevirici. |
| M22 | `qp_source::livebar` | Takvime hizalı canlı bar (`Ticks`, `CardDaily`, `Mixed`), ek yeri (`resume`), geç tik, `compare_bars`. |
| feed (T4, T5) | `qp_matriks::feed`, `qp replay --raw`, `qp live compare` | Canlı ve oynatmanın ortak hattı; ek yeri (`seed`); M22 karşılaştırma aracı. |
| T3 | `qp_schema::market`, `qp-session`, `qp-capsule`, kabuk | `MarketStatus`, `Session::set_market_status` (PROTOCOL §24), okuyucuya göre izdüşüm (güvenlik incelemesi bulgusu düzeltildi), kapsül dilinde seans evresi ve bayat ifadesi, kabukta durum satırı. |
| T7, M23 | `qp demo --live`, `crates/qp-cli/src/livehost.rs`, `ReplayHost::live` | REST geçmişi (gecikmeye göre kesilmiş) + tek bağlantı + `LiveFeed` + otorite + görüntüleyici; kopma → bayat (§18.4 boşluk kuralı) → yeniden bağlanma → REST onarımı; `/live/stats|drop|quit`; `--live` ile `--lan`/`--https-host` reddi. |
| perf (M24) | `qp-session` headparts, `qp demo` | Takvim önbelleği; canlıda görüntüleyici yeniden hesabı ≤4/sn (100 sembolde işlemci %74 → %17). |
| L1–L7 | `rounds/R6/experiments/LIVE-LOG.md` | Canlı denemeler (yalnız orkestratör, sırayla). |
| T8 | `docs/matriks/CONNECTIVITY.md` §8, `MESSAGES.md` §10, `LIVE.md` | RI1 sorularının canlı cevapları; bağlayıcı belgesi. |

Alt ajanlar sorunsuz başlatıldı (deneme alt ajanı ilk iş). Her alt görev kendi dalında / worktree'sinde;
birleştirmeden önce fmt + clippy + test yeşil; main'de tam `QP_VERIFY_BROWSER=1 scripts/verify.sh`
18/18 (590f3c7). Denetim ayrı alt ajanda (`AUDIT.md`).

## 3. Kabul ölçütleri

| # | Durum | Kanıt |
|---|---|---|
| 1 | ✅ | Süreç içi tek bağlantı (`second_open_in_same_process_is_refused`), makine kilidi iki süreçle (`tests/two_process.rs`: ikincisi PID'li nedenle başlamaz), çoklu bağlantı bildiriminde dur + yeniden deneme yok (`multiconnect_notice_halts_without_retry_until_reset`), kimlik `qapsule-<makine adı>` belirleyici (`config::tests`), maskeleme (`no_secret_reaches_any_log_or_error_text`). **Round boyunca çoklu bağlantı bildirimi: 0.** Kimlik değişkeni Arkin tarafından dolu ve `qapsule-` ile başlamıyor (her denemede uyarı; A51). |
| 2 | ✅ | 22 proto bayt eşit (`tests/provenance.rs`; kaynakla karşılaştırma yerelde, CI'da atlanır ve söyler); tipler derlemede üretiliyor, elle yazılmış mesaj tipi yok; alan varlığı testleri (`tests/presence.rs`, VERIFY 7); ilk durum + güncelleme; CI ağsız. |
| 3 | ✅ | Gecikmeli uçta `mx/timestamp` reddediliyor (ölçüldü); saat girdisi `updateDate` borsa zamanı + XIST takvimi + sembol başına `stockStatus`. Kapsül dili ve kabuk satırı (T3; VERIFY 11 testi `crates/qp-session/tests/market.rs`). **Canlıda gözlendi** (L6): borsa saati 18:00'ı geçince ekran "Seans kapalı" dedi; kapanış evreleri 5→3→7→27→1→26 her sembolde geldi. Bayat veri canlı gösterilmiyor (M23, aşağıda). Not: kapanış emir toplamada genel satır "kapalı" diyor (A48). |
| 4 | ⚠️ | M22 ölçüldü (§4), 100 sembol, 1m/5m/1h/1d, üç yöntem, iki hacim alanı; farklar açıklandı. **Tam seans değil:** en uzun kesintisiz kayıt borsa saatiyle ~16:08 → kapanış (L6; round 13:48'de başladı). Günlük bar kart alanlarından olduğu için tam günü kapsıyor. Tam seans için komut "Arkin'den beklenenler"de. |
| 5 | ✅ | Ek yeri: geçmiş (REST, gecikmeye göre kesilmiş, düzeltilmemiş) + `LiveFeed::seed`; birleşik seri tek seferde canlı kurulanla bit eşit, EMA/RSI ek yerinde akışkan = toplu (`crates/qp-matriks/tests/feed.rs`); `livebar` 9 kesim noktası × periyot × yöntem. Hacim birimi TL ve düzeltilmemiş seçimi ölçüldü ve belgelendi (`MESSAGES.md` §10.1, Ö72). |
| 6 | ✅ | M23 (§5): bilerek iki kesme + bir yerel ağ kopması; bayatken kapsül 0; onarım sonrası tamamlanmış barlar REST'ten; geri çekilme ölçülü (2 sn, kopma sonrası); tek bağlantı korundu (`local_drop_on_mock_broker_keeps_one_connection_and_repairs`: `max_concurrent == 1`); belirteç yenileme testi sahte sunucuda (`token_renewal_closes_old_connection_before_opening_new`). Canlıda belirteç süresi dolmadı (A50). |
| 7 | ✅ | `qp replay --raw`: aynı kayıt iki kez bayt eşit (gerçek 4 kapalı kayıtta 921–46.514 satır; test `tests/replay_raw.rs`); canlı yol = oynatma (`replay_raw_equals_the_live_path`); kayıtlar yalnız `fixtures/local/live/raw/` (86 MB). |
| 8 | ✅ | Gerçek sembollerle canlı kapsül akışı (L3–L6). L6 özeti (veri içermez): 2 sa 13 dk, 496.113 ileti, 110.294 veri olayı, 2.295 kesin bar, **761 canlı kapsül olayı** (son 200'ün dağılımı: 129 doğum, 68 geçersizleşme, 3 süre dolumu), bayatken 0. Cihaz görüntüleyici: akışta bar aboneliği yok, açık kapsülde yalnız o enstrüman (`bind`, K48). |
| 9 | ✅ | §6 tablosu; sınıra ulaşılmadı (580/580). |
| 10 | ✅ | `CONNECTIVITY.md` §8, `MESSAGES.md` §10 (soru → cevap → nasıl gözlendi); cevaplanamayanlar listeli (çoklu bağlantının anahtarı, `TimeMessage` birimi, belirteç ömrü, anlık kip). |
| 11 | ✅ | `crates/qp-compute/tests/address.rs`: `RSI:14`, `ISCTR:5m:EMA[RSI:18]:20`, `SMA[EMA[RSI:14]:10]:5`, üç düzey; doğrudan zincirle `to_bits` eşit (toplu, akışkan, son barı düzelt, tarama). |
| 12 | ✅ | Gösterim varsayılanları `korda` + `live_new`; sözlük güncel. |
| 13 | ⚠️→ | `verify.sh` 18/18 main'de; hesap özeti `7b1e99d40889f83c` aynı; altınlar ve 64 döküm bayt eşit; gizli tarama denetimde. WASM: bağlayıcı bağımlılıklarından hiçbiri pakette yok; temel paket ham boyut aynı (2.280.681 B, gzip +13 B), gösterim (replica) paketi +72,7 KB ham / +16,1 KB gzip (+%0,77; T3'ün piyasa durumu metni). CI sonucu §9'da. |
| 14 | ✅ | Bu rapor, `AUDIT.md`, `docs/matriks/LIVE.md`, `STATUS.md`, `ROADMAP.md`, önerilen kararlar Ö72–Ö76 / açık A47–A51 (`DECISIONS.md`), disk hijyeni (§10). |

## 4. M22 — canlı bar ↔ REST (L6 kaydı, 100 sembol, borsa saati ~16:08 → kapanış)

`qp live compare --raw fixtures/local/live/raw/<L6>.qpraw` (REST `divide=false`; fiyat toleransı 1e-6
mutlak, hacim %1 göreli; zaman `updateDate`). "full" = abonelikten sonra açılıp kapanan barlar;
"first" = abonelik anındaki kısmi bar; günlükte tek bar "first"tir.

| Periyot | Yöntem | Bar | Tümü tam eşit | Tümü toleransta | o / h / l / c tam % | v tam % | v ≤ %1 % | En büyük göreli fark (fiyat) |
|---|---|---|---|---|---|---|---|---|
| 1m | ticks | 10.147 | %14,8 | %83,1 | 91,6 / 94,6 / 94,0 / 94,2 | 15,3 | 95,3 | 1,3e-2 |
| 5m | ticks | 2.074 | %12,1 | %81,5 | 91,5 / 94,4 / 93,4 / 94,6 | 13,1 | 97,1 | 1,3e-2 |
| 5m | mixed | 2.074 | %12,1 | %81,8 | 91,5 / 94,5 / 93,8 / 94,6 | 13,1 | 97,1 | 1,3e-2 |
| 1h | ticks/mixed (kapanış dahil son bar) | 100 | %12,0 | %86,0 | 93 / 97 / 97 / 96 | 12 | 100 | 2,6e-3 |
| 1d | ticks (gün ortası başlangıç) | 100 | %0 | %0 | 8 / 9 / 39 / 96 | 0 | 0 | 7,3e-2 |
| **1d** | **card / mixed** | **100** | **%90** | **%90** | **98 / 95 / 95 / 96** | **100** | **100** | **7,8e-8** |

Hacim alanı `dailyVolume` ile `volume` bu kayıtta aynı sonucu verdi. Seans dışı tik 0, geç tik 0,
hacim geriye gitmesi 0. L2d kaydı (580 sembol, 31 dk, 100'ü REST'li) gün içinde aynı oranları verdi.

**Farkların açıklaması.** (1) Fiyat kartı bir işlem akışı değil, birleştirilmiş anlık görüntü akışıdır:
iki kart güncellemesi arasında birden çok işlem olabilir; kart yalnız sonuncusunun `last`'ını taşır,
bu yüzden bar içi tepe/dip ve açılış ara sıra görünmez (fiyat %5–9 fark, en çok %1,3). (2) Hacim:
birikimli tutarın farkı doğru toplamı verir, ama bar sınırındaki işlemlerin hangi bara düştüğü
`updateDate`'in (kart zamanı) işlem zamanından farklı olmasıyla kayıyor; toplam korunuyor (günlük
100/100 tam), bar başına %95–97 ≤ %1. (3) Günlük bar kartın günlük alanlarından tam (kalan 10 sembolün
farkı ondalık gösterim düzeyinde, 8e-8). (4) Kesin gün içi bar için REST (seans sonu ya da kopma
onarımı) kesin kaynak olarak kalır; tam işlem akışı (`mx/trade`) lisans kararından sonra.

**Seçim önerisi (Ö72):** (c) karma — günlük kart alanlarından, gün içi tiklerden; hacim TL,
düzeltilmemiş.

## 5. M23 — kopma, bayat, onarım (L3, 30 sembol, 5m)

| Kesme | Bayat süre | Onarım (REST, 30 besleme) | Patch / yeni bar | Bayatken kapsül | Not |
|---|---|---|---|---|---|
| Bilerek 20 sn | 40,6 s | 20,4 s | 0 / 0 | 0 | Aynı 5 dk bar içinde |
| Bilerek 330 sn | 350,6 s | 20,4 s | 30 / 30 | 0 | Bar sınırını aşan |
| Planlanmamış (yerel ağ, `os error 49`) | 22,6 s | 20,5 s | 52 / 0 | 0 | 2 sn geri çekilmeyle döndü |

Yeniden abonelikte ilk durum 30/30 geldi. Onarım süresini REST'in sembol başına 0,7 sn aralığı
belirliyor (580 sembolde ~7 dk olur; Öneriler). Kesilme anında süren bar onarılmaz (gecikmeli kipte REST
"gelecek" veridir); kesinleştiğinde REST'ten düzeltilir.

## 6. M24 ve yük (gösterim, gecikmeli kip, 5m, bir görüntüleyici açık)

| Sembol | Ölçüm | İleti/sn | Süreç işlemci (kararlı) | RSS | Tik → çerçeve p50 / p95 | Abonelik |
|---|---|---|---|---|---|---|
| 3 / 30 / 100 / 580 | kayıtçı (`qp live record`, L1–L2c) | ~6 / ~25 / ~50 / ~175 | 300 sn'de 0,5 / 0,77 sn (100 / 580) | 16,5 / 17,0 MB | — | 3 / 29 / 100 / 580 kabul; 100 sembol ~4 sn |
| 30 | gösterim L3 (önce, onarımlar dahil) | ~25 | %17 | 93 MB | 13,4 / 5.468 ms (onarım tamponu) | 30 |
| 100 | gösterim L4 (önce) | ~50 | %82 | 173 MB | 84 / 151 ms | 100 |
| 100 | gösterim L4c (perf sonrası) | ~71–76 | **%17** | 142 MB | **0,27 / 51 ms** | 100 |
| 100 | gösterim L6 (perf sonrası, 2 sa 13 dk) | ~62 | — | 214 MB | 0,28 / 57 ms | 100 |
| 580 | gösterim L5 (perf sonrası) | ~146–152 | %69 | 864–897 MB | 159 / 835 ms | 579/580 (1 sembolün REST geçmişi yok) |

**Gözlenen abonelik sınırı:** yok (580 sembol, tek bağlantı). Profil (L4): işlemcinin çoğu her veri
partisinden sonra bütün görüntüleyici kapsül cümlelerinin yeniden kurulmasıydı; bildirim birleştirme
(≤4/sn) ve takvim önbelleğiyle 100 sembolde %74 → %17. 580 sembolde bellek (geçmiş + kapsül dokümanı)
ve gecikme belirgin; bir sonraki adım dilim ve yüz hesabını artımlı yapmak (Öneriler).

## 7. RI1 açık soruları

`docs/matriks/CONNECTIVITY.md` §8 ve `MESSAGES.md` §10. Cevaplananlar: yeniden abonelikte ilk durum
(evet), `updateDate` biçimi/kökeni (borsa zamanı, `+03`, ms; gecikme 900 s), hacim alanları
(`dailyVolume` = TL = REST; `incrementalQuantity` hiç gelmiyor), `stockStatus` güncellemede de geliyor,
abonelik sınırı (580'de yok), eksiz konu (gecikmeli uçta ret), `mx/timestamp` ve `mx/messenger` market
ucunda ret, işlem görmeyen sembol 0x80, TLS doğrulaması açık çalışıyor. Cevaplanamayanlar: çoklu
bağlantının anahtarı (sınamak kurala aykırı), `TimeMessage` birimi/sıklığı (gecikmeli uçta yok),
belirteç ömrü ve süresi dolunca davranış (A50), anlık kipteki her şey (A49), `volume`/`dailyVolume`
ayrışması (A47).

## 8. Ne yapılamadı

- Tam seans kaydı (ölçüt 4): round 13:48'de başladı. Komut aşağıda.
- Anlık kip: gecikmeli kipte bütün denemeler geçti; anlık kip için `.env.local`'da
  `MATRIKS_MQTT_MODE=realtime` yapmanı bekliyorum (ajan değiştirmez).
- Akış ailesi (lisanslı) canlıda veri bulmuyor (kapsam dışı; sentetik akış canlı gösterime karışmıyor).

## 9. Açık sorular ve önerilen kararlar

`DECISIONS.md` "R6 önerileri": Ö72 (M22 karma), Ö73 (M23 bayat + yeniden abonelik + REST onarımı),
Ö74 (sembol başına abonelik, tek bağlantı; görüntüleyici ≤4/sn), Ö75 (bağlayıcı kitaplıkları), Ö76
(gecikmeli kipte saat kaynağı). Açık: A47–A51.

## 10. Öneriler (kapsam dışı, yapılmadı)

- Görüntüleyici dilimi ve yüzleri artımlı hesaplamak (580 sembolde işlemci %69, RSS ~0,9 GB).
- Onarımda REST'i yalnız etkilenen sembollerde ve paralel değil ama toplu (çok sembollü uç varsa) çekmek;
  geçmişi önbelleğe almak (`--live-bars`).
- `LiveFeed::reseed` (onarımda yeni hat kurmadan sayaç tabanını taşımak); `seed_with_counter` ile ek
  yerinde günün sayacı.
- Kapanış emir toplamada genel satırın "kapanış seansında" demesi (A48).
- `live-<uç>.halted` için insan onaylı sıfırlama komutu.
- `scripts/qapsule-gh.env` izlenmesi (eski öneri) korunuyor.

## 11. Disk

<!-- clean.sh sonrası doldurulur -->

## 12. Arkin için: canlı gösterimi nasıl açarım

1. `.env.local`'da canlı değişkenler dolu olsun (`scripts/live-env.sh` hangisinin eksik olduğunu
   söyler, değer yazmaz). Gecikmeli kip: `MATRIKS_MQTT_MODE=delayed`.
2. Aynı makinede başka bir Qapsule canlı süreci çalışmasın (kilit zaten reddeder).
3. Depo kökünden:
   ```sh
   scripts/demo.sh --live                                   # BIST30, 5 dk bar, 120 bar geçmiş
   scripts/demo.sh --live --symbols @fixtures/local/live/u100.txt --interval 15m
   ```
   İlk açılışta evren listesi yoksa: `scripts/live.sh symbols` (`u30/u100/utum.txt` yazar).
   Açılışta REST geçmişi sembol başına ~0,7 sn sürer (30 sembol ~20 sn).
4. Tarayıcıda `http://localhost:8390/`. Üst satır "Canlı · gecikmeli 15 dk" / "Seans kapalı" /
   "Bağlantı yok — veri bayat" der. Telefon için `--lan` **reddedilir** (gerçek veri yerel ağa açılmaz);
   Tailscale ile açmak istersen kendi vekilini kurman gerekir.
5. Konsolda basılan denetim sırrıyla: `curl -s localhost:8390/live/stats` (sayılar),
   `curl -s -X POST -H "X-Qp-Live-Control: <sır>" 'localhost:8390/live/drop?secs=20'` (kopma denemesi).
   Ctrl-C bağlantıyı düzgün kapatır.
6. "HALTED (multi-connect)" görürsen yeniden deneme; bana haber ver.
7. **Tam seans M22 (ölçüt 4) için** bir iş gününde 09:50'de başlat, 18:40'ta kapat:
   ```sh
   scripts/live.sh record --symbols @fixtures/local/live/u30.txt --seconds 31800
   scripts/live.sh bars --symbols @fixtures/local/live/u30.txt --period 1min --start <gün> --unadjusted   # 1min, 5min, 1hour, 1day için tekrarla
   qp live compare --raw fixtures/local/live/raw/<kayıt>.qpraw
   ```
8. **Anlık kip** için `.env.local`'da `MATRIKS_MQTT_MODE=realtime` yap ve haber ver; aynı denemeleri
   (L1–L6) anlık uçta tekrarlarım.
