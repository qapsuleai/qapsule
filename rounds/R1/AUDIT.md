# R1 — Bağımsız Denetim (T8)

Denetçi: ayrı alt ajan (işi yapan ajanlardan bağımsız). Hiçbir şey düzeltilmedi; yalnızca ölçüldü.
Asıl depoya bu dosya dışında yazılmadı, commit ve push yapılmadı, `gh` doğrudan çağrılmadı.

## Ortam

| Öğe | Değer |
|-----|-------|
| Tarih | 2026-11-14 |
| Denetlenen commit | `96023c08946ad09655be3661f8d703bee0bd1426` (`main`, "Merge R1/T7: integration and cross-target digest") |
| Klon yeri | `<scratchpad>/audit` — taze `git clone`, `target/` ve `node_modules/` yok; denetim sonunda klonda `git status` temiz |
| Asıl depo | Denetim sırasında orkestratörün commit edilmemiş değişiklikleri vardı (`ROADMAP.md`, `STATUS.md`, `docs/ARCHITECTURE.md`, `docs/CONCEPTS.md`, `rounds/R1/BENCH.md`, yeni `REPORT.md`). Denetim commit edilmiş `96023c0` üzerinedir. |
| Makine | Apple M4 Max (aarch64), macOS 27.0.1; yük ortalaması ölçümler sırasında 4,4–7,1 (başka işler çalışıyordu) |
| rustc / cargo | 1.92.0 (ded5c06cf 2025-12-08) |
| Kurulu hedefler | aarch64-apple-darwin, wasm32-unknown-unknown, x86_64-apple-darwin, x86_64-unknown-linux-gnu |
| node / wasm-pack | v24.3.0 / 0.14.0 (önceden kurulu; Playwright Chromium kullanıcı önbelleğinde) |

## VERIFY.md adımları

| # | Komut / kontrol | Gerçek çıktı (özet) | Sonuç |
|---|-----------------|---------------------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` (klonda) | rc=0, `verify: OK`, 104 s (soğuk `target/`). 11/11 PASS: fmt 1 s, clippy 6 s, test 19 s, schema 41 s (12 şema, deterministik), fixtures 0 s (4 geçerli), golden 0 s (bayt eşit), wasm 10 s (ham 1.325.440 B, gzip -9 445.415 B), hygiene 0 s, browser 12 s (Chromium altınla aynı), compute 0 s (rsi(14), macd geçerli `StudyOutput`), digest 15 s (`arm64, x86_64, wasm32: 7b1e99d40889f83c`). SKIP yok. | PASS |
| 2 | Asıl dizinde `scripts/preflight.sh`; `scripts/qgh run list/view/watch` | preflight: `TAMAM` (hesap `qapsuleai`, yazma izni, commit e-postası). En yeni çalışma `43614904957` başta `in_progress`'ti; `qgh run watch --exit-status` rc=0. `conclusion: success`, `headSha: 96023c0…` (= denetlenen commit). https://github.com/qapsuleai/qapsule/actions/runs/43614904957 . Günlükteki özet: 11/11 PASS, toplam 85 s, `digest PASS 14s x86_64, wasm32: 7b1e99d40889f83c`, golden bayt eşit, browser PASS. İlk çalışma `43611236862` (`35befe0`) de `success`; günlük özeti CI_FIRST_RUN.md ile birebir aynı (175 s, ham wasm 1.017.631 B). | PASS |
| 3 | `cargo test -p qp-compute` | 17 test geçti / 0 başarısız (chain 2, hand 7, legacy 1, modes 3, sweep 4). Özellik testleri sabit tohumlu: `RngSeed::Fixed(0x0051_2026)` (512 durum) ve `RngSeed::Fixed(0x0051_2027)` (256 durum), `failure_persistence: None`. | PASS |
| 4 | Bağımsız betik (Python, `math.fsum` iki geçişli; proje kodu kullanılmadı) | `sample.json` (200 bar) kapanışlarından RSI(14) Wilder, Bollinger(20,2) nüfus std, MACD(12,26,9) (SMA tohumlu EMA). `qp compute` ile azami fark: RSI 0; Bollinger middle 9,9e-14, upper/lower 1,5e-12, bandwidth 2,6e-14; MACD 1,4e-14, signal/histogram 8,0e-15. `null` konumları da birebir aynı. Ek zorlayıcı seri (2.000 bar, fiyat ≈ 50.000, düz pencereler): RSI ve MACD 0 fark, Bollinger upper/lower 7,0e-10 (sınırın altında ama yakın, bkz. Bulgu 5); 343 düz pencerenin hepsinde bant genişliği tam 0. | PASS |
| 5 | `qp compute … --study rsi --param period=14` ve `--study macd` | İkisi de rc=0. RSI: 200 değer, ilk 14 `null` (0–13), `t` 200. MACD: üç çıktı `macd` (25 `null`), `signal` ve `histogram` (33 `null`), hepsi 200 uzunlukta; `warmup` ve `params` alanları var. Hatalı parametreler (14.5, 0, −3, 1e30, fast ≥ slow) anlamlı hatayla reddediliyor. | PASS |
| 6 | Toplu / akışkan / son barı düzelt | `modes.rs::three_modes_are_bit_identical` (512 durum, beş hesap, her bar önce yanlış fiyatla gelip üç kez düzeltiliyor) ve `qp-source` `live_stream_drives_a_study_to_batch_bits` geçti. `core.rs` okundu: çıktı her zaman `peek` ile, durum yalnızca bir sonraki `push`'ta `commit` ile ilerliyor; `update_last` yalnızca bekleyen girdiyi değiştirip yeniden `peek` yapıyor. Bu yüzden eşitlik yapısal olarak garanti; beş hesabın `peek`/`commit` formülleri satır satır aynı. Kırma denemesi: bkz. Bulgu 3 ve 4. | PASS |
| 7 | `sweep_bench --reps 15` ve `scripts/wasm-sweep-bench.sh` | Aşağıdaki tabloda. Hepsi BENCH.md'nin ±%20'si içinde (en büyük sapma yerel EMA +%15). M4 Max'te tüm hedefler tutuyor; özetler BENCH.md ile aynı. **Ancak CI'daki gerçek Linux x86_64'te hedefler tutmuyor** (Bulgu 1). | PASS (yeniden üretim) |
| 8 | `cargo test -p qp-calendar` | 12 test geçti (calendar 9, tzdata 3; jiff'e karşı 1900–2100 her geçiş ve 6 saatte bir). Bağımsız kontrol (Python `zoneinfo`): NYSE 2026-03-06 14:30–21:00 UTC, 2026-03-09 **13:30**–20:00 UTC (EDT), 2026-10-30 13:30, 2026-11-02 14:30; 2026-11-27 yarım gün kapanışı 18:00 UTC; İstanbul 2015-11-02 10:00 → 07:00 UTC, 2015-11-09 → 08:00 UTC (2015 ertelenen yaz saati). Testteki beklenen değerlerle aynı. | PASS |
| 9 | Mimariler arası özet | `fixtures/compute/digest.txt` = `7b1e99d40889f83c`. aarch64 (`digest` örneği) aynı; `--target x86_64-apple-darwin` (Mach-O x86_64, Rosetta) aynı; wasm32 verify adım 11'de aynı; CI'da Linux x86_64 ve wasm32 aynı. | PASS |
| 10 | `grep -rn "struct Bar" crates/` | Tek satır: `crates/qp-schema/src/types.rs:28`. (Not: T7, `qp-calendar`'daki `BarSpan`'ı `Span` olarak yeniden adlandırdı; bkz. Bulgu 8.) | PASS |
| 11 | `grep -rn -i "sma" crates/qp-render/src` | Yalnızca sabitler (`OVERLAY_STUDY = "sma"`, `OVERLAY_LABEL = "SMA20"`) ve yorumlar. `scene.rs:153 overlay_values` → `qp_compute::compute_bars("sma", …)`; çizicide hesap döngüsü yok. | PASS |
| 12 | `git diff --stat d052f2d HEAD -- fixtures/golden` | Boş. verify `golden` ve `browser` adımları da bayt / 0 px eşit. | PASS |
| 13 | Gizli bilgi ve eski klasörler | `git ls-files \| grep -i -E '\.env$\|token\|secret'` → boş. `git grep` (Bearer + değer, `eyJ…` JWT, `password/api_key/secret = "…"`, `ghp_`, `github_pat_`) → yalnızca `verify.sh`'deki hata mesajı. `token/password` geçen dosyalar belge ve betik (`qgh` belirteci çalışma anında `gh auth token` ile alıyor, saklamıyor). `scripts/qapsule-gh.env` `.gitignore:11 *.env` ile dışlanmış, izlenmiyor (içeriği okunmadı). `find ../Services ../Docs -newer <2026-11-14 18:37:36>` → **0 dosya** (`target/` dahil). `.env` dosyası açılmadı. | PASS |

### Ölçüm karşılaştırması (adım 7, medyan ms)

| İndikatör | Yerel (denetçi) | Yerel (BENCH) | Fark | WASM (denetçi) | WASM (BENCH) | Fark | CI Linux x86_64 yerel / WASM (Node 22) |
|-----------|---:|---:|---:|---:|---:|---:|---|
| SMA | 14,9 | 14,2 | +%5 | 16,3 | 16,4 | −%1 | 17,5 / 26,9 |
| EMA | 9,3 | 8,1 | +%15 | 27,2 | 27,3 | 0 | 21,5 / 37,3 |
| RSI | 22,8 | 22,4 | +%2 | 58,4 | 58,6 | 0 | **62,8** / 118,3 |
| MACD | 41,0 | 41,2 | 0 | 81,0 | 81,5 | −%1 | **69,4** / **156,6** |
| Bollinger | 45,4 | 44,4 | +%2 | 71,3 | 71,4 | 0 | **72,5** / 120,0 |

Hedef: yerel < 50, WASM < 150. Kalın değerler hedefi aşıyor. Beş özet her koşuda BENCH.md ile aynı.
K16 WASM boyut ölçümü (`tools/tz-size-probe/measure.sh`) yeniden üretildi: gömülü tablo gzip +10.458 B, jiff static +71.274 B, chrono-tz +81.877 B, jiff+tzdb +153.135 B — K16 ile aynı.

## Kabul ölçütleri (GOAL.md)

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | Uzak depo; CI Linux x86_64'te `verify.sh`; altın ve tarayıcı dahil rapor | ✅ | Adım 2: iki çalışma `success`; son çalışma denetlenen commit'te, golden bayt eşit, browser PASS. CI_FIRST_RUN.md sayıları CI günlüğüyle aynı. Fark çıkmadı, K12 güncellemesi gerekmedi. |
| 2 | `qp-render` kendi bar tipini kullanmıyor; `fixtures/bars` şemayla doğrulanıyor; doğrulama `qp-schema`'da | ✅ | Adım 10; `qp-render` `qp_schema::Bar`/`Series` kullanıyor; verify `fixtures` 4 örnek geçerli; `jsonschema` yalnızca `qp-schema`'da (özellik bayrağı arkasında), CLI kütüphaneyi çağırıyor. |
| 3 | Beş `Study`; elle örnek ve eski kodla karşılaştırma (1e-9) | ✅ | `hand.rs` 7 test, `legacy.rs` 17 vaka (>10.000 değer, göreli 1e-9) geçti; denetçinin bağımsız betiği RSI/Bollinger/MACD için ≤ 1,5e-12 (adım 4). Eski kod kâhini kaydedilmiş fixture (`fixtures/oracle/legacy_indicators.json`); denetçi eski kodu yeniden çalıştırmadı. |
| 4 | Özellik testleri: üç kip eşit; uzunluk; ısınma `null`, sonrası sonlu | ✅ | Adım 3 ve 6. Bkz. Bulgu 3 (toplu = akışkan testi yapı gereği kendini doğruluyor) ve Bulgu 4. |
| 5 | Tarama ölçümü 10.000 × 1.000, beş indikatör, yerel ve WASM; hedef; tutmayan varsa neden | ⚠️ | M4 Max'te hepsi tutuyor ve ±%20 içinde yeniden üretildi. CI'daki Linux x86_64'te RSI/MACD/Bollinger yerel, MACD WASM hedefi aşıyor; commit edilmiş BENCH.md "hedeflerin hepsi tutuyor" diyor (Bulgu 1). Orkestratör bunu commit edilmemiş BENCH.md'ye ekliyor. |
| 6 | `SeriesSource` dosya + sentetik; `qp compute` JSON | ✅ | `qp-source` 7 test; adım 5; verify `compute` adımı. |
| 7 | Takvim BIST/NYSE gün içi/günlük/haftalık/aylık; DST; yarım gün; WASM boyut etkisi | ✅ | Adım 8; K16 sayıları yeniden üretildi. K14 hâlâ "Arkin'in onayı bekleniyor" (Bulgu 7). |
| 8 | Çizici SMA'sı `qp-compute`'ten; altın bayt aynı | ✅ | Adım 11–12. |
| 9 | Özet aarch64, x86_64, wasm32'de aynı; test `verify.sh`'de | ✅ | Adım 9; verify adım 11 üç hedefi yerelde, CI'da x86_64 + wasm32'yi sınıyor. |
| 10 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`, `CONCEPTS.md` güncel | ⚠️ (bekliyor) | Bu dosya yazıldı. Denetlenen commit'te `REPORT.md` yok ve `STATUS/ROADMAP/CONCEPTS` R1 boyunca commit edilmemiş; asıl depoda bunlar commit edilmemiş değişiklik olarak hazırlanıyor (orkestratör). |

## Bulgular

1. **Orta — Hız hedefleri gerçek Linux x86_64'te tutmuyor; commit edilmiş BENCH.md bunu söylemiyor.** CI çalışması 43614904957'deki bilgi amaçlı ölçüm: yerel RSI 62,8, MACD 69,4, Bollinger 72,5 ms (> 50); WASM MACD 156,6 ms (> 150, Node 22). BENCH.md (`96023c0`) yalnızca M4 Max ve Rosetta sayılarıyla "Hedeflerin hepsi tutuyor" diyor. Hedef makineye bağlı tanımlanmadığı için ölçüt 5 tam karşılanmış sayılmamalı; şerit sayıları yalnızca M4'te ayarlanmış. Asıl depoda commit edilmemiş BENCH.md bu tabloyu ve nedenleri ekliyor — REPORT'ta açıkça "tutmayan" olarak geçmeli.
2. **Düşük — M4 Max'te bile Bollinger ve MACD sınırda.** Denetçi ölçümünde Bollinger medyan 45,4 ms, en büyük koşu 48,5 ms (BENCH'te 50,3 ms) — yük altında tek koşular hedefi aşabiliyor.
3. **Düşük — "toplu = akışkan" testi yapı gereği kendini doğruluyor.** `qp_compute::run` (toplu) satır satır `push` çağırıyor; ayrı bir toplu yol yok. Bu tasarım tercihi eşitliği garanti ediyor (iyi), ama testin anlamlı kısmı yalnızca "düzelt" ve `sweep` hızlı yollarının (`step_steady`, Bollinger `step1`) tek çalıştırmayla karşılaştırması (`tests/sweep.rs`). Gelecekte gerçek bir vektörel toplu yol eklenirse ayrı test gerekir.
4. **Düşük — `Chain::update_last` kenar durumu ve kapsam.** Birinci hesabın çıktısı düzeltmeyle `Some` → `None` olursa ikinci hesabın bekleyen değeri silinmiyor ve sonraki `push`'ta commit ediliyor; toplu sonuçtan ayrışır. Bugünkü beş hesapta `None` yalnızca bar sayısına bağlı olduğu için tetiklenmiyor. `Chain` de `modes.rs` özellik testinde yok (yalnızca `chain.rs`'de iki örnek test). Değere bağlı `None` üreten bir hesap eklenmeden önce kapatılmalı.
5. **Düşük — 1e-9 mutlak tolerans fiyat ölçeğine bağlı.** ≈ 50.000 fiyatlı 2.000 barlık seride Bollinger bantları iki geçişli referanstan 7,0e-10, orta bant 5,8e-11 sapıyor (kayan toplam sürüklenmesi; BENCH.md'de "orta bant ~1e-13" yalnızca örnek fiyat düzeyi için). Göreli hata ~1e-14 ve kabul edilebilir, ama BTC gibi 1e5 üstü fiyatlarda ve uzun serilerde mutlak 1e-9 aşılabilir. Testler ve belgeler göreli tolerans diliyle yazılmalı.
6. **Düşük — Periyot üst sınırı 1e9.** `Ring::new(period-1)` belleği hemen ayırıyor; yerelde calloc tembel olduğu için zararsız (period=1e9, RSS 3,6 MB), ama wasm32'de 8 GB ayırma başarısız olup modül iptaline yol açar. Sınır, bar sayısına ya da makul bir değere çekilebilir.
7. **Bilgi — K14 hâlâ onay bekliyor** ("Arkin'in onayı bekleniyor"); takvim mekanizması buna dayanıyor.
8. **Bilgi — Yazma alanı sapmaları (küçük).** T3 (`da3f35a`) `crates/qp-wasm`, `.github/workflows` ve `scripts/`'e yazdı (TASKS.md'de T3'ün alanı ölçüm ve BENCH; qp-wasm T7'nin). T7 (`fc56f5e`) T6'nın crate'inde `BarSpan` → `Span` yeniden adlandırması yaptı; bunun doğrulama adımı 10'daki `grep "struct Bar"` eşleşmesini tek satıra indirmek için yapıldığı görülüyor — işlevsel zararı yok ama denetim komutuna göre kod değiştirmek olarak kayda geçmeli.
9. **Bilgi — Sözlük.** "Parametre taraması" (`sweep`), `Interval`, `BarEvent`, `Span` kavramları denetlenen commit'te `docs/CONCEPTS.md`'de yok (commit edilmemiş CONCEPTS değişikliği 3 satır ekliyor; içeriği denetlenmedi).
10. **Bilgi — Mimariler arası özet kapsamı.** `cross_target_digest` beş hesabı (tarama ızgarası + akışkan/düzelt) kapsıyor; takvim ve kaynak kodu kapsamıyor (takvim yalnızca tamsayı aritmetiği, risk düşük). CI'da aarch64 Linux yok; aarch64 yalnızca macOS'ta sınanıyor.
11. **Kural kontrolleri temiz:** `NORTH_STAR.md` `d052f2d..HEAD` arasında değişmemiş. Hesap crate'lerinde `f32`, `mul_add`, `powf`, `exp`, `ln`, `Instant`, `SystemTime`, `rand` yok; sentetik kaynak sabit tohumlu. Depoda elle yazılmış `*.schema.json` veya TS tipi yok; `Bar`, `Series`, `Point`, `StudyOutput`, `Instrument`, `Calendar` yalnızca `qp-schema`'da. `tools/legacy-oracle` `../Services`'e yol bağımlılığıyla bağlı ama çalışma alanının dışında; `../Services`, `../Docs` değişmemiş (adım 13).

## Sonuç

13/13 doğrulama adımı PASS; kabul ölçütlerinden 8'i ✅, 5 (hız hedefleri gerçek Linux x86_64'te tutmuyor, Bulgu 1) ve 10 (REPORT/STATUS/ROADMAP henüz commit edilmemiş) ⚠️. Doğruluk ve bit-eşitlikte hata bulunmadı; R1, Bulgu 1'in REPORT.md'de açıkça "tutmayan hedef" olarak yazılması ve ölçüt 10'un tamamlanması koşuluyla kapanabilir.
