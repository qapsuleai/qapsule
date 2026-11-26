# RL1 — Bağımsız kapanış denetimi

**Denetçi:** bağımsız kapanış denetçisi alt ajanı (görev D; kod yazmadı). **Tarih:** 2026-11-26.
**Denetlenen:** `main` @ `50eb42d0` (RL1: kapanış temizliği günlüğü). **Round başı:** `08bcb4d6`.
**Ortam:** ana ağaç (temiz), ayrı derleme klasörü
`CARGO_TARGET_DIR=target/wt/audit-d` (sıfırdan derleme; iş bitince silindi).
`QP_CONFIG_BASE=08bcb4d6`.

## Hüküm: **KOŞULLU KABUL**

Kod tarafında koşul yok. Davranış koruması, belirlenimcilik, çekirdek donukluğu ve yapı
kurallarını kendim ölçtüm ve tuttular:

- `QP_VERIFY_BROWSER=1 scripts/verify.sh` temiz derleme klasöründe **22/22 PASS** verdi
  (yeni `program` adımı dahil, 1269 sn).
- `compare.sh`: RF1 tutanağının 270 kaleminden **245'i bayt eşit**, 25'i farklı, 12 kalem yeni;
  silinen kalem yok. Farkları örnekleyerek denetledim; hepsi ekleme.
- `fixtures/` altında yalnız 9 yeni dosya var.
- `qp-authority/src` farkı tam olarak D2'nin iki özniteliği (+2/−2).
- Dış bağımlılık eklenmemiş. Program özeti üç mimaride eşit.

Koşullar kapanış belgeleriyle ilgili. Kapanmadan önce K1–K4 yapılmalı.

| # | Kapanmadan önce yapılacak |
|---|---|
| K1 | `REPORT.md`'de CLAUDE.md'nin istediği **"Disk" bölümü yok**. Üç liste eklensin: silinenler (`clean.log`: `target/` 35G → 0, 20,9 GiB), bilerek bırakılanlar (13 `rl1/*` dalı adıyla) ve kalan boyutlar. §6 satır 18 temizlik kanıtı olarak "§13"ü gösteriyor; §13 "beş dakikada deneme" bölümü (B1). |
| K2 | **Varsayılan WASM modülü** (verify adım 8) RF1'e göre **+%10,5 ham / +%5,6 gzip** büyümüş: 2.354.884 → 2.602.684 B, 773.646 → 817.143 B. Sözleşmede iki dayanak var: D4 "varsayılan modül ±%2 kuralında kalır" diyor, VERIFY 6 da varsayılan modülü sayıyor. REPORT §10 bu büyümeden hiç söz etmiyor; yalnız `perf/RESULT.md` "İ8 ölçüsü değil" notunu düşüyor. REPORT §10'a büyüme ve nedeni (I1 tipleri + `qp-lang`) yazılsın. ±%2 kuralının özellik round'unda geçerli olup olmadığı da Arkin'e açık soru olarak eklensin; D4'ün cümlesi artık doğru değil (B2). |
| K3 | REPORT §3–§4 denetim tablolarında **PB** (`qp-program`, birleştirme iletisi "ikinci denetim KABUL"; demek ki ilk denetim RET) ve **C** (birleştirme "denetim KABUL") satırı yok. Ayrıca §4 U3 satırında hâlâ "→ düzeltiliyor" yazıyor; düzeltme yapılmış (`qp-program/src/asked.rs:95`, sınamalar `qp-session/tests/program/queries.rs:87`, `audit.rs:65`) (B3, B4). |
| K4 | CI sonucu REPORT'ta yok ve ben göremedim. Ö3 "CI tam geçmiş + `config` adımı CI'da" ölçütünün koşu kanıtı, gönderimden sonra (`scripts/preflight.sh` ile) gösterilsin. Denetçi gönderim yapmaz (B9). |

## Kabul ölçütleri (GOAL 1–18)

| # | Ölçüt | Hüküm | Kanıt (kendi ölçümüm) |
|---|---|---|---|
| 1 | Ön işler | ✅ (CI: K4) | Ö1: kabuk paketi şimdi 7.167.256 / 2.078.599 B (`shells/web/pkg`, adım 18 derlemesi). `o1-wasm/RESULT.md` önce/sonra ölçümünü, açılışı ve K52'yi veriyor. Ö2: `calls.restricted` kuralı `arch-config`'te; öz-sınama ve verify adım 15 (yansı) geçiyor. Ö3: `Cargo.toml` `unimplemented`/`unreachable = "warn"`; clippy `-D warnings` PASS. `ci.yml` `fetch-depth: 0` commit `5d7232a1`'de. |
| 2 | Kavramlar, PROGRAMS, PROTOCOL koddan önce | ✅ | `ae5ad278` (00:42) PROGRAMS.md, PROGRAMS-SESSION.md, CONCEPTS ("Hesap dili (RL1)", 16 kavram) ve PROTOCOL §26'yı getiriyor. İlk `qp-lang` commit'i `c09f97b0` (02:22). `git merge-base --is-ancestor ae5ad278 c09f97b0` doğru; topolojik sırada 11. ve 29. commit. |
| 3 | Tipler `qp-schema`'da; dışa aktarım yalnız ekleme; döküm/altın/özet bayt eşit | ✅ | `compare.sh` 245/270 eşit; 25 fark + 12 yeni kalem `perf/COMPARE.md`'deki listeyle aynı. Örneklemim (aşağıda) yalnız ekleme gösterdi. verify `golden`, `views`, `compute`, `digest` PASS. |
| 4 | Bütün hatalar birden, konum, öneri; kod başına sınama | ✅ | `qp-lang` `codes.rs` 29 sınama, `all_at_once.rs` 6. PROGRAMS.md'deki 28 gerçek `E_…` kodunun hepsi sınamalarda geçiyor (karşılaştırma `comm`; dışarıda kalan tek ad yer tutucu `E_KOD`). |
| 5 | Belirlenimcilik; üç mimaride özet | ✅ | verify adım 12: arm64 = x86_64 = wasm32. Var olan özet `7b1e99d40889f83c`, rl1 özeti `e196f240e628c64d`, program özeti `4e77c249…3c41a`. Sınamalar `modes::two_runs_are_byte_equal` ve `modes::batch_equals_streaming_and_revised_live_bars` geçiyor. |
| 6 | Önek kararlılığı; olay yalnız kesin barda | ✅ | `prefix::prefix_stability_over_random_programs`: sabit tohum, 240 rastgele program; en az 200 olgu, olaylı olgu sayısı 20'den fazla ve hizalı 4s kutuları da var. `modes::live_bar_never_emits` ve `signals::*` geçiyor. |
| 7 | Beş `add_study` bit eşit | ✅ | `add_study::{sma,ema,rsi,macd,bollinger}_is_bit_equal` geçiyor. |
| 8 | Üst periyot seansa hizalı; 7/24 UTC | ✅ | `align::xist_daily_and_four_hour_follow_the_session`, `align::utc_alignment_for_24_7_instruments`, `incomplete_first_higher_bar_never_forms` ve `a_gap_closes_…` geçiyor. |
| 9 | Parametreli program, iki örnek | ✅ | `qp-lang/tests/params.rs` 7 sınama geçiyor. Oturumda `set_program_params` için `qp-session/tests/program` 32/32. |
| 10 | Metin ↔ ara gösterim; hata konumu; ret | ✅ | `qp-lang-text` `roundtrip` 5, `errors` 12, `limits` 8, `examples` 7 geçiyor. Bu iş üç denetim turundan geçti (iki RET, REPORT §3). |
| 11 | Maliyet önce; bütçe işi sayar; kişi başı `Check`'te; sınırlar kayıt defterinde | ✅ | `qp-schema/src/params/program.rs`'te 21 `program.*` anahtarı var. `exec::budget::*` (bar × ağırlık, kayıt/açık kayıt, okuma, durduktan sonra kayıt yok) geçiyor. `qp-program` `Phase::Check` ve `AfterCommit` kullanıyor. U3 B01–B13 sonuçları da var. |
| 12 | Oturum işlemleri/sorguları; seri grafikte ve koşulda; olay kesin barda; yeniden oynatma; ajan önerisi | ✅ | Dökümler 71–77 üç taşımada PASS (adım 14). Adım 22 aynı dökümleri yeniden türetti ("same"); adım 16 yeniden oynatma bayt eşit. Döküm 76 ajan önerisi. |
| 13 | `qp-authority/src` farkı | ✅ (D2 ile) | Fark tam olarak D2'nin iki özniteliği. Bkz. VERIFY 4. GOAL'ün harfiyle ("yalnız yumuşak dosya ya da boş") örtüşmüyor, ama Arkin'in D2 kararı bu farkı açıkça tanımlıyor ve net satır sıfır. Yeni istisna yok. `qp-session/src` farkı yalnız `compose.rs`: 1 kayıt satırı, gerisi sınama modülü. |
| 14 | MCP araçları, `qp program`; ajan belgeleri | ✅ | İnce yüzeyde `program_check/try/explain/add` var; karar aracı yok (aşağıda). `docs/AGENTS.md` "Programlar (RL1)" bölümü ve `agents/programci.md` var. `qp-cli` `program` sınamaları 7/7. |
| 15 | RX4 uygunluğu; her fark açıklanmış | ✅ | `rx4/RESULT.md` 52 "eşit" satır ve 4/4 ret görevi. Bit eşitliği 30684/42000. Bit farkı olan dört satırın (G2-01/02 × iki küme) dördü de §15 sapmasıyla açıklanmış; tolerans içinde. |
| 16 | Ajan teyidi; kötü niyet | ✅ | `agents/RESULT.md`: haiku ve opus 20/20, ret 4/4, uydurma ad 0, sessiz yanlış 0. REPORT §8 bunları RX4 ile yan yana veriyor. `abuse/REPORT.md` 115 deneme. P18 düzeltildi; L20 sonradan koşuldu (Ek, OK). Üç KOŞULLU sonuç A76'da. |
| 17 | K17, K52, WASM (İ8), tik maliyeti ölçüldü | ✅ (not) | Ölçüt "ölçüldü" diyor; ölçülmüş. K17 tanımı tarama API'si için (`kararlar.md` K17). O yol tutuyor: yerel ≤ 9,5 ms, WASM ≤ 20 ms. Program yolu (264 / 392 ms) bu round'da kapsamda olmayan program taramasının yeni bir ölçümü; REPORT onu ⚠️ ve A75 ile dürüstçe gösteriyor, gerileme gibi sunmuyor. Bu sunumu doğru buluyorum. Kabuk paketi D6 İ8'in içinde (%2,3 pay). K52: tik p95 değişmedi. İzleyici kipinde ilk sahne kurulumunun p95'i +15 ms (Öneri 5). Varsayılan modül büyümesi raporsuz (K2). |
| 18 | Bağımsız denetim, REPORT/STATUS/ROADMAP, temizlik | ⚠️ K1, K3 | AUDIT bu belge. STATUS (RL1 satırı) ve ROADMAP ("Tamamlandı") güncel. REPORT'ta Disk bölümü yok ve tablolarda eksikler var. Temizlik yapılmış: `clean.log`, worktree yalnız ana ağaç. |

**Kesilenler (REPORT "Kesilenler"):** yalnız iç içe gösterge adresi kesildi. Bu, GOAL zaman
kuralında kesme sırasının ilk maddesi ve GOAL'de "(kesilebilir)" diye işaretli. Dil bunu zaten
ifade ediyor (`ema(rsi(close, 18), 20)`); eksik olan yalnız adres ayrıştırıcısı ve A77'ye
yazılmış. Kesme sırasının öbür maddeleri yapılmış: kutulama (`bucket`), `state`, kanonik metin,
`stdev`/`percentrank` ve küçük model teyidi. Gerekçe yeterli.

## VERIFY 1–20

| # | Komut / bakış | Sonuç |
|---|---|---|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` (temiz `target/wt/audit-d`) | ✅ **22/22 PASS**, `total: 1269s`, `verify: OK`. Adımlar: fmt, clippy, test (637 sn), schema (32 şema), fixtures, golden, views, wasm, hygiene, browser, compute, digest, protocol (69 örnek), conform (77 × 3), replica, replay (1816 satır), shell, demo, arch, config, design, **program** ("5 PROGRAMS.md §12.5 examples ok; CLI check = MCP program_check; transcripts 71-77 re-derived"). |
| 2 | `scripts/refactor/compare.sh` (RF1 tutanağı) | ✅ (beklenen çıkış 1) 270 → 282 kalem: **245 bayt eşit**, 25 farklı, 12 yeni, 0 silinen. `COMPARE.md` ile bire bir aynı. Ekleme örneklemim: `schema/Query` ve `schema/Series` JSON düzeyinde eski her yol yerinde ve değişmemiş, yalnız ekleme var. `tests.names` −0 / +227. Döküm 03'ün 7. satırında 685 karakter ortak önek var; tek fark `expected` listesinin sonuna `,"add_program","set_program_params","remove_program"` eklenmesi (+52 B). `cli/qp.txt` +1 satır (`program`). `conform/inproc.txt` 70 satır aynı, sonra 71–77. `schema/Op` ve `Document` için tutanakta metin kopyası yok; onlarda C görevinin `08bcb4d6` derlemesiyle yaptığı denetime güvendim. |
| 3 | `git diff --name-status 08bcb4d6 HEAD -- fixtures/{golden,compute,protocol,views,schema}` | ✅ Yalnız `A` satırları (9): `compute/digest-program.txt`, `digest-rl1.txt`, `protocol/conformance/71…77`. `fixtures/` altında değişen ya da silinen dosya yok. |
| 4 | `git diff --stat 08bcb4d6 HEAD -- crates/qp-authority/src` | ✅ (D2) 2 dosya, +2/−2: `authority/session/dispatch.rs` ve `capsule/detail.rs`. İkisinde de `#[allow(clippy::too_many_lines)]` → `#[allow(clippy::too_many_lines, clippy::unreachable)] // invariant: …`. Tek commit (`1329fabc`), net satır 0. |
| 5 | `scripts/arch-check`, `--config-diff` (taban `08bcb4d6`) | ✅ 0 hata; 2 adlı istisna (İ1, İ3; İ3 12.744 = tavan, büyümedi). Taban çizgisi: `file.soft` 32→31, `fn.lines` üç öğede sıkılaştı. config-diff: 14 değişiklik, **0 kanıtsız gevşeme**. Kanıtlı gevşemeler: `lint.allow` qp-compute 27→30, qp-ops 20→23, qp-plane 1→2 (D2) ve `restricted_calls` anahtarı (D3; yeni sıkılaştırıcı kural). Yeni crate'ler `qp-lang` ve `qp-lang-text` `alan`, `qp-program` `oturum` katmanına yerleşmiş. |
| 6 | Kabuk paketi ham/gzip; varsayılan modül | ✅ / ⚠️ K2. Kabuk paketi (adım 18 derlemesi, `shells/web/pkg/qp_wasm_bg.wasm`) **7.167.256 / 2.078.599 B**. D6 İ8 tavanı 7.332.886 / 2.128.236; pay %2,3 / %2,3. `perf/RESULT.md` ile 104 B / 211 B içinde aynı. Varsayılan modül (adım 8): 2.602.684 / 817.143 B; RF1'e göre +%10,5 / +%5,6. REPORT'ta yok. |
| 7 | `git log` sırası | ✅ `ae5ad278` (şartname + CONCEPTS + PROTOCOL §26) ilk `qp-lang` kodu `c09f97b0`'ın atası. |
| 8 | `cargo test -p qp-lang -p qp-lang-text` | ✅ Hepsi geçiyor: `codes` 29, `all_at_once` 6, `abuse` 8, `checked` 5, `cost` 7, `exec` 36, `identity` 7, `params` 7, `registry` 2, birim 10; metin tarafında birim 3, `errors` 12, `examples` 7, `limits` 8, `roundtrip` 5. |
| 9 | Belirlenimcilik | ✅ Adım 12'de üç özet üç mimaride eşit (yukarıda). `modes::two_runs_are_byte_equal`, `batch_equals_streaming_and_revised_live_bars` ve `digest::digest_matches_the_fixture` geçiyor. |
| 10 | Önek kararlılığı; kesin bar | ✅ `prefix_stability_over_random_programs`, `live_bar_never_emits`, `transient_value_equals_closed_value_without_higher_timeframes`. |
| 11 | `add_study` eşdeğerliği | ✅ 5/5 bit eşit (toplu, akışkan, düzeltme). |
| 12 | Üst periyot hizası | ✅ XIST günlük/4s ve 7/24 UTC sınamaları geçiyor; RX4 G8-02/03 4s kutuları da var. |
| 13 | Metin ↔ ara gösterim; hata konumu | ✅ `roundtrip` (özellik), `errors` (satır, sütun, uzunluk). |
| 14 | Bütçe; kötü niyet raporu | ✅ `exec::budget` 3, `hostile` 2, `summary::budget_stops_the_try`; `qp-session/tests/program` (kişi başı, kabul, geri alma) 32/32. `abuse/REPORT.md`: R01 kodlu durma (`bar 18914`), B01–B13 kişi başı sınırlar. |
| 15 | Dökümler 71–77 üç taşımada; yeniden oynatma | ✅ Adım 14 "77 transcripts x inproc, serve, mcp"; adım 22 71–77 "same"; adım 16 iki koşu bayt eşit. |
| 16 | `qp program check` = MCP `program_check` | ✅ `qp-cli/tests/program.rs::cli_check_equals_mcp_program_check` geçiyor; verify adım 22 de aynısını denetliyor. |
| 17 | RX4 tablosu | ✅ 52/52 + 4/4. Her bit farkı satırında açıklama var. İlk koşudaki G8 farkı bir uygulama hatasıydı ve düzeltildi (`666f3939`, denetim KABUL). |
| 18 | Ajan teyidi raporda, RX4 ile yan yana | ✅ REPORT §8 tablosu `agents/RESULT.md` ile aynı. Hata iletisinden sonra RX4 sütunu "—" (RESULT'ta 0/0); tutarlı. |
| 19 | K17, K52, tik maliyeti | ✅ (not) K17 tarama yolu bütçe içinde. Program yolu ölçülmüş ve dürüstçe ❌ yazılmış (A75). K52 tik p95 bütçenin çok altında. Program tik maliyeti en çok ≈ 4,1 µs/tik (WASM, 32 seri). Perf sayılarını `perf/RESULT.md`'den okudum, kendim koşmadım. |
| 20 | worktree, `target/`, `Prompt.txt`, belgeler | ✅ / ⚠️ K1. `git worktree list` yalnız ana ağaç. `git ls-files Prompt.txt` boş, dosya yok. `du -sh target` benim temizliğimden sonra **1,8 MB**. Ana ağaçta izlenen değişiklik yok. Kalan izlenmeyen artık: `rounds/RL1/experiments/rx4/__pycache__/` (2 KB, gitignore'da); dokunmadım (B7). |

## Kesin kurallar (BRIEF)

| Kural | Hüküm | Kanıt |
|---|---|---|
| Yeni dış bağımlılık yok | ✅ | `Cargo.lock` farkında yeni `name =` satırları yalnız `qp-lang`, `qp-lang-text`, `qp-program`; yeni `source =` satırı yok. Yeni crate'lerin tek dış bağımlılığı zaten kilitte olan `serde_json`. SHA-256 ve öneri araması elle yazılmış. |
| `exp/ln/powf/mul_add` yok; saat/ağ/rastgelelik yok | ✅ | `qp-lang`, `qp-lang-text`, `qp-program` `src` altında `.exp( .ln( .powf( .powi( .mul_add(` ve trigonometri yok. `qp-compute` farkında da yok. `SystemTime`, `Instant`, `rand`, `std::net` yok. |
| Sınırlar kayıt defterinde | ✅ (not) | 21 `program.*` anahtarı. `qp-lang-text`'in `DEFAULT_MAX_*` sabitleri yalnız yedek: yüzeyler (`qp-cli`, `qp-mcp`) değeri kayıt defterinden geçiriyor. Kalan sabitler yapısal (ileti uzunluğu, öneri arama tavanı, zaman birimleri) (B6). |
| MCP'de karar aracı yok | ✅ | İnce yüzeyde program araçları yalnız `program_check/try/explain/add`. `accept_proposal`/`reject_proposal` ince yüzeyde yok. Ajan teyidinde "karar denemesi 0". |
| Sinyal tavsiye diye sunulmaz (K77) | ✅ | PROGRAMS §5.2 (kaynak zorunlu, "simüle", `mode: "replay"`, canlıda iddia yok); `agents/programci.md` "Tavsiye dili yok". `side al/sat` sözcükleri RX4 SPEC'ten geliyor. |
| Çekirdek donuk | ✅ (D2) | VERIFY 4. Sözleşmenin yetmediği iki yerde durulup sorulmuş (D1); eksik kancalar `hooks/HOOKS.md`'de satır tahminleriyle. |
| Gerçek veri yok | ✅ | `rx4/data/seen.bars.json` RX4'ün tohumlu sentetik verisi (`harness/README.md` `gen_data.py`). U3 LCG sentetik. |
| Her birleştirilen dalda bağımsız denetim | ✅ (belge: K3) | 12 birleştirme commit'i var; hepsinin iletisinde "denetim KABUL" yazıyor (L üçüncü, PB ikinci turda). Ö3 doğrudan main'e geldi; REPORT §1'de "KABUL". `9faef338` (D3 `rule_changes`, yalnız yapılandırma) doğrudan main'de; config-diff onu kanıtlı sıkılaştırma olarak gösteriyor (B5). Denetim raporları dosya olarak depoda değil (RF1'deki `audit-*.md` gibi); kanıt REPORT tabloları ve birleştirme iletileri. |

## Bulgular

| # | Önem | Bulgu | Kapanıştan önce? |
|---|---|---|---|
| B1 | Orta (kapanış belgesi) | REPORT'ta "Disk" bölümü yok (CLAUDE.md: silinenler, bilerek bırakılanlar, kalan boyutlar). §6 satır 18'in "§13" atfı yanlış. | **Evet (K1)** |
| B2 | Orta | Varsayılan WASM modülü +%10,5 ham / +%5,6 gzip; D4'ün "±%2 kuralında kalır" cümlesi artık doğru değil; REPORT §10'da yok. İ8 kabuk paketi için; varsayılan modülün tavanı belirsiz. | **Evet (K2)**: yazılsın + açık soru |
| B3 | Düşük | REPORT §3–§4'te PB ve C denetim satırları yok. PB'nin ilk denetimi RET olmuş, nedeni yazılmamış. | **Evet (K3)** |
| B4 | Düşük | REPORT §4 U3 satırı "→ düzeltiliyor"; §9 "düzeltildi". Kod düzeltilmiş (`asked.rs:95`). | **Evet (K3)**, metin |
| B5 | Bilgi | `9faef338` (D3 yapılandırma onarımı) denetimsiz doğrudan main'e geldi; yalnız yapılandırma ve kanıtlı. Bu kırılma Ö2 denetçisinin `--config-diff`'i koşmamasından doğdu; REPORT §1'de not var. | Hayır |
| B6 | Bilgi | `qp-lang-text` varsayılan sınırları kayıt defteri değerlerinin kopyası (64 Ki / 1024 / 64). Kayıt defteri değişirse yedek eskir; yüzeyler kayıt defterini kullandığı için bugün etkisiz. | Hayır (öneri) |
| B7 | Bilgi | `rounds/RL1/experiments/rx4/__pycache__/` gitignore'da duran artık (2 KB). Benim verify koşum `target/web-spike` (1,8 MB) ve `shells/web/pkg`'yi (13 MB, izlenmiyor) yeniden üretti. | Hayır |
| B8 | Bilgi | K17 program yolu 5,3× / 2,6× üstte (A75). K52'de izleyici kipinin ilk sahne kurulumu p95 +15 ms (Öneri 5). İkisi de REPORT'ta dürüstçe yazılı. | Hayır (Arkin'in sorusu) |
| B9 | Bilgi | CI'ı göremedim; REPORT RL1 için CI koşusu göstermiyor. | **Evet (K4)** |

## Doğrulayamadıklarım

- **CI:** uzak koşu yok/görmedim; `qgh` ve `preflight` kullanmadım (denetçi gönderim yapmaz).
- **K17, K52, tik ölçümleri ve ajan teyidi:** kendim koşmadım; `perf/RESULT.md`,
  `wasm/TICK.md`, `agents/RESULT.md`'den okudum, iç tutarlılığını ve REPORT ile eşliğini denetledim.
- **Dal denetimlerinin kendisi:** yeniden koşmadım; yalnız REPORT tabloları ve birleştirme
  iletileri var, denetim raporları depoda dosya değil.
- **`schema/Op`, `Document`'in eklemeliği:** tutanakta metin kopyası olmadığından C'nin
  `08bcb4d6` karşılaştırmasına güvendim; `Query` ve `Series`'i kendim denetledim.

## Arkin'e soru listesi (REPORT'takilere ek)

1. (REPORT) HOOKS.md önerileri R7'den önce (A74).
2. (REPORT) Program taramaları K17 kapsamında mı (A75)?
3. (REPORT) `qp serve`'de ajan kuralı varsayılan mı (A76)?
4. (yeni, K2) Varsayılan WASM modülünün (verify adım 8) bir tavanı var mı? RF1'den bu yana
   +%10,5. D4'teki ±%2 cümlesi düzeltilsin mi, yoksa İ8 gibi bir satır mı yazılsın?
5. (yeni, B6) Metin dil varsayılan sınırları kayıt defterinden mi okunsun (tek kaynak)?

## Temizlik

- `target/wt/audit-d` (19 GB) silindi. `du -sh target` sonrası 1,8 MB.
- Scratchpad'deki `compare.sh` ağacı, fark dosyası ve günlükler silindi.
- Ürün kodu değişmedi; birleştirme ve gönderim yapılmadı. Bu denetimin tek değişikliği bu dosya.
