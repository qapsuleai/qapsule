# RP4 — Bağımsız denetim

Denetçi: bağımsız alt ajan (işi yapmadı). Tarih: 2026-11-21. Dal `rp4/audit` (main `92ad1d4`'ten),
worktree `.claude/worktrees/rp4-audit`, `CARGO_TARGET_DIR=target/wt/audit`.
**Ağa çıkılmadı:** canlı bağlantı, REST, belirteç isteği yok; `.env.local` açılmadı;
`scripts/live.sh` ağ komutları çalıştırılmadı. `quantex` yalnız `git status`/`rev-parse` ile okundu.
Yeniden üretimler ana ağacın kökünden, worktree'de derlenen ikililerle; çıktılar yalnız
`fixtures/local/audit/` altına yazıldı ve iş bitince silindi. Bu dosyada gerçek veri, sembol
adı ya da fiyat yok; yalnız toplu sayılar.

Önem dereceleri: **Engelleyici** (kabulü durdurur), **Koşul** (kapanıştan önce düzeltilmeli),
**Bilgi** (kayda geçsin).

## Özet

| # | Konu | Sonuç |
|---|---|---|
| 1 | Geleceğe bakma (commit sırası, kod, NOTES) | **Temiz.** SPLIT → SPEC → düzenek → alternatifler → seçim → sınama sırası git'te ve dosya zamanlarında tutuyor; kalibrasyon kipi sınama barlarını okumuyor (kod + test); NOTES'larda sınama sayısı yok |
| 2 | Ölçümlerin önbellekten yeniden üretimi | M25/M26/cases **bayt eşit**; `rp4` = `yukdus` (gün içi) ve `rp4` = `r3` (günlük) gösterildi; benzetim iki koşu bayt eşit. **M27 tablosunun 1 gün satırı artık raporla aynı değil** (K1) |
| 3 | Benzetim yolu = canlı yol | Test geçiyor; kod incelemesi iddiayı destekliyor |
| 4 | Düzey 2 | Temiz (işlem `@lvl2` yerelde reddedilir, `trade_bs` çağrılmaz, işlem ucunda her yük temizlenir) |
| 5 | Sürümlü sabitler, verify | Varsayılan `r3`; hesap özeti `7b1e99d40889f83c`; `QP_VERIFY_BROWSER=1 scripts/verify.sh` 18/18 PASS |
| 6 | A52, sıfırlama, geç katılan, `--lan` | Testler yeşil |
| 7 | Gizli bilgi / gerçek veri | Bulgu yok; `git ls-files fixtures/local` 0 |
| 8 | Ağ disiplini | İstek günlükleri LIVE-LOG toplamıyla birebir (1.933); canlı deneme yok |
| 9 | `quantex` | Round başındakiyle aynı |
| 10 | Raporun dürüstlüğü | Hedef sayıları tutuyor; iki koşul (K1 M27 günlük, K2 A53'te sınama verisinin kullanımı), birkaç bilgi notu |

**Karar: koşullu kabul** (koşullar sonda).

## 1. Geleceğe bakma (ölçüt 15, VERIFY 3–4)

**Commit sırası** (`git log --follow --format='%h %ci %s'`):

| Ne | Commit | Zaman (+03) |
|---|---|---|
| `SPLIT.md` (tek commit; `--follow` başka değişiklik göstermiyor) | `cc476de` | 11-20 23:35:18 |
| `harness/SPEC.md` (tek commit, sonradan değişmedi) | `eb1a8c3` | 11-21 12:50:56 |
| Düzenek kodu (`a204dad`), main'e birleşme `8c1cfd3` | `a204dad` | 13:12:43 |
| Alternatif dalları: `m25d-olcek` 13:43:38, `m26b-olcek` 13:44:03, `m25b-teyit2` 13:44:15, `m26c-mevsim` 13:44:22, `m25a-taban` 13:44:44, `m25c-yukdus` 13:44:48, `m25e-savunu` 13:45:34 | her biri tek commit | 13:43–13:45 |
| Seçim (`M25/RESULTS.md`, `M26/RESULTS.md`) | `8bb7571` | 13:52:14 |
| Sınama (`S/RESULTS.md`) | `a8d0ac6`, `3213a7e` | 15:19:57, 15:20:14 |

- Yedi alternatif dalının hepsi `cc476de` ve `eb1a8c3`'ü içeriyor, `8bb7571`'i içermiyor
  (`git merge-base --is-ancestor`). Her dal düzenekte yalnız `src/registry.rs`'e bir satır ve
  `Cargo.toml`/`Cargo.lock`'a path bağımlılığı ekliyor; geri kalanı kendi klasörü.
- Seçimden sonra düzenekte değişen yalnız eklemeler: `rp4` kaydı (`ba86722`), `cases`
  (`e874864`), `births` (`7346fb9`). Ölçü kodu (`m25.rs`) en son `59e9267`'de (`--reference`,
  13:51, seçimden önce) değişti; `r3` kalibrasyon sonucum bu değişiklikten önce yazılmış
  `fixtures/local/rp4/results/M25/r3.5m.calib.json` ile bayt eşit, demek ki değişiklik ölçüleri
  oynatmadı.
- **Dosya zamanları:** bütün `*.test.json` (yerel önbellek ve oturum klasörleri tarandı) en erken
  13:53:08'de doğmuş, seçim commit'inden (13:52:14) sonra. `fixtures/local/rp4/full/`'ün ctime'ı
  13:53:02 (alt klasörlerinki 13:28:00): raporun "tam klasör bu evrede `chmod 000`" anlatımıyla
  uyumlu (izin seçimden hemen sonra geri açılmış). İzinlerin o sırada gerçekten 000 olduğu
  geriye dönük kanıtlanamaz (Bilgi B1).
- **Kod:** `harness/src/data.rs` `load_for(Split::Calib)` → `read_series(.., Some(cut))`:
  okuyucu kesimdeki ilk barda okumayı bırakır, sonraki kayıtlar çözülmez. `m25`, `m26`,
  `cases` yalnız `load_for` üzerinden okur (dosya okuyan başka yol: `prepare` REST girdisi,
  `report` sonuç JSON'u, `ticks` ham kayıt). Testler (`cargo test --release` düzenekte: 15 + 4
  geçti, 0 kaldı):
  `calibration_copy_has_no_test_bars`, `reader_stops_at_cut` (kesimden sonra bozuk kayıt
  okunmuyor), `m25_deterministic_and_blind_to_test_bars` (sınama barları değiştirilince
  kalibrasyon çıktısı bayt eşit kalıyor, sınama çıktısı değişiyor).
- **NOTES taraması** (`grep -iE 'test|sınama|2026-1[01]-(19..31)|2026-11'`): beş M25 ve iki
  M26 NOTES'u "`--split test` çalıştırılmadı" diyor; sınama aralığından sayı yok (geçen "sınama"
  sözcükleri "sınamada denetlenmeli" türü ileriye dönük notlar). Alternatiflerin bütün CSV
  satırları (544) `split = calib`.

## 2. Ölçümlerin yeniden üretimi (VERIFY 6, 7, 9, 10, 12)

`harness` (`cargo build --release`, worktree) ve `qp` (`-p qp-cli`) ile, ana ağaç kökünden:

| Komut | Sonuç | Raporla |
|---|---|---|
| `m25 --data rp4/calib --rule r3 --period 5m` | n 383.867; ECR 0,148; SENS 0,212; p 0,415/0,467/0,118 | **bayt eşit** (`rp4/results`, `rp4/eval`) |
| `m25 --data rp4/calib --rule rp4 --period 5m` | ECR 0,066; ECR_ref 0,115; SENS 0,099; p 0,342/0,342/0,316 | `yukdus.5m.calib.json` ile kural adı dışında **aynı** (`rp4` = `yukdus` gün içi) |
| `m25 --data rp4/full --rule r3 --period 5m --split test` | n 39.711; ECR 0,156; SENS 0,198; p_exp 0,106 | **bayt eşit** (`rp4/test`) |
| `m25 … --rule rp4 … --split test` | n 35.453; ECR 0,065 (oran 0,42); SENS 0,097; p_exp 0,304; kal. farkı +0,007 [+0,001, +0,013] | **bayt eşit** |
| `m25 … --period 1d --split test`, `r3` ve `rp4` | ikisi kural adı dışında aynı (`rp4` = `r3` günlük) | ikisi de bayt eşit |
| `m26 --data rp4/full --detectors rp4 --period 1h --split test` | — | **bayt eşit** |
| `cases --rule rp4 --period 5m --split test --day 2026-11-20` | 79 → 22; `rp4`: 7 gerçekleşti, 7 tutmadı, 5 süresi doldu, 2 açık, 1 doğmadı | iki dosya **bayt eşit** |
| `cases --rule umut …` (ek) | 22'nin 13'ü gerçekleşti | rapor §5 ile aynı |
| İşlem barlarıyla sınama (`rp4/test-trades`, okundu) | 5 dk `r3` ECR 0,156 / `rp4` 0,065, SENS 0,098 | "işlem barlarında aynı" üç basamakta tutuyor |

**M27, `qp live compare-trades` (varsayılanlar; 168 s):** 30/30 sembol, 34 gün, 44.859.586
işlem, hepsi `timestamp_ns ∈ [0, 1 gün)`. 1 dk / 5 dk / 1 sa satırları (lot ve TL, `mill` ve
`raw`) `M27/RESULTS.md` ana tablosuyla **aynı** (1 dk 464.890 bar + 13.470 yalnız işlem;
5 dk 95.871; 1 sa 8.010). **1 gün satırı farklı:** şimdi lot tümü tam **%99,90** (o/h/l/c
100/100/99,90/100; bir barın düşüğü REST'ten %3,6 göreli farklı), TL ≤ 1e-6 %99,90; raporda
ve RESULTS'ta %88,63 (o/h/l/c 100). Neden: `M27/RESULTS.md` `d7bb9c4` (15:00) ile commit
edildi, sınama günlerinin `trade_ex`'i (D1f, `trades_ex/requests.jsonl` son satırı 15:03) sonra
çekildi; önbellek değişti, tablo güncellenmedi → **K1**.

**Benzetim (VERIFY 7):** `qp sim --start 2026-11-18 --rule r3 --out …` iki kez: özet
`6ee09b68dcbca960…` iki koşuda aynı; olay akışı dosyaları (244.728.931 B) `cmp` ile **bayt
eşit**; 1.764.375 işlem, 9.683 kesin bar, 1.994 doğum, 989 durum değişimi; duvar 40,4 s ve
42,2 s (hedef ≤ 2 dk ✅; denetçinin başka ağır işi yokken). `--rule rp4`: özet
`78ca365ef8280f59…`, 1.802 doğum, 1.012 durum değişimi, 40,5 s. Raporun §9 taslağındaki özetlerle
aynı.

## 3. Benzetim yolu = canlı yol (VERIFY 8)

`cargo test -p qp-cli -- simulation_path_is_the_live_path` yeşil. İnceleme
(`crates/qp-cli/src/sim.rs` `runtime`/`drive`, `sim/tests.rs`): benzetim işlemleri tel biçimine
kodlayıp (`record`) `LiveIn::Trade` olarak aynı `LiveRuntime` + `MatriksFeed::with_trades` +
`host_from_history` zincirine verir; ayrı bir bar/kapsül yolu yok. Test aynı işlemleri sahte
aracının işlem ucundan gerçek bir `LiveConnection` ve `HubSink::for_trades` ile geçirir ve iki
yolun kaydedilmiş veri olaylarını bayt eşit bulur. Farklar yalnız taşıma ve saat kaynağı
(benzetimde `use_sim_clock`, `baseline_capsules`); karşılaştırılan veri olaylarıdır, kapsül
olayları ayrıca karşılaştırılmıyor (Bilgi B2).

## 4. Düzey 2 (VERIFY 5)

- `git grep @lvl2 -- crates scripts shells`: işlem konusunda yalnız testlerde ve reddin
  belgesinde (`sim/tests.rs`, `live_record_trades.rs`, `demo_live.rs`, `live.rs` testleri).
  `qp_live::is_level2_trade_topic` `mx/trade|tradeex/…@lvl2`'yi (yansı içinde de) yerelde
  reddeder; fiyat kartı `mx/symbol/<SYM>@lvl2` (R6'dan beri, AKDE değil) etkilenmez (Bilgi B3).
- `trade_bs`: yalnız `qp-live/src/rest.rs:195` yorumunda; çağrı yok.
- Alıcı/satıcı: REST CSV'de `trade_csv_level1` yalnız düzey 1 sütunları bırakır (test
  `level2_columns_never_survive`); canlıda yalnız varlık sayılır, değer okunmaz;
  `sim.rs` tel iletisinde `buyer/seller: None`.
- Ham kayıt: `RawSink::take_message` → `keepable_payload_on(topic, payload, trade_endpoint)`;
  işlem ucu sink'i (`for_trade_endpoint`, `demo.rs:497`, `live.rs:611`) her yükü konusundan
  bağımsız temizler, çözülemeyen yük **boş** yazılır (`1821a6f`). `HubSink::for_trades`
  bayrağı kendisi koymuyor, çağıranın `for_trade_endpoint` vermesine dayanıyor; iki üretim
  çağrısı veriyor (Bilgi B4).

## 5. Sürümlü sabitler ve verify (VERIFY 1, 13, 21)

- `qp_capsule::engine::DEFAULT_ENGINE = "r3"`; `EngineConfig::named` yalnız `r3`, `rp4`;
  `ENGINES = ["r3","rp4"]`; şemada `init.capsules.engine: Option<String>`.
- `fixtures/compute/digest.txt` = `7b1e99d40889f83c`; `c92b011..HEAD` arasında değişmedi.
- `QP_VERIFY_BROWSER=1 scripts/verify.sh` (worktree, `rp4/audit` = main `92ad1d4`):
  **çıkış 0, `verify: OK`, 18/18 PASS, SKIP yok**, 743 s (fmt, clippy, test 491 s,
  schema, fixtures, golden, views, wasm, hygiene, browser, compute, digest — arm64 ve wasm32
  `7b1e99d40889f83c`, r3 çalışmaları `914fcfdb9f653a6d` —, protocol, conform, replica, replay,
  shell, demo: sahneler altınlarla eşit, v1/v2 sabit, `p4-*` saatli). Ağ kullanılmadı.

## 6. A52, sıfırlama, geç katılan, `--lan` (VERIFY 14, 15, 17)

| Test | Sonuç |
|---|---|
| `qp-live --test live -- halt`: `two_unexplained_server_closes_within_ten_minutes_halt_until_reset`, `unexplained_server_closes_further_apart_do_not_halt`, `multiconnect_notice_halts_without_retry_until_reset`, `other_server_notice_is_reported_and_does_not_halt` ve 3 eski durma testi | 7/7 ok |
| `qp-live --test two_process` (`new_process_is_refused_by_the_unexplained_close_halt_until_reset`) | 3/3 ok |
| `qp-live` `halt::tests` (pencere, yalnız sunucu kapatması, işaret + notsuz sıfırlama reddi) | 4/4 ok |
| `qp-cli` `live::tests::reset_needs_a_note_and_a_person` | ok |
| Geç katılan: `livehost::tests::late_full_replica_has_the_same_series_as_an_early_one`, `…_matches_on_the_matriks_feed`, `tests/demo_live.rs::late_replica_gets_the_live_bars_of_an_early_one` | 3/3 ok |
| `tests/demo_sim.rs::sim_demo_refuses_the_local_network_and_serves_on_loopback` | ok |

Gerçek sunucuda durma davranışı ilk iş gününe bırakıldı (REPORT §13 adım 8); doğru.

## 7. Gizli bilgi ve gerçek veri (VERIFY 18)

`c92b011..HEAD` (181 dosya): eklenen satırlarda URL/IP taraması yalnız `crates.io-index`,
`127.0.0.1`/`localhost`/`0.0.0.0` ve `*.fake-vendor.test`, `wss://host…` test adresleri; JWT
(`eyJ…`), `Bearer`, `token/secret/password = "…"` deseni 0. Gerçek uç nokta adı yok. İkili
dosyalar yalnız `fixtures/golden/demo/p4-*.png` (T7, sentetik demo). Sembol adları: test
etiketleri (`qp-capsule` market testleri, `engine_rp4.rs`, protokol örnekleri), ilk iş günü
komutları ve LIVE-LOG N2–N3'te bir sembolün istek denemesinin satır sayıları — fiyat ya da
seri yok (Bilgi B5). `git ls-files fixtures/local` → 0.

## 8. Ağ disiplini (VERIFY 19)

İstek günlükleri (yalnız sayılar):

| Günlük | Satır | Kırılım | Başarısız |
|---|---|---|---|
| `fixtures/local/trades/requests.jsonl` | 896 | 3×100 pencere + 101 (11-16→20: 100 + N2) — D1a 399 ve N2 1; 495 tek günlük (D1d 55 + 440; 15 tekrar dahil); 1 belirteç reddi (N1) | 1 (403, N1) |
| `fixtures/local/trades_ex/requests.jsonl` | 120 | D1e 30 + D1f 90 | 0 |
| `fixtures/local/live/rest/requests.jsonl` | 917 | D1b 700, D1c 210, N3 4, N1 3 | 4 (N3'te 1×400, N1'de 3×403) |
| **Toplam** | **1.933** | işlem 1.016, bar 917 | **5** |

LIVE-LOG toplamı (1.933; işlem 1.016, bar 917; başarısız 5) **birebir tutuyor**. Küçük
tutarsızlık: N1 "veri isteği 0" diyor, ama günlükte belirteç aşamasında düşen 4 veri isteği
denemesi var (sayıma doğru girmiş); D1d'nin süresi LIVE-LOG'da 13:22–14:05, dosya zamanlarına
göre 13:22–13:29 (Bilgi B6). Canlı deneme yok: `fixtures/local/live/raw/` altında round
başından (11-20 23:30) sonra dosya yok, `*-trade` kilit dosyası hiç oluşmamış, `*.halted` yok.

## 9. `quantex` (VERIFY 20)

`git -C /Users/arkns/LocalGit/quantex status --porcelain`: ` M services/quantex-ai-insights/.env`,
` M services/quantex-news-visual/.env`, `?? services/quantex_indicators_crypto_cpp/`; HEAD
`f3d95aeaa5b8fc4858b3160acf882055fbf47f02`. Round başındaki durumla **aynı** (dosyalar açılmadı).

## 10. Raporun dürüstlüğü

**Hedefler sayılarla:**

| Hedef | Ölçülen | Durum |
|---|---|---|
| 5 dk ECR ≤ ½ (sınama, kalibrasyon bozulmadan) | 0,156 → 0,065 = 0,42× (yeniden üretildi); kal. farkı +0,007 (SPEC sınırı +0,03 içinde), Brier = taban | ✅; "küçük ama gerçek kötüleşme" raporda açık |
| M27 ≥ %99 (gün içi) | 1 dk/5 dk/1 sa lot ve fiyat %99,999–100 (yeniden üretildi) | ✅ |
| 580 sembol: işlemci ≤ %30, RSS ≤ 450 MB, p95 ≤ 150 ms | %11,2, 376–418 MB, 23,2 ms (T6.md) | ✅ (denetçi 21,5 dk'lık başsız tarayıcılı 1× koşuyu yeniden üretmedi; Bilgi B7) |
| Benzetim ≤ 2 dk | denetçi 40–42 s | ✅ |

**§3 ✅'lerin denetimi:** 1, 3, 4, 5, 6, 7, 8, 9, 11, 12 kanıtıyla tutuyor. 2'de 1 gün satırı
güncel değil (K1). 10 "benzetimde" kaydıyla doğru.

**Açıklananlar (yeterli):** alternatif ajanlarının ortak scratchpad'i ve iki ajanın birbirinin
kalibrasyon sonuçlarını görmesi (§2 süreç notu 3; sınama verisi değil — NOTES'lar ve CSV'ler
yalnız kalibrasyon, yukarıda); 15 tekrar istek (§1, LIVE-LOG D1d); ölçek ölçümünün başka
derlemelerle aynı anda alınması ve yan yana önce/sonra yöntemi (§8); `yukdus` SENS'inin
kısmen yapısal olduğu (§5, `M25/RESULTS.md` okunuş 1, A53). Denetçi bu sonuncuyu doğru
buluyor: SENS kararı veren barın **kapanışını** oynatır; `yukdus` yüksek/düşükle karar verdiği
için oynatma fitili aşmadıkça kararı değiştiremez — SPEC'in sırası bu kurala yapısal avantaj
veriyor; seçim yine de önceden yazılı kurala uygun.

**Bulgular:**

- **K1 (Koşul) — M27 günlük satırı güncel değil.** `M27/RESULTS.md` ana tablosu ve REPORT §3
  ölçüt 2 / §4 tablosu 1 gün için %88,63 (o/h/l/c 100) diyor; bugünkü önbellekten
  `qp live compare-trades` %99,90 veriyor ve düşükte bir bar %3,6 farklı. VERIFY 6 ("raporla
  aynı tablo") bu satırda tutmuyor. Düzeltme: D1f sonrası tabloyu (ya da yalnız 1 gün satırını)
  güncel sayıyla yazmak ve kalan tek barı açıklamak; eski satırı "D1f öncesi" diye tutmak.
- **K2 (Koşul) — A53'te sınama verisinden gelen gözlem.** "Arkin'in gördüğü desende `umut`
  daha iyi" (22'nin 13'ü) sınama aralığının son gününden (2026-11-20) ölçüldü. Bu, `yukdus`
  seçimini geçersiz kılmıyor (seçim `8bb7571`'de, sınamadan önce), ama A53'te `umut`'a
  geçilirse o seçim sınama verisine bakılarak yapılmış olur (GOAL: "sınama aralığına bakılarak
  seçilen sabit geçersizdir"). A53 ve REPORT §5'e "`umut`'a geçiş yeni bir, 2026-11-20 sonrası
  sınama aralığında doğrulanmadan ürün sabiti olamaz" notu eklenmeli.
- **B1–B7 (Bilgi)** yukarıda: `chmod 000` geriye dönük kanıtlanamaz (ctime uyumlu); VERIFY 8
  testi veri olaylarını karşılaştırır; fiyat kartı `@lvl2` (AKDE değil) sürüyor;
  `HubSink::for_trades` temizleme bayrağını kendisi koymuyor; sembol adları yalnız
  test/komut/istek sayısında; LIVE-LOG N1 ifadesi ve D1d süresi; ölçek 1× koşusu yeniden
  üretilmedi.
- **B8 (Bilgi) — M26 özet cümlesi.** REPORT §1 "Gün içinde hiçbir dedektör 0,55'e ulaşmıyor"
  kalibrasyon için doğru; sınamada `rp4`'ün 1 sa kırılımı 0,575 [0,554, 0,596] (S/RESULTS
  §4, A55'te "rejim" diye anılıyor). §1'e "kalibrasyonda" sözcüğü eklenirse tam olur.
- **B9 (Bilgi) — §9 taslağı.** "`@lvl2` işlem konusu yalnız ayrıştırıcı testinin girdisinde"
  ifadesi dar: birkaç uçtan uca ret testinde de geçiyor (hepsi test).

## Karar

**Koşullu kabul.** Geleceğe bakma yok (commit sırası, dosya zamanları, kod ve test, NOTES);
bütün M25/M26/cases ölçümleri önbellekten bayt eşit yeniden üretildi; benzetim belirleyici ve
canlı yolun kendisi; düzey 2, ağ disiplini, gizli bilgi ve `quantex` temiz.

Koşullar:
1. **K1:** `M27/RESULTS.md` ve REPORT §3/§4'te 1 gün satırını D1f sonrası yeniden üretilebilir
   sayıyla (%99,90; kalan bir bar açıklamalı) güncelle.
2. **K2:** A53 ve REPORT §5'e `umut` gözleminin sınama aralığından geldiğini ve `umut`'a geçişin
   2026-11-20 sonrası yeni bir sınamayla doğrulanması gerektiğini yaz.

Engelleyici bulgu yok. CI ve preflight (VERIFY 2) ile kapanış temizliği (VERIFY 22) bu
denetimin kapsamında koşulmadı; orkestratörün kapanışta göstermesi gerekir.
