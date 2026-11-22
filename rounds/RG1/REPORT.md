# RG1 — Yönetişim turu: rapor

Durum: **tamamlandı** (2026-11-22). Yeni özellik yok; davranış değişmedi. Karar bekleyenler:
istisna istekleri İ1–İ7 (§4), kural önerileri K-1…K-11 (§9), önerilen kararlar Ö103–Ö108 / A63–A64
(`docs/decisions/rounds/RG1.md`), `recovered/*` dalları (§11).

## 1. Özet

- **Davranış değişmedi.** Başlangıç tutanağının 225 kalemi round sonunda bayt eşit
  (`scripts/rg1-compare.sh`, son commit'te ve her birleştirmeden sonra `main`'de). Kalemler:
  - şema dışa aktarımı;
  - 12 görünümün görseli, açıklaması ve sahnesi;
  - hesap çıktıları ve dört hesap özeti (aarch64, r3, x86_64, wasm32);
  - üç taşımada ve görünümlerde uygunluk dökümleri, ayrıca 82 dökümün `qp serve` üstünden gerçek cevap akışı;
  - dört yansı/görüntüleyici bandı ve yeniden oynatma akışı;
  - dört MCP yüzeyi ve RD1 uyarlayıcısında `initialize` + `tools/list`, ayrıca sabit `tools/call` istekleri;
  - kabuksuz sürüş özetleri ve görünüş kayıtları;
  - komut satırının bütün `--help` ağacı ve `--version`;
  - iki puanlayıcı testi;
  - test adları ve sayısı;
  - kayıtlı bütün fikstürlerin SHA-256 listesi.

  `scripts/verify.sh` 19/19 PASS (yeni adım 19 `arch` dahil). CI yeşil (45035266520).
- **Kurallar depoda.** `scripts/arch-check` (+ `--self-test`, 20 bilinen ihlal ve mandal), borç
  taban çizgisi `scripts/arch-baseline.json` → `docs/DEBT.md` (238 kalem, mandallı), adlı istisnalar
  `scripts/arch-config.json`. Lint'ler açık (`unwrap_used`, `expect_used`, `panic`,
  `too_many_lines` 80, `cognitive_complexity`); yeni ihlal derlemeyi kırıyor.
- **Kod düzenlendi** (taşı, yeniden yazma). 800 satırı aşan kaynak dosya 56 → **0** (adlı iki tek
  fonksiyonlu dosya İ6 ve üretilmiş `advances.rs` dışında). Kabuklar 11,7 / 10,7 bin → **1,4 / 1,3 bin**.
  İki yeni crate: `qp-shellcore`, `qp-host`. `qp-session` konu klasörlerine bölündü.
  `qp-schema/protocol.rs` mesaj ailelerine ayrıldı.
- **Parametre kayıt defteri** `qp_schema::params`: tip, çözümleme, üretilmiş belge, envanter
  (`docs/PARAMETERS.md`, ≈ 130 sabit); 17 sabit kayıt defterinden okunuyor (16 anahtar).
- **`DECISIONS.md` bölündü** (`docs/decisions/`; 352 satır aynen, numaralar ve metinler aynı,
  yeni kırık başvuru 0).

## 2. Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|---|---|---|
| 1 | Başlangıç tutanağı ilk iş, commit | ✅ | `6d03df2` (ilk taşımadan önce), kod taşınmadan genişletildi `26c513d` (225 kalem); `experiments/baseline/` |
| 2 | Davranış değişmedi; verify yeşil; CI yeşil; hiçbir altın güncellenmedi | ✅ | `experiments/compare-final.txt` 225/225; `verify-final.log` 19/19; `git diff --stat 6d03df2 HEAD -- fixtures/{golden,compute,protocol,views}` boş; CI §13 |
| 3 | Performans ve WASM boyutu | ⚠️ İ7 | K17 gürültü içinde (A/B kontrolü); varsayılan WASM +%0,05; **kabuk paketi gzip +%2,5** (> ±%2) → İ7 (§6) |
| 4 | `arch-check` depo genelinde, verify adımı, öz-sınama; katman yönü ihlali 0 | ⚠️ İ1, İ2 | Yukarı bağımlılık: İ1 kabul edilirse 0 (`qp-session → qp-source`). Yan bağımlılık: 5, hepsi adıyla İ2. Kural metni harfiyen uygulanırsa 6 ihlal; saf taşımayla kalkmıyorlar (§4) |
| 5 | 800 satırı geçen kaynak dosya 0 | ✅ (İ6) | Yalnız `ops/expand/op.rs` (tek fonksiyon `expand_op`) ve `compile/layer/prepare.rs` (tek fonksiyon `prepare`) → İ6; `advances.rs` üretilmiş, adıyla listeli |
| 6 | Kabuklar < 4 bin | ✅ | `qp-cli` 1.419, `qp-wasm` 1.324 |
| 7 | 12 bin üstü crate yok ya da istisna; klasörde ≤ 15; yasak ad yok | ✅ (İ3–İ5) | `qp-session` 32,0 bin, `qp-grammar` 19,3 bin, `qp-capsule` 13,2 bin → İ3–İ5; `src/` klasörleri ≤ 15; `tests/` klasörleri 3 tane (M5 kesildi, borçta); yasak ad 0 |
| 8 | Her crate'te README; `lib.rs`/`mod.rs` yalnız bildirim | ✅ | 16/16 README; `root.logic` 0 (kesin kural) |
| 9 | Taban çizgisi ve mandal, testi | ✅ | `docs/DEBT.md`, `scripts/arch-baseline.json`; öz-sınama; `experiments/ratchet.txt` |
| 10 | Lint'ler açık; yeni ihlal derlemeyi kırıyor; mevcutlar taban çizgisinde | ✅ | Kök `Cargo.toml`, `clippy.toml`; 237 öğe izni (`lint.allow` crate başına mandallı); kırılma gösterimi L ve denetçi raporlarında |
| 11 | Parametre kayıt defteri; ≥ 10 sabit; davranış aynı | ✅ | §7 |
| 12 | `DECISIONS.md` bölündü, satır kümesi eşit, başvurular çalışıyor | ✅ | `scripts/rg1-decisions-check.py b341e2d 1957752`: 352 satır eşit; HEAD'de kayıp 0; yeni kırık 0 (önceden kırık 1: A46) |
| 13 | `recovered/*` karar listesi; Fark edilenler; Kural önerileri | ✅ | §11, §10, §9 |
| 14 | Bağımsız denetçi | ✅ | `AUDIT.md`: **koşullu kabul**; tutanağı kendisi yeniden koştu (225/225), 20 rastgele taşıma (15 saf, 5 yalnız görünürlük, 0 mantık), `arch-check`'e 26 hile denedi; bulgular ve cevaplar §15 |
| 15 | Kapanış temizliği; `target/` ≤ 6 GB; rapor, STATUS, ROADMAP | ✅ | §16 |

## 3. Önce ve sonra

**Crate'ler** (test dışı / test satırı; `scripts/arch-check --table`):

| Crate | Katman | Önce | Sonra |
|---|---|---|---|
| `qp-session` | oturum | 31.644 / 15.267 | 31.999 / 15.601 (konu klasörleri) — İ3 |
| `qp-grammar` | alan | 19.966 / 8.924 | 19.330 / 9.909 — İ4 |
| `qp-capsule` | alan | 13.059 / 5.621 | 13.230 / 5.690 — İ5 |
| `qp-cli` | kabuk | 11.653 / 6.131 | **1.419** / 3.516 |
| `qp-wasm` | kabuk | 10.660 / 4.522 | **1.324** / 3.872 |
| `qp-host` (yeni) | uyarlayıcı | — | 10.415 / 2.616 |
| `qp-shellcore` (yeni) | uyarlayıcı | — | 9.503 / 646 |
| `qp-mcp` | uyarlayıcı | 7.929 / 3.110 | 7.177 / 4.044 |
| `qp-schema` | şema | 7.893 / 778 | 8.208 / 1.328 (+ `params`) |
| `qp-live` | uyarlayıcı | 5.081 / 2.130 | 5.104 / 2.130 |
| `qp-compute` | çekirdek | 4.752 / 2.380 | 4.780 / 2.380 |
| `qp-source` | uyarlayıcı (İ1: alan) | 4.201 / 2.898 | 4.235 / 2.910 |
| `qp-matriks` | uyarlayıcı | 3.655 / 2.681 | 3.542 / 2.910 |
| `qp-facts` | alan | 2.097 / 983 | 2.124 / 983 |
| `qp-render` | çekirdek | 1.746 / 1.854 | 1.752 / 1.846 |
| `qp-calendar` | çekirdek | 1.178 / 400 | 716 / 893 |

Toplam test dışı satır az arttı (≈ +1 %): yeni `//!` modül belgeleri, `mod`/`use` satırları,
parametre kayıt defteri, lint izin satırları. Bazı crate'lerde "test" sütunu büyüdü: satır içi
test modülleri `tests.rs` dosyalarına taşındı ve test sayılıyor.

**En büyük kaynak dosyalar:**

| Önce | Satır | Sonra | Satır |
|---|---|---|---|
| `qp-schema/src/protocol.rs` | 4.916 | `qp-session/src/ops/expand/op.rs` (İ6) | 910 |
| `qp-session/src/session.rs` | 3.713 | `qp-grammar/src/compile/layer/prepare.rs` (İ6) | 891 |
| `qp-session/src/capsule.rs` | 3.425 | `qp-grammar/src/advances.rs` (üretilmiş) | 878 |
| `qp-wasm/src/shell.rs` | 3.419 | `qp-session/src/data/plane.rs` | 763 |
| `qp-cli/src/livehost.rs` | 3.067 | `qp-capsule/src/outcome.rs` | 762 |
| `qp-mcp/src/surfaces/query.rs` | 2.550 | `qp-session/src/capsule/dispute.rs` | 758 |
| `qp-cli/src/live.rs` | 2.472 | `qp-source/src/replay.rs` | 751 |
| `qp-session/src/expand.rs` | 2.291 | `qp-matriks/src/trade.rs` | 725 |
| `qp-capsule/src/voice/selvi.rs` | 2.160 | `qp-session/src/ops/gops.rs` | 723 |
| `qp-capsule/src/voice/marla.rs` | 2.001 | `qp-schema/src/protocol/ops.rs` | 712 |

800 üstü kaynak dosya 56 → 3 (yukarıdaki ilk üç); 600–800 arası 37 (taban çizgisinde).

## 4. İstisna istekleri (kararı Arkin verir)

`scripts/arch-config.json` içinde adıyla durur; `arch-check` her koşuda "EXCEPTION … onay bekliyor"
basar. Onaylanmayan istisna geri alınamaz değildir: yapılandırmadan silinince `arch-check` kırmızıya döner.

| No | Ne | Gerekçe | Öneri |
|---|---|---|---|
| **İ1** | `qp-source`'u katman haritasında **alan**a almak (§2: uyarlayıcı) | İçeriği veri düzlemi: `FeedStore` (§18.4), `SourceProvider`, `MemoryProvider`, sentetik piyasa, yeniden oynatma takvimi, canlı bar kurma. Oturum bunları otoritenin parçası olarak kullanıyor (`replica`, `claims`, `Session::new_inner` → `MemoryProvider::from_init`). Dış dünyaya bakan kısım yerel dosya okuma (`local.rs`, `file.rs`). Dosya okumayı dışarıdan vermek gövde değişikliği ister. | Kabul. Sonraki bir round'da dosya okuma bir uyarlayıcıya ayrılsın. |
| **İ2** | Aynı katmanda 5 bağımlılık: `qp-capsule → qp-facts` (alan); `qp-host → qp-live, qp-matriks, qp-mcp, qp-shellcore` (uyarlayıcı) | §2 "yana bağımlılık hata verir" der. Kapsül olgudan kurulur. Barındırma uyarlayıcıları birleştirir, kabuk inceltmesinin doğal sonucu. Saf taşımayla kalkmıyorlar. | K-2 (§9): katman içinde bildirilmiş, döngüsüz sıra. Ya da "barındırma" adlı bir alt katman. |
| **İ3** | `qp-session` 32,0 bin > 12 bin | 23 modül tek güçlü bağlı bileşen (≈ 26 bin). `Session`'ın `impl`'leri 18 dosyaya dağılmış; Rust'ta tipin kendi `impl`'i başka crate'te yazılamaz. Konu klasörlerine bölündü (§5). | Kabul. Crate bölmesi `Session`'ı alt sistemlere ayıran bir tasarım round'u (RF) ister. |
| **İ4** | `qp-grammar` 19,3 bin | Çekirdek ↔ tasarım klasörleri (faces, open, stories, glyph) adla kayıt üstünden döngüsel. Döngüsüz kesimlerle (`feeds`+`transitions`+`motion` yukarı, `glyph`+`theme` aşağı) bile ≈ 14,7 bin kalıyor. | Kabul. Kaydın enjeksiyona çevrilmesi tasarım işi. |
| **İ5** | `qp-capsule` 13,2 bin | `voice/` (4,75 bin) saf taşımayla `qp-voice`'a ayrılabilir; döngü yok. Ama alanda iki yeni yan bağımlılık açar (İ2 türü). | İ2 kararına bağlı: K-2 kabul edilirse `qp-voice` ayrılsın (≈ 1 saatlik taşıma). |
| **İ6** | İki dosya > 800: `qp-session/src/ops/expand/op.rs` (910, yalnız `expand_op`), `qp-grammar/src/compile/layer/prepare.rs` (891, yalnız `prepare`) | Tek fonksiyon 800'ü aşıyor; fonksiyon bölmek bu round'da yasaktı. | Kabul; RF'de iki fonksiyon adlandırılmış adımlara bölünsün (taban çizgisinde `fn.lines`). |
| **İ7** | Kabuk WASM paketi (`--features replica`) gzip 2.506.216 → 2.569.974 B (**+%2,54**), ham +%1,99. Varsayılan `qp_wasm.wasm` +%0,05. | Ölçüldü: 135.580 B'lık IBM Plex Mono pakette 3 kez gömülü, eskiden 2 kezdi. `qp_render::fonts::FILES` ve `cpu::FONT_BYTES` birer `const`; her kullanan crate kendi kopyasını gömüyor. Eskiden ikisi de `qp-wasm`'daydı, şimdi biri `qp-shellcore`'da. | `const` → `static` (iki satır; davranış aynı). Denemede paket tabanın **altına** iniyor (gzip −%4,0). Taşıma kuralı dışında olduğu için yapılmadı; onay gelirse ilk iş. |

## 5. `qp-session` ve kabuklar nasıl bölündü

**`qp-session`** (37 düz dosya → 7 konu klasörü; her klasör ≤ 15 dosya; her klasörde README):

| Klasör | İçerik |
|---|---|
| `authority/` | `session.rs` + `session/` (create, intake, dispatch, command, worlds, accept, settle, events, clock, presence), `apply`, `error`, `suggest`, `values`, `agent` |
| `ops/` | `expand.rs` + `expand/` (op [İ6], checks, bind, undo, describe), `gops`, `canon` (+ scale, layer, origin), `select` (+ sites, rows, resolve), `locate`, `summary` (+ hints) |
| `capsule.rs` + `capsule/` | kapsül yaşamı: sentences, reader, run, queries, project, settled; `capidx` (+ items), `claims`, `dispute`, `comments`, `watch`, `story`, `headparts`, `public` (+ commit) |
| `data/` | `plane`, `provider`, `live` (+ eval), `bindq` |
| `read/` | `query` (+ view), `present`, `reading`, `preview`, `explain`, `interact` (+ style, scenes) |
| `replica.rs` + `replica/` | follow, judge, `viewer` (+ open, updates), `tape` |
| `host/` | `replayhost` (+ setup, control, recorded), `conform` |

İç yollar (`crate::claims::…`) gövdelere dokunmadan `lib.rs`'teki `pub(crate) use` yeniden
bildirimleriyle çalışır. Dışa açık yollar `pub use` ile aynı. `rd!` makrosu `macros.rs`'te.

**Kabuklar:**
- `qp-wasm` → yeni **`qp-shellcore`** (kabuk çekirdeği; uyarlayıcı katman). Taşınanlar: `shell` (+ feed, inbound, input, look, metrics, scene, start ve var olan agent, bench, freq, live, motion, paint*), `gallery`, `drive`, `viewer`. `qp-wasm`'da yalnız `wasm_bindgen` bağlamaları kaldı (`bind/`: compute, render, shell, gallery, replica, spike). JS adları ve `qp_wasm.d.ts` aynı.
- `qp-cli` → yeni **`qp-host`** (barındırma; uyarlayıcı katman). Taşınanlar: `livehost`, `live`, `live_trades`, `sim`, `demo`, `demo_static`, `mcp_serve`, `clap` argüman yapılarıyla birlikte. `qp-cli`'de `Cli`/`Command` ve `cmd/` bağlamaları kaldı. `qp --help` ağacı bayt eşit. `qp-cli → qp-wasm` bağımlılığı kalktı. `qp-wasm`'ın `replica` testleri kendi geliştirme bağımlılığıyla koşuyor.
- **Katman düzeltmesi:** `qp-render → qp-source` (yukarı) kalktı. `qp_render::synthetic_bars` sarmalayıcısı düştü; çağıranlar aynı fonksiyonu (`qp_source::synthetic_series`) doğrudan çağırıyor. **Dışa açık yüzey değişiklikleri** (protokol, şema, CLI ve MCP dışında):
  - `qp_render::synthetic_bars` kalktı.
  - `qp_mcp::common` yerine `qp_mcp::server_tools` geldi.
  - `qp_session::replica::{viewer, tape}` ve `qp_session::capsule::headparts` ek yol olarak eklendi.

**Öbürleri:**
- `qp-schema/protocol.rs` → `protocol/` altında 15 aile dosyası: envelope, ops, change, query, reply, summary, capsule, notify, input, replay, viewer, transcript, selector, wire, story. Dışa aktarılan şema bayt eşit.
- `qp-mcp`: `surfaces/{query,task,mixed}/`.
- `qp-capsule`: `voice/{selvi,marla}/`, `engine/`, `comment/`, `realize/elapsed/`.
- `qp-grammar`: `compile/`, `present/`, `look/` ve tasarım alt klasörleri.
- Küçük crate'ler: `conn/`, `mock/`, `feed/`, `livebar/`; `lib.rs` mantığı konu dosyalarına.

## 6. Performans ve boyut

| Ölçü | Tutanak | Son | Not |
|---|---|---|---|
| K17 `sweep_bench` medyan ms (sma / ema / rsi / macd / bollinger) | 4,76 / 3,88 / 9,21 / 7,93 / 9,44 | 4,56 / 4,15 / 9,70 / 8,46 / 9,08 | Karışık ±%7 makine durumundan: aynı anda ölçülen tutanak commit'i 4,53 / 4,10 / 9,76 / 8,51 / 9,05 verdi, HEAD 4,48 / 4,11 / 9,56 / 8,36 / 9,22 (`experiments/perf-final.txt`). Gürültü içinde eşit. |
| K52 `metrics.mjs` CPU ×4: tik p95 / sahne p95 ms | 0,6 / 25,1 | 0,7 / 27,3 | Sahne n = 8 (gürültülü). Bağlayıcı K52 telefon ölçümü değil; aynı düzenek, sınır içinde. |
| WASM varsayılan (adım 8) ham / gzip -9 | 2.280.713 / 745.478 | 2.281.954 / 746.734 | +%0,05 / +%0,17 |
| Kabuk paketi (`replica`) ham / gzip -9 | 8.923.686 / 2.506.216 | 9.101.130 / 2.569.974 | +%1,99 / **+%2,54** → İ7 |

## 7. Parametre kayıt defteri

- **Tip:** `qp_schema::params`. Öğeleri:
  - `ParamDef`: anahtar, tip, birim, aralık, varsayılan, kapsamlar, kim değiştirir, hak mı, açıklama, sürüm;
  - `ParamScope`, `ChangedBy`, `ParamValue`;
  - `resolve`: saf; sıra nesne > kullanıcı > plan > sistem; `Resolved { value, source }` "neden 3? → sistem varsayılanı" sorusunu cevaplar;
  - `REGISTRY`, `by_key`.

  Dışa aktarılan JSON şemasına girmez.
- **Belge:** `docs/PARAMETERS.generated.md` (`cargo run -p qp-schema --example params_doc`, güncelliğini bir test sınar). Envanter `docs/PARAMETERS.md`: ≈ 130 sabit; her biri için crate + öğe yolu, önerilen anahtar, kapsam ve bağ durumu.
- **Bağlanan sabitler** (yalnız ilklendirici değişti; değer ve tip birebir, sahibi crate'te eşitlik testi):

| Sabit | Anahtar | Değer |
|---|---|---|
| `qp_capsule::realize::LEDGER_MIN_N` | `scorecard.min_n` | 20 |
| `qp_grammar::depth::GLYPH_MIN_LEDGER_N` | `glyph.min_ledger_n` | 20 |
| `qp_capsule::engine::FRESHNESS_HALF_BARS`, `qp_grammar::depth::FRESHNESS_HALF_BARS` | `glyph.freshness_half_bars` | 5,0 |
| `qp_capsule::theme::GAP_BARS` / `CONFIRM_BARS` | `story.theme.gap_bars` / `.flow_confirm_bars` | 10 / 1 |
| `qp_capsule::outcome::C_DIR` / `C_NEUTRAL` | `outcome.barrier.c_dir` / `.c_neutral` | 0,3 / 0,65 |
| `qp_capsule::outcome::ATR_FAST` / `ATR_SLOW` | `outcome.atr_fast` / `.atr_slow` | 14 / 100 |
| `qp_capsule::outcome::INTRADAY_MAX_STEP_MS` | `outcome.intraday_max_step_ms` | 43.200.000 |
| `qp_session::watch::MAX_WATCHES` | `watch.max_per_person` (sistem + plan, **hak**) | 32 |
| `qp_session::dispute::MAX_OPEN` / `MAX_PER_CONN` / `MAX_DIGESTS` | `dispute.max_open_per_conn` / `.max_per_conn` / `.max_digests` | 16 / 256 / 4 |
| `qp_session::interact::STORY_ZOOM_MIN` | `gesture.story_zoom_min_bars` | 20 |
| `qp_grammar::depth::CHART_LOOKBACK` | `depth.chart.lookback_bars` | 30 |

- **Tasarım notu:** ilk tasarım `const fn` erişimciyle `panic!` kullanıyordu. Bağımsız denetçi bunu kütüphane kodunda yasak saydı (KALDI). Yerine her anahtar için sahibinin tipinde adlı varsayılan geldi (`SCORECARD_MIN_N_DEFAULT: u32`), `ParamDef` ondan kuruluyor. Yeniden denetim GEÇTİ.
- **Yapılmayan:** kullanıcı ya da plan kapsamında değer saklama (profil) kapsam dışıydı. Kalan ≈ 110 sabit `docs/PARAMETERS.md`'de "—" (borç).

## 8. Borç taban çizgisinde ne kaldı (`docs/DEBT.md`)

| Kural | Kalem | Ne |
|---|---|---|
| `fn.lines` | 177 | 80 satırı aşan fonksiyonlar. En uzunları `expand_op` 905, `prepare` 885, `compile` ≈ 650, `pavka::extend` ≈ 590, `gops::expand_grammar` ≈ 570. Bu round'da bölünmedi. |
| `file.soft` | 37 | 600–800 satır dosyalar (bölmenin parçaları dahil) |
| `lint.allow` | 13 crate | 237 susturulmuş lint (142 `too_many_lines`, 73 `expect_used`, 19 `unwrap_used`, 18 `cognitive_complexity`, 6 `panic`) |
| `test.file` | 6 | 800 satırı aşan test dosyaları (M5 kesildi) |
| `dir.files` | 3 | `qp-session/tests` 39, `qp-wasm/tests` 20, `qp-compute/tests` 16 (M5 kesildi) |
| `doc.size` | 2 | `docs/PROTOCOL.md` (216 KB), `docs/matriks/INDICATORS.md` (111 KB) |

Kesin kurallar (yukarı ve yan bağımlılık, 800 sınırı, yasak ad, README, kök dosya mantığı, `#[path]`,
modül düzeyinde lint susturma, crate'te `[lints] workspace = true`) taban çizgisi almaz; yalnız §4'teki
adlı istisnalar.

**Geçici yeniden dışa aktarmalar ve iç yeniden bildirimler** (PLAN §2.6; taşınan kodun gövdesine
dokunmamak için; sonraki round'larda yollar doğrudan yazılınca kalkar):
- `qp-wasm/src/lib.rs`: `pub use qp_shellcore::{shell, viewer, gallery, drive}` (eski `qp_wasm::shell…` yolları; `qp-host` artık `qp_shellcore`'u doğrudan kullanıyor, bu yeniden dışa aktarma yalnız dış kullanıcılar için).
- `qp-session/src/lib.rs`: `pub(crate) use authority::{…}`, `ops::{…}`, `data::{…}`, `read::{…}`, `host::{…}`, `capsule::{claims, dispute, …}`, `replica::{viewer, tape}` — iç `crate::x::` yollarını koruyan yeniden bildirimler; dışa `pub use` ile `agent`, `headparts`, `conform`, `replayhost`, `viewer`, `tape`, `capsule`, `replica`.
- `qp-grammar/src/lib.rs`: `pub use compile::{scale, terms}`, `pub use present::{depth, gesture, motion}`, `pub(crate) use compile::{draw, frame, layer}`.
- `qp-mcp`: `mixed` `pub(crate)` fonksiyonları eski yolda `pub(crate) use` ile (`thin` için).
- `qp-source`, `qp-compute`, `qp-facts`: kökte `pub(crate) use` (`read`, `check_period`, `period_param`, `evidence`, `clip01`, `bar_f64`).
- `qp-cli/src/main.rs`: `use qp_host::{demo, live, mcp_serve, sim}` (eski `mod` yolları).

## 9. Kural önerileri (`docs/ENGINEERING.md` değiştirilmedi)

- **K-1** Geliştirme bağımlılıkları (testler, örnekler) katmanlanmasın. `arch-check` öyle uyguluyor (örn. `qp-render` testleri `qp-session`'ı kullanıyor). Metinde yazsın.
- **K-2** §2'ye katman içinde **bildirilmiş, döngüsüz sıra** (ya da alt katmanlar). "Yana bağımlılık yasak" bugünkü grafikle uygulanamıyor: `qp-capsule → qp-facts`, barındırma (İ2).
- **K-3** Crate ve kabuk bütçesi **test dışı** satırla sayılsın (`arch-check` öyle sayıyor); dosya sınırı ham satır (VERIFY 7 ile aynı).
- **K-4** Çözümleme sırasında **enstrüman** kapsamının yeri yok (§6 nesne > kullanıcı > plan > sistem); `resolve` bugün enstrüman değerini yok sayıyor. Öneri: nesne > enstrüman > kullanıcı > plan > sistem.
- **K-5** "Üretilmiş ve veri tablosu" bütçe dışı kuralı işaret yerine **adla listeleme** istesin (`arch-config.json` `generated_files`). Başlıktaki "üretilmiştir" sözcüğü tek başına kolay kandırılır.
- **K-6** Ortak `CARGO_TARGET_DIR` (K56) yalnız disk içindir; doğrulama iş başına ayrı klasörde koşsun (Ö108).
- **K-7** Tek fonksiyonu 800'ü aşan dosya için kural sırası belli olsun: önce fonksiyon bölünür (`fn.lines`), sonra dosya. Taşıma round'larında istisna.
- **K-8** `rg1-moved.py` / öğe parmak izi denetimi kalıcı bir RF aracı olsun (`scripts/rg1-*` → `scripts/refactor/`); `use`, `impl` başlığı, öğe düzeyi makro ve `cfg` denetimi eklensin (denetim B8).
- **K-9** `scripts/arch-config.json` ve `arch-baseline.json` farkları her round'da denetçinin kontrol listesinde olsun; istisna ve üretilmiş dosya eklemek karar satırı ister (denetim B5).
- **K-10** `arch-check`'in sözcük tarayıcısı yerine sözdizimi ağacı (makro, `include!`, kapanış, `#[path]` içeren modüller) — sonraki RF (denetim B2, B4, B6).
- **K-11** `unimplemented`, `unreachable` lint'leri de açılsın (ENGINEERING §5'te yasak; RG1 listesi beş lint'ti) (denetim B14).

## 10. Fark edilenler (dokunulmadı)

- **Belge ve yorum eskimeleri** (`.rs`/`.mjs`; belge yorumu bu round'da değiştirilemezdi):
  - `qp-host/src/demo.rs` `//!` ve `DriveArgs` `///` hâlâ `qp_wasm::gallery`/`qp_wasm::drive` diyor (doğru, yeniden dışa aktarma; ama eski ad).
  - `shells/web/arch-check.mjs:11,58`, `gallery.js:2`, `paint/gpu.js:4,25`, `paint/canvas.js:6,10` eski `qp-wasm/src/...` yollarını anıyor.
  - `qp_mcp` `config.rs` `[`common`]` rustdoc bağlantısı kırık.
  - `look.rs` `//!` "bu dosyaya yaz" diyor (görünüşler artık `look/*.rs`).
  - `qp-capsule` `lib.rs` `//!` "This file is the contract" (öğeler `contract.rs`'te).
  - `outcome::C_DIR` yorumu "ATR(14)" diyor (artık `outcome.atr_fast`).
- **Wasm32 clippy:** `cargo clippy -p qp-session --target wasm32-unknown-unknown` `drop_non_drop` veriyor (`host/conform.rs:313`). Önceden de vardı; `verify.sh` bu adımı koşmuyor.
- **Karar kaydında çift numara:** Ö39, Ö40, Ö41 hem R3 hem R4 bölümünde ayrı kararlar (dizinde not var). A46'ya başvuru var ama satırı yok (`rounds/R5/experiments/S/CONTRAST.md:3`).
- **`.gitignore` `*token*` kalıbı** yeni bir `mock/token.rs` dosyasını sessizce dışarıda bırakıyordu (M4d adı `issuer.rs` yaptı). Kalıp çok geniş.
- **`qp-source::local_bars_dir` derleme zamanı yolu** (R5B §6): öneri aynen duruyor. Davranış değişikliği olduğu için yapılmadı.
- **Kopyalar:** aynı değerin iki yerde tanımlı olduğu sabitler `docs/PARAMETERS.md` "kopyalar" bölümünde:
  - karne "20" üç yerde;
  - `MAX_CLAIMS` iki yerde;
  - yeniden oynatma varsayılanları üç yerde;
  - hassasiyet kesimleri iki yerde.
- **Yakın sınırdakiler:** `qp-capsule/src/outcome.rs` 762 ve `qp-session/src/capsule/dispute.rs` 758 satır, 800'e yakın.
- **Tam sınırdakiler:** `qp-schema/src/protocol/` ve `qp-shellcore/src/shell/` tam 15 dosyada; yeni dosya için yer yok.

## 11. `recovered/worktree-agent/*` dalları (karar listesi)

Ayrıntı `experiments/recovered.md`. 24 dalın **24'ünde de benzersiz iş yok**. Her birinin içeriği
ya `main`'de var (rebase edilmiş karşılığı) ya da adlı bir `r5/m16-*` dalında. Eğer bir fark varsa,
o fark yalnız commit edilmiş `target/` artığı ya da yer tutucu metin.

**Önerim: 24'ünü de sil.** `r5/m16-bertan` ve `r5/m16-kivra` kalmalı; elenen alternatifler onlarda. Silme
yapılmadı.

## 12. Kesilenler (GOAL kesme sırası)

1. **M6 veri tablolarının koddan ayrılması**: kesildi. Ses tabloları (`selvi`, `marla`) yalnız 800 sınırı için konu dosyalarına bölündü; içerik kodda.
2. **M5 test klasörlerinin konuya göre düzenlenmesi**: kesildi. Yalnız `tests/common` → `tests/support` (yasak ad). Borçta: 3 klasör (`dir.files`), 6 test dosyası (`test.file`).
3. **Lint sıkılaştırmasında mevcut ihlallerin düzeltilmesi**: kesildi. 237 ihlal öğe başına susturuldu, sayısı mandallı (`lint.allow`).

`STATUS.md` round geçmişi kısa (104 satır), arşive gerek görülmedi.

## 13. Doğrulama (VERIFY 1–15)

| # | Sonuç |
|---|---|
| 1 | Tutanak commit'i `6d03df2`, ilk kod taşımasından (`rg1/m*` dalları `decb3c8` sonrası) eski ✅ |
| 2 | `rg1-compare.sh`: 225 kalem bayt eşit (`experiments/compare-final.txt`) ✅ |
| 3 | `verify.sh` 19/19 PASS, `arch` dahil (`experiments/verify-final.log`) ✅ |
| 4 | `git diff --stat 6d03df2 HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views` boş ✅ |
| 5 | `arch-check`: kesin ihlal 0; 11 istisna onay bekliyor; taban çizgisi içindekiler listeleniyor ✅ (İ1–İ6) |
| 6 | `arch-check --self-test` PASS (20 örnek + mandal + istisna) ✅ |
| 7 | 800 üstü: `op.rs` 910, `prepare.rs` 891 (İ6), `advances.rs` 878 (üretilmiş, listeli) ⚠️ İ6 |
| 8 | Kabuklar 1.419 / 1.324 < 4 bin ✅; 12 bin üstü 3 crate → İ3–İ5 ⚠️ |
| 9 | Mandal denemesi (`experiments/ratchet.txt`): değer aşımı, yeni kalem, yazım reddi ✅ |
| 10 | `ls crates/*/README.md \| wc -l` = 16 = crate sayısı ✅ |
| 11 | `docs/PARAMETERS.md`, `docs/PARAMETERS.generated.md`; 17 sabit bağlı ✅ |
| 12 | `scripts/rg1-decisions-check.py b341e2d 1957752` ✅ |
| 13 | Performans / WASM tablosu §6 ⚠️ İ7 |
| 14 | `git worktree list` yalnız ana ağaç; `du -sh target` §16 ✅ |
| 15 | `AUDIT.md`, `STATUS.md`, `ROADMAP.md` ✅ |

**CI:** `07430545` (son kod durumu: bütün taşımalar, K, L) GitHub Actions koşusu **45035266520 yeşil**
(32 dk). Sonraki commit'ler yalnız araç (`arch-check` denetim düzeltmeleri), belge ve `AUDIT.md`; kapanış
commit'inin koşusu raporun yazıldığı anda sürüyordu (STATUS'a yazılır).

## 14. Yöntem, alt ajanlar, dallar

- **Sıra:** tutanak (`6d03df2`) → `arch-check` (`3b08a9d`) → plan v1 → iki bağımsız eleştiri → plan v2 (`2c170c7`) → taşımalar.
- **Taşıma ajanları:** her taşıma kendi worktree'si ve dalında ayrı bir alt ajanla yapıldı. Her dal ayrı bir denetçi alt ajandan GEÇTİ almadan birleştirilmedi. Denetçinin baktıkları:
  - fmt, clippy, test;
  - `rg1-compare`;
  - `rg1-moved` öğe parmak izi;
  - `arch-check --root`;
  - elle örneklem.
- **Dallar** (kod dalda da duruyor): `rg1/m1a-shellcore`, `rg1/m1b-host`, `rg1/m2-session`, `rg1/m3-schema`, `rg1/m4a-grammar`, `rg1/m4b-capsule`, `rg1/m4c-mcp`, `rg1/m4d-small`, `rg1/k-params`, `rg1/l-lint`, `rg1/r-readme`, `rg1/audit`.
- **Hatalarım ve düzeltmeler:**
  - **Ortak `target/` güvenilmezliği:** worktree'ler birbirinin derlediği crate'i taze sayabiliyordu (M4b ajanı buldu). O andan sonra her hüküm ayrı klasörle verildi (Ö108).
  - **`rg1-moved` hataları:** üç hata bulundu ve düzeltildi: `construct` → `const` okuması, `&'static`, rustfmt yeniden kırması ve `#[allow]` soyma. Düzeltmeler commit edildi. Her dal düzeltilmiş sürümle yeniden ölçüldü.
  - **`rg1-snapshot` boş izin listesi:** liste yalnız yorum taşıyınca betik sessizce düşüyordu (pipefail). Fark ilk `main` doğrulamasında görüldü ve düzeltildi.
  - **K parametre tasarımı:** `panic!` içeriyordu; denetim KALDI, yeniden tasarlandı.

## 15. Denetim bulguları ve cevaplar (`AUDIT.md`)

| No | Bulgu | Ağırlık | Cevap |
|---|---|---|---|
| B1 | Lint mandalı atlatılabiliyordu: modül düzeyinde `#![allow]` tek sayılıyordu; `#[expect]`, `cfg_attr(…allow…)`, `clippy::restriction` sayılmıyordu; `[lints] workspace = true` silinirse fark edilmiyordu | yüksek | **Düzeltildi** (`3f2af7e7`): üçü `lint.allow`'a girer; `src`'de modül düzeyinde susturma (`lint.inner`) ve eksik `[lints]` (`lints.workspace`) kesin kural; öz-sınamada. Yeniden denetim (AUDIT.md son bölüm): büyük ölçüde kapandı (yüksek → orta); kalan üç hile de sonra kapatıldı (`clippy :: unwrap_used` boşluklu yol, `clippy::nursery` grubu, `.cargo/config` içinde `-A`/`--cap-lints` → `lint.flags` kesin kural). Açık kalan: `[package.metadata]` içine yazılmış sahte `[lints]` metni (yapay; K-10). |
| B3 | Düz `pub mod tests;` ile ulaşılan `tests.rs` test sayılabiliyordu | orta | **Düzeltildi**: ancak kendi ata modülü `#[cfg(test)]` ile bildirirse test. |
| B5 | Elle küçültülen/ gevşek taban çizgisi kalemi yalnız "stale" uyarısı; `generated_files` sessiz | orta | **Kısmen**: gevşek kalem hata ("tighten"); üretilmiş dosyalar her koşuda basılır. Açık: taban çizgisi değeri ile kodun **aynı miktarda** birlikte büyütülmesi ve elle istisna eklenmesi araçla yakalanamaz (yapılandırmanın kendisi); git geçmişinde görünür. K-9: `verify.sh`'a `main`'e göre yapılandırma/taban çizgisi farkını basan ve her yeni istisna için karar satırı isteyen adım. |
| B2, B4, B6 | `lib.rs`'te `include!`/makro mantığı, `src` dışından `include!`, `[[bin]]` yolu, makroyla üretilen uzun fonksiyon, `const` içinde uzun kapanış, `#[path] mod util;` yakalanmıyor | orta/düşük | Bugünkü kodda yok (denetçi doğruladı). Kalıcı çözüm Rust sözdizimi ağacıyla çalışan bir denetim (ör. `syn` tabanlı küçük araç) — K-10, sonraki RF. |
| B7 | 3, 4, 5, 7. ölçütler istisnalara bağlı | orta | Kabul; §4 İ1–İ7 Arkin'in kararında. Rapor bunları "karşılandı" değil "⚠️ istisnaya bağlı" yazar. |
| B8 | `rg1-moved` `use` satırlarını, `impl` başlıklarını, öğe düzeyi makroları, `cfg`'yi görmüyor | orta | Denetçi bunları ayrıca denetledi, sorun yok. K-8: aracı kalıcı RF aracına çevirirken eklenmeli. |
| B9 | K52 tutanak çıktısı kısaltılmış; tek koşu | düşük | Doğru: tutanak `rg1-perf.sh`'ın ilk sürümüyle (çıktının son 40 satırı) alındı; tik ve sahne p95 görünür. K52'nin bağlayıcı ölçümü telefonda; RD2 her tasarımda yeniden ölçecek (K82: ara ölçüm tekrarlanmadı). |
| B10 | A63'te +%2,36, ölçüm +%2,54 | düşük | Düzeltildi (A63 ve §4 İ7: son ölçüm +%2,54). |
| B11 | Tutanak barındırma kodunun azını kapsıyor (`qp sim`, `qp live`, demo sunucusu yalnız adım 18) | düşük | Doğru. `qp sim`/`qp live` gerçek ya da önbellekli veri ister (`fixtures/local/`, tutanağa giremez). Taşıma öğe parmak iziyle 0 CHANGED; adım 18 demo sunucusunu uçtan uca sınadı. |
| B12 | `qp-cli`'de barındırmaya yakın kod kaldı (`cmd/replay.rs::replay_cmd`, `cmd/serve.rs` stdio döngüleri, `cmd/scene.rs::scene_session`) | düşük | Kabul; kabuk bütçesi içinde (1,4 bin). Sonraki kabuk temizliğinde `qp-host`'a; `docs/DEBT.md`'ye not. |
| B13 | Bağlanan sabitler `resolve`'u değil adlı varsayılanı okuyor; `resolve` min/max denetlemiyor | düşük | Bilinçli: bu round'da yalnız sistem kapsamı (profil saklama kapsam dışı); varsayılan = eski sabit. Min/max denetimi profil saklama gelince (değer kullanıcıdan gelince) eklenmeli. |
| B14 | `unimplemented!`/`unreachable!` lint dışında; kütüphanede 15 `unreachable!` | düşük | ENGINEERING §5 `unimplemented!`'ı da yasaklıyor; RG1 GOAL'ün listesi beş lint'ti. K-11: `unimplemented`, `unreachable` lint'leri sonraki round'da aynı yöntemle. |
| B15 | Geçici yeniden dışa aktarmalar listelenmemiş | düşük | Listelendi (§8). |
| B16 | Kapanış işleri bekliyordu | düşük | §13 (CI), §16 (disk), `STATUS.md`, `ROADMAP.md`. |

## 16. Disk (K56, K68)

**Silinenler:**

| Ne | Boyut |
|---|---|
| Taşıma ve denetçi worktree'leri | 23 worktree, `git worktree remove` ile birleştirilince; dallar kaldı. Harness'in `worktree-agent-*` taban dalları: hepsi `main`'in atası, benzersiz commit'i olmayanlar `git branch -d` ile silindi. |
| Özel derleme klasörleri `target/wt/<iş>` | her biri 11–14 GB; iş bitince silindi |
| `target/main` (ana ağacın doğrulama klasörü) | 23 GB |
| Ortak `target/` (`scripts/clean.sh --deep` → `cargo clean`) | 50,1 GiB (255.970 dosya) |
| Oturum karalama klasörü, `__pycache__` klasörleri, A/B için açılan geçici worktree | — |

Ayrıntı `experiments/clean.log`.

**Bilerek bırakılanlar:**
- `shells/web/node_modules` (18 MB) ve `shells/web/pkg` (13 MB): `verify.sh` adım 18 onlara bağlı.
- `fixtures/local/` (1,5 GB): yerel veri, bütçe içinde (≤ 2 GB ham, ≤ 5 GB önbellek).
- `rounds/R3/experiments/harness/target/` (140 MB): R3'ten kalan deney derlemesi. RG1'in değil; silinsin mi, ana akış karar versin.
- `/private/tmp/claude-501/…` altında başka oturumların klasörleri (≈ 1,8 GB): bu oturumun değil, dokunulmadı. Bu oturumunki 472 KB (görev çıktıları).
- `rg1/*` dalları: kod dalda da kalsın diye.

**Kalan boyutlar:**

| Klasör | Boyut |
|---|---|
| `target/` | 0 (bir sonraki `verify.sh` yeniden derler) |
| `.claude/worktrees/` | 32 KB |
| `fixtures/local/` | 1,5 GB |
| `.git` | 229 MB |
| Depo toplamı | ≈ 2,2 GB |

`git worktree list` yalnız ana ağacı gösteriyor. `git status --porcelain --ignored` içinde beklenmeyen artık yok: `.DS_Store`, `.env.local`, `fixtures/local/`, `node_modules`, `pkg` ve yukarıdaki R3 deney klasörü.
