# RD2 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlı |
|---|-------|-------------|---------|
| T0 | Hazırlık: ana akış belge değişikliklerini kaydet (`CLAUDE.md` "Ana akış değişiklikleri"). Alt ajan başlatılabiliyor mu, rapora yaz. | kök | — |
| Ö1 | A62: eski `qp mcp --init --principal` bağdaştırıcısının kaldırılması + test | `crates/qp-mcp`, `crates/qp-cli`, `docs/` | T0 |
| Ö2 | İ7: yazı tipi `const` → `static`; kabuk paketi önce/sonra | `crates/qp-render` | T0 |
| Ö3 | Silmeler: 24 `recovered/*` dalı, R3 deney derlemesi; istisna mandalı | git, `scripts/archcheck` | T0 |
| U | **Uyarlama envanteri** `docs/design/ADOPTED.md` (kaynak başına bir okuyucu alt ajan: OpenChart + Vela; Vibe-Trading + valuecell; OpenAlice + trading-command-center + OpenTerminalUI + nofx); oturumdaki tasarım becerilerinin listesi | `docs/design/` | T0 |
| S1 | Tasarım sözleşmesi taslağı + `design-check` (P1–P13) + `verify.sh` adımı | `docs/design/`, `scripts/`, `crates/qp-grammar` (test) | U |
| G | Karşılaştırma düzeneği: dört çapa ekran, aynı veri, DPR kuralı, başvuru ekranı yan yana, `oltan` aynı düzenekte; galeri | `shells/web`, `rounds/RD2/experiments/` | S1 |
| Y-a, Y-b, Y-c | Üç yön (her biri ayrı alt ajan, kendi dalında; birbirini görmez): görünüş kaydı + dört çapa | `crates/qp-grammar/src/look/`, `docs/design/mockups/<yön>/` | G |
| K1 | **Ara teslim:** tek sayfalık karşılaştırma; **dur, Arkin'e sor**; karar `DECISION.md` | `rounds/RD2/` | Y-* |
| C1 | Seçilen yön: görünüş kaydı (iki profil, iki tema), `css_vars`; elenen yönlerin kodu dalda kalır, worktree kalkar | `crates/qp-grammar` | K1 |
| C2 | Alışılmış grafik öğeleri gramerde (P4–P8) | `crates/qp-grammar`, `crates/qp-schema` (yalnız görünüş/sahne tipleri gerekiyorsa) | K1 |
| C3 | Kapsül yüzü, akış, açılmış kapsül, hikâye çizelgesi aynı dille; web kabuğu | `crates/qp-grammar`, `crates/qp-shellcore`, `shells/web` | C1 |
| M1 | `docs/design/SURFACES.md` Y01–Y37 | `docs/design/` | K1 |
| M2 | Kademe 1 maketleri (tam) | `docs/design/mockups/` | M1, C1 |
| M3 | Kademe 2 maketleri (ana durum) | `docs/design/mockups/` | M2 |
| E1 | Seçilen yönde karşıtlık ve renk körlüğü (R5 düzeneği, bir kez); K52, K17, WASM boyutu | `rounds/RD2/experiments/` | C1–C3 |
| B | Proje becerisi `.claude/skills/qapsule-design/SKILL.md` + taze alt ajanla sınama | `.claude/skills/` | C1, M2 |
| H | İnsan testi v4 paketi | `rounds/RD2/human-test/` | C3, M2 |
| K2 | **Son kabul:** Arkin'e sor; cevap `DECISION.md`; gösterim varsayılanı ancak kabulden sonra değişir | `rounds/RD2/` | E1, H |
| D | Bağımsız denetim (`AUDIT.md`) | `rounds/RD2/` | hepsi |
| X | `REPORT.md`, `STATUS.md`, `ROADMAP.md`, önerilen kararlar (`docs/decisions/rounds/RD2.md`), kapanış temizliği | kök | D |

Kurallar: aynı crate'e aynı anda iki alt ajan yazmaz (üç yön ayrı dallarda, ayrı görünüş
dosyalarında). Her dal birleştirmeden önce ayrı denetçiden geçer. Kesin hükmü veren derleme
iş başına ayrı klasörde (K87).
