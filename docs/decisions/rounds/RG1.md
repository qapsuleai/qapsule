# Karar Günlüğü — Önerilen (RG1) — Arkin'in onayını bekliyor (2026-11-22)

<!-- dizin: DECISIONS.md; RG1 yönetişim turunun önerdiği satırlar -->

RG1 bu satırları **önerir**; kesinleştirmez. Kanıt `rounds/RG1/REPORT.md`, `rounds/RG1/PLAN.md`,
`rounds/RG1/AUDIT.md`, `docs/DEBT.md`, `scripts/arch-config.json` (`exceptions`).

| # | Önerilen karar | Kanıt |
|---|----------------|-------|
| Ö103 | **`scripts/arch-check` depo genelindeki yapı denetimidir** (`verify.sh` adım 19): katman yönü (`cargo metadata`, normal ve derleme bağımlılıkları), dosya / fonksiyon / klasör / crate / kabuk / test / belge bütçeleri, yasak adlar, crate ve modül README'leri, `lib.rs`/`mod.rs`'te mantık, `src/` dışına çıkan `#[path]`, susturulmuş lint sayısı. Kesin kurallar taban çizgisi almaz; öbürleri `scripts/arch-baseline.json`'da mandallıdır (değer ve sayı yalnız azalır); istisnalar `arch-config.json`'da adıyla. `docs/DEBT.md` taban çizgisinden üretilir. | Öz-sınama 20 bilinen ihlal + mandal; `rounds/RG1/experiments/ratchet.txt`. |
| Ö104 | **Yeni crate `qp-shellcore`** (kabuk çekirdeği, `ShellCore`; uyarlayıcı katman): `qp-wasm`'dan taşındı; `qp-wasm` yalnız `wasm_bindgen` bağlamalarıdır. | `qp-wasm` 10,7 bin → 1,3 bin satır; 225 kalem bayt eşit. |
| Ö105 | **Yeni crate `qp-host`** (barındırma: canlı barındırıcı, Matriks kaydı, işlem karşılaştırması, benzetim, demo sunucusu, MCP sunucusunun bağlanması; uyarlayıcı katman): `qp-cli`'den taşındı; `qp-cli` yalnız `clap` ve bağlama. | `qp-cli` 11,7 bin → 1,4 bin satır; komut satırı `--help` ağacı bayt eşit. |
| Ö106 | **Parametre kayıt defteri `qp_schema::params`** (K78, ENGINEERING §6): `ParamDef` (anahtar, tip, birim, aralık, varsayılan, kapsamlar, kim değiştirir, hak mı, açıklama, sürüm), saf `resolve` (nesne > kullanıcı > plan > sistem), sistem varsayılanları sahibinin tipinde adlı sabitler (`*_DEFAULT`), bağlanan sabit yalnız ilklendiricisiyle onları okur; belge `docs/PARAMETERS.generated.md` üretilir; dışa aktarılan JSON şemasına girmez. | 17 sabit / 16 anahtar; panic'siz (denetim KALDI → düzeltme → GEÇTİ). |
| Ö107 | **Lint'ler açık:** `unwrap_used`, `expect_used`, `panic`, `too_many_lines` (80), `cognitive_complexity`; testlerde izinli (`clippy.toml`); mevcut ihlaller öğe başına `#[allow]` ile, sayısı `lint.allow` kuralında mandallı. | 237 öğe izni (hepsi `#[expect]` ile doğrulandı), 158 test/örnek kökünde iç izin. |
| Ö108 | **Ortak `CARGO_TARGET_DIR` doğrulama için kullanılmaz** (K56'yı inceltir): Cargo yol paketlerinin artefakt özetini çalışma alanına göreli hesapladığı için worktree'ler birbirinin derlediği crate'i "taze" sayabilir. Kesin hükmü veren derleme (denetçi, birleştirme öncesi ve sonrası karşılaştırma) iş başına ayrı klasörde (`target/wt/<iş>`, `target/main`) koşar ve iş bitince silinir. | RG1/M4b gözlemi; denetçiler ayrı klasörle koştu. |

| # | Açık karar (RG1) | Ne zaman | Not |
|---|------------|----------|-----|
| A63 | İstisna istekleri İ1–İ7 (`rounds/RG1/REPORT.md` §4): `qp-source` katmanı; aynı katmandaki bağımlılıklar; `qp-session`, `qp-grammar`, `qp-capsule` crate bütçesi; tek fonksiyonlu iki dosya; kabuk WASM paketi gzip +%2,54 | Bir sonraki ana akış oturumu | Her biri gerekçeli; `arch-check` her koşuda "EXCEPTION … onay bekliyor" basar. |
| A64 | Kural önerileri K-1…K-11 (`rounds/RG1/REPORT.md` §9): §2'ye sıralı alt katman; geliştirme bağımlılıklarının katmanlanmaması; enstrüman kapsamının çözümleme sırasındaki yeri; crate bütçesinin test dışı satırla sayılması … | `docs/ENGINEERING.md` güncellemesi | ENGINEERING metni RG1'de değiştirilmedi. |
