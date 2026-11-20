# R6 — Bağımsız Denetim

Denetçi: bağımsız alt ajan (işi yapan ajanlardan ve orkestratörden ayrı). Tarih: 2026-11-19/20.
Denetlenen: `main` @ `e3a2f90`. **Canlı bağlantı açılmadı**; `.env.local` açılmadı/yüklenmedi;
`quantex` deposunda yalnız okuma. Bu belgede gerçek veri, sembol başına sayı, uç nokta adresi ya da
belirteç yok; yalnız toplu sayılar ve evet/hayır.

## Kapsam ve yöntem

- Okunanlar: `CLAUDE.md`, `rounds/R6/{GOAL,TASKS,VERIFY,REPORT}.md`, `experiments/LIVE-LOG.md`,
  `docs/matriks/LIVE.md`, `CONNECTIVITY.md` §8, `MESSAGES.md` §10, `DECISIONS.md` "R6 önerileri",
  `crates/qp-live` (lock, conn, auth, backoff, rest, mock, config), `crates/qp-matriks` (notice,
  build.rs, proto), `crates/qp-cli/src/{live,livehost,demo}.rs`, `scripts/{demo,live,verify}.sh`.
- Çalıştırılanlar: ayrı worktree `.claude/worktrees/r6-audit` (`main` @ `e3a2f90`),
  `CARGO_TARGET_DIR=target/wt/audit`. Kayıt üzerinde `qp replay --raw` (×2), `qp live compare`,
  `qp live inspect` (ana ağacın `fixtures/local/live/` dosyaları; ağsız, `env -i` ile ortam boş).
  WASM boyutu için `e61928f` geçici worktree'de derlendi (sonra kaldırıldı).
- Çıktılar scratch'e yazıldı ve silindi.

## 1. Koruma kuralları (kod + test)

| İddia | Sınama | Sonuç | Not |
|---|---|---|---|
| Süreç içi tek bağlantı | `lock.rs` `claim_process_slot` (küresel küme); `live.rs::second_open_in_same_process_is_refused`; test koşusunda geçti | doğru | |
| Makine kilidi, iki süreç | `tests/two_process.rs`: test ikili kendini iki çocuk süreç olarak başlatır; biri bağlanır, öbürü PID'li nedenle reddedilir; aracıda `max_concurrent == 1`, reddedilen süreç belirteç istemez | doğru (qp-live düzeyinde) | **Bulgu B1:** kilit dizini CLI'da `<kök>/fixtures/local`; kök `qp live` için cwd'den yukarı `fixtures/`+`Cargo.toml` aranarak, `qp demo --live` için `--root` (varsayılan `.`) ile belirleniyor. Worktree içinden ya da farklı `--root`/cwd ile çalışan iki süreç **farklı kilit dosyaları** alır → makine kilidi atlanabilir. `qp_live::lock::default_lock_dir` worktree'yi ana ağaca çözüyor ama CLI bunu kullanmıyor. `.halted` işareti de aynı dizine bağlı. |
| Kilit bağlantı öncesi tüm ağ işini kapsar | `demo.rs::live_setup`: REST geçmişi (belirteç isteği dahil) **kilitten önce** çekiliyor; kilit `spawn_connection`'da alınıyor | kısmen | **B2:** ikinci bir `qp demo --live` ya da `qp live bars/symbols` (kilitsiz) çalışan bağlantı varken aynı kimlikle yeni belirteç alır. RI1 açık sorusu (yeni belirteç eskisini düşürür mü) cevaplanmadığı için risk. LIVE-LOG'da L4→L4b (3 sn) ve L5→L6 (7 sn) süreç ömürleri bu yüzden çakışıyor (MQTT oturumları çakışmıyor: yeni sürecin ham kaydı REST'ten ~1 dk sonra açılıyor). |
| Çoklu bağlantıda dur, yeniden deneme yok, kalıcı yasak | `conn.rs`: `Notice::MultiConnect` → DISCONNECT, `Halted`, `ban_endpoint` (süreç kümesi + `.halted`), `open` iki kez denetliyor; sıfırlama yalnız boş olmayan notla. Test `multiconnect_notice_halts_without_retry_until_reset` | doğru (bildirim gelirse) | **B3:** sınıflayıcı yalnız `mx/messenger` konusunu okuyor; market ucunda bu konu SUBACK 0x80 ile reddediliyor (L2a; CONNECTIVITY §8 bunu yazıyor) ve CLI varsayılan olarak abone olmuyor. Yani canlıda çoklu bağlantı yalnız "sunucu kapattı" olarak görünür ve **geri çekilme yoluna** düşer (2→60 sn, 6 ardışık hatada dur; 60 sn'den uzun süren oturum sayacı sıfırlar). Kural 3 kodda var ama market ucunda pratikte tetiklenemez; açıklanamayan sunucu kapatmasında durma kuralı yok. |
| Kimlik `qapsule-` önekli, belirleyici | `config.rs::derive_hardware_id` (makine adından, rastgelelik yok) + testler; MQTT istemci kimliği `qp-<fnv32>-<pid>` (rastgele değil) | doğru | Arkin'in değişkeni `qapsule-` ile başlamıyor; her denemede uyarı basıldı (günlüklerde 5/5) — A51. |
| Belirteç bellekte, diske yazılmıyor; istek sıklığı sınırlı | `auth.rs` (`Secret`, `Debug` gizli; ≥30 sn aralık, hatada ikiye katlanır, 5 hatada dur); ham kayıtlarda `eyJ` kalıbı: **0** (11 dosya); günlük/istatistik dosyalarında 0 | doğru | |
| Geri çekilme üst sınırlı, N hatada dur | `backoff.rs` 2 s ×2, üst 60 s, 6 hata; `consecutive_failures_halt_with_deterministic_backoff` | doğru | bkz. B3 (stable_after 60 s) |
| Yeniden bağlanmada aynı anda iki soket yok | `conn.rs` her çıkışta `close_ws` sonra yeni oturum; belirteç yenilemede önce yeni belirteç alınır, eski kapanır; `livehost::spawn_connection` drop'ta `c.close()` (iş parçacığını bekler) sonra `open`; test `local_drop_on_mock_broker_keeps_one_connection_and_repairs` (`max_concurrent == 1`) | doğru | |
| Onarım iş parçacığı / REST yolu | Demo'da REST ve bağlantı **aynı** `AuthClient`'ı paylaşıyor (ek belirteç yok); REST tek ana makine (`api.matriksdata.com`, belgeli), 0,7 sn aralık, yönlendirme ve vekil kapalı | doğru | `qp live bars/symbols` ayrı süreç, kilitsiz (B2). |
| `qp live record` + `qp demo --live` aynı anda | Aynı kökten: aynı kilit → ikincisi reddedilir (demo, REST geçmişini bitirdikten sonra). Farklı kök/worktree'den: iki kilit → ikisi de bağlanır | kısmen | B1, B2 |

## 2. VERIFY 1 — `verify.sh` ve ağ

| İddia | Sınama | Sonuç | Not |
|---|---|---|---|
| `QP_VERIFY_BROWSER=1 scripts/verify.sh` çıkış 0 | Worktree'de çalıştırıldı (koşular: aşağıda §Verify koşuları) | doğru | Son koşu 18/18 PASS, çıkış 0, 883 test / 0 hata (620 s) |
| Hiçbir adım dış ağa çıkmıyor | Sahte sunucular `127.0.0.1:0`'a bağlanıyor (`mock.rs`, `transport.rs` testi); `demo_live` testi `env_clear` + sahte belirteç/aracı + `--live-bars` (yerel dosya); `fake.test`/`example-data.com` adları yalnız ayrıştırma/maskeleme testlerinde (bağlantı yok); REST istemcisini çağıran test yok; tarayıcı adımı sayfanın yalnız kendi kökenine istek yaptığını denetliyor | doğru (kod okumasıyla) | Ağ fiziksel olarak kapatılmadı; değerlendirme koda dayanıyor. |

## 3. VERIFY 3, 4, 5, 7, 8, 10, 11, 12, 17

| # | Test / sınama | Sonuç | Not |
|---|---|---|---|
| 3 | `qp-live/tests/two_process.rs` (verify test adımında geçti) | doğru | B1: CLI düzeyindeki kök farkını kapsamıyor |
| 4 | `multiconnect_notice_halts_without_retry_until_reset` | doğru | B3 |
| 5 | `no_secret_reaches_any_log_or_error_text` (kullanıcı, parola, Basic, `eyJ`, adres, host:port; 401 gövdesi kimlik yansıtsa bile) | doğru | |
| 7 | `qp-matriks/tests/presence.rs` (yalnız bir alan → öteki alanlar değişmez; açık 0 yazılır, eksik yazılmaz) | doğru | |
| 8 | `qp-cli/tests/replay_raw.rs` + gerçek kayıtla elle (aşağıda §5) | doğru | |
| 10 | `livehost::drop_marks_stale_and_repair_matches_settled_bars` (bayat, bayatken bırakılan olay, onarılmış bar = kesin bar, bayatken kapsül 0), `demo_live` uçtan uca | doğru | Benzetim elde kurulmuş veriyle; gerçek kopma ölçümü L3 (aşağıda) |
| 11 | `qp-session/tests/market.rs::closed_market_is_said_in_the_viewer_feed_verify_11`, `stale_data_is_not_shown_as_live` | doğru | |
| 12 | `qp-compute/tests/address.rs` (`RSI:14`, `EMA[RSI:18]:20`, iki ve üç düzey; `to_bits` eşit) | doğru | |
| 17 | `scripts/demo.sh` `--lan`/`--https-host` denetimi betiğin **ilk** işi (derleme, `.env.local` yükleme ve bağlantıdan önce). Betik kopyası depo dışında `env -i` ile: `--live --lan` → çıkış 2, ret iletisi; `--live --https-host x` → çıkış 2. `qp demo` ayrıca `--live` ile `--lan`, `--https-host` ve 127.0.0.1 dışı `--bind`'ı `live_setup`'tan önce reddediyor | doğru | |

## 4. VERIFY 6 — depo taraması

| Tarama | Sonuç |
|---|---|
| `git ls-files fixtures/local` | 0 dosya; `.gitignore` `fixtures/local/` kuralı |
| İzlenen `.qpraw` | 0 |
| İzlenen `.env*` | 0 |
| `.playwright-mcp` | yok (izlenen 0, kökte klasör yok) |
| JWT kalıbı (`eyJ…`) R6'da eklenen satırlarda | 2 — ikisi de test için uydurulmuş (`alg:none` başlıklı sahte belirteç; `auth.rs`, `redact.rs`). Depo genelinde 1 ek eşleşme R2 dökümünde bir imza dizgisi (R6 öncesi; JWT değil) |
| `wss://` adresleri | yalnız `*.fake.test`, `example-data.com` (testte), `localhost`, belge şablonu `wss://…/market`; `docs/MATRIKS_PROTOCOL.md`'deki adres R0'dan (R6 eklemedi) |
| R6'da eklenen `matriksdata.com` adları | yalnız `api.matriksdata.com` (2; `DATA_ACCESS.md`'de belgeli REST) |
| R6'da eklenen IP | `127.0.0.1` ve maskeleme testinde `10.0.0.1` (sahte) |
| R6'da `fixtures/` altında değişen dosya | 0 |
| **Toplam gerçek bulgu** | **0** |

## 5. VERIFY 8 ve 9 — kayıttan yeniden üretim

| İddia | Sınama | Sonuç | Not |
|---|---|---|---|
| Aynı kayıt iki kez → bayt eşit | `qp replay --raw fixtures/local/live/raw/20261119T114037Z.qpraw` iki kez | doğru | 46.514 satır, iki çıktı `cmp` eşit (REPORT'taki üst sınırla aynı satır sayısı) |
| §4 M22 tablosu kayıttan aynı çıkar | `qp live compare --raw …/20261119T132307Z-demo.qpraw` (REST dosyaları yerel; ağsız, 4,6 sn) | doğru | 1m ticks, 5m ticks, 5m mixed, 1h (son bar), 1d ticks, 1d card/mixed satırlarının hepsi REPORT ile basamağına kadar aynı. Raporlanmayan ayrıntılar: 1m'de 4 yalnız-canlı / 5 yalnız-REST bar eşleşmedi; 5m'de 1 sembolün REST dosyası yok; 1h "full" satırı yok (tablo 1h için kapanış dahil son barı veriyor, doğru etiketlenmiş). |

## 6. VERIFY 13–14 — hesap özeti, altınlar, WASM

| İddia | Sınama | Sonuç | Not |
|---|---|---|---|
| Hesap özeti `7b1e99d40889f83c` | `cargo run -q --release -p qp-compute --example digest` → `7b1e99d40889f83c`; verify digest adımı arm64/x86_64/wasm32 aynı | doğru | |
| Altınlar ve dökümler değişmedi | verify golden/views/conform adımları geçti (12 görünüm ×2, 64 döküm) | doğru | |
| WASM'de bağlayıcı bağımlılığı yok | `cargo tree -p qp-wasm --features replica -e normal,build`: 61 crate; `qp-live`, `qp-matriks`, `prost`, `prost-build`, `protox`, `tungstenite`, `rustls`, `ring`, `ureq`, `webpki-roots`, `gethostname` **yok** | doğru | |
| Temel paket boyutu aynı | `--profile wasm`: `e61928f` 2.280.689 B / gzip 745.489 ↔ `main` 2.280.673 B / gzip 745.488 | doğru | Birkaç baytlık fark gömülü yol uzunluğundan (worktree adı); REPORT'taki 2.280.681 de bu aralıkta |
| Gösterim (replica) paketi | Aynı yöntemle (strip, `--features replica`, wasm-bindgen öncesi): 6.352.560 → 6.402.251 B (+49,7 KB, **+%0,78**), gzip 1.783.715 → 1.796.424 (+12,7 KB, +%0,71) | kısmen | REPORT +72,7 KB / +16,1 KB (+%0,77) — mutlak sayılar farklı ölçüm noktasından (wasm-pack çıktısı) olabilir; oran aynı. **VERIFY 13 "değişmedi" beklentisi gösterim paketi için tutmuyor**; artış bağlayıcıdan değil T3 piyasa durumu kodundan. REPORT bunu ⚠️ ile dürüstçe yazmış. |

## 7. VERIFY 16 — `quantex` deposu

`git -C /Users/arkns/LocalGit/quantex status --porcelain` (3 satır) `experiments/quantex-status-before.txt`
(3 satır) ile **bayt eşit**. HEAD `f3d95ae…` = köken kaydındaki `source_head_commit`. **Doğru.**

## 8. VERIFY 19 — proto kopyaları

| Sınama | Sonuç |
|---|---|
| Dosya sayısı: kaynak 22 (yalnız `.proto`), kopya 22, `provenance.json` 22 | eşit |
| Ad kümeleri üçünde aynı | evet |
| SHA-256 ve bayt: kopya = kaynak = kayıt | 22/22, uyumsuzluk 0 |
| Commit edilen blob = çalışma kopyası | 22/22 |
| Proto dosyalarına dokunan commit | 1 (ilk kopya) |
| `.gitattributes` `*.proto -text` (satır sonu dönüşümü yok) | evet |
| Tipler derlemede üretiliyor (`build.rs`: `protox` + `prost-build`), elle yazılmış mesaj tipi | yok |
| Kaynak klasörün son commit'i `b4de9781` 2026-03-23 = kayıt | evet |

**Doğru.**

## 9. VERIFY 15 — LIVE-LOG

| İddia | Sınama | Sonuç | Not |
|---|---|---|---|
| Her canlı deneme kayıtlı | 15 satır; ham kayıt dosyaları 11 (L1, L2a–d, L3, L4, L4b, L4c, L5, L6) zaman damgalarıyla eşleşiyor; L7 0 ileti (dosya yok — kabul edilebilir) | doğru | |
| Eşzamanlı deneme yok | Aralıklar: L4 bitiş 15:58:58 / L4b başlangıç 15:58:55; L5 bitiş 16:21:57 / L6 başlangıç 16:21:50 | kısmen | Süreçler 3 ve 7 sn çakışıyor; MQTT oturumları çakışmıyor (yeni sürecin ham kaydı ~1 dk sonra). Bu sürede yeni süreç belirteç alıp REST çekti (B2). Bildirim gelmedi. |
| Çoklu bağlantı olayları açıklamalı | Bildirim 0; günlüklerde `halted` alanı her özetinde `null` | doğru | **B4:** L3'teki planlanmamış kopma LIVE-LOG'da "sunucu kapattı", REPORT §5 ve CONNECTIVITY §8'de "yerel ağ, `os error 49`" diye geçiyor; saklanan özet yalnız "lost" diyor, nedeni gösteren kayıt yok. CONNECTIVITY §8 S1 "açıklanamayan sunucu kapatması olmadı" diyor. B3 nedeniyle bu ayrım önemli: tutarsızlık giderilmeli. |

## 10. Kabul ölçütleri — REPORT hükümleri

| # | REPORT | Denetim | Not |
|---|---|---|---|
| 1 | ✅ | kısmen | Testler var ve geçiyor. B1 (kilit dizini köke bağlı), B3 (market ucunda bildirim tetiklenemez) kodda kuralın tam karşılanmadığı yerler. |
| 2 | ✅ | doğru | §8 |
| 3 | ✅ | doğru | Test + canlı gözlem (L6 özetinde piyasa durumu, evre sayıları); A48 notu dürüst |
| 4 | ⚠️ | doğru (⚠️ yerinde) | Tam seans değil; kayıt ~16:08 → kapanış. Tablo kayıttan aynen yeniden üretildi. |
| 5 | ✅ | doğru | `feed.rs` testleri; hacim TL ve düzeltilmemiş seçimi belgeli (MESSAGES §10.1) |
| 6 | ✅ | doğru | L3 özetindeki üç onarım (30 besleme, ~20,4 s; patch 0/30/52; yeni bar 0/30/0; bayat 40,6 / 350,6 / 22,6 s) REPORT §5 ile aynı; bayatken kapsül 0. Belirteç yenileme yalnız sahte sunucuda (A50, dürüst). |
| 7 | ✅ | doğru | §5; kayıtlar yalnız `fixtures/local/live/raw/` (85 MB) |
| 8 | ✅ | doğru | L6 özetinden: 2 sa 13 dk, 496.113 ileti, 110.294 olay, 2.295 kesin bar, 761 canlı kapsül olayı, son 200: 129 doğum / 68 geçersizleşme / 3 süre dolumu, bayatken 0 — REPORT ile birebir; raporda yalnız toplu sayı var (veri yok). |
| 9 | ✅ | doğru (küçük sapma) | Durum dosyalarından işlemci oranları (82 / 17,5 / 70 %), p50/p95 ve RSS REPORT ile tutarlı; yalnız L4c RSS dosyada ~150 MB, raporda 142 MB (not). |
| 10 | ✅ | doğru | `qp live inspect` (L2d) ile örneklendi: `incrementalQuantity` 0; `stockStatus` 580 ilk + 529 güncelleme; görülen kodlar {2,3,13,53,55}; `publishReason` 0; `updateDate` tek biçim `…+03` ms'li; varış−updateDate p50 ≈ 900,8 s, p95 ≈ 901,3 s — MESSAGES §10 ile aynı. |
| 11 | ✅ | doğru | §3 |
| 12 | ✅ | doğru | |
| 13 | ⚠️ | kısmen | Gizli tarama temiz (§4); özet/altınlar aynı; WASM §6. REPORT "CI sonucu §9'da" diyor ama §9'da CI yok (CI/gönderim Arkin'de — STATUS'ta doğru yazılmış). |
| 14 | ✅ | kısmen | REPORT §11 "Disk" boş (clean.sh sonrası doldurulacak); `.claude/worktrees/r6-verify` (590f3c7) denetim başında hâlâ duruyordu. TASKS'taki "ham kayıtlar için saklama kuralı" yazılmamış. |

## 11. Belgeler

| İddia | Sınama | Sonuç | Not |
|---|---|---|---|
| `LIVE.md` komutları var, bayraklar doğru | `qp live record|inspect|symbols|bars|compare`, `--symbols --seconds --out --messenger --level --every`, `bars --period --start --end --unadjusted`, `compare --raw --rest-dir --intervals --methods …`, `qp demo --live --symbols --interval --history --live-bars`, `/live/stats|drop|quit` | kısmen | **B5:** `LIVE.md` §3 ve REPORT §12 adım 3 `scripts/live.sh symbols`'ın `u30/u100/utum.txt` yazdığını söylüyor; kod yalnız `u100.txt` ve `uall.txt` yazıyor. `qp demo --live` varsayılanı `@fixtures/local/live/u30.txt` — temiz bir makinede belgelenen yol çalışmaz. REPORT §12 adım 7 `qp live compare` çağırıyor (`qp` PATH'te değil; `scripts/live.sh compare` olmalı). |
| RI1 cevapları kayıtla tutarlı | §10 madde 10 | doğru | B4 tutarsızlığı dışında |

## Verify koşuları

| Koşu | Sonuç |
|---|---|
| 1 (19:0x) | fmt, clippy, test PASS (883 test, 0 hata); sonra `cargo build -p qp-cli --release` kilitte takıldı (başka verify süreci), orkestratör durdurdu |
| 2 | 17/18 PASS; **18 demo FAIL**: `shells/web/tests/gallery.mjs` sayfanın hazır olmasını 120 s bekleyip zaman aşımına düştü. Aynı koşuda test adımı 10.052 s, yük ortalaması ~33 (makine uyku/yük); bu adım `main`'in birleştirme koşusunda (590f3c7) ve R6 değişikliklerinden bağımsız galeri sayfasında |
| 3 (makine uykusuyla yarıda kaldı) | sonuç yok |
| 4 (2026-11-20, yük normal) | **18/18 PASS, çıkış 0**; test 883 geçti / 0 hata; wasm raw 2.280.673 B, gzip 745.488 B; digest arm64 = x86_64 = wasm32 = `7b1e99d40889f83c`; 64 döküm × 3 yol; demo adımı (galeri dahil) 154 s'de geçti. Koşu 2'deki galeri zaman aşımı yükten kaynaklı geçici bir hataydı. |

## Bulgular

| # | Ciddiyet | Bulgu |
|---|---|---|
| B1 | **koşul** | Makine kilidi ve `.halted` işareti çalışma köküne bağlı (`qp live`: cwd'den bulunan kök; `qp demo --live`: `--root`). Worktree'den ya da başka kökten çalışan iki süreç aynı uca bağlanabilir. Öneri: CLI'da `qp_live::lock::default_lock_dir()` (worktree → ana ağaç) ya da kullanıcı başına sabit bir dizin; iki farklı kökten iki süreç testi. |
| B2 | **koşul** | Kilit REST geçmişinden sonra alınıyor; `qp live bars/symbols` kilitsiz. Çalışan bağlantı varken aynı kimlikle yeni belirteç istenebiliyor (RI1 S1 açık). Öneri: kilidi süreç başında (belirteç isteğinden önce) almak; `bars/symbols` için de aynı kilidi ya da en azından "bağlantı varken çalışma" denetimini koymak. |
| B3 | **koşul** | Market ucunda `mx/messenger` reddedildiği için çoklu bağlantı bildirimi ileti olarak gelemez; kod bu durumda geri çekilmeyle yeniden bağlanır (60 sn'den uzun oturumlar sayacı sıfırlar). Kural 3 pratikte "açıklanamayan sunucu kapatması" düzeyinde uygulanmalı ya da en azından ilk sunucu kapatmasında durup insana sormalı; karar Arkin'in. REPORT/LIVE.md'de bu sınır açıkça yazılmalı. |
| B4 | not | L3 planlanmamış kopmasının nedeni belgeler arasında tutarsız ("sunucu kapattı" ↔ "yerel ağ, os error 49"); nedeni gösteren kayıt saklanmamış. Düzeltilmeli. |
| B5 | not | `LIVE.md` §3 / REPORT §12: `symbols` komutunun yazdığı dosyalar yanlış (`u30`, `utum` yazılmıyor; `uall` yazılıyor); `qp live compare` yerine `scripts/live.sh compare`. |
| B6 | not | REPORT §3 ölçüt 13 "CI sonucu §9'da" — §9'da CI yok. L4c RSS 142 ↔ ~150 MB. REPORT §11 Disk boş; `r6-verify` worktree'si duruyordu; ham kayıt saklama kuralı yok. |
| B7 | not | Ortak `CARGO_TARGET_DIR` ve farklı commit'teki worktree'ler: dosya zaman damgası temelli parmak izi yüzünden eski commit'in derleme çıktısı yeni ağaçta yeniden kullanılabiliyor (denetimde `qp-wasm` replica derlemesi bu yüzden hata verdi; kaynaklara `touch` ile giderildi). K56 kuralı için bilinmesi gereken bir tuzak. |

## Hüküm

**Koşullu kabul.**

Engelleyici bulgu yok: `verify.sh` ağ kullanmadan 18/18 yeşil; hesap özeti, altınlar ve dökümler
değişmedi; depoda gizli bilgi, uç nokta adresi ya da gerçek veri yok; protolar bayt eşit; `quantex`
değişmedi; M22 tablosu ve canlı kapsül özeti kayıtlardan aynen yeniden üretildi; round boyunca
çoklu bağlantı bildirimi alınmadı.

Koşullar (bir sonraki canlı denemeden önce kapatılmalı ya da Arkin açıkça kabul etmeli):

1. **B1:** kilit ve `.halted` işaretini çalışma kökünden bağımsız tek bir makine dizinine almak
   (ör. `qp_live::lock::default_lock_dir()`), iki farklı kökten iki süreç testiyle.
2. **B2:** kilidi belirteç/REST isteğinden önce almak; `qp live bars/symbols` da kilide uysun.
3. **B3:** market ucunda çoklu bağlantının ileti olarak gelmeyeceğini belgelere yazmak ve
   açıklanamayan sunucu kapatmasında ne olacağına karar vermek (dur ve sor / sınırlı geri çekilme).
   Karar Arkin'in.
4. B4–B6 belge düzeltmeleri (L3 kopma nedeni, `symbols` çıktı dosyaları, §9 CI atfı, Disk bölümü,
   saklama kuralı) round kapanışında yapılmalı.
