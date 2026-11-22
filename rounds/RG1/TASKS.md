# RG1 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlı |
|---|-------|-------------|---------|
| T0 | Hazırlık: ana akış değişikliklerini kaydet (`CLAUDE.md` "Ana akış değişiklikleri"; bu round'da `DECISIONS.md`, `ROADMAP.md`, `STATUS.md`, `CLAUDE.md`, `docs/ENGINEERING.md`, `docs/HANDOFF.md`, `docs/research/`, `rounds/RG1/`). Deneme alt ajanı. `docs/ENGINEERING.md`, `docs/ARCHITECTURE.md` ("Kod düzeni (hedef)") oku. | — | — |
| B | **Başlangıç tutanağı** (önce, commit): `verify.sh` tam koşu çıktısı; altınların, dökümlerin, şema dışa aktarımının, MCP araç tanımlarının, hesap özetinin SHA-256 listesi; performans ölçümleri (K17 hesap bench, K52 telefon bütçesi düzeneği, WASM boyutu); crate / dosya / klasör satır sayıları; karşılaştırma betiği `scripts/rg1-compare.sh` | `rounds/RG1/experiments/baseline/`, `scripts/` | T0 |
| A1 | `scripts/arch-check` + öz-sınama + `verify.sh` adımı; ilk koşunun ihlal listesi (hedef haritası) | `scripts/`, `rounds/RG1/experiments/` | B |
| P | **Bölme planı** (kod taşımadan önce, tek belge): her crate için hedef klasör/crate sınırları, katman, taşınacak dosyalar, sıra, risk; kabuklardan çıkacak mantığın gideceği yer; `qp-session` sınırları. Orkestratör yazar; iki bağımsız alt ajan ayrı ayrı eleştirir | `rounds/RG1/PLAN.md` | A1 |
| M1 | Kabukların inceltilmesi (`qp-cli`, `qp-wasm`) | ilgili crate'ler | P |
| M2 | `qp-session` bölünmesi | ilgili crate'ler | P |
| M3 | `qp-schema/protocol.rs` modüllere | `crates/qp-schema` | P |
| M4 | Kalan 800+ satır dosyalar, 15+ dosyalı klasörler, yasak adlar (crate başına bir alt ajan) | crate başına | P |
| M5 | Testlerin konuya göre düzenlenmesi (kesilebilir) | `crates/*/tests` | M1–M4 |
| M6 | Veri tablolarının koddan ayrılması (kesilebilir) | `qp-capsule`, `qp-grammar` | M4 |
| R | Crate `README.md`'leri | `crates/*/README.md` | M1–M4 |
| L | Lint sıkılaştırma + taban çizgisi | kök `Cargo.toml`, crate'ler | M1–M4 |
| K1 | Parametre tanım tipi + üretilen belge | `crates/qp-schema` | — |
| K2 | Sabit envanteri (`docs/PARAMETERS.md`) | `docs/` | M1–M4 (yollar değiştiği için sonra) |
| K3 | En az 10 sabitin kayıt defterine bağlanması | ilgili crate'ler | K1, K2 |
| D1 | `DECISIONS.md` bölünmesi + satır kümesi karşılaştırması + başvuru denetimi | `docs/decisions/`, kök dizin | — |
| W | `recovered/*` dallarının özeti (silme yok) | `rounds/RG1/REPORT.md` | — |
| Z | Borç taban çizgisi + mandal testi | `docs/DEBT.md`, `scripts/arch-baseline.json` | hepsi |
| C | Tutanak karşılaştırması (her birleştirmeden önce ve sonda) | `rounds/RG1/experiments/compare/` | sürekli |
| D | Bağımsız denetim | `rounds/RG1/AUDIT.md` | hepsi |
| X | Kapanış: rapor, durum, yol haritası, temizlik | `rounds/RG1/`, kök | D |

## Notlar

- **Sıra önemli:** tutanak → `arch-check` → plan → taşımalar. Plan yazılmadan kod taşınmaz.
- **Küçük adımlar:** her taşıma kendi dalında/worktree'sinde, kendi commit'iyle; birleştirmeden
  önce o dalda `scripts/verify.sh` (en az fmt, clippy, test) ve `scripts/rg1-compare.sh` yeşil.
  Aynı crate'e aynı anda iki alt ajan yazmaz (plan sırayı verir).
- **Taşı, yeniden yazma:** dosya `git mv` ile taşınır ki geçmiş izlenebilsin; içerik değişikliği
  yalnız `mod`, `use`, görünürlük (`pub(crate)`) ve yeniden dışa aktarma satırlarıdır. Fonksiyon
  gövdesi değişmez. 80 satırı geçen fonksiyonları bu round'da **bölme** (davranış riski);
  taban çizgisine yaz.
- **Dışa açık yollar:** crate'ler arası `pub` yollar yeniden dışa aktarmayla korunabilir; round
  sonunda geçici yeniden dışa aktarmalar listelenir (borç).
- **Yeni crate açarsan:** `qp-` öneki, katmanı `docs/ENGINEERING.md` §2'ye göre, `README.md`,
  `docs/ARCHITECTURE.md` "Kod düzeni"ne satır, `DECISIONS.md`'ye önerilen karar.
- **Üretilmiş dosyalar** (proto çıktısı vb.) başlarında "üretilmiştir" taşır; `arch-check` bu
  işareti tanır.
- **Parametre bağlama:** varsayılan değer eski sabitle birebir aynı; okuma yolu saf ve
  deterministik (hesap kodunda saat, ağ, rastgelelik yok). Anahtar adları İngilizce,
  noktalı (`sentinel.repeat_hold`, `scorecard.min_n`).
- **Disk (K56, K68):** bütün worktree'ler ortak `CARGO_TARGET_DIR`; taşıma bitince worktree
  kaldırılır. Bu round çok dal açar; `target/` bütçesini ara ara denetle.
- **GitHub:** `CLAUDE.md` kuralları (`scripts/qgh`, `preflight.sh`) geçerli.
